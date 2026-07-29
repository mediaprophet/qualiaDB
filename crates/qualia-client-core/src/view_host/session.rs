//! Session + projectors: library storage - filtered projection.

use qualia_core_db::entity_view::{
    decide_view, layout_scene_nodes, Circumstance, EntityId, EntityKind, EntityViewMeta, FlatCard,
    LayoutInput, ObserverStatus, PresentationLevel, ProjectionResult, SceneNodeProj,
    SensitivityClass,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::wellfair::hypermedia_store::{CommonsVisibility, HypermediaStore, LibraryEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MorphMode {
    Flatten,
    Spatialize,
    #[default]
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewSession {
    pub observer: ObserverStatus,
    pub presentation_level: PresentationLevel,
    pub selection: Vec<EntityId>,
    pub morph_mode: MorphMode,
    pub attention_url: Option<String>,
    /// Spatio-social-temporal circumstance (partial: design + session hooks).
    #[serde(default)]
    pub circumstance: Circumstance,
    #[serde(skip)]
    pub last_projection: Option<ProjectionResult>,
}

impl Default for ViewSession {
    fn default() -> Self {
        Self {
            observer: ObserverStatus::Principal,
            presentation_level: PresentationLevel::AppHabitat,
            selection: Vec::new(),
            morph_mode: MorphMode::Both,
            attention_url: None,
            circumstance: Circumstance::private_sanctuary(),
            last_projection: None,
        }
    }
}

fn entry_to_meta(e: &LibraryEntry) -> EntityViewMeta {
    let sens = SensitivityClass::parse(&e.sensitivity);
    let is_secret = e.is_secret() || e.section == "secret" || sens.is_high();
    let commons = matches!(
        e.commons_visibility,
        CommonsVisibility::Peers | CommonsVisibility::Commons
    ) || e.section == "commons";
    let peer = matches!(
        e.commons_visibility,
        CommonsVisibility::Peers | CommonsVisibility::Commons
    );
    EntityViewMeta {
        entity_id: EntityId::from_uri(&e.asset_uri),
        kind: EntityKind::Asset,
        sensitivity: sens,
        is_secret,
        commons_visible: commons && !is_secret,
        peer_offered: peer && !is_secret,
    }
}

fn entry_title(e: &LibraryEntry) -> String {
    let u = e.asset_uri.as_str();
    u.rsplit(['/', ':']).next().unwrap_or(u).to_string()
}

/// Project library section for observer into flat + scene nodes.
pub fn project_library_for_observer(
    storage_path: &str,
    section: Option<&str>,
    observer: ObserverStatus,
    level: PresentationLevel,
) -> Result<ProjectionResult, String> {
    let root = std::path::Path::new(storage_path);
    let store = HypermediaStore::open(root).map_err(|e| e.to_string())?;
    let entries = match section {
        Some(s) if !s.is_empty() && s != "all" => store
            .by_section(crate::wellfair::hypermedia_store::LibrarySection::parse(s))
            .map_err(|e| e.to_string())?,
        _ => store.all().map_err(|e| e.to_string())?,
    };

    let mut flat = Vec::new();
    let mut layout_in = Vec::new();
    let mut hidden = 0u32;

    for e in &entries {
        let meta = entry_to_meta(e);
        let decision = decide_view(observer, &meta);
        if !decision.visible {
            hidden += 1;
            continue;
        }
        let excerpt = e.excerpt.chars().take(160).collect::<String>();
        flat.push(FlatCard {
            entity_id: meta.entity_id.raw(),
            kind: meta.kind,
            title: entry_title(e),
            excerpt,
            wing: decision.wing,
            affordance_bits: decision.affordances.pack(),
            honesty: if e.topics.iter().any(|t| t.contains("seed")) {
                "partial".into()
            } else {
                "present".into()
            },
            uri: e.asset_uri.clone(),
        });
        layout_in.push(LayoutInput {
            entity_id: meta.entity_id,
            lat: e.lat,
            lon: e.lon,
            affordances: decision.affordances,
            wing: decision.wing,
        });
    }

    let mut scene_buf: Vec<SceneNodeProj> = (0..layout_in.len().max(1))
        .map(|_| SceneNodeProj {
            entity_id: 0,
            id: String::new(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            color: String::new(),
            radius: 0.0,
            alpha: 0.0,
            affordance_bits: 0,
        })
        .collect();
    let n = layout_scene_nodes(&layout_in, &mut scene_buf);
    scene_buf.truncate(n);

    Ok(ProjectionResult {
        observer: format!("{observer:?}").to_ascii_lowercase(),
        presentation_level: level.as_u8(),
        flat,
        scene_nodes: scene_buf,
        hidden_count: hidden,
    })
}

pub fn project_web_locus(url: &str, observer: ObserverStatus) -> serde_json::Value {
    let id = EntityId::from_uri(url);
    let meta = EntityViewMeta {
        entity_id: id,
        kind: EntityKind::WebLocus,
        sensitivity: SensitivityClass::Public,
        is_secret: false,
        commons_visible: true,
        peer_offered: true,
    };
    let d = decide_view(observer, &meta);
    json!({
        "entity_id": id.raw(),
        "kind": "web_locus",
        "uri": url,
        "visible": d.visible,
        "wing": d.wing,
        "affordance_bits": d.affordances.pack(),
        "title": url,
        "honesty": "present",
    })
}

pub fn morph_flatten(proj: &ProjectionResult) -> serde_json::Value {
    json!({
        "morph": "flatten",
        "observer": proj.observer,
        "presentation_level": proj.presentation_level,
        "flat": proj.flat,
        "hidden_count": proj.hidden_count,
    })
}

pub fn morph_spatialize(proj: &ProjectionResult) -> serde_json::Value {
    json!({
        "morph": "spatialize",
        "observer": proj.observer,
        "presentation_level": proj.presentation_level,
        "scene_nodes": proj.scene_nodes,
        "hidden_count": proj.hidden_count,
    })
}
