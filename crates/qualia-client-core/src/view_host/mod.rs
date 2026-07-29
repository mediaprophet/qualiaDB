//! App-global entity-view session composition (desktop-wide, not browser-only).
//!
//! Pure filter/layout from `qualia_core_db::entity_view`; this module binds storage + session.

mod session;

pub use session::{
    morph_flatten, morph_spatialize, project_library_for_observer, project_web_locus, MorphMode,
    ViewSession,
};

use qualia_core_db::entity_view::{Circumstance, EntityId, ObserverStatus, PresentationLevel};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Process-wide session for webizen-desktop (shell, studio, browser share one).
static SESSION: Mutex<Option<ViewSession>> = Mutex::new(None);

pub fn with_session<R>(f: impl FnOnce(&mut ViewSession) -> R) -> R {
    let mut g = SESSION.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        *g = Some(ViewSession::default());
    }
    f(g.as_mut().expect("session just set"))
}

pub fn get_session_snapshot() -> ViewSession {
    with_session(|s| s.clone())
}

pub fn set_observer(status: ObserverStatus) {
    with_session(|s| s.observer = status);
}

pub fn set_presentation_level(level: u8) {
    with_session(|s| s.presentation_level = PresentationLevel::from_u8(level));
}

pub fn select_entity(entity_id: u64) {
    with_session(|s| {
        s.selection.clear();
        s.selection.push(EntityId::from_raw(entity_id));
    });
}

/// Select by URI/DID/asset (stable EntityId via from_uri); also sets attention_url.
pub fn select_entity_uri(uri: &str) {
    let id = EntityId::from_uri(uri);
    with_session(|s| {
        s.selection.clear();
        s.selection.push(id);
        s.attention_url = Some(uri.to_string());
    });
}

pub fn clear_selection() {
    with_session(|s| {
        s.selection.clear();
        s.attention_url = None;
    });
}

/// Set circumstance (role / audience / quorum / environment / evaluatory).
pub fn set_circumstance(c: Circumstance) {
    with_session(|s| s.circumstance = c);
}

/// DTO for Tauri / studio JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectLibraryRequest {
    pub storage_path: String,
    pub section: Option<String>,
    pub observer: Option<String>,
    pub presentation_level: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectWebLocusRequest {
    pub url: String,
    pub observer: Option<String>,
}

pub fn parse_observer(s: &str) -> ObserverStatus {
    match s.trim().to_ascii_lowercase().as_str() {
        "peer" => ObserverStatus::Peer,
        "guardian" => ObserverStatus::Guardian,
        "steward" => ObserverStatus::Steward,
        "public" | "anonymous" => ObserverStatus::Public,
        "instrument" | "agent" => ObserverStatus::Instrument,
        "auditor" => ObserverStatus::Auditor,
        _ => ObserverStatus::Principal,
    }
}

/// Project library from disk path for given observer (updates session).
pub fn project_library_json(
    storage_path: &str,
    section: Option<&str>,
    observer: ObserverStatus,
    level: PresentationLevel,
) -> Result<serde_json::Value, String> {
    with_session(|s| {
        s.observer = observer;
        s.presentation_level = level;
        let result = project_library_for_observer(storage_path, section, observer, level)?;
        s.last_projection = Some(result.clone());
        s.morph_mode = MorphMode::Both;
        serde_json::to_value(&result).map_err(|e| e.to_string())
    })
}

pub fn project_web_locus_json(
    url: &str,
    observer: ObserverStatus,
) -> Result<serde_json::Value, String> {
    with_session(|s| {
        s.observer = observer;
        let card = project_web_locus(url, observer);
        if let Some(id) = card.get("entity_id").and_then(|v| v.as_u64()) {
            s.selection.clear();
            s.selection.push(EntityId::from_raw(id));
        }
        s.attention_url = Some(url.to_string());
        Ok(card)
    })
}

pub fn morph_json(mode: &str) -> Result<serde_json::Value, String> {
    with_session(|s| {
        let Some(ref proj) = s.last_projection else {
            return Err("no projection yet - call view_project_library first".into());
        };
        match mode.trim().to_ascii_lowercase().as_str() {
            "flatten" | "flat" => {
                s.morph_mode = MorphMode::Flatten;
                Ok(morph_flatten(proj))
            }
            "spatialize" | "spatial" | "scene" => {
                s.morph_mode = MorphMode::Spatialize;
                Ok(morph_spatialize(proj))
            }
            "both" => {
                s.morph_mode = MorphMode::Both;
                serde_json::to_value(proj).map_err(|e| e.to_string())
            }
            _ => Err(format!(
                "unknown morph mode '{mode}' (flatten|spatialize|both)"
            )),
        }
    })
}

/// Nearest scene node in last projection by normalized (x,y) in 0..1 (controller pick).
/// Returns entity_id when found; also updates selection.
pub fn pick_scene_node_at(nx: f64, ny: f64, max_dist: f64) -> Option<u64> {
    with_session(|s| {
        let proj = s.last_projection.as_ref()?;
        let mut best: Option<(f64, u64)> = None;
        for n in &proj.scene_nodes {
            if n.entity_id == 0 {
                continue;
            }
            let dx = n.x - nx;
            let dy = n.y - ny;
            let d = (dx * dx + dy * dy).sqrt();
            if d <= max_dist {
                if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                    best = Some((d, n.entity_id));
                }
            }
        }
        let id = best.map(|(_, id)| id)?;
        s.selection.clear();
        s.selection.push(EntityId::from_raw(id));
        Some(id)
    })
}
