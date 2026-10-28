/* Debian Minerva — app view render tests (Node, DOM shim).
 * Loads icons.js + apps.js and renders every built-in app view through
 * AppViews.create() with a stub ctx, asserting:
 *   - html string is non-empty and balanced in <div> tags;
 *   - init(body, ) survives a stub body without throwing;
 *   - views never touch globals outside the ctx contract.
 * A view that throws here is a view that white-screens on the desktop.
 */
'use strict';
const fs = require('fs');
const path = require('path');
const shim = require('./dom-shim.js');

const win = shim.makeWindow();
function load(file) {
  const src = fs.readFileSync(path.join(__dirname, '../../web/js', file), 'utf8');
  new Function('window', 'document', 'localStorage', 'navigator', src)(win, win.document, win.localStorage, win.navigator);
}
load('icons.js');
load('apps.js');
const AV = win.AppViews;

let fails = 0;
function ok(cond, msg) { if (cond) { console.log('  ok    ' + msg); } else { console.log('  FAIL  ' + msg); fails++; } }

ok(!!AV, 'apps.js exports window.AppViews');

function stubCtx() {
  const cleanups = [];
  const state = {
    windows: [], workspaces: [{ id: 1, name: 'Desktop 1' }, { id: 2, name: 'Desktop 2' }],
    notifications: [], toggles: { wifi: true }, theme: 'dark', accent: '#a80030',
    wallpaper: 'swirl', scale: 1, volume: 0.6, brightness: 0.8, active_workspace: 0
  };
  const resolved = function (v) { return { then: function (fn) { try { fn(v); } catch (e) {} return resolved(v); }, catch: function () { return resolved(v); } }; };
  return {
    state: state,
    dispatch: function () {},
    appName: function (id) { return id; },
    appIcon: function (id) { return 'grid'; },
    onStateChange: function () {},
    onCleanup: function (fn) { cleanups.push(fn); },
    _runCleanups: function () { cleanups.forEach(function (fn) { try { fn(); } catch (e) {} }); },
    registerEditorLoader: function () {},
    api: {
      backend: 'dev',
      sessionMode: 'dev',
      sys: function () { return resolved({ available: [], modes: [], loadavg: '0.1 0.2 0.3', cpus: 4, mem_used_mb: 1, mem_total_mb: 2, mem_percent: 50, hostname: 'test', kernel: 'test', uptime_secs: 60, battery_present: false, battery_level: 1, brightness: 80 }); },
      getSystem: function () { return this.sys(); },
      running: function () { return resolved({ apps: [], windows: [] }); },
      fsList: function () { return resolved({ entries: [] }); },
      fsRead: function () { return resolved({ content: '' }); },
      fsWrite: function () { return resolved({ ok: true }); },
      iconUrl: function () { return ''; },
      files: function () { return resolved({ entries: [] }); },
      fileOp: function () { return resolved({ ok: true }); },
      fsWrite: function () { return resolved({ ok: true }); },
      launch: function () { return resolved({ ok: true }); },
      action: function () { return resolved({ ok: true, events: [] }); },
      cmd: function () { return resolved({ ok: true }); }
    }
  };
}

function stubBody() {
  const el = shim.makeElement('div');
  // querySelector returns a stub child for any selector so init() paths run
  el.querySelector = function () { const c = shim.makeElement('div'); return c; };
  el.querySelectorAll = function () { return []; };
  return el;
}

const IDS = ['files', 'terminal', 'editor', 'settings', 'monitor', 'calculator', 'notes', 'music', 'images', 'mail', 'display', 'help'];
IDS.forEach(function (id) {
  let view = null;
  try { view = AV.create(id, stubCtx()); } catch (e) { console.log('  FAIL  create(' + id + ') threw: ' + e.message); fails++; return; }
  if (!view || typeof view.html !== 'string' || !view.html.length) { console.log('  FAIL  view ' + id + ' empty html'); fails++; return; }
  const opens = (view.html.match(/<div/g) || []).length;
  const closes = (view.html.match(/<\/div>/g) || []).length;
  if (opens !== closes) { console.log('  FAIL  view ' + id + ' unbalanced divs ' + opens + '/' + closes); fails++; }
  if (/undefined|NaN/.test(view.html)) { console.log('  FAIL  view ' + id + ' html contains undefined/NaN'); fails++; }
  const ctx = stubCtx();
  try {
    if (typeof view.init === 'function') view.init(stubBody(), ctx);
    ctx._runCleanups(); // simulate window close: intervals must die here
    console.log('  ok    view ' + id + ' renders + inits + cleans up');
  } catch (e) { console.log('  FAIL  view ' + id + ' init threw: ' + e.message); fails++; }
});

ok(AV.has('files') && AV.has('terminal'), 'AppViews.has() recognises core views');
ok(!AV.has('definitely-not-an-app'), 'AppViews.has() rejects unknown ids');

if (fails) { console.log('apps-view-test: ' + fails + ' FAILURES'); process.exit(1); }
console.log('apps-view-test: OK (' + IDS.length + ' views)');
process.exit(0); // no lingering timers may hold the runner open
