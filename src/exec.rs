//! Real-system backend.
//!
//! This is what makes Minerva an actual desktop environment instead of a
//! web page: apps are launched as real processes (parsed from the installed
//! `.desktop` files or a built-in candidate table), shell panels are real X
//! windows, and system controls (volume, brightness, wi-fi, resolution,
//! night light, power) shell out to the real system tools — pactl, nmcli,
//! xrandr, rfkill, systemctl. Everything is std-only.

use crate::json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub struct Exec {
    /// "x11" (real multi-window session), "kiosk" (single window) or "dev".
    pub session_mode: String,
    pub browser: Option<String>,
    pub session_pid: Option<u32>,
    pub runtime_dir: String,
    pub port: u16,
    pub token: Option<String>,
    /// app id -> pid of the launched process (for dock running indicators)
    launched: Mutex<Vec<(String, u32)>>,
    /// pid of the shared chromium process hosting on-demand panel windows
    panels_pid: Mutex<Option<u32>>,
    /// fallback requests from panel windows, consumed by the desktop window
    pending: Mutex<Vec<String>>,
    /// shell events queued for the shell surface (Super key -> launcher, ...)
    events: Mutex<Vec<String>>,
    icon_cache: Mutex<HashMap<String, Option<(&'static str, Vec<u8>)>>>,
}

// ---------------------------------------------------------------- utilities

fn run_out(cmd: &str) -> String {
    Command::new("sh").arg("-c").arg(cmd)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

fn spawn_detached(cmd: &str) -> Option<u32> {
    Command::new("sh").arg("-c").arg(cmd)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().ok().map(|c| c.id())
}

fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{}", pid)).exists()
}

fn first_bin(cands: &[&str]) -> Option<String> {
    for c in cands {
        if !run_out(&format!("command -v {}", c)).trim().is_empty() {
            return Some((*c).to_string());
        }
    }
    None
}

fn ok_json(extra: Vec<(&str, Value)>) -> Value {
    let mut v: Vec<(&str, Value)> = vec![("ok", Value::Bool(true))];
    v.extend(extra);
    Value::obj(v)
}

fn fail_json(reason: &str) -> Value {
    Value::obj(vec![
        ("ok", Value::Bool(false)),
        ("error", Value::s(reason.to_string())),
    ])
}

fn sanitized(s: &str) -> bool {
    !s.is_empty() && !s.contains('\'') && !s.contains('"') && !s.contains(';')
        && !s.contains('$') && !s.contains('`') && !s.contains('\n')
}

// ------------------------------------------------------- built-in app table

struct AppDef {
    bins: &'static [&'static str],
    arg: &'static str, // "", "$HOME" or "$ARG"
    hint: &'static str,
}

fn app_def(id: &str) -> Option<AppDef> {
    let d = match id {
        "terminal" => AppDef {
            bins: &["x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal",
                    "mate-terminal", "tilix", "alacritty", "xterm"],
            arg: "", hint: "sudo apt install gnome-terminal" },
        "files" => AppDef {
            bins: &["nautilus", "thunar", "dolphin", "nemo", "pcmanfm", "caja"],
            arg: "$HOME", hint: "sudo apt install nautilus" },
        "web" | "browser" => AppDef {
            bins: &["x-www-browser", "firefox-esr", "firefox", "chromium", "chromium-browser",
                    "epiphany-browser"],
            arg: "$ARG", hint: "sudo apt install firefox-esr" },
        "editor" | "text" | "notes" => AppDef {
            bins: &["gedit", "gnome-text-editor", "mousepad", "kate", "kwrite", "pluma", "leafpad"],
            arg: "$ARG", hint: "sudo apt install gedit" },
        "calculator" => AppDef {
            bins: &["gnome-calculator", "kcalc", "galculator"],
            arg: "", hint: "sudo apt install gnome-calculator" },
        "taskmanager" | "monitor" => AppDef {
            bins: &["gnome-system-monitor", "ksysguard", "lxtask", "mate-system-monitor"],
            arg: "", hint: "sudo apt install gnome-system-monitor" },
        "clock" => AppDef { bins: &["gnome-clocks", "kclock"], arg: "", hint: "sudo apt install gnome-clocks" },
        "calendar" => AppDef { bins: &["gnome-calendar", "korganizer"], arg: "", hint: "sudo apt install gnome-calendar" },
        "camera" => AppDef { bins: &["cheese", "kamoso", "guvcview"], arg: "", hint: "sudo apt install cheese" },
        "music" => AppDef {
            bins: &["rhythmbox", "lollypop", "audacious", "clementine", "pragha", "deadbeef"],
            arg: "", hint: "sudo apt install rhythmbox" },
        "mail" => AppDef {
            bins: &["thunderbird", "evolution", "geary", "claws-mail"],
            arg: "", hint: "sudo apt install thunderbird" },
        "displays" | "display" => AppDef {
            bins: &["gnome-control-center", "xfce4-display-settings", "systemsettings"],
            arg: "", hint: "sudo apt install gnome-control-center" },
        "ossettings" => AppDef {
            bins: &["gnome-control-center", "xfce4-settings-manager", "systemsettings"],
            arg: "", hint: "sudo apt install gnome-control-center" },
        "help" => AppDef { bins: &["yelp", "khelpcenter"], arg: "", hint: "" },
        _ => return None,
    };
    Some(d)
}

