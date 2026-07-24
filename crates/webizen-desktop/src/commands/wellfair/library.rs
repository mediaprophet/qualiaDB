#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Manager, State};

// ── Hypermedia asset library: ingest a document → find it by meaning ──

/// Ingest a text document into the library (derive topics + searchable text; guardianship flag→notify).
#[command]
#[allow(clippy::too_many_arguments)]
pub fn wellfair_ingest_document(
    app: AppHandle,
    uri: String,
    media_type: String,
    text: String,
    guardian_did: Option<String>,
    occurred_at: Option<i64>,
    place_label: Option<String>,
    lat: Option<f32>,
    lon: Option<f32>,
    project: Option<String>,
    purpose: Option<String>,
    sensitivity: Option<String>,
    section: Option<String>,
    commons_visibility: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let manual = qualia_client_core::wellfair::api::ManualFacets {
            occurred_at,
            place_label,
            lat,
            lon,
            projects: project.into_iter().filter(|s| !s.trim().is_empty()).collect(),
            purposes: purpose.into_iter().filter(|s| !s.trim().is_empty()).collect(),
            sensitivity,
            section,
            commons_visibility,
        };
        let summary = host.ingest_document_annotated(&uri, &media_type, &text, &manual, guardian_did)?;
        serde_json::to_string(&summary).map_err(|e| e.to_string())
    })?
}

/// Ingest a **binary asset** (photo / audio) whose bytes are passed hex-encoded. A photo's EXIF capture-time
/// + GPS auto-populate the timeline + map.
#[command]
pub fn wellfair_ingest_file_hex(
    app: AppHandle,
    uri: String,
    media_type: String,
    bytes_hex: String,
    caption: String,
    guardian_did: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let summary = host.ingest_file_hex(&uri, &media_type, &bytes_hex, &caption, guardian_did)?;
        serde_json::to_string(&summary).map_err(|e| e.to_string())
    })?
}

/// Search the library by facet (`topic` | `depicts` | `place` | `project` | `purpose`).
#[command]
pub fn wellfair_search_library(app: AppHandle, facet: String, value: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let results = host.search_library(&facet, &value)?;
        serde_json::to_string(&results).map_err(|e| e.to_string())
    })?
}

/// The timeline query — entries whose event instant falls within `[start, end]` (unix seconds).
#[command]
pub fn wellfair_search_library_time(app: AppHandle, start: i64, end: i64) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let results = host.search_library_time(start, end)?;
        serde_json::to_string(&results).map_err(|e| e.to_string())
    })?
}

