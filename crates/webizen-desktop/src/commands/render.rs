//! GPU render preview

#![allow(non_snake_case)]

use super::*;
use tauri::{command, AppHandle, Emitter, Manager, State};

// ── GPU render preview ──────────────────────────────────────────────────────────

/// Shared slot holding the latest rendered preview PNG. Served by the
/// `webizen://localhost/render/preview.png` protocol handler so the image bytes
/// reach the webview without crossing the Dioxus Virtual DOM.
#[derive(Default, Clone)]
pub struct PreviewState {
    pub png: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    /// Track node positions for picking interaction: (id, x, y, radius)
    pub node_positions: std::sync::Arc<std::sync::Mutex<Vec<(String, f64, f64, f64)>>>,
}

/// Shared slot holding the latest rendered 3D Anatomy body snapshot PNG. Served by the
/// `webizen://localhost/anatomy/body.png` protocol handler. The Studio UI bumps the epoch (query-string
/// cache-buster) after each `wellfair_render_body_snapshot` call so the webview refetches.
#[derive(Default, Clone)]
pub struct AnatomyBodyState {
    pub png: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
}

/// Atomic flag controlling the render daemon loop.
/// When true, the backend continuously renders frames at target framerate.
/// When false, the loop stops, enabling energy-aware rendering.
#[derive(Clone)]
pub struct RenderLoopState(pub std::sync::Arc<std::sync::atomic::AtomicBool>);

/// Active anchor node for graph navigation focus.
/// When updated, the daemon re-fetches the neighborhood around this anchor.
#[derive(Clone)]
pub struct ActiveAnchor(pub std::sync::Arc<std::sync::Mutex<Option<String>>>);

/// Mock QualiaDB projection for testing the rendering pipeline.
/// In production, this would query actual QualiaDB data.
/// Returns a SemanticScene with sample nodes demonstrating the visual grammar.
#[allow(dead_code)]
fn mock_qualia_projection() -> webizen_studio::render::qualia::SemanticScene {
    use webizen_studio::render::qualia::{ItemState, SceneItem};

    webizen_studio::render::qualia::SemanticScene {
        items: vec![
            // Person entity (blue, medium weight)
            SceneItem {
                id: "person-alice".to_string(),
                state: ItemState::Active,
                intensity: 0.7,
                provenance: Some("q42:abc123".to_string()),
                reasons: vec!["Core contributor".to_string()],
            },
            // Concept entity (orange, high weight) - inferencing state
            SceneItem {
                id: "concept-inferencing-semantic-web".to_string(),
                state: ItemState::Highlighted,
                intensity: 0.9,
                provenance: Some("q42:def456".to_string()),
                reasons: vec!["Central topic - actively inferencing".to_string()],
            },
            // Document entity (green, low weight)
            SceneItem {
                id: "document-spec".to_string(),
                state: ItemState::Default,
                intensity: 0.4,
                provenance: Some("q42:ghi789".to_string()),
                reasons: vec!["Reference material".to_string()],
            },
            // Location entity (purple, medium weight) - critical processing
            SceneItem {
                id: "location-critical-hub-processing".to_string(),
                state: ItemState::Alert,
                intensity: 0.6,
                provenance: Some("q42:jkl012".to_string()),
                reasons: vec!["Network node - critical processing".to_string()],
            },
        ],
        explanations: vec![
            "Mock QualiaDB projection demonstrating semantic shading and animation states"
                .to_string(),
        ],
    }
}

