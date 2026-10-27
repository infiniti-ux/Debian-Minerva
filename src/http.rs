//! Tiny HTTP/1.1 server for the Minerva shell and its JSON API.
//!
//! Kept deliberately small and dependency-free: one thread per connection,
//! a hand-written request parser, and a shared `Mutex<SessionState>`. On a real
//! Debian install the session script binds it to 127.0.0.1 with a
//! per-session token (`--token`); for development it can bind anywhere.

use crate::core::{SessionState, catalog_json};
use crate::exec::Exec;
use crate::json::{self, Value};
use crate::services::{self, Vfs, clock_json, system_json};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Server {
    pub port: u16,
    pub bind: String,
    /// When set, every `/api/v1/*` route except `/api/v1/health` requires a
    /// matching `X-Minerva-Token` header. The session script mints one token
    /// per login and hands it to the kiosk shell through the launch URL.
    pub token: Option<String>,
    pub web_root: PathBuf,
    pub data_dir: PathBuf,
    pub state: Arc<Mutex<SessionState>>,
    pub vfs: Arc<Vfs>,
    pub exec: Arc<Exec>,
}

struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: String,
}

struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
    extra: Vec<(String, String)>,
}

impl Response {
    fn text(status: u16, body: String) -> Self {
        Response { status, content_type: "text/plain; charset=utf-8", body: body.into_bytes(), extra: vec![] }
    }
    fn json(status: u16, v: Value) -> Self {
        Response {
            status,
            content_type: "application/json; charset=utf-8",
            body: json::stringify(&v).into_bytes(),
            extra: vec![("Cache-Control".into(), "no-store".into())],
        }
    }
    fn bytes(status: u16, ct: &'static str, body: Vec<u8>) -> Self {
        Response { status, content_type: ct, body, extra: vec![("Cache-Control".into(), "no-cache".into())] }
    }
}

impl Server {
    pub fn run(&self) -> std::io::Result<()> {
        let addr = format!("{}:{}", self.bind, self.port);
        let listener = TcpListener::bind(&addr)?;
        println!("  listening on http://{}", addr);
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let srv = self.clone_context();
                    thread::spawn(move || {
                        let _ = handle_connection(stream, srv);
                    });
                }
                Err(e) => eprintln!("minerva: accept error: {}", e),
            }
        }
        Ok(())
    }

    fn clone_context(&self) -> Server {
        Server {
            port: self.port,
            bind: self.bind.clone(),
            token: self.token.clone(),
            web_root: self.web_root.clone(),
            data_dir: self.data_dir.clone(),
            state: Arc::clone(&self.state),
            vfs: Arc::clone(&self.vfs),
            exec: Arc::clone(&self.exec),
        }
    }
}

fn handle_connection(mut stream: TcpStream, srv: Server) -> std::io::Result<()> {
    stream.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let req = match read_request(&mut stream) {
        Ok(Some(r)) => r,
        Ok(None) => return Ok(()),
        Err(e) => {
            let r = Response::text(400, format!("bad request: {}", e));
            return write_response(&mut stream, r);
        }
    };
    let resp = route(&req, &srv);
    write_response(&mut stream, resp)
}

fn read_request(stream: &mut TcpStream) -> Result<Option<Request>, String> {
    let mut buf = Vec::with_capacity(4096);
    let mut tmp = [0u8; 4096];
    let header_end;
    let mut total = 0usize;
    loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(None);
        }
        total += n;
        if total > 4 * 1024 * 1024 {
            return Err("request too large".into());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_bytes(&buf, b"\r\n\r\n") {
            header_end = pos + 4;
            break;
        }
    }
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let start = lines.next().ok_or("empty request")?;
    let mut start_parts = start.split_whitespace();
    let method = start_parts.next().ok_or("no method")?.to_string();
    let target = start_parts.next().ok_or("no target")?.to_string();
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() { continue; }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_lowercase(), v.trim().to_string()));
        }
    }
    let content_len = headers.iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok())
        .unwrap_or(0);
    while buf.len() < header_end + content_len {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        buf.extend_from_slice(&tmp[..n]);
    }
    let body = String::from_utf8_lossy(&buf[header_end..header_end + content_len.min(buf.len() - header_end)]).to_string();
    Ok(Some(Request { method, path, query, headers, body }))
}

