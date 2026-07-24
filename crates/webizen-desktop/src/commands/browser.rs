//! Browser navigation, trust, cookies, cert overrides, engine

#![allow(non_snake_case)]

use tauri::{command, AppHandle};

#[command]
pub async fn open_web_url(app: AppHandle, url: String) -> Result<(), String> {
    browser_navigate(app, url).await.map(|_| ())
}

/// Open shell if needed and navigate content to `url`.
#[command]
pub async fn browser_navigate(app: AppHandle, url: String) -> Result<String, String> {
    // WebView2: create windows off the sync command path (Tauri docs: avoid deadlock).
    let app2 = app.clone();
    let url2 = url.clone();
    tauri::async_runtime::spawn_blocking(move || crate::browser::open_browser_shell(&app2, &url2))
        .await
        .map_err(|e| e.to_string())?
}

/// Navigate only the content webview (chrome stays).
#[command]
pub async fn browser_navigate_content(app: AppHandle, url: String) -> Result<(), String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || crate::browser::navigate_content(&app2, &url))
        .await
        .map_err(|e| e.to_string())?
}

#[command]
pub fn browser_focus(app: AppHandle) -> Result<bool, String> {
    crate::browser::focus_window(&app)
}

#[command]
pub fn browser_reload(app: AppHandle) -> Result<(), String> {
    crate::browser::reload_content(&app)
}

#[command]
pub fn browser_reload_content(app: AppHandle) -> Result<(), String> {
    crate::browser::reload_content(&app)
}

#[command]
pub fn browser_go_back(app: AppHandle) -> Result<(), String> {
    crate::browser::content_history_back(&app)
}

#[command]
pub fn browser_go_forward(app: AppHandle) -> Result<(), String> {
    crate::browser::content_history_forward(&app)
}

#[command]
pub fn browser_status(app: AppHandle) -> Result<serde_json::Value, String> {
    Ok(crate::browser::status(&app))
}

/// Content URL for chrome omnibox poll (≤1s). Last host-driven navigation.
#[command]
pub fn browser_content_url(app: AppHandle) -> Result<String, String> {
    Ok(crate::browser::content_url(&app))
}

/// List browser bookmarks (qlinks JSON + library purpose=bookmark).
#[command]
pub fn list_qlinks() -> Result<serde_json::Value, String> {
    use qualia_client_core::state::{config_file_path, AgentConfig};
    use std::fs;
    let config_path = config_file_path();
    let storage_path = if let Ok(config_str) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<AgentConfig>(&config_str) {
            config.storage_path
        } else {
            qualia_client_core::state::dirs_default_path()
        }
    } else {
        qualia_client_core::state::dirs_default_path()
    };
    let list = qualia_client_core::wellfair::bookmarks::list_all_bookmarks(
        std::path::Path::new(&storage_path),
    )?;
    Ok(serde_json::json!({ "bookmarks": list, "count": list.len() }))
}

/// Cookie transparency graph summary for current URL (v0/v1 coverage).
#[command]
pub fn browser_cookie_summary(url: String) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    Ok(qualia_client_core::cookie_graph::summary_for_url(&root, &url))
}

/// Record observed Set-Cookie lines (agent / host) into the cookie graph.
#[command]
pub fn browser_cookie_observe(url: String, set_cookies: Vec<String>) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let g = qualia_client_core::cookie_graph::observe_set_cookies(&root, &url, &set_cookies, now)?;
    serde_json::to_value(g).map_err(|e| e.to_string())
}

/// Refresh cookie graph from the content webview jar (K1 — Tauri cookies_for_url).
#[command]
pub fn browser_cookies_refresh(app: AppHandle, url: Option<String>) -> Result<serde_json::Value, String> {
    let url = url
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| crate::browser::last_url());
    crate::browser::cookies::refresh_jar_for_url(&app, &url)
}

