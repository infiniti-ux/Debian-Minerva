# Packaging notes

The installable package is built from `../debian/` (debhelper 13, native
format 3.0). From the repo root:

```bash
make deb        # dpkg-buildpackage -us -uc -b  →  ../debian-minerva_0.2.0_<arch>.deb
sudo apt install ../debian-minerva_0.2.0_*.deb
```

What lands on the system:

| Path | Purpose |
|---|---|
| `/usr/bin/minerva-sessiond` | Rust session daemon |
| `/usr/bin/minerva-session` | DM session script (supervision + kiosk runtime) |
| `/usr/share/minerva/web/` | Shell assets served by the daemon |
| `/usr/share/wayland-sessions/minerva.desktop` | "Minerva" session entry (Wayland DMs) |
| `/usr/share/xsessions/minerva.desktop` | "Minerva" session entry (X11 DMs) |
| `/usr/lib/systemd/user/minerva-sessiond.service` | Optional headless dev unit |

Runtime recommends (from `debian/control`): `cage` + `chromium`/`firefox-esr`;
`weston` suggested as fallback. `make install` installs the same layout
without building a package (PREFIX/DESTDIR honoured).
