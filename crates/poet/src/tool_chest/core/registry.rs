//! Registry: the central registry for toolboxes, manifolds, and ontologies.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::construct::ConstructSeed;
use super::ontology::OntologyRegistry;
use super::sociality::{ManifoldParticipant, ManifoldSociality};
use super::toolbox::Toolbox;
use core::fmt;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// The central tool-chest registry.
///
/// Holds all installed toolboxes, loaded ontology modules, and
/// manifold definitions. The registry is initialised at startup
/// and remains static for the session (toolboxes may be enabled
/// or disabled but not hot-loaded in the initial implementation).
pub struct Registry {
    toolboxes: Vec<Toolbox>,
    ontologies: OntologyRegistry,
    manifolds: Vec<ManifoldSeed>,
    constructs: Vec<ConstructSeed>,
}

impl Registry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            toolboxes: Vec::new(),
            ontologies: OntologyRegistry::new(),
            manifolds: Vec::new(),
            constructs: Vec::new(),
        }
    }

    /// Register a toolbox.
    pub fn register_toolbox(&mut self, toolbox: Toolbox) {
        self.toolboxes.push(toolbox);
    }

    /// Register a manifold seed.
    pub fn register_manifold(&mut self, seed: ManifoldSeed) {
        self.manifolds.push(seed);
    }

    /// Access the ontology registry.
    pub fn ontologies(&self) -> &OntologyRegistry {
        &self.ontologies
    }

    /// Mutable access to the ontology registry.
    pub fn ontologies_mut(&mut self) -> &mut OntologyRegistry {
        &mut self.ontologies
    }

    /// All registered toolboxes.
    pub fn toolboxes(&self) -> &[Toolbox] {
        &self.toolboxes
    }

    /// Find a toolbox by id.
    pub fn toolbox(&self, id: &str) -> Option<&Toolbox> {
        self.toolboxes.iter().find(|t| t.metadata().id == id)
    }

    /// Mutable toolbox lookup (spec swarm merge).
    pub fn toolbox_mut(&mut self, id: &str) -> Option<&mut Toolbox> {
        self.toolboxes.iter_mut().find(|t| t.metadata().id == id)
    }

    /// All registered manifold seeds.
    pub fn manifolds(&self) -> &[ManifoldSeed] {
        &self.manifolds
    }

    /// Find a manifold seed by id.
    pub fn manifold(&self, id: &str) -> Option<&ManifoldSeed> {
        self.manifolds.iter().find(|m| m.id == id)
    }

    /// Register a construct seed.
    pub fn register_construct(&mut self, seed: ConstructSeed) {
        self.constructs.push(seed);
    }

    /// All registered constructs.
    pub fn constructs(&self) -> &[ConstructSeed] {
        &self.constructs
    }

    /// Find a construct seed by id.
    pub fn construct(&self, id: &str) -> Option<&ConstructSeed> {
        self.constructs.iter().find(|c| c.id == id)
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field("toolbox_count", &self.toolboxes.len())
            .field("manifold_count", &self.manifolds.len())
            .field("ontology_count", &self.ontologies.modules().len())
            .field("construct_count", &self.constructs.len())
            .finish()
    }
}

// ---------------------------------------------------------------------------
// ManifoldSeed
// ---------------------------------------------------------------------------

/// A manifold seed — the initial layout for a work surface.
///
/// Defines which containers are placed on the manifold when it is
/// first opened, and which panels are docked where.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ManifoldSeed {
    /// Unique manifold id — e.g. `social`, `settings`, `communications`.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Icon identifier.
    pub icon: String,
    /// Ontology prefix this manifold primarily uses.
    pub ontology_prefix: String,
    /// Short description.
    pub description: String,
    /// Containers to place when the manifold is first opened.
    pub containers: Vec<SeedContainer>,
    /// Connections (wires) between containers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<SeedConnection>,
    /// Panels to dock when the manifold is first opened.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<SeedPanel>,
    /// Personal (one observer) or social (many people). Default personal.
    #[serde(default)]
    pub sociality: ManifoldSociality,
    /// People and agents on a social lens. Empty on personal manifolds.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<ManifoldParticipant>,
}

