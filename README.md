# Debian Minerva

A desktop environment for Debian GNU/Linux, built with **Rust** as the primary
systems language. The UI is a near-1:1 recreation of the leaked
[Aluminium OS](references/aluminium-os/) desktop — top status bar, bottom dock,
app drawer, desktop folders, compact quick settings, virtual desktops, freeform
windows — with **all Google branding removed** and replaced by Debian Minerva.

> **Status:** installable desktop environment (0.6.x, wrapper base). The
> `.deb` adds a **Minerva** session to GDM/LightDM/SDDM. Logging in starts a
> real X11 desktop on proven Debian machinery — xfce4-session + xfwm4
> (openbox fallback) — with the Aluminium-accurate Minerva shell as the one
> fullscreen surface (status bar, taskbar dock, launcher, quick settings,
> notifications, settings, lock screen) and real applications as real
> windows on top. Verify any install with one command: `minerva-verify`
> (component matrix + live session report, saved to ~/minerva-report.txt).

---

## What's in the box

| Layer | Language | What it does |
|---|---|---|
| `minerva-sessiond` | **Rust (std only, zero crates)** | Authoritative session state + the real system layer: spawns real applications (from the installed `.desktop` catalogue), opens shell panels as real X windows, controls volume (pactl), Wi-Fi (nmcli), resolution (xrandr), brightness, night light and session power. HTTP/JSON API, VFS, terminal |
| Session (`minerva-session`) | POSIX sh | Starts Openbox (real WM), the daemon (loopback + per-session token), and the shell windows; supervises crashes; tears down on logout |
| Minerva Shell | HTML / CSS / vanilla JS | Renders each X surface (desktop, top bar, dock, launcher, Wi-Fi/audio/battery/power popovers, notifications, settings). Built-in JS apps act as fallbacks when no real application is installed |
| Reference gallery | — | 18 leaked Aluminium OS desktop screenshots in [`references/aluminium-os/`](references/aluminium-os/) |

### Desktop features

- **Colourful adaptive app icons** — neutral Material-style artwork, no Google logos.
- **Themed battery indicator** — real charge level from `/sys/class/power_supply`, follows the accent colour.
- **Spring-smooth motion** — eased window/panel/launcher transitions with `prefers-reduced-motion` support.
- **Real applications** — the launcher lists every installed `.desktop` app;
  clicking it spawns the real program as a real window managed by Openbox.
- **Real system controls** — Wi-Fi popover scans/connects via nmcli, audio
  popover drives pactl/amixer, Display settings apply real resolutions via
  xrandr, battery popover drives real brightness + night light.
- **Real package management** — the Software app searches the actual Debian
  archive (apt-cache) and installs through a terminal running sudo apt.
- **Top status bar** — clock/date, activities, notifications, Wi-Fi, audio,
  battery, power — each icon opens its own dedicated popover window.
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
- **No fake chrome** — no boot animation, no decorative buttons: everything
  on the bar does something real.

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

## Install on a Debian VM (desktop environment)

```bash
# 1. Build deps + session runtime
sudo apt update
sudo apt install -y git build-essential devscripts debhelper cargo rustc \
                    openbox chromium network-manager x11-xserver-utils
# Debian 12's apt rustc (1.63) is too old for some features; if the build
# complains, use rustup instead:
#   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
#   source "$HOME/.cargo/env"

# 2. Clone and build the .deb
git clone https://github.com/infiniti-ux/Debian-Minerva.git
cd Debian-Minerva
make deb                      # dpkg-buildpackage -us -uc -b

# 3. Install it
sudo apt install ../debian-minerva_0.2.0_*.deb

# 4. Log out, and at the display manager (GDM/LightDM/SDDM)
#    click the session chooser and pick "Minerva", then log in.
```

The session (`/usr/bin/minerva-session`) starts **Openbox** (window manager)
and the Rust daemon on `127.0.0.1:47801` with a per-session token, autostarts
standard services (notification daemon, clipboard, polkit agent), then opens
ONE fullscreen Minerva shell surface that draws the whole desktop — wallpaper,
top bar, full-width taskbar dock, launcher, popovers, notifications, settings.
Real applications run as ordinary X windows on top and appear in the dock
taskbar (EWMH client list): click focuses, right-click closes.
Keybinds: `Super` launcher, `Ctrl+Alt+T`/`Super+T` terminal, `Super+E` files,
`Super+W` browser, `Super+D` show desktop, `Alt+Tab` native window switching,
`Super+←/→` workspace switching, `Print` screenshot, right-click root menu.
Logs: `~/.local/state/minerva/session.log`.
Without X11/Openbox the session falls back to a single-window kiosk
(cage/weston on Wayland). Product roadmap & recovery guide: `docs/PRODUCT.md`.

### Development commands

```bash
make build                # cargo build --release
make run                  # serve the shell at http://localhost:8080 (dev)
make session              # run the full DE session right here (needs cage/X)
make test                 # cargo test (JSON parser + reducer tests)
make self-test            # reducer smoke test, prints the snapshot
make install              # install to /usr without the .deb
make deb                  # build the installable .deb
```

The daemon has **no external crates** — `Cargo.toml` has an empty
`[dependencies]`. `build.rs` only records the rustc version in the banner.
State is persisted to `$XDG_CONFIG_HOME/debian-minerva/session.json` and the
virtual home lives in `…/debian-minerva/home/`.

### Browser-only mode (no Rust toolchain)

```bash
node tools/preview.mjs --port 8080
```

With no daemon reachable the shell runs the identical API surface in the
browser (localStorage VFS, browser-side reducer) and the dock tray shows
**Preview** instead of **Rust core** — useful for UI development only.

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
| POST | `/api/v1/exec` | real system: launch app/desktop entry, open/close panel windows, session commands (logout/lock/reboot/shutdown), sys ops (volume, wifi, resolution, brightness, night light, apt search/install) |
| GET | `/api/v1/desktop-apps` | real installed applications (scanned `.desktop` catalogue) |
| GET | `/api/v1/running` | real processes launched by the session (dock indicators) |
| GET | `/api/v1/icon?name=` | resolved icon bytes from the system icon theme |

When started with `--token` (the real session always does), every endpoint
except `/api/v1/health` requires `X-Minerva-Token: <token>` (or `?token=`).

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
session/
  minerva-session    DM session script: Openbox + daemon + shell windows
  minerva-ctl        CLI for keybinds/root menu (panel, app, cmd)
  openbox/menu.xml   root menu template
  minerva.desktop    "Minerva" entry for xsessions/
src/
  main.rs        CLI, banner, self-test
  core.rs        SessionState, windows, workspaces, reducer, persistence
  json.rs        dependency-free JSON parser/serializer (+ tests)
  services.rs    VFS, terminal, /proc system info, clock
  exec.rs        real system layer: app spawning, panel windows, nmcli/pactl/xrandr/sysfs, .desktop scan, icon lookup
  http.rs        HTTP/1.1 server, static files, API routing
build.rs         records rustc version
web/
  index.html     shell structure (window roles via #hash)
  css/           full stylesheet (theming, windows, panels, popovers, taskbar)
  js/            icons, api client, app views, panel windows, shell
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
