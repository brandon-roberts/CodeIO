//! `codeio serve` — the app-server + web GUI. Std-only HTTP (no framework) so it builds fast
//! everywhere including Termux. Binds the engine, IR, and router to a port; serves a browser IDE.
//! This is the host the GUI IDE grows in and the future mesh coordinator.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

pub fn serve(bind: &str) {
    let listener = match TcpListener::bind(bind) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: cannot bind {bind}: {e}");
            std::process::exit(1);
        }
    };
    println!("CodeIO server on http://{bind}");
    println!("  open it in a browser (phone: use this machine's LAN IP, e.g. http://192.168.x.x:{})",
        bind.rsplit(':').next().unwrap_or("?"));
    println!("  Ctrl-C to stop");
    for stream in listener.incoming() {
        if let Ok(s) = stream {
            // one request per connection; simple and robust for a dev server
            std::thread::spawn(move || handle(s));
        }
    }
}

fn handle(mut stream: TcpStream) {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("/");

    // read headers to find content-length
    let mut content_length = 0usize;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line.is_empty() {
            break;
        }
        if let Some(v) = line.to_lowercase().strip_prefix("content-length:") {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        let _ = reader.read_exact(&mut body);
    }
    let body = String::from_utf8_lossy(&body).to_string();

    let (status, ctype, payload) = route(method, path, &body);
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(payload.as_bytes());
}

fn route(method: &str, path: &str, body: &str) -> (&'static str, &'static str, String) {
    match (method, path) {
        ("GET", "/") => ("200 OK", "text/html; charset=utf-8", INDEX_HTML.to_string()),
        ("POST", "/run") => ("200 OK", "application/json", api_run(body)),
        ("POST", "/ir") => ("200 OK", "application/json", api_ir(body)),
        ("GET", "/status") => ("200 OK", "application/json", api_status()),
        ("GET", "/map") => ("200 OK", "application/json", api_map()),
        _ => ("404 Not Found", "text/plain", "not found".into()),
    }
}

// extract the "code" field from a tiny JSON body {"code":"..."} without a json dep
fn extract_code(body: &str) -> String {
    if let Some(i) = body.find("\"code\"") {
        if let Some(colon) = body[i..].find(':') {
            let rest = &body[i + colon + 1..];
            if let Some(start) = rest.find('"') {
                let mut out = String::new();
                let mut chars = rest[start + 1..].chars();
                while let Some(c) = chars.next() {
                    match c {
                        '\\' => match chars.next() {
                            Some('n') => out.push('\n'),
                            Some('t') => out.push('\t'),
                            Some('"') => out.push('"'),
                            Some('\\') => out.push('\\'),
                            Some(o) => out.push(o),
                            None => break,
                        },
                        '"' => break,
                        c => out.push(c),
                    }
                }
                return out;
            }
        }
    }
    String::new()
}

fn json_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\t' => o.push_str("\\t"),
            '\r' => {}
            c => o.push(c),
        }
    }
    o
}

fn api_run(body: &str) -> String {
    let code = extract_code(body);
    // capture output: our print! goes to stdout, so run and collect the last value + printed text
    match codeio_lang::run_source(&code) {
        Ok(v) => format!("{{\"ok\":true,\"result\":\"{}\"}}", json_escape(&v.to_string())),
        Err(e) => format!("{{\"ok\":false,\"error\":\"{}\"}}", json_escape(&e)),
    }
}

