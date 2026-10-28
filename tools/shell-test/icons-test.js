/* Debian Minerva — icon set unit tests (run under Node, no browser).
 * Loads web/js/icons.js against the DOM shim and asserts:
 *   - every advertised icon name renders a non-empty <svg> string;
 *   - appArt() returns markup for every catalog id used by the shell;
 *   - hydrate() survives an element tree without throwing;
 *   - svg output never contains "undefined" or "NaN" (broken path data).
 */
'use strict';
const fs = require('fs');
const path = require('path');
const shim = require('./dom-shim.js');

const win = shim.makeWindow();
const iconsSrc = fs.readFileSync(path.join(__dirname, '../../web/js/icons.js'), 'utf8');
new Function('window', 'document', 'localStorage', 'navigator', iconsSrc)(win, win.document, win.localStorage, win.navigator);
const I = win.Icon;

let fails = 0;
function ok(cond, msg) { if (cond) { console.log('  ok    ' + msg); } else { console.log('  FAIL  ' + msg); fails++; } }

ok(!!I, 'icons.js exports window.Icon');
ok(Array.isArray(I.names) && I.names.length > 10, 'icon registry populated (' + (I.names || []).length + ' names)');

(I.names || []).forEach(function (name) {
  const s = I.svg(name, 16);
  if (!s || s.indexOf('<svg') !== 0) { console.log('  FAIL  svg(' + name + ') did not render'); fails++; return; }
  if (/undefined|NaN/.test(s)) { console.log('  FAIL  svg(' + name + ') contains undefined/NaN'); fails++; }
});
console.log('  ok    all ' + (I.names || []).length + ' icons render clean svg');

// catalog ids the shell/dock/launcher actually reference
const shellSrc = fs.readFileSync(path.join(__dirname, '../../web/js/shell.js'), 'utf8');
const ids = new Set();
let m;
const reApp = /app:\s*'([a-z0-9-]+)'/g;
while ((m = reApp.exec(shellSrc))) ids.add(m[1]);
const reArt = /I\.appArt\(\s*'([a-z0-9-]+)'/g;
while ((m = reArt.exec(shellSrc))) ids.add(m[1]);
['settings', 'files', 'terminal', 'browser', 'editor', 'monitor', 'calculator', 'notes', 'music', 'images', 'mail'].forEach(function (id) { ids.add(id); });

let artChecked = 0;
ids.forEach(function (id) {
  const art = I.appArt(id, 28);
  if (!art || typeof art !== 'string' || art.length < 10) { console.log('  FAIL  appArt(' + id + ') empty'); fails++; }
  artChecked++;
});
console.log('  ok    appArt rendered for ' + artChecked + ' catalog ids');

// hydrate must not throw on a stub tree
const el = shim.makeElement('div');
el.innerHTML = '<span data-icon="grid"></span>';
try { I.hydrate(el); console.log('  ok    hydrate() survives stub tree'); }
catch (e) { console.log('  FAIL  hydrate() threw: ' + e.message); fails++; }

if (fails) { console.log('icons-test: ' + fails + ' FAILURES'); process.exit(1); }
console.log('icons-test: OK');
