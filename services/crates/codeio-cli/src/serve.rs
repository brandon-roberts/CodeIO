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

const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no">
<title>CodeIO Blueprint</title>
<style>
  :root{ --blue:#1a3a6b; --blue2:#12294d; --ink:#dbe7ff; --line:#9fc1ff; --line2:#5d84c4; --accent:#ffd479; }
  html,body{margin:0;padding:0;height:100%;background:#0b1c38;color:var(--ink);overflow:hidden;
    font-family:ui-monospace,"Courier New",monospace;-webkit-user-select:none;user-select:none;}
  #wrap{position:fixed;inset:0;display:flex;flex-direction:column;}
  #bar{height:44px;display:flex;align-items:center;gap:8px;padding:0 10px;background:#0b1c38;border-bottom:1px solid var(--line2);z-index:5;}
  #bar h1{font-size:14px;margin:0;letter-spacing:2px;color:var(--line);}
  #bar .sp{flex:1;}
  #bar button{background:var(--blue);color:var(--ink);border:1px solid var(--line2);border-radius:5px;
    padding:7px 10px;font:inherit;font-size:12px;}
  #sheet{flex:1;position:relative;}
  canvas{position:absolute;inset:0;width:100%;height:100%;touch-action:none;display:block;}
  #editPane{position:absolute;left:0;right:0;bottom:0;height:40%;background:#0b1c38;border-top:2px solid var(--line2);
    transform:translateY(100%);transition:transform .2s;z-index:6;display:flex;flex-direction:column;}
  #editPane.open{transform:translateY(0);}
  #editPane textarea{flex:1;margin:8px;background:#12294d;color:var(--ink);border:1px solid var(--line2);
    border-radius:6px;padding:10px;font:inherit;font-size:15px;resize:none;}
  #editPane .er{display:flex;gap:8px;padding:0 8px 8px;}
  #editPane .er button{flex:1;padding:10px;border-radius:6px;border:1px solid var(--line2);background:var(--blue);color:var(--ink);font:inherit;font-weight:700;}
  #out{position:absolute;left:14px;top:10px;max-width:60%;background:rgba(11,28,56,.9);border:1px solid var(--line2);
    border-radius:6px;padding:6px 10px;font-size:11px;color:var(--ink);z-index:4;pointer-events:none;
    opacity:0;transition:opacity .3s;}
  #out.show{opacity:1;}
</style></head>
<body><div id="wrap">
  <div id="bar"><h1>CodeIO &middot; BLUEPRINT</h1><span class="sp"></span>
    <button onclick="zoomBy(1.25)">+</button>
    <button onclick="zoomBy(0.8)">&minus;</button>
    <button onclick="fitAll()">FIT</button>
    <button onclick="toggleEdit()">CODE</button>
  </div>
  <div id="sheet"><canvas id="c"></canvas><div id="out">loading blueprint&hellip;</div></div>
  <div id="editPane"><textarea id="code">table Trades { sym: Str, qty: Int, buy: Bool }
insert Trades { sym: "AAPL", qty: 10, buy: true }

fn total(a, b) { a + b }
let buys = from t in Trades where t.buy select t.sym
print("buys:", buys)</textarea>
    <div class="er"><button onclick="runCode()">RUN</button>
      <button onclick="renderLogic()">DRAW LOGIC</button>
      <button onclick="toggleEdit()">CLOSE</button></div>
  </div>
</div>
<script>
const c=document.getElementById('c'), ctx=c.getContext('2d');
const code=document.getElementById('code'), outEl=document.getElementById('out');
let cam={x:0,y:0,z:0.3}, world={crates:[],mode:'system',logic:null};
let outTimer=null; function out(s){ outEl.textContent=s; outEl.classList.add('show');
  clearTimeout(outTimer); outTimer=setTimeout(()=>outEl.classList.remove('show'),2500); }
function toggleEdit(){ document.getElementById('editPane').classList.toggle('open'); }

// ---------- data ----------
async function loadRepo(){ out('mapping system\u2026');
  const r=await fetch('/map'); const d=await r.json(); world.crates=layoutRepo(d.crates); fitAll();
  out(world.crates.length+' modules \u00b7 pinch to scale through the system'); }
