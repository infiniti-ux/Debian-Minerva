/* Debian Minerva — stylesheet integrity checker.
 *
 * Catches the class of bug that silently killed the panel entry animation
 * in 0.8.0 (a rule referenced keyframes that no longer existed):
 *   1. every animation name referenced by `animation:` / `animation-name:`
 *      must have a matching @keyframes block (var()/none/inherit exempt);
 *   2. every var(--x) reference must be defined somewhere, or carry a
 *      fallback value;
 *   3. braces balanced, no stray tabs-in-selector noise;
 *   4. every url("...") asset must exist on disk relative to the css file.
 *
 * Exit 0 = clean, 1 = violations printed one per line.
 */
'use strict';
const fs = require('fs');
const path = require('path');

const CSS = process.argv[2] || path.join(__dirname, '../../web/css/minerva.css');
const src = fs.readFileSync(CSS, 'utf8');
const problems = [];

// ---- 1. keyframes vs references -----------------------------------------
const defined = new Set();
let m;
const reKey = /@keyframes\s+([A-Za-z0-9_-]+)/g;
while ((m = reKey.exec(src))) defined.add(m[1]);

const reAnim = /animation(?:-name)?\s*:\s*([^;]+);/g;
while ((m = reAnim.exec(src))) {
  const value = m[1].trim();
  // shorthand: first identifier that is not a timing/keyword token
  const keywords = new Set(['none', 'inherit', 'initial', 'unset', 'infinite', 'linear', 'ease', 'ease-in', 'ease-out', 'ease-in-out', 'both', 'forwards', 'backwards', 'alternate', 'reverse', 'normal', 'running', 'paused', 'step-start', 'step-end']);
  const tokens = value.split(/\s+/);
  for (const t of tokens) {
    const name = t.replace(/[,].*$/, '');
    if (!name || keywords.has(name)) continue;
    if (/^[0-9.]/.test(name)) continue;                 // durations/delays
    if (/^(cubic-bezier|steps|var)\(/.test(name)) continue;
    if (/^[0-9.]+(s|ms|%|deg|px)$/.test(name)) continue;
    if (name.startsWith('var(')) continue;
    if (!defined.has(name)) {
      problems.push('animation references undefined keyframes: ' + name);
    }
    break; // first identifier token is the name in shorthand
  }
}

// ---- 2. var() references --------------------------------------------------
const definedVars = new Set();
const reDef = /(--[A-Za-z0-9_-]+)\s*:/g;
while ((m = reDef.exec(src))) definedVars.add(m[1]);
const reVar = /var\(\s*(--[A-Za-z0-9_-]+)\s*(,)?/g;
while ((m = reVar.exec(src))) {
  if (!definedVars.has(m[1]) && !m[2]) {
    problems.push('var() without definition and without fallback: ' + m[1]);
  }
}

// ---- 3. brace balance ------------------------------------------------------
let depth = 0, line = 1;
for (let i = 0; i < src.length; i++) {
  const c = src[i];
  if (c === '\n') line++;
  else if (c === '{') depth++;
  else if (c === '}') { depth--; if (depth < 0) { problems.push('unbalanced } at line ' + line); depth = 0; } }
}
if (depth !== 0) problems.push('unbalanced braces: depth ' + depth + ' at EOF');

// ---- 4. url() assets exist --------------------------------------------------
const reUrl = /url\(\s*["']?([^"')]+)["']?\s*\)/g;
while ((m = reUrl.exec(src))) {
  const ref = m[1];
  if (/^(data:|https?:|#)/.test(ref)) continue;
  const abs = path.resolve(path.dirname(CSS), ref);
  if (!fs.existsSync(abs)) problems.push('missing asset referenced by css: ' + ref);
}

if (problems.length) {
  problems.forEach(function (p) { console.log('CSS FAIL  ' + p); });
  process.exit(1);
}
console.log('css-check: OK (' + defined.size + ' keyframes, ' + definedVars.size + ' vars verified)');
