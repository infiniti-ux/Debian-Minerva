/* Debian Minerva — dependency-free SVG icon set (no external assets). */
// version marker: 0.9.0
(function (global) {
  'use strict';

  const P = {
    folder: '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
    globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c2.5 2.5 3.8 5.6 3.8 9s-1.3 6.5-3.8 9c-2.5-2.5-3.8-5.6-3.8-9S9.5 5.5 12 3z"/>',
    terminal: '<rect x="2.5" y="4" width="19" height="16" rx="2"/><path d="M6.5 9l3 3-3 3M12 15h5"/>',
    edit: '<path d="M12 20h9"/><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L8 18l-4 1 1-4z"/>',
    settings: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1-2.8 2.8-.1-.1a1.6 1.6 0 0 0-1.8-.3 1.6 1.6 0 0 0-1 1.5V21h-4v-.2a1.6 1.6 0 0 0-1-1.5 1.6 1.6 0 0 0-1.8.3l-.1.1L4.2 16l.1-.1a1.6 1.6 0 0 0 .3-1.8 1.6 1.6 0 0 0-1.5-1H3v-4h.2a1.6 1.6 0 0 0 1.5-1 1.6 1.6 0 0 0-.3-1.8L4.3 6.2 7.1 3.4l.1.1a1.6 1.6 0 0 0 1.8.3 1.6 1.6 0 0 0 1-1.5V2.2h4v.1a1.6 1.6 0 0 0 1 1.5 1.6 1.6 0 0 0 1.8-.3l.1-.1 2.8 2.8-.1.1a1.6 1.6 0 0 0-.3 1.8 1.6 1.6 0 0 0 1.5 1h.2v4h-.2a1.6 1.6 0 0 0-1.5 1z"/>',
    package: '<path d="M12 2.5 21 7v10l-9 4.5L3 17V7z"/><path d="M3 7l9 4.5L21 7M12 11.5v10"/>',
    activity: '<path d="M2 12h4l3-8 4 16 3-8h6"/>',
    calc: '<rect x="4" y="2.5" width="16" height="19" rx="2"/><path d="M8 7h8M8 12h.01M12 12h.01M16 12h.01M8 16h.01M12 16h.01M16 16h.01"/>',
    clock: '<circle cx="12" cy="12" r="9"/><path d="M12 7v5l3.5 2"/>',
    calendar: '<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4"/>',
    note: '<path d="M5 3h9l5 5v13a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z"/><path d="M14 3v5h5M8 13h8M8 17h6"/>',
    image: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="8.5" cy="9" r="1.5"/><path d="M21 16l-5-5-9 9"/>',
    mail: '<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 7l9 6 9-6"/>',
    music: '<path d="M9 18V5l11-2v13"/><circle cx="6" cy="18" r="3"/><circle cx="17" cy="16" r="3"/>',
    camera: '<rect x="2.5" y="6" width="19" height="13" rx="2"/><circle cx="12" cy="12.5" r="3.5"/><path d="M8 6l1.5-2h5L16 6"/>',
    info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>',
    widgets: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
    monitor: '<rect x="2.5" y="3.5" width="19" height="14" rx="2"/><path d="M8 21h8M12 17.5V21"/>',
    search: '<circle cx="11" cy="11" r="7"/><path d="M20 20l-4-4"/>',
    mic: '<rect x="9" y="2.5" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8 21h8"/>',
    grid: '<circle cx="5" cy="5" r="1.4"/><circle cx="12" cy="5" r="1.4"/><circle cx="19" cy="5" r="1.4"/><circle cx="5" cy="12" r="1.4"/><circle cx="12" cy="12" r="1.4"/><circle cx="19" cy="12" r="1.4"/><circle cx="5" cy="19" r="1.4"/><circle cx="12" cy="19" r="1.4"/><circle cx="19" cy="19" r="1.4"/>',
    sync: '<path d="M20 11a8 8 0 0 0-14.3-4.5L4 8M4 4v4h4M4 13a8 8 0 0 0 14.3 4.5L20 16M20 20v-4h-4"/>',
    location: '<path d="M12 21s7-6.2 7-11a7 7 0 1 0-14 0c0 4.8 7 11 7 11z"/><circle cx="12" cy="10" r="2.5"/>',
    record: '<circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="4" fill="currentColor" stroke="none"/>',
    bell: '<path d="M18 8a6 6 0 1 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4"/>',
    wifi: '<path d="M2 8.5a15 15 0 0 1 20 0M5.5 12a10 10 0 0 1 13 0M9 15.5a5 5 0 0 1 6 0"/><circle cx="12" cy="19" r="1" fill="currentColor" stroke="none"/>',
    battery: '<rect x="2" y="7" width="18" height="10" rx="2.5"/><path d="M22 10.5v3"/><rect x="4" y="9" width="13" height="6" rx="1" fill="currentColor" stroke="none" opacity=".75"/>',
    power: '<path d="M12 2.5v9M6.3 5.7a9 9 0 1 0 11.4 0"/>',
    close: '<path d="M6 6l12 12M18 6L6 18"/>',
    minus: '<path d="M5 12h14"/>',
    square: '<rect x="5" y="5" width="14" height="14" rx="2"/>',
    restore: '<rect x="7" y="7" width="12" height="12" rx="2"/><path d="M10 7V5h9v9h-2"/>',
    sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/>',
    volume: '<path d="M4 9v6h4l5 4V5L8 9zM17 9a4 4 0 0 1 0 6M19.5 6.5a7.5 7.5 0 0 1 0 11"/>',
    volumeMute: '<path d="M4 9v6h4l5 4V5L8 9z"/><path d="M17 9.5l5 5M22 9.5l-5 5"/>',
    bluetooth: '<path d="M7 7l10 10-5 4V3l5 4L7 17"/>',
    moon: '<path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5z"/>',
    plane: '<path d="M21 15.5 13.5 12V5.5a1.5 1.5 0 0 0-3 0V12L3 15.5V18l7.5-2v3.5L8.5 21v1l3.5-1 3.5 1v-1l-2-1.5V16L21 18z"/>',
    shield: '<path d="M12 2.5 4.5 6v6c0 4.8 3.2 8.3 7.5 9.5 4.3-1.2 7.5-4.7 7.5-9.5V6z"/><path d="M9 12l2 2 4-4"/>',
    chevronLeft: '<path d="M15 18l-6-6 6-6"/>',
    chevronRight: '<path d="M9 18l6-6-6-6"/>',
    chevronDown: '<path d="M6 9l6 6 6-6"/>',
    home: '<path d="M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
    download: '<path d="M12 3v12M7 10l5 5 5-5M4 20h16"/>',
    doc: '<path d="M6 2.5h8l4 4V21H6z"/><path d="M14 2.5v4h4M9 12h6M9 16h6"/>',
    trash: '<path d="M4 7h16M9 7V4h6v3M6 7l1 14h10l1-14M10 11v6M14 11v6"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    check: '<path d="M4 12.5l5 5L20 6"/>',
    refresh: '<path d="M20 11a8 8 0 0 0-14.3-4.5L4 8M4 4v4h4M4 13a8 8 0 0 0 14.3 4.5L20 16M20 20v-4h-4"/>',
    play: '<path d="M7 4.5v15l13-7.5z"/>',
    pause: '<path d="M8 4.5v15M16 4.5v15"/>',
    skip: '<path d="M5 4.5v15l10-7.5zM18 4.5v15"/>',
    user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
    logout: '<path d="M10 4H5a1 1 0 0 0-1 1v14a1 1 0 0 0 1 1h5M14 8l4 4-4 4M18 12H9"/>',
    lock: '<rect x="4" y="10" width="16" height="11" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/>',
    palette: '<path d="M12 3a9 9 0 1 0 0 18h1.5a2 2 0 0 0 1.5-3.3 1.6 1.6 0 0 1 1.2-2.7H19a2 2 0 0 0 2-2A9 9 0 0 0 12 3z"/><circle cx="7.5" cy="11" r="1"/><circle cx="10" cy="7.5" r="1"/><circle cx="15" cy="7.5" r="1"/>',
    keyboard: '<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h.01M10 9h.01M14 9h.01M18 9h.01M6 13h.01M10 13h.01M14 13h.01M18 13h.01M8 16h8"/>',
    display: '<rect x="2.5" y="3.5" width="19" height="14" rx="2"/><path d="M8 21h8M12 17.5V21M9 10h6"/>',
    help: '<circle cx="12" cy="12" r="9"/><path d="M9.5 9a2.5 2.5 0 1 1 3.4 2.3c-.9.4-1.4 1-1.4 2M12 17h.01"/>',
    arrowLeft: '<path d="M19 12H5M11 6l-6 6 6 6"/>',
    arrowRight: '<path d="M5 12h14M13 6l6 6-6 6"/>',
    file: '<path d="M6 2.5h8l4 4V21H6z"/><path d="M14 2.5v4h4"/>',
    bolt: '<path d="M13 2 4 14h7l-1 8 9-12h-7z"/>',
    cpu: '<rect x="5" y="5" width="14" height="14" rx="2"/><rect x="9" y="9" width="6" height="6" rx="1"/><path d="M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3"/>',
    smile: '<circle cx="12" cy="12" r="9"/><path d="M8 14a5 5 0 0 0 8 0M8.5 9h.01M15.5 9h.01"/>',
  };

  const APP_COLORS = {
    files: ['#e8f0ff', '#3b6fe0'], browser: ['#e7f4ff', '#1a73c7'],
    terminal: ['#1c2230', '#7fd1a6'], editor: ['#fff2e0', '#d97706'],
    settings: ['#ece9fb', '#6d4bd8'], software: ['#ffe9ef', '#c2185b'],
    monitor: ['#e6f6ef', '#0f8a5f'], calculator: ['#f0ecff', '#5b3ec7'],
    clock: ['#e8eefc', '#2f55a8'], calendar: ['#ffecef', '#c33a54'],
    notes: ['#fff6d9', '#c49000'], gallery: ['#eae4fb', '#7b4bc9'],
    mail: ['#e5f2ff', '#0b65b5'], music: ['#fce7f3', '#c2256b'],
    camera: ['#e9edf3', '#3d4a63'], help: ['#e6f6f6', '#0d7d84'],
    widgets: ['#eef0fd', '#4654c9'], display: ['#eaf3ff', '#2563a8'],
    system: ['#ffecef', '#a80030'],
  };

  function svg(name, size) {
    size = size || 18;
    const body = P[name] || P.info;
    return '<svg viewBox="0 0 24 24" width="' + size + '" height="' + size +
      '" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">' +
      body + '</svg>';
  }

  /** Coloured app tile used in the launcher / dock / desktop. */
  function appTile(appId, iconName, size) {
    size = size || 40;
    const c = APP_COLORS[appId] || APP_COLORS.system;
    const glyph = Math.round(size * 0.55);
    return '<span class="app-tile" style="width:' + size + 'px;height:' + size +
      'px;background:' + c[0] + ';color:' + c[1] + ';border-radius:' + Math.round(size * 0.28) +
      'px;display:inline-grid;place-items:center;flex:none">' + svg(iconName || appId, glyph) + '</span>';
  }

  /** Replace every <span data-icon="x"> in a subtree. */
  function hydrate(root) {
    (root || document).querySelectorAll('[data-icon]').forEach(function (el) {
      if (el.dataset.hydrated) return;
      el.innerHTML = svg(el.dataset.icon, Number(el.dataset.size) || 18);
      el.dataset.hydrated = '1';
    });
  }


  /* ---------------------------------------------------------------- app art
   * Colourful adaptive artwork for the launcher / dock / desktop / windows.
   * Shapes follow the Aluminium OS leaks (multi-colour, squircle/circle) but
   * every glyph is neutral Debian Minerva artwork — no Google logos. */
  const ART_BG = {
    files:'#3b6fe0', browser:'#fff', terminal:'#20242c', editor:'#eda118',
    settings:'#9334e6', software:'#2f9e63', monitor:'#00897b', calculator:'#5f6368',
    clock:'#3b6fe0', calendar:'#fff', notes:'#eda118', gallery:'#fff',
    mail:'#fff', music:'#d81b60', camera:'#455a64', help:'#00838f',
    widgets:'#7b1fa2', display:'#3f51b5', system:'#a80030'
  };
  const APP_GLYPH = {
    files:'folder', browser:'globe', terminal:'terminal', editor:'edit',
    settings:'settings', software:'package', monitor:'activity', calculator:'calc',
    clock:'clock', calendar:'calendar', notes:'note', gallery:'image',
    mail:'mail', music:'music', camera:'camera', help:'info', widgets:'widgets',
    display:'monitor', system:'info'
  };
  const ART = {
    browser: '<circle cx="24" cy="24" r="23" fill="#2fa36b"/>' +
      '<path d="M24 24 L24 1 A23 23 0 0 1 43.9 35.5 Z" fill="#e05252"/>' +
      '<path d="M24 24 L43.9 35.5 A23 23 0 0 1 4.1 35.5 Z" fill="#f0b429"/>' +
      '<path d="M24 24 L4.1 35.5 A23 23 0 0 1 24 1 Z" fill="#3b6fe0"/>' +
      '<circle cx="24" cy="24" r="10" fill="#fff"/><circle cx="24" cy="24" r="7.6" fill="#3b6fe0"/>' +
      '<path d="M16.6 24h14.8M24 16.6v14.8" stroke="#fff" stroke-width="1" opacity=".55"/>',
    gallery: '<circle cx="24" cy="24" r="23" fill="#f1f3f4"/>' +
      '<path d="M23 4a20 20 0 0 1 20 20h-9A11 11 0 0 0 23 13z" fill="#f0b429"/>' +
      '<path d="M43 24a20 20 0 0 1-20 20v-9a11 11 0 0 0 11-11z" fill="#e05252"/>' +
      '<path d="M23 44A20 20 0 0 1 3 24h9a11 11 0 0 0 11 11z" fill="#2fa36b"/>' +
      '<path d="M3 24A20 20 0 0 1 23 4v9A11 11 0 0 0 12 24z" fill="#3b6fe0"/>',
    mail: '<rect x="2" y="9" width="44" height="30" rx="5" fill="#fff"/>' +
      '<path d="M4 13l20 14 20-14" fill="none" stroke="#e05252" stroke-width="6" stroke-linejoin="round" stroke-linecap="round"/>' +
      '<path d="M4 35h40" stroke="#3b6fe0" stroke-width="4" stroke-linecap="round"/>' +
      '<path d="M4 13v22M44 13v22" stroke="#2fa36b" stroke-width="3" stroke-linecap="round"/>',
    calendar: '<rect x="4" y="6" width="40" height="36" rx="5" fill="#fff"/>' +
      '<path d="M4 11a5 5 0 0 1 5-5h30a5 5 0 0 1 5 5v6H4z" fill="#e05252"/>' +
      '<rect x="13" y="2" width="3.4" height="9" rx="1.7" fill="#b23b3b"/>' +
      '<rect x="32" y="2" width="3.4" height="9" rx="1.7" fill="#b23b3b"/>' +
      '<text x="24" y="35" text-anchor="middle" font-size="19" font-weight="700" fill="#3c4043" font-family="Arial,sans-serif">' + new Date().getDate() + '</text>',
    clock: '<circle cx="24" cy="24" r="22" fill="#3b6fe0"/>' +
      '<circle cx="24" cy="24" r="17.5" fill="#fff"/>' +
      '<path d="M24 13v11l8 5" stroke="#3b6fe0" stroke-width="3" fill="none" stroke-linecap="round" stroke-linejoin="round"/>' +
      '<circle cx="24" cy="24" r="2.6" fill="#e05252"/>' +
      '<path d="M24 8v3M40 24h-3M24 40v-3M8 24h3" stroke="#a9c4f5" stroke-width="2" stroke-linecap="round"/>',
    terminal: '<rect x="1" y="1" width="46" height="46" rx="23" fill="#20242c"/>' +
      '<path d="M11 18l7 6-7 6" stroke="#7fd1a6" stroke-width="3.2" fill="none" stroke-linecap="round" stroke-linejoin="round"/>' +
      '<path d="M23 31h12" stroke="#7fd1a6" stroke-width="3.2" stroke-linecap="round"/>' +
      '<circle cx="10" cy="9" r="1.6" fill="#e05252"/><circle cx="15" cy="9" r="1.6" fill="#f0b429"/><circle cx="20" cy="9" r="1.6" fill="#2fa36b"/>',
    files: '<circle cx="24" cy="24" r="23" fill="#3b6fe0"/>' +
      '<path d="M11 18a3 3 0 0 1 3-3h7l3 3h9a3 3 0 0 1 3 3v12a3 3 0 0 1-3 3H14a3 3 0 0 1-3-3z" fill="#fff"/>' +
      '<path d="M11 25h26v8a3 3 0 0 1-3 3H14a3 3 0 0 1-3-3z" fill="#a9c4f5"/>',
    editor: '<circle cx="24" cy="24" r="23" fill="#eda118"/>' +
      '<path d="M14 8h14l7 7v24a2 2 0 0 1-2 2H14a2 2 0 0 1-2-2V10a2 2 0 0 1 2-2z" fill="#fff"/>' +
      '<path d="M28 8v7h7z" fill="#ffe6b8"/>' +
      '<path d="M17 22h13M17 27h13M17 32h9" stroke="#eda118" stroke-width="2.2" stroke-linecap="round"/>',
    settings: '<circle cx="24" cy="24" r="23" fill="#9334e6"/>' +
      '<g fill="#fff"><circle cx="24" cy="24" r="13"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2" transform="rotate(45 24 24)"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2" transform="rotate(90 24 24)"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2" transform="rotate(135 24 24)"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2" transform="rotate(180 24 24)"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2" transform="rotate(225 24 24)"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2" transform="rotate(270 24 24)"/>' +
      '<rect x="21" y="5" width="6" height="12" rx="2" transform="rotate(315 24 24)"/></g>' +
      '<circle cx="24" cy="24" r="6" fill="#9334e6"/>'
  };

  /** Colourful app artwork. `id` is an app id from the catalogue. */
  function hashColor(id) {
    var h = 0, s = String(id || '');
    for (var i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) % 360;
    return 'hsl(' + h + ', 52%, 44%)';
  }

  function appArt(id, size) {
    size = size || 40;
    var inner = ART[id];
    if (!inner) {
      inner = '<circle cx="24" cy="24" r="23" fill="' + (ART_BG[id] || hashColor(id)) + '"/>' +
        '<g transform="translate(5.2 5.2) scale(1.57)" fill="none" stroke="#fff" ' +
        'stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round" opacity=".97">' +
        (P[APP_GLYPH[id]] || P.info) + '</g>';
    }
    return '<svg class="app-art" viewBox="0 0 48 48" width="' + size + '" height="' + size +
      '" aria-hidden="true">' + inner + '</svg>';
  }

  global.Icon = { svg: svg, appTile: appTile, appArt: appArt, hydrate: hydrate, names: Object.keys(P) };
})(window);