function layoutRepo(crates){ const cols=Math.ceil(Math.sqrt(crates.length)); const CW=560,CH=420,G=70;
  let fc=0,sc=0;
  const out=crates.map((c,i)=>{ const x=(i%cols)*(CW+G), y=Math.floor(i/cols)*(CH+G);
    const files=c.files; fc+=files.length;
    const fcols=Math.max(1,Math.ceil(Math.sqrt(files.length))), FW=(CW-50)/fcols;
    const lf=files.map((f,fi)=>{ const fx=x+25+(fi%fcols)*FW, fy=y+70+Math.floor(fi/fcols)*100; sc+=f.symbols.length;
      const scol=Math.max(1,Math.ceil(Math.sqrt(Math.max(1,f.symbols.length)))), SW=(FW-14)/scol;
      const ls=f.symbols.map((s,si)=>({name:s.name,kind:s.kind,x:fx+6+(si%scol)*SW,y:fy+40+Math.floor(si/scol)*20,w:SW-4}));
      return {name:f.name,loc:f.loc,x:fx,y:fy,w:FW-10,h:88,syms:ls}; });
    return {name:c.name,x,y,w:CW,h:CH,files:lf}; });
  world._fc=fc; world._sc=sc; return out; }

// ---------- camera ----------
function fitAll(){ if(!world.crates.length)return; const cols=Math.ceil(Math.sqrt(world.crates.length));
  const W=cols*630, H=Math.ceil(world.crates.length/cols)*490;
  cam.z=Math.min(c.clientWidth/W, c.clientHeight/H)*0.92; cam.x=(c.clientWidth-W*cam.z)/2; cam.y=(c.clientHeight-H*cam.z)/2; render(); }
function zoomBy(f){ const mx=c.clientWidth/2,my=c.clientHeight/2; cam.x=mx-(mx-cam.x)*f; cam.y=my-(my-cam.y)*f;
  cam.z=Math.max(0.02,Math.min(6,cam.z*f)); render(); }

// ---------- drafting primitives ----------
function line(x1,y1,x2,y2,w,dash){ ctx.save(); ctx.lineWidth=w/cam.z; if(dash)ctx.setLineDash(dash.map(d=>d/cam.z));
  ctx.strokeStyle='#9fc1ff'; ctx.beginPath(); ctx.moveTo(x1,y1); ctx.lineTo(x2,y2); ctx.stroke(); ctx.restore(); }
function box(x,y,w,h,lw){ ctx.lineWidth=lw/cam.z; ctx.strokeStyle='#bcd6ff'; ctx.strokeRect(x,y,w,h); }
function label(txt,x,y,size,col){ ctx.fillStyle=col||'#dbe7ff'; ctx.font='bold '+size+'px "Courier New",monospace';
  ctx.fillText(txt,x,y); }

// ---------- render: one sheet, scale-through ----------
function render(){ const dpr=window.devicePixelRatio||1; c.width=c.clientWidth*dpr; c.height=c.clientHeight*dpr;
  ctx.setTransform(dpr,0,0,dpr,0,0);
  // blueprint paper
  ctx.fillStyle='#12345f'; ctx.fillRect(0,0,c.clientWidth,c.clientHeight);
  drawPaperGrid();
  if(world.mode==='logic' && world.logic){ drawLogicSheet(); }
  else { drawSystemSheet(); }
  drawBorderAndTitle();
}
function drawPaperGrid(){ ctx.save(); ctx.strokeStyle='rgba(159,193,255,0.10)'; ctx.lineWidth=1;
  const step=26; const ox=(cam.x%step+step)%step, oy=(cam.y%step+step)%step;
  for(let x=ox;x<c.clientWidth;x+=step){ctx.beginPath();ctx.moveTo(x,0);ctx.lineTo(x,c.clientHeight);ctx.stroke();}
  for(let y=oy;y<c.clientHeight;y+=step){ctx.beginPath();ctx.moveTo(0,y);ctx.lineTo(c.clientWidth,y);ctx.stroke();}
  ctx.restore(); }
