# Debian Minerva — product & architecture roadmap

Status: **v0.4.0** (session v3, single-surface X11). License: MIT. No Google
components, no telemetry, no bundled AI.

This document maps the full "build your own desktop environment" programme
(compositor → shell → session → services → packaging → QA) onto what Minerva
ships today, what lands in v0.4.x, and what is reserved for the v1.0
Wayland milestone. Debian is the base OS and package/driver provider; every
Minerva-specific layer (shell, session, settings/services daemon, theme) is
our own code. We do **not** wrap GNOME or XFCE as the desktop.

## Fase 0 — product decisions

| # | Decision | Choice |
|---|----------|--------|
| 1 | DE name | Minerva (package `debian-minerva`) |
| 2 | Target users | Debian users wanting a modern, calm, Aluminium-style desktop without Google services |
| 3 | Target devices | old PCs, laptops, workstations (lightweight X11 stack; ~150 MB RSS session) |
| 4 | Base | Debian stable (13 "trixie") |
| 5 | Architecture | amd64 (any-arch build; arm64 works, untested) |
| 6 | Release model | periodic releases (0.x now, 1.0 with Wayland compositor) |
| 7 | "From scratch" level | own shell, own session manager, own settings/services daemon, own theme; window-manager component = Openbox on X11 for v0.4 (see below); own Wayland compositor = v1.0 milestone |
| 8 | Allowed foundation libraries | libX11-class system tools via CLI (xprop/wmctrl/xrandr/pactl/nmcli), PipeWire/PulseAudio, NetworkManager, CUPS — system services, not desktops |
| 9 | Main language | Rust (daemon, dependency-free) + POSIX sh (session) + JS/HTML/CSS (shell surface) |
| 10 | UI toolkit for panel/settings | Minerva shell surface (web runtime in kiosk window); settings is a first-party shell view backed by the Rust daemon |
| 11 | Wayland-only? | No — v0.4 is X11; v1.0 adds the Wayland compositor with XWayland |
| 12 | Separate X11 mode? | Yes, X11 is the v0.4 production mode; kiosk/Wayland-cage is the rescue fallback |
| 13 | Accessibility | reduced-motion support, UI scale 0.75–1.5, keyboard-driven launcher & window keys; full a11y audit in v1.0 |
| 14 | Localisation | en + id strings in shell; full i18n table in v1.0 |
| 15 | Security | per-session token on the local API (mode-600 env file), loopback-only bind, no remote endpoints |
| 16 | License | MIT |
| 17 | Team | single-maintainer project + contributors |
| 18 | Milestones | 0.4.x hardening → 0.5 multi-monitor & i18n → 1.0 Wayland compositor |
| 19 | "Production ready" definition | installs from .deb on clean Debian 13, logs in from GDM, terminal/files/browser/settings all functional, survives logout/reboot, diagnostics on-screen |
| 20 | Vision doc | this file |

### Why v0.4 ships X11 and the compositor is v1.0

The user mandate is explicit: X11, not Wayland ("asal bukan Wayland lemot"),
and the reference hardware is a VMware VM with software GL. A wlroots-class
compositor also cannot be built or validated in this project's offline
toolchain today. So v0.4 uses Openbox as the window-manager *component*
(the same category as using wlroots: a WM library/process, not a finished
desktop), while shell, session, settings, services, theme and packaging are
all first-party. The from-scratch compositor (output/seat/surface
management, layer-shell, XWayland) is the v1.0 milestone and is tracked
below, not abandoned.

## Fase 1–2 — architecture & Debian foundation

* Compositor/WM (v0.4): Openbox process, EWMH-compliant; Minerva session
  writes its config and theme at login (`~/.config/openbox/rc.xml`,
  `~/.local/share/themes/Minerva`). v1.0: replaced by `minerva-comp`.
* Shell/panel: one fullscreen surface (`chromium --kiosk`) rendering
  wallpaper, top bar, taskbar dock, launcher, popovers, notification shade,
  settings — the Aluminium-OS replica UI.