/// Fetch local neighborhood from QualiaDB using NQuin queries.
/// Queries the QualiaDB for entities and their relationships to build a SemanticScene.
#[allow(dead_code)]
fn fetch_local_neighborhood(
    qualia_db_path: &str,
) -> Result<webizen_studio::render::qualia::SemanticScene, String> {
    use qualia_core_db::{
        q_hash,
        query_engine::{mmap_query_subject, mmap_sample_quins},
    };
    use webizen_studio::render::qualia::{ItemState, SceneItem};

    let quins = if std::path::Path::new(qualia_db_path).is_file() {
        mmap_sample_quins(qualia_db_path, 64)
            .ok()
            .filter(|q| !q.is_empty())
            .or_else(|| {
                mmap_query_subject(qualia_db_path, q_hash("webizen:render:root"))
                    .ok()
                    .filter(|q| !q.is_empty())
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    if quins.is_empty() {
        return Ok(mock_qualia_projection());
    }

    // Convert NQuin results to SemanticScene
    let mut items = Vec::new();

    for quin in &quins {
        // Extract entity information from NQuin
        // This is a simplified mapping - in production would use proper lexicon lookup
        let entity_type = match quin.predicate {
            p if p == q_hash("rdf:type") => "entity",
            p if p == q_hash("schema:Person") => "person",
            p if p == q_hash("schema:Concept") => "concept",
            p if p == q_hash("schema:Document") => "document",
            p if p == q_hash("schema:Location") => "location",
            _ => "generic",
        };

        let id = format!("{}-{}", entity_type, quin.object);
        let state = match quin.metadata & 0xF {
            0 => ItemState::Default,
            1 => ItemState::Active,
            2 => ItemState::Highlighted,
            3 => ItemState::Alert,
            _ => ItemState::Default,
        };

        let intensity = ((quin.object % 100) as f64) / 100.0;

        items.push(SceneItem {
            id,
            state,
            intensity,
            provenance: Some(format!("q42:{:x}", quin.subject)),
            reasons: vec![format!("Queried from QualiaDB context {:x}", quin.context)],
        });
    }

    let entity_count = items.len();

    Ok(webizen_studio::render::qualia::SemanticScene {
        items,
        explanations: vec![format!(
            "Live QualiaDB projection from {} ({} entities)",
            qualia_db_path, entity_count
        )],
    })
}

/// Navigate to a specific node in the graph.
/// Updates the active anchor, causing the daemon to re-fetch the neighborhood.
#[command]
pub async fn navigate_to_node(
    node_id: String,
    active_anchor: State<'_, ActiveAnchor>,
) -> Result<(), String> {
    let mut anchor = active_anchor
        .0
        .lock()
        .map_err(|e| format!("Failed to lock anchor: {}", e))?;
    *anchor = Some(node_id);
    Ok(())
}

/// Select a node by screen coordinates for interaction.
/// Returns the node ID if a hit is found, None otherwise.
#[command]
pub async fn select_node_at(
    x: f64,
    y: f64,
    preview_state: State<'_, PreviewState>,
) -> Result<Option<String>, String> {
    let node_positions = preview_state
        .node_positions
        .lock()
        .map_err(|e| format!("Failed to lock node positions: {}", e))?;

    // Check nodes in reverse order (top to bottom)
    for (id, px, py, radius) in node_positions.iter().rev() {
        let dx = x - px;
        let dy = y - py;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance <= *radius {
            return Ok(Some(id.clone()));
        }
    }

    Ok(None)
}
/// When active, the backend continuously renders frames at target framerate
/// and broadcasts events to the UI. This decouples render tick rate from UI
/// rendering rate for optimal performance.
#[command]
pub async fn toggle_render_loop(
    is_active: bool,
    loop_state: State<'_, RenderLoopState>,
    _active_anchor: State<'_, ActiveAnchor>,
    _temporal_slice: State<'_, TemporalSlice>,
    _preview_state: State<'_, PreviewState>,
    _app: AppHandle,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    loop_state.0.store(is_active, Ordering::SeqCst);
    Ok(())
}

/// Render the headless GPU preview, store the PNG in shared state, and notify the
/// frontend via the `render-preview-ready` event. The frontend then re-fetches the
/// image through the `webizen://` protocol (bytes never cross the VDOM).
///
/// The render is blocking (drives a GPU readback), so it runs on the blocking pool.
#[command]
pub async fn update_render_preview(
    width: u32,
    height: u32,
    panes: Option<Vec<render_pipeline::StudioPaneInput>>,
    state: State<'_, PreviewState>,
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let width = width.max(64).min(4096);
    let height = height.max(64).min(4096);

    let storage_path = app_state
        .config
        .lock()
        .map_err(|e| format!("config lock poisoned: {e}"))?
        .storage_path
        .clone();
    let qualia_db_path = std::path::PathBuf::from(&storage_path)
        .join("Index")
        .join("graph.q42")
        .to_string_lossy()
        .to_string();

    let workspace_panes = panes.unwrap_or_default();
    let pick_slot = state.node_positions.clone();

    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let semantic = fetch_local_neighborhood(&qualia_db_path)?;
        let mut render_scene = render_pipeline::semantic_to_render_scene(&semantic);
        render_pipeline::merge_workspace_panes(&mut render_scene, &workspace_panes);
        let picks = render_pipeline::compute_pick_positions(&render_scene, width, height);

        let png = webizen_render::render_scene_png(&render_scene, width, height)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())?;

        if let Ok(mut guard) = pick_slot.lock() {
            *guard = picks;
        }

        Ok(png)
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    if let Ok(mut guard) = state.png.lock() {
        *guard = png;
    }

    let _ = app.emit("render-preview-ready", ());
    Ok(())
}

/// Background daemon tick — same pipeline as [`update_render_preview`], for spatial view loops.
pub async fn render_preview_tick(app: &AppHandle) -> Result<(), String> {
    let preview = app
        .try_state::<PreviewState>()
        .ok_or_else(|| "PreviewState not mounted".to_string())?;
    let app_state = app
        .try_state::<std::sync::Arc<qualia_client_core::state::AppState>>()
        .ok_or_else(|| "AppState not mounted".to_string())?;

    let width = 960u32;
    let height = 540u32;
    let storage_path = app_state
        .config
        .lock()
        .map_err(|e| format!("config lock poisoned: {e}"))?
        .storage_path
        .clone();
    let qualia_db_path = std::path::PathBuf::from(&storage_path)
        .join("Index")
        .join("graph.q42")
        .to_string_lossy()
        .to_string();

    let pick_slot = preview.node_positions.clone();
    let png = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let semantic = fetch_local_neighborhood(&qualia_db_path)?;
        let render_scene = render_pipeline::semantic_to_render_scene(&semantic);
        let picks = render_pipeline::compute_pick_positions(&render_scene, width, height);
        let png = webizen_render::render_scene_png(&render_scene, width, height)
            .ok_or_else(|| "GPU render_scene_png returned no frame".to_string())?;
        if let Ok(mut guard) = pick_slot.lock() {
            *guard = picks;
        }
        Ok(png)
    })
    .await
    .map_err(|e| format!("render task join failed: {e}"))??;

    if let Ok(mut guard) = preview.png.lock() {
        *guard = png;
    }
    let _ = app.emit("render-preview-ready", ());
    Ok(())
}