function drawSystemSheet(){ ctx.save(); ctx.translate(cam.x,cam.y); ctx.scale(cam.z,cam.z); const z=cam.z;
  world.crates.forEach(cr=>{
    // module: heavy border box (title block style)
    box(cr.x,cr.y,cr.w,cr.h,2.5);
    // module title bar
    ctx.fillStyle='rgba(159,193,255,0.08)'; ctx.fillRect(cr.x,cr.y,cr.w,44);
    line(cr.x,cr.y+44,cr.x+cr.w,cr.y+44,1.5);
    label('MODULE',cr.x+12,cr.y+18,13,'#7fa8e8'); label(cr.name.toUpperCase(),cr.x+12,cr.y+37,20,'#ffe6a8');
    if(z>0.10){ cr.files.forEach(f=>{ box(f.x,f.y,f.w,f.h,1.2);
      ctx.fillStyle='rgba(159,193,255,0.05)'; ctx.fillRect(f.x,f.y,f.w,22);
      label(f.name,f.x+6,f.y+15,12,'#cfe0ff'); label(f.loc+' LOC',f.x+f.w-64,f.y+15,10,'#7fa8e8');
      if(z>0.32){ f.syms.forEach(s=>{ drawSymbolCell(s); }); }
    }); }
  });
  ctx.restore(); }
const SK={fn:'#ffd479',struct:'#7fd6dd',enum:'#c39bff',trait:'#ff9e80'};
function drawSymbolCell(s){ const col=SK[s.kind]||'#9fc1ff'; ctx.lineWidth=1/cam.z; ctx.strokeStyle=col;
  ctx.strokeRect(s.x,s.y,Math.max(8,s.w),15);
  // kind glyph
  ctx.fillStyle=col;
  if(s.kind==='fn'){ ctx.fillRect(s.x+2,s.y+5,4,4); }
  else if(s.kind==='struct'){ ctx.strokeRect(s.x+2,s.y+4,5,6); }
  else { ctx.beginPath(); ctx.arc(s.x+4,s.y+7,2.5,0,7); ctx.stroke(); }
  if(cam.z>0.6){ ctx.fillStyle='#e8f0ff'; ctx.font='9px "Courier New",monospace';
    ctx.fillText((s.name||'').slice(0,Math.floor(s.w/6)), s.x+10, s.y+11); } }

// ---------- logic sheet (a program's algorithm, drafted) ----------
function drawLogicSheet(){ const g=world.logic; ctx.save(); ctx.translate(cam.x,cam.y); ctx.scale(cam.z,cam.z);
  // connectors
  g.nodes.forEach(n=>{ const a=g.pos[n.id]; if(!a)return; (n.children||[]).forEach(cid=>{ const b=g.pos[cid]; if(!b)return;
    const x1=a.x+a.w/2,y1=a.y+a.h,x2=b.x+b.w/2,y2=b.y,my=(y1+y2)/2;
    ctx.strokeStyle='#7fa8e8'; ctx.lineWidth=1.4/cam.z; ctx.beginPath();
    ctx.moveTo(x1,y1);ctx.lineTo(x1,my);ctx.lineTo(x2,my);ctx.lineTo(x2,y2-6);ctx.stroke();
    ctx.fillStyle='#7fa8e8'; ctx.beginPath(); ctx.moveTo(x2,y2);ctx.lineTo(x2-4,y2-7);ctx.lineTo(x2+4,y2-7);ctx.closePath();ctx.fill(); }); });
  g.nodes.forEach(n=>{ const p=g.pos[n.id]; if(p) drawLogicSymbol(n,p); });
  ctx.restore(); }
const LSYM={FN:['FUNC','#ffd479','ported'],CALL:['PROC','#ffb84d','proc'],QUERY:['QUERY','#c39bff','para'],
  TABLE_DEF:['TABLE','#7fd6dd','store'],RECORD:['REC','#ff9ecb','rec'],MATCH:['BRANCH','#ffe08a','dia'],
  LITERAL:['VAL','#8de6a0','pill'],REF:['REF','#9fc1ff','pill'],EFFECT:['EFF','#9fc1ff','proc']};
