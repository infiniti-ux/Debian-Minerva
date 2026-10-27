# Debian Minerva

A desktop environment for Debian GNU/Linux, built with **Rust** as the primary
systems language. The UI is a near-1:1 recreation of the leaked
[Aluminium OS](references/aluminium-os/) desktop — top status bar, bottom dock,
app drawer, desktop folders, compact quick settings, virtual desktops, freeform
windows — with **all Google branding removed** and replaced by Debian Minerva.

> **Status:** working Phase-1 prototype. The Rust session daemon + full web
> shell are implemented. A native Wayland compositor is the next milestone
> (see [docs/ROADMAP.md](docs/ROADMAP.md)).

---

## What's in the box

| Layer | Language | What it does |
|---|---|---|
| `minerva-sessiond` | **Rust (std only, zero crates)** | Authoritative session state, window/workspace model, notification bus, settings persistence, virtual filesystem, terminal, system info, HTTP/JSON API |
| Minerva Shell | HTML / CSS / vanilla JS | Renders the desktop, windows, dock, panels and built-in apps. Talks to the daemon over `/api/v1/*` |
| Built-in apps | JS | Files, Terminal, Settings, Text Editor, Task Manager, Calculator, Clock, Calendar, Notes, Wallpapers, Software, Web, Mail, Music, Camera, Widgets, Displays, Help |
| Reference gallery | — | 18 leaked Aluminium OS desktop screenshots in [`references/aluminium-os/`](references/aluminium-os/) |

### Desktop features

- **Top status bar** — clock/date, activity button, sync, location, screen
  record, keyboard layout, assistant, notifications, Wi-Fi, battery, power.
- **Compact Quick Settings** — slides down from the top-right (brightness,
  volume, Wi-Fi, Bluetooth, DND, night light, airplane, sync, location, record).
- **Notification shade** — cards, per-item dismiss, clear all, DND.
- **Bottom dock/taskbar** — pinned apps, running indicators, app drawer,
  activities, backend badge.
- **App launcher** — Material-style panel with search and full app grid.
- **Desktop** — date/weather widget, centred search bar, desktop shortcuts,
  right-click context menu (Wallpaper & style, Widgets, Apps list, New folder…).
- **Window manager** — drag, resize from any edge, minimise/maximise/close,
  double-click titlebar, edge snapping (left / right / full), focus & z-order,
  right-click titlebar menu, Alt+Tab.
- **Virtual desktops** — Activities overview with workspace strip, mini window
  previews, add/remove workspaces, drag windows between desktops.
- **Theming** — light/dark panels, 6 accent colours, 5 SVG wallpapers, scale,
  brightness/volume, all persisted.
- **Terminal** — `help`, `apps`, `open`, `ls`, `cat`, `mkdir`, `write`, `rm`,
  `theme`, `accent`, `wallpaper`, `workspace`, `sysinfo`, `ps`, `neofetch`,
  `notify`, `poweroff` and more. Commands mutate the real Rust session state.
- **Boot + first-run setup wizard** — Debian-flavoured, skippable.

### Keyboard shortcuts

| Keys | Action |
|---|---|
| `Super` | App launcher |
| `Super + W` | Activities / virtual desktops |
| `Alt + Tab` | Switch windows |
| `Ctrl + Alt + T` | Terminal |
| `Ctrl + Alt + ←/→` | Switch workspace |
| `Super + D` | Show desktop |
| `F11` | Toggle maximise |
| `Esc` | Close panels |

---

## Quick start

### Native (Debian)

```bash
sudo apt install cargo rustc
cargo build --release
./target/release/minerva-sessiond --port 8080 --web-root ./web
# open http://localhost:8080
```

Other useful commands:

```bash
make test                 # cargo test (JSON parser + reducer tests)
make self-test            # run the reducer smoke test, print the snapshot
make run                  # build + serve
make install              # install to /usr (binary, web assets, session file)
```

The daemon has **no external crates** — `Cargo.toml` has an empty
`[dependencies]`. `build.rs` only records the rustc version in the banner.
State is persisted to `$XDG_CONFIG_HOME/debian-minerva/session.json` and the
virtual home lives in `…/debian-minerva/home/`.

### Preview without a Rust toolchain

```bash
node tools/preview.mjs --port 8080
# or: python3 -m http.server 8080 --directory web
```

The shell detects the missing daemon and runs the identical API surface in the
browser (localStorage VFS, browser-side reducer, `/proc`-less system info).
The dock tray shows **Preview** instead of **Rust core**. This is how the live
preview in the Arena sandbox is served — no Rust compiler is available there.

Add `?setup=1` to the URL if you want to walk through the optional first-run
setup wizard; by default the preview opens straight onto the desktop.

---

## API

| Method | Path | Description |
|---|---|---|
| GET | `/api/v1/health` | daemon identity |
| GET | `/api/v1/state` | full session snapshot |
| GET | `/api/v1/apps` | application catalogue |
| GET | `/api/v1/system` | `/proc` system info + clock |
| GET | `/api/v1/files?path=/` | VFS listing |
| POST | `/api/v1/files` | `{op, path, content}` — read/write/mkdir/remove |
| POST | `/api/v1/action` | reducer action (launch, move, snap, toggle, …) |
| POST | `/api/v1/terminal` | `{cmd}` → `{output, events, state}` |
| POST | `/api/v1/save` | flush session to disk |

Example:

```bash
curl -s localhost:8080/api/v1/health
curl -s -X POST localhost:8080/api/v1/action \
  -H 'Content-Type: application/json' \
  -d '{"type":"launch","app":"terminal"}'
```

---

## Layout

```
src/
  main.rs        CLI, banner, self-test
  core.rs        SessionState, windows, workspaces, reducer, persistence
  json.rs        dependency-free JSON parser/serializer (+ tests)
  services.rs    VFS, terminal, /proc system info, clock
  http.rs        HTTP/1.1 server, static files, API routing
build.rs         records rustc version
web/
  index.html     shell structure
  css/           full stylesheet (theming, windows, panels, apps)
  js/            icons, api client, app views, shell/window manager
  assets/        SVG logo + 5 procedurally generated wallpapers
debian/          Debian packaging metadata
packaging/       systemd user unit + XSession .desktop
tools/           sandbox preview host
docs/            architecture, roadmap, UI reference notes
references/      leaked Aluminium OS desktop screenshots + gallery
```

---

## Why web tech for the shell?

Aluminium OS itself renders its shell and most of its apps with web technology
(the leak shows web-wrapped Google apps). Minerva follows the same split used by
ChromeOS and several modern Linux desktops: a **Rust core** owns state and
system services, while a lightweight renderer draws pixels with HTML/CSS.
Phase 2 replaces the browser with a Smithay-based Wayland compositor while
keeping the exact same Rust core. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

MIT — see [LICENSE](LICENSE). The reference screenshots in `references/` are
third-party press/leak images kept for design study only; see the sources in
[`references/aluminium-os/README.md`](references/aluminium-os/README.md).
