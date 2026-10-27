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
| `/usr/bin/minerva-sessiond` | Rust session daemon (state + real system layer) |
| `/usr/bin/minerva-session` | DM session script (Openbox + daemon + shell windows) |
| `/usr/bin/minerva-ctl` | CLI used by keybinds/root menu (panel, app, cmd) |
| `/usr/share/minerva/web/` | Shell assets served by the daemon |
| `/usr/share/minerva/openbox/menu.xml` | Root menu template |
| `/usr/share/xsessions/minerva.desktop` | "Minerva" session entry |
| `/usr/lib/systemd/user/minerva-sessiond.service` | Optional headless dev unit |

Hard deps (from `debian/control`): `openbox`, `curl`. Recommends: `chromium`,
`network-manager`, `x11-xserver-utils`. Suggests: `cage`, `weston` (kiosk
fallback without X11). `make install` installs the same layout without
building a package (PREFIX/DESTDIR honoured).
