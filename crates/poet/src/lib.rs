//! Webizen Poet — Tool-Chest, Manifolds, and Browser Preview.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)

pub mod tool_chest {
    pub mod constructs;
    pub mod core;
    pub mod manifolds;
    pub mod payloads;
}

pub mod browser;

/// Frozen `vibe-host-0.1` four-op surface (G-A).
pub mod vibe_host;

pub use tool_chest::core::{
    build_payload, ActionType, ConstructSeed, ConstructSource, ContextRef, DockPosition, IntentBus,
    IntentReceipt, IntentStatus, Manifest, ManifestChain, ManifestTool, ManifoldParticipant,
    ManifoldSeed, ManifoldSociality, OntologyClass, OntologyModule, OntologyProperty,
    OntologyRegistry, Provenance, Registry, SeedConnection, SeedContainer, SeedPanel, SimpleTool,
    SubjectSeed, TargetIdentifier, TargetKind, Tool, ToolChain, ToolChainMetadata, ToolKind,
    ToolMetadata, Toolbox, ToolboxMetadata, VibeScriptPayload,
};

/// Thin vibe-host-0.1 facade — prefer these over reaching into `vibe` AST/Host.
pub use vibe_host::{
    capability_invoke, check_cell, check_program, diagnose, diagnose_json, host_version,
    language_version, parse_cell, parse_program, CRATE_STAMP, HOST_VERSION, LANGUAGE_VERSION,
};
