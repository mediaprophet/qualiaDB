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

/// Base URL of the local Webizen daemon HTTP server (manifest, telemetry).
pub const DAEMON_HTTP: &str = "http://127.0.0.1:8080";
/// Local native-LLM / handshake WebSocket.
pub const NATIVE_WS: &str = "ws://127.0.0.1:4242";
/// Custom web-module sandbox RPC socket.
pub const MODULE_RPC_WS: &str = "ws://127.0.0.1:9001";

/// `{DAEMON_HTTP}/manifest` — workspace manifest GET/POST.
pub fn manifest_url() -> String {
    format!("{DAEMON_HTTP}/manifest")
}

/// `{DAEMON_HTTP}/telemetry` — server-sent telemetry stream.
pub fn telemetry_url() -> String {
    format!("{DAEMON_HTTP}/telemetry")
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
