//! Tool-Chest Core: traits, registry, and ontology loading.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! This module re-exports all core types and traits for the tool-chest.
//! Phase 2 of the tool-chest architecture.

pub mod intent_bus;
pub mod manifest;
pub mod ontology;
pub mod registry;
pub mod tool;
pub mod tool_chain;
pub mod toolbox;

pub use intent_bus::{
    ActionType, ContextRef, IntentBus, IntentReceipt, IntentStatus,
    Provenance, TargetIdentifier, TargetKind, VibeScriptPayload,
};
pub use manifest::{Manifest, ManifestChain, ManifestTool};
pub use ontology::{OntologyClass, OntologyModule, OntologyProperty, OntologyRegistry};
pub use registry::{DockPosition, ManifoldSeed, Registry, SeedContainer, SeedPanel};
pub use tool::{SimpleTool, Tool, ToolKind, ToolMetadata};
pub use tool_chain::{ToolChain, ToolChainMetadata};
pub use toolbox::{Toolbox, ToolboxMetadata};
