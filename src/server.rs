use crate::{ai, engine, models, report, rules, scan, store, types, updates};
use serde_json::{json, Value};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tiny_http::{Header, Method, Response, Server};
use types::{CompareRequest, Overrides, ScanRequest};

const MAX_BODY_BYTES: u64 = 1024 * 1024;
const MAX_CONCURRENT_AUDITS: usize = 3;

/// Crawls and model comparisons are the expensive operations. A public,
/// unauthenticated instance needs a ceiling so one client cannot spawn
/// unlimited crawls or inference threads.
static ACTIVE_AUDITS: AtomicUsize = AtomicUsize::new(0);

struct AuditSlot;

impl AuditSlot {
    fn acquire() -> Option<AuditSlot> {
        let mut current = ACTIVE_AUDITS.load(Ordering::SeqCst);
        loop {
            if current >= MAX_CONCURRENT_AUDITS {
                return None;
            }
            match ACTIVE_AUDITS.compare_exchange(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Some(AuditSlot),
                Err(actual) => current = actual,
            }
        }
    }
}

impl Drop for AuditSlot {
    fn drop(&mut self) {
        ACTIVE_AUDITS.fetch_sub(1, Ordering::SeqCst);
    }
}
const CONTENT_SECURITY_POLICY: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; font-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";
const SECURITY_HEADERS: [(&str, &str); 6] = [
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "DENY"),
    ("referrer-policy", "no-referrer"),
    ("permissions-policy", "camera=(), microphone=(), geolocation=()"),
    ("content-security-policy", CONTENT_SECURITY_POLICY),
    ("cross-origin-opener-policy", "same-origin"),
];

struct Reply {
    status: u16,
    content_type: String,
    body: Vec<u8>,
    disposition: Option<String>,
}

fn json_reply(value: Value, status: u16) -> Reply {
    Reply {
        status,
        content_type: "application/json; charset=utf-8".to_string(),
        body: serde_json::to_vec(&value).unwrap_or_default(),
        disposition: None,
    }
}

fn error_reply(message: &str, status: u16) -> Reply {
    json_reply(json!({ "error": message }), status)
}

fn content_type_for(path: &str) -> &'static str {
    let extension = path.rsplit('.').next().unwrap_or("");
    match extension {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "text/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn query_param(query: &str, key: &str) -> Option<String> {
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.to_string())
}

fn header_value<'a>(headers: &'a [Header], name: &'static str) -> Option<&'a str> {
    headers
        .iter()
        .find(|header| header.field.equiv(name))
        .map(|header| header.value.as_str())
}

fn local_authority(value: &str, port: u16) -> bool {
    let value = value.trim().trim_end_matches('/').to_ascii_lowercase();
    value == format!("127.0.0.1:{port}")
        || value == format!("localhost:{port}")
        || value == format!("[::1]:{port}")
}

fn public_authority(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Lowercased host plus any non-default port, e.g. `demo.hf.space` or
/// `localhost:7860`. Default ports are dropped so an https origin without a
/// port matches the plain Host header a proxy forwards.
fn canonical_authority(value: &str, default_port: Option<u16>) -> Option<String> {
    let value = value.trim().trim_end_matches('/').to_ascii_lowercase();
    if value.is_empty() {
        return None;
    }
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        (format!("[{host}]"), tail.strip_prefix(':').unwrap_or(""))
    } else if let Some((host, port)) = value.rsplit_once(':') {
        if !port.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) {
            (host.to_string(), port)
        } else {
            (value.clone(), "")
        }
    } else {
        (value.clone(), "")
    };
    if host.is_empty() {
        return None;
    }
    let port = match (port, default_port) {
        ("", _) => String::new(),
        (port, Some(default)) if port.parse::<u16>().ok() == Some(default) => String::new(),
        (port, _) => format!(":{port}"),
    };
    Some(format!("{host}{port}"))
}

