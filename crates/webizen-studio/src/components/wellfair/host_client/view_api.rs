//! App-wide entity-view host client (`view_*` Tauri commands).
//!
//! Shared session across shell, Library, Browser — not browser-only.

#[cfg(target_arch = "wasm32")]
use crate::components::qapp_engine::invoke_json;

/// Snapshot of app-global entity-view session.
#[cfg(target_arch = "wasm32")]
pub async fn view_session() -> Result<serde_json::Value, String> {
    invoke_json("view_session", serde_json::json!({})).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_session() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "observer": "principal",
        "presentation_level": 1,
        "selection": [],
        "morph_mode": "both",
        "attention_url": null
    }))
}

/// Set observer: principal | peer | guardian | steward | public | instrument | auditor.
#[cfg(target_arch = "wasm32")]
pub async fn view_set_observer(status: &str) -> Result<serde_json::Value, String> {
    invoke_json("view_set_observer", serde_json::json!({ "status": status })).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_set_observer(status: &str) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "observer": status }))
}

/// Presentation morphology level 0–6.
#[cfg(target_arch = "wasm32")]
pub async fn view_set_presentation_level(level: u8) -> Result<serde_json::Value, String> {
    invoke_json(
        "view_set_presentation_level",
        serde_json::json!({ "level": level }),
    )
    .await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_set_presentation_level(level: u8) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "presentation_level": level }))
}

/// Project Library (Lived Memory) for observer → flat cards + scene nodes + hidden_count.
#[cfg(target_arch = "wasm32")]
pub async fn view_project_library(
    section: Option<&str>,
    observer: Option<&str>,
    presentation_level: Option<u8>,
) -> Result<serde_json::Value, String> {
    let mut args = serde_json::Map::new();
    if let Some(s) = section {
        args.insert("section".into(), serde_json::Value::String(s.into()));
    }
    if let Some(o) = observer {
        args.insert("observer".into(), serde_json::Value::String(o.into()));
    }
    if let Some(l) = presentation_level {
        args.insert("presentationLevel".into(), serde_json::json!(l));
    }
    invoke_json("view_project_library", serde_json::Value::Object(args)).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_project_library(
    _section: Option<&str>,
    _observer: Option<&str>,
    _presentation_level: Option<u8>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "observer": "principal",
        "presentation_level": 1,
        "flat": [],
        "scene_nodes": [],
        "hidden_count": 0
    }))
}

/// Morph last projection: flatten | spatialize | both.
#[cfg(target_arch = "wasm32")]
pub async fn view_morph(mode: &str) -> Result<serde_json::Value, String> {
    invoke_json("view_morph", serde_json::json!({ "mode": mode })).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_morph(mode: &str) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "morph": mode, "flat": [], "scene_nodes": [], "hidden_count": 0 }))
}

/// Pick nearest scene node (normalized 0..1 coords) → shared selection.
#[cfg(target_arch = "wasm32")]
pub async fn view_pick_scene(nx: f64, ny: f64) -> Result<serde_json::Value, String> {
    invoke_json("view_pick_scene", serde_json::json!({ "nx": nx, "ny": ny })).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_pick_scene(_nx: f64, _ny: f64) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "found": false, "entity_id": null }))
}

/// Select entity by raw id (shared across surfaces).
#[cfg(target_arch = "wasm32")]
pub async fn view_select(entity_id: u64) -> Result<serde_json::Value, String> {
    invoke_json("view_select", serde_json::json!({ "entityId": entity_id })).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_select(_entity_id: u64) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "selection": [] }))
}

/// Select by URI (Library ↔ browser shared identity).
#[cfg(target_arch = "wasm32")]
pub async fn view_select_uri(uri: &str) -> Result<serde_json::Value, String> {
    invoke_json("view_select_uri", serde_json::json!({ "uri": uri })).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_select_uri(uri: &str) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "attention_url": uri, "selection": [] }))
}

#[cfg(target_arch = "wasm32")]
pub async fn view_clear_selection() -> Result<serde_json::Value, String> {
    invoke_json("view_clear_selection", serde_json::json!({})).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_clear_selection() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "selection": [] }))
}

/// URL → web_locus entity card (browser + habitat shared).
#[cfg(target_arch = "wasm32")]
pub async fn view_project_web_locus(
    url: &str,
    observer: Option<&str>,
) -> Result<serde_json::Value, String> {
    let mut args = serde_json::Map::new();
    args.insert("url".into(), serde_json::Value::String(url.into()));
    if let Some(o) = observer {
        args.insert("observer".into(), serde_json::Value::String(o.into()));
    }
    invoke_json("view_project_web_locus", serde_json::Value::Object(args)).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_project_web_locus(
    url: &str,
    _observer: Option<&str>,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "entity_id": 0,
        "kind": "web_locus",
        "uri": url,
        "visible": true,
        "honesty": "present"
    }))
}

/// Machine-readable capability report for design process.
#[cfg(target_arch = "wasm32")]
pub async fn view_capability_report() -> Result<serde_json::Value, String> {
    invoke_json("view_capability_report", serde_json::json!({})).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_capability_report() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "version": 1,
        "module": { "path": "entity_view" },
        "capabilities": []
    }))
}

/// Prestige GPU frame from last Memory projection.
#[cfg(target_arch = "wasm32")]
pub async fn view_render_memory_spatial(
    width: Option<u32>,
    height: Option<u32>,
) -> Result<serde_json::Value, String> {
    let mut args = serde_json::Map::new();
    if let Some(w) = width {
        args.insert("width".into(), serde_json::json!(w));
    }
    if let Some(h) = height {
        args.insert("height".into(), serde_json::json!(h));
    }
    invoke_json(
        "view_render_memory_spatial",
        serde_json::Value::Object(args),
    )
    .await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_render_memory_spatial(
    _width: Option<u32>,
    _height: Option<u32>,
) -> Result<serde_json::Value, String> {
    Err("GPU Memory spatial render requires the desktop host".into())
}

/// Phone installable remote controller URLs (PWA shell).
#[cfg(target_arch = "wasm32")]
pub async fn view_remote_controller_info() -> Result<serde_json::Value, String> {
    invoke_json("view_remote_controller_info", serde_json::json!({})).await
}
#[cfg(not(target_arch = "wasm32"))]
pub async fn view_remote_controller_info() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "urls": { "localhost": "http://127.0.0.1:8080/remote-controller/" },
        "native_app": false
    }))
}
