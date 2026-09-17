//! VibeScript metamodel registry and semantic projection (Prompt Precision P1A).
//!
//! Emits neutral RDF statements / Turtle — does **not** depend on `qualia-core-db`.

mod export;
mod fields;
mod namespace;
mod node_kinds;
mod project;
mod validate;

pub use export::{export_rdfs_turtle, export_shacl_turtle, ExportBudget};
pub use fields::{property_iri, FieldDescriptor, FIELD_DESCRIPTORS};
pub use namespace::{
    LANGUAGE_VERSION_IRI, VIBE_NS, VIBE_SCHEMA_VERSION,
};
pub use node_kinds::{
    kind_iri, NodeKindDescriptor, EFFECT_KINDS, EXPR_KINDS, ITEM_KINDS, MODAL_KINDS, PATTERN_KINDS, STMT_KINDS,
    TYPE_KINDS,
};
pub use project::{project_program_summary, SemanticStatement, StatementKind};
pub use validate::{coverage_report, CoverageReport, CoverageStatus};
