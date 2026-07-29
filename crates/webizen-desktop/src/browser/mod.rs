//! Webizen Browser shell: own chrome (P0.1), trust store surface (P1), agent (P2).
//!
//! Architecture:
//! - Window `webizen-browser` hosts two child webviews (Tauri `unstable` multi-webview):
//!   - `webizen-browser-chrome` — toolbar / trust badge / agent drawer (our HTML)
//!   - `webizen-browser-content` — top-level page navigation (real sites load)
//! - **Default home** is the Chora-generated universe view (`chora-universe.html`), not DuckDuckGo.
//! - Trust policy + agent logic: `qualia_client_core::{webizen_trust, browser_agent}`
//! - Engine preference (S1/S2): [`engine`] — default OS WebView; Servo is experimental preference only

pub mod cert_override;
pub mod cookies;
pub mod engine;

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::webview::WebviewBuilder;
use tauri::window::WindowBuilder;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl};

pub const WINDOW_LABEL: &str = "webizen-browser";
pub const CHROME_LABEL: &str = "webizen-browser-chrome";
pub const CONTENT_LABEL: &str = "webizen-browser-content";
pub const CHROME_H: f64 = 52.0;

/// Canonical home — Chora universe (App asset), not an external search engine.
pub const DEFAULT_HOME: &str = "qualia://chora/universe";
pub const UNIVERSE_ASSET: &str = "chora-universe.html";

static LAST_URL: Mutex<String> = Mutex::new(String::new());

const CHROME_HTML: &str = include_str!("chrome.html");
const UNIVERSE_HTML: &str = include_str!("universe.html");

fn set_last_url(url: &str) {
    if let Ok(mut g) = LAST_URL.lock() {
        *g = url.to_string();
    }
}

pub fn last_url() -> String {
    LAST_URL.lock().map(|g| g.clone()).unwrap_or_default()
}

/// True when the navigation target is the Chora universe home (local App page).
pub fn is_chora_universe_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.is_empty()
        || u == "about:home"
        || u == "about:blank"
        || u == DEFAULT_HOME
        || u == "qualia://chora"
        || u == "qualia://home"
        || u == "webizen://chora"
        || u == "webizen://chora/universe"
        || u == UNIVERSE_ASSET
        || u.ends_with("/chora-universe.html")
        || u.contains("chora-universe.html")
}

/// Normalize empty / home aliases to the canonical Chora universe URL.
pub fn resolve_start_url(url: &str) -> String {
    let t = url.trim();
    if is_chora_universe_url(t) {
        DEFAULT_HOME.into()
    } else {
        t.to_string()
    }
}

/// Ensure chrome + universe HTML are available via App URL (frontendDist).
pub fn ensure_chrome_asset() -> Result<PathBuf, String> {
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../webizen-studio/dist");
    std::fs::create_dir_all(&dist).map_err(|e| e.to_string())?;
    let chrome = dist.join("browser-chrome.html");
    std::fs::write(&chrome, CHROME_HTML).map_err(|e| e.to_string())?;
    let universe = dist.join(UNIVERSE_ASSET);
    std::fs::write(&universe, UNIVERSE_HTML).map_err(|e| e.to_string())?;
    Ok(chrome)
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
            let _ = win_label;
            let _ = relayout_browser(&app);
        }
    });
}

fn content_webview_url(start: &str) -> Result<WebviewUrl, String> {
    if is_chora_universe_url(start) {
        return Ok(WebviewUrl::App(UNIVERSE_ASSET.into()));
    }
    let parsed: tauri::Url = start
        .parse()
        .map_err(|e| format!("Invalid start URL '{start}': {e}"))?;
    Ok(WebviewUrl::External(parsed))
}

/// Drop and re-create the content webview (needed to switch App ↔ External).
fn replace_content_webview(app: &AppHandle, url: &str) -> Result<(), String> {
    let window = app
        .get_window(WINDOW_LABEL)
        .ok_or_else(|| "browser window not open".to_string())?;
    if let Some(w) = app.get_webview(CONTENT_LABEL) {
        let _ = w.close();
    }
    let (width, height) = logical_inner(&window)?;
    let content_h = (height - CHROME_H).max(120.0);
    let wv_url = content_webview_url(url)?;
    window
        .add_child(
            WebviewBuilder::new(CONTENT_LABEL, wv_url),
            LogicalPosition::new(0.0, CHROME_H),
            LogicalSize::new(width, content_h),
        )
        .map_err(|e| format!("content webview: {e}"))?;
    // Re-bind cert-override after content webview recreation.
    let _ = cert_override::attach_to_content_webview(app);
    Ok(())
}

