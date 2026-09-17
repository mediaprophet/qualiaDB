//! Registry: the central registry for toolboxes, manifolds, and ontologies.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use core::fmt;
use super::ontology::OntologyRegistry;
use super::toolbox::Toolbox;

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
}

impl Registry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            toolboxes: Vec::new(),
            ontologies: OntologyRegistry::new(),
            manifolds: Vec::new(),
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

    /// All registered manifold seeds.
    pub fn manifolds(&self) -> &[ManifoldSeed] {
        &self.manifolds
    }

    /// Find a manifold seed by id.
    pub fn manifold(&self, id: &str) -> Option<&ManifoldSeed> {
        self.manifolds.iter().find(|m| m.id == id)
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
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    /// Panels to dock when the manifold is first opened.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub panels: Vec<SeedPanel>,
}

/// A container placed by a manifold seed.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeedContainer {
    /// Container type — e.g. `social`, `settings`, `pulse`.
    pub container_type: String,
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
}

fn default_z() -> f32 {
    100.0
}

fn default_honesty() -> String {
    "missing".into()
}

/// A panel docked by a manifold seed.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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
            }],
            panels: vec![SeedPanel {
                panel_type: "pulse-panel".into(),
                dock: DockPosition::Bottom,
            }],
        });

        assert_eq!(reg.manifolds().len(), 1);
        assert!(reg.manifold("social").is_some());
        assert!(reg.manifold("nonexistent").is_none());
    }
}
