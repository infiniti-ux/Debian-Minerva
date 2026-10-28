# Minerva vs "build a DE from scratch" tutorial parity

How the canonical recipe, the freedesktop platform contracts, and Minerva's
components line up — including the contracts layer added in 0.9.0 that the
tutorials never mention but every real desktop ships.

Sources the user pointed at, read for this release:

- Debian Wiki — *DesktopEnvironment*: a DE is a coherent suite; its parts are
  the **desktop environment surface**, a **window manager**, a **display
  manager** entry and a **display server** session, plus theming and default
  applications.
- LearnLinuxTV community thread *"How to create your own Desktop Environment"*
  (the request thread behind the YouTube series the user linked): the recipe
  is assembling proven components — xsessions entry, session script, window
  manager, panel, launcher, notification daemon, autostart, theming.
- *Omarchy* (the Hyprland/Arch setup the user compared us to): same recipe,
  different parts — Wayland compositor + waybar + rofi + dunst, wrapped in one
  opinionated install script.

Minerva follows the identical recipe on Debian + X11 (project spec: X11 now,
own Wayland compositor is the v1.0 milestone; "asal bukan Wayland lemot").

| # | Tutorial step | Minerva component |
|---|---------------|-------------------|
| 1 | `.desktop` session entry in `/usr/share/xsessions` | `session/minerva.desktop` |
| 2 | Session bootstrap script (dbus guard, env, resources) | `/usr/bin/minerva-session` |
| 3 | Session manager (autostart, save-session, polkit) | `xfce4-session` (wrapper base, 0.6.0) |
| 4 | Window manager (EWMH, real windows) | `xfwm4` (fallback: `openbox`) |
| 5 | Panel / bar | Minerva shell top bar + taskbar dock (Aluminium surface) |
| 6 | Application launcher / menu | shell launcher sheet, `Super` via xfconf/openbox binds |
| 7 | Notification centre | shell notification shade + `dunst` for app `notify-send` |
| 8 | Autostart machinery | XDG autostart through xfce4-session; XFCE's own panel/desktop masked with `Hidden=true` overrides |
| 9 | Clipboard manager | `parcellite`, started by `minerva-shell-start` |
| 10 | Polkit authentication agent | polkit-gnome agent, started by `minerva-shell-start` |
| 11 | Theming / wallpaper | shell wallpapers + Aluminium panels; xfwm4/openbox window chrome themes; `dunstrc` in Aluminium colours |
| 12 | Default applications & launching | Rust daemon `app_def` table + real `.desktop` exec parsing |
| 13 | Power / lock / suspend | Rust daemon sys layer + shell lock screen (`Super+L`) |
| 14 | Install/verify everything in one shot | `minerva-bootstrap --check / --fix` (component matrix) + `minerva-doctor` (live session report) |

What differs from Omarchy on purpose: Debian instead of Arch, X11 instead of
Hyprland/Wayland (spec + VMware friendliness), and the bar/launcher/notifications
are our own Aluminium-accurate shell surface instead of waybar+rofi — that
surface is the product; the plumbing underneath is Debian's proven parts.

## Platform contracts (0.9.0) — the layer tutorials skip

Tutorials stop at "assemble components"; a production desktop also honors
freedesktop contracts. Minerva now ships them:

| Contract | Implementation |
|---|---|
| Desktop identity | `XDG_CURRENT_DESKTOP=Minerva` + `XDG_SESSION_DESKTOP=Minerva` exported session-wide (portals, OnlyShowIn, theme queries) |
| Portals | `xdg-desktop-portal` + GTK backend with Minerva preference file `/usr/share/xdg-desktop-portal/minerva-portals.conf` (file pickers, screenshots, screen share, settings) |
| Service lifecycle | systemd **user** units `minerva-dunst/parcellite/polkit.service` (`WantedBy=graphical-session.target`, restart on failure) + vendor preset `90-minerva.preset`; shell-start spawns only as fallback |
| Toolkit display server | `GDK_BACKEND=x11`, `CLUTTER_BACKEND=x11`, `QT_QPA_PLATFORM=xcb`, `MOZ_ENABLE_WAYLAND=0`, `SAL_USE_VCLPLUGIN=gtk3` — the X11 mandate enforced per process |
| MIME defaults | `/etc/xdg/mimeapps.list` written by postinst (browser/editor/files); user-level file still wins per spec |
| Cursor theme | `XCURSOR_THEME`/`XCURSOR_SIZE` exported to match the xsettings channel |

Verification: `minerva-doctor` prints a `[platform contracts]` section and
`minerva-verify` dumps the exported contract env into the report.