fn find_bytes(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn write_response(stream: &mut TcpStream, resp: Response) -> std::io::Result<()> {
    let reason = match resp.status {
        200 => "OK", 201 => "Created", 204 => "No Content",
        400 => "Bad Request", 403 => "Forbidden", 404 => "Not Found",
        405 => "Method Not Allowed", 500 => "Internal Server Error", _ => "OK",
    };
    let mut head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nX-Powered-By: Debian-Minerva/Rust\r\nConnection: close\r\n",
        resp.status, reason, resp.content_type, resp.body.len()
    );
    for (k, v) in &resp.extra {
        head.push_str(&format!("{}: {}\r\n", k, v));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(&resp.body)?;
    stream.flush()
}

fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == key { Some(percent_decode(v)) } else { None }
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn route(req: &Request, srv: &Server) -> Response {
    if req.method == "OPTIONS" {
        return Response::text(204, String::new());
    }
    let path = req.path.as_str();
    if path.starts_with("/api/v1/") {
        return api(req, srv);
    }
    if req.method != "GET" {
        return Response::text(405, "method not allowed".into());
    }
    serve_static(path, &srv.web_root)
}

fn serve_static(path: &str, root: &Path) -> Response {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if rel.contains("..") || rel.contains('\\') {
        return Response::text(403, "forbidden".into());
    }
    let mut file = root.join(rel);
    if file.is_dir() {
        file = file.join("index.html");
    }
    match fs::read(&file) {
        Ok(data) => Response::bytes(200, mime_type(&file), data),
        Err(_) => Response::text(404, format!("404 — {} not found", rel)),
    }
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn ok(v: Value) -> Response { Response::json(200, v) }
fn err(msg: impl Into<String>) -> Response {
    Response::json(400, Value::obj(vec![("ok", Value::Bool(false)), ("error", Value::s(msg.into()))]))
}

fn api(req: &Request, srv: &Server) -> Response {
    let path = req.path.as_str();
    // Session token gate: health stays open so the session script can probe
    // readiness; everything else is session-private.
    if let Some(expected) = &srv.token {
        if path != "/api/v1/health" {
            let given = req.headers.iter()
                .find(|(k, _)| k == "x-minerva-token")
                .map(|(_, v)| v.clone())
                .or_else(|| query_param(&req.query, "token"));
            if given.as_deref() != Some(expected.as_str()) {
                return Response::json(401, Value::obj(vec![
                    ("ok", Value::Bool(false)),
                    ("error", Value::s("missing or invalid session token".into())),
                ]));
            }
        }
    }
    if path == "/api/v1/icon" {
        let name = query_param(&req.query, "name").unwrap_or_default();
        return match srv.exec.icon(&name) {
            Some((mime, bytes)) => Response::bytes(200, mime, bytes),
            None => Response::text(404, "icon not found".into()),
        };
    }
    match (req.method.as_str(), path) {
        ("GET", "/api/v1/health") => ok(Value::obj(vec![
            ("ok", Value::Bool(true)),
            ("de", Value::s(crate::core::DE_NAME)),
            ("version", Value::s(crate::core::VERSION)),
            ("runtime", Value::s("minerva-sessiond (rust)")),
            ("session_mode", Value::s(srv.exec.session_mode.clone())),
        ])),
        ("GET", "/api/v1/apps") => ok(Value::obj(vec![("apps", catalog_json())])),
        ("GET", "/api/v1/desktop-apps") => ok(srv.exec.desktop_apps()),
        ("GET", "/api/v1/running") => ok(srv.exec.running_json()),
        ("GET", "/api/v1/state") => {
            let state = srv.state.lock().unwrap();
            ok(state.snapshot())
        }
        ("GET", "/api/v1/system") => ok(Value::obj(vec![
            ("system", system_json()),
            ("clock", clock_json()),
        ])),
        ("GET", "/api/v1/files") => {
            let p = query_param(&req.query, "path").unwrap_or_else(|| "/".into());
            match srv.vfs.list(&p) {
                Ok(v) => ok(v),
                Err(e) => err(e),
            }
        }
        ("POST", "/api/v1/files") => {
            let body = json::parse(&req.body);
            let v = match body { Ok(v) => v, Err(e) => return err(format!("invalid JSON: {}", e)) };
            let op = v.get("op").as_str().to_string();
            let p = v.get("path").as_str().to_string();
            let result = match op.as_str() {
                "read" => srv.vfs.read(&p).map(|c| Value::obj(vec![("content", Value::s(c))])),
                "write" => srv.vfs.write(&p, v.get("content").as_str()).map(|_| Value::obj(vec![("ok", Value::Bool(true))])),
                "mkdir" => srv.vfs.mkdir(&p).map(|_| Value::obj(vec![("ok", Value::Bool(true))])),
                "remove" => srv.vfs.remove(&p).map(|_| Value::obj(vec![("ok", Value::Bool(true))])),
                "list" => srv.vfs.list(&p),
                _ => Err("unknown file op".into()),
            };
            match result { Ok(v) => ok(v), Err(e) => err(e) }
        }
        ("POST", "/api/v1/action") => {
            let action = match json::parse(&req.body) {
                Ok(v) => v,
                Err(e) => return err(format!("invalid JSON: {}", e)),
            };
            let mut state = srv.state.lock().unwrap();
            match state.apply(&action) {
                Ok(mut events) => {
                    persist(&*state, &srv.data_dir);
                    let mut extra = Vec::new();
                    extra.append(&mut events);
                    ok(Value::obj(vec![
                        ("ok", Value::Bool(true)),
                        ("events", Value::arr(extra)),
                        ("state", state.snapshot()),
                    ]))
                }
                Err(e) => err(e),
            }
        }
        ("POST", "/api/v1/exec") => {
            let v = match json::parse(&req.body) {
                Ok(v) => v,
                Err(e) => return err(format!("invalid JSON: {}", e)),
            };
            let sys = v.get("sys").as_str().to_string();
            if !sys.is_empty() { return ok(srv.exec.sys(&sys, &v)); }
            let cmd = v.get("cmd").as_str().to_string();
            if !cmd.is_empty() { return ok(srv.exec.session_cmd(&cmd)); }
            if v.get("panel_close").as_bool() { return ok(srv.exec.close_panels()); }
            let panel = v.get("panel").as_str().to_string();
            if !panel.is_empty() { return ok(srv.exec.open_panel(&panel)); }
            let fallback = v.get("app_fallback").as_str().to_string();
            if !fallback.is_empty() { return ok(srv.exec.push_pending(&fallback)); }
            if v.get("pending").as_str() == "take" { return ok(srv.exec.take_pending()); }
            let desktop = v.get("desktop").as_str().to_string();
            let app = v.get("app").as_str().to_string();
            if !desktop.is_empty() {
                let r = srv.exec.launch_desktop_app(&desktop);
                if r.get("ok").as_bool() || app.is_empty() { return ok(r); }
            }
            if !app.is_empty() {
                let arg = v.get("arg").as_str().to_string();
                return ok(srv.exec.launch_app(&app, &arg));
            }
            err("exec: nothing to do")
        }
        ("POST", "/api/v1/terminal") => {
            let v = match json::parse(&req.body) {
                Ok(v) => v,
                Err(e) => return err(format!("invalid JSON: {}", e)),
            };
            let cmd = v.get("cmd").as_str().to_string();
            let mut state = srv.state.lock().unwrap();
            let (output, mut actions) = services::terminal(&mut *state, &srv.vfs, &cmd);
            let mut events = Vec::new();
            for a in actions.drain(..) {
                if a.get("type").as_str() == "notify" {
                    let n = a.clone();
                    if let Ok(mut evs) = state.apply(&n) {
                        events.append(&mut evs);
                    }
                } else if let Ok(mut evs) = state.apply(&a) {
                    events.append(&mut evs);
                }
            }
            persist(&*state, &srv.data_dir);
            ok(Value::obj(vec![
                ("ok", Value::Bool(true)),
                ("output", Value::s(output)),
                ("events", Value::arr(events)),
                ("state", state.snapshot()),
            ]))
        }
        ("POST", "/api/v1/save") => {
            let state = srv.state.lock().unwrap();
            persist(&*state, &srv.data_dir);
            ok(Value::obj(vec![("ok", Value::Bool(true))]))
        }
        _ => Response::json(404, Value::obj(vec![
            ("ok", Value::Bool(false)),
            ("error", Value::s("unknown endpoint")),
        ])),
    }
}

fn persist(state: &SessionState, data_dir: &Path) {
    if let Err(e) = state.save(data_dir) {
        eprintln!("minerva: failed to persist session: {}", e);
    }
}