* Session manager: `/usr/bin/minerva-session` (daemon watchdog, WM start +
  watchdog, cleanup) + `/usr/bin/minerva-autostart` on the stock Openbox
  `--startup` hook + `.desktop` session entry for GDM/LightDM/SDDM. The
  bootstrap follows proven Debian session conventions adapted from the
  Openbox project (openbox-session/autostart) and startxfce4: dbus-launch
  guard, environment files, xrdb, EWMH root cleanup, XDG_CURRENT_DESKTOP,
  XDG autostart chain (openbox-autostart / openbox-xdg-autostart / dex).
* Settings daemon: Rust `minerva-sessiond` — volume (pactl/amixer), Wi-Fi
  (nmcli), resolution/brightness/night-light (xrandr), theme/wallpaper/
  scale state, apt search/install, power (logind).
* Notification daemon: dunst for app notifications + first-party shade UI.
* Clipboard: parcellite autostart. Lock screen: logind lock + DM greeter.
* Power: logind via session commands (suspend/reboot/shutdown/lock).
* Audio/network/Bluetooth/printer: PipeWire-Pulse, NetworkManager, bluez,
  CUPS — system services configured by the settings daemon, not replaced.
* Multi-monitor/workspace/keybinding/theme architectures: Openbox native
  (4 desktops, Alt+Tab, Alt+drag), shell theme engine (2 themes, 6 accents,
  5 wallpapers).
* Internal protocol: loopback HTTP + token (`/api/v1/*`), plus a shell event
  queue (`/api/v1/events`) for WM-originated actions (Super key).
* Foundation items 41–55: Debian stable base, standard repos, locale/TZ/
  keyboard left to the installer; Minerva writes only its own config dirs
  (`~/.config/openbox`, `~/.local/share/themes/Minerva`,
  `~/.local/state/minerva`) and a mode-600 runtime env file.

## Fase 3 — Wayland compositor (v1.0 milestone, tracked)

56–75 are the v1.0 work programme: core compositor (`minerva-comp`, Rust +
wlroots bindings or C shim), output/monitor management, surfaces, seat and
input (keyboard/pointer/touchpad/touch), clipboard, drag-and-drop, XWayland,
layer-shell for the panel surface, idle-inhibit, screencopy, fractional
scaling, hotplug, fallback modes, crash handling, logging, config file.
Not started; v0.4 deliberately ships X11 (see rationale above).

## Fase 4 — window management (v0.4: Openbox-native, shipped)

Floating windows, focus, 4 workspaces, move/resize (Alt+drag and Openbox
menus), fullscreen, maximize/minimize, per-app rules, key/mouse bindings,
basic iconify animation off (calm desktop), dialog/popup handling by the WM.
Tiling and window grouping: v1.0 compositor feature.

## Fase 5 — shell UI (shipped, v0.3→v0.4)

Shell, status panel, clock/battery/network/audio/workspace/tray widgets,
app menu, launcher, notification popups, control centre, settings app,
logout/shutdown dialog, wallpaper manager, shell theme, icon set, fonts,
panel layout + config: all first-party in the shell surface. Multi-monitor
shell and shell i18n: v0.5.

## Fase 6 — session & services (shipped, v0.4)

Session manager, autostart (dunst, parcellite, polkit agent), notification
daemon, clipboard manager, polkit agent, power manager (logind), audio/
network services integration, screen lock, logging (`session.log` +
on-screen failure toast), crash recovery (daemon watchdog + kiosk rescue
mode), session config, systemd user unit, display-manager integration.
Update notifier & crash reporter: v0.5.

## Fase 7 — default applications (wired in v0.4)

| Role | Default (Recommends) | Fallbacks probed |
|------|---------------------|------------------|
| terminal | xfce4-terminal | x-terminal-emulator, gnome-terminal, xterm (Depends) |
| files | pcmanfm | thunar, nautilus, nemo, caja |
| editor | mousepad | gedit, pluma, geany |
| browser | chromium | firefox-esr |
| images | viewnior | feh, eog |
| archive | xarchiver | file-roller, engrampa |
| pdf | atril | evince, okular |
| screenshot | scrot | import (ImageMagick) |
| calculator | galculator | gnome-calculator |
| clipboard | parcellite | — |
| notifications | dunst | — |