// ------------------------------------------------------------------- impl

impl Exec {
    pub fn from_env(port: u16, token: Option<String>) -> Exec {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        Exec {
            session_mode: env("MINERVA_SESSION_MODE").unwrap_or_else(|| "dev".into()),
            browser: env("MINERVA_BROWSER"),
            session_pid: env("MINERVA_SESSION_PID").and_then(|p| p.parse().ok()),
            runtime_dir: env("XDG_RUNTIME_DIR").unwrap_or_else(|| "/tmp".into()),
            port,
            token,
            launched: Mutex::new(Vec::new()),
            panels_pid: Mutex::new(None),
            pending: Mutex::new(Vec::new()),
            events: Mutex::new(Vec::new()),
            icon_cache: Mutex::new(HashMap::new()),
        }
    }

    fn shell_url(&self, hash: &str) -> String {
        match &self.token {
            Some(t) => format!("http://127.0.0.1:{}/?token={}#{}", self.port, t, hash),
            None => format!("http://127.0.0.1:{}/#{}", self.port, hash),
        }
    }

    fn browser_bin(&self) -> Option<String> {
        self.browser.clone().or_else(|| first_bin(&["chromium", "chromium-browser"]))
    }

    // ------------------------------------------------------------- apps

    /// Launch a real application. Returns ok:false when no real binary is
    /// installed so the shell can fall back or show an install hint.
    pub fn launch_app(&self, id: &str, arg: &str) -> Value {
        // 1. real .desktop entries win
        if let Some(cmd) = desktop_app_exec(id) {
            if let Some(pid) = spawn_detached(&cmd) {
                self.record(id, pid);
                return ok_json(vec![("target".into(), Value::s("desktop-entry"))]);
            }
            return fail_json("spawn failed");
        }
        // 2. built-in candidate table
        if let Some(def) = app_def(id) {
            if first_bin(def.bins).is_none() {
                let mut v = fail_json("not installed");
                if !def.hint.is_empty() {
                    if let Value::Obj(ref mut items) = v {
                        items.push(("hint".into(), Value::s(def.hint.to_string())));
                    }
                }
                return v;
            }
            let argpart = match def.arg {
                "" => String::new(),
                "$HOME" => " \"$HOME\"".to_string(),
                _ => if !arg.is_empty() && sanitized(arg) { format!(" '{}'", arg.replace('\'', "")) } else { String::new() },
            };
            let bins = def.bins.join(" ");
            let script = format!(
                "for c in {}; do if command -v \"$c\" >/dev/null 2>&1; then exec \"$c\"{}; fi; done; exit 42",
                bins, argpart
            );
            if let Some(pid) = spawn_detached(&script) {
                self.record(id, pid);
                return ok_json(vec![("target".into(), Value::s("builtin"))]);
            }
            return fail_json("spawn failed");
        }
        fail_json("unknown app")
    }

    fn record(&self, id: &str, pid: u32) {
        let mut l = self.launched.lock().unwrap();
        l.retain(|(_, p)| pid_alive(*p));
        l.push((id.to_string(), pid));
        if l.len() > 64 { let drain = l.len() - 64; l.drain(0..drain); }
    }

