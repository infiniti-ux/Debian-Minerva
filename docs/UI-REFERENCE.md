# UI reference notes

Derived from the 18 leaked screenshots in `references/aluminium-os/`.
These are the concrete details recreated in Minerva's shell.

## Two leak waves

1. **January 2026 — Google Issue Tracker bug report** (`ALOS`, Android 16,
   HP Elite Dragonfly Chromebook, build `ZL1A.260119.001.A1`).
   Shows split-screen Chrome windows, Play Store, top status bar with time +
   date + screen-record indicator + `EN` + Wi-Fi + battery, and a
   full-width white bottom taskbar with centred icons.
2. **May 2026 — Mystic Leaks** (Android 17, MacBook Pro via UTM, 16-minute
   video + screenshot gallery). Shows the home screen with desktop icons,
   Google search bar, app drawer, compact Quick Settings, notification shade,
   widget picker, volume panel, right-click desktop context menu, windowed
   Settings, and "Android version 17".

## Elements copied

- **Top status bar**: left = time/date; right = status glyphs + battery %.
- **Bottom bar**: white full-width strip, centred icon row with a grid/app
  drawer button at the left and a separator before running apps.
- **Desktop**: date + weather widget top-left, centred pill search bar,
  desktop shortcuts (Play Store + Google folder → Minerva shortcuts).
- **Right-click menu**: Wallpaper & style / Widgets / Apps list / Home
  settings / New folder.
- **App drawer**: centred rounded panel, search field, 6-column app grid.
- **Quick Settings**: compact panel from the top-right, two sliders
  (brightness, volume), 2×4 tile grid, settings + edit footer.
- **Notifications**: separate compact panel, stacked cards, silent section.
- **Windows**: light title bar, icon + title on the left, minimise / maximise /
  close on the right, freeform resizing, 50:50 split-screen snapping.
- **Virtual desktops**: integrated into the Recents / Activities overview.
- **Settings**: two-pane layout with sidebar, windowed, Material-3 styling.

## Elements intentionally changed

- Google Search → **Minerva Search** (searches local apps only, no network)
- Play Store → **Software** (local app catalogue)
- Chrome/Chrome Dev → **Minerva Web** (bundled start page, no network)
- Android version / build strings → Debian Minerva version + `/proc` kernel
- All wallpapers regenerated as procedural SVGs (no leaked binary assets
  shipped in the product)
- Debian red (`#a80030`) as the default accent

## Known gaps in the leaks

Virtual desktops, the task manager, terminal and touchpad gestures are mostly
visible in the 16-minute video rather than still screenshots, so those parts
of Minerva are designed from the written descriptions in the press coverage.