function drawLogicSymbol(n,p){ const d=LSYM[n.kind]||['?','#9fc1ff','proc']; const x=p.x,y=p.y,w=p.w,h=p.h;
  ctx.lineWidth=2/cam.z; ctx.strokeStyle=d[1]; ctx.fillStyle='rgba(18,52,95,0.9)';
  if(d[2]==='store'){ ctx.beginPath();ctx.ellipse(x+w/2,y+8,w/2,8,0,0,7);ctx.moveTo(x,y+8);ctx.lineTo(x,y+h-8);
      ctx.ellipse(x+w/2,y+h-8,w/2,8,0,0,Math.PI);ctx.lineTo(x+w,y+8);ctx.fill();ctx.stroke(); }
  else if(d[2]==='dia'){ ctx.beginPath();ctx.moveTo(x+w/2,y);ctx.lineTo(x+w,y+h/2);ctx.lineTo(x+w/2,y+h);ctx.lineTo(x,y+h/2);ctx.closePath();ctx.fill();ctx.stroke(); }
  else if(d[2]==='para'){ ctx.beginPath();ctx.moveTo(x+14,y);ctx.lineTo(x+w,y);ctx.lineTo(x+w-14,y+h);ctx.lineTo(x,y+h);ctx.closePath();ctx.fill();ctx.stroke(); }
  else if(d[2]==='pill'){ rr(x,y,w,h,h/2);ctx.fill();ctx.stroke(); }
  else if(d[2]==='ported'){ rr(x,y,w,h,5);ctx.fill();ctx.stroke(); ctx.fillStyle=d[1];ctx.fillRect(x-4,y+h/2-4,4,8);ctx.fillRect(x+w,y+h/2-4,4,8); }
  else { rr(x,y,w,h,3);ctx.fill();ctx.stroke(); }
  ctx.fillStyle=d[1]; ctx.font='bold 9px "Courier New"'; ctx.fillText(d[0],x+8,y+15);
  ctx.fillStyle='#e8f0ff'; ctx.font='11px "Courier New"'; ctx.fillText((n.label||'').slice(0,15),x+8,y+32); }
function rr(x,y,w,h,r){ ctx.beginPath();ctx.moveTo(x+r,y);ctx.arcTo(x+w,y,x+w,y+h,r);ctx.arcTo(x+w,y+h,x,y+h,r);
  ctx.arcTo(x,y+h,x,y,r);ctx.arcTo(x,y,x+w,y,r);ctx.closePath(); }

// ---------- title block + border (drafting furniture) ----------
function drawBorderAndTitle(){ ctx.save();
  ctx.strokeStyle='#9fc1ff'; ctx.lineWidth=2; ctx.strokeRect(6,6,c.clientWidth-12,c.clientHeight-12);
  ctx.strokeRect(11,11,c.clientWidth-22,c.clientHeight-22);
  // corner coordinate ticks
  ctx.font='10px "Courier New"'; ctx.fillStyle='#7fa8e8';
  ['A','B','C','D'].forEach((L,i)=>ctx.fillText(L, 16, 40+i*((c.clientHeight-80)/4)));
  ['1','2','3','4'].forEach((N,i)=>ctx.fillText(N, 40+i*((c.clientWidth-80)/4), 22));
  // title block bottom-right
  const bw=230,bh=70,bx=c.clientWidth-bw-12,by=c.clientHeight-bh-12;
  ctx.fillStyle='#0b1c38'; ctx.fillRect(bx,by,bw,bh); ctx.strokeStyle='#9fc1ff'; ctx.lineWidth=1.5; ctx.strokeRect(bx,by,bw,bh);
  ctx.beginPath();ctx.moveTo(bx,by+24);ctx.lineTo(bx+bw,by+24);ctx.moveTo(bx,by+46);ctx.lineTo(bx+bw,by+46);
  ctx.moveTo(bx+140,by+24);ctx.lineTo(bx+140,by+bh);ctx.stroke();
  ctx.fillStyle='#ffe6a8'; ctx.font='bold 12px "Courier New"'; ctx.fillText('CodeIO SYSTEM', bx+8, by+17);
  const lvl = world.mode==='logic'?'LOGIC': cam.z<0.10?'SYSTEM': cam.z<0.32?'MODULE': cam.z<0.6?'SYMBOLS':'DETAIL';
  ctx.fillStyle='#dbe7ff'; ctx.font='10px "Courier New"';
  ctx.fillText('VIEW: '+lvl, bx+8, by+40); ctx.fillText('SCALE '+cam.z.toFixed(2)+'x', bx+8, by+62);
  ctx.fillText('DWG 001', bx+148, by+40); ctx.fillText('REV A', bx+148, by+62);
  ctx.restore(); }

