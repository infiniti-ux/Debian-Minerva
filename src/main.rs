//! Debian Minerva — session daemon.
//!
//! ```text
//! cargo build --release
//! ./target/release/minerva-sessiond --port 8080 --web-root ./web
//! ```
//!
//! The daemon owns the authoritative desktop state (windows, virtual desktops,
//! notifications, settings), serves the shell to a browser, and exposes a small
//! JSON API. No external crates: the whole DE system layer is Rust + std.

mod core;
mod exec;
mod http;
mod json;
mod services;

use core::SessionState;
use services::Vfs;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const BANNER: &str = r#"
     ______           _     _             __  __ _
    |  _  \         (_)   | |           |  \/  (_)
    | | | |___  __ _ _  __| |_ __   __ _| \  / |_ _ __   _____ _ __ __ _
    | | | / _ \/ _` | |/ _` | '_ \ / _` | |\/| | | '_ \ / _ \ \ /\ / / '__/ _` |
    | |/ /  __/ (_| | | (_| | | | | (_| | |  | | | | | |  __/ / / \ V /| | | (_| |
    |___/ \___|\__, |_|\__,_|_| |_|\__,_|_|  |_|_|_| |_|\___| \_/   \_/ |_|  \__,_|
                __/ |
               |___/     Debian Minerva · Rust desktop environment · v0.5.3
"#;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut port: u16 = 8080;
    let mut bind: String = "127.0.0.1".into();
    let mut token: Option<String> = None;
    let mut web_root: Option<PathBuf> = None;
    let mut data_dir: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => { print_help(); return; }
            "--version" | "-V" => { println!("minerva-sessiond {}", core::VERSION); return; }
            "--self-test" => { run_self_test(); return; }
            "--port" | "-p" => {
                i += 1;
                port = args.get(i).and_then(|s| s.parse().ok()).unwrap_or_else(|| {
                    eprintln!("minerva: invalid port");
                    std::process::exit(2);
                });
            }
            "--bind" => { i += 1; bind = args.get(i).cloned().unwrap_or_else(|| "127.0.0.1".into()); }
            "--token" => { i += 1; token = args.get(i).cloned(); }
            "--web-root" => { i += 1; web_root = args.get(i).map(PathBuf::from); }
            "--data-dir" => { i += 1; data_dir = args.get(i).map(PathBuf::from); }
            other => { eprintln!("minerva: unknown argument: {}", other); print_help(); std::process::exit(2); }
        }
        i += 1;
    }

    let web_root = web_root.unwrap_or_else(detect_web_root);
    let data_dir = data_dir.unwrap_or_else(default_data_dir);

    println!("{}", BANNER);
    println!("  version   : {}", core::VERSION);
    println!("  rustc     : {}", env!("MINERVA_RUSTC_VERSION"));
    println!("  bind      : {}:{}", bind, port);
    println!("  web root  : {}", web_root.display());
    println!("  token     : {}", if token.is_some() { "enabled (session-scoped)" } else { "disabled" });
    println!("  data dir  : {}", data_dir.display());
    println!("  pid       : {}", std::process::id());
    println!();

    if !web_root.join("index.html").exists() {
        eprintln!("minerva: {}/index.html not found — pass --web-root", web_root.display());
        std::process::exit(1);
    }
    if let Err(e) = std::fs::create_dir_all(&data_dir) {
        eprintln!("minerva: cannot create {}: {}", data_dir.display(), e);
        std::process::exit(1);
    }

    let state = Arc::new(Mutex::new(SessionState::load(&data_dir)));
    let vfs = Arc::new(Vfs::new(&data_dir));
    let exec = Arc::new(exec::Exec::from_env(port, token.clone()));
    let server = http::Server { port, bind, token, web_root, data_dir, state, vfs, exec };
    if let Err(e) = server.run() {
        eprintln!("minerva: server error: {}", e);
        std::process::exit(1);
    }
}

fn print_help() {
    println!("Debian Minerva session daemon v{}\n", core::VERSION);
    println!("USAGE:\n    minerva-sessiond [OPTIONS]\n");
    println!("OPTIONS:");
    println!("    -p, --port <PORT>       HTTP port (default 8080)");
    println!("        --bind <ADDR>       bind address (default 127.0.0.1)");
    println!("        --token <TOKEN>     require X-Minerva-Token on /api/v1/* (except health)");
    println!("    env MINERVA_SESSION_MODE=x11|kiosk|dev, MINERVA_BROWSER, MINERVA_SESSION_PID");
    println!("    real-system API: POST /api/v1/exec, GET /api/v1/desktop-apps, /api/v1/icon");
    println!("        --web-root <DIR>    shell assets (default ./web)");
    println!("        --data-dir <DIR>    session state (default ~/.config/debian-minerva)");
    println!("        --self-test         run the reducer/parser tests and print JSON");
    println!("    -V, --version           print version");
    println!("    -h, --help              print help");
}

fn detect_web_root() -> PathBuf {
    let candidates = [
        PathBuf::from("web"),
        PathBuf::from("./web"),
        PathBuf::from("/usr/share/debian-minerva/web"),
        PathBuf::from("/usr/share/minerva/web"),
    ];
    for c in candidates {
        if c.join("index.html").exists() { return c; }
    }
    PathBuf::from("web")
}

fn default_data_dir() -> PathBuf {
    env::var("XDG_CONFIG_HOME")
        .map(|p| PathBuf::from(p).join("debian-minerva"))
        .unwrap_or_else(|_| {
            env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config/debian-minerva"))
                .unwrap_or_else(|_| PathBuf::from(".minerva"))
        })
}

/// A quick, dependency-free smoke test of the whole Rust core. Handy on a
/// Debian box before wiring up a display manager.
fn run_self_test() {
    let mut s = SessionState::new();
    let actions = vec![
        json::Value::obj(vec![("type", json::Value::s("launch")), ("app", json::Value::s("terminal"))]),
        json::Value::obj(vec![("type", json::Value::s("launch")), ("app", json::Value::s("files"))]),
        json::Value::obj(vec![("type", json::Value::s("add_workspace"))]),
        json::Value::obj(vec![("type", json::Value::s("set_theme")), ("value", json::Value::s("light"))]),
        json::Value::obj(vec![("type", json::Value::s("notify")),
            ("title", json::Value::s("self-test")), ("body", json::Value::s("reducer ok"))]),
    ];
    for a in &actions {
        if let Err(e) = s.apply(a) {
            eprintln!("FAIL: {}", e);
            std::process::exit(1);
        }
    }
    let snap = s.snapshot();
    let text = json::stringify(&snap);
    let reparsed = json::parse(&text).expect("snapshot must be valid JSON");
    assert_eq!(reparsed.get("windows").as_arr().len(), 2);
    assert_eq!(reparsed.get("workspaces").as_arr().len(), 3);
    assert_eq!(reparsed.get("theme").as_str(), "light");
    println!("{}", text);
    eprintln!("self-test: OK (2 windows, 3 workspaces, JSON roundtrip)");
}
