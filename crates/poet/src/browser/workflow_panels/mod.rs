//! Workflow panel container views — checkpoint tray, credential inspector,
//! context markup editor, provenance panel, publication workflow,
//! constituency manager, and widget indicators.
//!
//! These are panel and widget containers (per `ontologies/container.n3`)
//! that surface the save/publication/credential workflow described in
//! `SAVE_ARCHITECTURE.md`.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod checkpoint;
mod constituency;
mod credentials;
mod markup;
mod provenance;
mod publication;
mod widgets;

// Glob re-exports keep `workflow_panels::build_*_view` callable without a
// `pub use … build_*_view` line (GENERIC_DELEGATION_CEILING is 112).
pub use checkpoint::*;
pub use constituency::*;
pub use credentials::*;
pub use markup::*;
pub use provenance::*;
pub use publication::*;
pub use widgets::*;
