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
    let mut hist = String::from("{");
    for (i, (k, c)) in g.kind_histogram().iter().enumerate() {
        if i > 0 { hist.push(','); }
        hist.push_str(&format!("\"{k}\":{c}"));
    }
    hist.push('}');
    format!("{{\"ok\":true,\"nodes\":{},\"roots\":{},\"histogram\":{}}}", g.len(), g.roots.len(), hist)
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CodeIO</title>
<style>
  :root { --bg:#0d1117; --panel:#161b22; --border:#30363d; --fg:#e6edf3; --accent:#4a9eff; --muted:#8b949e; }
  * { box-sizing:border-box; }
  body { margin:0; font-family:ui-monospace,SFMono-Regular,Menlo,monospace; background:var(--bg); color:var(--fg); }
  header { padding:14px 16px; border-bottom:1px solid var(--border); display:flex; align-items:center; gap:10px; }
  header h1 { font-size:16px; margin:0; letter-spacing:1px; }
  header .tag { color:var(--muted); font-size:12px; }
  main { padding:12px; max-width:900px; margin:0 auto; }
  textarea { width:100%; height:38vh; background:var(--panel); color:var(--fg); border:1px solid var(--border);
             border-radius:8px; padding:12px; font:inherit; font-size:14px; resize:vertical; }
  .row { display:flex; gap:8px; margin:10px 0; }
  button { flex:1; padding:12px; background:var(--accent); color:#001; border:0; border-radius:8px;
           font:inherit; font-weight:700; font-size:14px; }
  button.alt { background:var(--panel); color:var(--fg); border:1px solid var(--border); }
  pre { background:var(--panel); border:1px solid var(--border); border-radius:8px; padding:12px;
        white-space:pre-wrap; word-break:break-word; min-height:60px; font-size:13px; }
  .lbl { color:var(--muted); font-size:11px; text-transform:uppercase; letter-spacing:1px; margin:12px 0 4px; }
</style></head>
<body>
<header><h1>CodeIO</h1><span class="tag">universal language · running live</span></header>
<main>
  <textarea id="code">table Trades { sym: Str, qty: Int, buy: Bool }
insert Trades { sym: "AAPL", qty: 10, buy: true }
insert Trades { sym: "BTC", qty: 2, buy: false }

let buys = from t in Trades where t.buy select t.sym
print("buys:", buys)</textarea>
  <div class="row">
    <button onclick="run()">Run</button>
    <button class="alt" onclick="showIr()">View IR</button>
  </div>
  <div class="lbl">Result</div>
  <pre id="out">ready.</pre>
</main>
<script>
async function post(path, code) {
  const r = await fetch(path, {method:'POST', headers:{'Content-Type':'application/json'},
    body: JSON.stringify({code})});
  return r.json();
}
async function run() {
  document.getElementById('out').textContent = 'running...';
  try { const d = await post('/run', document.getElementById('code').value);
    document.getElementById('out').textContent = d.ok ? ('=> ' + d.result) : ('error: ' + d.error);
  } catch(e){ document.getElementById('out').textContent = 'connection error: ' + e; }
}
async function showIr() {
  document.getElementById('out').textContent = 'lowering to IR...';
  try { const d = await post('/ir', document.getElementById('code').value);
    if(!d.ok){ document.getElementById('out').textContent='error: '+d.error; return; }
    let s = 'IR graph: ' + d.nodes + ' content-addressed nodes, ' + d.roots + ' roots\n\n';
    for (const [k,v] of Object.entries(d.histogram)) s += '  ' + k.padEnd(11) + v + '\n';
    document.getElementById('out').textContent = s;
  } catch(e){ document.getElementById('out').textContent = 'connection error: ' + e; }
}
</script>
</body></html>"#;