    /// Live window list. In an X11 session this mirrors whatever the window
    /// manager actually manages (EWMH _NET_CLIENT_LIST), so the dock taskbar
    /// shows every real window — not just processes this daemon spawned.
    pub fn running_json(&self) -> Value {
        let mut wins: Vec<Value> = Vec::new();
        let mut apps: Vec<String> = Vec::new();
        if self.session_mode == "x11" {
            let active = Self::xprop_hex(&run_out("xprop -root _NET_ACTIVE_WINDOW 2>/dev/null"));
            let clients = run_out("xprop -root _NET_CLIENT_LIST 2>/dev/null");
            for hex in clients.split('#').skip(1)
                .map(Self::hex_token)
                .filter(|h| !h.is_empty())
            {
                let out = run_out(&format!("xprop -id 0x{} _NET_WM_NAME WM_CLASS _NET_WM_DESKTOP 2>/dev/null", hex));
                let title = Self::xprop_first(&out, "_NET_WM_NAME");
                let class = Self::xprop_second(&out, "WM_CLASS");
                // the fullscreen shell surface is not a taskbar entry
                if class.starts_with("minerva") || title == "Minerva" { continue; }
                if title.is_empty() && class.is_empty() { continue; }
                let app = Self::guess_app(&class);
                if !app.is_empty() && !apps.contains(&app) { apps.push(app.clone()); }
                let raw_desk = Self::xprop_num(&out, "_NET_WM_DESKTOP");
                let desktop = if raw_desk < 0 || raw_desk > 0xFFFFFFF { -1 } else { raw_desk };
                wins.push(Value::obj(vec![
                    ("id".into(), Value::s(hex.clone())),
                    ("title".into(), Value::s(title)),
                    ("class".into(), Value::s(class)),
                    ("app".into(), Value::s(app)),
                    ("desktop".into(), Value::n(desktop as f64)),
                    ("focused".into(), Value::Bool(hex == active)),
                ]));
            }
        }
        let mut l = self.launched.lock().unwrap();
        l.retain(|(_, p)| pid_alive(*p));
        for (id, _) in l.iter() {
            if !apps.contains(id) { apps.push(id.clone()); }
        }
        let cur = Self::xprop_num(&run_out("xprop -root _NET_CURRENT_DESKTOP 2>/dev/null"), "_NET_CURRENT_DESKTOP");
        Value::obj(vec![
            ("current_desktop".into(), Value::n(if cur < 0 { 0 } else { cur } as f64)),
            ("apps".into(), Value::arr(apps.into_iter()
                .map(|id| Value::obj(vec![("id".into(), Value::s(id)), ("pid".into(), Value::n(0.0))]))
                .collect())),
            ("windows".into(), Value::arr(wins)),
        ])
    }

    fn xprop_hex(out: &str) -> String {
        out.split('#').skip(1).map(Self::hex_token).next().unwrap_or_default()
    }
    /// xprop renders ids as "#x4600007"; accept both "x4600007" and "4600007".
    fn hex_token(t: &str) -> String {
        let t = t.trim();
        let t = t.strip_prefix('x').unwrap_or(t);
        t.chars().take_while(|c| c.is_ascii_hexdigit()).collect()
    }
    fn xprop_quoted(out: &str, key: &str) -> Vec<String> {
        for line in out.lines() {
            if line.starts_with(key) {
                let mut items = Vec::new();
                let mut rest = line;
                while let Some(a) = rest.find('"') {
                    let tail = &rest[a + 1..];
                    match tail.find('"') {
                        Some(b) => { items.push(tail[..b].to_string()); rest = &tail[b + 1..]; }
                        None => break,
                    }
                }
                return items;
            }
        }
        Vec::new()
    }
    fn xprop_num(out: &str, key: &str) -> i64 {
        for line in out.lines() {
            if line.starts_with(key) {
                if let Some(t) = line.split('=').nth(1) {
                    if let Ok(n) = t.trim().parse::<i64>() { return n; }
                }
            }
        }
        -2
    }
    fn xprop_first(out: &str, key: &str) -> String {
        Self::xprop_quoted(out, key).into_iter().next().unwrap_or_default()
    }
    fn xprop_second(out: &str, key: &str) -> String {
        let q = Self::xprop_quoted(out, key);
        if q.len() > 1 { q[1].clone() } else { q.first().cloned().unwrap_or_default() }
    }
    fn guess_app(class: &str) -> String {
        let c = class.to_lowercase();
        let table: &[(&str, &[&str])] = &[
            ("terminal", &["terminal", "xterm", "urxvt", "alacritty", "kitty", "konsole", "tilix", "pty"]),
            ("files", &["pcmanfm", "thunar", "nautilus", "dolphin", "nemo", "caja", "filemanager", "file-manager"]),
            ("browser", &["chromium", "chrome", "firefox", "epiphany"]),
            ("editor", &["mousepad", "gedit", "pluma", "geany", "kate", "xed"]),
            ("images", &["viewnior", "feh", "eog", "gthumb", "sxiv", "nomacs"]),
            ("pdf", &["atril", "evince", "okular", "zathura"]),
            ("calc", &["galculator", "calculator", "gnome-calculator", "speedcrunch"]),
            ("archive", &["xarchiver", "file-roller", "fileroller", "engrampa", "ark"]),
        ];
        for (id, needles) in table {
            for n in *needles {
                if c.contains(n) { return (*id).to_string(); }
            }
        }
        String::new()
    }

