#!/usr/bin/env node
/**
 * Debian Minerva — sandbox preview host.
 *
 * This is NOT the desktop environment. It is a zero-dependency static file
 * server used when `minerva-sessiond` (the Rust daemon) is unavailable, e.g.
 * in a design sandbox without a Rust toolchain. The shell detects the missing
 * Rust API and runs its built-in browser fallback.
 *
 * On a normal Debian machine, use the real thing instead:
 *     cargo build --release
 *     ./target/release/minerva-sessiond --port 8080 --web-root ./web
 *
 * Usage: node tools/preview.mjs [--port 8080] [--root web]
 */
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(__dirname, '..');
const argv = process.argv.slice(2);
const opt = (name, def) => {
  const i = argv.indexOf(name);
  return i !== -1 && argv[i + 1] ? argv[i + 1] : def;
};
const port = Number(opt('--port', process.env.PORT || 8080));
const root = path.resolve(repo, opt('--root', 'web'));

const MIME = {
  '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8', '.mjs': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8', '.svg': 'image/svg+xml',
  '.png': 'image/png', '.jpg': 'image/jpeg', '.webp': 'image/webp', '.ico': 'image/x-icon'
};

const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://localhost');
  const send = (code, body, type) => {
    res.writeHead(code, {
      'Content-Type': type || 'text/plain; charset=utf-8',
      'Access-Control-Allow-Origin': '*',
      'Cache-Control': 'no-cache'
    });
    res.end(body);
  };

  if (url.pathname === '/api/v1/health') {
    return send(200, JSON.stringify({
      ok: true, de: 'Debian Minerva', version: '0.1.0', runtime: 'preview'
    }), 'application/json');
  }
  if (url.pathname.startsWith('/api/')) {
    return send(404, JSON.stringify({ ok: false, error: 'Rust daemon not attached; shell uses browser fallback' }), 'application/json');
  }

  let rel = decodeURIComponent(url.pathname);
  if (rel === '/') rel = '/index.html';
  const file = path.join(root, rel);
  if (!file.startsWith(root)) return send(403, 'forbidden');
  fs.readFile(file, (err, data) => {
    if (err) return send(404, '404 — ' + rel + ' not found');
    send(200, data, MIME[path.extname(file)] || 'application/octet-stream');
  });
});

server.listen(port, '0.0.0.0', () => {
  console.log('┌──────────────────────────────────────────────────────────────┐');
  console.log('│  Debian Minerva — shell preview                              │');
  console.log('└──────────────────────────────────────────────────────────────┘');
  console.log(`  serving : ${root}`);
  console.log(`  listen  : http://0.0.0.0:${port}`);
  console.log('  backend : browser fallback (Rust daemon not attached)');
  console.log('  hint    : cargo build --release && minerva-sessiond --port ' + port);
});