fn api_ir(body: &str) -> String {
    let code = extract_code(body);
    let toks = match codeio_lang::lexer::Lexer::new(&code).tokenize() {
        Ok(t) => t,
        Err(e) => return format!("{{\"ok\":false,\"error\":\"{}\"}}", json_escape(&e)),
    };
    let stmts = match codeio_lang::parser::Parser::new(toks).parse_program() {
        Ok(s) => s,
        Err(e) => return format!("{{\"ok\":false,\"error\":\"{}\"}}", json_escape(&e)),
    };
    let g = codeio_ir::lower(&stmts);
    // full node list with kind, attrs summary, children — for the blueprint canvas
    let mut nodes = String::from("[");
    let mut ids: Vec<&String> = g.nodes.keys().collect();
    ids.sort();
    for (i, id) in ids.iter().enumerate() {
        let n = &g.nodes[*id];
        if i > 0 { nodes.push(','); }
        let label = n.attrs.get("name").or(n.attrs.get("op")).or(n.attrs.get("value"))
            .or(n.attrs.get("binding")).cloned().unwrap_or_default();
        let kids: Vec<String> = n.children.iter().map(|c| format!("\"{}\"", &c[..8.min(c.len())])).collect();
        nodes.push_str(&format!(
            "{{\"id\":\"{}\",\"kind\":\"{}\",\"label\":\"{}\",\"children\":[{}]}}",
            &id[..8.min(id.len())], n.kind.as_str(), json_escape(&label), kids.join(",")));
    }
    nodes.push(']');
    let roots: Vec<String> = g.roots.iter().map(|r| format!("\"{}\"", &r[..8.min(r.len())])).collect();
    format!("{{\"ok\":true,\"count\":{},\"roots\":[{}],\"nodes\":{}}}", g.len(), roots.join(","), nodes)
}

fn repo_root_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_default();
    loop { if dir.join("features.toml").exists() { return dir; } if !dir.pop() { break; } }
    std::env::current_dir().unwrap_or_default()
}

// Scan the real repository into a hierarchy for the codebase map (middle-out semantic zoom).
fn api_map() -> String {
    let root = repo_root_dir();
    let mut crates: Vec<String> = Vec::new();
    let services = root.join("services/crates");
    if let Ok(entries) = std::fs::read_dir(&services) {
        let mut cdirs: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir()).collect();
        cdirs.sort();
        for cdir in cdirs {
            let cname = cdir.file_name().unwrap().to_string_lossy().to_string();
            let src = cdir.join("src");
            let mut files_json: Vec<String> = Vec::new();
            if let Ok(fs_entries) = std::fs::read_dir(&src) {
                let mut files: Vec<_> = fs_entries.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().map_or(false, |x| x == "rs")).collect();
                files.sort();
                for f in files {
                    let fname = f.file_name().unwrap().to_string_lossy().to_string();
                    let content = std::fs::read_to_string(&f).unwrap_or_default();
                    let loc = content.lines().count();
                    // extract top-level symbols: fn / struct / enum / trait / pub fn
                    let mut syms: Vec<String> = Vec::new();
                    for line in content.lines() {
                        let l = line.trim_start();
                        for (kw, kind) in [("pub fn ","fn"),("fn ","fn"),("pub struct ","struct"),
                                           ("struct ","struct"),("pub enum ","enum"),("enum ","enum"),
                                           ("pub trait ","trait"),("trait ","trait")] {
                            if let Some(rest) = l.strip_prefix(kw) {
                                let name: String = rest.chars()
                                    .take_while(|c| c.is_alphanumeric() || *c=='_').collect();
                                if !name.is_empty() {
                                    syms.push(format!("{{\"name\":\"{}\",\"kind\":\"{}\"}}", json_escape(&name), kind));
                                }
                                break;
                            }
                        }
                    }
                    files_json.push(format!(
                        "{{\"name\":\"{}\",\"loc\":{},\"symbols\":[{}]}}",
                        json_escape(&fname), loc, syms.join(",")));
                }
            }
            crates.push(format!("{{\"name\":\"{}\",\"files\":[{}]}}", json_escape(&cname), files_json.join(",")));
        }
    }
    format!("{{\"ok\":true,\"crates\":[{}]}}", crates.join(","))
}

