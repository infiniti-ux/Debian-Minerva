/* Debian Minerva — panel windows & real popovers.
 *
 * In a real X11 session every popover (Wi-Fi, audio, battery, calendar,
 * power) and the Settings window is its own X window loaded with a #role
 * hash. In single-window/preview mode the same renderers fill in-page
 * panels. All data here is REAL: nmcli networks, pactl volume, xrandr
 * resolutions — served by the Rust daemon's /api/v1/exec sys calls.
 */
(function (global) {
  'use strict';

  var API = global.MinervaAPI;
  var $ = function (s, r) { return (r || document).querySelector(s); };
  var esc = function (s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]; }); };
  var timers = [];
  var WALLS = ['swirl', 'aurora', 'dusk', 'light', 'mesh'];
  var ACCENTS = ['#a80030', '#3b6fe0', '#7c4dff', '#00897b', '#e8710a', '#c2185b'];

  function shell() { return global.MinervaShell || null; }
  function state() { var s = shell(); return s ? s.state() : null; }
  function dispatch(a) { var s = shell(); if (s) s.dispatch(a); }
  function toast(m) { var s = shell(); if (s) s.toast(m); }
  function hydrate(el) { if (global.Icon && Icon.hydrate) Icon.hydrate(el); }

  function isWindowMode() { return !!document.documentElement.dataset.mode; }
  function selfClose() { if (isWindowMode()) { try { window.close(); } catch (e) {} } }

  // ------------------------------------------------------------- wi-fi
  function wifiBody() {
    return '<div class="pop-head"><span class="ph-icon" data-icon="wifi"></span><strong>Wi-Fi</strong>' +
      '<span style="flex:1"></span><button class="switch" data-wifi-switch><span></span></button></div>' +
      '<div class="pop-scroll" data-wifi-list><div class="pop-empty">Scanning…</div></div>' +
      '<div class="pop-connect hidden" data-wifi-connect>' +
      '<div class="pc-title">Join <b data-wifi-ssid></b></div>' +
      '<input type="password" data-wifi-pass placeholder="Password" autocomplete="off">' +
      '<div class="pc-actions"><button class="text-btn" data-wifi-cancel>Cancel</button>' +
      '<button class="primary-btn" data-wifi-join>Connect</button></div></div>';
  }

  function bars(sig) {
    var n = sig > 70 ? 4 : sig > 50 ? 3 : sig > 30 ? 2 : 1;
    var h = '';
    for (var i = 1; i <= 4; i++) h += '<i class="bar' + (i <= n ? ' on' : '') + '" style="height:' + (3 + i * 2.5) + 'px"></i>';
    return '<span class="sig">' + h + '</span>';
  }

  function wifiRefresh(el) {
    var list = $('[data-wifi-list]', el);
    if (!list) return;
    API.sys('wifi_state').then(function (st) {
      var sw = $('[data-wifi-switch]', el);
      if (sw) sw.classList.toggle('on', !!(st && st.on));
      if (st && st.available === false) {
        list.innerHTML = '<div class="pop-empty">NetworkManager is not installed.<br><code>sudo apt install network-manager</code></div>';
        return;
      }
      if (st && !st.on) { list.innerHTML = '<div class="pop-empty">Wi-Fi is off</div>'; return; }
      API.sys('wifi_list').then(function (r) {
        var nets = (r && r.networks) || [];
        var keepScroll = list.scrollTop;
        list.innerHTML = nets.length ? nets.map(function (n) {
          return '<button class="pop-item' + (n.active ? ' active' : '') + '" data-ssid="' + esc(n.ssid) + '" data-secure="' + (n.secure ? 1 : 0) + '">' +
            bars(n.signal) + '<span class="pi-name">' + esc(n.ssid) + '</span>' +
            (n.secure ? '<span class="pi-lock" data-icon="lock"></span>' : '') +
            (n.active ? '<span class="pi-check" data-icon="check"></span>' : '') + '</button>';
        }).join('') : '<div class="pop-empty">No networks found</div>';
        list.scrollTop = keepScroll;
        hydrate(list);
      });
    });
  }

  function wireWifi(el) {
    var connecting = null;
    el.addEventListener('click', function (e) {
      if (e.target.closest('[data-wifi-switch]')) {
        var sw = $('[data-wifi-switch]', el);
        var on = !sw.classList.contains('on');
        API.sys('wifi_toggle', { on: on });
        dispatch({ type: 'toggle', key: 'wifi' });
        sw.classList.toggle('on', on);
        setTimeout(function () { wifiRefresh(el); }, 1200);
        return;
      }
      var item = e.target.closest('[data-ssid]');
      if (item) {
        var ssid = item.dataset.ssid;
        if (item.dataset.secure === '1') {
          connecting = ssid;
          $('[data-wifi-ssid]', el).textContent = ssid;
          $('[data-wifi-connect]', el).classList.remove('hidden');
          var p = $('[data-wifi-pass]', el); p.value = '';
          var jb0 = $('[data-wifi-join]', el); if (jb0) jb0.disabled = false;
          setTimeout(function () { p.focus(); }, 50);
        } else {
          API.sys('wifi_connect', { ssid: ssid, password: '' });
          toast('Connecting to ' + ssid + '…');
        }
        return;
      }
      if (e.target.closest('[data-wifi-cancel]')) { $('[data-wifi-connect]', el).classList.add('hidden'); connecting = null; }
      var join = e.target.closest('[data-wifi-join]');
      if (join) {
        if (!connecting) return;
        join.disabled = true;
        var pass = $('[data-wifi-pass]', el).value;
        API.sys('wifi_connect', { ssid: connecting, password: pass });
        $('[data-wifi-connect]', el).classList.add('hidden');
        toast('Connecting to ' + connecting + '…');
        connecting = null;
        setTimeout(function () { join.disabled = false; wifiRefresh(el); }, 3000);
      }
    });
    el.addEventListener('keydown', function (e) {
      if (e.key === 'Enter' && e.target.matches('[data-wifi-pass]')) {
        var jb = $('[data-wifi-join]', el); if (jb && !jb.disabled) jb.click();
      }
      if (e.key === 'Escape') {
        var pc = $('[data-wifi-connect]', el);
        if (pc && !pc.classList.contains('hidden')) { pc.classList.add('hidden'); connecting = null; }
      }
    });
    timers.push(setInterval(function () {
      if (el.offsetParent !== null || isWindowMode()) wifiRefresh(el);
    }, 6000));
    wifiRefresh(el);
  }

  // ------------------------------------------------------------- audio
  function audioBody() {
    return '<div class="pop-head"><span class="ph-icon" data-icon="volume"></span><strong>Audio</strong>' +
      '<span style="flex:1"></span><button class="icon-btn" data-mute title="Mute"><span data-icon="volume"></span></button></div>' +
      '<div class="pop-pad"><div class="pop-volrow"><input type="range" min="0" max="100" data-vol>' +
      '<b data-volnum>—</b></div><div class="pop-hint" data-icon="info"> Real system volume (pactl / amixer)</div></div>';
  }

  function paintMute(el, muted) {
    var mb = $('[data-mute]', el); if (!mb) return;
    mb.classList.toggle('muted', !!muted);
    mb.title = muted ? 'Unmute' : 'Mute';
    var ic = mb.querySelector('[data-icon]');
    if (ic && global.Icon && Icon.svg) { ic.dataset.icon = muted ? 'volumeMute' : 'volume'; ic.innerHTML = Icon.svg(ic.dataset.icon, 18); }
  }
  function wireAudio(el) {
    var slider = $('[data-vol]', el);
    function volGet() {
      API.sys('volume_get').then(function (r) {
        if (r && r.ok) {
          slider.value = Math.round(r.volume);
          var vn = $('[data-volnum]', el); if (vn) vn.textContent = Math.round(r.volume) + '%';
          paintMute(el, r.muted);
        } else {
          var vn2 = $('[data-volnum]', el); if (vn2) vn2.textContent = 'n/a';
        }
      });
    }
    volGet();
    slider.addEventListener('input', function () {
      var n = Number(slider.value);
      $('[data-volnum]', el).textContent = n + '%';
      API.sys('volume_set', { n: n });
      dispatch({ type: 'set_volume', value: n / 100 });
    });
    el.addEventListener('click', function (e) {
      if (e.target.closest('[data-mute]')) {
        API.sys('mute_toggle');
        $('[data-mute]', el).classList.toggle('muted');
        setTimeout(volGet, 600);
      }
    });
  }

  // ----------------------------------------------------------- battery
  function batteryBody() {
    return '<div class="pop-head"><span class="ph-icon" data-icon="battery"></span><strong>Power & display</strong></div>' +
      '<div class="pop-pad">' +
      '<div class="pop-bigbatt"><span data-batt-icon></span><b data-batt-pct>—</b><small data-batt-state></small></div>' +
      '<div class="pop-sliderrow"><span data-icon="sun"></span><input type="range" min="20" max="100" data-bright><b data-brightnum></b></div>' +
      '<button class="pop-item" data-night><span class="ph-icon" data-icon="moon"></span><span class="pi-name">Night light</span><span class="switch" data-night-sw><span></span></span></button>' +
      '</div>';
  }

  function batteryRefresh(el) {
    if (!el || !el.isConnected || !$('[data-batt-pct]', el)) return;
    API.getSystem().then(function (sys) {
      if (!el.isConnected) return;
      var pct = Math.round((sys.battery_present ? sys.battery_level : 1) * 100);
      var pe = $('[data-batt-pct]', el); if (pe) pe.textContent = sys.battery_present ? pct + '%' : 'AC';
      var se = $('[data-batt-state]', el); if (se) se.textContent = !sys.battery_present ? 'No battery — mains power'
        : (sys.battery_charging ? 'Charging' : 'On battery power');
      var icon = $('[data-batt-icon]', el);
      if (icon) {
        icon.innerHTML = '<span class="batt-mini' + (sys.battery_charging ? ' charging' : '') + '"><i style="width:' + pct + '%"></i></span>';
      }
    });
    var s = state();
    if (s) {
      var br = $('[data-bright]', el); if (br) br.value = Math.round(s.brightness * 100);
      var bn = $('[data-brightnum]', el); if (bn) bn.textContent = Math.round(s.brightness * 100) + '%';
      var ns = $('[data-night-sw]', el); if (ns) ns.classList.toggle('on', !!s.toggles.night_light);
    }
  }

  function wireBattery(el) {
    $('[data-bright]', el).addEventListener('input', function (e) {
      var n = Number(e.target.value);
      $('[data-brightnum]', el).textContent = n + '%';
      dispatch({ type: 'set_brightness', value: n / 100 });
      API.sys('brightness_set', { n: n });
    });
    el.addEventListener('click', function (e) {
      if (e.target.closest('[data-night]')) {
        var s = state();
        var on = !(s && s.toggles.night_light);
        dispatch({ type: 'toggle', key: 'night_light' });
        API.sys('night_set', { on: on });
        $('[data-night-sw]', el).classList.toggle('on', on);
      }
    });
    batteryRefresh(el);
    timers.push(setInterval(function () { if (el.offsetParent !== null || isWindowMode()) batteryRefresh(el); }, 15000));
  }

  // ------------------------------------------------------------- power
  function powerBody() {
    return '<div class="pop-head"><span class="ph-icon" data-icon="power"></span><strong>Power</strong></div>' +
      '<div class="pop-pad">' +
      '<button class="pop-item" data-p="lock"><span class="ph-icon" data-icon="lock"></span><span class="pi-name">Lock screen</span></button>' +
      '<button class="pop-item" data-p="logout"><span class="ph-icon" data-icon="logout"></span><span class="pi-name">Log out</span></button>' +
      '<button class="pop-item" data-p="suspend"><span class="ph-icon" data-icon="moon"></span><span class="pi-name">Suspend</span></button>' +
      '<button class="pop-item" data-p="reboot"><span class="ph-icon" data-icon="refresh"></span><span class="pi-name">Restart</span></button>' +
      '<button class="pop-item danger" data-p="shutdown"><span class="ph-icon" data-icon="power"></span><span class="pi-name">Shut down</span></button>' +
      '</div>';
  }

  function wirePower(el) {
    el.addEventListener('click', function (e) {
      var b = e.target.closest('[data-p]');
      if (!b) return;
      var s = shell();
      if (s) s.power(b.dataset.p);
      setTimeout(selfClose, 400);
    });
  }

  // ---------------------------------------------------------- settings
  function settingsBody() {
    return '<div class="sw-head"><img src="assets/logo.svg" alt="" width="20"><strong>Settings</strong>' +
      '<span style="flex:1"></span><button class="icon-btn" data-sw-close><span data-icon="close"></span></button></div>' +
      '<div class="sw-body"><nav class="sw-nav">' +
      [['appearance', 'Appearance', 'palette'], ['display', 'Display', 'monitor'], ['sound', 'Sound', 'volume'],
       ['network', 'Network', 'wifi'], ['about', 'About', 'info']].map(function (p) {
        return '<button class="sw-nav-item" data-page="' + p[0] + '"><span data-icon="' + p[2] + '"></span>' + p[1] + '</button>';
      }).join('') +
      '</nav><div class="sw-main" data-sw-main></div></div>';
  }

  function renderSettingsPage(el, page) {
    var main = $('[data-sw-main]', el);
    var s = state() || {};
    var nav = $('.sw-nav', el);
    if (nav) nav.querySelectorAll('.sw-nav-item').forEach(function (b) { b.classList.toggle('active', b.dataset.page === page); });
    if (page === 'appearance') {
      main.innerHTML =
        '<div class="settings-card"><h3>Colour theme</h3><div class="theme-list">' +
        [['dark', 'Dark'], ['light', 'Light']].map(function (t) {
          return '<button class="theme-chip ' + ((s.theme || 'dark') === t[0] ? 'active' : '') + '" data-theme="' + t[0] + '">' + t[1] + '</button>';
        }).join('') + '</div></div>' +
        '<div class="settings-card"><h3>Accent colour</h3><div class="accent-list">' +
        ACCENTS.map(function (a) { return '<button class="accent-chip ' + (s.accent === a ? 'active' : '') + '" style="background:' + a + '" data-accent="' + a + '"></button>'; }).join('') +
        '</div></div>' +
        '<div class="settings-card"><h3>Wallpaper</h3><div class="wallpaper-list">' +
        WALLS.map(function (w) { return '<button class="wp-chip ' + (s.wallpaper === w ? 'active' : '') + '" data-wall="' + w + '" style="background-image:url(assets/wallpapers/' + w + '.svg)" title="' + w + '"></button>'; }).join('') +
        '</div></div>' +
        '<div class="settings-card"><h3>Interface scale</h3><div class="setting-row"><div class="sr-label"><strong>Scale</strong><small data-scale-num>' + Math.round((s.scale || 1) * 100) + '%</small></div>' +
        '<input type="range" min="75" max="150" step="5" value="' + Math.round((s.scale || 1) * 100) + '" data-scale></div></div>';
    } else if (page === 'display') {
      main.innerHTML =
        '<div class="settings-card"><h3>Resolution</h3><div class="pop-hint" data-icon="info"> Applied with xrandr — real display modes</div>' +
        '<div class="res-list" data-res-list><div class="pop-empty">Reading display modes…</div></div></div>' +
        '<div class="settings-card"><h3>Brightness</h3><div class="pop-sliderrow"><span data-icon="sun"></span>' +
        '<input type="range" min="20" max="100" value="' + Math.round((s.brightness || 0.8) * 100) + '" data-bright><b data-brightnum>' + Math.round((s.brightness || 0.8) * 100) + '%</b></div></div>';
      API.sys('res_list').then(function (r) {
        var box = $('[data-res-list]', el);
        if (!box) return;
        if (!r || !r.available) { box.innerHTML = '<div class="pop-empty">xrandr not available — install x11-xserver-utils</div>'; return; }
        var modes = r.modes || [];
        box.innerHTML = modes.length ? modes.map(function (m) {
          return '<button class="pop-item res-item' + (m.current ? ' active' : '') + '" data-res="' + esc(m.mode) + '">' +
            '<span class="ph-icon" data-icon="monitor"></span><span class="pi-name">' + esc(m.mode) + '</span>' +
            (m.current ? '<span class="pi-check" data-icon="check"></span>' : '') + '</button>';
        }).join('') : '<div class="pop-empty">No modes reported</div>';
        hydrate(box);
      });
    } else if (page === 'sound') {
      main.innerHTML = '<div class="settings-card">' + audioBody() + '</div>';
      wireAudio($('.settings-card', main));
      hydrate(main);
      return;
    } else if (page === 'network') {
      main.innerHTML = '<div class="settings-card">' + wifiBody() + '</div>';
      wireWifi($('.settings-card', main));
      hydrate(main);
      return;
    } else {
      main.innerHTML = '<div class="settings-card"><h3>About this system</h3><dl class="kv" data-about><dt>Loading…</dt><dd></dd></dl></div>';
      API.getSystem().then(function (sys) {
        var dl = $('[data-about]', el);
        if (!dl) return;
        var rows = [
          ['Operating system', sys.os || 'Debian GNU/Linux'],
          ['Hostname', sys.hostname || '—'],
          ['Kernel', sys.kernel || '—'],
          ['Architecture', sys.arch || '—'],
          ['Memory', Math.round(sys.mem_used_mb || 0) + ' / ' + Math.round(sys.mem_total_mb || 0) + ' MB'],
          ['Desktop', (sys.de || 'Debian Minerva') + ' ' + (sys.de_version || '')],
          ['Session core', sys.rustc || 'minerva-sessiond (rust)'],
          ['Session type', API.sessionMode === 'x11' ? 'X11 · Openbox · real windows' : 'Single window']
        ];
        dl.innerHTML = rows.map(function (r) { return '<dt>' + esc(r[0]) + '</dt><dd>' + esc(r[1]) + '</dd>'; }).join('');
      });
    }
    hydrate(main);
  }

  function wireSettings(el) {
    var current = 'appearance';
    var brightDeb = 0;
    el.addEventListener('click', function (e) {
      if (e.target.closest('[data-sw-close]')) {
        if (isWindowMode()) selfClose();
        else el.classList.add('hidden');
        return;
      }
      var nav = e.target.closest('[data-page]');
      if (nav) { current = nav.dataset.page; renderSettingsPage(el, current); return; }
      var s = state() || {};
      var t = e.target.closest('[data-theme]');
      if (t) { dispatch({ type: 'set_theme', value: t.dataset.theme }); renderSettingsPage(el, current); return; }
      var a = e.target.closest('[data-accent]');
      if (a) { dispatch({ type: 'set_accent', value: a.dataset.accent }); renderSettingsPage(el, current); return; }
      var w = e.target.closest('[data-wall]');
      if (w) { dispatch({ type: 'set_wallpaper', value: w.dataset.wall }); renderSettingsPage(el, current); return; }
      var r = e.target.closest('[data-res]');
      if (r) { API.sys('res_set', { mode: r.dataset.res }); toast('Switching to ' + r.dataset.res + '…'); setTimeout(function () { renderSettingsPage(el, current); }, 2500); return; }
    });
    el.addEventListener('input', function (e) {
      if (e.target.matches('[data-scale]')) {
        var v = Number(e.target.value);
        var n = $('[data-scale-num]', el); if (n) n.textContent = v + '%';
        dispatch({ type: 'set_scale', value: v / 100 });
      }
      if (e.target.matches('[data-bright]')) {
        var b = Number(e.target.value);
        var bn = $('[data-brightnum]', el); if (bn) bn.textContent = b + '%';
        dispatch({ type: 'set_brightness', value: b / 100 });
        clearTimeout(brightDeb);
        brightDeb = setTimeout(function () { API.sys('brightness_set', { n: b }); }, 250);
      }
    });
    renderSettingsPage(el, current);
  }

  // ---------------------------------------------------------------- init
  var FILLS = {
    wifi: { body: wifiBody, wire: wireWifi },
    audio: { body: audioBody, wire: wireAudio },
    battery: { body: batteryBody, wire: wireBattery },
    power: { body: powerBody, wire: wirePower }
  };

  function fill(name, el) {
    if (!el || el.dataset.filled === name) return;
    var f = FILLS[name];
    if (!f) return;
    el.innerHTML = f.body();
    el.dataset.filled = name;
    hydrate(el);
    f.wire(el);
  }

  function init(mode) {
    if (mode && FILLS[mode]) {
      var el = $('#' + mode + '-pop');
      if (el) {
        fill(mode, el);
        el.classList.remove('hidden');
        armClose(el, mode);
      }
    } else if (mode === 'settings') {
      var sw = $('#settings-win');
      if (sw) { sw.innerHTML = settingsBody(); sw.classList.remove('hidden'); hydrate(sw); wireSettings(sw); armClose(sw, mode); }
    } else if (mode === 'calendar') {
      var s = shell();
      if (s && s.renderCalendar) s.renderCalendar();
      var cp = $('#calendar-pop');
      if (cp) { cp.classList.remove('hidden'); armClose(cp, mode); }
    } else if (mode === 'shade') {
      var np = $('#notif-panel');
      if (np) { np.classList.remove('hidden'); armClose(np, mode); }
    } else if (mode === 'launcher') {
      var li = $('#launcher-input');
      if (li) setTimeout(function () { li.focus(); }, 120);
    }
  }

  function armClose(el, mode) {
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') { if (isWindowMode()) selfClose(); else el.classList.add('hidden'); }
    });
    // popovers behave like real DE popovers: they close when they lose focus
    var blurClose = ['wifi', 'audio', 'battery', 'calendar', 'power', 'launcher'];
    if (isWindowMode() && blurClose.indexOf(mode) !== -1) {
      window.addEventListener('blur', function () { setTimeout(selfClose, 160); });
    }
  }

  global.PanelWin = { fill: fill, init: init, selfClose: selfClose, isWindowMode: isWindowMode };
})(window);