    /// Focus / close / minimize a real X window by EWMH id (wmctrl).
    pub fn window_op(&self, op: &str, id: &str) -> Value {
        if self.session_mode != "x11" { return fail_json("not in x11 session mode"); }
        if !id.chars().all(|c| c.is_ascii_hexdigit()) || id.is_empty() { return fail_json("bad window id"); }
        let wid = format!("0x{}", id);
        let cmd = match op {
            "focus" => format!("wmctrl -i -b remove,hidden -r {0} 2>/dev/null; wmctrl -i -a {0} 2>/dev/null", wid),
            "close" => format!("wmctrl -i -c {} 2>/dev/null", wid),
            "minimize" => format!("wmctrl -i -b add,hidden -r {} 2>/dev/null", wid),
            "restore" => format!("wmctrl -i -b remove,hidden -r {} 2>/dev/null", wid),
            _ => return fail_json("bad win op"),
        };
        run_out(&cmd);
        ok_json(vec![("op".into(), Value::s(op.to_string())), ("id".into(), Value::s(id.to_string()))])
    }

    /// Queue a shell event (consumed by the shell surface via /api/v1/events).
    pub fn push_event(&self, name: &str) -> Value {
        self.events.lock().unwrap().push(name.to_string());
        ok_json(vec![("event".into(), Value::s(name.to_string()))])
    }

    pub fn take_events(&self) -> Value {
        let mut e = self.events.lock().unwrap();
        let arr = Value::arr(e.iter().map(|s| Value::s(s.clone())).collect());
        e.clear();
        Value::obj(vec![("events".into(), arr)])
    }

    pub fn push_pending(&self, id: &str) -> Value {
        let mut p = self.pending.lock().unwrap();
        if !p.iter().any(|x| x == id) { p.push(id.to_string()); }
        ok_json(vec![])
    }

    pub fn take_pending(&self) -> Value {
        let mut p = self.pending.lock().unwrap();
        let ids: Vec<Value> = p.iter().map(|s| Value::s(s.clone())).collect();
        p.clear();
        Value::obj(vec![("ok".into(), Value::Bool(true)), ("apps".into(), Value::arr(ids))])
    }

    // ----------------------------------------------------------- panels