/// Everything in the library (newest first). Optional `section`: secret|wellfair|personal|work|commons|all.
#[command]
pub fn wellfair_list_library(app: AppHandle, section: Option<String>) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard.as_ref().ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let results = host.list_library_section(section.as_deref())?;
        serde_json::to_string(&results).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_set_library_commons(
    app: AppHandle,
    asset_uri: String,
    visibility: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let r = host.set_library_commons_visibility(&asset_uri, &visibility)?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_library_commons_share_card(
    app: AppHandle,
    asset_uri: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
        let r = host.library_commons_share_card(&asset_uri)?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_search_library_text(app: AppHandle, query: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let results = host.search_library_text(&query)?;
        serde_json::to_string(&results).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_query_library_faceted(
    app: AppHandle,
    filter_json: String,
    sort: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let sort = sort.unwrap_or_else(|| "newest".into());
        let results = host.query_library_faceted(&filter_json, &sort)?;
        serde_json::to_string(&results).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_library_facet_counts(
    app: AppHandle,
    filter_json: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let filter = filter_json.unwrap_or_default();
        let results = host.library_facet_counts(&filter)?;
        serde_json::to_string(&results).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_seed_studio_qapps(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.seed_studio_qapps_library()?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_seed_perception_library(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.seed_perception_library()?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

/// Also available without vault when AppState storage is ready (seed weights + catalogue).
#[command]
pub fn library_seed_perception_assets(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<serde_json::Value, String> {
    let config = state.config.lock().unwrap().clone();
    let root = std::path::PathBuf::from(&config.storage_path);
    let store = qualia_client_core::wellfair::hypermedia_store::HypermediaStore::open(&root)
        .map_err(|e| e.to_string())?;
    let report = qualia_client_core::wellfair::perception_catalog::seed_perception_into_library(
        &store, &root,
    )?;
    serde_json::to_value(report).map_err(|e| e.to_string())
}

// ── Vault-free hypermedia reads (same shelf as HostApi; no Sanctuary unlock) ──
//
// Why these exist: Library UI used to call only `wellfair_*` commands, which fail
// with "unlock vault first" when HostApi is nil. Seeding via `library_seed_perception_assets`
// then appeared to do nothing because list/stats still required the vault. Reads go
// through AppState.storage_path → HypermediaStore directly.

fn library_storage_root(
    state: &State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> std::path::PathBuf {
    let config = state.config.lock().unwrap().clone();
    std::path::PathBuf::from(&config.storage_path)
}

/// List library entries without Sanctuary unlock. Optional section filter.
#[command]
pub fn library_list(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    section: Option<String>,
) -> Result<serde_json::Value, String> {
    let root = library_storage_root(&state);
    let entries = qualia_client_core::wellfair::api::list_library_section_at(
        &root,
        section.as_deref(),
    )?;
    Ok(serde_json::Value::Array(entries))
}

/// Faceted library query without Sanctuary unlock.
#[command]
pub fn library_query_faceted(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    filter_json: String,
    sort: Option<String>,
) -> Result<serde_json::Value, String> {
    let root = library_storage_root(&state);
    let sort = sort.unwrap_or_else(|| "newest".into());
    qualia_client_core::wellfair::api::query_library_faceted_at(&root, &filter_json, &sort)
}

/// Library header stats without Sanctuary unlock.
#[command]
pub fn library_stats(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<serde_json::Value, String> {
    let root = library_storage_root(&state);
    qualia_client_core::wellfair::api::library_stats_at(&root)
}

/// Facet search without Sanctuary unlock.
#[command]
pub fn library_search(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    facet: String,
    value: String,
) -> Result<serde_json::Value, String> {
    let root = library_storage_root(&state);
    let entries = qualia_client_core::wellfair::api::search_library_at(&root, &facet, &value)?;
    Ok(serde_json::Value::Array(entries))
}

/// Free-text search without Sanctuary unlock.
#[command]
pub fn library_search_text(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    query: String,
) -> Result<serde_json::Value, String> {
    let root = library_storage_root(&state);
    let entries = qualia_client_core::wellfair::api::search_library_text_at(&root, &query)?;
    Ok(serde_json::Value::Array(entries))
}

/// Timeline search without Sanctuary unlock.
#[command]
pub fn library_search_time(
    state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    start: i64,
    end: i64,
) -> Result<serde_json::Value, String> {
    let root = library_storage_root(&state);
    let entries =
        qualia_client_core::wellfair::api::search_library_time_at(&root, start, end)?;
    Ok(serde_json::Value::Array(entries))
}

#[command]
pub fn wellfair_ingest_legislation_text(
    app: AppHandle,
    text: String,
    register_id: Option<String>,
    jurisdiction: Option<String>,
    title_hint: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.ingest_legislation_text(
            &text,
            register_id.as_deref(),
            jurisdiction.as_deref(),
            title_hint.as_deref(),
        )?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_ingest_legislation_pdf_hex(
    app: AppHandle,
    hex_bytes: String,
    register_id: Option<String>,
    jurisdiction: Option<String>,
    title_hint: Option<String>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.ingest_legislation_pdf_hex(
            &hex_bytes,
            register_id.as_deref(),
            jurisdiction.as_deref(),
            title_hint.as_deref(),
        )?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_build_cml_context(
    app: AppHandle,
    uri: String,
    title: String,
    text: String,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.build_cml_context_graph(&uri, &title, &text)?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_build_cof_package(
    app: AppHandle,
    uri: String,
    title: String,
    text: String,
    max_chars: Option<u64>,
    dual_surface: Option<bool>,
) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.build_cof_html_package(
            &uri,
            &title,
            &text,
            max_chars.map(|n| n as usize),
            dual_surface.unwrap_or(false),
        )?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_enrich_library_cml(app: AppHandle, asset_uri: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.enrich_library_entry_cml(&asset_uri)?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_list_qapp_catalog_categories(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.list_qapp_catalog_categories()?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_library_stats(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let stats = host.library_stats()?;
        serde_json::to_string(&stats).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_remove_library_entry(app: AppHandle, asset_uri: String) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.remove_library_entry(&asset_uri)?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

#[command]
pub fn wellfair_export_library_graph(app: AppHandle) -> Result<String, String> {
    let state = app.state::<HostApiState>();
    state.0.execute_sync(move |guard| {
        let host = guard
            .as_ref()
            .ok_or_else(|| "Host API not initialized — unlock Sanctuary vault first".to_string())?;
        let r = host.export_library_graph()?;
        serde_json::to_string(&r).map_err(|e| e.to_string())
    })?
}

