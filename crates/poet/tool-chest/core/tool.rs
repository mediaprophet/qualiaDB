//! Tool trait: the smallest unit of user action in the tool-chest.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

use core::fmt;
use super::intent_bus::{ActionType, VibeScriptPayload};

// ---------------------------------------------------------------------------
// ToolKind
// ---------------------------------------------------------------------------

/// Classification of a tool's primary interaction mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    /// Places a container on the active manifold (e.g. "+ Document").
    PlaceContainer,
    /// Runs a VibeScript action on the active container or graph.
    RunAction,
    /// Queries the graph (read-only).
    Query,
    /// Navigates to a different manifold, container, or section.
    Navigate,
    /// Toggles a UI state (panel visibility, view mode).
    Toggle,
}

impl fmt::Display for ToolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolKind::PlaceContainer => f.write_str("place_container"),
            ToolKind::RunAction => f.write_str("run_action"),
            ToolKind::Query => f.write_str("query"),
            ToolKind::Navigate => f.write_str("navigate"),
            ToolKind::Toggle => f.write_str("toggle"),
        }
    }
}

// ---------------------------------------------------------------------------
// ToolMetadata
// ---------------------------------------------------------------------------

/// Static metadata describing a tool — its identity, label, icon, and
/// capability requirements.
///
/// This is the "name plate" on the tool. The actual behaviour is
/// defined by the [`Tool`] trait's `invoke` method.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ToolMetadata {
    /// Unique tool id — e.g. `social:place_social_graph`.
    pub id: String,
    /// Human-readable label — e.g. "Place Social Graph".
    pub label: String,
    /// Icon identifier (rendered by the presentation layer).
    pub icon: String,
    /// Tool interaction kind.
    pub kind: ToolKind,
    /// Capability scope required to use this tool — e.g. `graph:read`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_scope: Option<String>,
    /// Ontology prefix this tool belongs to — e.g. `soc`, `set`, `comm`.
    pub ontology_prefix: String,
    /// Short description shown in tooltips.
    pub description: String,
}

// ---------------------------------------------------------------------------
// Tool trait
// ---------------------------------------------------------------------------

/// A single tool — the smallest unit of user action in the tool-chest.
///
/// Tools are picked up by the user to place containers, run actions,
/// query the graph, or navigate. Each tool emits a [`VibeScriptPayload`]
/// through the [`IntentBus`](super::intent_bus::IntentBus).
///
/// Tools are registered in a [`ToolChain`](super::tool_chain::ToolChain)
/// which is registered in a [`Toolbox`](super::toolbox::Toolbox).
pub trait Tool: Send + Sync {
    /// Static metadata for this tool.
    fn metadata(&self) -> &ToolMetadata;

    /// The action type this tool emits when invoked.
    fn action_type(&self) -> ActionType;

    /// Build a VibeScript payload for this tool's action.
    ///
    /// The `params` argument is a serialisable parameter struct specific
    /// to this tool. The tool wraps it in a [`VibeScriptPayload`] with
    /// the correct action type and target.
    fn build_payload<P>(
        &self,
        target: super::intent_bus::TargetIdentifier,
        params: P,
    ) -> VibeScriptPayload<P>
    where
        P: serde::Serialize + Send + Sync,
    {
        VibeScriptPayload::new(self.action_type(), target, params)
            .with_context(format!("https://qualiadb.org/schema/ui/{}#", self.metadata().ontology_prefix))
    }
}

// ---------------------------------------------------------------------------
// SimpleTool — a concrete tool implementation for declarative tools
// ---------------------------------------------------------------------------

/// A simple, declarative tool that carries its metadata and action type
/// as fields. Most tools in the tool-chest are this kind — they declare
/// what they do and the presentation layer handles the rest.
#[derive(Clone, Debug)]
pub struct SimpleTool {
    meta: ToolMetadata,
    action: ActionType,
}

impl SimpleTool {
    /// Create a new simple tool.
    pub fn new(meta: ToolMetadata, action: ActionType) -> Self {
        Self { meta, action }
    }
}

impl Tool for SimpleTool {
    fn metadata(&self) -> &ToolMetadata {
        &self.meta
    }

    fn action_type(&self) -> ActionType {
        self.action
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::intent_bus::TargetIdentifier;

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    struct TestParams {
        edge_type: String,
    }

    #[test]
    fn simple_tool_construction() {
        let tool = SimpleTool::new(
            ToolMetadata {
                id: "social:place_social_graph".into(),
                label: "Place Social Graph".into(),
                icon: "graph".into(),
                kind: ToolKind::PlaceContainer,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "soc".into(),
                description: "Place a social graph container.".into(),
            },
            ActionType::Query,
        );

        assert_eq!(tool.metadata().id, "social:place_social_graph");
        assert_eq!(tool.action_type(), ActionType::Query);
    }

    #[test]
    fn tool_builds_payload() {
        let tool = SimpleTool::new(
            ToolMetadata {
                id: "social:query_edges".into(),
                label: "Query Edges".into(),
                icon: "query".into(),
                kind: ToolKind::Query,
                capability_scope: Some("graph:read".into()),
                ontology_prefix: "soc".into(),
                description: "Query social edges.".into(),
            },
            ActionType::Query,
        );

        let payload = tool.build_payload(
            TargetIdentifier::iri("https://qualiadb.org/graph/social"),
            TestParams { edge_type: "friendship".into() },
        );

        assert_eq!(payload.action_type, ActionType::Query);
        assert_eq!(payload.parameters.edge_type, "friendship");
    }
}
