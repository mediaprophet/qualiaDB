//! Webizen Browser shell: own chrome (P0.1), trust store surface (P1), agent (P2).
//!
//! Architecture:
//! - Window `webizen-browser` hosts two child webviews (Tauri `unstable` multi-webview):
//!   - `webizen-browser-chrome` — toolbar / trust badge / agent drawer (our HTML)
//!   - `webizen-browser-content` — top-level page navigation (real sites load)
//! - Trust policy + agent logic: `qualia_client_core::{webizen_trust, browser_agent}`
//!
//! Layout: chrome is fixed-height at the top; content fills the remainder. On
//! window resize we re-position both children. Content URL → chrome omnibox is
//! polled via `browser_content_url` (≤1s) because in-page link navigation does
//! not always surface a host event on every platform.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::webview::WebviewBuilder;
use tauri::window::WindowBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl};

pub const WINDOW_LABEL: &str = "webizen-browser";
pub const CHROME_LABEL: &str = "webizen-browser-chrome";
pub const CONTENT_LABEL: &str = "webizen-browser-content";
pub const CHROME_H: f64 = 52.0;

static LAST_URL: Mutex<String> = Mutex::new(String::new());

const CHROME_HTML: &str = include_str!("chrome.html");

fn set_last_url(url: &str) {
    if let Ok(mut g) = LAST_URL.lock() {
        *g = url.to_string();
    }
}

pub fn last_url() -> String {
    LAST_URL.lock().map(|g| g.clone()).unwrap_or_default()
}

/// Ensure chrome HTML is available via App URL (frontendDist).
pub fn ensure_chrome_asset() -> Result<PathBuf, String> {
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../webizen-studio/dist");
    std::fs::create_dir_all(&dist).map_err(|e| e.to_string())?;
    let path = dist.join("browser-chrome.html");
    std::fs::write(&path, CHROME_HTML).map_err(|e| e.to_string())?;
    Ok(path)
}

fn storage_root() -> PathBuf {
    PathBuf::from(qualia_client_core::state::dirs_default_path())
}

fn logical_inner(window: &tauri::Window) -> Result<(f64, f64), String> {
    let scale = window.scale_factor().unwrap_or(1.0);
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let width = (size.width as f64 / scale).max(320.0);
    let height = (size.height as f64 / scale).max(200.0);
    Ok((width, height))
}

/// Re-layout chrome (top strip) and content (remainder) for the current window size.
pub fn relayout_browser(app: &AppHandle) -> Result<(), String> {
    let Some(window) = app.get_window(WINDOW_LABEL) else {
        return Ok(());
    };
    let (width, height) = logical_inner(&window)?;
    let content_h = (height - CHROME_H).max(120.0);

    if let Some(chrome) = app.get_webview(CHROME_LABEL) {
        let _ = chrome.set_position(LogicalPosition::new(0.0, 0.0));
        let _ = chrome.set_size(LogicalSize::new(width, CHROME_H));
    }
    if let Some(content) = app.get_webview(CONTENT_LABEL) {
        let _ = content.set_position(LogicalPosition::new(0.0, CHROME_H));
        let _ = content.set_size(LogicalSize::new(width, content_h));
    }
    Ok(())
}

fn attach_resize_handler(window: &tauri::Window, app: AppHandle) {
    let win_label = WINDOW_LABEL.to_string();
    window.on_window_event(move |event| {
        use tauri::WindowEvent;
        if matches!(
            event,
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
        ) {
            // Only relayout our browser window (handler is attached only there).
            let _ = win_label;
            let _ = relayout_browser(&app);
        }
    });
}

/// Create or focus the browser window with chrome + content child webviews.
pub fn open_browser_shell(app: &AppHandle, start_url: &str) -> Result<String, String> {
    let _ = ensure_chrome_asset()?;
    let start = start_url.trim();
    let start = if start.is_empty() {
        "https://duckduckgo.com/"
    } else {
        start
    };
    set_last_url(start);

    if app.get_window(WINDOW_LABEL).is_some() {
        // Ensure children still exist (user may have closed a webview on some platforms).
        if app.get_webview(CONTENT_LABEL).is_none() || app.get_webview(CHROME_LABEL).is_none() {
            // Fail closed: destroy shell and rebuild.
            if let Some(w) = app.get_window(WINDOW_LABEL) {
                let _ = w.close();
            }
        } else {
            navigate_content(app, start)?;
            let _ = relayout_browser(app);
            focus_window(app)?;
            return Ok(start.to_string());
        }
    }

    let chrome_path = format!(
        "browser-chrome.html?url={}",
        urlencoding::encode(start)
    );
    let start_parsed: tauri::Url = start
        .parse()
        .map_err(|e| format!("Invalid start URL '{start}': {e}"))?;

    let window = WindowBuilder::new(app, WINDOW_LABEL)
        .title("Webizen Browser")
        .inner_size(1280.0, 900.0)
        .center()
        .build()
        .map_err(|e| format!("browser window: {e}"))?;

    let (width, height) = logical_inner(&window)?;
    let content_h = (height - CHROME_H).max(120.0);

    window
        .add_child(
            WebviewBuilder::new(CHROME_LABEL, WebviewUrl::App(chrome_path.into())),
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(width, CHROME_H),
        )
        .map_err(|e| {
            let _ = window.close();
            format!(
                "chrome webview failed: {e}. Fallback: use Reach pane chrome + browser_navigate only, or reopen after rebuild."
            )
        })?;

    window
        .add_child(
            WebviewBuilder::new(CONTENT_LABEL, WebviewUrl::External(start_parsed)),
            LogicalPosition::new(0.0, CHROME_H),
            LogicalSize::new(width, content_h),
        )
        .map_err(|e| {
            let _ = window.close();
            format!(
                "content webview failed: {e}. Fallback: single-window navigation via open_web_url / Reach Focus."
            )
        })?;

    attach_resize_handler(&window, app.clone());
    let _ = window.set_focus();
    Ok(start.to_string())
}

