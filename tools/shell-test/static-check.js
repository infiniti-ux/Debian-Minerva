/* Debian Minerva — cross-file static integrity checks.
 *
 * The bug classes this file exists to kill:
 *   - dead DOM selectors: $('#foo') in JS where #foo is not in index.html
 *     (silent no-ops that read as "the button does nothing");
 *   - I.svg('nope') names that do not exist in the icon registry;
 *   - <script src>/<link href>/favicon references to missing files;
 *   - version drift between Cargo.toml, js version markers, index.html
 *     meta and the top stanza of debian/changelog (half-upgraded builds).
 *
 * Exit 0 = clean, 1 = violations.
 */
'use strict';
const fs = require('fs');
const path = require('path');
const shim = require('./dom-shim.js');

const ROOT = path.join(__dirname, '../..');
const WEB = path.join(ROOT, 'web');
const problems = [];
const notes = [];

function read(p) { return fs.readFileSync(path.join(ROOT, p), 'utf8'); }

// ---- icon registry (real load, not regex) ---------------------------------
const win = shim.makeWindow();
new Function('window', 'document', 'localStorage', 'navigator', read('web/js/icons.js'))(win, win.document, win.localStorage, win.navigator);
const iconNames = new Set(win.Icon.names);

// ---- ids present in index.html ---------------------------------------------
const html = read('web/index.html');
const htmlIds = new Set();
let m;
const reId = /id="([A-Za-z0-9_-]+)"/g;
while ((m = reId.exec(html))) htmlIds.add(m[1]);

// ids created dynamically by JS (innerHTML templates) count as present too
const jsFiles = ['web/js/shell.js', 'web/js/apps.js', 'web/js/panelwin.js', 'web/js/api.js'];
const jsSrc = jsFiles.map(read).join('\n');
const reDynId = /id=["']([A-Za-z0-9_-]+)["']/g;
while ((m = reDynId.exec(jsSrc))) htmlIds.add(m[1]);
const reDynId2 = /id=\\"([A-Za-z0-9_-]+)\\"/g;
while ((m = reDynId2.exec(jsSrc))) htmlIds.add(m[1]);
// generated suffix patterns like id="' + id + '-rows'
const reSuffix = /'-([a-z-]+)'/g;
const suffixes = new Set();
while ((m = reSuffix.exec(jsSrc))) suffixes.add(m[1]);

// ---- dead selector scan -----------------------------------------------------
const reSel = /\$\(\s*'#([A-Za-z0-9_-]+)'\s*\)/g;
const seen = new Set();
while ((m = reSel.exec(jsSrc))) {
  const id = m[1];
  if (seen.has(id)) continue;
  seen.add(id);
  if (!htmlIds.has(id)) {
    // allow ids built at runtime from app ids (e.g. '<app>-rows')
    const tail = id.split('-').pop();
    if (!suffixes.has(tail)) problems.push("dead selector '#" + id + "' — no such id in index.html or JS templates");
  }
}
notes.push(seen.size + ' distinct #id selectors verified against index.html+templates');

// ---- svg name scan ------------------------------------------------------------
const reSvg = /I\.svg\(\s*'([A-Za-z0-9_-]+)'/g;
while ((m = reSvg.exec(jsSrc))) {
  if (!iconNames.has(m[1])) problems.push("I.svg('" + m[1] + "') — name not in icon registry");
}
const reIconAttr = /data-icon="([a-z0-9-]+)"/g;
while ((m = reIconAttr.exec(html))) {
  if (!iconNames.has(m[1])) problems.push('index.html data-icon="' + m[1] + '" not in icon registry');
}

// ---- referenced files exist -----------------------------------------------------
const reSrc = /(?:src|href)="([^"#]+)"/g;
while ((m = reSrc.exec(html))) {
  const ref = m[1];
  if (/^(https?:|mailto:|javascript:)/.test(ref)) continue;
  if (!fs.existsSync(path.join(WEB, ref))) problems.push('index.html references missing file: ' + ref);
}

// ---- version coherence -----------------------------------------------------------
const cargo = read('Cargo.toml');
const cargoVer = (cargo.match(/^version\s*=\s*"([0-9.]+)"/m) || [])[1];
const changelog = read('debian/changelog');
const clVer = (changelog.match(/^debian-minerva \(([0-9.]+)\)/m) || [])[1];
const metaVer = (html.match(/minerva-version" content="([0-9.]+)"/) || [])[1];
const markerFiles = ['web/js/shell.js', 'web/js/api.js', 'web/js/apps.js', 'web/js/panelwin.js', 'web/js/icons.js'];
if (!cargoVer) problems.push('Cargo.toml version not found');
if (clVer !== cargoVer) problems.push('changelog top stanza ' + clVer + ' != Cargo.toml ' + cargoVer);
if (metaVer && metaVer !== cargoVer) problems.push('index.html meta version ' + metaVer + ' != Cargo.toml ' + cargoVer);
markerFiles.forEach(function (f) {
  const src = read(f);
  const mk = (src.match(/version marker: ([0-9.]+)/) || src.match(/Debian Minerva ([0-9.]+) — MIT/) || [])[1];
  if (mk && mk !== cargoVer) problems.push(f + ' version marker ' + mk + ' != Cargo.toml ' + cargoVer);
});
notes.push('version coherence checked at ' + cargoVer);

// ---- brace/paren balance per js file ----------------------------------------------
jsFiles.forEach(function (f) {
  const s = read(f);
  let d = 0;
  for (const c of s) { if (c === '{') d++; else if (c === '}') d--; }
  if (d !== 0) problems.push(f + ' unbalanced braces (' + d + ')');
});

if (problems.length) {
  problems.forEach(function (p) { console.log('STATIC FAIL  ' + p); });
  process.exit(1);
}
notes.forEach(function (n) { console.log('  note  ' + n); });
console.log('static-check: OK');
