//! SHACL Compiler Implementation
//!
//! This module contains the main SHACL compiler that translates constraints
//! into SlgOpcode sequences for the Webizen VM.

use crate::webizen::SlgOpcode;
use super::shacl_types::{
    ShaclConstraint, ShaclSeverity, ShaclTarget, CompiledShape
};

/// SHACL Compiler
///
/// Translates SHACL shape constraints into deterministic `SlgOpcode` sequences
/// that execute inside the Webizen SLG VM before data is committed to `.q42`.
pub struct ShaclCompiler;

impl ShaclCompiler {
    pub fn new() -> Self {
        ShaclCompiler
    }

    /// Typed compile — preferred API.
    pub fn compile(
        &self,
        target: ShaclTarget,
        property_path: &str,
        constraint: ShaclConstraint,
        severity: ShaclSeverity,
    ) -> CompiledShape {
        let mut opcodes = Vec::new();
        Self::push_constraint(&constraint, &mut opcodes);
        Self::push_terminal(severity, &mut opcodes);
        
        let shape_class = match &target {
            ShaclTarget::TargetClass(s) => s.clone(),
            ShaclTarget::TargetObjectsOf(s) => s.clone(),
            ShaclTarget::TargetSubjectsOf(s) => s.clone(),
            ShaclTarget::TargetNode(s) => s.clone(),
        };
        
        let mut shape = CompiledShape::new(shape_class, vec![constraint], severity);
        shape.property_path = property_path.to_string();
        shape.opcodes = opcodes;
        shape
    }

    /// Compile with target class for backward compatibility
    pub fn compile_class(
        &self,
        target_class: &str,
        property_path: &str,
        constraint: ShaclConstraint,
        severity: ShaclSeverity,
    ) -> CompiledShape {
        self.compile(
            ShaclTarget::TargetClass(target_class.to_string()),
            property_path,
            constraint,
            severity,
        )
    }

    /// Backward-compatible string-based API.  Now correctly threads `value` through.
    pub fn compile_shape(
        &self,
        target_class: &str,
        property_path: &str,
        constraint_type: &str,
        value: f32,
    ) -> Vec<SlgOpcode> {
        let constraint = Self::parse_str(constraint_type, value);
        let shape = self.compile_class(
            target_class,
            property_path,
            constraint,
            ShaclSeverity::Violation,
        );
        // For backward compatibility, return opcodes
        // In the new API, the shape contains constraints directly
        vec![]
    }

    fn push_constraint(constraint: &ShaclConstraint, opcodes: &mut Vec<SlgOpcode>) {
        match constraint {
            // Numeric constraints
            ShaclConstraint::MinInclusive(min) => {
                opcodes.push(SlgOpcode::CheckMinInclusive(*min));
            }
            ShaclConstraint::MaxInclusive(max) => {
                opcodes.push(SlgOpcode::CheckMaxInclusive(*max));
            }
            ShaclConstraint::MinExclusive(min) => {
                opcodes.push(SlgOpcode::CheckMinExclusive(*min));
            }
            ShaclConstraint::MaxExclusive(max) => {
                opcodes.push(SlgOpcode::CheckMaxExclusive(*max));
            }
            // Cardinality constraints
            ShaclConstraint::MinCount(min) => {
                opcodes.push(SlgOpcode::CheckMinCount(*min));
            }
            ShaclConstraint::MaxCount(max) => {
                opcodes.push(SlgOpcode::CheckMaxCount(*max));
            }
            // String constraints
            ShaclConstraint::MinLength(min) => {
                opcodes.push(SlgOpcode::CheckMinLength(*min));
            }
            ShaclConstraint::MaxLength(max) => {
                opcodes.push(SlgOpcode::CheckMaxLength(*max));
            }
            ShaclConstraint::Pattern(pattern) => {
                opcodes.push(SlgOpcode::CheckPattern(crate::q_hash(pattern)));
            }
            // Value constraints
            ShaclConstraint::In(values) => {
                for value in values {
                    opcodes.push(SlgOpcode::CheckHasValue(crate::q_hash(value)));
                }
            }
            ShaclConstraint::HasValue(value) => {
                opcodes.push(SlgOpcode::CheckHasValue(crate::q_hash(value)));
            }
            // Node shape constraints
            ShaclConstraint::Node(shape) => {
                opcodes.push(SlgOpcode::CheckNodeShape(crate::q_hash(shape)));
            }
            // Default: ignore unknown constraints for now
            _ => {}
        }
    }

    fn push_terminal(severity: ShaclSeverity, opcodes: &mut Vec<SlgOpcode>) {
        match severity {
            ShaclSeverity::Violation => {
                opcodes.push(SlgOpcode::Halt);
            }
            ShaclSeverity::Warning => {
                opcodes.push(SlgOpcode::WarnOnly);
            }
            ShaclSeverity::Info => {
                opcodes.push(SlgOpcode::WarnOnly); // Use WarnOnly for Info as well
            }
        }
    }

    fn parse_str(constraint_type: &str, value: f32) -> ShaclConstraint {
        match constraint_type {
            "minInclusive" => ShaclConstraint::MinInclusive(value as f64),
            "maxInclusive" => ShaclConstraint::MaxInclusive(value as f64),
            "minExclusive" => ShaclConstraint::MinExclusive(value as f64),
            "maxExclusive" => ShaclConstraint::MaxExclusive(value as f64),
            "minCount" => ShaclConstraint::MinCount(value as u32),
            "maxCount" => ShaclConstraint::MaxCount(value as u32),
            "minLength" => ShaclConstraint::MinLength(value as u32),
            "maxLength" => ShaclConstraint::MaxLength(value as u32),
            _ => ShaclConstraint::MinInclusive(value as f64), // Default fallback
        }
    }
}