GTK/Qt/icon/cursor/font theming of third-party apps: v0.5 (ship a Minerva
GTK theme + icon theme; v0.4 keeps distro defaults so apps stay functional).
MIME defaults, file picker, desktop portals: v0.5/v1.0.

## Fase 8 — packaging & branding (shipped)

Single .deb (`debian-minerva`) acting as meta-package (Depends/Recommends
encode the base+desktop+app package lists), session entry, theme, branding
(name, logo glyphs, about page), installation/user/developer docs
(README.md, docs/), changelog, recovery guide (below). Distro-level items
(live ISO, installer branding, bootloader theme, signed apt repo) are
out of scope by decision: Minerva is a DE installed on Debian, not a
Debian remix; a signed repo and live image are post-1.0 options.

## Fase 9 — QA & production

Checked every release: `dash -n` on all shell scripts, `node --check` on all
JS, tree-sitter parse of all Rust, jsdom shell smoke suite, session E2E with
stubbed X binaries, clean-VM install + login + terminal/files/browser/
settings + logout/reboot. On the machine itself, `minerva-doctor` (root menu
or terminal) prints an OK/FAIL report for daemon, WM, shell surface, browser,
default apps, services and logs — the first thing to run and paste when
anything misbehaves. Hardware matrix (suspend/resume, multi-monitor,
Bluetooth, printers), performance and security audits: v0.5/v1.0 gates.
Release artefacts: tagged .deb + checksums (post-1.0 signing key).

## Recovery guide (user-facing)

* Black screen after login: Ctrl+Alt+F3 → TTY login →
  `cat ~/.local/state/minerva/session.log`; `sudo apt install chromium openbox`
  if missing; `reboot`.
* Shell surface died but WM alive: `minerva-ctl app web` still works; log
  out/in restarts everything.
* Reset Minerva config: `rm ~/.config/openbox/rc.xml ~/.local/share/themes/Minerva -rf`
  (regenerated at next login; a pre-Minerva backup is kept as
  `rc.xml.pre-minerva`).
* Remove Minerva completely: `sudo apt remove --purge debian-minerva`.

## 0.6.x — wrapper base architecture (current)

The session machinery is delegated to proven Debian components; Minerva owns
the surface and the system layer only:

- **xfce4-session + xfwm4** run the session (autostart, polkit, save-session,
  window management). xfdesktop / xfce4-panel / xfce4-screensaver /
  xfce4-power-manager are masked with standard `Hidden=true` autostart
  overrides; **openbox** is the fallback base, a single kiosk window the last
  resort.
- **minerva-session** starts the daemon (mode exported before daemon start),
  writes xfconf per-channel defaults, masks XFCE parts, starts the base, then
  starts **minerva-shell-start** directly (browser pick, kiosk flags, respawn
  loop, loud self-check: report file + xmessage on any missing subsystem).
- **Lock** is a real locker: the shell draws the lock surface, the daemon
  flips `$XDG_RUNTIME_DIR/minerva/locked`, and the session enforcer restacks
  the shell above every window while the flag exists.
- Desktop services (dunst, parcellite, polkit agent) run as systemd
  **user** units wanted by `graphical-session.target`, enabled for every
  user through a vendor preset; `minerva-shell-start` only spawns them as
  a fallback when the user unit is not active.
- Platform contracts exported session-wide: `XDG_CURRENT_DESKTOP=Minerva`,
  portal preference (`minerva-portals.conf` → GTK backend), per-toolkit
  X11 enforcement (`GDK_BACKEND`, `QT_QPA_PLATFORM`, …), system MIME
  defaults via postinst.
- Evidence tooling: `minerva-bootstrap --check/--fix` (install matrix,
  now including portal/user-unit/preset/MIME rows), `minerva-doctor`
  (live session + platform-contract checks), `minerva-verify` (both plus
  the exported contract env, teed to ~/minerva-report.txt).
- Recipe parity with the canonical "build a DE from scratch" tutorials and
  Omarchy-style builds is mapped step by step in `docs/PARITY.md`.
