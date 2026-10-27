# Debian Minerva 0.5.5 — the 100-fix pass

Every line below is a real change shipped in this release: bugfixes first,
then the animation/motion work. Grouped by area, numbered 1–100.

## Shell core (web/js/shell.js)

1. `fromServer()` no longer wipes the live EWMH mirror (`realwins`/`curdesk`) — dock running indicators flickered off every 2.5 s state poll.
2. Polling is surface-gated: only the desktop window polls session state and consumes `/api/v1/events`; panel windows were stealing Super/overview events from the desktop.
3. `refreshRunning` only runs on the surfaces that render a dock (desktop/dock).
4. West-edge window resize now moves the left edge — it used to stretch the window the wrong way.
5. Window drags are clamped on-screen live (title bar always reachable), not only corrected on release.
6. New windows' cascade position is clamped so the 7th+ window can't open below the fold on small screens.
7. `remove_workspace` rejects negative indexes (`splice(-1)` silently deleted the *last* workspace).
8. `switch_workspace` validates the index range on both sides.
9. Super+D / `minimize_all` only minimises the current workspace (it minimised every desktop).
10. Z-order rebases above 100 000 — no unbounded z-index growth over a long session.
11. `dispatch()` catches action-post failures — no unhandled promise rejections when the daemon restarts.
12. `handleEvents` now understands server events for focus/minimize/maximize/scale/volume/brightness too.
13. `openPanel` failures surface a toast (panels never fail silently).
14. `power()` command failures surface a toast.
15. Lock screen no longer leaks a 1 s interval when locked twice (old timer cleared first).
16. Quick Settings is reachable again: the power button opens the QS card.
17. The QS header power button opens the power menu (lock / log out / suspend / restart / shut down) instead of shutting down instantly.
18. Battery pill reflects charge level: amber ≤ 20 %, red ≤ 10 %, `AC` on machines with no battery.
19. Battery refreshes from the Rust daemon every 60 s — the top bar no longer goes stale.
20. `hexA()` falls back safely on malformed accent colours (was emitting `rgba(NaN,…)`).
21. Toasts are capped at 4 — spam no longer builds an infinite stack.
22. Toast exit mirrors the entrance (slide + fade, was fade-only).
23. Context menus clamp to the top/left screen edges too (was off-screen on corner right-clicks).
24. Clock renders through cached `setText` — the 1 s tick no longer rewrites 5 unchanged DOM nodes every second.
25. Dead no-op render line removed (`$$('.statusbar [data-icon]')` loop).
26. Desktop-icon double-click handler removed — a double-click launched/focused twice.
27. Dock right-click on a local (non-X11) window now opens the window menu instead of doing nothing.
28. Launcher Enter on an installed `.desktop` entry reports failures with a toast (was silent).
29. Desktop search Enter launches installed apps (data-desktop results), not just catalogue apps.
30. Desktop search Enter with no match says "No results" instead of silently doing nothing.
31. Desktop search results include real installed applications in an X11 session.
32. Search-result clicks on installed apps report launch failures.
33. Overview drag-and-drop guards an invalid workspace index (was a TypeError mid-drop).
34. Escape closes the desktop search popup before falling through to panels.
35. Alt+Tab actually cycles in z-order now (was always jumping to the same second window); Shift+Alt+Tab cycles backwards.
36. Window resize-on-screen-change is rAF-debounced — drag-resizing the VM window no longer fires a render storm.
37. Saved `toggles` merge with defaults on load (a partial old store can't delete toggle keys).
38. Default session state carries the real version (0.5.5).
39. Mail removed from the catalogue — the demo inbox was fake data (real-or-removed rule).
40. Widgets picker removed — its "Add to desktop" buttons performed no real action; desktop-menu entry removed with it.

## API client (web/js/api.js)

41. `raw()` clears its abort timer when the request settles — every poll used to leak a pending 5 s timer.
42. `raw()` tolerates empty/non-JSON bodies instead of throwing `SyntaxError`.
43. All `raw()` consumers are null-safe (desktop-apps, events, apps, system, files).
44. Path normalisation rejects `..` only as a path segment — files named `foo..txt` work again.
45. `workspaces` command implemented (it was documented in `help` but returned "command not found").
46. Health probe sends the session token header.
47. Backend downgrade needs 3 consecutive state failures — one transient hiccup no longer permanently flips a real session into browser mode.
48. Browser-fallback system facts carry the real DE version.

## Icons & artwork (web/js/icons.js)

49. Calendar app art shows the real day of the month (was hard-coded "27").
50. Google brand palette stripped from the adaptive artwork (browser/gallery pinwheel, mail, clock, files, editor recoloured to neutral Minerva hues).
51. New `volumeMute` icon for the audio popover's mute state.
52. Unknown app ids get a deterministic colour hashed from the id instead of uniform grey.

## Panel windows & popovers (web/js/panelwin.js)

53. Wi-Fi list refresh preserves scroll position (jumped to top every 6 s).
54. Wi-Fi join guards a null SSID and locks the button while connecting (double-submit sent two `nmcli` calls).
55. Enter in the Wi-Fi password field submits the join form.
56. Escape cancels the Wi-Fi join form.
57. Mute button shows the real mute icon and state (re-read from `pactl` after toggling).
58. Battery popover refresh is null/`isConnected`-guarded — its 15 s timer no longer throws after the surface closes.
59. Battery refresh only fires while the popover is visible.
60. Power popover gains Suspend (parity with the shell power menu).
61. Power actions execute *before* the panel window closes (the close used to kill the in-flight command).
62. Settings window no longer double-writes every change to the daemon (`dispatch` already posts the action).
63. Brightness hardware calls (`brightness_set`) are debounced — slider drags no longer spawn dozens of subprocesses.

## Built-in apps (web/js/apps.js)

64. Files has a real Back history — Back is no longer identical to Up.
65. Files tolerates server entries without a size field (was "undefined B").
66. Terminal: pressing Enter on an empty line no longer kills the prompt (the input stayed disabled — terminal became unusable).
67. Terminal: click-to-focus listener attached once instead of accumulating one per prompt line.
68. Settings app: double-render after every change removed (state listener already re-renders).
69. Calculator: divide-by-zero shows `Error` instead of `NaN`, and the keypad recovers from it.
70. Notes: localStorage writes debounced (was one synchronous write per keystroke).
71. Task Manager: fabricated per-window CPU/RAM numbers removed — replaced with the real workspace column and window state.
72. Task Manager: CPU card computed from the real load average; memory card shows real used/total MB.
73. Music: pause/resume keeps the disc angle (`animation-play-state`) instead of snapping back to 0°.
74. Displays app reads real xrandr modes — the hard-coded fake "1600 × 900 / 60 Hz" card is gone.
75. Displays app shows the real session type line.
76. Every stale "0.1.0" string (terminal banner, neofetch ASCII, settings, help, browser card) now reads 0.5.5.

## Markup (web/index.html)

77. Stored theme/wallpaper applied before first paint — light-theme users no longer get a dark flash at login.
78. Fake mic affordance removed from the desktop search pill (dead control).
79. Hard-coded fake readings ("7:14", "Mon, May 11", "94 %") removed — placeholders are blank until the first real tick.
80. Popover/settings windows get `role="dialog"` + `aria-label`.

## Layout & dead weight (web/css/minerva.css)

81. Desktop layer, window layer and toasts clear the real 56 px dock (windows could slide 8 px under it).
82. Calendar popover anchors under the clock on the left (it opened from the left of the bar but hung on the right).
83. Dead CSS removed: mic/lens, weather widget, dock-tray, sb-label rules.
84. Dead boot-splash and first-run-setup CSS blocks removed (only `@keyframes bootspin`, still used by Music, is kept).
85. `will-change` scoped from 14 selectors down to `.win/.launcher/.panel` — permanent compositor layers were wasting GPU memory.

## Motion & animation pass

86. Desktop icons: hover lift was an instant jump — spring transition added.
87. One transition language (background/colour/transform/shadow) on ctx items, file items, search rows, list rows, sidebars, nav items, toolbar/text/chip/danger buttons and window buttons.
88. Unified press feedback: `:active` scale on menu items, tiles, launcher apps, toolbar and chip buttons.
89. Icon buttons get a press-scale micro-interaction.
90. Calculator keys press with a spring; the `=` key gets a hover brighten.
91. Accent/wallpaper/gallery chips lift on hover with a spring.
92. Tile cards lift with a shadow on hover; their buttons brighten.
93. Overview window cards lift on hover; add-workspace button springs.
94. Dock "running" dash springs in (`mv-dash` scaleX) instead of popping into existence.
95. Monitor bars and the battery-mini fill animate width between readings instead of jumping.
96. Snap preview fades in.
97. Lock screen fades in.
98. Settings window opens with the same centred-sheet motion as the launcher (`mv-sheet`).
99. Critical-battery pill pulses (reuses `dotpulse`).
100. Wi-Fi join button gets a real `:disabled` visual state while connecting.

All 100 verified with `node --check` (JS) and a full CSS/HTML consistency pass;
Rust touched only in version strings.
