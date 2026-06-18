//! SHACL Module
//!
//! This module contains the SHACL compiler and type definitions.

pub mod shacl_types;
pub mod shacl_compiler;
pub mod shacl_extension_bridge;

// Re-export for convenience
pub use shacl_types::{
    ShaclSeverity, ProteinScoringMatrix, ClinicalRiskModel, CalcComputeTarget,
    NodeKindType, PropertyPath, ShaclTarget, ValidationReport, ValidationResult,
    ShaclConstraint, CompiledShape
};
pub use shacl_compiler::ShaclCompiler;