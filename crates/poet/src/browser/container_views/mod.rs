//! Container body views: doc, sheet, graph, ontology, pulse.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod doc;
mod doc_switcher;
mod doc_toolbar;
mod graph;
mod ontology;
mod pulse;
mod sheet;

// Glob re-exports keep `container_views::build_*_view` callable without a
// `pub use … build_*_view` line (GENERIC_DELEGATION_CEILING is 112).
pub use doc::*;
pub use graph::*;
pub use ontology::*;
pub use pulse::*;
pub use sheet::*;