// ---------- code exec / logic ----------
async function post(p,src){ const r=await fetch(p,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({code:src})}); return r.json(); }
async function runCode(){ out('running\u2026'); const d=await post('/run',code.value); out(d.ok?('=> '+d.result):('error: '+d.error)); }
async function renderLogic(){ const d=await post('/ir',code.value); if(!d.ok){ out('error: '+d.error); return; }
  layoutLogic(d); world.mode='logic'; toggleEdit(); fitLogic(); out('logic drawn \u00b7 '+d.count+' components \u2014 tap FIT / pinch to scale'); }
function layoutLogic(d){ const byId={}; d.nodes.forEach(n=>byId[n.id]=n); const depth={},seen={}; let q=d.roots.map(r=>[r,0]);
  while(q.length){ const [id,dp]=q.shift(); if(seen[id])continue; seen[id]=1; depth[id]=dp;
    (byId[id]?.children||[]).forEach(cc=>{ if(!seen[cc])q.push([cc,dp+1]); }); }
  const rows={}; d.nodes.forEach(n=>{ const dp=depth[n.id]??0;(rows[dp]=rows[dp]||[]).push(n.id); });
  const pos={}; Object.keys(rows).forEach(dp=>rows[dp].forEach((id,i)=>pos[id]={x:i*200,y:dp*130,w:140,h:56}));
  d.pos=pos; world.logic=d; }
function fitLogic(){ const g=world.logic; if(!g)return; const xs=Object.values(g.pos).map(p=>p.x),ys=Object.values(g.pos).map(p=>p.y);
  const W=Math.max(...xs)+200,H=Math.max(...ys)+120; cam.z=Math.min(c.clientWidth/W,c.clientHeight/H,1.2)||1;
  cam.x=(c.clientWidth-W*cam.z)/2; cam.y=40; render(); }

// ---------- input ----------
let drag=null,pinch=null;
c.addEventListener('touchstart',e=>{ if(e.touches.length===1)drag={x:e.touches[0].clientX-cam.x,y:e.touches[0].clientY-cam.y};
  else if(e.touches.length===2)pinch=dst(e); },{passive:true});
c.addEventListener('touchmove',e=>{ if(e.touches.length===1&&drag){cam.x=e.touches[0].clientX-drag.x;cam.y=e.touches[0].clientY-drag.y;render();}
  else if(e.touches.length===2&&pinch){ const d=dst(e),f=d/pinch,mx=(e.touches[0].clientX+e.touches[1].clientX)/2,my=(e.touches[0].clientY+e.touches[1].clientY)/2;
    cam.x=mx-(mx-cam.x)*f;cam.y=my-(my-cam.y)*f;cam.z=Math.max(0.02,Math.min(6,cam.z*f));pinch=d;render(); } },{passive:true});
c.addEventListener('touchend',()=>{drag=null;pinch=null;});
function dst(e){ return Math.hypot(e.touches[0].clientX-e.touches[1].clientX,e.touches[0].clientY-e.touches[1].clientY); }
// double-tap system<->reset
let lastTap=0; c.addEventListener('touchend',e=>{ const now=Date.now(); if(now-lastTap<300){ if(world.mode==='logic'){world.mode='system';fitAll();} else fitAll(); } lastTap=now; });
window.addEventListener('resize',render);
loadRepo();
</script>
</body></html>"##;
