//! Shared helpers for browser panes.

//! **Reach / Webizen Browser chrome** (studio pane).
//!
//! Pages load in a **native top-level WebView** window (`webizen-browser` via
//! `browser_navigate` / `open_web_url`) — not an iframe (X-Frame-Options) and not
//! the missing `spawn_native_webview` child-pane path.
//!
//! This pane owns: tabs, omnibox, back/forward/reload (app history + engine), focus,
//! and the dialectical sidebar. See `docs/plans/webizen-browser-and-trust.md` P0/P0.1.

pub use dioxus::prelude::*;
pub use serde_json::json;
pub use uuid::Uuid;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen::JsCast;

pub use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
pub use crate::components::qapp_engine::invoke_json;

pub async fn invoke_tauri(cmd: &str, args: serde_json::Value) -> Result<String, String> {
    let res = invoke_json(cmd, args).await?;
    if let Some(s) = res.as_str() {
        Ok(s.to_string())
    } else {
        // Commands may return JSON objects (browser_status) or bare strings.
        Ok(res.to_string())
    }
}

pub fn is_web_or_app_url(url: &str) -> bool {
    let u = url.trim().to_ascii_lowercase();
    u.starts_with("http://")
        || u.starts_with("https://")
        || u.starts_with("qualia://")
        || u.starts_with("webizen://")
}

/// Parse cookie refresh/summary JSON into UI fields (K2).
pub fn apply_cookie_summary(
    v: &serde_json::Value,
    url: &str,
    mut summary_text: Signal<String>,
    mut first_party: Signal<Vec<serde_json::Value>>,
    mut third_party: Signal<Vec<serde_json::Value>>,
    mut third_domains: Signal<Vec<String>>,
    mut coverage: Signal<String>,
) {
    let cookies = v
        .get("cookies")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    let third = v
        .get("third_parties")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let count = v
        .get("cookie_count")
        .and_then(|x| x.as_u64())
        .unwrap_or(cookies.len() as u64);
    let synced = v
        .get("synced")
        .and_then(|x| x.as_u64())
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into());
    let source = v
        .get("source")
        .and_then(|x| x.as_str())
        .unwrap_or("—");
    let note = v
        .get("coverage_note")
        .and_then(|x| x.as_str())
        .unwrap_or("Best-effort jar visibility — not complete Chromium parity.");
    let fp: Vec<serde_json::Value> = cookies
        .iter()
        .filter(|c| !c.get("third_party").and_then(|x| x.as_bool()).unwrap_or(false))
        .cloned()
        .collect();
    let tp: Vec<serde_json::Value> = cookies
        .iter()
        .filter(|c| c.get("third_party").and_then(|x| x.as_bool()).unwrap_or(false))
        .cloned()
        .collect();
    summary_text.set(format!(
        "{url} · count={count} · synced={synced} · source={source}"
    ));
    first_party.set(fp);
    third_party.set(tp);
    third_domains.set(third);
    coverage.set(note.to_string());
}

pub fn display_title_for(url: &str) -> String {
    let u = url.trim();
    if u.is_empty() {
        return "New Tab".into();
    }
    // Host-ish label without depending on the `url` crate.
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .or_else(|| u.strip_prefix("qualia://"))
        .or_else(|| u.strip_prefix("webizen://"))
        .unwrap_or(u);
    let host = rest.split(['/', '?', '#']).next().unwrap_or(rest);
    if host.is_empty() {
        u.chars().take(40).collect()
    } else {
        host.chars().take(48).collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTab {
    pub id: String,
    pub title: String,
    pub url: String,
    pub history: Vec<String>,
    pub history_index: usize,
}

impl BrowserTab {
    pub fn new(url: String) -> Self {
        let title = display_title_for(&url);
        Self {
            id: Uuid::new_v4().to_string(),
            title,
            url: url.clone(),
            history: vec![url],
            history_index: 0,
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.history_index > 0
    }

    pub fn can_go_forward(&self) -> bool {
        self.history_index + 1 < self.history.len()
    }

    pub fn go_back(&mut self) -> Option<String> {
        if self.can_go_back() {
            self.history_index -= 1;
            self.url = self.history[self.history_index].clone();
            self.title = display_title_for(&self.url);
            Some(self.url.clone())
        } else {
            None
        }
    }

    pub fn go_forward(&mut self) -> Option<String> {
        if self.can_go_forward() {
            self.history_index += 1;
            self.url = self.history[self.history_index].clone();
            self.title = display_title_for(&self.url);
            Some(self.url.clone())
        } else {
            None
        }
    }

    pub fn navigate(&mut self, url: String) {
        self.history.truncate(self.history_index + 1);
        self.history.push(url.clone());
        self.history_index = self.history.len() - 1;
        self.url = url;
        self.title = display_title_for(&self.url);
    }
}

/// Open URL in the native Webizen Browser window (top-level WebView).
pub async fn navigate_native(url: &str) -> Result<String, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty URL".into());
    }
    // Prefer browser_navigate (returns URL string); fall back to open_web_url.
    match invoke_tauri("browser_navigate", json!({ "url": url })).await {
        Ok(s) => {
            // May be JSON-quoted string
            if let Ok(v) = serde_json::from_str::<String>(&s) {
                Ok(v)
            } else if s.starts_with('"') {
                serde_json::from_str(&s).map_err(|e| e.to_string())
            } else {
                Ok(if s.is_empty() { url.to_string() } else { s })
            }
        }
        Err(_) => {
            invoke_tauri("open_web_url", json!({ "url": url })).await?;
            Ok(url.to_string())
        }
    }
}

