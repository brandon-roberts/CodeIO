//! `codeio` — the single main entry point into the CodeIO system.
//!
//! Subcommands:
//!   codeio start      Launch all implemented services as child processes
//!   codeio status     Health-check every service port from config/codeio.toml
//!   codeio features   Show the feature registry with live/building/planned flags
//!
//! Future (per ROADMAP.md): `codeio run <file.cio>`, `codeio repl`, `codeio ide`.

use clap::{Parser, Subcommand};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "codeio", about = "CodeIO — universal language + frontier IDE", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Launch all implemented services
    Start,
    /// Health-check all configured services
    Status,
    /// Run a .cio program
    Run {
        /// Path to the .cio file
        file: PathBuf,
    },
    /// Start an interactive REPL
    Repl,
    /// Scan the environment: toolchains, services, AI backends — go/no-go report
    Doctor,
    /// Show feature registry (live / building / planned)
    Features {
        /// Only show features with this status
        #[arg(long)]
        status: Option<String>,
    },
}

/// Services implemented today, in start order: (name, crate, default port).
const IMPLEMENTED: &[(&str, &str, u16)] = &[
    ("index", "codeio-index", 50052),
    ("spotlight", "codeio-spotlight", 50053),
    ("context", "codeio-context", 50054),
    ("depmap", "codeio-depmap", 50055),
];

fn repo_root() -> PathBuf {
    // Walk up from the executable/cwd until we find features.toml.
    let mut dir = std::env::current_dir().expect("cwd");
    loop {
        if dir.join("features.toml").exists() {
            return dir;
        }
        if !dir.pop() {
            eprintln!("error: run `codeio` from inside the CodeIO repository");
            std::process::exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let root = repo_root();
    match cli.cmd {
        Cmd::Start => start(&root),
        Cmd::Status => status(),
        Cmd::Run { file } => run_file(&file),
        Cmd::Repl => repl(),
        Cmd::Doctor => doctor(),
        Cmd::Features { status } => features(&root, status.as_deref()),
    }
}

struct Check {
    name: &'static str,
    kind: &'static str, // required | optional | ai
    probe: &'static [&'static str],
    install_hint: &'static str,
}

const CHECKS: &[Check] = &[
    Check { name: "cargo (Rust)", kind: "required", probe: &["cargo", "--version"], install_hint: "https://rustup.rs  |  win: winget install Rustlang.Rustup" },
    Check { name: "protoc", kind: "required", probe: &["protoc", "--version"], install_hint: "apt install protobuf-compiler  |  win: winget install protobuf" },
    Check { name: "python3", kind: "required", probe: &["python3", "--version"], install_hint: "apt install python3  |  win: winget install Python.Python.3.12" },
    Check { name: "git", kind: "required", probe: &["git", "--version"], install_hint: "apt install git  |  win: winget install Git.Git" },
    Check { name: "node", kind: "optional", probe: &["node", "--version"], install_hint: "needed for IDE shell (M6): winget install OpenJS.NodeJS" },
    Check { name: "ghc (Haskell)", kind: "optional", probe: &["ghc", "--version"], install_hint: "needed for frontend layer: https://www.haskell.org/ghcup/" },
    Check { name: "g++ (C++)", kind: "optional", probe: &["g++", "--version"], install_hint: "needed for VM layer: apt install g++  |  win: VS Build Tools" },
];

fn probe_cmd(args: &[&str]) -> Option<String> {
    Command::new(args[0])
        .args(&args[1..])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            let s = if s.trim().is_empty() { String::from_utf8_lossy(&o.stderr).to_string() } else { s.to_string() };
            s.lines().next().unwrap_or("").trim().to_string()
        })
}

fn probe_http(addr: &str) -> bool {
    TcpStream::connect_timeout(&addr.parse().expect("addr"), Duration::from_millis(500)).is_ok()
}