/// Clear site data: cookie graph for origin + best-effort jar clear for content webview.
#[command]
pub fn browser_clear_site_data(
    app: AppHandle,
    url: Option<String>,
    all: Option<bool>,
) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    if all.unwrap_or(false) {
        let g = qualia_client_core::cookie_graph::clear_graph_all(&root)?;
        let jar = crate::browser::cookies::clear_jar_all(&app).unwrap_or_else(|e| {
            serde_json::json!({ "jar_clear": "error", "detail": e })
        });
        return Ok(serde_json::json!({
            "graph": g,
            "jar": jar,
            "note": "Full clear requested. Coverage is best-effort.",
        }));
    }
    let url = url
        .filter(|u| !u.trim().is_empty())
        .unwrap_or_else(|| crate::browser::last_url());
    let g = qualia_client_core::cookie_graph::clear_graph_for_origin(&root, &url)?;
    let jar = crate::browser::cookies::clear_jar_for_url(&app, &url).unwrap_or_else(|e| {
        serde_json::json!({ "jar_clear": "error", "detail": e })
    });
    Ok(serde_json::json!({
        "url": url,
        "graph": g,
        "jar": jar,
        "note": "Origin graph cleared; jar clear best-effort. Explicit principal action.",
    }))
}

/// Cert-override status (C1) — active / disabled / unavailable.
#[command]
pub fn browser_cert_override_status() -> Result<serde_json::Value, String> {
    Ok(crate::browser::cert_override::status_json())
}

/// Interactive escape hatch: allow_once | always (pin) | deny (sticky session).
#[command]
pub fn browser_cert_escape_hatch(
    host: String,
    action: String,
) -> Result<serde_json::Value, String> {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return Err("empty host".into());
    }
    let act = action.trim().to_ascii_lowercase();
    match act.as_str() {
        "allow_once" | "once" => {
            crate::browser::cert_override::session_allow_once(&host);
            crate::browser::cert_override::record_public(&host, "allow_once", "escape_hatch_session");
            Ok(serde_json::json!({
                "host": host,
                "action": "allow_once",
                "note": "Session allow only — not a permanent pin. Logged. No re-prompt until process restart if soft-deny not set."
            }))
        }
        "always" | "pin" | "allow_always" => {
            // Permanent host pin via existing command path
            let pin = browser_trust_pin_host(host.clone())?;
            crate::browser::cert_override::record_public(&host, "always_pin", "escape_hatch_pin");
            Ok(serde_json::json!({
                "host": host,
                "action": "always",
                "pin": pin,
                "note": "Host pin written to trust store — subsequent cert errors silent for this host (WebID-TLS lesson)."
            }))
        }
        "deny" | "soft_deny" => {
            crate::browser::cert_override::session_soft_deny(&host);
            // Persist soft-deny in store for durability
            let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
            let mut store = qualia_client_core::webizen_trust::TrustStore::load(&root);
            let material = qualia_client_core::webizen_trust::host_deny_material(&host);
            if !store.anchors.iter().any(|a| a.material.eq_ignore_ascii_case(&material)) {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                use qualia_client_core::webizen_trust::{AnchorKind, TrustAnchor};
                store.anchors.push(TrustAnchor {
                    id: format!("policy:deny:{host}"),
                    label: format!("Deny cert override: {host}"),
                    kind: AnchorKind::PolicyLabel,
                    material,
                    enabled: true,
                    notes: "Principal soft-deny — no re-prompt storm".into(),
                    added_unix: now,
                });
                store.save(&root)?;
            }
            crate::browser::cert_override::record_public(&host, "deny", "escape_hatch_soft_deny");
            Ok(serde_json::json!({
                "host": host,
                "action": "deny",
                "note": "Sticky deny for host (session + store). Fail closed."
            }))
        }
        _ => Err("action must be allow_once | always | deny".into()),
    }
}

#[command]
pub fn browser_agent_tls_status() -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let st = qualia_client_core::browser_agent::agent_tls_status(&root);
    serde_json::to_value(st).map_err(|e| e.to_string())
}

/// Enable/disable cert-override policy consultation (default enabled when hook attached).
#[command]
pub fn browser_cert_override_set_enabled(enabled: bool) -> Result<serde_json::Value, String> {
    crate::browser::cert_override::set_override_enabled(enabled);
    Ok(crate::browser::cert_override::status_json())
}

/// Re-attach cert-override hook to content webview (after recreate).
#[command]
pub fn browser_cert_override_attach(app: AppHandle) -> Result<serde_json::Value, String> {
    let ok = crate::browser::cert_override::attach_to_content_webview(&app)?;
    let mut v = crate::browser::cert_override::status_json();
    if let Some(obj) = v.as_object_mut() {
        obj.insert("attach_result".into(), serde_json::json!(ok));
    }
    Ok(v)
}