    /// Open a shell panel (launcher, wifi, audio, ...) as a real X window.
    pub fn open_panel(&self, role: &str) -> Value {
        if self.session_mode != "x11" { return fail_json("not in x11 session mode"); }
        if !role.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return fail_json("bad role");
        }
        let browser = match self.browser_bin() { Some(b) => b, None => return fail_json("no browser") };
        let url = self.shell_url(role);
        let profile = format!("{}/minerva/panels", self.runtime_dir);
        let script = format!(
            "exec {} --app='{}' --wm-class=minerva-{2} --class=minerva-{2} --user-data-dir='{3}' \
             --no-first-run --no-default-browser-check --disable-session-crashed-bubble \
             --password-store=basic --disable-translate",
            browser, url, role, profile
        );
        match spawn_detached(&script) {
            Some(pid) => {
                *self.panels_pid.lock().unwrap() = Some(pid);
                ok_json(vec![("panel".into(), Value::s(role.to_string()))])
            }
            None => fail_json("spawn failed"),
        }
    }

    pub fn close_panels(&self) -> Value {
        let pid = self.panels_pid.lock().unwrap().take();
        if let Some(p) = pid { let _ = Command::new("kill").arg(p.to_string()).status(); }
        ok_json(vec![])
    }

    // ------------------------------------------------------------ cmds

    pub fn session_cmd(&self, cmd: &str) -> Value {
        match cmd {
            "logout" => {
                self.close_panels();
                match self.session_pid {
                    Some(p) => {
                        let _ = Command::new("kill").arg(p.to_string()).status();
                        ok_json(vec![])
                    }
                    None => fail_json("no session pid"),
                }
            }
            "lock" => {
                // Real PAM lock when the base locker runs (xfce4-screensaver,
                // configured invisible: no auto-blank, manual --lock requires
                // auth); otherwise the shell locker surface fallback — the
                // enforcer restacks it above every window while the flag
                // exists.
                let have = Command::new("sh")
                    .args(["-c", "pgrep -x xfce4-screensaver >/dev/null 2>&1"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if have {
                    spawn_detached("xfce4-screensaver-command --lock");
                } else {
                    let _ = std::fs::create_dir_all(&self.runtime_dir);
                    let _ = std::fs::write(format!("{}/locked", self.runtime_dir), b"1");
                }
                ok_json(vec![])
            }
            "unlock" => {
                let _ = std::fs::remove_file(format!("{}/locked", self.runtime_dir));
                ok_json(vec![])
            }
            "reboot" => {
                spawn_detached("systemctl reboot || pkexec systemctl reboot");
                ok_json(vec![])
            }
            "shutdown" => {
                spawn_detached("systemctl poweroff || pkexec systemctl poweroff");
                ok_json(vec![])
            }
            "suspend" => {
                spawn_detached("systemctl suspend || pkexec systemctl suspend");
                ok_json(vec![])
            }
            "showdesktop" => {
                spawn_detached("wmctrl -k on 2>/dev/null || xdotool key super+d 2>/dev/null || true");
                ok_json(vec![])
            }
            _ => fail_json("unknown cmd"),
        }
    }

    // ------------------------------------------------------------- sys

    pub fn sys(&self, op: &str, value: &Value) -> Value {
        match op {
            "volume_get" => {
                let s = run_out("pactl get-sink-volume @DEFAULT_SINK@ 2>/dev/null | grep -o '[0-9]\\+%' | head -1 | tr -d '%'");
                let v = s.trim().parse::<i64>();
                let m = run_out("pactl get-sink-mute @DEFAULT_SINK@ 2>/dev/null | grep -o 'yes' | head -1");
                match v {
                    Ok(n) => ok_json(vec![
                        ("volume".into(), Value::n(n as f64)),
                        ("muted".into(), Value::Bool(m.trim() == "yes")),
                        ("backend".into(), Value::s("pactl")),
                    ]),
                    Err(_) => {
                        let a = run_out("amixer get Master 2>/dev/null | grep -o '\\[[0-9]\\+%\\]' | head -1 | tr -dc '0-9'");
                        match a.trim().parse::<i64>() {
                            Ok(n) => ok_json(vec![("volume".into(), Value::n(n as f64)), ("muted".into(), Value::Bool(false)), ("backend".into(), Value::s("amixer"))]),
                            Err(_) => fail_json("no mixer"),
                        }
                    }
                }
            }
            "volume_set" => {
                let n = value.get("n").as_f64().round().clamp(0.0, 100.0) as i64;
                spawn_detached(&format!(
                    "pactl set-sink-volume @DEFAULT_SINK@ {}% 2>/dev/null || amixer -q set Master {}% 2>/dev/null", n, n
                ));
                ok_json(vec![("volume".into(), Value::n(n as f64))])
            }
            "mute_toggle" => {
                spawn_detached("pactl set-sink-mute @DEFAULT_SINK@ toggle 2>/dev/null || amixer -q set Master toggle");
                ok_json(vec![])
            }
            "brightness_set" => {
                let pct = value.get("n").as_f64().round().clamp(5.0, 100.0);
                // sysfs first (real backlight), xrandr software fallback
                let wrote = write_backlight(pct);
                let f = pct / 100.0;
                spawn_detached(&format!(
                    "o=$(xrandr 2>/dev/null | awk '/ primary/{{print $1; exit}}'); [ -n \"$o\" ] && xrandr --output \"$o\" --brightness {:.2} 2>/dev/null; true", f
                ));
                ok_json(vec![("brightness".into(), Value::n(pct)), ("sysfs".into(), Value::Bool(wrote))])
            }
            "wifi_state" => {
                let on = run_out("nmcli radio wifi 2>/dev/null").trim() == "enabled";
                ok_json(vec![("on".into(), Value::Bool(on)), ("available".into(), Value::Bool(has_bin("nmcli")))])
            }
            "wifi_list" => {
                let raw = run_out("nmcli -t -f SSID,SIGNAL,SECURITY,ACTIVE dev wifi list 2>/dev/null");
                let mut seen: Vec<String> = Vec::new();
                let mut nets: Vec<Value> = Vec::new();
                for line in raw.lines() {
                    let parts: Vec<&str> = line.splitn(4, ':').collect();
                    if parts.len() < 4 { continue; }
                    let ssid = parts[0];
                    if ssid.is_empty() || seen.iter().any(|s| s == ssid) { continue; }
                    seen.push(ssid.to_string());
                    let sig: i64 = parts[1].parse().unwrap_or(0);
                    nets.push(Value::obj(vec![
                        ("ssid".into(), Value::s(ssid.to_string())),
                        ("signal".into(), Value::n(sig as f64)),
                        ("secure".into(), Value::Bool(!parts[2].is_empty())),
                        ("active".into(), Value::Bool(parts[3] == "yes")),
                    ]));
                }
                ok_json(vec![("networks".into(), Value::arr(nets))])
            }
            "wifi_toggle" => {
                let on = value.get("on").as_bool();
                let s = if on { "nmcli radio wifi on" } else { "nmcli radio wifi off" };
                if has_bin("nmcli") { spawn_detached(s); ok_json(vec![("on".into(), Value::Bool(on))]) }
                else { fail_json("network-manager not installed") }
            }
            "wifi_connect" => {
                let ssid = value.get("ssid").as_str().to_string();
                let pass = value.get("password").as_str().to_string();
                if !sanitized(&ssid) { return fail_json("bad ssid"); }
                if !pass.is_empty() && !sanitized(&pass) { return fail_json("bad password"); }
                if !has_bin("nmcli") { return fail_json("network-manager not installed"); }
                let cmd = if pass.is_empty() {
                    format!("nmcli dev wifi connect '{}'", ssid)
                } else {
                    format!("nmcli dev wifi connect '{}' password '{}'", ssid, pass)
                };
                spawn_detached(&cmd);
                ok_json(vec![("ssid".into(), Value::s(ssid))])
            }
            "bt_toggle" => {
                let on = value.get("on").as_bool();
                let s = if on { "rfkill unblock bluetooth; bluetoothctl power on" } else { "bluetoothctl power off; rfkill block bluetooth" };
                spawn_detached(&format!("{} >/dev/null 2>&1", s));
                ok_json(vec![("on".into(), Value::Bool(on))])
            }
            "res_list" => {
                let raw = run_out("xrandr 2>/dev/null");
                let mut modes: Vec<Value> = Vec::new();
                let mut seen: Vec<String> = Vec::new();
                for line in raw.lines() {
                    if !line.starts_with(' ') && !line.starts_with('\t') { continue; }
                    let mut it = line.split_whitespace();
                    let m = match it.next() { Some(m) => m, None => continue };
                    if !m.contains('x') || !m.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) { continue; }
                    let current = line.contains('*');
                    if seen.iter().any(|s| s == m) { continue; }
                    seen.push(m.to_string());
                    modes.push(Value::obj(vec![
                        ("mode".into(), Value::s(m.to_string())),
                        ("current".into(), Value::Bool(current)),
                    ]));
                }
                ok_json(vec![("modes".into(), Value::arr(modes)), ("available".into(), Value::Bool(has_bin("xrandr")))])
            }
            "res_set" => {
                let mode = value.get("mode").as_str().to_string();
                let valid = !mode.is_empty() && mode.chars().all(|c| c.is_ascii_digit() || c == 'x');
                if !valid { return fail_json("bad mode"); }
                spawn_detached(&format!("xrandr -s {} 2>/dev/null", mode));
                ok_json(vec![("mode".into(), Value::s(mode))])
            }
            "night_set" => {
                let on = value.get("on").as_bool();
                let g = if on { "1.0:0.82:0.62" } else { "1.0:1.0:1.0" };
                spawn_detached(&format!(
                    "o=$(xrandr 2>/dev/null | awk '/ primary/{{print $1; exit}}'); [ -n \"$o\" ] && xrandr --output \"$o\" --gamma {} 2>/dev/null; true", g
                ));
                ok_json(vec![("on".into(), Value::Bool(on))])
            }
            "apt_search" => {
                let q = value.get("q").as_str().to_string();
                if !sanitized(&q) { return fail_json("bad query"); }
                let raw = run_out(&format!("apt-cache search --names-only '{}' 2>/dev/null | head -80", q));
                let mut out: Vec<Value> = Vec::new();
                for line in raw.lines() {
                    let mut it = line.splitn(2, ' ');
                    let name = match it.next() { Some(n) => n.trim(), None => continue };
                    if name.is_empty() { continue; }
                    let desc = it.next().unwrap_or("").trim();
                    out.push(Value::obj(vec![("name".into(), Value::s(name.to_string())), ("desc".into(), Value::s(desc.to_string()))]));
                }
                ok_json(vec![("results".into(), Value::arr(out))])
            }
            "apt_install" => {
                let pkg = value.get("pkg").as_str().to_string();
                let valid = !pkg.is_empty() && pkg.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '+' | '.' | '_'));
                if !valid { return fail_json("bad package name"); }
                let term = first_bin(&["x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm"]);
                match term {
                    Some(t) => {
                        spawn_detached(&format!("{} -e sh -c 'sudo apt-get install -y {}; echo; echo Done — press Enter to close; read x'", t, pkg));
                        ok_json(vec![("pkg".into(), Value::s(pkg))])
                    }
                    None => fail_json("no terminal emulator"),
                }
            }
            _ => fail_json("unknown sys op"),
        }
    }

    // ---------------------------------------------------- desktop entries

    /// Scan the real `.desktop` catalogue: everything the user has installed.
    pub fn desktop_apps(&self) -> Value {
        let mut dirs = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
        ];
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(format!("{}/.local/share/applications", home)));
        }
        let mut apps: Vec<Value> = Vec::new();
        let mut seen_names: Vec<String> = Vec::new();
        for dir in dirs {
            let entries = match std::fs::read_dir(&dir) { Ok(e) => e, Err(_) => continue };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e != "desktop").unwrap_or(true) { continue; }
                if let Some(a) = parse_desktop(&path) {
                    let name = match a.get("name") { Value::Str(s) => s.clone(), _ => continue };
                    if seen_names.iter().any(|n| n == &name) { continue; }
                    seen_names.push(name);
                    apps.push(a);
                }
            }
        }
        apps.sort_by(|a, b| {
            let an = match a.get("name") { Value::Str(s) => s.clone(), _ => String::new() };
            let bn = match b.get("name") { Value::Str(s) => s.clone(), _ => String::new() };
            an.to_lowercase().cmp(&bn.to_lowercase())
        });
        Value::obj(vec![("apps".into(), Value::arr(apps))])
    }

    /// Launch a real app straight from its .desktop id.
    pub fn launch_desktop_app(&self, id: &str) -> Value {
        match desktop_app_exec(id) {
            Some(cmd) => match spawn_detached(&cmd) {
                Some(pid) => { self.record(id, pid); ok_json(vec![("target".into(), Value::s("desktop-entry"))]) }
                None => fail_json("spawn failed"),
            },
            None => fail_json("unknown desktop entry"),
        }
    }

    // ------------------------------------------------------------ icons

    pub fn icon(&self, name: &str) -> Option<(&'static str, Vec<u8>)> {
        {
            let c = self.icon_cache.lock().unwrap();
            if let Some(hit) = c.get(name) { return hit.clone(); }
        }
        let found = find_icon(name);
        let mut c = self.icon_cache.lock().unwrap();
        c.insert(name.to_string(), found.clone());
        found
    }
}

