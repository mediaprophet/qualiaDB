//! Local Webizen / QualiaDB Native Daemon Discovery & Health Monitor.
//!
//! Checks whether a native Webizen daemon (`qualia-core-db` loopback server)
//! is running locally on the machine (probing candidate ports: 8000, 3030, 4242, 8080).
//!
//! Enables Poet to dynamically switch between:
//! - **Native Acceleration Mode** (direct hardware pipelines, resident graph, heavy GGUF/GPU compute, signed WAL persistence).
//! - **Standalone / WASM Sandbox Mode** (in-browser WASM AST engine, local storage, mocked hardware).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::cell::RefCell;

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, Response};

pub const DEFAULT_CANDIDATE_PORTS: &[u16] = &[8000, 3030, 4242, 8080];

/// Information returned by the native Webizen daemon's `/health` endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonHealthResponse {
    pub status: String,
    pub engine: Option<String>,
    pub version: Option<String>,
    pub dev_mode: Option<bool>,
    pub graph_quin_count: Option<usize>,
    pub graph_revision: Option<u64>,
}

/// Dynamic connection state of the local Webizen daemon.
#[derive(Debug, Clone, PartialEq)]
pub enum DaemonConnectionState {
    Unchecked,
    Probing,
    Connected {
        url: String,
        port: u16,
        engine: String,
        version: String,
        graph_quin_count: usize,
        dev_mode: bool,
    },
    Offline {
        candidate_ports: Vec<u16>,
        reason: String,
    },
}

thread_local! {
    static DAEMON_STATE: RefCell<DaemonConnectionState> = RefCell::new(DaemonConnectionState::Unchecked);
}

/// Get the current connection state of the native daemon.
pub fn get_daemon_state() -> DaemonConnectionState {
    DAEMON_STATE.with(|s| s.borrow().clone())
}

/// Check if the native Webizen daemon is currently connected and active.
pub fn is_daemon_connected() -> bool {
    matches!(get_daemon_state(), DaemonConnectionState::Connected { .. })
}

/// Get the base URL of the connected daemon (e.g. `http://127.0.0.1:8000`).
pub fn get_connected_daemon_url() -> Option<String> {
    match get_daemon_state() {
        DaemonConnectionState::Connected { url, .. } => Some(url),
        _ => None,
    }
}

/// Update the thread-local state and refresh all badge UI elements in the DOM.
fn set_daemon_state(state: DaemonConnectionState) {
    DAEMON_STATE.with(|s| {
        *s.borrow_mut() = state;
    });

    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            update_all_status_badges(&doc);
        }
    }
}

// ---------------------------------------------------------------------------
// Asynchronous Daemon Probing
// ---------------------------------------------------------------------------

/// Asynchronously probe candidate loopback ports for a running Webizen daemon.
pub fn spawn_daemon_probe() {
    set_daemon_state(DaemonConnectionState::Probing);

    wasm_bindgen_futures::spawn_local(async {
        let ports = DEFAULT_CANDIDATE_PORTS;
        for &port in ports {
            let url = format!("http://127.0.0.1:{port}");
            let health_url = format!("{url}/health");

            if let Some(health) = fetch_daemon_health(&health_url).await {
                web_sys::console::log_1(
                    &format!(
                        "[Webizen Probe] Found running native daemon at {url} (engine: {}, quins: {})",
                        health.engine.as_deref().unwrap_or("qualia-core-db"),
                        health.graph_quin_count.unwrap_or(0)
                    )
                    .into(),
                );

                set_daemon_state(DaemonConnectionState::Connected {
                    url,
                    port,
                    engine: health.engine.unwrap_or_else(|| "qualia-core-db".into()),
                    version: health.version.unwrap_or_else(|| "0.0.34".into()),
                    graph_quin_count: health.graph_quin_count.unwrap_or(0),
                    dev_mode: health.dev_mode.unwrap_or(false),
                });
                return;
            }
        }

        web_sys::console::log_1(
            &"[Webizen Probe] No native daemon running on local ports (running in Standalone WASM mode)".into(),
        );

        set_daemon_state(DaemonConnectionState::Offline {
            candidate_ports: ports.to_vec(),
            reason: "Connection refused on candidate loopback ports".into(),
        });
    });
}

async fn fetch_daemon_health(health_url: &str) -> Option<DaemonHealthResponse> {
    let window = web_sys::window()?;
    let promise = window.fetch_with_str(health_url);
    let resp_val = wasm_bindgen_futures::JsFuture::from(promise).await.ok()?;
    let resp: Response = resp_val.dyn_into().ok()?;

    if !resp.ok() {
        return None;
    }

    let json_promise = resp.json().ok()?;
    let json_val = wasm_bindgen_futures::JsFuture::from(json_promise).await.ok()?;
    serde_wasm_bindgen::from_value(json_val).ok()
}

