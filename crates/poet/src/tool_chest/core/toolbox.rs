//! Toolbox: a removable drawer of related tool-chains.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::tool_chain::ToolChain;
use core::fmt;

// ---------------------------------------------------------------------------
// ToolboxMetadata
// ---------------------------------------------------------------------------

/// Static metadata describing a toolbox.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolboxMetadata {
    /// Unique toolbox id — e.g. `social`, `settings`, `communications`.
    pub id: String,
    /// Human-readable label — e.g. "Social Toolbox".
    pub label: String,
    /// Icon identifier.
    pub icon: String,
    /// Ontology prefix this toolbox uses.
    pub ontology_prefix: String,
    /// Short description.
    pub description: String,
    /// Whether this toolbox is enabled by default.
    #[serde(default = "default_true")]
    pub enabled_by_default: bool,
    /// Family group this toolbox belongs to — e.g. "epistemic", "office",
    /// "media", "spatial", "communication", "rights", "health", "code",
    /// "ai", "graph". Toolboxes in the same family are grouped in the dock.
    #[serde(default)]
    pub family: String,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// Toolbox
// ---------------------------------------------------------------------------

/// A toolbox — a themed drawer inside the tool-chest.
///
/// Each toolbox is a self-contained plugin that can be added, removed,
/// or updated independently. It groups related tool-chains.
///
/// Example: the `social` toolbox contains `graph`, `connections`,
/// `communities`, and `reputation` tool-chains.
pub struct Toolbox {
    metadata: ToolboxMetadata,
    chains: Vec<ToolChain>,
}

impl Toolbox {
    /// Create a new toolbox.
    pub fn new(metadata: ToolboxMetadata, chains: Vec<ToolChain>) -> Self {
        Self { metadata, chains }
    }

    /// Toolbox metadata.
    pub fn metadata(&self) -> &ToolboxMetadata {
        &self.metadata
    }

    /// All tool-chains in this toolbox.
    pub fn chains(&self) -> &[ToolChain] {
        &self.chains
    }

    /// Find a tool-chain by id.
    pub fn chain(&self, id: &str) -> Option<&ToolChain> {
        self.chains.iter().find(|c| c.metadata().id == id)
    }

    /// Append a chain (spec swarm merge).
    pub fn add_chain(&mut self, chain: ToolChain) {
        self.chains.push(chain);
    }

    /// Mutable chain lookup (spec swarm merge).
    pub fn chain_mut(&mut self, id: &str) -> Option<&mut ToolChain> {
        self.chains.iter_mut().find(|c| c.metadata().id == id)
    }
}

impl fmt::Debug for Toolbox {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Toolbox")
            .field("id", &self.metadata.id)
            .field("label", &self.metadata.label)
            .field("chain_count", &self.chains.len())
            .finish()
    }
}