fn has_bin(b: &str) -> bool {
    !run_out(&format!("command -v {}", b)).trim().is_empty()
}

fn write_backlight(pct: f64) -> bool {
    let base = Path::new("/sys/class/backlight");
    let entries = match std::fs::read_dir(base) { Ok(e) => e, Err(_) => return false };
    for entry in entries.flatten() {
        let dir = entry.path();
        let max: f64 = match std::fs::read_to_string(dir.join("max_brightness")) {
            Ok(s) => match s.trim().parse() { Ok(v) => v, Err(_) => continue },
            Err(_) => continue,
        };
        if max <= 0.0 { continue; }
        let target = ((pct / 100.0) * max).round() as i64;
        if std::fs::write(dir.join("brightness"), target.to_string()).is_ok() { return true; }
    }
    false
}

// ----------------------------------------------------- .desktop parsing

fn parse_desktop(path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut in_entry = false;
    let mut name = String::new();
    let mut generic = String::new();
    let mut exec = String::new();
    let mut icon = String::new();
    let mut terminal = false;
    let mut nodisplay = false;
    let mut tryexec = String::new();
    let mut categories = String::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.is_empty() || line.starts_with('#') { continue; }
        let mut it = line.splitn(2, '=');
        let key = it.next().unwrap_or("").trim();
        let val = it.next().unwrap_or("").trim();
        match key {
            "Name" => name = val.to_string(),
            "GenericName" => generic = val.to_string(),
            "Exec" => exec = val.to_string(),
            "Icon" => icon = val.to_string(),
            "Terminal" => terminal = val == "true",
            "NoDisplay" | "Hidden" => nodisplay = nodisplay || val == "true",
            "TryExec" => tryexec = val.to_string(),
            "Categories" => categories = val.to_string(),
            _ => {}
        }
    }
    if nodisplay || name.is_empty() || exec.is_empty() { return None; }
    if !tryexec.is_empty() && !has_bin(tryexec.split_whitespace().next().unwrap_or("")) { return None; }
    let cleaned = clean_exec(&exec);
    let bin = cleaned.split_whitespace().next().unwrap_or("").trim_matches('"');
    if bin.is_empty() || !has_bin(bin) { return None; }
    let id = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Some(Value::obj(vec![
        ("id".into(), Value::s(id)),
        ("name".into(), Value::s(if name.is_empty() { generic } else { name })),
        ("icon".into(), Value::s(icon)),
        ("exec".into(), Value::s(cleaned)),
        ("terminal".into(), Value::Bool(terminal)),
        ("categories".into(), Value::s(categories)),
    ]))
}

