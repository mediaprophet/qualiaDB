//! App-wide entity-view commands (shell, studio habitat, browser - not browser-only).

#![allow(non_snake_case)]

use qualia_client_core::state::APP_STATE;
use qualia_client_core::view_host::{
    clear_selection, get_session_snapshot, morph_json, parse_observer, pick_scene_node_at,
    project_library_json, project_web_locus_json, select_entity, select_entity_uri,
    set_circumstance, set_observer, set_presentation_level,
};
use tauri::command;

fn storage_path() -> String {
    if let Some(state) = APP_STATE.get() {
        if let Ok(cfg) = state.config.lock() {
            return cfg.storage_path.clone();
        }
    }
    qualia_client_core::state::dirs_default_path()
}

/// Snapshot of app-global entity-view session (observer, selection, morph mode).
#[command]
pub fn view_session() -> Result<serde_json::Value, String> {
    let s = get_session_snapshot();
    serde_json::to_value(s).map_err(|e| e.to_string())
}

/// Set observer status: principal | peer | guardian | steward | public | instrument | auditor.
#[command]
pub fn view_set_observer(status: String) -> Result<serde_json::Value, String> {
    set_observer(parse_observer(&status));
    view_session()
}

/// Set presentation morphology level 0-6 (document - multi-sensory).
#[command]
pub fn view_set_presentation_level(level: u8) -> Result<serde_json::Value, String> {
    set_presentation_level(level);
    view_session()
}

/// Project Lived Memory library for current/default observer into flat + scene nodes.
#[command]
pub fn view_project_library(
    section: Option<String>,
    observer: Option<String>,
    presentation_level: Option<u8>,
) -> Result<serde_json::Value, String> {
    let path = storage_path();
    let obs = observer
        .as_deref()
        .map(parse_observer)
        .unwrap_or(qualia_core_db::entity_view::ObserverStatus::Principal);
    let level = presentation_level
        .map(qualia_core_db::entity_view::PresentationLevel::from_u8)
        .unwrap_or(qualia_core_db::entity_view::PresentationLevel::AppHabitat);
    project_library_json(&path, section.as_deref(), obs, level)
}

/// Project a web URL as a web-locus entity (browser + habitat shared).
#[command]
pub fn view_project_web_locus(
    url: String,
    observer: Option<String>,
) -> Result<serde_json::Value, String> {
    let obs = observer
        .as_deref()
        .map(parse_observer)
        .unwrap_or(qualia_core_db::entity_view::ObserverStatus::Principal);
    project_web_locus_json(&url, obs)
}

/// Morph last projection: flatten | spatialize | both.
#[command]
pub fn view_morph(mode: String) -> Result<serde_json::Value, String> {
    morph_json(&mode)
}

/// Pick nearest scene node from last projection (normalized x,y in 0..1).
/// Updates shared selection to that entity_id when found.
#[command]
pub fn view_pick_scene(nx: f64, ny: f64) -> Result<serde_json::Value, String> {
    let id = pick_scene_node_at(nx, ny, 0.08);
    let s = get_session_snapshot();
    Ok(serde_json::json!({
        "entity_id": id,
        "found": id.is_some(),
        "selection": s.selection.iter().map(|e| e.raw()).collect::<Vec<_>>(),
        "attention_url": s.attention_url,
    }))
}

/// Select entity by raw id (shared across shell / studio / browser).
#[command]
pub fn view_select(entity_id: u64) -> Result<serde_json::Value, String> {
    select_entity(entity_id);
    view_session()
}

/// Select entity by URI (Library asset, web locus, DID) — shared session continuity.
#[command]
pub fn view_select_uri(uri: String) -> Result<serde_json::Value, String> {
    select_entity_uri(&uri);
    view_session()
}

#[command]
pub fn view_clear_selection() -> Result<serde_json::Value, String> {
    clear_selection();
    view_session()
}

/// Build a bifurcated package digest demo from entity id lists (cold path).
#[command]
pub fn view_bifurcate_package(
    package_key: String,
    private_uris: Vec<String>,
    offered_uris: Vec<String>,
    commons_uris: Vec<String>,
) -> Result<serde_json::Value, String> {
    use qualia_core_db::entity_view::{BifurcatedPackage, EntityId};
    let priv_ids: Vec<_> = private_uris.iter().map(|u| EntityId::from_uri(u)).collect();
    let off_ids: Vec<_> = offered_uris.iter().map(|u| EntityId::from_uri(u)).collect();
    let com_ids: Vec<_> = commons_uris.iter().map(|u| EntityId::from_uri(u)).collect();
    let pkg = BifurcatedPackage::new(&package_key, &priv_ids, &off_ids, &com_ids);
    serde_json::to_value(pkg).map_err(|e| e.to_string())
}

/// Machine-readable capability report for design process (ready / partial / planned).
#[command]
pub fn view_capability_report() -> Result<serde_json::Value, String> {
    Ok(qualia_core_db::entity_view::entity_view_capability_report())
}