impl ManifoldSeed {
    pub fn is_social(&self) -> bool {
        self.sociality.is_social() || super::sociality::bundled_social_manifold(&self.id)
    }
}

/// Container kind — discriminates content, panel, and widget containers.
/// Aligns with `container:ContainerKind` in `ontologies/container.n3`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerKind {
    /// Content containers hold media (documents, sheets, maps, code, 3D, audio).
    Content,
    /// Panel containers hold UI chrome (inspectors, property sheets, outlines, tool palettes).
    Panel,
    /// Widget containers are small UI elements (mini-map, status bar, badges, indicators).
    Widget,
}

impl Default for ContainerKind {
    fn default() -> Self {
        Self::Content
    }
}

impl ContainerKind {
    /// Infer the kind from a container_type string.
    /// Falls back to Content for unknown types.
    pub fn from_type(container_type: &str) -> Self {
        match container_type {
            // Panel containers
            "checkpoint-tray"
            | "credential-inspector"
            | "context-markup-editor"
            | "provenance-panel"
            | "publication-workflow"
            | "constituency-manager"
            | "inspector"
            | "property-sheet"
            | "outline"
            | "tool-palette"
            | "aura-tray"
            | "pulse-panel"
            | "graph-panel"
            | "wire-inspector"
            | "world-outliner"
            | "physics-inspector"
            | "fixture-patch"
            | "cue-stack" => Self::Panel,

            // Widget containers
            "capability-badge"
            | "checkpoint-indicator"
            | "consent-indicator"
            | "mini-map"
            | "status-bar"
            | "breadcrumb"
            | "progress-indicator" => Self::Widget,

            // Everything else is content
            _ => Self::Content,
        }
    }

    /// CSS class suffix for the kind.
    pub fn class_suffix(&self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Panel => "panel",
            Self::Widget => "widget",
        }
    }
}

/// A container placed by a manifold seed.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeedContainer {
    /// Stable identity used by wires, history, duplication, and persistence.
    /// Older manifests omit this field and are normalised when loaded.
    #[serde(default)]
    pub id: String,
    /// Container type — e.g. `social`, `settings`, `pulse`.
    pub container_type: String,
    /// Container kind — `content`, `panel`, or `widget`.
    /// Inferred from `container_type` if not specified.
    #[serde(default)]
    pub kind: ContainerKind,
    /// Container title.
    pub title: String,
    /// Initial position (x, y) in manifold coordinates.
    pub x: f32,
    pub y: f32,
    /// Initial size (width, height).
    pub width: f32,
    pub height: f32,
    /// Z-order.
    #[serde(default = "default_z")]
    pub z: f32,
    /// Honesty label — `live`, `partial`, `present`, `missing`.
    #[serde(default = "default_honesty")]
    pub honesty: String,
    /// Optional semantic type/URI assigned through the container inspector.
    #[serde(default)]
    pub semantic_type: String,
    #[serde(default)]
    pub semantic_uri: String,
    /// Editable document markup owned by this container.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub content_html: String,
    /// Tool Chest settings keyed by their stable widget id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tool_settings: BTreeMap<String, String>,
    /// User-editable state captured from the specialist container view.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub view_state: BTreeMap<String, String>,
    /// Nested-manifold portal: switch the pager to this manifold.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_manifold: String,
    /// Construct portal: open this construct (optional `target_manifold` inside it).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_construct: String,
}

