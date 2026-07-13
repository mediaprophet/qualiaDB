//! SHACL Module
//!
//! This module contains the SHACL compiler and type definitions.

pub mod shacl_compiler;
pub mod shacl_extension_bridge;
pub mod shacl_types;
pub mod text_input;
pub mod validate;

// Re-export for convenience
pub use shacl_compiler::ShaclCompiler;
pub use shacl_types::{
    CalcComputeTarget, ClinicalRiskModel, CompiledShape, NodeKindType, PropertyPath,
    ProteinScoringMatrix, ShaclConstraint, ShaclSeverity, ShaclTarget, ValidationReport,
    ValidationResult,
};
pub use text_input::{build_graph, validate_json, ConstraintSpec, ShapeSpec};
pub use validate::ShaclEngine;
