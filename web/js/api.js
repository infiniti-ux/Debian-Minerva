/* Debian Minerva — API client.
 *
 * When served by `minerva-sessiond` (the Rust daemon) every call goes to the
 * real Rust core. When the shell is opened from a plain static server (e.g. a
 * design preview), the same surface is emulated in the browser so the desktop
 * stays fully usable. `MinervaAPI.backend` reports which one is live.
 */
(function (global) {
  'use strict';

  var backend = 'detecting';
  var listeners = [];

  function notify() { listeners.forEach(function (f) { try { f(backend); } catch (e) {} }); }
  function onBackend(fn) { listeners.push(fn); }

  function timeout(ms) {
    var c = new AbortController();
    setTimeout(function () { c.abort(); }, ms || 4000);
    return c.signal;
  }

  // Session token: the display-manager session mints one per login and hands
  // it to the shell via the launch URL (?token=...). Without it we are in
  // plain development mode and send nothing.
  var SESSION_TOKEN = (function () {
    try {
      var q = new URLSearchParams(location.search).get('token');
      if (q) return q;
    } catch (e) {}
    return window.MINERVA_TOKEN || '';
  })();

  function withToken(headers) {
    var h = Object.assign({}, headers || {});
    if (SESSION_TOKEN) h['X-Minerva-Token'] = SESSION_TOKEN;
    return h;
  }

  async function raw(path, options) {
    var ctl = new AbortController();
    var timer = setTimeout(function () { ctl.abort(); }, 5000);
    var opts = Object.assign({}, options || {}, { signal: ctl.signal });
    opts.headers = withToken(opts.headers);
    try {
      var res = await fetch(path, opts);
      if (!res.ok) throw new Error('HTTP ' + res.status);
      var text = await res.text();
      return text ? JSON.parse(text) : null;
    } finally { clearTimeout(timer); }
  }

  async function post(path, body) {
    return raw(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body || {})
    });
  }

  // ------------------------------------------------------------- offline VFS
  var FS_KEY = 'minerva.vfs.v1';
  var DEFAULT_FILES = {
    '/README.md': '# Welcome to Debian Minerva\n\nThis is your Minerva home directory.\nEverything here is served by the Rust session daemon in native mode.\n',
    '/Documents/notes.md': '# Notes\n\n- Press Super to open the launcher\n- Alt+Tab switches windows\n- Drag a window to the screen edge to snap it\n',
    '/Documents/todo.txt': '1. Explore Debian Minerva\n2. Open the Terminal and type `help`\n3. Customise the wallpaper in Settings\n',
    '/Projects/minerva.txt': 'Debian Minerva — a Rust desktop session for Debian GNU/Linux.\n',
  };

  function readFs() {
    try {
      var raw = localStorage.getItem(FS_KEY);
      if (raw) return JSON.parse(raw);
    } catch (e) {}
    var init = { dirs: ['/Documents', '/Downloads', '/Pictures', '/Projects'], files: {} };
    Object.keys(DEFAULT_FILES).forEach(function (k) { init.files[k] = DEFAULT_FILES[k]; });
    writeFs(init);
    return init;
  }
  function writeFs(v) {
    try { localStorage.setItem(FS_KEY, JSON.stringify(v)); } catch (e) {}
  }
  function norm(p) {
    if (!p || p === '/') return '/';
    p = ('/' + p.replace(/^\/+/, '')).replace(/\/+$/, '');
    if (p.split('/').indexOf('..') !== -1) return '/';
    return p;
  }

  function fsList(path) {
    path = norm(path);
    var fs = readFs(), entries = [], seen = {};
    fs.dirs.forEach(function (d) {
      var parent = d.substring(0, d.lastIndexOf('/')) || '/';
      if (parent === path && !seen[d]) { seen[d] = 1; entries.push({ name: d.split('/').pop(), kind: 'dir', size: 0, ext: '' }); }
    });
    Object.keys(fs.files).forEach(function (f) {
      var parent = f.substring(0, f.lastIndexOf('/')) || '/';
      if (parent === path && !seen[f]) {
        seen[f] = 1;
        var name = f.split('/').pop();
        entries.push({ name: name, kind: 'file', size: fs.files[f].length, ext: (name.split('.').pop() || '').toLowerCase() });
      }
    });
    entries.sort(function (a, b) {
      if (a.kind !== b.kind) return a.kind === 'dir' ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
    return { path: path, entries: entries };
  }

  function fsOp(op, path, content) {
    path = norm(path);
    var fs = readFs();
    if (op === 'list') return fsList(path);
    if (op === 'read') return { content: fs.files[path] === undefined ? '(file not found)' : fs.files[path] };
    if (op === 'write') { fs.files[path] = String(content || ''); writeFs(fs); return { ok: true }; }
    if (op === 'mkdir') { if (fs.dirs.indexOf(path) === -1) fs.dirs.push(path); writeFs(fs); return { ok: true }; }
    if (op === 'remove') {
      delete fs.files[path];
      fs.dirs = fs.dirs.filter(function (d) { return d !== path && d.indexOf(path + '/') !== 0; });
      Object.keys(fs.files).forEach(function (f) { if (f.indexOf(path + '/') === 0) delete fs.files[f]; });
      writeFs(fs);
      return { ok: true };
    }
    throw new Error('unknown file op');
  }

  // ------------------------------------------------------ offline terminal
  function offlineTerminal(cmd, ctx) {
    var parts = (cmd || '').trim().split(/\s+/);
    if (!parts[0]) return { output: '', events: [] };
    var c = parts[0].toLowerCase(), a = parts.slice(1), events = [], out = '';
    var apps = (ctx && ctx.apps) || [];
    var sys = systemFallback();
    switch (c) {
      case 'help': case '?': out = HELP; break;
      case 'clear': out = '\x00clear'; break;
      case 'uname': out = a.indexOf('-a') !== -1 ? 'Linux minerva 6.12.0-minerva x86_64 GNU/Linux' : 'Linux'; break;
      case 'whoami': out = 'user'; break;
      case 'pwd': out = '/home/user'; break;
      case 'date': out = new Date().toUTCString(); break;
      case 'uptime': out = 'up ' + Math.floor(sys.uptime_secs / 3600) + 'h ' + Math.floor(sys.uptime_secs % 3600 / 60) + 'm'; break;
      case 'echo': out = a.join(' '); break;
      case 'ls': out = fsList(a[0] || '/').entries.map(function (e) { return e.kind === 'dir' ? e.name + '/' : e.name; }).join('  ') || '(empty)'; break;
      case 'cat': out = a[0] ? fsOp('read', a[0]).content : 'cat: missing operand'; break;
      case 'mkdir': out = a[0] ? (fsOp('mkdir', a[0]), '') : 'mkdir: missing operand'; break;
      case 'rm': out = a[0] ? (fsOp('remove', a[0]), '') : 'rm: missing operand'; break;
      case 'touch': case 'write': out = a[0] ? (fsOp('write', a[0], a.slice(1).join(' ')), '') : c + ': missing operand'; break;
      case 'apps': out = apps.map(function (x) { return '  ' + x.id.padEnd(12) + ' ' + x.name; }).join('\n'); break;
      case 'open': case 'launch':
        if (!a[0]) { out = 'open: missing app id (try \'apps\')'; break; }
        events.push({ type: 'launch', app: a[0] }); out = 'launched ' + a[0]; break;
      case 'notify': events.push({ type: 'notify', app: 'terminal', icon: 'terminal', title: a[0] || 'Minerva', body: a.slice(1).join(' ') }); out = 'notification queued'; break;
      case 'theme': events.push({ type: 'set_theme', value: a[0] || 'dark' }); out = 'theme set to ' + (a[0] || 'dark'); break;
      case 'accent': events.push({ type: 'set_accent', value: a[0] || '#a80030' }); out = 'accent set to ' + (a[0] || '#a80030'); break;
      case 'wallpaper': events.push({ type: 'set_wallpaper', value: a[0] || 'swirl' }); out = 'wallpaper set to ' + (a[0] || 'swirl'); break;
      case 'workspace': events.push({ type: 'switch_workspace', index: (parseInt(a[0], 10) || 1) - 1 }); out = 'switched workspace'; break;
      case 'workspaces': out = ((ctx && ctx.workspaces) || []).map(function (w, i) { return '  ' + (i + 1) + '. ' + w.name; }).join('\n') || 'no workspaces'; break;
      case 'win': case 'windows': out = (ctx && ctx.windows || []).map(function (w) { return '  #' + w.id + ' ' + w.app.padEnd(12) + ' ' + w.state; }).join('\n') || 'no windows open'; break;
      case 'close': events.push({ type: 'close', id: parseInt(a[0], 10) || 0 }); out = 'closed window #' + (a[0] || '?'); break;
      case 'sysinfo': case 'free':
        out = 'OS       ' + sys.os + '\nKernel   ' + sys.kernel + '\nMemory   ' + sys.mem_used_mb + ' / ' + sys.mem_total_mb + ' MB\nCPUs     ' + sys.cpus + '\nSession  ' + sys.session; break;
      case 'ps': out = 'PID    APP            STATE\n' + ((ctx && ctx.windows || []).map(function (w) { return String(w.id).padEnd(6) + ' ' + w.app.padEnd(14) + ' ' + w.state; }).join('\n') || 'no processes'); break;
      case 'neofetch': case 'minerva': out = '\x00ascii\nUSER     user@minerva\nOS       Debian Minerva\nKERNEL   ' + sys.kernel + '\nSHELL    minerva-sh (Rust)\nDE       Debian Minerva 0.6.1\nTHEME    ' + (ctx && ctx.theme || 'dark') + '\nWINDOWS  ' + (ctx && ctx.windows || []).length; break;
      case 'about': out = 'Debian Minerva 0.6.1 — a Rust desktop session for Debian GNU/Linux.\nNo Google components. Just Debian + Rust.'; break;
      case 'poweroff': case 'logout': out = '\x00poweroff'; break;
      case 'reboot': out = '\x00reboot'; break;
      case 'reset': events.push({ type: 'reset' }); out = 'session reset'; break;
      default: out = "minerva-sh: command not found: " + c + "\nType 'help' for a list of commands.";
    }
    return { output: out, events: events };
  }

  var HELP = [
    'Debian Minerva shell — available commands:', '',
    '  help              this help',
    '  apps              list installed applications',
    '  open <app>        launch an application',
    '  win               list open windows',
    '  close <id>        close a window by id',
    '  workspaces        list virtual desktops',
    '  workspace <n>     switch to desktop n', '',
    '  ls [path]         list files         cat <path>       read a file',
    '  mkdir <path>      create directory   rm <path>        remove a file',
    '  write <p> <text>  write a file       pwd              current directory', '',
    '  theme dark|light  switch theme       accent <hex>     set accent colour',
    '  wallpaper <name>  swirl|aurora|dusk|light|mesh',
    '  notify <title> <body>                push a notification', '',
    '  uname [-a]        kernel info        date             current date',
    '  uptime            session uptime     sysinfo | free   resources',
    '  ps                process list       neofetch         system summary',
    '  echo <text>       print text         clear            clear terminal',
    '  about              about this DE      reset            reset session',
    '  reboot | poweroff  restart / log out'
  ].join('\n');

  function systemFallback() {
    var nav = navigator || {};
    return {
      hostname: 'minerva', kernel: '6.12.0-minerva', os: 'Debian GNU/Linux 13 (trixie)',
      arch: 'x86_64', cpus: nav.hardwareConcurrency || 4,
      mem_total_mb: 8192, mem_used_mb: 2640, mem_percent: 32.2,
      uptime_secs: Math.floor(performance.now() / 1000) + 3600,
      loadavg: '0.42 0.38 0.31', session: 'minerva-shell (browser preview)',
      toolkit: 'Minerva Shell / Rust-compatible core', de: 'Debian Minerva',
      de_version: '0.6.1', rustc: 'rustc (native daemon not attached)',
      battery_level: 0.94, battery_charging: false, battery_present: false
    };
  }

  // ------------------------------------------------------------- public API
  var sessionMode = 'dev';
  async function probe() {
    try {
      var r = await fetch('/api/v1/health', { signal: timeout(1600), headers: withToken({}) });
      var j = await r.json();
      backend = (r.ok && j && j.runtime && j.runtime.indexOf('rust') !== -1) ? 'rust' : 'browser';
      if (backend === 'rust' && j && j.session_mode) sessionMode = j.session_mode;
    } catch (e) { backend = 'browser'; }
    notify();
    return backend;
  }

  // ------------------------------------------------------- real system API
  // These only mean something when the Rust daemon runs inside a real
  // session; in browser-only development they resolve to inert defaults.
  async function exec(body) {
    if (backend !== 'rust') return { ok: false, error: 'offline' };
    try { return await post('/api/v1/exec', body); } catch (e) { return { ok: false, error: String(e) }; }
  }
  function execApp(id, arg) { return exec({ app: id, desktop: id, arg: arg || '' }); }
  function execDesktop(id) { return exec({ desktop: id, app: id.indexOf('.') === -1 ? id : '' }); }
  function execFallback(id) { return exec({ app_fallback: id }); }
  function takePending() { return exec({ pending: 'take' }); }
  function openPanel(role) { return exec({ panel: role }); }
  function closePanels() { return exec({ panel_close: true }); }
  function cmd(name) { return exec({ cmd: name }); }
  function sys(op, value) { return exec(Object.assign({ sys: op }, value || {})); }
  async function desktopApps() {
    if (backend !== 'rust') return [];
    try { var j = await raw('/api/v1/desktop-apps'); return (j && j.apps) || []; } catch (e) { return []; }
  }
  async function running() {
    if (backend !== 'rust') return { apps: [], windows: [] };
    try { var j = await raw('/api/v1/running'); return j || { apps: [], windows: [] }; } catch (e) { return { apps: [], windows: [] }; }
  }
  async function events() {
    if (backend !== 'rust') return [];
    try { var j = await raw('/api/v1/events'); return (j && j.events) || []; } catch (e) { return []; }
  }
  function iconUrl(name) {
    return '/api/v1/icon?name=' + encodeURIComponent(name || '') +
      (SESSION_TOKEN ? '&token=' + encodeURIComponent(SESSION_TOKEN) : '');
  }

  var stateFails = 0;
  async function getState() {
    if (backend === 'rust') {
      try { var s = await raw('/api/v1/state'); stateFails = 0; return s; }
      catch (e) { stateFails++; if (stateFails >= 3) { stateFails = 0; backend = 'browser'; notify(); } }
    }
    return null;
  }

  async function getApps() {
    if (backend === 'rust') {
      try { var j = await raw('/api/v1/apps'); return j && j.apps; } catch (e) {}
    }
    return null;
  }

  async function getSystem() {
    if (backend === 'rust') {
      try {
        var j = await raw('/api/v1/system');
        return j ? Object.assign({}, j.system, { clock: j.clock }) : systemFallback();
      } catch (e) {}
    }
    return systemFallback();
  }

  async function action(a) {
    if (backend === 'rust') {
      try { return await post('/api/v1/action', a); } catch (e) { /* keep local state */ }
    }
    return { ok: true, offline: true, events: [], state: null };
  }

  async function terminal(cmd, ctx) {
    if (backend === 'rust') {
      try {
        var j = await post('/api/v1/terminal', { cmd: cmd });
        return { output: j.output, events: j.events || [] };
      } catch (e) { /* fall through */ }
    }
    return offlineTerminal(cmd, ctx);
  }

  async function files(path) {
    if (backend === 'rust') {
      try { return (await raw('/api/v1/files?path=' + encodeURIComponent(path || '/'))) || fsList(path || '/'); } catch (e) {}
    }
    return fsList(path || '/');
  }

  async function fileOp(op, path, content) {
    if (backend === 'rust') {
      try { return await post('/api/v1/files', { op: op, path: path, content: content || '' }); } catch (e) {}
    }
    return fsOp(op, path, content);
  }

  global.MinervaAPI = {
    probe: probe, onBackend: onBackend,
    get backend() { return backend; },
    get sessionMode() { return sessionMode; },
    getState: getState, getApps: getApps, getSystem: getSystem,
    action: action, terminal: terminal, files: files, fileOp: fileOp,
    execApp: execApp, execDesktop: execDesktop, execFallback: execFallback,
    takePending: takePending, openPanel: openPanel, closePanels: closePanels,
    cmd: cmd, sys: sys, desktopApps: desktopApps, running: running, events: events, iconUrl: iconUrl,
    HELP: HELP, systemFallback: systemFallback
  };
})(window);
