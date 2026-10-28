/* Debian Minerva — built-in application views.
 * Each app returns HTML plus an init(body, ctx) hook. Apps never touch global
 * shell state directly: they call ctx.dispatch() / ctx.api.* only.
 */
(function (global) {
  'use strict';

  var I = global.Icon;
  var esc = function (s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]; }); };
  var gid = function (p) { return p + '-' + Math.random().toString(36).slice(2, 8); };

  var V = {};

  // ------------------------------------------------------------------ Files
  V.files = function (ctx) {
    var path = '/';
    var hist = [];
    var id = gid('files');
    return {
      html: '<div class="app"><div class="app-toolbar">' +
        '<button class="tb-btn" data-a="back">' + I.svg('arrowLeft', 15) + ' Back</button>' +
        '<button class="tb-btn" data-a="up">' + I.svg('chevronDown', 15) + ' Up</button>' +
        '<button class="tb-btn" data-a="newfolder">' + I.svg('plus', 15) + ' New folder</button>' +
        '<button class="tb-btn" data-a="newfile">' + I.svg('file', 15) + ' New file</button>' +
        '<span class="spacer"></span><button class="tb-btn" data-a="refresh">' + I.svg('refresh', 15) + '</button>' +
        '</div><div class="file-layout"><div class="file-side" id="' + id + '-side"></div>' +
        '<div class="app-content" id="' + id + '-main"></div></div>' +
        '<div class="app-status" id="' + id + '-status">Minerva Files · Rust VFS</div></div>',
      init: function (body) {
        var main = body.querySelector('#' + id + '-main');
        var side = body.querySelector('#' + id + '-side');
        var status = body.querySelector('#' + id + '-status');
        var shortcuts = [
          ['Home', '/', 'home'], ['Documents', '/Documents', 'folder'],
          ['Downloads', '/Downloads', 'download'], ['Pictures', '/Pictures', 'image'],
          ['Projects', '/Projects', 'package']
        ];
        function renderSide() {
          side.innerHTML = shortcuts.map(function (s) {
            return '<button class="file-side-item ' + (path === s[1] ? 'active' : '') + '" data-p="' + s[1] + '">' + I.svg(s[2], 16) + esc(s[0]) + '</button>';
          }).join('');
        }
        function render() {
          renderSide();
          ctx.api.files(path).then(function (data) {
            var crumb = '<div class="file-crumb"><button data-p="/">Home</button>' +
              (path === '/' ? '' : path.split('/').filter(Boolean).map(function (seg, i, arr) {
                return '<span>/</span><button data-p="/' + arr.slice(0, i + 1).join('/') + '">' + esc(seg) + '</button>';
              }).join('')) + '</div>';
            var grid = data.entries.map(function (e) {
              var glyph = e.kind === 'dir' ? 'folder' : (['md', 'txt', 'json', 'rs', 'js', 'css', 'html'].indexOf(e.ext) !== -1 ? 'doc' : (['png', 'jpg', 'svg', 'webp'].indexOf(e.ext) !== -1 ? 'image' : 'file'));
              return '<button class="file-item" data-name="' + esc(e.name) + '" data-kind="' + e.kind + '">' +
                '<span class="fi-glyph">' + I.svg(glyph, 26) + '</span><span>' + esc(e.name) + '</span>' +
                '<small>' + (e.kind === 'dir' ? 'Folder' : (typeof e.size === 'number' ? (e.size < 1024 ? e.size + ' B' : Math.round(e.size / 1024) + ' KB') : '')) + '</small></button>';
            }).join('') || '<p class="muted">This folder is empty.</p>';
            main.innerHTML = crumb + '<div class="file-grid">' + grid + '</div>';
            status.textContent = path + ' · ' + data.entries.length + ' items · ' + (ctx.api.backend === 'rust' ? 'Rust VFS' : 'browser VFS');
          });
        }
        function go(p) { if (p !== path) hist.push(path); path = p; render(); }
        side.addEventListener('click', function (e) { var b = e.target.closest('[data-p]'); if (b) go(b.dataset.p); });
        main.addEventListener('click', function (e) {
          var b = e.target.closest('[data-p], [data-name]'); if (!b) return;
          if (b.dataset.p) { go(b.dataset.p); return; }
          var full = (path === '/' ? '' : path) + '/' + b.dataset.name;
          if (b.dataset.kind === 'dir') go(full);
          else { ctx.openEditor(full); }
        });
        main.addEventListener('dblclick', function (e) { e.preventDefault(); });
        body.querySelector('.app-toolbar').addEventListener('click', function (e) {
          var b = e.target.closest('[data-a]'); if (!b) return;
          var a = b.dataset.a;
          if (a === 'refresh') render();
          if (a === 'back' && hist.length) { path = hist.pop(); render(); }
          if (a === 'up') { path = path === '/' ? '/' : (path.substring(0, path.lastIndexOf('/')) || '/'); render(); }
          if (a === 'newfolder') {
            var name = prompt('New folder name:', 'New folder'); if (!name) return;
            ctx.api.fileOp('mkdir', (path === '/' ? '' : path) + '/' + name).then(render);
          }
          if (a === 'newfile') {
            var fn = prompt('New file name:', 'untitled.txt'); if (!fn) return;
            ctx.api.fileOp('write', (path === '/' ? '' : path) + '/' + fn, '').then(function () { render(); ctx.openEditor((path === '/' ? '' : path) + '/' + fn); });
          }
        });
        render();
      }
    };
  };

  // --------------------------------------------------------------- Terminal
  V.terminal = function (ctx) {
    var id = gid('term');
    var history = [], hidx = 0;
    return {
      html: '<div class="app"><div class="app-toolbar"><span style="font-size:12px;color:var(--text-muted)">minerva-sh — ' +
        (ctx.api.backend === 'rust' ? 'connected to Rust daemon' : 'browser preview') + '</span><span class="spacer"></span>' +
        '<button class="tb-btn" data-a="clear">' + I.svg('trash', 15) + ' Clear</button></div>' +
        '<div class="terminal" id="' + id + '-out"><div class="term-line">Debian Minerva 0.7.1 · minerva-sh</div>' +
        '<div class="term-line muted">Type <b>help</b> for a list of commands.</div><br></div>' +
        '<div class="app-status">Tip: try <b>neofetch</b>, <b>open settings</b>, <b>theme light</b>, <b>wallpaper aurora</b></div></div>',
      init: function (body) {
        var out = body.querySelector('#' + id + '-out');
        var curInput = null;
        out.addEventListener('click', function (e) { if (!e.target.closest('input') && curInput) curInput.focus(); });
        function line(text, cls) {
          var d = document.createElement('div');
          d.className = 'term-line ' + (cls || '');
          d.innerHTML = text;
          out.appendChild(d);
          out.scrollTop = out.scrollHeight;
        }
        function promptLine() {
          var wrap = document.createElement('div');
          wrap.className = 'term-inputline';
          wrap.innerHTML = '<span class="term-prompt">user@minerva</span><span class="term-path">~</span><span>$</span>' +
            '<input autocomplete="off" spellcheck="false">';
          out.appendChild(wrap);
          var input = wrap.querySelector('input');
          input.focus();
          out.scrollTop = out.scrollHeight;
          input.addEventListener('keydown', function (e) {
            if (e.key === 'Enter') {
              var cmd = input.value;
              input.disabled = true;
              line('<span class="term-prompt">user@minerva</span> <span class="term-path">~</span> $ ' + esc(cmd));
              if (cmd.trim()) { history.push(cmd); hidx = history.length; }
              run(cmd);
            } else if (e.key === 'ArrowUp') {
              e.preventDefault(); if (hidx > 0) { hidx--; input.value = history[hidx] || ''; }
            } else if (e.key === 'ArrowDown') {
              e.preventDefault(); if (hidx < history.length) { hidx++; input.value = history[hidx] || ''; }
            }
          });
          curInput = input;
          return input;
        }
        function ascii() {
          return [
            '        <span class="term-ascii">   /\\  </span>        user@minerva',
            '       <span class="term-ascii">  /  \\ </span>       -------------',
            '      <span class="term-ascii"> / /\\ \\</span>       OS      : Debian Minerva',
            '     <span class="term-ascii">/ /  \\ \\</span>      Kernel  : 6.12.0-minerva',
            '    <span class="term-ascii">/_( )_  \\</span>     Shell   : minerva-sh (Rust)',
            '   <span class="term-ascii">\\      /</span>       DE      : Minerva 0.7.1',
            '    <span class="term-ascii">\\____/</span>        Theme   : ' + esc(ctx.state.theme)
          ].join('\n');
        }
        function run(cmd) {
          ctx.api.terminal(cmd, ctx.termContext()).then(function (res) {
            var o = res.output || '';
            if (o.indexOf('\x00') === 0) {
              var code = o.slice(1).split('\n')[0];
              o = o.slice(o.indexOf('\n') + 1);
              if (code === 'clear') out.innerHTML = '';
              if (code === 'poweroff') ctx.power('logout');
              if (code === 'reboot') ctx.power('reboot');
              if (code === 'ascii') line(ascii());
            }
            if (o) o.split('\n').forEach(function (l) { line(esc(l)); });
            if (res.events && res.events.length) ctx.handleEvents(res.events);
            promptLine();
          });
        }
        body.querySelector('.app-toolbar').addEventListener('click', function (e) {
          if (e.target.closest('[data-a="clear"]')) { out.innerHTML = ''; promptLine(); }
        });
        promptLine();
      }
    };
  };

  // --------------------------------------------------------------- Settings
  V.settings = function (ctx) {
    var id = gid('set');
    var pages = [
      ['appearance', 'Appearance', 'palette'], ['wallpaper', 'Wallpaper', 'image'],
      ['system', 'System', 'cpu'], ['network', 'Network', 'wifi'],
      ['displays', 'Displays', 'display'], ['keyboard', 'Keyboard', 'keyboard'],
      ['about', 'About', 'info']
    ];
    var current = 'appearance';
    var WALLS = ['swirl', 'aurora', 'dusk', 'light', 'mesh'];
    var ACCENTS = ['#a80030', '#3b6fe0', '#7c4dff', '#00897b', '#e8710a', '#c2185b'];
    return {
      html: '<div class="app"><div class="settings-layout">' +
        '<div class="settings-side" id="' + id + '-side">' + pages.map(function (p) {
          return '<button class="settings-side-item" data-page="' + p[0] + '">' + I.svg(p[2], 17) + esc(p[1]) + '</button>';
        }).join('') + '</div><div class="settings-main" id="' + id + '-main"></div></div></div>',
      init: function (body) {
        var main = body.querySelector('#' + id + '-main');
        var side = body.querySelector('#' + id + '-side');
        function header(t, s) { return '<h2>' + t + '</h2><div class="sub">' + s + '</div>'; }
        function render() {
          side.querySelectorAll('.settings-side-item').forEach(function (b) { b.classList.toggle('active', b.dataset.page === current); });
          var s = ctx.state;
          var h = '';
          if (current === 'appearance') {
            h = header('Appearance', 'Theme, accent colour and interface scale.') +
              '<div class="settings-card"><h3>Colour theme</h3><div class="theme-list">' +
              ['light', 'dark'].map(function (t) { return '<button class="chip-btn ' + (s.theme === t ? 'active' : '') + '" data-theme="' + t + '">' + (t === 'light' ? '☀ Light' : '☾ Dark') + '</button>'; }).join('') +
              '</div></div>' +
              '<div class="settings-card"><h3>Accent colour</h3><div class="accent-list">' +
              ACCENTS.map(function (a) { return '<button class="accent-chip ' + (s.accent === a ? 'active' : '') + '" style="background:' + a + '" data-accent="' + a + '"></button>'; }).join('') +
              '</div><p class="muted" style="font-size:11px;margin-top:12px">Default is Debian red. Accent applies to panels, buttons and window borders.</p></div>' +
              '<div class="settings-card"><h3>Interface scale</h3><div class="setting-row"><div class="sr-label"><strong>Scale</strong><small>' + Math.round((s.scale || 1) * 100) + '%</small></div>' +
              '<input type="range" min="75" max="150" value="' + Math.round((s.scale || 1) * 100) + '" data-scale style="width:180px"></div></div>';
          } else if (current === 'wallpaper') {
            h = header('Wallpaper', 'Pick a background. All wallpapers ship as SVG with Debian Minerva.') +
              '<div class="settings-card"><h3>Wallpapers</h3><div class="wallpaper-list">' +
              WALLS.map(function (w) { return '<button class="wp-chip ' + (s.wallpaper === w ? 'active' : '') + '" data-wall="' + w + '" style="background-image:url(assets/wallpapers/' + w + '.svg)" title="' + w + '"></button>'; }).join('') +
              '</div></div>';
          } else if (current === 'system') {
            h = header('System', 'Session information and power controls.') +
              '<div class="settings-card"><h3>This session</h3><dl class="kv" id="' + id + '-kv">' +
              '<dt>Desktop</dt><dd>Debian Minerva 0.7.1</dd><dt>Session</dt><dd>minerva (Rust)</dd>' +
              '<dt>Virtual desktops</dt><dd>' + s.workspaces.length + '</dd><dt>Open windows</dt><dd>' + s.windows.length + '</dd>' +
              '<dt>Backend</dt><dd>' + (ctx.api.backend === 'rust' ? 'minerva-sessiond (Rust)' : 'browser preview') + '</dd></dl></div>' +
              '<div class="settings-card"><h3>Danger zone</h3><div class="setting-row"><div class="sr-label"><strong>Reset session</strong><small>Close all windows and restore defaults</small></div><button class="danger-btn" data-reset>Reset</button></div></div>';
          } else if (current === 'network') {
            h = header('Network', 'Connectivity toggles for this session.') +
              '<div class="settings-card"><h3>Devices</h3>' +
              [['wifi', 'Wi-Fi', 'wifi'], ['bluetooth', 'Bluetooth', 'bluetooth'], ['airplane', 'Airplane mode', 'plane'], ['location', 'Location', 'location']].map(function (t) {
                var on = ctx.toggle(t[0]);
                return '<div class="setting-row"><div class="sr-label"><strong>' + t[1] + '</strong><small>' + (on ? 'On' : 'Off') + '</small></div>' +
                  '<button class="switch ' + (on ? 'on' : '') + '" data-toggle="' + t[0] + '"><span></span></button></div>';
              }).join('') + '</div>';
          } else if (current === 'displays') {
            h = header('Displays', 'Screen layout and virtual desktops.') +
              '<div class="settings-card"><h3>Virtual desktops</h3>' +
              s.workspaces.map(function (w, i) { return '<div class="setting-row"><div class="sr-label"><strong>' + esc(w.name) + '</strong><small>' + s.windows.filter(function (x) { return x.workspace === i; }).length + ' windows</small></div>' +
                (i === s.active_workspace ? '<span class="muted">Active</span>' : '<button class="chip-btn" data-ws="' + i + '">Switch</button>') + '</div>'; }).join('') +
              '<div class="setting-row"><div class="sr-label"><strong>Add a workspace</strong><small>Create another canvas</small></div><button class="chip-btn" data-addws>+ Add</button></div></div>';
          } else if (current === 'keyboard') {
            h = header('Keyboard', 'Shortcuts that work everywhere in Minerva.') +
              '<div class="settings-card"><h3>Shortcuts</h3><ul class="shortcut-list">' +
              [['Super', 'App launcher'], ['Super + W', 'Activities overview'], ['Alt + Tab', 'Switch windows'],
               ['Ctrl + Alt + T', 'Terminal'], ['Super + D', 'Show desktop'], ['Ctrl + Alt + ←/→', 'Switch workspace'],
               ['Esc', 'Close panels'], ['F11', 'Toggle maximise']].map(function (x) {
                return '<li><kbd>' + x[0] + '</kbd><span>' + x[1] + '</span></li>';
              }).join('') + '</ul></div>';
          } else if (current === 'about') {
            h = '<div class="about-hero"><img src="assets/logo.svg" alt=""><div><h1>Debian Minerva</h1>' +
              '<p>A Rust-built desktop environment for Debian GNU/Linux.</p><p>Version 0.7.1 · MIT licensed</p></div></div>' +
              '<div class="settings-card"><h3>About this desktop</h3><p style="font-size:12.5px;line-height:1.7">' +
              'Debian Minerva is a desktop session with a Rust core: the window model, virtual desktops, ' +
              'notifications, settings and system services all live in <code>minerva-sessiond</code>. ' +
              'The shell is rendered with web technology — the same approach as other modern desktops — ' +
              'so apps are lightweight and easy to build.</p>' +
              '<p style="font-size:12.5px;line-height:1.7;margin-top:10px">No Google services, no proprietary blobs. Just Debian and Rust.</p></div>' +
              '<div class="settings-card"><h3>System</h3><dl class="kv" id="' + id + '-sys"><dt>Loading…</dt><dd></dd></dl></div>';
            ctx.api.getSystem().then(function (sys) {
              var kv = body.querySelector('#' + id + '-sys'); if (!kv) return;
              kv.innerHTML = '<dt>OS</dt><dd>' + esc(sys.os) + '</dd><dt>Kernel</dt><dd>' + esc(sys.kernel) + '</dd>' +
                '<dt>Architecture</dt><dd>' + esc(sys.arch) + '</dd><dt>CPUs</dt><dd>' + esc(sys.cpus) + '</dd>' +
                '<dt>Memory</dt><dd>' + esc(sys.mem_total_mb) + ' MB</dd><dt>Session</dt><dd>' + esc(sys.session) + '</dd>' +
                '<dt>Toolkit</dt><dd>' + esc(sys.toolkit) + '</dd><dt>Rust</dt><dd>' + esc(sys.rustc) + '</dd>';
            });
          }
          main.innerHTML = h;
        }
        side.addEventListener('click', function (e) { var b = e.target.closest('[data-page]'); if (b) { current = b.dataset.page; render(); } });
        main.addEventListener('click', function (e) {
          var b = e.target.closest('button'); if (!b) return;
          if (b.dataset.theme) ctx.dispatch({ type: 'set_theme', value: b.dataset.theme });
          if (b.dataset.accent) ctx.dispatch({ type: 'set_accent', value: b.dataset.accent });
          if (b.dataset.wall) ctx.dispatch({ type: 'set_wallpaper', value: b.dataset.wall });
          if (b.dataset.toggle) ctx.dispatch({ type: 'toggle', key: b.dataset.toggle });
          if (b.dataset.ws !== undefined) ctx.dispatch({ type: 'switch_workspace', index: Number(b.dataset.ws) });
          if (b.dataset.addws !== undefined) ctx.dispatch({ type: 'add_workspace' });
          if (b.dataset.reset !== undefined) ctx.dispatch({ type: 'reset' });
        });
        main.addEventListener('input', function (e) {
          if (e.target.matches('[data-scale]')) ctx.dispatch({ type: 'set_scale', value: Number(e.target.value) / 100 });
        });
        ctx.onStateChange(render);
        render();
      }
    };
  };

  // ----------------------------------------------------------------- Editor
  V.editor = function (ctx) {
    var id = gid('edit');
    return {
      html: '<div class="app"><div class="app-toolbar"><span>' + I.svg('doc', 15) + '</span>' +
        '<input id="' + id + '-path" value="/Documents/notes.md" style="min-width:220px" spellcheck="false">' +
        '<button class="tb-btn" data-a="open">' + I.svg('folder', 15) + ' Open</button>' +
        '<button class="tb-btn" data-a="save">' + I.svg('check', 15) + ' Save</button><span class="spacer"></span>' +
        '<span id="' + id + '-stat" style="font-size:11px;color:var(--text-faint)"></span></div>' +
        '<textarea class="editor-area" id="' + id + '-area" spellcheck="false"></textarea>' +
        '<div class="app-status">Minerva Text Editor · files are stored in the Minerva VFS</div></div>',
      init: function (body, opts) {
        var area = body.querySelector('#' + id + '-area');
        var pathInput = body.querySelector('#' + id + '-path');
        var stat = body.querySelector('#' + id + '-stat');
        function load(p) {
          pathInput.value = p;
          ctx.api.fileOp('read', p).then(function (r) { area.value = r.content || ''; stat.textContent = 'loaded ' + p; });
        }
        function save() {
          ctx.api.fileOp('write', pathInput.value, area.value).then(function () {
            stat.textContent = 'saved ' + pathInput.value;
            ctx.toast('Saved ' + pathInput.value);
          });
        }
        body.querySelector('.app-toolbar').addEventListener('click', function (e) {
          var b = e.target.closest('[data-a]'); if (!b) return;
          if (b.dataset.a === 'open') load(pathInput.value);
          if (b.dataset.a === 'save') save();
        });
        area.addEventListener('keydown', function (e) {
          if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') { e.preventDefault(); save(); }
        });
        load((opts && opts.path) || '/Documents/notes.md');
        ctx.registerEditorLoader(load);
      }
    };
  };

  // ------------------------------------------------------------ Task Manager
  V.monitor = function (ctx) {
    var id = gid('mon');
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Task Manager</strong><span class="spacer"></span>' +
        '<button class="tb-btn" data-a="refresh">' + I.svg('refresh', 15) + ' Refresh</button></div>' +
        '<div class="app-content"><div class="stat-cards" id="' + id + '-stats"></div>' +
        '<h3 style="font-size:12px;text-transform:uppercase;letter-spacing:.08em;color:var(--text-muted)">Running applications</h3>' +
        '<table class="monitor-table"><thead><tr><th>ID</th><th>Application</th><th>Desktop</th><th>State</th><th></th></tr></thead>' +
        '<tbody id="' + id + '-rows"></tbody></table></div>' +
        '<div class="app-status" id="' + id + '-status"></div></div>',
      init: function (body) {
        var rows = body.querySelector('#' + id + '-rows');
        var stats = body.querySelector('#' + id + '-stats');
        var status = body.querySelector('#' + id + '-status');
        function render() {
          var ws = ctx.state.windows;
          rows.innerHTML = ws.map(function (w) {
            return '<tr><td>#' + w.id + '</td><td>' + I.svg(ctx.appIcon(w.app), 14) + ' ' + esc(ctx.appName(w.app)) + '</td>' +
              '<td>' + esc((ctx.state.workspaces[w.workspace] || {}).name || ('Desktop ' + ((w.workspace || 0) + 1))) + '</td>' +
              '<td>' + esc(w.state) + '</td>' +
              '<td><button class="danger-btn" data-kill="' + w.id + '">End task</button></td></tr>';
          }).join('') || '<tr><td colspan="5" style="color:var(--text-muted);text-align:center;padding:26px">No applications running</td></tr>';
          ctx.api.getSystem().then(function (sys) {
            var load = parseFloat(String(sys.loadavg || '').split(' ')[0]) || 0;
            var cpuPct = Math.min(100, Math.round(load / (sys.cpus || 4) * 100));
            stats.innerHTML = '<div class="stat-card"><small>CPU · load ' + load.toFixed(2) + '</small><strong>' + (sys.cpus || 4) + ' cores</strong><div class="bar"><span style="width:' + cpuPct + '%"></span></div></div>' +
              '<div class="stat-card"><small>Memory</small><strong>' + Math.round(sys.mem_used_mb || 0) + ' / ' + Math.round(sys.mem_total_mb || 0) + ' MB</strong><div class="bar"><span style="width:' + Math.round(sys.mem_percent || 0) + '%"></span></div></div>' +
              '<div class="stat-card"><small>Windows</small><strong>' + ws.length + '</strong></div>' +
              '<div class="stat-card"><small>Workspaces</small><strong>' + ctx.state.workspaces.length + '</strong></div>';
            status.textContent = sys.hostname + ' · ' + sys.kernel + ' · uptime ' + Math.floor((sys.uptime_secs || 0) / 60) + ' min · ' + (ctx.api.backend === 'rust' ? 'live /proc data' : 'preview data');
          });
        }
        body.querySelector('.app-toolbar').addEventListener('click', function (e) { if (e.target.closest('[data-a="refresh"]')) render(); });
        rows.addEventListener('click', function (e) { var b = e.target.closest('[data-kill]'); if (b) { ctx.dispatch({ type: 'close', id: Number(b.dataset.kill) }); setTimeout(render, 50); } });
        ctx.onStateChange(render);
        render();
        var t = setInterval(render, 4000);
        ctx.onCleanup(function () { clearInterval(t); });
      }
    };
  };

  // ------------------------------------------------------------- Calculator
  V.calculator = function (ctx) {
    var id = gid('calc');
    var keys = [['C', 'op'], ['±', 'op'], ['%', 'op'], ['÷', 'op'],
                ['7', ''], ['8', ''], ['9', ''], ['×', 'op'],
                ['4', ''], ['5', ''], ['6', ''], ['−', 'op'],
                ['1', ''], ['2', ''], ['3', ''], ['+', 'op'],
                ['0', ''], ['.', ''], ['⌫', 'op'], ['=', 'eq']];
    return {
      html: '<div class="calc"><div class="calc-display"><small id="' + id + '-expr"></small><span id="' + id + '-val">0</span></div>' +
        '<div class="calc-grid">' + keys.map(function (k) { return '<button class="calc-key ' + k[1] + '" data-k="' + k[0] + '">' + k[0] + '</button>'; }).join('') + '</div></div>',
      init: function (body) {
        var val = body.querySelector('#' + id + '-val'), expr = body.querySelector('#' + id + '-expr');
        var cur = '0', prev = '', op = '';
        function calc() {
          var a = parseFloat(prev), b = parseFloat(cur);
          if (op === '+') return a + b; if (op === '−') return a - b;
          if (op === '×') return a * b; if (op === '÷') return b === 0 ? NaN : a / b;
          return b;
        }
        function show() { val.textContent = cur; expr.textContent = prev + (op ? ' ' + op : ''); }
        body.querySelector('.calc-grid').addEventListener('click', function (e) {
          var k = e.target.closest('[data-k]'); if (!k) return;
          var c = k.dataset.k;
          if (/[0-9]/.test(c)) cur = (cur === '0' || cur === 'Error') ? c : cur + c;
          else if (c === '.') { if (cur === 'Error') cur = '0.'; else if (cur.indexOf('.') === -1) cur += '.'; }
          else if (c === 'C') { cur = '0'; prev = ''; op = ''; }
          else if (c === '⌫') cur = cur.length > 1 ? cur.slice(0, -1) : '0';
          else if (c === '±') cur = String(-parseFloat(cur));
          else if (c === '%') cur = String(parseFloat(cur) / 100);
          else if (c === '=') { if (op) { var r = calc(); cur = isFinite(r) ? String(+r.toFixed(8)) : 'Error'; prev = ''; op = ''; } }
          else { if (cur === 'Error') cur = '0'; prev = cur; cur = '0'; op = c; }
          show();
        });
      }
    };
  };

  // ------------------------------------------------------------------ Clock
  V.clock = function (ctx) {
    var id = gid('clock');
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Clock</strong><span class="spacer"></span><span class="muted" style="font-size:11px">Local time</span></div>' +
        '<div class="app-content" style="text-align:center"><div class="clock-face" id="' + id + '-face">' +
        '<span class="clock-hand" id="' + id + '-h"></span><span class="clock-hand" id="' + id + '-m"></span>' +
        '<span class="clock-hand" id="' + id + '-s"></span><span class="clock-center"></span></div>' +
        '<div class="clock-digital" id="' + id + '-dig"></div><p class="muted" id="' + id + '-date"></p></div></div>',
      init: function (body) {
        function tick() {
          var d = new Date();
          var h = d.getHours() % 12, m = d.getMinutes(), s = d.getSeconds();
          var el = function (x) { return body.querySelector('#' + id + x); };
          el('-h').style.cssText = 'height:56px;width:5px;transform:translateX(-50%) rotate(' + (h * 30 + m * .5) + 'deg)';
          el('-m').style.cssText = 'height:78px;width:3px;transform:translateX(-50%) rotate(' + (m * 6 + s * .1) + 'deg)';
          el('-s').style.cssText = 'height:88px;width:2px;background:var(--text-muted);transform:translateX(-50%) rotate(' + (s * 6) + 'deg)';
          el('-dig').textContent = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
          el('-date').textContent = d.toLocaleDateString([], { weekday: 'long', year: 'numeric', month: 'long', day: 'numeric' });
        }
        tick(); var t = setInterval(tick, 1000);
        ctx.onCleanup(function () { clearInterval(t); });
      }
    };
  };

  // --------------------------------------------------------------- Calendar
  V.calendar = function (ctx) {
    var id = gid('cal');
    return {
      html: '<div class="app"><div class="app-toolbar"><button class="tb-btn" data-a="prev">' + I.svg('chevronLeft', 15) + '</button>' +
        '<strong id="' + id + '-title" style="font-size:13px"></strong><button class="tb-btn" data-a="next">' + I.svg('chevronRight', 15) + '</button>' +
        '<span class="spacer"></span><button class="tb-btn" data-a="today">Today</button></div>' +
        '<div class="app-content"><div class="cal-grid" id="' + id + '-grid"></div></div></div>',
      init: function (body) {
        var d = new Date(); var view = new Date(d.getFullYear(), d.getMonth(), 1);
        function render() {
          var y = view.getFullYear(), m = view.getMonth();
          body.querySelector('#' + id + '-title').textContent = view.toLocaleDateString([], { month: 'long', year: 'numeric' });
          var first = new Date(y, m, 1).getDay();
          var days = new Date(y, m + 1, 0).getDate();
          var html = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map(function (x) { return '<span class="hdr">' + x + '</span>'; }).join('');
          for (var i = 0; i < first; i++) html += '<span></span>';
          for (var day = 1; day <= days; day++) {
            var today = day === d.getDate() && m === d.getMonth() && y === d.getFullYear();
            html += '<span class="' + (today ? 'today' : '') + '">' + day + '</span>';
          }
          body.querySelector('#' + id + '-grid').innerHTML = html;
        }
        body.querySelector('.app-toolbar').addEventListener('click', function (e) {
          var b = e.target.closest('[data-a]'); if (!b) return;
          if (b.dataset.a === 'prev') view = new Date(view.getFullYear(), view.getMonth() - 1, 1);
          if (b.dataset.a === 'next') view = new Date(view.getFullYear(), view.getMonth() + 1, 1);
          if (b.dataset.a === 'today') view = new Date(d.getFullYear(), d.getMonth(), 1);
          render();
        });
        render();
      }
    };
  };

  // ------------------------------------------------------------------ Notes
  V.notes = function (ctx) {
    var id = gid('notes');
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Notes</strong><span class="spacer"></span>' +
        '<button class="tb-btn" data-a="save">' + I.svg('check', 15) + ' Save</button></div>' +
        '<textarea class="editor-area" id="' + id + '-area" placeholder="Write something…"></textarea>' +
        '<div class="app-status" id="' + id + '-stat">Stored locally in your Minerva session</div></div>',
      init: function (body) {
        var area = body.querySelector('#' + id + '-area');
        try { area.value = localStorage.getItem('minerva.notes') || '# Notes\n\n'; } catch (e) { area.value = '# Notes\n\n'; }
        var deb = 0;
        function saveNow() { clearTimeout(deb); try { localStorage.setItem('minerva.notes', area.value); } catch (e) {} body.querySelector('#' + id + '-stat').textContent = 'Saved · ' + new Date().toLocaleTimeString(); }
        area.addEventListener('input', function () { clearTimeout(deb); deb = setTimeout(saveNow, 300); });
        body.querySelector('[data-a="save"]').addEventListener('click', function () { saveNow(); ctx.toast('Note saved'); });
      }
    };
  };

  // --------------------------------------------------------------- Gallery
  V.gallery = function (ctx) {
    var WALLS = ['swirl', 'aurora', 'dusk', 'light', 'mesh'];
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Wallpapers</strong><span class="spacer"></span><span class="muted" style="font-size:11px">SVG, scalable, no binary blobs</span></div>' +
        '<div class="app-content"><div class="wp-gallery">' + WALLS.map(function (w) {
          return '<button class="wp-card ' + (ctx.state.wallpaper === w ? 'active' : '') + '" data-w="' + w + '" style="background-image:url(assets/wallpapers/' + w + '.svg)"><span>' + w + '</span></button>';
        }).join('') + '</div></div></div>',
      init: function (body) {
        body.addEventListener('click', function (e) {
          var b = e.target.closest('[data-w]'); if (!b) return;
          ctx.dispatch({ type: 'set_wallpaper', value: b.dataset.w });
          body.querySelectorAll('.wp-card').forEach(function (x) { x.classList.toggle('active', x.dataset.w === b.dataset.w); });
          ctx.toast('Wallpaper: ' + b.dataset.w);
        });
      }
    };
  };

  // -------------------------------------------------------------- Software
  V.software = function (ctx) {
    var real = ctx.api && ctx.api.backend === 'rust';
    if (real) {
      // Real package management: searches the actual Debian archive with
      // apt-cache and installs through a real terminal running sudo apt.
      return {
        html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Software</strong>' +
          '<input class="sw-search" data-q placeholder="Search Debian packages (apt)" spellcheck="false" autocomplete="off">' +
          '<button class="tb-btn" data-search>Search</button></div>' +
          '<div class="app-content"><div class="sw-results" data-results>' +
          '<div class="app-status">Search the real Debian archive — installs run in a terminal with sudo.</div></div></div>' +
          '<div class="app-status">apt · minerva-sessiond</div></div>',
        init: function (body) {
          var results = body.querySelector('[data-results]');
          var input = body.querySelector('[data-q]');
          function search() {
            var q = input.value.trim();
            if (!q) return;
            results.innerHTML = '<div class="app-status">Searching apt…</div>';
            ctx.api.sys('apt_search', { q: q }).then(function (r) {
              var list = (r && r.results) || [];
              results.innerHTML = list.length ? list.map(function (pk) {
                return '<div class="sw-pkg"><div class="sw-pkg-info"><strong>' + esc(pk.name) + '</strong><p>' + esc(pk.desc) + '</p></div>' +
                  '<button data-install="' + esc(pk.name) + '">Install</button></div>';
              }).join('') : '<div class="app-status">No packages match “' + esc(q) + '”</div>';
            });
          }
          body.querySelector('[data-search]').addEventListener('click', search);
          input.addEventListener('keydown', function (e) { if (e.key === 'Enter') search(); });
          setTimeout(function () { input.focus(); }, 60);
          results.addEventListener('click', function (e) {
            var b = e.target.closest('[data-install]');
            if (!b) return;
            ctx.api.sys('apt_install', { pkg: b.dataset.install });
            ctx.toast('A terminal window will open — enter your sudo password to install ' + b.dataset.install);
          });
        }
      };
    }
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Software</strong><span class="spacer"></span>' +
        '<span class="muted" style="font-size:11px">Minerva app catalogue</span></div>' +
        '<div class="app-content"><div class="tile-grid">' + ctx.apps.map(function (a) {
          return '<div class="tile-card"><div class="tc-icon">' + I.svg(a.icon, 22) + '</div><strong>' + esc(a.name) + '</strong>' +
            '<p>' + esc(a.summary) + '</p><button data-open="' + a.id + '">Open</button></div>';
        }).join('') + '</div></div><div class="app-status">' + ctx.apps.length + ' applications installed · Debian Minerva</div></div>',
      init: function (body) {
        body.addEventListener('click', function (e) { var b = e.target.closest('[data-open]'); if (b) ctx.openApp(b.dataset.open); });
      }
    };
  };

  // ---------------------------------------------------------------- Browser
  V.browser = function (ctx) {
    var id = gid('br');
    return {
      html: '<div class="app"><div class="app-toolbar"><button class="tb-btn" data-a="back">' + I.svg('arrowLeft', 15) + '</button>' +
        '<button class="tb-btn" data-a="home">' + I.svg('home', 15) + '</button>' +
        '<input id="' + id + '-url" value="minerva://start" style="flex:1" spellcheck="false"><button class="tb-btn" data-a="go">Go</button></div>' +
        '<div class="browser-page" id="' + id + '-page"></div>' +
        '<div class="app-status">Minerva Web · local content only (no network requests)</div></div>',
      init: function (body) {
        var page = body.querySelector('#' + id + '-page');
        var url = body.querySelector('#' + id + '-url');
        function render() {
          page.innerHTML = '<div class="browser-hero"><h1>Debian Minerva</h1>' +
            '<p>A desktop session built with Rust. This start page is bundled with the shell — no telemetry, no external requests.</p></div>' +
            '<div class="browser-cards">' +
            [['Getting started', 'Press Super to open the launcher, or click the grid icon in the dock.'],
             ['Virtual desktops', 'Super + W opens the overview. Add workspaces with the + button.'],
             ['Terminal', 'Try `neofetch`, `apps`, `open settings`, `theme light`.'],
             ['Customise', 'Change wallpaper, accent and theme in Settings → Appearance.'],
             ['Files', 'Your home directory is served by the Rust session daemon.'],
             ['About', 'Debian Minerva 0.7.1 — MIT licensed, no Google components.']].map(function (c) {
              return '<div class="browser-card"><h3>' + c[0] + '</h3><p>' + c[1] + '</p></div>';
            }).join('') + '</div>';
          url.value = 'minerva://start';
        }
        body.querySelector('.app-toolbar').addEventListener('click', function (e) {
          var b = e.target.closest('[data-a]'); if (!b) return;
          if (b.dataset.a === 'home' || b.dataset.a === 'go' || b.dataset.a === 'back') { render(); ctx.toast('minerva://start'); }
        });
        url.addEventListener('keydown', function (e) { if (e.key === 'Enter') render(); });
        render();
      }
    };
  };

  V.music = function (ctx) {
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Music</strong></div>' +
        '<div class="app-content music-player"><div class="music-disc"><div></div></div>' +
        '<h2 style="margin:0">Minerva Ambient</h2><p class="muted" style="font-size:12px">Debian Minerva · Demo track</p>' +
        '<div class="music-controls" style="margin-top:20px"><button class="icon-btn" data-a="prev">' + I.svg('skip', 20) + '</button>' +
        '<button class="icon-btn" data-a="play" style="width:52px;height:52px;background:var(--accent);color:#fff">' + I.svg('play', 24) + '</button>' +
        '<button class="icon-btn" data-a="next">' + I.svg('skip', 20) + '</button></div>' +
        '<p class="muted" style="font-size:11px;margin-top:18px">Playback is a UI demo — no audio backend yet.</p></div></div>',
      init: function (body) {
        var playing = false;
        var disc = body.querySelector('.music-disc');
        disc.style.animation = 'bootspin 6s linear infinite';
        disc.style.animationPlayState = 'paused';
        body.querySelector('[data-a="play"]').addEventListener('click', function (e) {
          playing = !playing;
          e.currentTarget.innerHTML = I.svg(playing ? 'pause' : 'play', 24);
          disc.style.animationPlayState = playing ? 'running' : 'paused';
        });
      }
    };
  };

  V.camera = function (ctx) {
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Camera</strong><span class="spacer"></span><span class="muted" style="font-size:11px">No device attached</span></div>' +
        '<div class="camera-stage">' + I.svg('camera', 42) + '<div style="margin-top:12px">Camera preview unavailable in this session</div></div>' +
        '<div class="app-status">Minerva Camera · hardware access requires native daemon permissions</div></div>',
      init: function () {}
    };
  };

  // ----------------------------------------------------------- Help / About
  V.help = function (ctx) {
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Help & About</strong></div>' +
        '<div class="app-content"><div class="about-hero"><img src="assets/logo.svg" alt=""><div><h1>Debian Minerva</h1>' +
        '<p>Version 0.7.1 · Rust desktop session</p></div></div>' +
        '<div class="settings-card"><h3>What is this?</h3><p style="font-size:12.5px;line-height:1.7">' +
        'Debian Minerva is a prototype desktop environment for Debian GNU/Linux. ' +
        'Its system layer is written in Rust with no external crates; the UI shell is a lightweight web renderer.</p></div>' +
        '<div class="settings-card"><h3>Quick shortcuts</h3><ul class="shortcut-list">' +
        [['Super', 'App launcher'], ['Super + W', 'Activities overview'], ['Alt + Tab', 'Switch windows'],
         ['Ctrl + Alt + T', 'Terminal'], ['Super + D', 'Show desktop'], ['Esc', 'Close panels']].map(function (x) {
          return '<li><kbd>' + x[0] + '</kbd><span>' + x[1] + '</span></li>';
        }).join('') + '</ul></div>' +
        '<div class="settings-card"><h3>Credits</h3><p style="font-size:12.5px;line-height:1.7">Built with Rust + vanilla JavaScript. ' +
        'UI layout inspired by the Aluminium OS leaks (May 2026), with all Google branding removed.</p></div></div></div>',
      init: function () {}
    };
  };

  // --------------------------------------------------------------- Displays
  V.display = function (ctx) {
    return {
      html: '<div class="app"><div class="app-toolbar"><strong style="font-size:12px">Displays</strong></div>' +
        '<div class="app-content"><div class="settings-card"><h3>Screen</h3>' +
        '<div class="setting-row"><div class="sr-label"><strong>Resolution</strong><small data-rescur>Reading xrandr…</small></div><span class="muted">Live</span></div>' +
        '<div class="setting-row"><div class="sr-label"><strong>Session</strong><small data-ressess>—</small></div><span class="muted">X11</span></div></div>' +
        '<div class="settings-card"><h3>Virtual desktops</h3><p style="font-size:12.5px">You have ' + ctx.state.workspaces.length + ' workspaces. ' +
        'Open the Activities overview with Super + W to manage them.</p>' +
        '<button class="chip-btn" data-overview style="margin-top:12px">Open overview</button></div></div></div>',
      init: function (body) {
        body.addEventListener('click', function (e) { if (e.target.closest('[data-overview]')) ctx.dispatch({ type: 'overview', value: true }); });
        var cur = body.querySelector('[data-rescur]');
        var sess = body.querySelector('[data-ressess]');
        if (sess) sess.textContent = ctx.api.sessionMode === 'x11' ? 'Openbox · real windows' : 'Single window';
        ctx.api.sys('res_list').then(function (r) {
          if (!cur) return;
          if (r && r.available && (r.modes || []).length) {
            var c = r.modes.filter(function (m) { return m.current; })[0] || r.modes[0];
            cur.textContent = c.mode + ' · ' + r.modes.length + ' mode(s) reported by xrandr';
          } else {
            cur.textContent = 'xrandr not available — install x11-xserver-utils';
          }
        });
      }
    };
  };

  global.AppViews = {
    create: function (appId, ctx) {
      var fn = V[appId] || V.help;
      return fn(ctx);
    },
    has: function (id) { return !!V[id]; }
  };
})(window);