impl SeedContainer {
    /// Create a new container with the kind inferred from its type.
    pub fn new(container_type: &str, title: &str, x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            id: String::new(),
            container_type: container_type.to_string(),
            kind: ContainerKind::from_type(container_type),
            title: title.to_string(),
            x,
            y,
            width: w,
            height: h,
            z: default_z(),
            honesty: default_honesty(),
            semantic_type: String::new(),
            semantic_uri: String::new(),
            content_html: String::new(),
            tool_settings: BTreeMap::new(),
            view_state: BTreeMap::new(),
            target_manifold: String::new(),
            target_construct: String::new(),
        }
    }
}

fn default_z() -> f32 {
    100.0
}

fn default_honesty() -> String {
    "missing".into()
}

impl Default for SeedContainer {
    fn default() -> Self {
        Self {
            id: String::default(),
            container_type: String::default(),
            kind: ContainerKind::default(),
            title: String::default(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            z: default_z(),
            honesty: default_honesty(),
            semantic_type: String::default(),
            semantic_uri: String::default(),
            content_html: String::default(),
            tool_settings: BTreeMap::default(),
            view_state: BTreeMap::default(),
            target_manifold: String::default(),
            target_construct: String::default(),
        }
    }
}

/// A connection (wire) between two containers on a manifold.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeedConnection {
    /// Connection id.
    #[serde(default)]
    pub id: String,
    /// Source container index (into `containers`).
    pub from: usize,
    /// Target container index (into `containers`).
    pub to: usize,
    /// Wire type — e.g. `active`, `event`, `ontology`, `subjective`, `objective`.
    #[serde(default = "default_wire_type")]
    pub wire_type: String,
    /// Predicate label shown on the wire midpoint.
    pub label: String,
}

fn default_wire_type() -> String {
    "active".into()
}

/// A panel docked by a manifold seed.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SeedPanel {
    /// Panel type — e.g. `inspector`, `graph-panel`, `pulse-panel`.
    pub panel_type: String,
    /// Dock position.
    pub dock: DockPosition,
}

/// Dock position for panels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockPosition {
    Left,
    Right,
    Top,
    Bottom,
    Float,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_construction() {
        let mut reg = Registry::new();
        reg.register_manifold(ManifoldSeed {
            id: "social".into(),
            label: "Social".into(),
            icon: "users".into(),
            ontology_prefix: "soc".into(),
            description: "Social manifold".into(),
            containers: vec![SeedContainer {
                container_type: "social".into(),
                title: "Social Graph".into(),
                x: 100.0,
                y: 70.0,
                width: 420.0,
                height: 320.0,
                z: 100.0,
                honesty: "live".into(),
                ..Default::default()
            }],
            connections: vec![],
            panels: vec![SeedPanel {
                panel_type: "pulse-panel".into(),
                dock: DockPosition::Bottom,
            }],
            sociality: ManifoldSociality::Social,
            ..Default::default()
        });

        assert_eq!(reg.manifolds().len(), 1);
        assert!(reg.manifold("social").is_some());
        assert!(reg.manifold("nonexistent").is_none());
    }

    #[test]
    fn container_authoring_state_roundtrips() {
        let mut container = SeedContainer::new("doc", "Draft", 10.0, 20.0, 400.0, 300.0);
        container.id = "container-draft-1".into();
        container.content_html = "<p>A retained draft</p>".into();
        container.tool_settings.insert(
            "office:typography:font_family".into(),
            "Merriweather".into(),
        );
        container
            .view_state
            .insert("clinical-risk-age".into(), "value:52".into());

        let json = serde_json::to_string(&container).unwrap();
        let restored: SeedContainer = serde_json::from_str(&json).unwrap();

        assert_eq!(restored, container);
    }

    #[test]
    fn legacy_container_manifest_defaults_new_state_fields() {
        let json = r#"{
            "container_type":"doc","title":"Legacy","x":0.0,"y":0.0,
            "width":400.0,"height":300.0
        }"#;
        let restored: SeedContainer = serde_json::from_str(json).unwrap();

        assert!(restored.id.is_empty());
        assert!(restored.content_html.is_empty());
        assert!(restored.tool_settings.is_empty());
        assert!(restored.view_state.is_empty());
    }
}
