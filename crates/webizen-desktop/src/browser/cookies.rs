//! WebView cookie-jar bridge (K1) — Tauri `cookies_for_url` → cookie_graph.
//!
//! Honest coverage: best-effort jar visibility for the content webview profile.
//! Not Chromium complete parity; no MITM.

use tauri::{AppHandle, Manager, Url};

use super::{CONTENT_LABEL, last_url};
use qualia_client_core::cookie_graph::{hypothesize_purpose, CookieGraph, CookieNode};

fn storage_root() -> std::path::PathBuf {
    std::path::PathBuf::from(qualia_client_core::state::dirs_default_path())
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn host_of(url: &str) -> String {
    let u = url.trim();
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .unwrap_or(u);
    rest.split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn origin_of(url: &str) -> String {
    let host = host_of(url);
    if url.starts_with("https") {
        format!("https://{host}")
    } else if url.starts_with("http") {
        format!("http://{host}")
    } else {
        url.to_string()
    }
}

/// Pull cookies for `url` from the content webview and upsert into the graph.
pub fn refresh_jar_for_url(app: &AppHandle, url: &str) -> Result<serde_json::Value, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty URL".into());
    }
    // Local App pages have no http cookies.
    if url.starts_with("qualia://") || url.starts_with("webizen://") || !url.contains("://") {
        return Ok(serde_json::json!({
            "url": url,
            "cookie_count": 0,
            "synced": 0,
            "cookies": [],
            "third_parties": [],
            "coverage_note": "Local qualia:// / webizen:// pages have no HTTP cookies. Coverage is N/A until you navigate an https origin in the content WebView.",
            "source": "n/a_local",
        }));
    }

    let parsed: Url = url
        .parse()
        .map_err(|e| format!("Invalid URL '{url}': {e}"))?;

    let webview = app
        .get_webview(CONTENT_LABEL)
        .ok_or_else(|| "content webview not open".to_string())?;

    let cookies = webview
        .cookies_for_url(parsed)
        .map_err(|e| format!("cookies_for_url: {e}"))?;

    let page_host = host_of(url);
    let origin = origin_of(url);
    let now = now_unix();
    let mut graph = CookieGraph::load(&storage_root());
    graph.coverage_note = "v1: WebView jar via Tauri cookies_for_url + graph upsert — not complete Chromium parity; partitioned cookies may be incomplete.".into();

    let mut synced = 0usize;
    for c in cookies {
        let name = c.name().to_string();
        let domain = c
            .domain()
            .map(|d| d.trim_start_matches('.').to_ascii_lowercase())
            .unwrap_or_else(|| page_host.clone());
        let path = c.path().unwrap_or("/").to_string();
        let secure = c.secure().unwrap_or(false);
        let same_site = c
            .same_site()
            .map(|s| format!("{s:?}"))
            .unwrap_or_else(|| "Lax".into());
        let expiry = c.expires().map(|t| format!("{t:?}"));
        let third_party = {
            let d = domain.trim_start_matches('.');
            !d.is_empty() && d != page_host && !page_host.ends_with(&format!(".{d}"))
        };
        let purpose = hypothesize_purpose(&name);
        graph.upsert(CookieNode {
            origin: origin.clone(),
            name,
            domain,
            path,
            secure,
            same_site,
            expiry,
            purpose,
            third_party,
            source: "webview_jar".into(),
            observed_unix: now,
        });
        synced += 1;
    }
    graph.save(&storage_root())?;

    let summary = qualia_client_core::cookie_graph::summary_for_url(&storage_root(), url);
    Ok(serde_json::json!({
        "url": url,
        "synced": synced,
        "cookie_count": summary.get("cookie_count").cloned().unwrap_or(serde_json::json!(0)),
        "third_parties": summary.get("third_parties").cloned().unwrap_or(serde_json::json!([])),
        "cookies": summary.get("cookies").cloned().unwrap_or(serde_json::json!([])),
        "coverage_note": graph.coverage_note,
        "source": "webview_jar",
        "last_url": last_url(),
    }))
}

/// Delete jar cookies for `url` (best-effort) via Tauri `delete_cookie`.
pub fn clear_jar_for_url(app: &AppHandle, url: &str) -> Result<serde_json::Value, String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty URL".into());
    }
    if url.starts_with("qualia://") || url.starts_with("webizen://") || !url.contains("://") {
        return Ok(serde_json::json!({
            "url": url,
            "deleted": 0,
            "source": "n/a_local",
        }));
    }
    let parsed: Url = url
        .parse()
        .map_err(|e| format!("Invalid URL '{url}': {e}"))?;
    let webview = app
        .get_webview(CONTENT_LABEL)
        .ok_or_else(|| "content webview not open".to_string())?;
    let cookies = webview
        .cookies_for_url(parsed)
        .map_err(|e| format!("cookies_for_url: {e}"))?;
    let mut deleted = 0usize;
    let mut errors = Vec::new();
    for c in cookies {
        match webview.delete_cookie(c) {
            Ok(()) => deleted += 1,
            Err(e) => errors.push(format!("{e}")),
        }
    }
    Ok(serde_json::json!({
        "url": url,
        "deleted": deleted,
        "errors": errors,
        "source": "webview_jar_delete",
        "note": "Best-effort jar clear for origin cookies returned by cookies_for_url.",
    }))
}

/// Delete all cookies visible via `webview.cookies()` (best-effort).
pub fn clear_jar_all(app: &AppHandle) -> Result<serde_json::Value, String> {
    let webview = app
        .get_webview(CONTENT_LABEL)
        .ok_or_else(|| "content webview not open".to_string())?;
    let cookies = webview.cookies().map_err(|e| format!("cookies: {e}"))?;
    let mut deleted = 0usize;
    let mut errors = Vec::new();
    for c in cookies {
        match webview.delete_cookie(c) {
            Ok(()) => deleted += 1,
            Err(e) => errors.push(format!("{e}")),
        }
    }
    Ok(serde_json::json!({
        "deleted": deleted,
        "errors": errors,
        "source": "webview_jar_delete_all",
        "note": "Best-effort full jar clear; partitioned/httpOnly edge cases may remain.",
    }))
}
