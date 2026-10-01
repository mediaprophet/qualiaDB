//! SHACL Module
//!
//! This module contains the SHACL compiler and type definitions.

pub mod shacl_compiler;
pub mod shacl_extension_bridge;
pub mod shacl_types;
pub mod text_input;
pub mod turtle_shapes;
pub mod validate;

// Re-export for convenience
pub use shacl_compiler::ShaclCompiler;
pub use shacl_types::{
    CalcComputeTarget, ClinicalRiskModel, CompiledShape, NodeKindType, PropertyPath,
    ProteinScoringMatrix, ShaclConstraint, ShaclSeverity, ShaclTarget, ValidationReport,
    ValidationResult,
};
pub use text_input::{
    build_graph, shape_from_spec, validate_json, validate_text, ConstraintSpec, ShapeSpec,
};
pub use turtle_shapes::{shapes_from_turtle, validate_turtle_shapes};
pub use validate::ShaclEngine;