fn doctor() {
    println!("CodeIO system doctor — environment scan
");
    let mut required_missing = 0;
    println!("── Toolchains ──");
    for c in CHECKS {
        match probe_cmd(c.probe) {
            Some(v) => println!("  ✅ {:<14} {}", c.name, v),
            None => {
                let mark = if c.kind == "required" { required_missing += 1; "❌" } else { "⚪" };
                println!("  {mark} {:<14} missing — {}", c.name, c.install_hint);
            }
        }
    }
    println!("
── AI backends ──");
    if probe_http("127.0.0.1:11434") {
        // try to list models via the API for a richer report
        match probe_cmd(&["ollama", "list"]) {
            Some(_) => println!("  ✅ ollama         serving on :11434 (run `ollama list` for models)"),
            None => println!("  ✅ ollama         serving on :11434"),
        }
    } else {
        println!("  ⚪ ollama         not detected on :11434 — https://ollama.com/download (win: winget install Ollama.Ollama)");
    }
    println!("
── CodeIO services ──");
    for (name, _, port) in IMPLEMENTED {
        let up = probe_http(&format!("127.0.0.1:{port}"));
        println!("  {} {name:<14} :{port}", if up { "✅" } else { "⚪" });
    }
    println!();
    if required_missing == 0 {
        println!("GO: all required tooling present. Optional items above unlock further layers.");
    } else {
        println!("NO-GO: {required_missing} required tool(s) missing — install hints above.");
        std::process::exit(1);
    }
}

fn run_file(file: &Path) {
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", file.display());
            std::process::exit(1);
        }
    };
    if let Err(e) = codeio_lang::run_source(&src) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn repl() {
    use std::io::{BufRead, Write};
    println!("CodeIO v0 REPL — type expressions; Ctrl-D to exit");
    let env = codeio_lang::interp::Env::root();
    let stdin = std::io::stdin();
    loop {
        print!("cio> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match codeio_lang::run_in(line, &env) {
                    Ok(v) => {
                        let s = v.to_string();
                        if s != "nil" {
                            println!("{s}");
                        }
                    }
                    Err(e) => println!("error: {e}"),
                }
            }
            Err(_) => break,
        }
    }
    println!();
}

fn start(root: &Path) {
    println!("Starting CodeIO services (Ctrl-C to stop)...");
    let mut children = Vec::new();
    for (name, krate, port) in IMPLEMENTED {
        let child = Command::new("cargo")
            .args(["run", "-q", "-p", krate])
            .current_dir(root.join("services"))
            .env("RUST_LOG", "info")
            .spawn();
        match child {
            Ok(c) => {
                println!("  ▶ {name:<10} :{port}  (pid {})", c.id());
                children.push(c);
            }
            Err(e) => eprintln!("  ✗ {name:<10} failed to spawn: {e}"),
        }
    }
    for mut c in children {
        let _ = c.wait();
    }
}

fn status() {
    println!("CodeIO service status:");
    for (name, _, port) in IMPLEMENTED {
        let addr = format!("127.0.0.1:{port}");
        let up = TcpStream::connect_timeout(
            &addr.parse().expect("valid addr"),
            Duration::from_millis(400),
        )
        .is_ok();
        println!("  {} {name:<10} :{port}", if up { "✅ UP  " } else { "❌ DOWN" });
    }
    println!("\n(Services not yet implemented — parse, typecheck, vm, meta, orchestrator — are tracked in FEATURES.md)");
}

fn features(root: &Path, filter: Option<&str>) {
    let text = std::fs::read_to_string(root.join("features.toml"))
        .expect("features.toml missing at repo root");
    let doc: toml::Value = text.parse().expect("features.toml is invalid TOML");
    let list = doc["feature"].as_array().expect("[[feature]] entries");
    let badge = |s: &str| match s {
        "live" => "✅ LIVE    ",
        "building" => "🚧 BUILDING",
        _ => "📋 PLANNED ",
    };
    let (mut live, mut building, mut planned) = (0, 0, 0);
    for f in list {
        let s = f["status"].as_str().unwrap_or("planned");
        match s {
            "live" => live += 1,
            "building" => building += 1,
            _ => planned += 1,
        }
        if filter.map_or(true, |want| want == s) {
            println!(
                "{} [{}] {} — {}",
                badge(s),
                f["pillar"].as_str().unwrap_or("?"),
                f["name"].as_str().unwrap_or("?"),
                f["desc"].as_str().unwrap_or("")
            );
        }
    }
    println!("\n{live} live · {building} building · {planned} planned  (source of truth: features.toml)");
}