fn api_status() -> String {
    // read features.toml (repo root discovered by walking up) to show real system state
    let mut dir = std::env::current_dir().unwrap_or_default();
    let mut path = None;
    loop {
        let p = dir.join("features.toml");
        if p.exists() { path = Some(p); break; }
        if !dir.pop() { break; }
    }
    let text = path.and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    // crude TOML scan: collect (name,status) pairs
    let mut items: Vec<(String,String)> = Vec::new();
    let (mut name, mut status) = (String::new(), String::new());
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("name") { name = l.splitn(2,'=').nth(1).unwrap_or("").trim().trim_matches('"').to_string(); }
        if l.starts_with("status") { status = l.splitn(2,'=').nth(1).unwrap_or("").trim().trim_matches('"').to_string();
            if !name.is_empty() { items.push((name.clone(), status.clone())); } }
    }
    let (mut live, mut building, mut planned) = (0,0,0);
    for (_,s) in &items { match s.as_str() { "live"=>live+=1, "building"=>building+=1, _=>planned+=1 } }
    let arr: Vec<String> = items.iter().map(|(n,s)| format!("{{\"name\":\"{}\",\"status\":\"{}\"}}", json_escape(n), s)).collect();
    format!("{{\"live\":{live},\"building\":{building},\"planned\":{planned},\"features\":[{}]}}", arr.join(","))
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CodeIO</title>
<style>
  html,body { margin:0; padding:0; min-height:100%; background:#0d1117; color:#e6edf3;
              -webkit-backface-visibility:hidden; overflow-x:hidden; }
  body { font-family:ui-monospace,SFMono-Regular,Menlo,monospace; }
  header { padding:12px 16px; border-bottom:1px solid #30363d; display:flex; align-items:center; gap:10px; }
  header h1 { font-size:16px; margin:0; letter-spacing:1px; }
  header .tag { color:#8b949e; font-size:11px; }
  .tabs { display:flex; border-bottom:1px solid #30363d; }
  .tabs button { flex:1; padding:12px; background:#0d1117; color:#8b949e; border:0; border-bottom:2px solid transparent;
                 font:inherit; font-size:13px; font-weight:700; }
  .tabs button.on { color:#e6edf3; border-bottom-color:#4a9eff; }
  main { padding:12px; max-width:960px; margin:0 auto; }
  .view { display:none; } .view.on { display:block; }
  textarea { width:100%; height:280px; background:#161b22; color:#e6edf3; border:1px solid #30363d;
             border-radius:8px; padding:12px; font:inherit; font-size:16px; resize:vertical;
             -webkit-appearance:none; backface-visibility:hidden; }
  .row { display:flex; gap:8px; margin:10px 0; }
  button.act { flex:1; padding:12px; background:#4a9eff; color:#001; border:0; border-radius:8px; font:inherit; font-weight:700; }
  button.alt { flex:1; padding:12px; background:#161b22; color:#e6edf3; border:1px solid #30363d; border-radius:8px; font:inherit; font-weight:700; }
  pre { background:#161b22; border:1px solid #30363d; border-radius:8px; padding:12px; white-space:pre-wrap;
        word-break:break-word; min-height:50px; font-size:13px; }
  canvas { width:100%; height:60vh; background:#0a0d12; border:1px solid #30363d; border-radius:8px; touch-action:none; }
  .lbl { color:#8b949e; font-size:11px; text-transform:uppercase; letter-spacing:1px; margin:12px 0 4px; }
  .feat { display:flex; justify-content:space-between; padding:8px 10px; border:1px solid #30363d;
          border-radius:6px; margin:4px 0; background:#161b22; font-size:12px; }
  .badge { font-weight:700; } .live{color:#3fb950;} .building{color:#d29922;} .planned{color:#8b949e;}
  .counts { display:flex; gap:8px; margin:8px 0; }
  .counts div { flex:1; text-align:center; padding:10px; border:1px solid #30363d; border-radius:8px; background:#161b22; }
  .counts .n { font-size:22px; font-weight:700; } .counts .k { font-size:10px; color:#8b949e; text-transform:uppercase; }
</style></head>
<body>
<header><h1>CodeIO</h1><span class="tag">universal language · live</span></header>
<div class="tabs">
  <button class="on" onclick="tab('edit',this)">Editor</button>
  <button onclick="tab('bp',this)">Blueprint</button>
  <button onclick="tab('map',this)">Codebase</button>
  <button onclick="tab('sys',this)">System</button>
</div>
<main>
  <section id="edit" class="view on">
    <textarea id="code">table Trades { sym: Str, qty: Int, buy: Bool }
insert Trades { sym: "AAPL", qty: 10, buy: true }
insert Trades { sym: "BTC", qty: 2, buy: false }

fn total(a, b) { a + b }
let buys = from t in Trades where t.buy select t.sym
print("buys:", buys)</textarea>
    <div class="row"><button class="act" onclick="run()">Run</button>
      <button class="alt" onclick="tab('bp',document.querySelectorAll('.tabs button')[1]); draw()">Blueprint</button></div>
    <div class="lbl">Result</div><pre id="out">ready.</pre>
  </section>

  <section id="bp" class="view">
    <div class="lbl">Blueprint — live IR graph (pinch/drag to zoom & pan)</div>
    <canvas id="cv"></canvas>
    <div class="row"><button class="alt" onclick="draw()">Refresh from code</button>
      <button class="alt" onclick="fit()">Fit</button></div>
    <pre id="bpinfo">tap "Refresh from code" to render the IR of your program.</pre>
  </section>

  <section id="map" class="view">
    <div class="lbl">Codebase map — zoom to scale through the whole system (middle-out)</div>
    <canvas id="mv"></canvas>
    <div class="row"><button class="alt" onclick="loadMap()">Load repo</button>
      <button class="alt" onclick="mapZoom(1.4)">Zoom +</button>
      <button class="alt" onclick="mapZoom(0.7)">Zoom &minus;</button></div>
    <pre id="mapinfo">tap "Load repo" to map the entire codebase.</pre>
  </section>

  <section id="sys" class="view">
    <div class="lbl">System status — what is actually live</div>
    <div class="counts"><div><div class="n live" id="cl">-</div><div class="k">live</div></div>
      <div><div class="n building" id="cb">-</div><div class="k">building</div></div>
      <div><div class="n planned" id="cp">-</div><div class="k">planned</div></div></div>
    <div id="feats"></div>
  </section>
</main>
<script>
function tab(id, btn){ document.querySelectorAll('.view').forEach(v=>v.classList.remove('on'));
  document.getElementById(id).classList.add('on');
  document.querySelectorAll('.tabs button').forEach(b=>b.classList.remove('on')); if(btn)btn.classList.add('on');
  if(id==='sys') loadStatus(); if(id==='map'&&!MAP) loadMap(); }
async function post(p,code){ const r=await fetch(p,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({code})}); return r.json(); }
async function run(){ document.getElementById('out').textContent='running...';
  try{ const d=await post('/run',code.value); document.getElementById('out').textContent=d.ok?('=> '+d.result):('error: '+d.error);}catch(e){document.getElementById('out').textContent='err: '+e;} }
async function loadStatus(){ try{ const r=await fetch('/status'); const d=await r.json();
  cl.textContent=d.live; cb.textContent=d.building; cp.textContent=d.planned;
  feats.innerHTML=d.features.map(f=>`<div class="feat"><span>${f.name}</span><span class="badge ${f.status}">${f.status}</span></div>`).join('');
}catch(e){ feats.textContent='err: '+e; } }

// ---- Blueprint: engineering-drawing renderer over the real IR ----
let G={nodes:[],roots:[]}, view={x:30,y:60,s:1};
const cv=document.getElementById('cv'); const ctx=cv.getContext('2d');
// drafting palette per construct
const SYM={
  FN:       {shape:'ported', col:'#f78166', tag:'FUNC'},
  CALL:     {shape:'process',col:'#d29922', tag:'PROC'},
  QUERY:    {shape:'io',     col:'#a371f7', tag:'QUERY'},
  TABLE_DEF:{shape:'store',  col:'#56d4dd', tag:'TABLE'},
  RECORD:   {shape:'record', col:'#db61a2', tag:'REC'},
  MATCH:    {shape:'decision',col:'#e3b341',tag:'BRANCH'},
  LITERAL:  {shape:'terminal',col:'#3fb950',tag:'VAL'},
  REF:      {shape:'terminal',col:'#4a9eff',tag:'REF'},
  EFFECT:   {shape:'process',col:'#8b949e', tag:'EFF'},
};
const NW=140, NH=56, GAPX=190, GAPY=120;
async function draw(){ const d=await post('/ir',code.value);
  if(!d.ok){ document.getElementById('bpinfo').textContent='error: '+d.error; return; }
  G=d; layout(); fit(); render();
  document.getElementById('bpinfo').textContent=d.count+' components · '+d.roots.length+' roots · engineering view';
}
function layout(){ const byId={}; G.nodes.forEach(n=>byId[n.id]=n);
  const depth={}, seen={}; let q=G.roots.map(r=>[r,0]);
  while(q.length){ const [id,dp]=q.shift(); if(seen[id])continue; seen[id]=1; depth[id]=dp;
    (byId[id]?.children||[]).forEach(c=>{ if(!seen[c])q.push([c,dp+1]); }); }
  const rows={}; G.nodes.forEach(n=>{ const dp=depth[n.id]??0; (rows[dp]=rows[dp]||[]).push(n.id); });
  const pos={}; Object.keys(rows).forEach(dp=>{ rows[dp].forEach((id,i)=>{ pos[id]={x:i*GAPX,y:dp*GAPY}; }); });
  G.pos=pos; G.byId=byId;
}
function fit(){ if(!G.nodes.length)return; const xs=Object.values(G.pos).map(p=>p.x),ys=Object.values(G.pos).map(p=>p.y);
  const w=Math.max(...xs)+NW+60,h=Math.max(...ys)+NH+60; view.s=Math.min(cv.clientWidth/w,cv.clientHeight/h,1.3)||1;
  view.x=30; view.y=60; render(); }
function render(){ const dpr=window.devicePixelRatio||1; cv.width=cv.clientWidth*dpr; cv.height=cv.clientHeight*dpr;
  ctx.setTransform(dpr,0,0,dpr,0,0); ctx.clearRect(0,0,cv.clientWidth,cv.clientHeight);
  drawGrid(); drawTitleBlock();
  ctx.save(); ctx.translate(view.x,view.y); ctx.scale(view.s,view.s);
  // orthogonal ported connectors first (under nodes)
  G.nodes.forEach(n=>{ const a=G.pos[n.id]; if(!a)return;
    (n.children||[]).forEach(c=>{ const b=G.pos[c]; if(!b)return; connector(a,b); }); });
  // components
  G.nodes.forEach(n=>{ const p=G.pos[n.id]; if(!p)return; component(n,p); });
  ctx.restore();
  drawLegend();
}
function drawGrid(){ ctx.save(); ctx.strokeStyle='#131820'; ctx.lineWidth=1; const step=28*view.s;
  const ox=view.x%step, oy=view.y%step;
  for(let x=ox;x<cv.clientWidth;x+=step){ctx.beginPath();ctx.moveTo(x,0);ctx.lineTo(x,cv.clientHeight);ctx.stroke();}
  for(let y=oy;y<cv.clientHeight;y+=step){ctx.beginPath();ctx.moveTo(0,y);ctx.lineTo(cv.clientWidth,y);ctx.stroke();}
  ctx.restore(); }
function drawTitleBlock(){ ctx.save(); ctx.fillStyle='#0d1117'; ctx.strokeStyle='#30363d'; ctx.lineWidth=1;
  ctx.fillRect(0,0,cv.clientWidth,34); ctx.strokeRect(0,0,cv.clientWidth,34);
  ctx.fillStyle='#e6edf3'; ctx.font='bold 12px monospace'; ctx.fillText('CodeIO · BLUEPRINT', 10, 22);
  ctx.fillStyle='#8b949e'; ctx.font='10px monospace';
  ctx.fillText('components: '+(G.nodes?.length||0)+'   scale: '+view.s.toFixed(2)+'x   sheet 1/1', cv.clientWidth-260, 22);
  ctx.restore(); }
function drawLegend(){ const items=[['FUNC','#f78166'],['PROC','#d29922'],['QUERY','#a371f7'],['TABLE','#56d4dd'],['BRANCH','#e3b341'],['VAL/REF','#3fb950']];
  ctx.save(); const bx=8,by=cv.clientHeight-18-items.length*14, bw=110, bh=items.length*14+10;
  ctx.fillStyle='rgba(13,17,23,0.9)'; ctx.strokeStyle='#30363d'; ctx.fillRect(bx,by,bw,bh); ctx.strokeRect(bx,by,bw,bh);
  ctx.font='9px monospace'; items.forEach((it,i)=>{ ctx.fillStyle=it[1]; ctx.fillRect(bx+6,by+8+i*14,10,8);
    ctx.fillStyle='#c9d1d9'; ctx.fillText(it[0], bx+22, by+16+i*14); }); ctx.restore(); }
// orthogonal connector: output port (bottom-center of parent) -> input port (top-center of child)
function connector(a,b){ const x1=a.x+NW/2, y1=a.y+NH, x2=b.x+NW/2, y2=b.y; const my=(y1+y2)/2;
  ctx.strokeStyle='#4a5568'; ctx.lineWidth=1.5; ctx.beginPath();
  ctx.moveTo(x1,y1); ctx.lineTo(x1,my); ctx.lineTo(x2,my); ctx.lineTo(x2,y2-6); ctx.stroke();
  // arrowhead
  ctx.fillStyle='#4a5568'; ctx.beginPath(); ctx.moveTo(x2,y2); ctx.lineTo(x2-4,y2-7); ctx.lineTo(x2+4,y2-7); ctx.closePath(); ctx.fill(); }
function component(n,p){ const s=SYM[n.kind]||{shape:'process',col:'#8b949e',tag:n.kind};
  ctx.lineWidth=2; ctx.strokeStyle=s.col; ctx.fillStyle='#161b22';
  const x=p.x,y=p.y,w=NW,h=NH;
  switch(s.shape){
    case 'store': // data-store cylinder
      ctx.beginPath(); ctx.ellipse(x+w/2,y+8,w/2,8,0,0,Math.PI*2); ctx.moveTo(x,y+8); ctx.lineTo(x,y+h-8);
      ctx.ellipse(x+w/2,y+h-8,w/2,8,0,0,Math.PI); ctx.lineTo(x+w,y+8); ctx.fill(); ctx.stroke();
      ctx.beginPath(); ctx.ellipse(x+w/2,y+8,w/2,8,0,0,Math.PI*2); ctx.stroke(); break;
    case 'decision': // diamond
      ctx.beginPath(); ctx.moveTo(x+w/2,y); ctx.lineTo(x+w,y+h/2); ctx.lineTo(x+w/2,y+h); ctx.lineTo(x,y+h/2); ctx.closePath(); ctx.fill(); ctx.stroke(); break;
    case 'io': case 'query': // parallelogram
      ctx.beginPath(); ctx.moveTo(x+16,y); ctx.lineTo(x+w,y); ctx.lineTo(x+w-16,y+h); ctx.lineTo(x,y+h); ctx.closePath(); ctx.fill(); ctx.stroke(); break;
    case 'terminal': // rounded pill
      rr(x,y,w,h,h/2); ctx.fill(); ctx.stroke(); break;
    case 'ported': // function block with ports
      rr(x,y,w,h,6); ctx.fill(); ctx.stroke();
      ctx.fillStyle=s.col; ctx.fillRect(x-4,y+h/2-5,4,10); ctx.fillRect(x+w,y+h/2-5,4,10); break;
    default: // process rectangle
      rr(x,y,w,h,4); ctx.fill(); ctx.stroke();
  }
  ctx.fillStyle=s.col; ctx.font='bold 9px monospace'; ctx.fillText(s.tag, x+10, y+16);
  ctx.fillStyle='#e6edf3'; ctx.font='11px monospace'; ctx.fillText((n.label||'').slice(0,15), x+10, y+34);
}
function rr(x,y,w,h,r){ ctx.beginPath(); ctx.moveTo(x+r,y); ctx.arcTo(x+w,y,x+w,y+h,r);
  ctx.arcTo(x+w,y+h,x,y+h,r); ctx.arcTo(x,y+h,x,y,r); ctx.arcTo(x,y,x+w,y,r); ctx.closePath(); }
let drag=null,pinch=null;
cv.addEventListener('touchstart',e=>{ if(e.touches.length===1)drag={x:e.touches[0].clientX-view.x,y:e.touches[0].clientY-view.y};
  else if(e.touches.length===2)pinch=dist(e); },{passive:true});
cv.addEventListener('touchmove',e=>{ if(e.touches.length===1&&drag){view.x=e.touches[0].clientX-drag.x;view.y=e.touches[0].clientY-drag.y;render();}
  else if(e.touches.length===2&&pinch){const d=dist(e);view.s=Math.max(0.2,Math.min(3,view.s*d/pinch));pinch=d;render();} },{passive:true});
cv.addEventListener('touchend',()=>{drag=null;pinch=null;});
function dist(e){ const dx=e.touches[0].clientX-e.touches[1].clientX,dy=e.touches[0].clientY-e.touches[1].clientY; return Math.hypot(dx,dy); }

// ===== Codebase map: camera-based semantic zoom over the WHOLE repo =====
let MAP=null, cam={x:0,y:0,z:0.15};
const mv=document.getElementById('mv'), mctx=mv.getContext('2d');
async function loadMap(){ document.getElementById('mapinfo').textContent='mapping repo...';
  try{ const r=await fetch('/map'); const d=await r.json(); MAP=layoutMap(d.crates); centerMap(); renderMap();
    document.getElementById('mapinfo').textContent=MAP.crates.length+' crates · '+MAP.fileCount+' files · '+MAP.symCount+' symbols — pinch to scale in/out';
  }catch(e){ document.getElementById('mapinfo').textContent='err: '+e; } }
function layoutMap(crates){ let fileCount=0, symCount=0;
  const cols=Math.ceil(Math.sqrt(crates.length)); const CW=520, CH=380, GAP=40;
  const laidCrates=crates.map((c,i)=>{ const cx=(i%cols)*(CW+GAP), cy=Math.floor(i/cols)*(CH+GAP);
    const files=c.files; fileCount+=files.length;
    const fcols=Math.max(1,Math.ceil(Math.sqrt(files.length))); const FW=(CW-40)/fcols;
    const laidFiles=files.map((f,fi)=>{ const fx=cx+20+(fi%fcols)*FW, fy=cy+50+Math.floor(fi/fcols)*90;
      symCount+=f.symbols.length;
      const scols=Math.max(1,Math.ceil(Math.sqrt(Math.max(1,f.symbols.length)))); const SW=(FW-10)/scols;
      const laidSyms=f.symbols.map((s,si)=>({name:s.name,kind:s.kind, x:fx+4+(si%scols)*SW, y:fy+34+Math.floor(si/scols)*16, w:SW-3}));
      return {name:f.name,loc:f.loc, x:fx, y:fy, w:FW-8, h:80, syms:laidSyms}; });
    return {name:c.name, x:cx, y:cy, w:CW, h:CH, laidFiles}; });
  return {crates:laidCrates, fileCount, symCount, cols, CW, CH, GAP}; }
function centerMap(){ if(!MAP)return; const cols=MAP.cols;
  const totalW=cols*(MAP.CW+MAP.GAP), totalH=Math.ceil(MAP.crates.length/cols)*(MAP.CH+MAP.GAP);
  cam.z=Math.min(mv.clientWidth/totalW, mv.clientHeight/totalH)*0.9;
  cam.x=(mv.clientWidth-totalW*cam.z)/2; cam.y=(mv.clientHeight-totalH*cam.z)/2; }
const CKIND={fn:'#d29922',struct:'#56d4dd',enum:'#a371f7',trait:'#f78166'};
function renderMap(){ if(!MAP)return; const dpr=window.devicePixelRatio||1;
  mv.width=mv.clientWidth*dpr; mv.height=mv.clientHeight*dpr; mctx.setTransform(dpr,0,0,dpr,0,0);
  mctx.clearRect(0,0,mv.clientWidth,mv.clientHeight);
  mctx.save(); mctx.translate(cam.x,cam.y); mctx.scale(cam.z,cam.z); const z=cam.z;
  MAP.crates.forEach(c=>{ mctx.fillStyle='#11161d'; mctx.strokeStyle='#30363d'; mctx.lineWidth=2/z;
    mctx.fillRect(c.x,c.y,c.w,c.h); mctx.strokeRect(c.x,c.y,c.w,c.h);
    mctx.fillStyle='#4a9eff'; mctx.font='bold 18px monospace'; mctx.fillText(c.name, c.x+14, c.y+30);
    if(z>0.12){ c.laidFiles.forEach(f=>{ mctx.fillStyle='#161b22'; mctx.strokeStyle='#3a4150'; mctx.lineWidth=1/z;
      mctx.fillRect(f.x,f.y,f.w,f.h); mctx.strokeRect(f.x,f.y,f.w,f.h);
      mctx.fillStyle='#c9d1d9'; mctx.font='11px monospace'; mctx.fillText(f.name+'  ·  '+f.loc+' LOC', f.x+6, f.y+16);
      if(z>0.35){ f.syms.forEach(s=>{ mctx.fillStyle=CKIND[s.kind]||'#8b949e';
        mctx.fillRect(s.x,s.y,Math.max(6,s.w),12);
        if(z>0.7){ mctx.fillStyle='#0d1117'; mctx.font='8px monospace'; mctx.fillText((s.name||'').slice(0,Math.floor(s.w/5)), s.x+2, s.y+9); } }); }
    }); } });
  mctx.restore();
  mctx.fillStyle='#0d1117'; mctx.fillRect(0,0,mv.clientWidth,26); mctx.strokeStyle='#30363d'; mctx.strokeRect(0,0,mv.clientWidth,26);
  const lvl = z<0.12?'SYSTEM (crates)': z<0.35?'MODULE (files)': z<0.7?'SYMBOLS':'DETAIL';
  mctx.fillStyle='#e6edf3'; mctx.font='bold 11px monospace'; mctx.fillText('LEVEL: '+lvl+'   scale '+z.toFixed(2)+'x', 10, 18); }
function mapZoom(f){ const cx=mv.clientWidth/2, cy=mv.clientHeight/2;
  cam.x=cx-(cx-cam.x)*f; cam.y=cy-(cy-cam.y)*f; cam.z*=f; renderMap(); }
let mdrag=null,mpinch=null;
mv.addEventListener('touchstart',e=>{ if(e.touches.length===1)mdrag={x:e.touches[0].clientX-cam.x,y:e.touches[0].clientY-cam.y};
  else if(e.touches.length===2)mpinch=dist(e); },{passive:true});
mv.addEventListener('touchmove',e=>{ if(e.touches.length===1&&mdrag){cam.x=e.touches[0].clientX-mdrag.x;cam.y=e.touches[0].clientY-mdrag.y;renderMap();}
  else if(e.touches.length===2&&mpinch){ const d=dist(e); const f=d/mpinch;
    const mx=(e.touches[0].clientX+e.touches[1].clientX)/2, my=(e.touches[0].clientY+e.touches[1].clientY)/2;
    cam.x=mx-(mx-cam.x)*f; cam.y=my-(my-cam.y)*f; cam.z=Math.max(0.04,Math.min(2,cam.z*f)); mpinch=d; renderMap(); } },{passive:true});
mv.addEventListener('touchend',()=>{mdrag=null;mpinch=null;});

</script>
</body></html>"#;
