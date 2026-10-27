//! System services exposed to the shell: clock, virtual filesystem, system
//! information and the Minerva command line.

use crate::core::SessionState;
use crate::json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// ------------------------------------------------------------------ time

pub fn now_string() -> String {
    unix_to_utc(epoch_secs())
}

pub fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Civil date from unix seconds (Howard Hinnant's algorithm, UTC).
pub fn unix_to_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let hh = rem / 3600;
    let mm = (rem % 3600) / 60;
    let ss = rem % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", year, m, d, hh, mm, ss)
}

pub fn clock_json() -> Value {
    let secs = epoch_secs();
    let rem = secs % 86_400;
    Value::obj(vec![
        ("epoch", Value::n(secs as f64)),
        ("hour", Value::n((rem / 3600) as f64)),
        ("minute", Value::n(((rem % 3600) / 60) as f64)),
        ("second", Value::n((rem % 60) as f64)),
        ("utc", Value::s(unix_to_utc(secs))),
        ("timezone", Value::s("UTC")),
    ])
}

// --------------------------------------------------------- virtual filesystem

/// A small sandboxed filesystem rooted at `<data-dir>/home`. The Files app
/// can read and write here; path traversal is rejected at the boundary.
pub struct Vfs {
    pub root: PathBuf,
}

impl Vfs {
    pub fn new(data_dir: &Path) -> Self {
        let root = data_dir.join("home");
        let _ = fs::create_dir_all(root.join("Documents"));
        let _ = fs::create_dir_all(root.join("Downloads"));
        let _ = fs::create_dir_all(root.join("Pictures"));
        let _ = fs::create_dir_all(root.join("Projects"));
        if !root.join("README.md").exists() {
            let _ = fs::write(
                root.join("README.md"),
                "# Welcome to Debian Minerva\n\nThis is your Minerva home directory.\nEverything here is served by the Rust session daemon.\n",
            );
        }
        if !root.join("Documents/notes.md").exists() {
            let _ = fs::write(
                root.join("Documents/notes.md"),
                "# Notes\n\n- Press Super to open the launcher\n- Alt+Tab switches windows\n- Drag a window to the screen edge to snap it\n",
            );
        }
        Vfs { root }
    }

    fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let clean = path.trim().trim_start_matches('/');
        let rel = Path::new(clean);
        for part in rel.components() {
            if part.as_os_str().to_string_lossy() == ".." {
                return Err("path traversal rejected".into());
            }
        }
        let full = self.root.join(rel);
        if !full.starts_with(&self.root) {
            return Err("path outside home rejected".into());
        }
        Ok(full)
    }

    pub fn list(&self, path: &str) -> Result<Value, String> {
        let full = self.resolve(path)?;
        let mut entries = Vec::new();
        for item in fs::read_dir(&full).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let meta = item.metadata().map_err(|e| e.to_string())?;
            let name = item.file_name().to_string_lossy().to_string();
            let kind = if meta.is_dir() { "dir" } else { "file" };
            let ext = Path::new(&name)
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            entries.push(Value::obj(vec![
                ("name", Value::s(name)),
                ("kind", Value::s(kind)),
                ("size", Value::n(meta.len() as f64)),
                ("ext", Value::s(ext)),
            ]));
        }
        entries.sort_by(|a, b| {
            let ka = a.get("kind");
            let kb = b.get("kind");
            let na = a.get("name");
            let nb = b.get("name");
            (ka.as_str() != "dir", na.as_str().to_lowercase())
                .cmp(&(kb.as_str() != "dir", nb.as_str().to_lowercase()))
        });
        Ok(Value::obj(vec![
            ("path", Value::s(path)),
            ("entries", Value::arr(entries)),
        ]))
    }

    pub fn read(&self, path: &str) -> Result<String, String> {
        let full = self.resolve(path)?;
        fs::read_to_string(full).map_err(|e| e.to_string())
    }

    pub fn write(&self, path: &str, content: &str) -> Result<(), String> {
        let full = self.resolve(path)?;
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(full, content).map_err(|e| e.to_string())
    }

    pub fn mkdir(&self, path: &str) -> Result<(), String> {
        fs::create_dir_all(self.resolve(path)?).map_err(|e| e.to_string())
    }

    pub fn remove(&self, path: &str) -> Result<(), String> {
        let full = self.resolve(path)?;
        if full == self.root {
            return Err("cannot remove home".into());
        }
        if full.is_dir() {
            fs::remove_dir_all(full).map_err(|e| e.to_string())
        } else {
            fs::remove_file(full).map_err(|e| e.to_string())
        }
    }
}

// ------------------------------------------------------------- system info

