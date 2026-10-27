# Architecture

Debian Minerva is split into two processes, mirroring how ChromeOS-like
desktops isolate the system layer from the renderer.

```
┌───────────────────────────────────────────────────────────────┐
│ Browser / shell renderer (HTML + CSS + vanilla JS)             │
│  ┌──────────┬──────────┬───────────┬──────────────────────┐  │
│  │ Status   │ Dock /   │ Window    │ App views            │  │
│  │ bar      │ launcher │ manager   │ (18 built-ins)       │  │
│  └──────────┴──────────┴───────────┴──────────────────────┘  │
│                      ▲  local mirror for instant interaction   │
│                      ▼  actions + JSON snapshots               │
├───────────────────────────────────────────────────────────────┤
│ /api/v1/* over HTTP — loopback only, per-session token in production                 │
├───────────────────────────────────────────────────────────────┤
│ minerva-sessiond — Rust, std only, no crates                   │
│  ┌────────────┐ ┌────────────┐ ┌──────────┐ ┌─────────────┐ │
│  │ SessionState│ │ JSON       │ │ VFS      │ │ HTTP server │ │
│  │ + reducer  │ │ parser     │ │ / terminal│ │ + thread pool│ │
│  └────────────┘ └────────────┘ └──────────┘ └─────────────┘ │
│             │              │                                  │
│      session.json    /proc, /etc/os-release                    │
└───────────────────────────────────────────────────────────────┘
```

## State ownership

`SessionState` (`src/core.rs`) is authoritative. It contains:

- `windows: Vec<Window>` — id, app, title, geometry, z-order, state
  (normal / minimized / maximized), workspace, restore rect
- `workspaces: Vec<Workspace>` — virtual desktops
- `notifications`, `toggles`, theme / accent / wallpaper / scale / volume /
  brightness, pinned apps, recent apps

Every mutation goes through `SessionState::apply(&Value) -> Result<Vec<Value>>`.
The reducer is pure apart from id/z counters, so it is trivially testable and
replayable. `cargo test` covers the JSON parser, the reducer and VFS traversal
protection.

The shell keeps a **local mirror** of the snapshot so dragging, resizing and
focus feel instant. After each action it POSTs the same action to the daemon
(debounced for `move`/`resize`) and, when the Rust backend is attached, can
reconcile from `GET /api/v1/state`.

## Why no crates

The daemon implements its own ~250-line JSON parser (`src/json.rs`) instead of
pulling `serde`/`serde_json`. Consequences:

- `cargo build --release` works fully offline on a Debian buildd
- no vendoring, no `dh-cargo` dance, no supply-chain surface
- the whole system layer is auditable Rust code in this repository

## Sandboxed filesystem

`Vfs` (`src/services.rs`) roots all file operations at
`$XDG_CONFIG_HOME/debian-minerva/home`. Every path is normalised and `..`
components are rejected before joining. The Files app and terminal `ls`/`cat`/
`write`/`mkdir`/`rm` all go through it.

## System information

`system_json()` reads `/proc/meminfo`, `/proc/uptime`, `/proc/loadavg`,
`/proc/sys/kernel/osrelease`, `/etc/os-release`, `/etc/hostname` and
`available_parallelism()`. No shelling out for the common path.

## Renderer fallback

`web/js/api.js` exposes one API surface with two implementations:

- `backend === 'rust'` → real HTTP calls to `minerva-sessiond`
- otherwise → in-browser equivalents (localStorage VFS, JS reducer,
  `navigator`/`performance` system info)

The desktop is fully functional in either mode. The dock tray displays which
one is active. This is what makes the Arena live preview possible without a
Rust toolchain in the sandbox.

## Branding

All Google-specific elements from the reference leaks were removed: no Chrome,
Play Store, Google Search bar, or Google account flows. Replacements:

| Aluminium OS | Debian Minerva |
|---|---|
| Google Search bar | Minerva Search (local app/files search) |
| Chrome | Minerva Web |
| Play Store | Software |
| Google apps folder | Debian app grid |
| Android setup wizard | Debian Minerva welcome wizard |

## Session lifecycle (production)

A display manager (GDM, LightDM, SDDM) starts the **Minerva** session from
`/usr/share/xsessions/minerva.desktop`, whose `Exec` is `/usr/bin/minerva-session`:

1. The script detects screen geometry (xrandr/xdpyinfo), writes an Openbox
   `rc.xml` with pixel-exact rules for every Minerva surface, and mints a
   per-session token (`$XDG_RUNTIME_DIR/minerva/session.env`, mode 600).
2. It starts `minerva-sessiond` bound to `127.0.0.1:47801` (env
   `MINERVA_SESSION_MODE=x11`, `MINERVA_BROWSER`, `MINERVA_SESSION_PID`).
   A supervisor loop restarts the daemon if it crashes.
3. **Openbox** starts as the window manager, then the shell is opened as
   three native X windows from one Chromium profile:
   `#desktop` (fullscreen, bottom layer), `#topbar` (always-on-top strip) and
   `#dock` (full-width taskbar flush to the bottom). Each window is the same
   `index.html`; the `#role` hash decides which surface it renders
   (`html[data-mode]` CSS gating).
4. On-demand surfaces (launcher, Wi-Fi, audio, battery, calendar, power,
   notifications, settings) are opened by the daemon as real X windows
   (`POST /api/v1/exec {panel}`) with per-role WM_CLASS; Openbox rules place
   them. They close on blur/Esc like real DE popovers.
5. Every `/api/v1/*` call except health requires the session token
   (`X-Minerva-Token` header or `?token=`). Without X11/Openbox the session
   falls back to a single-window kiosk (`session_mode=kiosk`, cage/weston on
   Wayland).
6. Logging out (`minerva-ctl cmd logout` → daemon kills the session pid),
   closing the desktop window, or a DM logout ends the script; its trap stops
   the panel process, Openbox and the daemon (which persists `session.json`).

## Real system layer (`src/exec.rs`)

The daemon is the system backend — std-only, no crates:

- **Apps**: launches real processes. `.desktop` entries are parsed from
  `/usr/share/applications` (+ user/local dirs), field-codes stripped,
  `Terminal=true` wrapped in a terminal emulator; a built-in candidate table
  (`terminal`, `files`, `web`, …) covers apps without a desktop entry. If no
  real binary exists the daemon reports an install hint and the shell can
  fall back to its JS window (delivered to the desktop window via a pending
  queue). Launched pids are tracked (`/proc/<pid>`) for dock indicators.
- **System ops** (`POST /api/v1/exec {sys}`): volume/mute via pactl→amixer,
  Wi-Fi state/list/toggle/connect via nmcli, resolution list/apply via
  xrandr, brightness via `/sys/class/backlight` + xrandr software fallback,
  night light via xrandr gamma, bluetooth via rfkill/bluetoothctl, apt
  search via apt-cache, apt install via a real terminal running sudo, and
  session power (lock/logout/reboot/shutdown) via loginctl/systemctl.
- **Icons**: `GET /api/v1/icon?name=` resolves and serves bytes from the
  hicolor/gnome/Adwaita themes and pixmaps, with an in-memory cache.

Logs live in `~/.local/state/minerva/session.log`; the systemd user unit
`minerva-sessiond.service` offers the same daemon for headless development.
