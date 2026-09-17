//! Search workbench — a modal overlay with four search modes:
//!
//! 1. **Faceted Search** — filter by ontology prefix, entity type,
//!    epistemic modality, strata, honesty, and container type. Shows
//!    result count and matching live canvas containers.
//! 2. **Visual Query Builder** — build SPARQL queries by adding
//!    triple patterns (subject, predicate, object) via UI controls.
//!    Generates a SPARQL SELECT query that can be inspected, edited,
//!    or saved.
//! 3. **Manual SPARQL** — write or edit a SPARQL query directly in a
//!    textarea. Supports loading saved queries, editing them, and
//!    saving new ones.
//! 4. **Saved Queries** — list, load, place, and delete persisted queries.
//!
//! Saved queries are persisted in localStorage as named objects with
//! metadata (name, mode, query text, facets, timestamp). Saved queries
//! can be used as container content sources — placing a "query-results"
//! container that retains the query definition. SPARQL execution requires the
//! QualiaDB daemon; the UI does not fabricate an offline result set.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod builder;
mod catalog;
mod faceted;
mod persist;
mod placement;
mod saved;
mod shell;
mod sparql;

pub use shell::{
    build_search_workbench, open_to_mode, toggle_search_workbench, wire_search_workbench_shortcut,
};