/// Render last library projection as prestige GPU frame (entity_view → PortalGpu PNG).
/// Emits `render-preview-ready` like other previews; frontend loads via webizen:// protocol.
#[command]
pub async fn view_render_memory_spatial(
    width: Option<u32>,
    height: Option<u32>,
    state: tauri::State<'_, crate::commands::render::PreviewState>,
    app: tauri::AppHandle,
) -> Result<serde_json::Value, String> {
    use crate::commands::render_pipeline::entity_view_to_render_scene;
    use tauri::Emitter;

    let width = width.unwrap_or(1280).max(64).min(4096);
    let height = height.unwrap_or(720).max(64).min(4096);
    let snap = get_session_snapshot();
    let proj = snap
        .last_projection
        .ok_or_else(|| "no projection yet — call view_project_library first".to_string())?;
    let selected = snap
        .selection
        .first()
        .map(|e| e.raw())
        .unwrap_or(0);

    let nodes = proj.scene_nodes.clone();
    let pick_slot = state.node_positions.clone();
    let png_slot = state.png.clone();

    let (png, node_count) = tokio::task::spawn_blocking(move || -> Result<(Vec<u8>, usize), String> {
        let scene = entity_view_to_render_scene(&nodes, selected);
        let count = nodes.len();
        let picks =
            crate::commands::render_pipeline::compute_pick_positions(&scene, width, height);
        let png = webizen_render::render_scene_png(&scene, width, height)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())?;
        if let Ok(mut g) = pick_slot.lock() {
            *g = picks;
        }
        Ok((png, count))
    })
    .await
    .map_err(|e| format!("render join: {e}"))??;

    if let Ok(mut g) = png_slot.lock() {
        *g = png;
    }
    let _ = app.emit("render-preview-ready", ());
    Ok(serde_json::json!({
        "ok": true,
        "node_count": node_count,
        "width": width,
        "height": height,
        "selected_entity_id": selected,
        "protocol": "webizen://render-preview",
        "honesty": "GPU frame from entity_view projection — not a film sequence"
    }))
}

/// Installable remote controller (phone PWA) URLs + honesty notes.
#[command]
pub fn view_remote_controller_info() -> Result<serde_json::Value, String> {
    let settings_port = crate::settings_server::current_settings_port();
    let companion_port = crate::companion_gateway::companion_listen_port();
    let lan = crate::companion_gateway::guess_lan_ipv4();
    Ok(serde_json::json!({
        "product": "Webizen Remote Surface Controller",
        "delivery": "installable_pwa_shell",
        "native_app": false,
        "urls": {
            "localhost": format!("http://127.0.0.1:{settings_port}/remote-controller/"),
            "lan": format!("http://{lan}:{companion_port}/remote-controller/"),
        },
        "honesty": {
            "install_requires_secure_origin": "Browsers install PWAs on HTTPS or localhost; plain LAN HTTP may not offer Add to Home Screen on all devices.",
            "shell_not_full_wasm_module": "Controller is an installable PWA shell talking to desktop view_* session; full wasm controller module is a later upgrade path.",
            "local_apparatus": true,
        }
    }))
}

/// Set circumstance fields (partial API — design + session; not full path-steering yet).
#[command]
pub fn view_set_circumstance(
    role: Option<String>,
    audience: Option<String>,
    quorum: Option<u8>,
    environment: Option<String>,
    evaluatory: Option<String>,
) -> Result<serde_json::Value, String> {
    use qualia_core_db::entity_view::{Circumstance, EnvironmentKind, EvaluatoryFocus};
    let mut c = get_session_snapshot().circumstance;
    if let Some(r) = role {
        c.role = r;
    }
    if let Some(a) = audience {
        c.audience = a;
    }
    if let Some(q) = quorum {
        c.quorum = q;
    }
    if let Some(env) = environment {
        c.environment = match env.trim().to_ascii_lowercase().as_str() {
            "workplace" | "work" => EnvironmentKind::Workplace,
            "public_cafe" | "cafe" | "public" => EnvironmentKind::PublicCafe,
            "clinical" | "care" => EnvironmentKind::ClinicalCare,
            "education" | "school" => EnvironmentKind::Education,
            "field" | "mobile" => EnvironmentKind::FieldMobile,
            "sanctuary" | "private" => EnvironmentKind::PrivateSanctuary,
            _ => EnvironmentKind::Unspecified,
        };
    }
    if let Some(ev) = evaluatory {
        c.evaluatory = match ev.trim().to_ascii_lowercase().as_str() {
            "care" | "care_safety" | "safety" => EvaluatoryFocus::CareSafety,
            "work" | "work_delivery" => EvaluatoryFocus::WorkDelivery,
            "learning" => EvaluatoryFocus::Learning,
            "social" | "commons" => EvaluatoryFocus::SocialCommons,
            "legal" | "legal_process" => EvaluatoryFocus::LegalProcess,
            _ => EvaluatoryFocus::Open,
        };
    }
    // Preserve preset helpers when both role+env empty strings were not used:
    if c.role.is_empty() && c.audience.is_empty() {
        c = Circumstance::private_sanctuary();
    }
    set_circumstance(c);
    view_session()
}
