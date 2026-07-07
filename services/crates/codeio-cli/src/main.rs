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
        Cmd::Features { status } => features(&root, status.as_deref()),
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