/// Navigate only the content webview (chrome stays put).
pub fn navigate_content(app: &AppHandle, url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty URL".into());
    }
    let parsed: tauri::Url = url
        .parse()
        .map_err(|e| format!("Invalid URL '{url}': {e}"))?;
    set_last_url(url);

    if let Some(w) = app.get_webview(CONTENT_LABEL) {
        w.navigate(parsed).map_err(|e| e.to_string())?;
        return Ok(());
    }
    // Shell not open yet.
    open_browser_shell(app, url).map(|_| ())
}

pub fn reload_content(app: &AppHandle) -> Result<(), String> {
    let last = last_url();
    if !last.is_empty() {
        return navigate_content(app, &last);
    }
    if let Some(w) = app.get_webview(CONTENT_LABEL) {
        let _ = w.eval("window.location.reload()");
        return Ok(());
    }
    Err("browser content not open".into())
}

pub fn content_history_back(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview(CONTENT_LABEL) {
        w.eval("window.history.back()").map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("browser not open".into())
}

pub fn content_history_forward(app: &AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview(CONTENT_LABEL) {
        w.eval("window.history.forward()")
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    Err("browser not open".into())
}

/// Best-effort content URL for chrome omnibox sync (poll ≤1s from chrome.html).
pub fn content_url(app: &AppHandle) -> String {
    // Prefer last navigated URL we set; in-page SPA navigations may lag until
    // platform navigation hooks land (honest: poll is the documented path).
    let last = last_url();
    if !last.is_empty() {
        return last;
    }
    if app.get_webview(CONTENT_LABEL).is_some() {
        "https://duckduckgo.com/".into()
    } else {
        String::new()
    }
}

pub fn focus_window(app: &AppHandle) -> Result<bool, String> {
    if let Some(w) = app.get_window(WINDOW_LABEL) {
        let _ = w.unminimize();
        w.set_focus().map_err(|e| e.to_string())?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn status(app: &AppHandle) -> serde_json::Value {
    serde_json::json!({
        "open": app.get_window(WINDOW_LABEL).is_some(),
        "chrome_open": app.get_webview(CHROME_LABEL).is_some(),
        "content_open": app.get_webview(CONTENT_LABEL).is_some(),
        "label": WINDOW_LABEL,
        "content_label": CONTENT_LABEL,
        "last_url": last_url(),
        "chrome": "in-window multi-webview",
        "substrate": "os-webview",
        "phases": ["P0", "P0.1", "P1-store", "P2-agent"],
        "url_sync": "poll browser_content_url ≤1s",
        "note": "TLS for content webview still uses the OS store; custom PEM roots apply to agent HTTPS fetch. Platform cert-override is the next hook.",
    })
}

// ── Trust store ──────────────────────────────────────────────────────────────

pub fn trust_list() -> Result<serde_json::Value, String> {
    let s = qualia_client_core::webizen_trust::TrustStore::load(&storage_root());
    serde_json::to_value(&s).map_err(|e| e.to_string())
}

pub fn trust_add_pem(label: String, pem: String, notes: String) -> Result<serde_json::Value, String> {
    let root = storage_root();
    let mut s = qualia_client_core::webizen_trust::TrustStore::load(&root);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let a = s.add_pem_root(&label, &pem, &notes, now)?;
    s.save(&root)?;
    serde_json::to_value(a).map_err(|e| e.to_string())
}

pub fn trust_add_did(label: String, did: String, notes: String) -> Result<serde_json::Value, String> {
    let root = storage_root();
    let mut s = qualia_client_core::webizen_trust::TrustStore::load(&root);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let a = s.add_did(&label, &did, &notes, now)?;
    s.save(&root)?;
    serde_json::to_value(a).map_err(|e| e.to_string())
}

pub fn trust_set_enabled(id: String, enabled: bool) -> Result<(), String> {
    let root = storage_root();
    let mut s = qualia_client_core::webizen_trust::TrustStore::load(&root);
    s.set_enabled(&id, enabled)?;
    s.save(&root)
}

pub fn trust_remove(id: String) -> Result<bool, String> {
    let root = storage_root();
    let mut s = qualia_client_core::webizen_trust::TrustStore::load(&root);
    let ok = s.remove(&id);
    s.save(&root)?;
    Ok(ok)
}

pub fn trust_verdict(url: String) -> Result<serde_json::Value, String> {
    let s = qualia_client_core::webizen_trust::TrustStore::load(&storage_root());
    let v = qualia_client_core::webizen_trust::evaluate_url(&s, &url);
    serde_json::to_value(v).map_err(|e| e.to_string())
}

pub async fn agent_ask(
    url: String,
    question: String,
    ingest_to_library: bool,
) -> Result<serde_json::Value, String> {
    let req = qualia_client_core::browser_agent::BrowserAgentRequest {
        url,
        question,
        ingest_to_library,
    };
    let resp =
        qualia_client_core::browser_agent::run_browser_agent(&storage_root(), req).await?;
    serde_json::to_value(resp).map_err(|e| e.to_string())
}