/// Strip field codes (%u, %F, ...) from an Exec= value.
fn clean_exec(exec: &str) -> String {
    exec.split_whitespace()
        .filter(|t| !t.starts_with('%'))
        .collect::<Vec<_>>()
        .join(" ")
}

fn desktop_app_exec(id: &str) -> Option<String> {
    if !id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) { return None; }
    let dirs = ["/usr/share/applications", "/usr/local/share/applications"];
    let mut home_dir = String::new();
    if let Ok(home) = std::env::var("HOME") { home_dir = format!("{}/.local/share/applications", home); }
    let mut search: Vec<&str> = dirs.to_vec();
    if !home_dir.is_empty() { search.push(home_dir.as_str()); }
    for dir in search {
        let path = Path::new(dir).join(format!("{}.desktop", id));
        if !path.exists() { continue; }
        if let Some(v) = parse_desktop(&path) {
            let (exec, terminal) = match (&v.get("exec"), &v.get("terminal")) {
                (Value::Str(e), Value::Bool(t)) => (e.clone(), *t),
                _ => continue,
            };
            if terminal {
                if let Some(t) = first_bin(&["x-terminal-emulator", "gnome-terminal", "xterm"]) {
                    let quoted = exec.replace('\'', "'\\''");
                    return Some(format!("exec {} -e sh -c '{}'", t, quoted));
                }
            }
            return Some(format!("exec {}", exec));
        }
    }
    None
}

