//! Native macOS desktop shell for GeoSignal.
//!
//! The existing HTTP app is started in-process and shown in a WKWebView. The
//! page gets a runtime panel that can stop and restart that server through the
//! webview IPC bridge, so hosting (and model downloads) can be controlled from
//! inside the app instead of a terminal.

use geosignal::server::{self, ServerHandle};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Runs inside the real WKWebView: opens the runtime panel, clicks Stop and
/// Start, and reports each state through the desktop IPC bridge.
const UI_SMOKE_JS: &str = r#"
(async () => {
  const wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const snap = () => ({
    state: document.querySelector('[data-runtime-state]')?.textContent ?? '',
    chip: document.querySelector('#runtime-chip')?.textContent.trim() ?? '',
    chipClass: document.querySelector('#runtime-chip')?.className ?? '',
    statusClass: document.querySelector('.runtime-status')?.className ?? '',
    startDisabled: Boolean(document.querySelector('[data-runtime="start"]')?.disabled),
    stopDisabled: Boolean(document.querySelector('[data-runtime="stop"]')?.disabled),
  });
  const health = async () => {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 2000);
    try {
      const response = await fetch('/api/health', { cache: 'no-store', signal: controller.signal });
      return 'http-' + response.status;
    } catch (error) {
      return 'error-' + String(error && error.name ? error.name : error);
    } finally {
      clearTimeout(timer);
    }
  };
  const report = (phase, extra = {}) => {
    window.ipc.postMessage(JSON.stringify({ cmd: 'ui-report', phase, ...extra }));
  };
  try {
    const chip = document.querySelector('#runtime-chip');
    if (!chip) throw new Error('runtime chip missing');
    chip.click();
    await wait(500);
    report('opened', snap());
    const stop = document.querySelector('[data-runtime="stop"]');
    if (!stop || stop.disabled) throw new Error('stop button unavailable');
    stop.click();
    let stopped = snap();
    for (let i = 0; i < 40 && !stopped.state.includes('stopped'); i += 1) {
      await wait(200);
      stopped = snap();
    }
    report('stopped', { ...stopped, health: await health() });
    const start = document.querySelector('[data-runtime="start"]');
    if (!start || start.disabled) throw new Error('start button unavailable');
    start.click();
    let started = snap();
    for (let i = 0; i < 40 && !started.state.includes('running'); i += 1) {
      await wait(200);
      started = snap();
    }

    let guideButton = null;
    for (let i = 0; i < 20 && !guideButton; i += 1) {
      await wait(150);
      guideButton = document.querySelector('#standards-root [data-guide]');
    }
    if (!guideButton) throw new Error('fix guide button missing');
    guideButton.click();
    await wait(300);
    const guide = {
      open: !document.querySelector('#help-modal').hidden,
      title: document.querySelector('#help-title').textContent,
      steps: document.querySelector('#help-body ol')?.children.length ?? 0,
    };
    document.querySelector('[data-help-close]')?.click();
    report('guide', guide);

    let sourceLink = null;
    for (let i = 0; i < 20 && !sourceLink; i += 1) {
      await wait(150);
      sourceLink = document.querySelector('#standards-root a[href^="http"]');
    }
    if (!sourceLink) throw new Error('source link missing');
    sourceLink.click();
    await wait(300);
    report('link', { url: sourceLink.href });

    report('done', { ...started, health: await health() });
  } catch (error) {
    report('error', { message: String(error && error.message ? error.message : error) });
  }
})();
"#;

fn data_dir() -> PathBuf {
    if let Ok(value) = std::env::var("GEOSIGNAL_DATA") {
        return PathBuf::from(value);
    }
    if let Ok(cwd) = std::env::current_dir() {
        let local = cwd.join("data");
        if local.is_dir() {
            return local;
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join("Library")
        .join("Application Support")
        .join("GeoSignal")
        .join("data")
}

fn copy_dir(source: &Path, target: &Path) {
    let Ok(entries) = std::fs::read_dir(source) else {
        return;
    };
    let _ = std::fs::create_dir_all(target);
    for entry in entries.flatten() {
        let from = entry.path();
        let to = target.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to);
        } else {
            let _ = std::fs::copy(&from, &to);
        }
    }
}

fn seed_data(target: &Path) {
    if target.join("models").is_dir() || target.join("scans").is_dir() {
        return;
    }
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    if source.is_dir() && source != target {
        println!("[desktop] seeding {} from {}", target.display(), source.display());
        copy_dir(&source, target);
    }
}