/// Public deployments accept any Host they are reached on, so the browser
/// Origin must match that host exactly, including a non-default port. Only the
/// scheme and host are compared; the scheme itself is not forced, because a
/// reverse proxy may terminate TLS before the request arrives.
fn public_origin(headers: &[Header], host: &str) -> bool {
    let Some(origin) = header_value(headers, "origin") else {
        return true;
    };
    let value = origin.trim().trim_end_matches('/');
    let Some((scheme, authority)) = value.split_once("://") else {
        return false;
    };
    let default_port = match scheme.to_ascii_lowercase().as_str() {
        "http" => 80,
        "https" => 443,
        _ => return false,
    };
    let Some(origin_key) = canonical_authority(authority, Some(default_port)) else {
        return false;
    };
    canonical_authority(host, None).is_some_and(|host_key| host_key == origin_key)
}

fn loopback_bind(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    value == "127.0.0.1" || value == "localhost" || value == "::1" || value == "[::1]"
}

fn local_origin(headers: &[Header], port: u16) -> bool {
    let Some(origin) = header_value(headers, "origin") else {
        return true;
    };
    let origin = origin.trim_end_matches('/').to_ascii_lowercase();
    let authority = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .unwrap_or("");
    local_authority(authority, port)
}

pub fn assets_dir() -> PathBuf {
    std::env::var("GEOSIGNAL_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("public"))
}

fn static_reply(path: &str, assets: &Path) -> Reply {
    let relative = if path == "/" {
        "index.html".to_string()
    } else {
        path.trim_start_matches('/').to_string()
    };
    if relative.contains("..")
        || relative.is_empty()
        || relative
            .split('/')
            .any(|part| part.starts_with('.') || part.contains('\\'))
    {
        return error_reply("Not found", 404);
    }
    let file = assets.join(&relative);
    match fs::read(&file) {
        Ok(bytes) => Reply {
            status: 200,
            content_type: content_type_for(&relative).to_string(),
            body: bytes,
            disposition: None,
        },
        Err(_) => error_reply("Not found", 404),
    }
}

fn route(
    method: &Method,
    path: &str,
    query: &str,
    body: &str,
    lock: &Arc<Mutex<()>>,
    assets: &Path,
) -> Reply {
    if path == "/api/health" {
        return json_reply(json!({ "ok": true, "version": "0.1.0" }), 200);
    }

    if path == "/api/models" {
        let status = ai::ollama_status();
        let installed: Vec<String> = status.models.iter().map(|model| model.name.clone()).collect();
        return json_reply(
            json!({
                "online": status.online,
                "host": status.host,
                "error": status.error,
                "models": status.models.iter().map(|model| json!({
                    "name": model.name,
                    "sizeBytes": model.size_bytes,
                    "params": model.params,
                })).collect::<Vec<_>>(),
                "presets": ai::PRESETS.iter().map(|preset| json!({
                    "id": preset.id,
                    "label": preset.label,
                    "tag": preset.tag,
                    "size": preset.size,
                    "family": preset.family,
                    "notes": preset.notes,
                    "license": preset.license,
                    "embedded": engine::is_installed(preset),
                    "installed": ai::is_installed(preset.tag, &installed) || engine::is_installed(preset),
                })).collect::<Vec<_>>(),
                "engine": engine::status_json(),
            }),
            200,
        );
    }

    if path == "/api/models/custom" && *method == Method::Post {
        let parsed: models::CustomModel = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(error) => return error_reply(&format!("Invalid custom model: {error}"), 400),
        };
        return match models::add(parsed) {
            Ok(model) => json_reply(json!({ "ok": true, "model": model }), 201),
            Err(error) => error_reply(&error, 400),
        };
    }

    if let Some(id) = path.strip_prefix("/api/models/custom/") {
        if *method == Method::Delete && !id.contains('/') {
            return match models::remove(id) {
                Ok(true) => json_reply(json!({ "ok": true, "id": id }), 200),
                Ok(false) => error_reply("Custom model not found", 404),
                Err(error) => error_reply(&error, 500),
            };
        }
    }

    if path == "/api/engine" {
        return json_reply(engine::status_json(), 200);
    }

    if path == "/api/updates" && *method == Method::Get {
        return json_reply(updates::status(), 200);
    }

    if path == "/api/content" && *method == Method::Get {
        return json_reply(updates::content_json(), 200);
    }

    if path == "/api/updates/source" && (*method == Method::Put || *method == Method::Post) {
        let parsed: Value = serde_json::from_str(body).unwrap_or(json!({}));
        let source = parsed
            .get("source")
            .or_else(|| parsed.get("url"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        return match updates::set_source(source) {
            Ok(status) => json_reply(status, 200),
            Err(error) => error_reply(&error, 400),
        };
    }

    if path == "/api/updates/check" && *method == Method::Post {
        return match updates::check() {
            Ok(result) => json_reply(result, 200),
            Err(error) => error_reply(&error, 422),
        };
    }

    if path == "/api/updates/apply" && *method == Method::Post {
        let content: updates::ContentRelease = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(error) => return error_reply(&format!("Invalid content update: {error}"), 400),
        };
        return match updates::apply(content) {
            Ok(result) => json_reply(result, 200),
            Err(error) => error_reply(&error, 422),
        };
    }

    if path == "/api/updates/reset" && *method == Method::Post {
        return match updates::clear_content() {
            Ok(()) => json_reply(json!({ "ok": true }), 200),
            Err(error) => error_reply(&error, 500),
        };
    }

    if path == "/api/engine/download" && *method == Method::Post {
        let parsed: Value = serde_json::from_str(body).unwrap_or(json!({}));
        let id = parsed.get("id").and_then(|value| value.as_str()).unwrap_or_default();
        return match engine::start_download(id) {
            Ok(()) => json_reply(json!({ "ok": true, "id": id }), 202),
            Err(error) => error_reply(&error, 400),
        };
    }

    if path == "/api/standards" {
        return json_reply(
            json!({
                "categories": rules::CATEGORIES.iter().map(|(key, label, blurb)| json!({
                    "key": key, "label": label, "blurb": blurb
                })).collect::<Vec<_>>(),
                "rules": rules::RULES.iter().map(|rule| json!({
                    "id": rule.id,
                    "title": rule.title,
                    "category": rule.category,
                    "weight": rule.weight,
                    "why": rule.why,
                    "fix": rule.fix,
                    "sources": rule.sources.iter().map(|(label, url)| json!({
                        "label": label, "url": url
                    })).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            }),
            200,
        );
    }

    if path == "/api/overrides" {
        match *method {
            Method::Get => {
                let overrides = {
                    let _guard = lock.lock().unwrap();
                    store::get_overrides()
                };
                return json_reply(serde_json::to_value(overrides).unwrap_or(json!({})), 200);
            }
            Method::Put | Method::Post => {
                let parsed: Overrides = match serde_json::from_str(body) {
                    Ok(value) => value,
                    Err(error) => return error_reply(&format!("Invalid overrides: {}", error), 400),
                };
                let saved = {
                    let _guard = lock.lock().unwrap();
                    match store::save_overrides(parsed) {
                        Ok(value) => value,
                        Err(error) => return error_reply(&error, 500),
                    }
                };
                return json_reply(serde_json::to_value(saved).unwrap_or(json!({})), 200);
            }
            _ => return error_reply("Method not allowed", 405),
        }
    }

    if path == "/api/scan" && *method == Method::Post {
        let request: ScanRequest = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(error) => return error_reply(&format!("Invalid scan request: {}", error), 400),
        };
        if request.url.trim().is_empty() {
            return error_reply("Enter a website address, e.g. example.com", 400);
        }
        let Some(_audit_slot) = AuditSlot::acquire() else {
            return error_reply("Another audit is already running. Try again in a moment.", 429);
        };
        let overrides = {
            let _guard = lock.lock().unwrap();
            store::get_overrides()
        };
        let scan = match scan::scan_site(
            &request.url,
            scan::ScanOptions {
                max_pages: request.max_pages,
                keyword: request.keyword,
                city: request.city,
            },
        ) {
            Ok(value) => value,
            Err(error) => return error_reply(&error, 422),
        };
        let report = rules::build_report(&scan, &overrides);
        {
            let _guard = lock.lock().unwrap();
            let _ = store::save_scan(&scan, &report);
        }
        println!(
            "[scan] {} — {} pages, score {}",
            scan.host,
            scan.pages.len(),
            report.score
        );
        return json_reply(json!({ "scan": scan, "report": report }), 200);
    }

    if path == "/api/scans" && *method == Method::Get {
        let scans = {
            let _guard = lock.lock().unwrap();
            store::list_scans()
        };
        return json_reply(json!({ "scans": scans }), 200);
    }

    if let Some(rest) = path.strip_prefix("/api/scan/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() == 2 && parts[1] == "report" && *method == Method::Get {
            let stored = {
                let _guard = lock.lock().unwrap();
                store::get_scan(parts[0])
            };
            let Some(stored) = stored else {
                return error_reply("Scan not found", 404);
            };
            let overrides = {
                let _guard = lock.lock().unwrap();
                store::get_overrides()
            };
            let report = rules::build_report(&stored.scan, &overrides);
            let compare = query_param(query, "compareId")
                .and_then(|id| {
                    let _guard = lock.lock().unwrap();
                    store::get_compare(&id)
                });
            let format = query_param(query, "format").unwrap_or_else(|| "html".to_string());
            let filename = format!(
                "local-seo-{}-{}",
                stored
                    .scan
                    .host
                    .chars()
                    .map(|ch| if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' { ch } else { '_' })
                    .collect::<String>(),
                stored.scan.id
            );
            if format == "md" {
                return Reply {
                    status: 200,
                    content_type: "text/markdown; charset=utf-8".to_string(),
                    body: report::build_markdown(&stored.scan, &report, compare.as_ref()).into_bytes(),
                    disposition: Some(format!("attachment; filename=\"{}.md\"", filename)),
                };
            }
            return Reply {
                status: 200,
                content_type: "text/html; charset=utf-8".to_string(),
                body: report::build_html_report(&stored.scan, &report, compare.as_ref()).into_bytes(),
                disposition: Some(format!("inline; filename=\"{}.html\"", filename)),
            };
        }
        if parts.len() == 1 {
            if *method == Method::Delete {
                let deleted = {
                    let _guard = lock.lock().unwrap();
                    store::delete_scan(parts[0])
                };
                return if deleted {
                    json_reply(json!({ "ok": true }), 200)
                } else {
                    error_reply("Scan not found", 404)
                };
            }
            if *method == Method::Get {
                let stored = {
                    let _guard = lock.lock().unwrap();
                    store::get_scan(parts[0])
                };
                let Some(stored) = stored else {
                    return error_reply("Scan not found", 404);
                };
                let overrides = {
                    let _guard = lock.lock().unwrap();
                    store::get_overrides()
                };
                let report = rules::build_report(&stored.scan, &overrides);
                return json_reply(json!({ "scan": stored.scan, "report": report }), 200);
            }
        }
    }

    if path == "/api/ai/compare" && *method == Method::Post {
        let request: CompareRequest = match serde_json::from_str(body) {
            Ok(value) => value,
            Err(error) => return error_reply(&format!("Invalid compare request: {}", error), 400),
        };
        let stored = {
            let _guard = lock.lock().unwrap();
            store::get_scan(&request.scan_id)
        };
        let Some(stored) = stored else {
            return error_reply("Scan not found. Run a scan first.", 404);
        };
        let Some(_audit_slot) = AuditSlot::acquire() else {
            return error_reply("Another audit is already running. Try again in a moment.", 429);
        };
        let overrides = {
            let _guard = lock.lock().unwrap();
            store::get_overrides()
        };
        let report = rules::build_report(&stored.scan, &overrides);
        let mode = if request.mode == "sequential" { "sequential" } else { "parallel" };
        let mut compare = match ai::compare_models(
            &stored.scan,
            &report,
            &request.models,
            mode,
            &overrides,
        ) {
            Ok(value) => value,
            Err(error) => return error_reply(&error, 400),
        };
        {
            let _guard = lock.lock().unwrap();
            let _ = store::save_compare(&compare);
        }
        let leaderboard = {
            let _guard = lock.lock().unwrap();
            store::get_leaderboard()
        };
        compare.leaderboard = leaderboard.clone();
        println!(
            "[ai] {} — {} ({})",
            stored.scan.host,
            request.models.join(", "),
            mode
        );
        return json_reply(
            json!({ "compare": compare, "leaderboard": leaderboard }),
            200,
        );
    }

    if path == "/api/compares" && *method == Method::Get {
        let compares = {
            let _guard = lock.lock().unwrap();
            store::list_compares()
        };
        let trimmed: Vec<_> = compares.into_iter().take(40).collect();
        return json_reply(json!({ "compares": trimmed }), 200);
    }

    if let Some(id) = path.strip_prefix("/api/compare/") {
        if *method == Method::Get {
            let (compare, leaderboard) = {
                let _guard = lock.lock().unwrap();
                (store::get_compare(id), store::get_leaderboard())
            };
            return match compare {
                Some(value) => json_reply(json!({ "compare": value, "leaderboard": leaderboard }), 200),
                None => error_reply("Comparison not found", 404),
            };
        }
    }

    if path == "/api/leaderboard" && *method == Method::Get {
        let leaderboard = {
            let _guard = lock.lock().unwrap();
            store::get_leaderboard()
        };
        return json_reply(json!({ "leaderboard": leaderboard }), 200);
    }

    if path.starts_with("/api/") {
        return error_reply("Not found", 404);
    }

    static_reply(path, assets)
}

fn respond(request: tiny_http::Request, reply: Reply) {
    let mut response = Response::from_data(reply.body).with_status_code(reply.status);
    if let Ok(header) = Header::from_bytes(&b"content-type"[..], reply.content_type.as_bytes()) {
        response = response.with_header(header);
    }
    if let Some(disposition) = reply.disposition {
        if let Ok(header) = Header::from_bytes(&b"content-disposition"[..], disposition.as_bytes()) {
            response = response.with_header(header);
        }
    }
    if let Ok(header) = Header::from_bytes(&b"cache-control"[..], &b"no-store"[..]) {
        response = response.with_header(header);
    }
    for (name, value) in SECURITY_HEADERS {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response = response.with_header(header);
        }
    }
    let _ = request.respond(response);
}

fn handle(
    mut request: tiny_http::Request,
    lock: Arc<Mutex<()>>,
    assets: Arc<PathBuf>,
    port: u16,
    public: bool,
) {
    let method = request.method().clone();
    let host = header_value(request.headers(), "host").unwrap_or("");
    let host_ok = local_authority(host, port) || (public && public_authority(host));
    if !host_ok {
        respond(request, error_reply("Forbidden host", 403));
        return;
    }
    let changes_state = matches!(&method, Method::Post | Method::Put | Method::Delete);
    if changes_state && !(local_origin(request.headers(), port) || (public && public_origin(request.headers(), host))) {
        respond(request, error_reply("Forbidden origin", 403));
        return;
    }

    let raw_url = request.url().to_string();
    let (path, query) = match raw_url.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (raw_url.clone(), String::new()),
    };

    let mut body = String::new();
    if method == Method::Post || method == Method::Put {
        let too_large = header_value(request.headers(), "content-length")
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|length| length > MAX_BODY_BYTES);
        if too_large {
            respond(request, error_reply("Request body too large", 413));
            return;
        }
        let mut reader = request.as_reader().take(MAX_BODY_BYTES);
        let _ = reader.read_to_string(&mut body);
    }

    let reply = route(&method, &path, &query, &body, &lock, assets.as_ref());
    respond(request, reply);
}

