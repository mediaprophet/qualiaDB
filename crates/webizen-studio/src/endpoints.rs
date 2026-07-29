//! Centralized local-daemon endpoints and runtime capability detection.
//!
//! Previously these `127.0.0.1` URLs were scattered as string literals through
//! `studio_canvas.rs`. In the web demo there is no local daemon, so every one of
//! those fetches failed and spammed the console. This module gives them one home
//! and a single capability gate (`is_native_host`) so the network effects only
//! fire where a daemon can actually exist.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostSurface {
    PublicWeb,
    DesktopWebview,
}

const DEFAULT_DAEMON_PORT: u16 = 8080;
/// Local native-LLM / handshake WebSocket.
pub const NATIVE_WS: &str = "ws://127.0.0.1:4242";
/// Custom web-module sandbox RPC socket.
pub const MODULE_RPC_WS: &str = "ws://127.0.0.1:9001";

/// `{DAEMON_HTTP}/manifest` — workspace manifest GET/POST.
pub fn daemon_http() -> String {
    format!("http://127.0.0.1:{}", settings_port())
}

#[cfg(target_arch = "wasm32")]
fn settings_port() -> u16 {
    js_sys::Reflect::get(
        &js_sys::global(),
        &wasm_bindgen::JsValue::from_str("__WEBIZEN_SETTINGS_PORT"),
    )
    .ok()
    .and_then(|v| v.as_f64())
    .map(|v| v as u16)
    .filter(|port| *port > 0)
    .unwrap_or(DEFAULT_DAEMON_PORT)
}

#[cfg(not(target_arch = "wasm32"))]
fn settings_port() -> u16 {
    DEFAULT_DAEMON_PORT
}

pub fn manifest_url() -> String {
    format!("{}/manifest", daemon_http())
}

/// `{DAEMON_HTTP}/manifest/history` — Quin WAL deploy checkpoints.
pub fn manifest_history_url() -> String {
    format!("{}/manifest/history", daemon_http())
}

/// Live Qualia Portal design studio (T2 WASM stack).
pub fn portal_design_studio_url() -> String {
    format!("{}/design-studio.html", daemon_http())
}

pub fn assets_catalog_url() -> String {
    format!("{}/api/assets/catalog", daemon_http())
}

pub fn assets_enqueue_url() -> String {
    format!("{}/api/assets/enqueue", daemon_http())
}

pub fn job_url(job_id: &str) -> String {
    format!("{}/api/jobs/{job_id}", daemon_http())
}

pub fn manifest_replay_url(revision: u64) -> String {
    format!("{}/manifest/replay/{revision}", daemon_http())
}

/// `{DAEMON_HTTP}/generate_pane` — keyword/domain pane layout planner.
pub fn generate_pane_url() -> String {
    format!("{}/generate_pane", daemon_http())
}

pub fn manifest_undo_chain_url() -> String {
    format!("{}/manifest/undo-chain", daemon_http())
}

pub fn manifest_undo_frame_url(stack_index: u16) -> String {
    format!(
        "{}/manifest/undo-frame?stack_index={stack_index}",
        daemon_http()
    )
}

/// `{DAEMON_HTTP}/telemetry` — server-sent telemetry stream.
pub fn telemetry_url() -> String {
    format!("{}/telemetry", daemon_http())
}

pub fn logs_page_url() -> String {
    format!("{}/logs", daemon_http())
}

/// Native LLM / handshake WebSocket endpoint (desktop daemon).
pub fn native_handshake_ws() -> &'static str {
    NATIVE_WS
}

/// Human-readable label for the active host surface (studio status chips).
pub fn host_surface_label(surface: HostSurface) -> &'static str {
    match surface {
        HostSurface::PublicWeb => "Public web demo",
        HostSurface::DesktopWebview => "Desktop webview",
    }
}

/// All host surfaces the studio can run in (picker + capability docs).
pub fn all_host_surfaces() -> [HostSurface; 2] {
    [HostSurface::DesktopWebview, HostSurface::PublicWeb]
}

/// Probe whether the native LLM handshake port is reachable (TCP preflight).
#[cfg(not(target_arch = "wasm32"))]
pub fn probe_native_handshake_port() -> bool {
    let addr = NATIVE_WS
        .trim_start_matches("ws://")
        .trim_start_matches("wss://");
    std::net::TcpStream::connect(addr).is_ok()
}

#[cfg(target_arch = "wasm32")]
pub fn probe_native_handshake_port() -> bool {
    false
}

/// Bundled Shoelace path (offline Tauri desktop). GH Pages demo uses CDN.
pub const SHOELACE_VENDOR_BASE: &str = "/vendor/shoelace";
pub const SHOELACE_CDN_BASE: &str =
    "https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.15.0/cdn";

pub fn shoelace_base() -> &'static str {
    if is_native_host() {
        SHOELACE_VENDOR_BASE
    } else {
        SHOELACE_CDN_BASE
    }
}

pub fn shoelace_stylesheet_href() -> String {
    format!("{}/themes/dark.css", shoelace_base())
}

pub fn shoelace_autoloader_src() -> String {
    format!("{}/shoelace-autoloader.js", shoelace_base())
}

/// True when a local Webizen daemon could plausibly be reached.
///
/// On native builds this is always true. On wasm it is true only inside the
/// Tauri desktop webview (detected via `window.__TAURI__`); a plain browser tab
/// (the GitHub Pages demo) returns false so callers can skip daemon traffic.
#[cfg(target_arch = "wasm32")]
pub fn is_native_host() -> bool {
    current_host_surface() != HostSurface::PublicWeb
}

/// True when a local Webizen daemon could plausibly be reached (always, native).
#[cfg(not(target_arch = "wasm32"))]
pub fn is_native_host() -> bool {
    true
}

#[cfg(target_arch = "wasm32")]
pub fn current_host_surface() -> HostSurface {
    let tauri_present = js_sys::Reflect::get(
        &js_sys::global(),
        &wasm_bindgen::JsValue::from_str("__TAURI__"),
    )
    .map(|v| !v.is_undefined() && !v.is_null())
    .unwrap_or(false);

    if tauri_present {
        HostSurface::DesktopWebview
    } else {
        HostSurface::PublicWeb
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn current_host_surface() -> HostSurface {
    HostSurface::DesktopWebview
}

pub fn supports_browser_pane() -> bool {
    matches!(current_host_surface(), HostSurface::DesktopWebview)
}