// ------------------------------------------------------------ icon files

fn find_icon(name: &str) -> Option<(&'static str, Vec<u8>)> {
    if name.is_empty() { return None; }
    if name.starts_with('/') {
        // absolute Icon= paths from .desktop files are legitimate, but the
        // API must not become an arbitrary file read: system roots only.
        if name.contains("..") { return None; }
        let allowed = ["/usr/share/", "/usr/lib/", "/opt/"];
        if !allowed.iter().any(|r| name.starts_with(r)) { return None; }
        let data = std::fs::read(name).ok()?;
        return Some((mime_for(name), data));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) { return None; }
    let sizes = ["scalable", "symbolic", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32"];
    let roots = ["/usr/share/icons/hicolor", "/usr/share/icons/gnome", "/usr/share/icons/Adwaita"];
    let exts = [".svg", ".png", ".xpm"];
    for root in roots {
        for size in sizes {
            for dir in ["apps", "places", "devices", "mimetypes", "categories", "actions"] {
                for ext in exts {
                    let p = format!("{}/{}/{}/{}{}", root, size, dir, name, ext);
                    if let Ok(data) = std::fs::read(&p) { return Some((mime_for(&p), data)); }
                }
            }
        }
    }
    for ext in exts {
        let p = format!("/usr/share/pixmaps/{}{}", name, ext);
        if let Ok(data) = std::fs::read(&p) { return Some((mime_for(&p), data)); }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::Exec;

    // Regression: xprop prints window ids as "#x4600007". The parser must
    // skip the optional 'x' or the whole EWMH taskbar silently empties.
    #[test]
    fn hex_token_handles_x_prefix() {
        assert_eq!(Exec::hex_token("x4600007, window id "), "4600007");
        assert_eq!(Exec::hex_token("4600007"), "4600007");
        assert_eq!(Exec::hex_token(""), "");
        assert_eq!(Exec::hex_token("xzzz"), "");
    }

    #[test]
    fn client_list_ids() {
        let out = "_NET_CLIENT_LIST(WINDOW): window id #x4600007, window id #x4600009\n";
        let ids: Vec<String> = out.split('#').skip(1)
            .map(Exec::hex_token)
            .filter(|h| !h.is_empty())
            .collect();
        assert_eq!(ids, vec!["4600007".to_string(), "4600009".to_string()]);
    }

    #[test]
    fn xprop_field_parsers() {
        let out = "_NET_WM_DESKTOP(CARDINAL) = 2\n\
                   _NET_WM_NAME(UTF8_STRING) = \"Terminal\"\n\
                   WM_CLASS(STRING) = \"xfce4-terminal\", \"Xfce4-terminal\"\n";
        assert_eq!(Exec::xprop_num(out, "_NET_WM_DESKTOP"), 2);
        assert_eq!(Exec::xprop_num(out, "_NET_WM_PID"), -2);
        assert_eq!(Exec::xprop_first(out, "_NET_WM_NAME"), "Terminal");
        assert_eq!(Exec::xprop_second(out, "WM_CLASS"), "Xfce4-terminal");
    }

    #[test]
    fn guess_app_maps_common_classes() {
        assert_eq!(Exec::guess_app("Xfce4-terminal"), "terminal");
        assert_eq!(Exec::guess_app("chromium-browser"), "browser");
        assert_eq!(Exec::guess_app("Pcmanfm"), "files");
        assert_eq!(Exec::guess_app("some-unknown-app"), "");
    }
}

fn mime_for(p: &str) -> &'static str {
    if p.ends_with(".svg") { "image/svg+xml" }
    else if p.ends_with(".png") { "image/png" }
    else if p.ends_with(".xpm") { "image/xpm" }
    else { "application/octet-stream" }
}