pub struct ServerHandle {
    port: u16,
    server: Arc<Server>,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl ServerHandle {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn stop(&mut self) {
        if self.join.is_none() {
            return;
        }
        self.stop.store(true, Ordering::SeqCst);
        self.server.unblock();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }

    pub fn wait(&mut self) {
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn start(port: u16) -> Result<ServerHandle, String> {
    // Loopback is the default. Public hosting must be explicit: it means the
    // unauthenticated app is reachable by anyone and must be treated as such.
    let public = crate::util::public_mode();
    let bind = std::env::var("GEOSIGNAL_BIND").unwrap_or_else(|_| "127.0.0.1".to_string());
    if !public && !loopback_bind(&bind) {
        return Err("Refusing to bind outside loopback. Set GEOSIGNAL_PUBLIC=1 only for an intentional public deployment.".to_string());
    }
    let server = Server::http((bind.as_str(), port))
        .map_err(|error| format!("could not bind {bind}:{port} - {error}"))?;
    let bound_port = server
        .server_addr()
        .to_string()
        .rsplit(':')
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(port);
    let server = Arc::new(server);
    let stop = Arc::new(AtomicBool::new(false));
    let assets = Arc::new(assets_dir());
    let lock = Arc::new(Mutex::new(()));

    let listener = Arc::clone(&server);
    let stop_flag = Arc::clone(&stop);
    let join = std::thread::spawn(move || {
        for request in listener.incoming_requests() {
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }
            let lock = Arc::clone(&lock);
            let assets = Arc::clone(&assets);
            std::thread::spawn(move || handle(request, lock, assets, bound_port, public));
        }
    });

    Ok(ServerHandle {
        port: bound_port,
        server,
        stop,
        join: Some(join),
    })
}

pub fn run_cli() -> Result<(), String> {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|value| *value != 0)
        .unwrap_or(4173);
    let mut handle = start(port)?;

    let public = crate::util::public_mode();
    let bind = std::env::var("GEOSIGNAL_BIND").unwrap_or_else(|_| "127.0.0.1".to_string());
    println!("\n  GeoSignal — local SEO auditor (Rust)");
    println!("  http://{bind}:{}", handle.port());
    if public {
        println!("  Public mode is ON: the unauthenticated app is reachable from the network.");
    }
    println!(
        "  AI models: built-in llama.cpp · weights in {} (Ollama optional at {})\n",
        engine::models_dir().display(),
        std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://127.0.0.1:11434".to_string())
    );

    handle.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(name: &'static str, value: &str) -> Header {
        Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap()
    }

    #[test]
    fn accepts_only_loopback_authorities() {
        assert!(local_authority("127.0.0.1:4173", 4173));
        assert!(local_authority("localhost:4173", 4173));
        assert!(local_authority("[::1]:4173", 4173));
        assert!(!local_authority("example.com:4173", 4173));
        assert!(!local_authority("127.0.0.1:9999", 4173));
    }

    #[test]
    fn public_mode_requires_matching_origin() {
        assert!(public_origin(&[], "demo.hf.space"));
        assert!(public_origin(
            &[header("origin", "https://demo.hf.space")],
            "demo.hf.space"
        ));
        assert!(!public_origin(
            &[header("origin", "https://evil.example")],
            "demo.hf.space"
        ));
    }

    #[test]
    fn public_origin_checks_scheme_host_and_port() {
        assert!(public_origin(
            &[header("origin", "https://demo.hf.space:443")],
            "demo.hf.space"
        ));
        assert!(public_origin(
            &[header("origin", "http://localhost:7860")],
            "localhost:7860"
        ));
        assert!(!public_origin(
            &[header("origin", "https://demo.hf.space:8443")],
            "demo.hf.space"
        ));
        assert!(!public_origin(
            &[header("origin", "ftp://demo.hf.space")],
            "demo.hf.space"
        ));
        assert!(!public_origin(&[header("origin", "null")], "demo.hf.space"));
    }

    #[test]
    fn refuses_non_loopback_binds_without_public_mode() {
        assert!(loopback_bind("127.0.0.1"));
        assert!(loopback_bind("localhost"));
        assert!(!loopback_bind("0.0.0.0"));
    }

    #[test]
    fn accepts_only_local_or_missing_origins() {
        assert!(local_origin(&[], 4173));
        assert!(local_origin(
            &[header("origin", "http://127.0.0.1:4173")],
            4173
        ));
        assert!(local_origin(
            &[header("origin", "http://localhost:4173")],
            4173
        ));
        assert!(!local_origin(
            &[header("origin", "https://example.com")],
            4173
        ));
        assert!(!local_origin(&[header("origin", "null")], 4173));
    }
}