/// Create or focus the browser window with chrome + content child webviews.
pub fn open_browser_shell(app: &AppHandle, start_url: &str) -> Result<String, String> {
    let _ = ensure_chrome_asset()?;
    let start = resolve_start_url(start_url);
    set_last_url(&start);

    if app.get_window(WINDOW_LABEL).is_some() {
        if app.get_webview(CONTENT_LABEL).is_none() || app.get_webview(CHROME_LABEL).is_none() {
            if let Some(w) = app.get_window(WINDOW_LABEL) {
                let _ = w.close();
            }
        } else {
            navigate_content(app, &start)?;
            let _ = relayout_browser(app);
            focus_window(app)?;
            return Ok(start);
        }
    }

    let chrome_path = format!("browser-chrome.html?url={}", urlencoding::encode(&start));

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
                "chrome webview failed: {e}. Fallback: use Reach pane chrome + browser_navigate only."
            )
        })?;

    let content_url = content_webview_url(&start).map_err(|e| {
        let _ = window.close();
        e
    })?;

    window
        .add_child(
            WebviewBuilder::new(CONTENT_LABEL, content_url),
            LogicalPosition::new(0.0, CHROME_H),
            LogicalSize::new(width, content_h),
        )
        .map_err(|e| {
            let _ = window.close();
            format!(
                "content webview failed: {e}. Fallback: single-window navigation via Reach Focus."
            )
        })?;

    attach_resize_handler(&window, app.clone());
    // C1: attach cert-override on content webview (Windows). Best-effort.
    let _ = cert_override::attach_to_content_webview(app);
    let _ = window.set_focus();
    Ok(start)
}

/// Navigate only the content webview (chrome stays put).
pub fn navigate_content(app: &AppHandle, url: &str) -> Result<(), String> {
    let url = resolve_start_url(url);
    if url.is_empty() {
        return Err("empty URL".into());
    }

    if app.get_webview(CONTENT_LABEL).is_none() {
        set_last_url(&url);
        return open_browser_shell(app, &url).map(|_| ());
    }

    let prev = last_url();
    let to_universe = is_chora_universe_url(&url);
    let from_universe = is_chora_universe_url(&prev);
    set_last_url(&url);

    // App ↔ External switch requires recreating the content child.
    if to_universe || from_universe {
        replace_content_webview(app, &url)?;
        let _ = cert_override::attach_to_content_webview(app);
        return Ok(());
    }

    let parsed: tauri::Url = url
        .parse()
        .map_err(|e| format!("Invalid URL '{url}': {e}"))?;

    if let Some(w) = app.get_webview(CONTENT_LABEL) {
        match w.navigate(parsed) {
            Ok(()) => Ok(()),
            Err(_) => {
                replace_content_webview(app, &url)?;
                let _ = cert_override::attach_to_content_webview(app);
                Ok(())
            }
        }
    } else {
        open_browser_shell(app, &url).map(|_| ())
    }
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
    let last = last_url();
    if !last.is_empty() {
        return last;
    }
    if app.get_webview(CONTENT_LABEL).is_some() {
        DEFAULT_HOME.into()
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
        "default_home": DEFAULT_HOME,
        "chrome": "in-window multi-webview",
        "substrate": "os-webview",
        "phases": ["P0", "P0.1", "P1-store", "P2-agent", "chora-home", "S1-S2-engine"],
        "url_sync": "poll browser_content_url ≤1s",
        "cert_override": cert_override::status_json(),
        "engine": engine::status_json(),
        "cookie_jar": "webview_cookies_for_url",
        "suggested_trust_catalog": "empty_until_principal_curates",
        "note": "Default content is Chora universe (App). Engine default is OS WebView; ServoExperimental is preference-only until libservo is linked. Cert-override consults store when hook attached; else OS TLS.",
    })
}

// ── Trust store ──────────────────────────────────────────────────────────────

pub fn trust_list() -> Result<serde_json::Value, String> {
    let s = qualia_client_core::webizen_trust::TrustStore::load(&storage_root());
    serde_json::to_value(&s).map_err(|e| e.to_string())
}

pub fn trust_add_pem(
    label: String,
    pem: String,
    notes: String,
) -> Result<serde_json::Value, String> {
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

pub fn trust_add_did(
    label: String,
    did: String,
    notes: String,
) -> Result<serde_json::Value, String> {
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
    let resp = qualia_client_core::browser_agent::run_browser_agent(&storage_root(), req).await?;
    serde_json::to_value(resp).map_err(|e| e.to_string())
}
