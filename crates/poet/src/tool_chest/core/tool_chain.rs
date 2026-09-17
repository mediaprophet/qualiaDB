//! Tool-Chain: a grouped set of related tools within a toolbox.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use super::tool::Tool;
use core::fmt;

// ---------------------------------------------------------------------------
// ToolChainMetadata
// ---------------------------------------------------------------------------

/// Static metadata describing a tool-chain.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolChainMetadata {
    /// Unique chain id — e.g. `social:connections`.
    pub id: String,
    /// Human-readable label — e.g. "Social Connections".
    pub label: String,
    /// Icon identifier.
    pub icon: String,
    /// Short description.
    pub description: String,
}

// ---------------------------------------------------------------------------
// ToolChain
// ---------------------------------------------------------------------------

/// A grouped set of related tools within a toolbox.
///
/// Example: the `social` toolbox has a `connections` tool-chain
/// containing tools for placing social graph containers, sending
/// connection requests, and viewing risk assessments.
pub struct ToolChain {
    metadata: ToolChainMetadata,
    tools: Vec<Box<dyn Tool>>,
}

impl ToolChain {
    /// Create a new tool-chain with the given metadata and tools.
    pub fn new(metadata: ToolChainMetadata, tools: Vec<Box<dyn Tool>>) -> Self {
        Self { metadata, tools }
    }

    /// Chain metadata.
    pub fn metadata(&self) -> &ToolChainMetadata {
        &self.metadata
    }

    /// All tools in this chain.
    pub fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    /// Find a tool by id.
    pub fn tool(&self, id: &str) -> Option<&dyn Tool> {
        self.tools
            .iter()
            .find(|t| t.metadata().id == id)
            .map(|b| b.as_ref())
    }

    /// Append a tool (spec swarm merge). Skip if the id is already present.
    pub fn add_tool(&mut self, tool: Box<dyn Tool>) {
        if self
            .tools
            .iter()
            .any(|existing| existing.metadata().id == tool.metadata().id)
        {
            return;
        }
        self.tools.push(tool);
    }
}

impl fmt::Debug for ToolChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolChain")
            .field("id", &self.metadata.id)
            .field("label", &self.metadata.label)
            .field("tool_count", &self.tools.len())
            .finish()
    }
}
