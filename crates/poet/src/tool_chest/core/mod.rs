//! Tool-Chest Core: traits, registry, and ontology loading.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! This module re-exports all core types and traits for the tool-chest.
//! Phase 2 of the tool-chest architecture.

pub mod construct;
pub mod device;
pub mod intent_bus;
pub mod manifest;
pub mod ontology;
pub mod registry;
pub mod sociality;
pub mod subject;
pub mod tool;
pub mod tool_chain;
pub mod toolbox;
pub mod workspace;

pub use construct::{ConstructSeed, ConstructSource};
pub use device::{DeviceCaps, DeviceProfile, DeviceStatus, DeviceType, DisplayInfo};
pub use intent_bus::{
    ActionType, ContextRef, IntentBus, IntentReceipt, IntentStatus, Provenance, TargetIdentifier,
    TargetKind, VibeScriptPayload,
};
pub use manifest::{Manifest, ManifestChain, ManifestTool};
pub use ontology::{OntologyClass, OntologyModule, OntologyProperty, OntologyRegistry};
pub use registry::{
    DockPosition, ManifoldSeed, Registry, SeedConnection, SeedContainer, SeedPanel,
};
pub use sociality::{ManifoldParticipant, ManifoldSociality};
pub use subject::SubjectSeed;
pub use tool::{build_payload, SimpleTool, Tool, ToolKind, ToolMetadata};
pub use tool_chain::{ToolChain, ToolChainMetadata};
pub use toolbox::{Toolbox, ToolboxMetadata};
pub use workspace::{
    ContainerOverride, DeviceAssignment, DeviceRole, WorkspaceDelta, WorkspaceState,
};
