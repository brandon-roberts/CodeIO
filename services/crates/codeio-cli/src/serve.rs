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
  if(id==='sys') loadStatus(); }
async function post(p,code){ const r=await fetch(p,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({code})}); return r.json(); }
async function run(){ document.getElementById('out').textContent='running...';
  try{ const d=await post('/run',code.value); document.getElementById('out').textContent=d.ok?('=> '+d.result):('error: '+d.error);}catch(e){document.getElementById('out').textContent='err: '+e;} }
async function loadStatus(){ try{ const r=await fetch('/status'); const d=await r.json();
  cl.textContent=d.live; cb.textContent=d.building; cp.textContent=d.planned;
  feats.innerHTML=d.features.map(f=>`<div class="feat"><span>${f.name}</span><span class="badge ${f.status}">${f.status}</span></div>`).join('');
}catch(e){ feats.textContent='err: '+e; } }

// ---- Blueprint canvas: real IR graph ----
let G={nodes:[],roots:[]}, view={x:20,y:20,s:1};
const cv=document.getElementById('cv'); const ctx=cv.getContext('2d');
const KCOL={LITERAL:'#3fb950',REF:'#4a9eff',CALL:'#d29922',FN:'#f78166',QUERY:'#a371f7',TABLE_DEF:'#56d4dd',RECORD:'#db61a2',EFFECT:'#8b949e',MATCH:'#e3b341'};
async function draw(){ const d=await post('/ir',code.value);
  if(!d.ok){ document.getElementById('bpinfo').textContent='error: '+d.error; return; }
  G=d; layout(); fit(); render();
  document.getElementById('bpinfo').textContent=d.count+' IR nodes, '+d.roots.length+' roots — colored by kind';
}
function layout(){ // simple layered layout by BFS depth from roots
  const pos={}, depth={}, byId={}; G.nodes.forEach(n=>byId[n.id]=n);
  let q=G.roots.map(r=>[r,0]); const seen={};
  while(q.length){ const [id,d]=q.shift(); if(seen[id])continue; seen[id]=1; depth[id]=d;
    (byId[id]?.children||[]).forEach(c=>{ if(!seen[c]) q.push([c,d+1]); }); }
  const rows={}; G.nodes.forEach(n=>{ const d=depth[n.id]??0; (rows[d]=rows[d]||[]).push(n.id); });
  Object.keys(rows).forEach(d=>{ rows[d].forEach((id,i)=>{ pos[id]={x:i*150,y:d*90}; }); });
  G.pos=pos; G.byId=byId;
}
function fit(){ if(!G.nodes.length)return; let xs=Object.values(G.pos).map(p=>p.x), ys=Object.values(G.pos).map(p=>p.y);
  const w=Math.max(...xs)+140, h=Math.max(...ys)+80; view.s=Math.min(cv.width/w, cv.height/h, 1.2)||1;
  view.x=20; view.y=20; render(); }
function render(){ const dpr=window.devicePixelRatio||1; cv.width=cv.clientWidth*dpr; cv.height=cv.clientHeight*dpr;
  ctx.setTransform(dpr,0,0,dpr,0,0); ctx.clearRect(0,0,cv.width,cv.height);
  ctx.save(); ctx.translate(view.x,view.y); ctx.scale(view.s,view.s);
  // edges
  ctx.strokeStyle='#30363d'; ctx.lineWidth=1.5;
  G.nodes.forEach(n=>{ const a=G.pos[n.id]; (n.children||[]).forEach(c=>{ const b=G.pos[c]; if(a&&b){
    ctx.beginPath(); ctx.moveTo(a.x+60,a.y+30); ctx.lineTo(b.x+60,b.y); ctx.stroke(); }}); });
  // nodes
  G.nodes.forEach(n=>{ const p=G.pos[n.id]; if(!p)return; const col=KCOL[n.kind]||'#8b949e';
    ctx.fillStyle='#161b22'; ctx.strokeStyle=col; ctx.lineWidth=2;
    roundRect(p.x,p.y,120,44,8); ctx.fill(); ctx.stroke();
    ctx.fillStyle=col; ctx.font='bold 10px monospace'; ctx.fillText(n.kind, p.x+8, p.y+16);
    ctx.fillStyle='#e6edf3'; ctx.font='11px monospace';
    ctx.fillText((n.label||'').slice(0,14), p.x+8, p.y+32); });
  ctx.restore();
}
function roundRect(x,y,w,h,r){ ctx.beginPath(); ctx.moveTo(x+r,y); ctx.arcTo(x+w,y,x+w,y+h,r);
  ctx.arcTo(x+w,y+h,x,y+h,r); ctx.arcTo(x,y+h,x,y,r); ctx.arcTo(x,y,x+w,y,r); ctx.closePath(); }
// pan & pinch-zoom
let drag=null, pinch=null;
cv.addEventListener('touchstart',e=>{ if(e.touches.length===1){drag={x:e.touches[0].clientX-view.x,y:e.touches[0].clientY-view.y};}
  else if(e.touches.length===2){ pinch=dist(e); } },{passive:true});
cv.addEventListener('touchmove',e=>{ if(e.touches.length===1&&drag){ view.x=e.touches[0].clientX-drag.x; view.y=e.touches[0].clientY-drag.y; render(); }
  else if(e.touches.length===2&&pinch){ const d=dist(e); view.s*=d/pinch; pinch=d; render(); } },{passive:true});
cv.addEventListener('touchend',()=>{drag=null;pinch=null;});
function dist(e){ const dx=e.touches[0].clientX-e.touches[1].clientX, dy=e.touches[0].clientY-e.touches[1].clientY; return Math.hypot(dx,dy); }
</script>
</body></html>"#;
