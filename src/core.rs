//! Core session state: the compositor's authoritative model.
//!
//! Every window, workspace, notification and setting lives here. The web shell
//! is a *renderer* — it never owns state, it sends `Action`s and paints the
//! resulting `Snapshot`. That separation is what lets the same shell talk to a
//! real Wayland compositor later (see docs/ROADMAP.md).

use crate::json::{self, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const VERSION: &str = "0.5.7";
pub const DE_NAME: &str = "Debian Minerva";

#[derive(Clone, Debug, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
}

#[derive(Clone, Debug)]
pub struct Window {
    pub id: u64,
    pub app_id: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub z: u32,
    pub state: WindowState,
    pub workspace: usize,
    pub restore: (i32, i32, i32, i32),
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub id: u64,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct Notification {
    pub id: u64,
    pub app: String,
    pub title: String,
    pub body: String,
    pub icon: String,
    pub time: String,
}

#[derive(Clone, Debug)]
pub struct AppDef {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub group: &'static str,
    pub w: i32,
    pub h: i32,
    pub pinned: bool,
    pub summary: &'static str,
}

/// Built-in application catalogue. No Google/Chrome/Play branding —
/// everything is Debian Minerva native.
pub fn catalog() -> Vec<AppDef> {
    vec![
        AppDef { id: "files", name: "Files", icon: "folder", group: "System", w: 880, h: 540, pinned: true, summary: "Browse your home directory" },
        AppDef { id: "browser", name: "Minerva Web", icon: "globe", group: "Internet", w: 980, h: 620, pinned: true, summary: "A lightweight web view" },
        AppDef { id: "terminal", name: "Terminal", icon: "terminal", group: "System", w: 760, h: 460, pinned: true, summary: "Minerva command line" },
        AppDef { id: "editor", name: "Text Editor", icon: "edit", group: "Utilities", w: 760, h: 520, pinned: true, summary: "Write notes and code" },
        AppDef { id: "settings", name: "Settings", icon: "settings", group: "System", w: 900, h: 580, pinned: true, summary: "Configure your desktop" },
        AppDef { id: "software", name: "Software", icon: "package", group: "System", w: 900, h: 560, pinned: false, summary: "Manage Debian packages" },
        AppDef { id: "monitor", name: "Task Manager", icon: "activity", group: "System", w: 820, h: 520, pinned: false, summary: "Processes and resources" },
        AppDef { id: "calculator", name: "Calculator", icon: "calc", group: "Utilities", w: 360, h: 520, pinned: false, summary: "Quick arithmetic" },
        AppDef { id: "clock", name: "Clock", icon: "clock", group: "Utilities", w: 460, h: 440, pinned: false, summary: "Time, timers and alarms" },
        AppDef { id: "calendar", name: "Calendar", icon: "calendar", group: "Utilities", w: 720, h: 520, pinned: false, summary: "Your schedule" },
        AppDef { id: "notes", name: "Notes", icon: "note", group: "Utilities", w: 600, h: 460, pinned: false, summary: "Scratchpad" },
        AppDef { id: "gallery", name: "Wallpapers", icon: "image", group: "Utilities", w: 780, h: 520, pinned: false, summary: "Choose a wallpaper" },
        AppDef { id: "mail", name: "Mail", icon: "mail", group: "Internet", w: 900, h: 560, pinned: false, summary: "Local mail client" },
        AppDef { id: "music", name: "Music", icon: "music", group: "Media", w: 720, h: 480, pinned: false, summary: "Local audio player" },
        AppDef { id: "camera", name: "Camera", icon: "camera", group: "Media", w: 640, h: 480, pinned: false, summary: "Webcam preview" },
        AppDef { id: "help", name: "Help & About", icon: "info", group: "System", w: 700, h: 500, pinned: false, summary: "About Debian Minerva" },
        AppDef { id: "widgets", name: "Widgets", icon: "widgets", group: "Utilities", w: 760, h: 520, pinned: false, summary: "Desktop widget picker" },
        AppDef { id: "display", name: "Displays", icon: "monitor", group: "System", w: 700, h: 480, pinned: false, summary: "Screen layout" },
    ]
}

pub fn app_by_id(id: &str) -> Option<AppDef> {
    catalog().into_iter().find(|a| a.id == id)
}

pub fn catalog_json() -> Value {
    Value::arr(
        catalog()
            .iter()
            .map(|a| {
                Value::obj(vec![
                    ("id", Value::s(a.id)),
                    ("name", Value::s(a.name)),
                    ("icon", Value::s(a.icon)),
                    ("group", Value::s(a.group)),
                    ("w", Value::n(a.w as f64)),
                    ("h", Value::n(a.h as f64)),
                    ("pinned", Value::Bool(a.pinned)),
                    ("summary", Value::s(a.summary)),
                ])
            })
            .collect(),
    )
}

#[derive(Clone, Debug)]
pub struct SessionState {
    pub theme: String,
    pub accent: String,
    pub wallpaper: String,
    pub scale: f64,
    pub volume: f64,
    pub brightness: f64,
    pub toggles: Vec<(String, bool)>,
    pub windows: Vec<Window>,
    pub workspaces: Vec<Workspace>,
    pub active_workspace: usize,
    pub notifications: Vec<Notification>,
    pub next_id: u64,
    pub z_top: u32,
    pub seq: u64,
    pub boot_time: String,
    pub show_overview: bool,
    pub show_launcher: bool,
    pub pinned: Vec<String>,
    pub recent_apps: Vec<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        SessionState {
            theme: "dark".into(),
            accent: "#a80030".into(),
            wallpaper: "swirl".into(),
            scale: 1.0,
            volume: 0.65,
            brightness: 0.82,
            toggles: vec![
                ("wifi".into(), true),
                ("bluetooth".into(), true),
                ("dnd".into(), false),
                ("night_light".into(), false),
                ("airplane".into(), false),
                ("sync".into(), true),
                ("location".into(), false),
                ("screen_record".into(), false),
            ],
            windows: Vec::new(),
            workspaces: vec![
                Workspace { id: 1, name: "Desktop 1".into() },
                Workspace { id: 2, name: "Desktop 2".into() },
            ],
            active_workspace: 0,
            notifications: vec![Notification {
                id: 1,
                app: "system".into(),
                title: "Welcome to Debian Minerva".into(),
                body: "Your session is ready. Press Super to open the launcher.".into(),
                icon: "info".into(),
                time: "just now".into(),
            }],
            next_id: 2,
            z_top: 10,
            seq: 1,
            boot_time: crate::services::now_string(),
            show_overview: false,
            show_launcher: false,
            pinned: catalog().iter().filter(|a| a.pinned).map(|a| a.id.into()).collect(),
            recent_apps: Vec::new(),
        }
    }