// ---------------------------------------------------------------------------
// DOM Badges & Status Indicators
// ---------------------------------------------------------------------------

/// Build an interactive Webizen Native status badge with click-to-probe.
pub fn build_daemon_status_badge(document: &Document) -> Element {
    let badge = document.create_element("span").unwrap();
    badge.set_class_name("webizen-native-status-badge");
    let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; font-weight: 600; padding: 2px 7px; \
         border-radius: var(--radius-xs); cursor: pointer; user-select: none; transition: all 0.2s; \
         display: inline-flex; align-items: center; gap: 4px;",
    );

    render_badge_content(&badge);

    let click_closure = Closure::wrap(Box::new(move |_e: Event| {
        web_sys::console::log_1(&"[Webizen Probe] Manual probe requested by user".into());
        spawn_daemon_probe();
    }) as Box<dyn FnMut(Event)>);
    badge
        .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
        .unwrap();
    click_closure.forget();

    badge
}

/// Refresh all rendered `.webizen-native-status-badge` elements across the DOM.
pub fn update_all_status_badges(document: &Document) {
    if let Ok(list) = document.query_selector_all(".webizen-native-status-badge") {
        for i in 0..list.length() {
            if let Some(el) = list.item(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                render_badge_content(&el);
            }
        }
    }
}

fn render_badge_content(badge: &Element) {
    let state = get_daemon_state();
    let b_el: Result<HtmlElement, _> = badge.clone().dyn_into();
    let Ok(b_el) = b_el else { return };

    match state {
        DaemonConnectionState::Unchecked | DaemonConnectionState::Probing => {
            b_el.style().set_css_text(
                "background: rgba(255, 184, 52, 0.12); color: var(--accent-amber, #ffb834); \
                 border: 1px solid rgba(255, 184, 52, 0.3); font-family: var(--font-mono); \
                 font-size: 10px; font-weight: 600; padding: 2px 7px; border-radius: var(--radius-xs); \
                 cursor: pointer; display: inline-flex; align-items: center; gap: 4px;",
            );
            badge.set_text_content(Some("\u{25CB} Probing Webizen\u{2026}"));
            let _ = badge.set_attribute("title", "Probing local ports (8000, 3030, 4242, 8080) for native daemon...");
        }
        DaemonConnectionState::Connected {
            port,
            graph_quin_count,
            ..
        } => {
            b_el.style().set_css_text(
                "background: rgba(0, 242, 169, 0.12); color: var(--accent-emerald, #00f2a9); \
                 border: 1px solid rgba(0, 242, 169, 0.3); font-family: var(--font-mono); \
                 font-size: 10px; font-weight: 600; padding: 2px 7px; border-radius: var(--radius-xs); \
                 cursor: pointer; display: inline-flex; align-items: center; gap: 4px;",
            );
            badge.set_text_content(Some(&format!("\u{25CF} Native: Connected (:{port})")));
            let _ = badge.set_attribute(
                "title",
                &format!(
                    "QualiaDB Native Daemon Active on port {port} · {graph_quin_count} Quins in graph. Click to re-probe."
                ),
            );
        }
        DaemonConnectionState::Offline { .. } => {
            b_el.style().set_css_text(
                "background: rgba(94, 115, 148, 0.15); color: var(--text-muted, #94a3b8); \
                 border: 1px solid var(--border-subtle, rgba(255,255,255,0.08)); font-family: var(--font-mono); \
                 font-size: 10px; font-weight: 600; padding: 2px 7px; border-radius: var(--radius-xs); \
                 cursor: pointer; display: inline-flex; align-items: center; gap: 4px;",
            );
            badge.set_text_content(Some("\u{25CB} Native: Standalone WASM"));
            let _ = badge.set_attribute(
                "title",
                "Local Webizen daemon offline. Running in in-browser WASM mode. Click to probe for running daemon.",
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_state_default() {
        assert_eq!(get_daemon_state(), DaemonConnectionState::Unchecked);
        assert!(!is_daemon_connected());
    }

    #[test]
    fn test_daemon_state_connected_query() {
        set_daemon_state(DaemonConnectionState::Connected {
            url: "http://127.0.0.1:8000".into(),
            port: 8000,
            engine: "qualia-core-db".into(),
            version: "0.0.34".into(),
            graph_quin_count: 42000,
            dev_mode: true,
        });

        assert!(is_daemon_connected());
        assert_eq!(
            get_connected_daemon_url(),
            Some("http://127.0.0.1:8000".into())
        );
    }
}
