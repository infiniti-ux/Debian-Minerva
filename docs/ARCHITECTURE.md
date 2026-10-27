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
`/usr/share/{wayland-sessions,xsessions}/minerva.desktop`, whose `Exec` is
`/usr/bin/minerva-session`:

1. The script mints a per-session token and starts `minerva-sessiond` bound
   to `127.0.0.1:47801`, serving `/usr/share/minerva/web`. A supervisor loop
   restarts the daemon if it crashes.
2. It waits for `/api/v1/health` (with the token) to answer, then launches the
   shell full-screen: a kiosk browser under X11, otherwise **cage** (preferred
   Wayland kiosk compositor), otherwise **weston**.
3. The shell URL carries `?token=…`; `web/js/api.js` then sends it as
   `X-Minerva-Token` on every API call. Without a valid token the daemon
   answers `401` — the desktop can never be driven by other local processes.
4. When the kiosk runtime exits, the script stops the daemon (persisting
   `session.json` first) and the session ends, returning to the login screen.

Logs live in `~/.local/state/minerva/session.log`; the systemd user unit
`minerva-sessiond.service` offers the same daemon for headless development.