    pub fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn toggle(&self, key: &str) -> bool {
        self.toggles
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| *v)
            .unwrap_or(false)
    }

    pub fn set_toggle(&mut self, key: &str, value: bool) {
        if let Some(slot) = self.toggles.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value;
        } else {
            self.toggles.push((key.into(), value));
        }
    }

    /// The reducer. All mutations go through here so the shell can be replayed,
    /// persisted, and (later) driven by a real compositor's event loop.
    pub fn apply(&mut self, action: &Value) -> Result<Vec<Value>, String> {
        let kind = action.get("type").as_str().to_string();
        let mut events = Vec::new();
        self.seq += 1;
        match kind.as_str() {
            "launch" => {
                let app_id = action.get("app").as_str().to_string();
                let def = app_by_id(&app_id)
                    .ok_or_else(|| format!("unknown app: {}", app_id))?;
                let existing = self
                    .windows
                    .iter()
                    .find(|w| w.app_id == app_id && w.workspace == self.active_workspace)
                    .map(|w| w.id);
                match existing {
                    Some(id) if !action.get("allow_duplicate").as_bool() => self.focus_window(id),
                    _ => self.open_window(&def, &app_id),
                }
                self.show_launcher = false;
                self.show_overview = false;
                if !self.recent_apps.iter().any(|a| a == &app_id) {
                    self.recent_apps.insert(0, app_id.clone());
                    self.recent_apps.truncate(8);
                }
                events.push(Value::obj(vec![("type", Value::s("launched")), ("app", Value::s(app_id))]));
            }
            "focus" => self.focus_window(action.get("id").as_i64() as u64),
            "minimize" => {
                let id = action.get("id").as_i64() as u64;
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.state = WindowState::Minimized;
                }
            }
            "maximize" => {
                let id = action.get("id").as_i64() as u64;
                let found = {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                        if w.state == WindowState::Maximized {
                            w.state = WindowState::Normal;
                            let (x, y, ww, hh) = w.restore;
                            w.x = x;
                            w.y = y;
                            w.w = ww;
                            w.h = hh;
                        } else {
                            w.restore = (w.x, w.y, w.w, w.h);
                            w.state = WindowState::Maximized;
                        }
                        true
                    } else {
                        false
                    }
                };
                if found {
                    self.focus_window(id);
                }
            }
            "close" => {
                let id = action.get("id").as_i64() as u64;
                self.windows.retain(|w| w.id != id);
            }
            "move" => {
                let id = action.get("id").as_i64() as u64;
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.x = action.get("x").as_i64() as i32;
                    w.y = action.get("y").as_i64() as i32;
                    if w.state == WindowState::Maximized {
                        w.state = WindowState::Normal;
                        let (x, y, ww, hh) = w.restore;
                        w.x = x;
                        w.y = y;
                        w.w = ww;
                        w.h = hh;
                    }
                }
            }
            "resize" => {
                let id = action.get("id").as_i64() as u64;
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.w = (action.get("w").as_i64() as i32).max(320);
                    w.h = (action.get("h").as_i64() as i32).max(220);
                    w.state = WindowState::Normal;
                }
            }
            "snap" => {
                let id = action.get("id").as_i64() as u64;
                let side = action.get("side").as_str().to_string();
                let (sw, sh) = (1440.0, 810.0);
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.restore = (w.x, w.y, w.w, w.h);
                    w.state = WindowState::Normal;
                    match side.as_str() {
                        "left" => { w.x = 0; w.y = 40; w.w = (sw / 2.0) as i32; w.h = (sh - 88.0) as i32; }
                        "right" => { w.x = (sw / 2.0) as i32; w.y = 40; w.w = (sw / 2.0) as i32; w.h = (sh - 88.0) as i32; }
                        "full" => {
                            w.restore = (w.x, w.y, w.w, w.h);
                            w.state = WindowState::Maximized;
                        }
                        _ => {}
                    }
                }
            }
            "set_title" => {
                let id = action.get("id").as_i64() as u64;
                let title = action.get("title").as_str().to_string();
                if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                    w.title = title;
                }
            }
            "add_workspace" => {
                let id = self.next_id();
                self.workspaces.push(Workspace {
                    id,
                    name: format!("Desktop {}", self.workspaces.len() + 1),
                });
                self.active_workspace = self.workspaces.len() - 1;
            }
            "remove_workspace" => {
                let idx = action.get("index").as_i64() as usize;
                if self.workspaces.len() > 1 && idx < self.workspaces.len() {
                    self.workspaces.remove(idx);
                    for w in self.windows.iter_mut() {
                        if w.workspace == idx {
                            w.workspace = 0;
                        } else if w.workspace > idx {
                            w.workspace -= 1;
                        }
                    }
                    self.active_workspace = self.active_workspace.min(self.workspaces.len() - 1);
                }
            }
            "switch_workspace" => {
                let idx = action.get("index").as_i64() as usize;
                if idx < self.workspaces.len() {
                    self.active_workspace = idx;
                    self.show_overview = false;
                }
            }
            "move_window_to_workspace" => {
                let id = action.get("id").as_i64() as u64;
                let idx = action.get("index").as_i64() as usize;
                if idx < self.workspaces.len() {
                    if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
                        w.workspace = idx;
                    }
                }
            }
            "overview" => {
                let v = action.get("value");
                self.show_overview = if v.is_null() { !self.show_overview } else { v.as_bool() };
            }
            "launcher" => {
                let v = action.get("value");
                self.show_launcher = if v.is_null() { !self.show_launcher } else { v.as_bool() };
            }
            "set_theme" => self.theme = action.get("value").as_str().to_string(),
            "set_accent" => self.accent = action.get("value").as_str().to_string(),
            "set_wallpaper" => self.wallpaper = action.get("value").as_str().to_string(),
            "set_scale" => self.scale = action.get("value").as_f64().clamp(0.75, 1.5),
            "set_volume" => self.volume = action.get("value").as_f64().clamp(0.0, 1.0),
            "set_brightness" => self.brightness = action.get("value").as_f64().clamp(0.2, 1.0),
            "toggle" => {
                let key = action.get("key").as_str().to_string();
                let current = self.toggle(&key);
                let next = if action.get("value").is_null() { !current } else { action.get("value").as_bool() };
                self.set_toggle(&key, next);
                if key == "dnd" && next {
                    events.push(Value::obj(vec![
                        ("type", Value::s("toast")),
                        ("message", Value::s("Do not disturb is on")),
                    ]));
                }
            }
            "notify" => {
                let id = self.next_id();
                let n = Notification {
                    id,
                    app: action.get("app").as_str().to_string(),
                    title: action.get("title").as_str().to_string(),
                    body: action.get("body").as_str().to_string(),
                    icon: action.get("icon").as_str().to_string(),
                    time: "just now".into(),
                };
                self.notifications.insert(0, n);
                self.notifications.truncate(30);
            }
            "dismiss_notification" => {
                let id = action.get("id").as_i64() as u64;
                self.notifications.retain(|n| n.id != id);
            }
            "clear_notifications" => self.notifications.clear(),
            "minimize_all" => {
                for w in self.windows.iter_mut() {
                    w.state = WindowState::Minimized;
                }
            }
            "close_all" => self.windows.clear(),
            "reset" => {
                let keep = self.boot_time.clone();
                *self = SessionState::new();
                self.boot_time = keep;
            }
            other => return Err(format!("unknown action: {}", other)),
        }
        Ok(events)
    }

    fn open_window(&mut self, def: &AppDef, app_id: &str) {
        let id = self.next_id();
        let cascade = (self.windows.len() % 6) as i32 * 28;
        self.z_top += 1;
        let w = Window {
            id,
            app_id: app_id.into(),
            title: def.name.into(),
            x: 180 + cascade,
            y: 90 + cascade,
            w: def.w,
            h: def.h,
            z: self.z_top,
            state: WindowState::Normal,
            workspace: self.active_workspace,
            restore: (180 + cascade, 90 + cascade, def.w, def.h),
        };
        self.windows.push(w);
    }

    fn focus_window(&mut self, id: u64) {
        self.z_top += 1;
        let z = self.z_top;
        if let Some(w) = self.windows.iter_mut().find(|w| w.id == id) {
            w.z = z;
            if w.state == WindowState::Minimized {
                w.state = WindowState::Normal;
            }
        }
    }

    pub fn snapshot(&self) -> Value {
        let windows = self.windows.iter().map(|w| Value::obj(vec![
            ("id", Value::n(w.id as f64)),
            ("app", Value::s(w.app_id.clone())),
            ("title", Value::s(w.title.clone())),
            ("x", Value::n(w.x as f64)),
            ("y", Value::n(w.y as f64)),
            ("w", Value::n(w.w as f64)),
            ("h", Value::n(w.h as f64)),
            ("z", Value::n(w.z as f64)),
            ("state", Value::s(match w.state { WindowState::Normal => "normal", WindowState::Minimized => "minimized", WindowState::Maximized => "maximized" })),
            ("workspace", Value::n(w.workspace as f64)),
        ])).collect();
        let workspaces = self.workspaces.iter().map(|ws| Value::obj(vec![
            ("id", Value::n(ws.id as f64)),
            ("name", Value::s(ws.name.clone())),
        ])).collect();
        let notifications = self.notifications.iter().map(|n| Value::obj(vec![
            ("id", Value::n(n.id as f64)),
            ("app", Value::s(n.app.clone())),
            ("title", Value::s(n.title.clone())),
            ("body", Value::s(n.body.clone())),
            ("icon", Value::s(n.icon.clone())),
            ("time", Value::s(n.time.clone())),
        ])).collect();
        let toggles = self.toggles.iter().map(|(k, v)| Value::obj(vec![
            ("key", Value::s(k.clone())),
            ("value", Value::Bool(*v)),
        ])).collect();
        Value::obj(vec![
            ("de", Value::s(DE_NAME)),
            ("version", Value::s(VERSION)),
            ("seq", Value::n(self.seq as f64)),
            ("boot_time", Value::s(self.boot_time.clone())),
            ("theme", Value::s(self.theme.clone())),
            ("accent", Value::s(self.accent.clone())),
            ("wallpaper", Value::s(self.wallpaper.clone())),
            ("scale", Value::n(self.scale)),
            ("volume", Value::n(self.volume)),
            ("brightness", Value::n(self.brightness)),
            ("active_workspace", Value::n(self.active_workspace as f64)),
            ("show_overview", Value::Bool(self.show_overview)),
            ("show_launcher", Value::Bool(self.show_launcher)),
            ("toggles", Value::arr(toggles)),
            ("windows", Value::arr(windows)),
            ("workspaces", Value::arr(workspaces)),
            ("notifications", Value::arr(notifications)),
            ("pinned", Value::arr(self.pinned.iter().map(|p| Value::s(p.clone())).collect())),
            ("recent_apps", Value::arr(self.recent_apps.iter().map(|p| Value::s(p.clone())).collect())),
        ])
    }

    /// Restore a persisted snapshot (only the fields the daemon owns).
    pub fn restore(&mut self, v: &Value) {
        if !v.get("theme").is_null() { self.theme = v.get("theme").as_str().into(); }
        if !v.get("accent").is_null() { self.accent = v.get("accent").as_str().into(); }
        if !v.get("wallpaper").is_null() { self.wallpaper = v.get("wallpaper").as_str().into(); }
        if !v.get("volume").is_null() { self.volume = v.get("volume").as_f64(); }
        if !v.get("brightness").is_null() { self.brightness = v.get("brightness").as_f64(); }
        if !v.get("scale").is_null() { self.scale = v.get("scale").as_f64(); }
        for t in v.get("toggles").as_arr() {
            let key = t.get("key").as_str().to_string();
            self.set_toggle(&key, t.get("value").as_bool());
        }
        if !v.get("pinned").is_null() {
            self.pinned = v.get("pinned").as_arr().iter().map(|x| x.as_str().to_string()).collect();
        }
    }

    pub fn state_path(data_dir: &Path) -> PathBuf {
        data_dir.join("session.json")
    }

    pub fn load(data_dir: &Path) -> SessionState {
        let path = Self::state_path(data_dir);
        match fs::read_to_string(&path) {
            Ok(raw) => match json::parse(&raw) {
                Ok(v) => {
                    let mut s = SessionState::new();
                    s.restore(&v);
                    s
                }
                Err(e) => {
                    eprintln!("minerva: ignoring corrupt session file: {}", e);
                    SessionState::new()
                }
            },
            Err(_) => SessionState::new(),
        }
    }

    pub fn save(&self, data_dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(data_dir)?;
        let persistable = Value::obj(vec![
            ("theme", Value::s(self.theme.clone())),
            ("accent", Value::s(self.accent.clone())),
            ("wallpaper", Value::s(self.wallpaper.clone())),
            ("scale", Value::n(self.scale)),
            ("volume", Value::n(self.volume)),
            ("brightness", Value::n(self.brightness)),
            ("toggles", Value::arr(self.toggles.iter().map(|(k, v)| Value::obj(vec![
                ("key", Value::s(k.clone())), ("value", Value::Bool(*v)),
            ])).collect())),
            ("pinned", Value::arr(self.pinned.iter().map(|p| Value::s(p.clone())).collect())),
            ("saved_at", Value::s(crate::services::now_string())),
        ]);
        fs::write(Self::state_path(data_dir), json::stringify(&persistable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(t: &str, extra: Vec<(&str, Value)>) -> Value {
        let mut items = vec![("type", Value::s(t))];
        items.extend(extra);
        Value::obj(items)
    }

    #[test]
    fn launch_and_close() {
        let mut s = SessionState::new();
        assert!(s.windows.is_empty());
        s.apply(&act("launch", vec![("app", Value::s("terminal"))])).unwrap();
        assert_eq!(s.windows.len(), 1);
        // launching twice should focus, not duplicate
        s.apply(&act("launch", vec![("app", Value::s("terminal"))])).unwrap();
        assert_eq!(s.windows.len(), 1);
        let id = s.windows[0].id as f64;
        s.apply(&act("close", vec![("id", Value::n(id))])).unwrap();
        assert!(s.windows.is_empty());
    }

    #[test]
    fn workspaces_move_windows() {
        let mut s = SessionState::new();
        s.apply(&act("launch", vec![("app", Value::s("files"))])).unwrap();
        s.apply(&act("add_workspace", vec![])).unwrap();
        assert_eq!(s.workspaces.len(), 3);
        let id = s.windows[0].id as f64;
        s.apply(&act("move_window_to_workspace", vec![
            ("id", Value::n(id)), ("index", Value::n(2.0)),
        ])).unwrap();
        assert_eq!(s.windows[0].workspace, 2);
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut s = SessionState::new();
        s.apply(&act("set_theme", vec![("value", Value::s("light"))])).unwrap();
        let snap = json::stringify(&s.snapshot());
        let parsed = json::parse(&snap).unwrap();
        assert_eq!(parsed.get("theme").as_str(), "light");
        assert_eq!(parsed.get("de").as_str(), "Debian Minerva");
    }

    #[test]
    fn unknown_app_rejected() {
        let mut s = SessionState::new();
        assert!(s.apply(&act("launch", vec![("app", Value::s("chrome"))])).is_err());
    }
}