fn assets_dir() -> PathBuf {
    if let Ok(value) = std::env::var("GEOSIGNAL_ASSETS") {
        return PathBuf::from(value);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(contents) = exe.parent().and_then(Path::parent) {
            let bundled = contents.join("Resources").join("public");
            if bundled.is_dir() {
                return bundled;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("public")
}

fn is_external_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    ["http://", "https://", "mailto:", "tel:"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
}

fn is_up(port: u16) -> bool {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_millis(700))
        .build();
    agent
        .get(&format!("http://127.0.0.1:{port}/api/health"))
        .call()
        .map(|response| response.status() == 200)
        .unwrap_or(false)
}

fn wait_for_server(port: u16) -> bool {
    for _ in 0..50 {
        if is_up(port) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

fn start_server(state: &Arc<Mutex<Option<ServerHandle>>>, port: u16) -> Result<(), String> {
    let mut guard = state.lock().map_err(|_| "runtime lock poisoned".to_string())?;
    if guard.is_some() {
        return Ok(());
    }
    let handle = server::start(port)?;
    *guard = Some(handle);
    Ok(())
}

fn stop_server(state: &Arc<Mutex<Option<ServerHandle>>>) {
    if let Ok(mut guard) = state.lock() {
        if let Some(mut handle) = guard.take() {
            handle.stop();
        }
    }
}

fn smoke() -> Result<(), String> {
    println!("[smoke] starting server on an ephemeral port");
    let mut handle = server::start(0)?;
    let port = handle.port();
    println!("[smoke] port {port}");
    if !wait_for_server(port) {
        return Err("server did not answer /api/health".to_string());
    }
    println!("[smoke] health ok");
    handle.stop();
    drop(handle);
    if is_up(port) {
        return Err("server still answering after stop".to_string());
    }
    println!("[smoke] stopped cleanly");
    let mut restarted = server::start(port).map_err(|error| format!("restart failed: {error}"))?;
    if !wait_for_server(port) {
        return Err("restarted server did not answer".to_string());
    }
    println!("[smoke] restart ok on the same port");
    restarted.stop();
    drop(restarted);
    println!("[smoke] PASS");
    Ok(())
}

fn main() {
    let ui_smoke = std::env::args().any(|arg| arg == "--ui-smoke");

    if std::env::args().any(|arg| arg == "--smoke") {
        if let Err(error) = smoke() {
            eprintln!("[smoke] FAIL: {error}");
            std::process::exit(1);
        }
        return;
    }

    let data = data_dir();
    std::env::set_var("GEOSIGNAL_DATA", &data);
    std::env::set_var("GEOSIGNAL_ASSETS", assets_dir());
    if let Err(error) = std::fs::create_dir_all(&data) {
        eprintln!("[desktop] could not create {}: {error}", data.display());
    }
    seed_data(&data);

    let initial = server::start(0).expect("GeoSignal server could not start");
    let port = initial.port();
    if !wait_for_server(port) {
        eprintln!("[desktop] server did not become ready");
        std::process::exit(1);
    }
    let runtime = Arc::new(Mutex::new(Some(initial)));
    let url = format!("http://127.0.0.1:{port}/");

    println!("\n  GeoSignal desktop {VERSION}");
    println!("  {url}");
    println!("  data: {}\n", data.display());

    let ipc_state = Arc::clone(&runtime);
    let link_ok = Arc::new(AtomicBool::new(false));
    let link_flag = Arc::clone(&link_ok);
    let ipc_url = url.clone();
    let ipc_data = data.clone();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("GeoSignal")
        .with_inner_size(LogicalSize::new(1280.0, 900.0))
        .with_min_inner_size(LogicalSize::new(960.0, 700.0))
        .build(&event_loop)
        .expect("window could not be created");
    window.set_focus();

    let init_script = format!(
        "window.__GEOSIGNAL_DESKTOP__ = {};",
        serde_json::json!({
            "port": port,
            "dataDir": data.display().to_string(),
            "version": VERSION,
        })
    );

    let webview = WebViewBuilder::new()
        .with_url(&url)
        .with_initialization_script(init_script)
        .with_ipc_handler(move |request| {
            let payload: Value = serde_json::from_str(request.body()).unwrap_or(Value::Null);
            match payload.get("cmd").and_then(|value| value.as_str()).unwrap_or("") {
                "start" => {
                    println!("[desktop] start requested");
                    let port = ipc_url
                        .trim_start_matches("http://127.0.0.1:")
                        .trim_end_matches('/')
                        .parse::<u16>()
                        .unwrap_or(port);
                    if let Err(error) = start_server(&ipc_state, port) {
                        eprintln!("[desktop] start failed: {error}");
                    }
                }
                "stop" => stop_server(&ipc_state),
                "open-browser" => {
                    let _ = std::process::Command::new("open").arg(&ipc_url).spawn();
                }
                "reveal-data" => {
                    let _ = std::fs::create_dir_all(&ipc_data);
                    let _ = std::process::Command::new("open").arg(&ipc_data).spawn();
                }
                "open-url" => {
                    let url = payload.get("url").and_then(Value::as_str).unwrap_or("");
                    if is_external_url(url) {
                        if ui_smoke {
                            link_flag.store(true, Ordering::SeqCst);
                            println!("[ui-smoke] link command received: {url}");
                        } else {
                            let _ = std::process::Command::new("open").arg(url).spawn();
                        }
                    } else {
                        eprintln!("[desktop] blocked unsupported link: {url}");
                    }
                }
                "quit" => {
                    stop_server(&ipc_state);
                    std::process::exit(0);
                }
                "ui-report" => {
                    let phase = payload.get("phase").and_then(Value::as_str).unwrap_or("");
                    let state = payload.get("state").and_then(Value::as_str).unwrap_or("");
                    let chip = payload.get("chip").and_then(Value::as_str).unwrap_or("");
                    let status_class = payload.get("statusClass").and_then(Value::as_str).unwrap_or("");
                    let health = payload.get("health").and_then(Value::as_str).unwrap_or("");
                    println!(
                        "[ui-smoke] {phase}: state={state:?} chip={chip:?} class={status_class:?} page_health={health:?}"
                    );
                    if phase == "error" {
                        let message = payload.get("message").and_then(Value::as_str).unwrap_or("unknown");
                        eprintln!("[ui-smoke] FAIL: {message}");
                        std::process::exit(1);
                    }
                    if phase == "opened" && !state.contains("running") {
                        eprintln!("[ui-smoke] FAIL: panel did not show a running server");
                        std::process::exit(1);
                    }
                    if phase == "stopped" {
                        let running = ipc_state.lock().map(|guard| guard.is_some()).unwrap_or(true);
                        if running || is_up(port) || !state.contains("stopped") {
                            eprintln!(
                                "[ui-smoke] FAIL: Stop check running={running} port_up={} state={state:?}",
                                is_up(port)
                            );
                            std::process::exit(1);
                        }
                        println!("[ui-smoke] Stop verified: server offline, UI red");
                    }
                    if phase == "guide" {
                        let open = payload.get("open").and_then(Value::as_bool).unwrap_or(false);
                        let steps = payload.get("steps").and_then(Value::as_u64).unwrap_or(0);
                        if !open || steps == 0 {
                            eprintln!("[ui-smoke] FAIL: fix guide did not open with steps");
                            std::process::exit(1);
                        }
                        println!("[ui-smoke] guide verified: {steps} steps");
                    }
                    if phase == "link" {
                        let url = payload.get("url").and_then(Value::as_str).unwrap_or("");
                        if !url.starts_with("http") {
                            eprintln!("[ui-smoke] FAIL: source link was not an http URL");
                            std::process::exit(1);
                        }
                    }
                    if phase == "done" {
                        let running = ipc_state.lock().map(|guard| guard.is_some()).unwrap_or(false);
                        if !link_ok.load(Ordering::SeqCst) {
                            eprintln!("[ui-smoke] FAIL: source link was not routed to the desktop handler");
                            std::process::exit(1);
                        }
                        if !running || !is_up(port) || !state.contains("running") {
                            eprintln!(
                                "[ui-smoke] FAIL: Start check running={running} port_up={} state={state:?}",
                                is_up(port)
                            );
                            std::process::exit(1);
                        }
                        println!("[ui-smoke] Start verified: health ok on {port}, UI green");
                        println!("[ui-smoke] PASS");
                        stop_server(&ipc_state);
                        std::process::exit(0);
                    }
                }
                other => eprintln!("[desktop] unknown command: {other}"),
            }
        })
        .build(&window)
        .expect("webview could not be created");

    let ui_deadline = Instant::now() + Duration::from_millis(2200);
    let mut ui_ran = false;
    if ui_smoke {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_secs(35));
            eprintln!("[ui-smoke] FAIL: timed out waiting for the window controls");
            std::process::exit(1);
        });
    }

    let close_state = Arc::clone(&runtime);
    event_loop.run(move |event, _, control_flow| {
        *control_flow = if ui_smoke && !ui_ran {
            ControlFlow::WaitUntil(ui_deadline)
        } else {
            ControlFlow::Wait
        };
        if ui_smoke && !ui_ran && Instant::now() >= ui_deadline {
            ui_ran = true;
            println!("[ui-smoke] clicking the runtime controls in the real window");
            if let Err(error) = webview.evaluate_script(UI_SMOKE_JS) {
                eprintln!("[ui-smoke] FAIL: could not run page script: {error}");
                std::process::exit(1);
            }
        }
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                stop_server(&close_state);
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
