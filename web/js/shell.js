/* =========================================================================
 * Debian Minerva — desktop shell
 *
 * The shell is a renderer for the Rust session state. It keeps a local mirror
 * for instant interaction (drag, resize, focus) and syncs actions back to
 * minerva-sessiond when that daemon is available. See docs/ARCHITECTURE.md.
 * ========================================================================= */
(function () {
  'use strict';

  var I = window.Icon, API = window.MinervaAPI, Views = window.AppViews;
  var $ = function (s, r) { return (r || document).querySelector(s); };
  var $$ = function (s, r) { return Array.prototype.slice.call((r || document).querySelectorAll(s)); };
  var esc = function (s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]; }); };
  var clamp = function (v, a, b) { return Math.max(a, Math.min(b, v)); };

  // Window role in a real X11 session (each role is its own X window loaded
  // with a #hash): '', 'desktop', 'topbar', 'dock', 'launcher', 'wifi', ...
  var MODE = document.documentElement.dataset.mode || '';
  var X11 = false;          // real multi-window session (set after probe)
  var REAL_APPS = [];       // installed .desktop applications (real catalogue)
  var REAL_RUNNING = {};    // id -> true for real processes launched via daemon

  // ------------------------------------------------------------- app catalog
  var APPS = [
    { id: 'files', name: 'Files', icon: 'folder', group: 'System', w: 880, h: 540, pinned: true, summary: 'Browse your home directory' },
    { id: 'browser', name: 'Minerva Web', icon: 'globe', group: 'Internet', w: 980, h: 620, pinned: true, summary: 'A lightweight web view' },
    { id: 'terminal', name: 'Terminal', icon: 'terminal', group: 'System', w: 760, h: 460, pinned: true, summary: 'Minerva command line' },
    { id: 'editor', name: 'Text Editor', icon: 'edit', group: 'Utilities', w: 760, h: 520, pinned: true, summary: 'Write notes and code' },
    { id: 'settings', name: 'Settings', icon: 'settings', group: 'System', w: 900, h: 580, pinned: true, summary: 'Configure your desktop' },
    { id: 'software', name: 'Software', icon: 'package', group: 'System', w: 900, h: 560, pinned: false, summary: 'Manage Debian packages' },
    { id: 'monitor', name: 'Task Manager', icon: 'activity', group: 'System', w: 820, h: 520, pinned: false, summary: 'Processes and resources' },
    { id: 'calculator', name: 'Calculator', icon: 'calc', group: 'Utilities', w: 360, h: 520, pinned: false, summary: 'Quick arithmetic' },
    { id: 'clock', name: 'Clock', icon: 'clock', group: 'Utilities', w: 460, h: 440, pinned: false, summary: 'Time, timers and alarms' },
    { id: 'calendar', name: 'Calendar', icon: 'calendar', group: 'Utilities', w: 720, h: 520, pinned: false, summary: 'Your schedule' },
    { id: 'notes', name: 'Notes', icon: 'note', group: 'Utilities', w: 600, h: 460, pinned: false, summary: 'Scratchpad' },
    { id: 'gallery', name: 'Wallpapers', icon: 'image', group: 'Utilities', w: 780, h: 520, pinned: false, summary: 'Choose a wallpaper' },
    { id: 'mail', name: 'Mail', icon: 'mail', group: 'Internet', w: 900, h: 560, pinned: false, summary: 'Local mail client' },
    { id: 'music', name: 'Music', icon: 'music', group: 'Media', w: 720, h: 480, pinned: false, summary: 'Local audio player' },
    { id: 'camera', name: 'Camera', icon: 'camera', group: 'Media', w: 640, h: 480, pinned: false, summary: 'Webcam preview' },
    { id: 'help', name: 'Help & About', icon: 'info', group: 'System', w: 700, h: 500, pinned: false, summary: 'About Debian Minerva' },
    { id: 'widgets', name: 'Widgets', icon: 'widgets', group: 'Utilities', w: 760, h: 520, pinned: false, summary: 'Desktop widget picker' },
    { id: 'display', name: 'Displays', icon: 'monitor', group: 'System', w: 700, h: 480, pinned: false, summary: 'Screen layout' }
  ];
  var appById = function (id) { return APPS.filter(function (a) { return a.id === id; })[0] || { id: id, name: id, icon: 'info', w: 720, h: 480, summary: '' }; };
  var appName = function (id) { return appById(id).name; };
  var appIcon = function (id) { return appById(id).icon; };

  // ------------------------------------------------------------------ state
  var STORE = 'minerva.session.v1';
  var S = defaultState();

  function defaultState() {
    return {
      de: 'Debian Minerva', version: '0.1.0', seq: 1,
      theme: 'light', accent: '#a80030', wallpaper: 'swirl', scale: 1,
      volume: 0.65, brightness: 0.82,
      active_workspace: 0, show_overview: false, show_launcher: false,
      toggles: { wifi: true, bluetooth: true, dnd: false, night_light: false, airplane: false, sync: true, location: false, screen_record: false },
      windows: [], workspaces: [{ id: 1, name: 'Desktop 1' }, { id: 2, name: 'Desktop 2' }],
      notifications: [{ id: 1, app: 'system', icon: 'info', title: 'Welcome to Debian Minerva', body: 'Your session is ready. Press Super to open the launcher.', time: 'just now' }],
      pinned: APPS.filter(function (a) { return a.pinned; }).map(function (a) { return a.id; }),
      recent_apps: [], next_id: 10, z_top: 10
    };
  }

  function loadLocal() {
    try {
      var raw = localStorage.getItem(STORE);
      if (!raw) return;
      var v = JSON.parse(raw);
      ['theme', 'accent', 'wallpaper', 'scale', 'volume', 'brightness', 'active_workspace', 'pinned', 'recent_apps', 'toggles'].forEach(function (k) {
        if (v[k] !== undefined) S[k] = v[k];
      });
    } catch (e) {}
  }
  var saveTimer;
  function saveLocal() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(function () {
      try {
        localStorage.setItem(STORE, JSON.stringify({
          theme: S.theme, accent: S.accent, wallpaper: S.wallpaper, scale: S.scale,
          volume: S.volume, brightness: S.brightness, active_workspace: S.active_workspace,
          pinned: S.pinned, recent_apps: S.recent_apps, toggles: S.toggles
        }));
      } catch (e) {}
    }, 250);
  }

  function fromServer(snap) {
    var t = {};
    (snap.toggles || []).forEach(function (x) { t[x.key] = x.value; });
    S = Object.assign(defaultState(), {
      seq: snap.seq || S.seq, theme: snap.theme || 'light', accent: snap.accent || '#a80030',
      wallpaper: snap.wallpaper || 'swirl', scale: snap.scale || 1,
      volume: snap.volume == null ? .65 : snap.volume,
      brightness: snap.brightness == null ? .82 : snap.brightness,
      active_workspace: snap.active_workspace || 0,
      show_overview: !!snap.show_overview, show_launcher: !!snap.show_launcher,
      toggles: Object.keys(t).length ? t : S.toggles,
      windows: (snap.windows || []).map(function (w) { return { id: w.id, app: w.app, title: w.title, x: w.x, y: w.y, w: w.w, h: w.h, z: w.z, state: w.state, workspace: w.workspace, restore: null }; }),
      workspaces: snap.workspaces || S.workspaces,
      notifications: snap.notifications || [],
      pinned: snap.pinned || S.pinned, recent_apps: snap.recent_apps || []
    });
    S.next_id = Math.max(10, S.windows.reduce(function (m, w) { return Math.max(m, w.id); }, 0) + 1);
    S.z_top = Math.max(10, S.windows.reduce(function (m, w) { return Math.max(m, w.z); }, 0));
  }

  // --------------------------------------------------------- local reducer
  function avW() { return window.innerWidth; }
  function avH() { return Math.max(300, window.innerHeight - 88); }

  function focusWin(id) {
    S.z_top += 1;
    var w = S.windows.filter(function (x) { return x.id === id; })[0];
    if (w) { w.z = S.z_top; if (w.state === 'minimized') w.state = 'normal'; }
  }

  function applyLocal(a) {
    S.seq += 1;
    var w, i;
    switch (a.type) {
      case 'launch':
        var def = appById(a.app);
        w = S.windows.filter(function (x) { return x.app === a.app && x.workspace === S.active_workspace; })[0];
        if (w && !a.allow_duplicate) { focusWin(w.id); }
        else {
          var cascade = (S.windows.length % 6) * 26;
          S.z_top += 1;
          S.windows.push({
            id: S.next_id++, app: a.app, title: def.name,
            x: Math.min(90 + cascade, Math.max(20, avW() - def.w - 40)),
            y: 40 + cascade, w: Math.min(def.w, avW() - 60), h: Math.min(def.h, avH() - 40),
            z: S.z_top, state: 'normal', workspace: S.active_workspace, restore: null
          });
        }
        S.show_launcher = false; S.show_overview = false;
        if (S.recent_apps.indexOf(a.app) === -1) { S.recent_apps.unshift(a.app); S.recent_apps = S.recent_apps.slice(0, 8); }
        break;
      case 'focus': focusWin(Number(a.id)); break;
      case 'minimize': w = findWin(a.id); if (w) w.state = 'minimized'; break;
      case 'maximize':
        w = findWin(a.id);
        if (w) { if (w.state === 'maximized') { w.state = 'normal'; if (w.restore) { w.x = w.restore.x; w.y = w.restore.y; w.w = w.restore.w; w.h = w.restore.h; } } else { w.restore = { x: w.x, y: w.y, w: w.w, h: w.h }; w.state = 'maximized'; } focusWin(w.id); }
        break;
      case 'close': S.windows = S.windows.filter(function (x) { return x.id !== Number(a.id); }); destroyWindow(Number(a.id)); break;
      case 'move':
        w = findWin(a.id);
        if (w) { w.x = clamp(Number(a.x), -w.w + 120, avW() - 120); w.y = clamp(Number(a.y), 0, avH() - 40); if (w.state === 'maximized') w.state = 'normal'; }
        break;
      case 'resize':
        w = findWin(a.id);
        if (w) { w.w = Math.max(320, Number(a.w)); w.h = Math.max(220, Number(a.h)); w.state = 'normal'; }
        break;
      case 'snap':
        w = findWin(a.id);
        if (w) {
          w.restore = { x: w.x, y: w.y, w: w.w, h: w.h };
          w.state = 'normal';
          if (a.side === 'left') { w.x = 0; w.y = 0; w.w = avW() / 2; w.h = avH(); }
          if (a.side === 'right') { w.x = avW() / 2; w.y = 0; w.w = avW() / 2; w.h = avH(); }
          if (a.side === 'full') { w.state = 'maximized'; }
        }
        break;
      case 'set_title': w = findWin(a.id); if (w) w.title = a.title; break;
      case 'add_workspace':
        S.workspaces.push({ id: Date.now(), name: 'Desktop ' + (S.workspaces.length + 1) });
        S.active_workspace = S.workspaces.length - 1; break;
      case 'remove_workspace':
        i = Number(a.index);
        if (S.workspaces.length > 1 && i < S.workspaces.length) {
          S.workspaces.splice(i, 1);
          S.windows.forEach(function (x) { if (x.workspace === i) x.workspace = 0; else if (x.workspace > i) x.workspace--; });
          S.active_workspace = Math.min(S.active_workspace, S.workspaces.length - 1);
        }
        break;
      case 'switch_workspace': if (Number(a.index) < S.workspaces.length) { S.active_workspace = Number(a.index); S.show_overview = false; } break;
      case 'move_window_to_workspace': w = findWin(a.id); if (w) w.workspace = Number(a.index); break;
      case 'overview': S.show_overview = a.value === undefined ? !S.show_overview : !!a.value; S.show_launcher = false; break;
      case 'launcher': S.show_launcher = a.value === undefined ? !S.show_launcher : !!a.value; S.show_overview = false; break;
      case 'set_theme': S.theme = a.value; break;
      case 'set_accent': S.accent = a.value; break;
      case 'set_wallpaper': S.wallpaper = a.value; break;
      case 'set_scale': S.scale = clamp(Number(a.value), .75, 1.5); break;
      case 'set_volume': S.volume = clamp(Number(a.value), 0, 1); break;
      case 'set_brightness': S.brightness = clamp(Number(a.value), .2, 1); break;
      case 'toggle': S.toggles[a.key] = a.value === undefined ? !S.toggles[a.key] : !!a.value; break;
      case 'notify': S.notifications.unshift({ id: S.next_id++, app: a.app || 'system', icon: a.icon || 'info', title: a.title || '', body: a.body || '', time: 'just now' }); S.notifications = S.notifications.slice(0, 30); break;
      case 'dismiss_notification': S.notifications = S.notifications.filter(function (n) { return n.id !== Number(a.id); }); break;
      case 'clear_notifications': S.notifications = []; break;
      case 'minimize_all': S.windows.forEach(function (x) { x.state = 'minimized'; }); break;
      case 'close_all': S.windows.slice().forEach(function (x) { destroyWindow(x.id); }); S.windows = []; break;
      case 'reset': S.windows.slice().forEach(function (x) { destroyWindow(x.id); }); S = defaultState(); break;
    }
  }
  function findWin(id) { return S.windows.filter(function (x) { return x.id === Number(id); })[0]; }

  var actionTimers = {};
  function dispatch(a, opts) {
    opts = opts || {};
    applyLocal(a);
    render();
    saveLocal();
    var key = a.type + ':' + (a.id || '');
    clearTimeout(actionTimers[key]);
    actionTimers[key] = setTimeout(function () {
      API.action(a).then(function (res) { if (res && res.events && res.events.length) handleEvents(res.events); });
    }, (a.type === 'move' || a.type === 'resize') ? 220 : 0);
  }

  function handleEvents(events) {
    if (!events || !events.length) return;
    var actionTypes = ['launch', 'close', 'set_theme', 'set_accent', 'set_wallpaper', 'switch_workspace', 'reset', 'notify', 'toggle'];
    events.forEach(function (ev) {
      if (actionTypes.indexOf(ev.type) !== -1) applyLocal(ev);
    });
    render();
    saveLocal();
  }

  // ------------------------------------------------------------ window DOM
  var RECORDS = {};   // id -> { el, appId, cleanups, stateListeners, editorLoader }

  function makeCtx(winId) {
    var rec = RECORDS[winId] || (RECORDS[winId] = { cleanups: [], stateListeners: [] });
    var ctx = {
      api: API, apps: APPS, dispatch: dispatch, toast: toast, power: power,
      appName: appName, appIcon: appIcon,
      toggle: function (k) { return !!S.toggles[k]; },
      openApp: openApp,
      termContext: termContext,
      handleEvents: handleEvents,
      openEditor: openEditor,
      registerEditorLoader: function (fn) { rec.editorLoader = fn; },
      onCleanup: function (fn) { rec.cleanups.push(fn); },
      onStateChange: function (fn) { rec.stateListeners.push(fn); },
      setTitle: function (t) { dispatch({ type: 'set_title', id: winId, title: t }); },
      winId: winId
    };
    Object.defineProperty(ctx, 'state', { get: function () { return S; } });
    return ctx;
  }

  function openEditor(path) {
    var editors = S.windows.filter(function (w) { return w.app === 'editor' && w.workspace === S.active_workspace; });
    if (editors.length) {
      var rec = RECORDS[editors[0].id];
      focusWin(editors[0].id);
      if (rec && rec.editorLoader) rec.editorLoader(path);
      render();
    } else {
      pendingEditorPath = path;
      dispatch({ type: 'launch', app: 'editor', allow_duplicate: true });
    }
  }
  var pendingEditorPath = null;

  function openPanelGuarded(role) {
    // Panel windows close themselves on blur; clicking the same top-bar icon
    // again right after must NOT reopen (that would defeat toggle behaviour).
    var now = Date.now(), lastRole = '', lastTs = 0;
    try {
      lastRole = localStorage.getItem('minerva.lp.role') || '';
      lastTs = Number(localStorage.getItem('minerva.lp.ts') || 0);
    } catch (e) {}
    if (lastRole === role && now - lastTs < 900) {
      try { localStorage.removeItem('minerva.lp.role'); } catch (e) {}
      return;
    }
    try { localStorage.setItem('minerva.lp.role', role); localStorage.setItem('minerva.lp.ts', String(now)); } catch (e) {}
    API.openPanel(role);
  }

  function popover(name, sel) {
    if (X11) { openPanelGuarded(name); return; }
    if (window.PanelWin) PanelWin.fill(name, $(sel));
    togglePanel(sel);
  }

  function refreshRunning() {
    if (!X11) return;
    API.running().then(function (list) {
      REAL_RUNNING = {};
      (list || []).forEach(function (a) { REAL_RUNNING[a.id] = true; });
      if (!MODE || MODE === 'dock' || MODE === 'desktop') renderDock();
    });
  }

  function startPolling() {
    var lastSig = '';
    setInterval(function () {
      API.getState().then(function (snap) {
        if (!snap) return;
        var sig = JSON.stringify(snap);
        if (sig === lastSig) return;
        lastSig = sig;
        fromServer(snap);
        render();
      });
    }, 2500);
    setInterval(refreshRunning, 4000);
    if (!MODE || MODE === 'desktop') {
      // JS-window fallbacks requested from panel windows (launcher, dock)
      setInterval(function () {
        API.takePending().then(function (r) {
          if (r && r.apps && r.apps.length) {
            r.apps.forEach(function (id) { dispatch({ type: 'launch', app: id }); });
          }
        });
      }, 2000);
    }
  }

  function openApp(id, arg) {
    if (X11) {
      if (id === 'settings') { openPanelGuarded('settings'); return; }
      API.execApp(id, arg).then(function (r) {
        if (r && r.ok) { refreshRunning(); if (MODE === 'launcher') PanelWin.selfClose(); return; }
        if (r && r.hint) { toast(r.hint); if (MODE === 'launcher') PanelWin.selfClose(); return; }
        if (MODE && MODE !== 'desktop') {
          // panel window: ask the desktop surface to open the JS fallback
          API.execFallback(id);
          if (MODE === 'launcher') PanelWin.selfClose();
        } else {
          dispatch({ type: 'launch', app: id });
        }
      });
      return;
    }
    dispatch({ type: 'launch', app: id });
  }

  function termContext() {
    return {
      apps: APPS, theme: S.theme,
      windows: S.windows.map(function (w) { return { id: w.id, app: w.app, state: w.state }; }),
      workspaces: S.workspaces
    };
  }

  function destroyWindow(id) {
    var rec = RECORDS[id];
    if (!rec) return;
    (rec.cleanups || []).forEach(function (fn) { try { fn(); } catch (e) {} });
    if (rec.el && rec.el.parentNode) rec.el.parentNode.removeChild(rec.el);
    delete RECORDS[id];
  }

  var dragRaf = 0;
  function buildWindow(w) {
    var def = appById(w.app);
    var el = document.createElement('div');
    el.className = 'win';
    el.dataset.id = w.id;
    el.innerHTML =
      '<div class="win-titlebar"><span class="wt-icon">' + I.appArt(w.app, 20) + '</span>' +
      '<span class="wt-title">' + esc(w.title) + '</span><span class="win-controls">' +
      '<button class="win-btn" data-c="min" title="Minimise">' + I.svg('minus', 14) + '</button>' +
      '<button class="win-btn" data-c="max" title="Maximise">' + I.svg('square', 14) + '</button>' +
      '<button class="win-btn close" data-c="close" title="Close">' + I.svg('close', 14) + '</button>' +
      '</span></div><div class="win-body"></div>' +
      '<div class="win-resize e"></div><div class="win-resize w"></div><div class="win-resize s"></div><div class="win-resize se"></div>';
    $('#windows').appendChild(el);
    RECORDS[w.id] = Object.assign(RECORDS[w.id] || {}, { el: el, appId: w.app, cleanups: [], stateListeners: [], editorLoader: null });
    var ctx = makeCtx(w.id);
    var view = Views.create(w.app, ctx);
    var body = $('.win-body', el);
    body.innerHTML = view.html;
    try { view.init(body, { path: pendingEditorPath }); } catch (e) { body.innerHTML = '<div class="app-content"><p>App error: ' + esc(e.message) + '</p></div>'; }
    pendingEditorPath = null;
    I.hydrate(body);

    // controls
    el.querySelector('.win-controls').addEventListener('click', function (e) {
      var b = e.target.closest('[data-c]'); if (!b) return;
      e.stopPropagation();
      var c = b.dataset.c;
      if (c === 'min') dispatch({ type: 'minimize', id: w.id });
      if (c === 'max') dispatch({ type: 'maximize', id: w.id });
      if (c === 'close') dispatch({ type: 'close', id: w.id });
    });
    el.addEventListener('pointerdown', function () { if (w.state !== 'minimized') dispatch({ type: 'focus', id: w.id }, { silent: true }); });
    // double-click titlebar to maximise
    $('.win-titlebar', el).addEventListener('dblclick', function (e) { if (!e.target.closest('.win-controls')) dispatch({ type: 'maximize', id: w.id }); });
    // right-click titlebar
    $('.win-titlebar', el).addEventListener('contextmenu', function (e) { e.preventDefault(); showWindowMenu(e, w.id); });
    setupDrag(el, w.id);
    return el;
  }

  function setupDrag(el, id) {
    var bar = $('.win-titlebar', el);
    var mode = null, start = null, orig = null, snapSide = null;
    var side = '';
    function down(e, kind, which) {
      if (e.button !== 0) return;
      var w = findWin(id); if (!w) return;
      mode = kind; side = which || '';
      start = { x: e.clientX, y: e.clientY };
      orig = { x: w.x, y: w.y, w: w.w, h: w.h };
      try { el.setPointerCapture(e.pointerId); } catch (err) {}
      e.preventDefault();
    }
    bar.addEventListener('pointerdown', function (e) { if (!e.target.closest('.win-controls')) down(e, 'move'); });
    [['e', 'ew', 'e'], ['w', 'ew', 'w'], ['s', 'ns', 's'], ['se', 'both', 'se']].forEach(function (p) {
      $('.win-resize.' + p[0], el).addEventListener('pointerdown', function (e) { down(e, p[1], p[2]); e.stopPropagation(); });
    });
    el.addEventListener('pointermove', function (e) {
      if (!mode) return;
      var w = findWin(id); if (!w) return;
      var dx = e.clientX - start.x, dy = e.clientY - start.y;
      if (mode === 'move') {
        w.x = orig.x + dx; w.y = orig.y + dy;
        snapSide = null;
        if (e.clientX < 12) snapSide = 'left';
        else if (e.clientX > window.innerWidth - 12) snapSide = 'right';
        else if (e.clientY < 50) snapSide = 'full';
        showSnap(snapSide);
      } else {
        if (mode === 'ew' || mode === 'both') w.w = Math.max(320, side === 'w' ? orig.w - dx : orig.w + dx);
        if (mode === 'ns' || mode === 'both') w.h = Math.max(220, orig.h + dy);
        w.state = 'normal';
      }
      if (!dragRaf) {
        dragRaf = requestAnimationFrame(function () {
          dragRaf = 0;
          var ww = findWin(id);
          if (ww) paintWindow(ww);
        });
      }
    });
    el.addEventListener('pointerup', function (e) {
      if (!mode) return;
      var w = findWin(id);
      var wasMove = mode === 'move';
      mode = null; hideSnap();
      if (!w) return;
      if (wasMove && snapSide) dispatch({ type: 'snap', id: id, side: snapSide });
      else if (wasMove) dispatch({ type: 'move', id: id, x: Math.round(w.x), y: Math.round(w.y) });
      else dispatch({ type: 'resize', id: id, w: Math.round(w.w), h: Math.round(w.h) });
      snapSide = null;
    });
  }

  function showSnap(side) {
    var el = $('#snap-preview');
    if (!side) { el.classList.add('hidden'); return; }
    el.classList.remove('hidden');
    var w = avW(), h = avH();
    if (side === 'left') el.style.cssText = 'left:0;top:40px;width:' + (w / 2) + 'px;height:' + h + 'px';
    if (side === 'right') el.style.cssText = 'left:' + (w / 2) + 'px;top:40px;width:' + (w / 2) + 'px;height:' + h + 'px';
    if (side === 'full') el.style.cssText = 'left:0;top:40px;width:' + w + 'px;height:' + h + 'px';
  }
  function hideSnap() { $('#snap-preview').classList.add('hidden'); }

  function paintWindow(w) {
    var rec = RECORDS[w.id]; if (!rec || !rec.el) return;
    var el = rec.el;
    el.classList.toggle('maximized', w.state === 'maximized');
    el.classList.toggle('minimized', w.state === 'minimized');
    el.classList.toggle('focused', isFocused(w));
    if (w.state === 'maximized') {
      el.style.cssText = 'left:0;top:0;width:100%;height:100%;z-index:' + w.z;
    } else {
      el.style.cssText = 'left:' + w.x + 'px;top:' + w.y + 'px;width:' + w.w + 'px;height:' + w.h + 'px;z-index:' + w.z;
    }
    $('.wt-title', el).textContent = w.title;
    el.style.display = (w.workspace === S.active_workspace || S.show_overview) ? '' : 'none';
    var maxBtn = $('[data-c="max"]', el);
    if (maxBtn) maxBtn.innerHTML = I.svg(w.state === 'maximized' ? 'restore' : 'square', 14);
  }
  function isFocused(w) { return w.z === Math.max.apply(null, [0].concat(S.windows.map(function (x) { return x.z; }))); }

  // ------------------------------------------------------------------ render
  var lastWinSig = '';
  function render() {
    document.body.className = 'theme-' + S.theme + ' wallpaper-' + S.wallpaper;
    document.body.style.setProperty('--accent', S.accent);
    document.body.style.setProperty('--accent-soft', hexA(S.accent, .16));
  document.body.style.setProperty('--ui-scale', String(S.scale || 1));
    $('#wallpaper').style.filter = 'brightness(' + (0.55 + S.brightness * .55) + ')';
    renderClock();
    if (!MODE || MODE === 'desktop') { renderDesktopIcons(); syncWindows(); renderOverview(); }
    if (!MODE || MODE === 'dock' || MODE === 'desktop') renderDock();
    if (!MODE || MODE === 'launcher') renderLauncher();
    if (!MODE || MODE === 'shade') renderNotifications();
    $$('.statusbar [data-icon]').forEach(function (e) {});
    I.hydrate(document);
    // notify app-level listeners
    Object.keys(RECORDS).forEach(function (id) {
      var rec = RECORDS[id];
      (rec.stateListeners || []).forEach(function (fn) { try { fn(); } catch (e) {} });
    });
  }


  // ---------------------------------------------------------------- battery
  // The Rust daemon reads /sys/class/power_supply; the browser preview falls
  // back to the Battery Status API. The icon fill follows the theme accent and
  // turns amber/red when the level is low.
  var battery = { level: 0.94, charging: false, present: false };
  function setBattery(b) {
    if (!b) return;
    if (typeof b.level === 'number' && isFinite(b.level)) battery.level = clamp(b.level, 0, 1);
    if (typeof b.charging === 'boolean') battery.charging = b.charging;
    if (typeof b.present === 'boolean') battery.present = b.present;
    renderBattery();
  }
  function batteryFill() {
    if (battery.charging) return '#22c55e';
    if (battery.level <= 0.20) return '#ef4444';
    if (battery.level <= 0.40) return '#f59e0b';
    return getComputedStyle(document.body).getPropertyValue('--accent').trim() || '#a80030';
  }
  function renderBattery() {
    var holder = $('#sb-battery [data-battery]');
    if (!holder) return;
    var pct = Math.round(battery.level * 100);
    var w = Math.max(2, Math.round(battery.level * 15));
    var light = document.body.classList.contains('wallpaper-light');
    var outline = light ? 'rgba(20,22,34,.55)' : 'rgba(255,255,255,.72)';
    holder.innerHTML = '<svg class="battery-svg" viewBox="0 0 26 14" width="26" height="14" aria-hidden="true">' +
      '<rect x="1" y="1.5" width="21" height="11" rx="3" fill="none" stroke="' + outline + '" stroke-width="1.3"/>' +
      '<rect x="23.4" y="5" width="2.2" height="4" rx="1.1" fill="' + outline + '"/>' +
      '<rect x="3" y="3.5" width="' + w + '" height="7" rx="1.8" fill="' + batteryFill() + '"/>' +
      (battery.charging ? '<path d="M13 2.5l-4 5.2h3l-1 3.8 4.6-5.4h-3z" fill="#fff" stroke="none"/>' : '') +
      '</svg>';
    $('#sb-batt-num').textContent = pct + '%' + (battery.charging ? '+' : '');
    $('#sb-battery').title = 'Battery ' + pct + '%' + (battery.charging ? ' (charging)' : '');
  }
  function initBattery() {
    // native daemon
    if (API.backend === 'rust') {
      API.getSystem().then(function (sys) {
        if (!sys) return;
        setBattery({ level: sys.battery_level, charging: sys.battery_charging, present: sys.battery_present });
      }).catch(function () {});
    }
    // browser fallback / live updates
    try {
      if (navigator.getBattery) {
        navigator.getBattery().then(function (b) {
          battery.present = true;
          function upd() { setBattery({ level: b.level, charging: b.charging, present: true }); }
          upd();
          b.addEventListener('levelchange', upd);
          b.addEventListener('chargingchange', upd);
        }).catch(function () {});
      }
    } catch (e) {}
    renderBattery();
  }

  function hexA(hex, a) {
    var h = hex.replace('#', '');
    if (h.length === 3) h = h.split('').map(function (c) { return c + c; }).join('');
    var n = parseInt(h, 16);
    return 'rgba(' + ((n >> 16) & 255) + ',' + ((n >> 8) & 255) + ',' + (n & 255) + ',' + a + ')';
  }

  function syncWindows() {
    var ids = S.windows.map(function (w) { return w.id; });
    Object.keys(RECORDS).forEach(function (id) {
      if (ids.indexOf(Number(id)) === -1) destroyWindow(Number(id));
    });
    S.windows.forEach(function (w) {
      if (!RECORDS[w.id] || !RECORDS[w.id].el) buildWindow(w);
      paintWindow(w);
    });
  }

  function renderClock() {
    var d = new Date();
    var hm = d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' });
    var full = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    $('#sb-time').textContent = hm;
    $('#sb-date').textContent = d.toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric' });
    $('#dw-date').textContent = d.toLocaleDateString([], { weekday: 'short', month: 'short', day: 'numeric' });
    $('#qs-clock').textContent = full;
    $('#qs-date').textContent = d.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric' });
    $('#notif-dot').classList.toggle('hidden', S.notifications.length === 0);
    renderBattery();
  }

  function renderDesktopIcons() {
    var items = [['files', 'My Files'], ['software', 'Software'], ['settings', 'Settings'], ['help', 'About']];
    $('#desktop-icons').innerHTML = items.map(function (it) {
      return '<button class="desktop-icon" data-app="' + it[0] + '"><span class="di-glyph">' + I.appArt(it[0], 32) + '</span><span>' + it[1] + '</span></button>';
    }).join('');
  }

  function renderDock() {
    var running = S.windows.filter(function (w) { return w.workspace === S.active_workspace; });
    var pinned = S.pinned.filter(function (id) { return appById(id); });
    var extra = running.filter(function (w) { return pinned.indexOf(w.app) === -1; }).map(function (w) { return w.app; });
    var list = pinned.concat(extra.filter(function (x, i, a) { return a.indexOf(x) === i; }));
    if (list.indexOf('settings') === -1) list.push('settings');
    var html = '<button class="dock-item" id="dock-launcher" title="App launcher (Super)">' + I.svg('grid', 23) + '</button>' +
      '<span class="dock-sep"></span>' +
      list.map(function (id) {
        var w = running.filter(function (x) { return x.app === id; })[0];
        var isRun = !!w || !!REAL_RUNNING[id];
        return '<button class="dock-item ' + (isRun ? 'running' : '') + ' ' + (w && isFocused(w) && w.state !== 'minimized' ? 'active' : '') + '" data-app="' + id + '" title="' + esc(appName(id)) + '">' +
          I.appArt(id, 28) + '</button>';
      }).join('');
    $('#dock').innerHTML = html;
    I.hydrate($('#dock'));
  }

  function realAppButton(a) {
    var letter = esc((a.name || '?').trim().charAt(0).toUpperCase());
    var glyph = a.icon
      ? '<img class="la-img" src="' + API.iconUrl(a.icon) + '" alt="" onerror="this.style.display=\'none\';this.nextElementSibling.style.display=\'\'">' +
        '<span class="la-letter" style="display:none">' + letter + '</span>'
      : '<span class="la-letter">' + letter + '</span>';
    return '<button class="launcher-app" data-desktop="' + esc(a.id) + '" title="' + esc(a.name) + '">' +
      '<span class="la-glyph">' + glyph + '</span><span>' + esc(a.name) + '</span></button>';
  }

  function renderLauncher() {
    var el = $('#launcher');
    el.classList.toggle('hidden', !S.show_launcher && MODE !== 'launcher');
    if (!S.show_launcher && MODE !== 'launcher') return;
    var q = ($('#launcher-input').value || '').toLowerCase();
    var html = '', count = 0;
    if (X11) {
      var mine = APPS.filter(function (a) { return a.id === 'settings' || a.id === 'software'; })
        .filter(function (a) { return !q || (a.name + ' ' + a.id).toLowerCase().indexOf(q) !== -1; });
      html += mine.map(function (a) {
        return '<button class="launcher-app" data-app="' + a.id + '"><span class="la-glyph">' + I.appArt(a.id, 34) + '</span><span>' + esc(a.name) + '</span></button>';
      }).join('');
      count += mine.length;
      var real = REAL_APPS.filter(function (a) { return !q || (a.name + ' ' + a.id).toLowerCase().indexOf(q) !== -1; });
      html += real.map(realAppButton).join('');
      count += real.length;
    } else {
      var list = APPS.filter(function (a) { return !q || (a.name + ' ' + a.id + ' ' + a.summary).toLowerCase().indexOf(q) !== -1; });
      html = list.map(function (a) {
        return '<button class="launcher-app" data-app="' + a.id + '"><span class="la-glyph">' + I.appArt(a.id, 34) + '</span><span>' + esc(a.name) + '</span></button>';
      }).join('');
      count = list.length;
    }
    $('#launcher-grid').innerHTML = html || '<div class="launcher-empty">No apps match “' + esc(q) + '”</div>';
    $('#launcher-count').textContent = count + ' apps';
    $('#launcher-title').textContent = q ? 'Search results' : (X11 ? 'Installed apps' : 'All apps');
  }

  var QS_TILES = [
    ['wifi', 'Wi-Fi', 'wifi'], ['bluetooth', 'Bluetooth', 'bluetooth'],
    ['dnd', 'Do not disturb', 'moon'], ['night_light', 'Night light', 'sun'],
    ['airplane', 'Airplane', 'plane'], ['sync', 'Sync', 'sync'],
    ['location', 'Location', 'location'], ['screen_record', 'Record', 'record']
  ];
  function renderQS() {
    $('#qs-brightness').value = Math.round(S.brightness * 100);
    $('#qs-volume').value = Math.round(S.volume * 100);
    $('#notif-dnd').classList.toggle('on', !!S.toggles.dnd);
    var srec = $('#sb-record'); if (srec) srec.classList.toggle('active', !!S.toggles.screen_record);
    $('#qs-tiles').innerHTML = QS_TILES.map(function (t) {
      var on = !!S.toggles[t[0]];
      return '<button class="qs-tile ' + (on ? 'on' : '') + '" data-tile="' + t[0] + '"><span class="qt-icon">' + I.svg(t[2], 18) + '</span>' + t[1] + '</button>';
    }).join('');
    I.hydrate($('#qs-tiles'));
  }

  function renderNotifications() {
    var list = S.notifications;
    $('#notif-list').innerHTML = list.length ? list.map(function (n) {
      return '<div class="notif-card"><div class="nc-icon">' + I.svg(n.icon || 'info', 18) + '</div>' +
        '<div style="flex:1"><strong>' + esc(n.title) + '</strong><p>' + esc(n.body) + '</p><small>' + esc(n.app) + ' · ' + esc(n.time) + '</small></div>' +
        '<button class="icon-btn nc-close" data-dismiss="' + n.id + '">' + I.svg('close', 13) + '</button></div>';
    }).join('') : '<div class="notif-empty">You’re all caught up</div>';
    I.hydrate($('#notif-list'));
  }

  function renderOverview() {
    var el = $('#overview');
    el.classList.toggle('hidden', !S.show_overview);
    if (!S.show_overview) return;
    var wallUrl = 'assets/wallpapers/' + S.wallpaper + '.svg';
    $('#ov-workspaces').innerHTML = S.workspaces.map(function (ws, i) {
      var wins = S.windows.filter(function (w) { return w.workspace === i; });
      var minis = wins.map(function (w) {
        return '<span class="ov-ws-mini" style="left:' + (w.x / avW() * 100) + '%;top:' + (w.y / avH() * 100) + '%;width:' + (w.w / avW() * 100) + '%;height:' + (w.h / avH() * 100) + '%"></span>';
      }).join('');
      return '<div class="ov-ws ' + (i === S.active_workspace ? 'active' : '') + '" data-ws="' + i + '" style="background-image:url(' + wallUrl + ')">' + minis +
        '<span class="ov-ws-label">' + esc(ws.name) + (wins.length ? ' · ' + wins.length : '') + '</span>' +
        (S.workspaces.length > 1 ? '<button class="icon-btn" data-rmws="' + i + '" style="position:absolute;right:2px;top:2px;color:#fff">' + I.svg('close', 13) + '</button>' : '') + '</div>';
    }).join('') + '<button class="ov-ws-add" id="ov-add" title="Add workspace">+</button>';
    var q = ($('#ov-search').value || '').toLowerCase();
    var wins = S.windows.filter(function (w) { return !q || (appName(w.app) + ' ' + w.title).toLowerCase().indexOf(q) !== -1; });
    $('#ov-windows').innerHTML = wins.map(function (w) {
      return '<div class="ov-win" data-win="' + w.id + '" draggable="true">' +
        '<div class="ov-win-bar">' + I.appArt(w.app, 18) + esc(w.title) + '</div>' +
        '<div class="ov-win-body">Workspace ' + (w.workspace + 1) + ' · ' + esc(w.state) + '</div>' +
        '<button class="icon-btn ov-win-close" data-close="' + w.id + '">' + I.svg('close', 13) + '</button></div>';
    }).join('') || '<p style="color:#aab3d0;font-size:12px">No open windows</p>';
    I.hydrate($('#overview'));
  }

  // ------------------------------------------------------------- UI helpers
  function toast(msg) {
    var d = document.createElement('div');
    d.className = 'toast'; d.textContent = msg;
    $('#toasts').appendChild(d);
    setTimeout(function () { d.style.opacity = '0'; d.style.transition = 'opacity .3s'; setTimeout(function () { if (d.parentNode) d.parentNode.removeChild(d); }, 320); }, 2600);
  }

  function closePanels() {
    ['#qs', '#notif-panel', '#context-menu', '#power-menu', '#calendar-pop',
     '#search-results', '#wifi-pop', '#audio-pop', '#battery-pop', '#power-pop'].forEach(function (sel) {
      var el = $(sel); if (el) el.classList.add('hidden');
    });
  }
  function togglePanel(id) {
    var el = $(id), wasHidden = el.classList.contains('hidden');
    closePanels();
    if (wasHidden) el.classList.remove('hidden');
    if (id === '#qs') renderQS();
    if (id === '#notif-panel') renderNotifications();
    if (id === '#calendar-pop') renderCalendar();
    return !wasHidden;
  }

  function renderCalendar() {
    var d = new Date(), y = d.getFullYear(), m = d.getMonth();
    var first = new Date(y, m, 1).getDay(), days = new Date(y, m + 1, 0).getDate();
    var html = '<div class="cal-head"><strong>' + d.toLocaleDateString([], { month: 'long', year: 'numeric' }) + '</strong></div><div class="cal-grid">' +
      ['S', 'M', 'T', 'W', 'T', 'F', 'S'].map(function (x) { return '<span class="hdr">' + x + '</span>'; }).join('');
    for (var i = 0; i < first; i++) html += '<span></span>';
    for (var day = 1; day <= days; day++) html += '<span class="' + (day === d.getDate() ? 'today' : '') + '">' + day + '</span>';
    $('#calendar-pop').innerHTML = html + '</div>';
  }

  function showMenu(html, x, y, sel) {
    var el = $(sel);
    el.innerHTML = html; I.hydrate(el);
    el.classList.remove('hidden');
    var r = el.getBoundingClientRect();
    el.style.left = Math.min(x, window.innerWidth - r.width - 8) + 'px';
    el.style.top = Math.min(y, window.innerHeight - r.height - 8) + 'px';
  }

  function showDesktopMenu(x, y) {
    showMenu([
      ['Wallpaper & style', 'palette', 'settings'],
      ['Widgets', 'widgets', 'widgets'],
      ['Apps list', 'grid', 'launcher'],
      ['sep'],
      ['New folder', 'plus', 'newfolder'],
      ['Change wallpaper', 'image', 'gallery'],
      ['Display settings', 'display', 'display'],
      ['sep'],
      ['Terminal', 'terminal', 'terminal']
    ].map(function (i) {
      return i[0] === 'sep' ? '<div class="ctx-sep"></div>' :
        '<button class="ctx-item" data-m="' + i[2] + '">' + I.svg(i[1], 16) + i[0] + '</button>';
    }).join(''), x, y, '#context-menu');
  }

  function showWindowMenu(e, id) {
    showMenu([
      '<button class="ctx-item" data-wm="min">' + I.svg('minus', 16) + 'Minimise</button>',
      '<button class="ctx-item" data-wm="max">' + I.svg('square', 16) + 'Maximise</button>',
      '<div class="ctx-sep"></div>'
    ].concat(S.workspaces.map(function (ws, i) {
      return '<button class="ctx-item" data-wm="ws" data-i="' + i + '">' + I.svg('monitor', 16) + 'Move to ' + esc(ws.name) + '</button>';
    })).concat([
      '<div class="ctx-sep"></div>',
      '<button class="ctx-item" data-wm="close">' + I.svg('close', 16) + 'Close</button>'
    ]).join(''), e.clientX, e.clientY, '#context-menu');
    $('#context-menu').dataset.win = id;
  }

  function power(action) {
    if (X11) { API.cmd(action); return; }
    if (action === 'lock') {
      var l = $('#lock-screen');
      l.innerHTML = '<div class="lock-clock" id="lock-clock"></div><div class="lock-date" id="lock-date"></div><img src="assets/logo.svg" alt=""><div style="font-size:12px;color:#8f9dc4;margin-top:10px">Click anywhere to unlock</div>';
      l.classList.remove('hidden');
      var tick = function () { var d = new Date(); $('#lock-clock').textContent = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }); $('#lock-date').textContent = d.toLocaleDateString([], { weekday: 'long', month: 'long', day: 'numeric' }); };
      tick(); l.dataset.timer = setInterval(tick, 1000);
      return;
    }
    if (action === 'reboot') { location.reload(); return; }
    if (action === 'shutdown') { document.body.innerHTML = '<div style="position:fixed;inset:0;background:#000;color:#555;display:grid;place-items:center;font:13px sans-serif">It is now safe to turn off your computer.</div>'; return; }
    if (action === 'logout') { location.reload(); return; }
  }

  // ---------------------------------------------------------------- events
  function wire() {
    // statusbar — every icon opens its own dedicated popover with real data
    $('#sb-clock').addEventListener('click', function () { popover('calendar', '#calendar-pop'); });
    $('#sb-notif').addEventListener('click', function () { if (X11) openPanelGuarded('shade'); else togglePanel('#notif-panel'); });
    $('#sb-wifi').addEventListener('click', function () { popover('wifi', '#wifi-pop'); });
    $('#sb-audio').addEventListener('click', function () { popover('audio', '#audio-pop'); });
    $('#sb-battery').addEventListener('click', function () { popover('battery', '#battery-pop'); });
    $('#sb-power').addEventListener('click', function (e) {
      e.stopPropagation();
      if (X11) { openPanelGuarded('power'); return; }
      showMenu([
        '<button class="ctx-item" data-p="lock">' + I.svg('lock', 16) + 'Lock screen</button>',
        '<button class="ctx-item" data-p="logout">' + I.svg('logout', 16) + 'Log out</button>',
        '<div class="ctx-sep"></div>',
        '<button class="ctx-item" data-p="reboot">' + I.svg('refresh', 16) + 'Restart</button>',
        '<button class="ctx-item" data-p="shutdown">' + I.svg('power', 16) + 'Shut down</button>'
      ].join(''), window.innerWidth - 230, 42, '#power-menu');
    });
    $('#sb-activities').addEventListener('click', function () { if (X11) openPanelGuarded('launcher'); else dispatch({ type: 'overview' }); });
    $('#qs-power').addEventListener('click', function () { power('shutdown'); });

    // quick settings
    $('#qs-tiles').addEventListener('click', function (e) {
      var b = e.target.closest('[data-tile]'); if (!b) return;
      var k = b.dataset.tile;
      if (k === 'dnd' && !S.toggles.dnd) toast('Do not disturb is on');
      dispatch({ type: 'toggle', key: k });
    });
    $('#qs-brightness').addEventListener('input', function (e) { dispatch({ type: 'set_brightness', value: Number(e.target.value) / 100 }); });
    $('#qs-volume').addEventListener('input', function (e) { dispatch({ type: 'set_volume', value: Number(e.target.value) / 100 }); });
    $('#qs-settings').addEventListener('click', function () { closePanels(); openApp('settings'); });
    $('#qs-edit').addEventListener('click', function () { toast('Tile editor is not available in this build'); });

    // notifications
    $('#notif-list').addEventListener('click', function (e) {
      var b = e.target.closest('[data-dismiss]'); if (b) dispatch({ type: 'dismiss_notification', id: Number(b.dataset.dismiss) });
    });
    $('#notif-clear').addEventListener('click', function () { dispatch({ type: 'clear_notifications' }); });
    $('#notif-close').addEventListener('click', function () { if (MODE === 'shade') { PanelWin.selfClose(); } else { $('#notif-panel').classList.add('hidden'); } });
    $('#notif-dnd').addEventListener('click', function () { dispatch({ type: 'toggle', key: 'dnd' }); });

    // dock
    $('#dock').addEventListener('click', function (e) {
      if (e.target.closest('#dock-launcher')) {
        if (X11) openPanelGuarded('launcher');
        else { dispatch({ type: 'launcher' }); setTimeout(function () { $('#launcher-input').focus(); }, 30); }
        return;
      }
      var b = e.target.closest('[data-app]'); if (!b) return;
      var w = S.windows.filter(function (x) { return x.app === b.dataset.app && x.workspace === S.active_workspace; })[0];
      if (w && isFocused(w) && w.state !== 'minimized') dispatch({ type: 'minimize', id: w.id });
      else openApp(b.dataset.app);
    });

    // launcher
    $('#launcher-input').addEventListener('input', renderLauncher);
    $('#launcher-input').addEventListener('keydown', function (e) {
      if (e.key === 'Enter') {
        var first = $('#launcher-grid [data-app], #launcher-grid [data-desktop]');
        if (!first) return;
        if (first.dataset.desktop) { API.execDesktop(first.dataset.desktop).then(function (r) { if (r && r.ok) refreshRunning(); if (MODE === 'launcher') PanelWin.selfClose(); }); }
        else openApp(first.dataset.app);
      }
    });
    $('#launcher-grid').addEventListener('click', function (e) {
      var d = e.target.closest('[data-desktop]');
      if (d) {
        API.execDesktop(d.dataset.desktop).then(function (r) {
          if (r && r.ok) refreshRunning();
          else if (r && !r.ok) toast('Could not launch ' + d.dataset.desktop);
          if (MODE === 'launcher') PanelWin.selfClose();
        });
        return;
      }
      var b = e.target.closest('[data-app]'); if (b) openApp(b.dataset.app);
    });

    // desktop icons + search + context menu
    $('#desktop-icons').addEventListener('click', function (e) { var b = e.target.closest('[data-app]'); if (b) openApp(b.dataset.app); });
    $('#desktop-icons').addEventListener('dblclick', function (e) { var b = e.target.closest('[data-app]'); if (b) openApp(b.dataset.app); });
    $('#desktop-search').addEventListener('click', function (e) { if (!e.target.closest('input')) $('#ds-input').focus(); });
    $('#ds-input').addEventListener('input', renderSearch);
    $('#ds-input').addEventListener('keydown', function (e) {
      if (e.key === 'Enter') { var first = $('#search-results [data-app]'); if (first) openApp(first.dataset.app); $('#search-results').classList.add('hidden'); $('#ds-input').value = ''; }
      if (e.key === 'Escape') { $('#search-results').classList.add('hidden'); $('#ds-input').blur(); }
    });
    $('#search-results').addEventListener('click', function (e) { var b = e.target.closest('[data-app]'); if (b) { openApp(b.dataset.app); $('#search-results').classList.add('hidden'); $('#ds-input').value = ''; } });
    $('#desktop-layer').addEventListener('contextmenu', function (e) { e.preventDefault(); showDesktopMenu(e.clientX, e.clientY); });
    $('#desktop-layer').addEventListener('click', function (e) { if (!e.target.closest('.desktop-icon') && !e.target.closest('.desktop-search')) closePanels(); });

    // overview
    $('#ov-close').addEventListener('click', function () { dispatch({ type: 'overview', value: false }); });
    $('#ov-search').addEventListener('input', renderOverview);
    $('#ov-workspaces').addEventListener('click', function (e) {
      var rm = e.target.closest('[data-rmws]');
      if (rm) { e.stopPropagation(); dispatch({ type: 'remove_workspace', index: Number(rm.dataset.rmws) }); return; }
      if (e.target.closest('#ov-add')) { dispatch({ type: 'add_workspace' }); return; }
      var ws = e.target.closest('[data-ws]'); if (ws) dispatch({ type: 'switch_workspace', index: Number(ws.dataset.ws) });
    });
    $('#ov-windows').addEventListener('click', function (e) {
      var c = e.target.closest('[data-close]'); if (c) { dispatch({ type: 'close', id: Number(c.dataset.close) }); return; }
      var w = e.target.closest('[data-win]'); if (w) { dispatch({ type: 'focus', id: Number(w.dataset.win) }); dispatch({ type: 'overview', value: false }); }
    });
    $('#ov-windows').addEventListener('dragstart', function (e) { var w = e.target.closest('[data-win]'); if (w) e.dataTransfer.setData('text/plain', w.dataset.win); });
    $('#ov-workspaces').addEventListener('dragover', function (e) { e.preventDefault(); });
    $('#ov-workspaces').addEventListener('drop', function (e) {
      e.preventDefault();
      var ws = e.target.closest('[data-ws]'); var id = e.dataTransfer.getData('text/plain');
      if (ws && id) { dispatch({ type: 'move_window_to_workspace', id: Number(id), index: Number(ws.dataset.ws) }); toast('Moved to ' + S.workspaces[Number(ws.dataset.ws)].name); }
    });

    // menus (delegated on document)
    document.addEventListener('click', function (e) {
      var m = e.target.closest('#context-menu [data-m]');
      if (m) {
        var a = m.dataset.m;
        $('#context-menu').classList.add('hidden');
        if (a === 'launcher') dispatch({ type: 'launcher', value: true });
        else if (a === 'newfolder') { toast('Created “New folder” on the desktop'); dispatch({ type: 'notify', app: 'system', icon: 'folder', title: 'New folder', body: 'A new folder was created on the desktop.' }); }
        else openApp(a);
        return;
      }
      var wm = e.target.closest('#context-menu [data-wm]');
      if (wm) {
        var id = Number($('#context-menu').dataset.win), act = wm.dataset.wm;
        $('#context-menu').classList.add('hidden');
        if (act === 'min') dispatch({ type: 'minimize', id: id });
        if (act === 'max') dispatch({ type: 'maximize', id: id });
        if (act === 'close') dispatch({ type: 'close', id: id });
        if (act === 'ws') dispatch({ type: 'move_window_to_workspace', id: id, index: Number(wm.dataset.i) });
        return;
      }
      var p = e.target.closest('#power-menu [data-p]');
      if (p) { $('#power-menu').classList.add('hidden'); power(p.dataset.p); return; }
      if (!e.target.closest('.context-menu') && !e.target.closest('.panel') && !e.target.closest('.launcher') && !e.target.closest('.statusbar') && !e.target.closest('#overview')) {
        $('#context-menu').classList.add('hidden'); $('#power-menu').classList.add('hidden');
      }
    });

    // lock screen
    $('#lock-screen').addEventListener('click', function () {
      this.classList.add('hidden');
      clearInterval(Number(this.dataset.timer));
    });

    // keyboard
    document.addEventListener('keydown', function (e) {
      var tag = (e.target.tagName || '').toLowerCase();
      var typing = tag === 'input' || tag === 'textarea' || e.target.isContentEditable;
      if (e.key === 'Escape') {
        if (S.show_launcher) { dispatch({ type: 'launcher', value: false }); return; }
        if (S.show_overview) { dispatch({ type: 'overview', value: false }); return; }
        closePanels(); return;
      }
      if (typing) return;
      if (X11) return; // openbox owns Super, Alt+Tab, Super+D/W in a real session
      if (e.key === 'Meta' || (e.key === 'Super')) { e.preventDefault(); dispatch({ type: 'launcher' }); setTimeout(function () { if (S.show_launcher) $('#launcher-input').focus(); }, 30); return; }
      if (e.key.toLowerCase() === 'w' && (e.metaKey || e.superKey)) { e.preventDefault(); dispatch({ type: 'overview' }); return; }
      if (e.key.toLowerCase() === 'd' && (e.metaKey || e.superKey)) { e.preventDefault(); dispatch({ type: 'minimize_all' }); return; }
      if (e.altKey && e.key === 'Tab') {
        e.preventDefault();
        var ws = S.windows.filter(function (w) { return w.workspace === S.active_workspace && w.state !== 'minimized'; });
        if (ws.length) {
          ws.sort(function (a, b) { return b.z - a.z; });
          var next = ws[e.shiftKey ? ws.length - 1 : (ws.length > 1 ? 1 : 0)];
          dispatch({ type: 'focus', id: next.id });
        }
        return;
      }
      if (e.ctrlKey && e.altKey && e.key.toLowerCase() === 't') { e.preventDefault(); openApp('terminal'); return; }
      if (e.ctrlKey && e.altKey && (e.key === 'ArrowRight' || e.key === 'ArrowLeft')) {
        e.preventDefault();
        var d = e.key === 'ArrowRight' ? 1 : -1;
        var n = (S.active_workspace + d + S.workspaces.length) % S.workspaces.length;
        dispatch({ type: 'switch_workspace', index: n });
        toast(S.workspaces[n].name);
        return;
      }
      if (e.key === 'F11') {
        e.preventDefault();
        var f = S.windows.filter(function (w) { return isFocused(w) && w.workspace === S.active_workspace; })[0];
        if (f) dispatch({ type: 'maximize', id: f.id });
      }
    });

    window.addEventListener('resize', function () {
      S.windows.forEach(function (w) { if (w.state !== 'maximized') { w.w = Math.min(w.w, avW() - 20); w.h = Math.min(w.h, avH() - 20); } });
      render();
    });
  }

  function renderSearch() {
    var q = ($('#ds-input').value || '').trim().toLowerCase();
    var el = $('#search-results');
    if (!q) { el.classList.add('hidden'); return; }
    var matches = APPS.filter(function (a) { return (a.name + ' ' + a.id + ' ' + a.summary).toLowerCase().indexOf(q) !== -1; });
    el.innerHTML = matches.slice(0, 8).map(function (a) {
      return '<button class="sr-item" data-app="' + a.id + '">' + I.appArt(a.id, 22) + '<span>' + esc(a.name) + '</span><small>' + esc(a.group) + '</small></button>';
    }).join('') || '<div class="sr-item muted">No results for “' + esc(q) + '”</div>';
    I.hydrate(el);
    el.classList.remove('hidden');
  }

  // ------------------------------------------------------------------ start
  async function start() {
    loadLocal();
    wire();
    I.hydrate(document);
    await API.probe();
    X11 = API.sessionMode === 'x11';
    document.body.classList.toggle('session-x11', X11);

    // When the Rust daemon owns the state, a terminal command may mutate the
    // session server-side (e.g. `open settings`). Reconcile once afterwards.
    var nativeTerminal = API.terminal;
    API.terminal = async function (cmd, c) {
      var res = await nativeTerminal(cmd, c);
      if (API.backend === 'rust') {
        try {
          var snap = await API.getState();
          if (snap) { fromServer(snap); render(); }
        } catch (e) {}
      }
      return res;
    };

    if (API.backend === 'rust') {
      try {
        var snap = await API.getState();
        if (snap) fromServer(snap);
        var apps = await API.getApps();
        if (apps && apps.length) APPS.length = 0, apps.forEach(function (a) { APPS.push(a); });
      } catch (e) {}
    }
    if (X11) {
      try { REAL_APPS = await API.desktopApps(); } catch (e) {}
      startPolling();
      refreshRunning();
    }
    render();
    setInterval(renderClock, 1000);
    initBattery();
    if (window.PanelWin) PanelWin.init(MODE);
  }

  // Panel windows (panelwin.js) drive their content through this surface.
  window.MinervaShell = {
    dispatch: dispatch, toast: toast, power: power, openApp: openApp,
    renderCalendar: renderCalendar,
    state: function () { return S; },
    mode: function () { return MODE; },
    isX11: function () { return X11; }
  };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