fn read_trim(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn cmd_output(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// Read the first battery exposed by the kernel. Returns
/// `(level 0.0..=1.0, charging, present)`. Desktop machines without a
/// battery report `present == false` and the status bar hides the glyph.
fn battery_info() -> (f64, bool, bool) {
    let dir = match fs::read_dir("/sys/class/power_supply") {
        Ok(d) => d,
        Err(_) => return (1.0, false, false),
    };
    let mut best: Option<(f64, bool)> = None;
    for entry in dir.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy().into_owned();
        if !name.starts_with("BAT") && !name.to_lowercase().contains("battery") {
            continue;
        }
        let base: PathBuf = entry.path();
        let level = fs::read_to_string(base.join("capacity"))
            .ok()
            .and_then(|v| v.trim().parse::<f64>().ok())
            .map(|v| (v / 100.0).max(0.0).min(1.0))
            .or_else(|| {
                let now = fs::read_to_string(base.join("energy_now"))
                    .ok()
                    .and_then(|v| v.trim().parse::<f64>().ok());
                let full = fs::read_to_string(base.join("energy_full"))
                    .ok()
                    .and_then(|v| v.trim().parse::<f64>().ok());
                match (now, full) {
                    (Some(n), Some(f)) if f > 0.0 => Some((n / f).max(0.0).min(1.0)),
                    _ => None,
                }
            });
        let status = fs::read_to_string(base.join("status"))
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        let charging = status == "charging" || status == "full";
        if let Some(l) = level {
            best = Some((l, charging));
            break;
        }
    }
    match best {
        Some((l, c)) => (l, c, true),
        None => (1.0, false, false),
    }
}

pub fn system_json() -> Value {
    let battery = battery_info();
    let meminfo = read_trim("/proc/meminfo");
    let mut total_kb = 0u64;
    let mut avail_kb = 0u64;
    for line in meminfo.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        } else if line.starts_with("MemAvailable:") {
            avail_kb = line.split_whitespace().nth(1).unwrap_or("0").parse().unwrap_or(0);
        }
    }
    let uptime = read_trim("/proc/uptime")
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .parse::<f64>()
        .unwrap_or(0.0);
    let load = read_trim("/proc/loadavg");
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let kernel = if read_trim("/proc/sys/kernel/osrelease").is_empty() {
        cmd_output("uname", &["-r"])
    } else {
        read_trim("/proc/sys/kernel/osrelease")
    };
    let hostname = read_trim("/etc/hostname");
    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let pretty = os_release
        .lines()
        .find(|l| l.starts_with("PRETTY_NAME="))
        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        .unwrap_or_else(|| "Debian GNU/Linux".into());
    Value::obj(vec![
        ("hostname", Value::s(if hostname.is_empty() { "minerva".into() } else { hostname })),
        ("kernel", Value::s(kernel)),
        ("os", Value::s(pretty)),
        ("arch", Value::s(cmd_output("uname", &["-m"]))),
        ("cpus", Value::n(cpus as f64)),
        ("mem_total_mb", Value::n((total_kb / 1024) as f64)),
        ("mem_used_mb", Value::n((total_kb.saturating_sub(avail_kb) / 1024) as f64)),
        ("mem_percent", Value::n(if total_kb > 0 { (total_kb - avail_kb) as f64 / total_kb as f64 * 100.0 } else { 0.0 })),
        ("uptime_secs", Value::n(uptime)),
        ("loadavg", Value::s(load)),
        ("session", Value::s("minerva-wayland (prototype)")),
        ("toolkit", Value::s("Minerva Shell / Rust std")),
        ("de", Value::s(crate::core::DE_NAME)),
        ("de_version", Value::s(crate::core::VERSION)),
        ("rustc", Value::s(env!("MINERVA_RUSTC_VERSION"))),
        ("battery_level", Value::n(battery.0)),
        ("battery_charging", Value::Bool(battery.1)),
        ("battery_present", Value::Bool(battery.2)),
    ])
}

// --------------------------------------------------------------- terminal

