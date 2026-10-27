# Roadmap

## Phase 1 — Shell prototype ✅ (this repository)

- [x] Rust session daemon: reducer, JSON, VFS, terminal, system info, HTTP API
- [x] Full desktop shell mirroring the Aluminium OS leaks without Google
- [x] Window manager: drag, resize, snap, minimise/maximise, focus
- [x] Virtual desktops + Activities overview
- [x] Quick settings, notifications, app launcher, dock, status bar
- [x] 18 built-in applications
- [x] Theming, SVG wallpapers, persistence, boot + setup wizard
- [x] Reference screenshot gallery with sources
- [x] Debian packaging skeleton + systemd unit + XSession entry

## Phase 2 — Native compositor

- [ ] Replace the browser renderer with a **Smithay**-based Wayland compositor
      in Rust (`minerva-compositor`), reusing `SessionState` verbatim
- [ ] Replace HTTP with a Wayland + D-Bus IPC layer (keep the JSON protocol as
      a debug/remote-control interface)
- [ ] XDG shell protocol: toplevel, popup, positioning, activation
- [ ] Layer shell for status bar / dock / panels
- [ ] Input: libinput, pointer constraints, keyboard shortcuts, gestures
- [ ] Output: DRM/KMS, multi-monitor, HiDPI, fractional scaling
- [ ] GTK/Adwaita-style native app toolkit or keep web apps via a WebKit GTK
      runtime (no Google components)

## Phase 3 — Debian integration

- [ ] Real `.deb` built by `dpkg-buildpackage` / sbuild
- [ ] Greeter integration (GDM / greetd), `minerva-session` login
- [ ] systemd user targets, logind, polkit, power management
- [ ] NetworkManager + BlueZ + PipeWire + UPower bindings in Rust
- [ ] Real power actions, screen locking, idle dimming
- [ ] Accessibility (AT-SPI), screen reader, high contrast
- [ ] Localisation (gettext), at least en + id

## Phase 4 — App ecosystem

- [ ] Flatpak / APT integration for third-party apps
- [ ] Rust-native app SDK (window, menu, notification, settings APIs)
- [ ] Task manager backed by real `/proc` processes
- [ ] Terminal backed by a PTY (not a command emulator)

## Non-goals

- Bundling any Google service, Play Services, Chrome, Gemini or proprietary
  blobs
- Tracking the Aluminium OS UI beyond design inspiration
- Replacing Debian's packaging or init system