/// Add host-allow policy pin so cert errors for that host may be always-allowed.
#[command]
pub fn browser_trust_pin_host(host: String) -> Result<serde_json::Value, String> {
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return Err("empty host".into());
    }
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let mut store = qualia_client_core::webizen_trust::TrustStore::load(&root);
    let material = format!("host-allow:{host}");
    if store
        .anchors
        .iter()
        .any(|a| a.material.eq_ignore_ascii_case(&material))
    {
        return Err("host already pinned".into());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    use qualia_client_core::webizen_trust::{AnchorKind, TrustAnchor};
    let id = format!("policy:host:{}", host);
    store.anchors.push(TrustAnchor {
        id: id.clone(),
        label: format!("Allow cert errors: {host}"),
        kind: AnchorKind::PolicyLabel,
        material,
        enabled: true,
        notes: "Principal host-pin for WebView2 ServerCertificateErrorDetected".into(),
        added_unix: now,
    });
    store.save(&root)?;
    Ok(serde_json::json!({ "id": id, "host": host, "enabled": true }))
}

/// Engine preference + honesty status (S1/S2). Never claims Servo paints pages unless linked.
#[command]
pub fn browser_engine_status() -> Result<serde_json::Value, String> {
    Ok(crate::browser::engine::status_json())
}

/// Persist engine preference under `{storage}/webizen/browser_engine.json`.
/// Selecting Servo without a linked embed keeps OS WebView as the active content renderer.
#[command]
pub fn browser_set_engine(engine: String) -> Result<serde_json::Value, String> {
    use crate::browser::engine::EngineKind;
    let kind = EngineKind::parse(&engine).ok_or_else(|| {
        format!("unknown engine '{engine}' (os_web_view|servo_experimental)")
    })?;
    let _pref = crate::browser::engine::set_engine(kind)?;
    // Return full status (includes banner / active_renderer) so chrome can update in one call.
    Ok(crate::browser::engine::status_json())
}

/// List suggested trust catalog (empty until principal curates).
#[command]
pub fn browser_trust_list_suggested() -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let cat = qualia_client_core::webizen_trust::SuggestedTrustCatalog::load_for_storage(&root)?;
    serde_json::to_value(cat).map_err(|e| e.to_string())
}

/// Import a suggested catalog entry into the live trust store.
#[command]
pub fn browser_trust_import_suggested(id: String, enable: Option<bool>) -> Result<serde_json::Value, String> {
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let cat = qualia_client_core::webizen_trust::SuggestedTrustCatalog::load_for_storage(&root)?;
    let base = qualia_client_core::webizen_trust::bundled_catalog_path()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| root.join("webizen"));
    let mut store = qualia_client_core::webizen_trust::TrustStore::load(&root);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let a = qualia_client_core::webizen_trust::import_suggested_into_store(
        &mut store,
        &cat,
        &id,
        &base,
        now,
        enable.unwrap_or(false),
    )?;
    store.save(&root)?;
    serde_json::to_value(a).map_err(|e| e.to_string())
}

#[command]
pub fn browser_trust_list() -> Result<serde_json::Value, String> {
    crate::browser::trust_list()
}

#[command]
pub fn browser_trust_add_pem(label: String, pem: String, notes: Option<String>) -> Result<serde_json::Value, String> {
    crate::browser::trust_add_pem(label, pem, notes.unwrap_or_default())
}

#[command]
pub fn browser_trust_add_did(label: String, did: String, notes: Option<String>) -> Result<serde_json::Value, String> {
    crate::browser::trust_add_did(label, did, notes.unwrap_or_default())
}

#[command]
pub fn browser_trust_set_enabled(id: String, enabled: bool) -> Result<(), String> {
    crate::browser::trust_set_enabled(id, enabled)
}

#[command]
pub fn browser_trust_remove(id: String) -> Result<bool, String> {
    crate::browser::trust_remove(id)
}

#[command]
pub fn browser_trust_verdict(url: String) -> Result<serde_json::Value, String> {
    crate::browser::trust_verdict(url)
}

#[command]
pub async fn browser_agent_ask(
    url: String,
    question: String,
    ingest_to_library: Option<bool>,
) -> Result<serde_json::Value, String> {
    crate::browser::agent_ask(url, question, ingest_to_library.unwrap_or(true)).await
}