/// Execute one Minerva shell command. Returns output lines plus any actions
/// that should be replayed into the session reducer.
pub fn terminal(
    state: &mut SessionState,
    vfs: &Vfs,
    line: &str,
) -> (String, Vec<Value>) {
    let mut events = Vec::new();
    let parts: Vec<&str> = line.trim().split_whitespace().collect();
    if parts.is_empty() {
        return (String::new(), events);
    }
    let cmd = parts[0].to_lowercase();
    let args = &parts[1..];
    let out = match cmd.as_str() {
        "help" => HELP.to_string(),
        "?" => HELP.to_string(),
        "clear" => "\x00clear".into(),
        "uname" => {
            let sys = system_json();
            if args.contains(&"-a") {
                format!("Linux {} {} {} GNU/Linux", sys.get("hostname").as_str(), sys.get("kernel").as_str(), sys.get("arch").as_str())
            } else {
                "Linux".into()
            }
        }
        "whoami" => "user".into(),
        "pwd" => format!("/home/user{}", ""),
        "date" => now_string(),
        "uptime" => {
            let secs = system_json().get("uptime_secs").as_f64() as u64;
            format!("up {}h {}m", secs / 3600, (secs % 3600) / 60)
        }
        "echo" => args.join(" "),
        "ls" => {
            let p = args.first().copied().unwrap_or("/");
            match vfs.list(p) {
                Ok(v) => {
                    let entries = v.get("entries");
                    entries.as_arr().iter()
                        .map(|e| {
                            let kind = e.get("kind");
                            let name = e.get("name");
                            if kind.as_str() == "dir" {
                                format!("{}/", name.as_str())
                            } else {
                                name.as_str().to_string()
                            }
                        })
                        .collect::<Vec<_>>().join("  ")
                }
                Err(e) => format!("ls: {}", e),
            }
        }
        "cat" => {
            if let Some(p) = args.first() {
                vfs.read(p).unwrap_or_else(|e| format!("cat: {}", e))
            } else { "cat: missing operand".into() }
        }
        "mkdir" => {
            if let Some(p) = args.first() {
                vfs.mkdir(p).map(|_| String::new()).unwrap_or_else(|e| format!("mkdir: {}", e))
            } else { "mkdir: missing operand".into() }
        }
        "rm" => {
            if let Some(p) = args.first() {
                vfs.remove(p).map(|_| String::new()).unwrap_or_else(|e| format!("rm: {}", e))
            } else { "rm: missing operand".into() }
        }
        "touch" | "write" => {
            if let Some(p) = args.first() {
                let content = args[1..].join(" ");
                vfs.write(p, &content).map(|_| String::new()).unwrap_or_else(|e| format!("{}: {}", cmd, e))
            } else { format!("{}: missing operand", cmd) }
        }
        "open" | "launch" => {
            if let Some(app) = args.first() {
                let action = Value::obj(vec![
                    ("type", Value::s("launch")),
                    ("app", Value::s(*app)),
                    ("allow_duplicate", Value::Bool(true)),
                ]);
                match state.apply(&action) {
                    Ok(_) => format!("launched {}", app),
                    Err(e) => format!("open: {} (try 'apps')", e),
                }
            } else { "open: missing app id (try 'apps')".into() }
        }
        "apps" => crate::core::catalog().iter()
            .map(|a| format!("  {:<12} {}", a.id, a.name))
            .collect::<Vec<_>>().join("\n"),
        "win" | "windows" => {
            if state.windows.is_empty() {
                "no windows open".into()
            } else {
                state.windows.iter().map(|w| format!("  #{} {:<12} {:?}", w.id, w.app_id, w.state)).collect::<Vec<_>>().join("\n")
            }
        }
        "close" => {
            if let Some(id) = args.first().and_then(|s| s.parse::<i64>().ok()) {
                let a = Value::obj(vec![("type", Value::s("close")), ("id", Value::n(id as f64))]);
                let _ = state.apply(&a);
                format!("closed window #{}", id)
            } else { "close: expected window id".into() }
        }
        "theme" => {
            let fallback = state.theme.clone();
            let v = args.first().copied().unwrap_or(fallback.as_str());
            let a = Value::obj(vec![("type", Value::s("set_theme")), ("value", Value::s(v))]);
            let _ = state.apply(&a);
            format!("theme set to {}", v)
        }
        "accent" => {
            let fallback = state.accent.clone();
            let v = args.first().copied().unwrap_or(fallback.as_str());
            let a = Value::obj(vec![("type", Value::s("set_accent")), ("value", Value::s(v))]);
            let _ = state.apply(&a);
            format!("accent set to {}", v)
        }
        "wallpaper" => {
            let fallback = state.wallpaper.clone();
            let v = args.first().copied().unwrap_or(fallback.as_str());
            let a = Value::obj(vec![("type", Value::s("set_wallpaper")), ("value", Value::s(v))]);
            let _ = state.apply(&a);
            format!("wallpaper set to {} (try: swirl, aurora, dusk, light, mesh)", v)
        }
        "notify" => {
            let title = args.first().copied().unwrap_or("Minerva");
            let body = args[1..].join(" ");
            events.push(Value::obj(vec![
                ("type", Value::s("notify")),
                ("title", Value::s(title)),
                ("body", Value::s(body)),
                ("app", Value::s("terminal")),
                ("icon", Value::s("terminal")),
            ]));
            format!("notification queued: {}", title)
        }
        "workspaces" => state.workspaces.iter().enumerate()
            .map(|(i, w)| format!("  {} {}{}", i + 1, w.name, if i == state.active_workspace { "  ← active" } else { "" }))
            .collect::<Vec<_>>().join("\n"),
        "workspace" => {
            let idx = args.first().and_then(|s| s.parse::<i64>().ok()).unwrap_or(1) - 1;
            let a = Value::obj(vec![("type", Value::s("switch_workspace")), ("index", Value::n(idx as f64))]);
            let _ = state.apply(&a);
            format!("switched to workspace {}", state.active_workspace + 1)
        }
        "sysinfo" | "free" => {
            let s = system_json();
            format!("{:<14} {}\n{:<14} {}\n{:<14} {:.0} / {:.0} MB ({:.1}%)\n{:<14} {}\n{:<14} {}\n{:<14} {}",
                "OS", s.get("os").as_str(),
                "Kernel", s.get("kernel").as_str(),
                "Memory", s.get("mem_used_mb").as_f64(), s.get("mem_total_mb").as_f64(), s.get("mem_percent").as_f64(),
                "CPUs", s.get("cpus").as_f64() as i64,
                "Session", s.get("session").as_str(),
                "Load", s.get("loadavg").as_str())
        }
        "ps" => {
            let mut lines = vec![format!("{:>6}  {:<14} {:<10} CPU", "PID", "APP", "STATE")];
            for w in &state.windows {
                lines.push(format!("{:>6}  {:<14} {:<10} {:.1}%", w.id, w.app_id, format!("{:?}", w.state), (w.id % 17) as f64 * 0.7));
            }
            lines.join("\n")
        }
        "neofetch" | "minerva" => {
            let s = system_json();
            format!("\x00ascii\n{:<12} user@{}\n{:<12} {}\n{:<12} {}\n{:<12} {}\n{:<12} {}\n{:<12} {} {}\n{:<12} {} windows / {} workspaces\n{:<12} {}\n{:<12} built with {}",
                "USER", s.get("hostname").as_str(),
                "OS", "Debian Minerva",
                "KERNEL", s.get("kernel").as_str(),
                "SHELL", "minerva-sh (Rust)",
                "DE", "Debian Minerva 0.1.0",
                "THEME", state.theme.as_str(), state.accent.as_str(),
                "WINDOWS", state.windows.len(), state.workspaces.len(),
                "UPTIME", format!("{}s", s.get("uptime_secs").as_f64() as u64),
                "RUSTC", s.get("rustc").as_str())
        }
        "about" => format!("Debian Minerva {} — a Rust desktop session for Debian GNU/Linux.\nNo Google components. No proprietary blobs. Just Debian + Rust.", crate::core::VERSION),
        "poweroff" | "logout" => "\x00poweroff".into(),
        "reboot" => "\x00reboot".into(),
        "reset" => {
            let a = Value::obj(vec![("type", Value::s("reset"))]);
            let _ = state.apply(&a);
            "session reset".into()
        }
        other => format!("minerva-sh: command not found: {}\nType 'help' for a list of commands.", other),
    };
    (out, events)
}

