//! SHACL Module
//!
//! This module contains the SHACL compiler and type definitions.

pub mod shacl_types;
pub mod shacl_compiler;
pub mod shacl_extension_bridge;
pub mod validate;
pub mod text_input;

// Re-export for convenience
pub use shacl_types::{
    ShaclSeverity, ProteinScoringMatrix, ClinicalRiskModel, CalcComputeTarget,
    NodeKindType, PropertyPath, ShaclTarget, ValidationReport, ValidationResult,
    ShaclConstraint, CompiledShape
};
pub use shacl_compiler::ShaclCompiler;
pub use validate::ShaclEngine;
pub use text_input::{build_graph, validate_json, ConstraintSpec, ShapeSpec};