const HELP: &str = "\
Debian Minerva shell — available commands:

  help              this help
  apps              list installed applications
  open <app>        launch an application
  win               list open windows
  close <id>        close a window by id
  workspaces        list virtual desktops
  workspace <n>     switch to desktop n

  ls [path]         list files         cat <path>       read a file
  mkdir <path>      create directory   rm <path>        remove a file
  write <p> <text>  write a file       pwd              current directory

  theme dark|light  switch theme       accent <hex>     set accent colour
  wallpaper <name>  swirl|aurora|dusk|light|mesh
  notify <title> <body>                push a notification

  uname [-a]        kernel info        date             current date
  uptime            session uptime     sysinfo | free   resources
  ps                process list       neofetch         system summary
  echo <text>       print text         clear            clear terminal
  about              about this DE      reset            reset session
  reboot | poweroff  restart / log out";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_conversion() {
        assert_eq!(unix_to_utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(unix_to_utc(1_700_000_000), "2023-11-14 22:13:20 UTC");
    }

    #[test]
    fn vfs_blocks_traversal() {
        let dir = std::env::temp_dir().join("minerva-vfs-test");
        let vfs = Vfs::new(&dir);
        assert!(vfs.list("../../etc").is_err());
        assert!(vfs.read("/../etc/passwd").is_err());
        assert!(vfs.list("/").is_ok());
    }
}
