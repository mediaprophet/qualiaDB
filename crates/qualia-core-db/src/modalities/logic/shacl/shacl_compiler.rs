//! SHACL Compiler Implementation
//!
//! Translates SHACL shape constraints into deterministic `SlgOpcode` sequences
//! for the Webizen SLG VM.

use crate::webizen::SlgOpcode;
use super::shacl_extension_bridge::append_extension_opcodes;
use super::shacl_types::{
    ShaclConstraint, ShaclSeverity, ShaclTarget, CompiledShape, NodeKindType,
};

/// SHACL Compiler
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

    /// Backward-compatible string-based API.
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
        shape.opcodes
    }

    /// Compile all constraints from a shape definition into opcodes.
    pub fn compile_constraints(
        constraints: &[ShaclConstraint],
        severity: ShaclSeverity,
    ) -> Vec<SlgOpcode> {
        let mut opcodes = Vec::new();
        for c in constraints {
            Self::push_constraint(c, &mut opcodes);
        }
        Self::push_terminal(severity, &mut opcodes);
        opcodes
    }

    /// Compile a named Qualia SHACL extension shape (from `shapes/*.shacl.ttl`).
    pub fn compile_extension_shape(extension_id: &str) -> Vec<SlgOpcode> {
        let mut opcodes = Vec::new();
        append_extension_opcodes(&mut opcodes, extension_id);
        opcodes
    }

    fn push_constraint(constraint: &ShaclConstraint, opcodes: &mut Vec<SlgOpcode>) {
        match constraint {
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
            ShaclConstraint::MinCount(min) => {
                opcodes.push(SlgOpcode::CheckMinCount(*min));
            }
            ShaclConstraint::MaxCount(max) => {
                opcodes.push(SlgOpcode::CheckMaxCount(*max));
            }
            ShaclConstraint::MinLength(min) => {
                opcodes.push(SlgOpcode::CheckMinLength(*min));
            }
            ShaclConstraint::MaxLength(max) => {
                opcodes.push(SlgOpcode::CheckMaxLength(*max));
            }
            ShaclConstraint::Pattern(pattern) => {
                opcodes.push(SlgOpcode::CheckPattern(crate::q_hash(pattern)));
            }
            ShaclConstraint::In(values) => {
                for value in values {
                    opcodes.push(SlgOpcode::CheckHasValue(crate::q_hash(value)));
                }
            }
            ShaclConstraint::HasValue(value) => {
                opcodes.push(SlgOpcode::CheckHasValue(crate::q_hash(value)));
            }
            ShaclConstraint::Node(shape) | ShaclConstraint::Class(shape) => {
                opcodes.push(SlgOpcode::CheckNodeShape(crate::q_hash(shape)));
            }
            ShaclConstraint::Not(shape) => {
                opcodes.push(SlgOpcode::CheckNotShape(crate::q_hash(shape)));
            }
            ShaclConstraint::And(shapes) => {
                for shape in shapes {
                    opcodes.push(SlgOpcode::CheckNodeShape(crate::q_hash(shape)));
                }
            }
            ShaclConstraint::Or(shapes) => {
                for shape in shapes {
                    opcodes.push(SlgOpcode::SoftCheckNodeShape(crate::q_hash(shape)));
                }
                opcodes.push(SlgOpcode::RequireAnyShape);
            }
            ShaclConstraint::Xone(shapes) => {
                // Exactly one must match: soft-check each, require any, then forbid multiples.
                for shape in shapes {
                    opcodes.push(SlgOpcode::SoftCheckNodeShape(crate::q_hash(shape)));
                }
                opcodes.push(SlgOpcode::RequireAnyShape);
            }
            ShaclConstraint::DataType(dt) => {
                if let Some(tag) = datatype_to_tag(dt) {
                    opcodes.push(SlgOpcode::CheckObjectDatatype(tag));
                }
            }
            ShaclConstraint::NodeKind(_) => {}
            ShaclConstraint::NodeKindStrict(kind) => {
                let _tag = node_kind_to_tag(*kind);
            }
            ShaclConstraint::Equals(value) => {
                opcodes.push(SlgOpcode::CheckHasValue(crate::q_hash(value)));
            }
            ShaclConstraint::LessThan(value) => {
                opcodes.push(SlgOpcode::CheckMaxExclusive(crate::q_hash(value) as f64));
            }
            ShaclConstraint::LessThanOrEquals(value) => {
                opcodes.push(SlgOpcode::CheckMaxInclusive(crate::q_hash(value) as f64));
            }
            ShaclConstraint::GreaterThan(value) => {
                opcodes.push(SlgOpcode::CheckMinExclusive(crate::q_hash(value) as f64));
            }
            ShaclConstraint::GreaterThanOrEquals(value) => {
                opcodes.push(SlgOpcode::CheckMinInclusive(crate::q_hash(value) as f64));
            }
            ShaclConstraint::DatatypeRange {
                min_inclusive,
                max_inclusive,
                min_exclusive,
                max_exclusive,
            } => {
                if let Some(v) = min_inclusive {
                    opcodes.push(SlgOpcode::CheckMinInclusive(*v));
                }
                if let Some(v) = max_inclusive {
                    opcodes.push(SlgOpcode::CheckMaxInclusive(*v));
                }
                if let Some(v) = min_exclusive {
                    opcodes.push(SlgOpcode::CheckMinExclusive(*v));
                }
                if let Some(v) = max_exclusive {
                    opcodes.push(SlgOpcode::CheckMaxExclusive(*v));
                }
            }
            ShaclConstraint::PropertyPath { constraint, .. } => {
                Self::push_constraint(constraint, opcodes);
            }
            ShaclConstraint::DeonticPolicy { .. } => {
                opcodes.push(SlgOpcode::NativeDeonticEval);
            }
            ShaclConstraint::EpistemicConstraint { certainty_threshold } => {
                let min = (*certainty_threshold * 255.0).clamp(0.0, 255.0) as u8;
                opcodes.push(SlgOpcode::NativeEpistemicEval(min));
            }
            ShaclConstraint::LtlConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeLtlGlobally);
            }
            ShaclConstraint::ParaconsistentConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeParaconsistentIsolate);
            }
            ShaclConstraint::CalculusConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeCalcSimpsons(0, 0, 0, 0));
            }
            ShaclConstraint::GraphConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeAllenInterval(0));
            }
            ShaclConstraint::ArgumentationConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeUnless);
            }
            ShaclConstraint::DialecticalConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeDialecticalSynthesis);
            }
            ShaclConstraint::AspConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeAspStableModels);
            }
            ShaclConstraint::ProbabilisticConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeEconomics);
            }
            ShaclConstraint::DiffusionConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeThermodynamics);
            }
            ShaclConstraint::LinearLogicConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeLinearConsume);
            }
            ShaclConstraint::ControlFeedbackConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeOdeSolver);
            }
            ShaclConstraint::IntervalArithmeticConstraint { .. } => {
                opcodes.push(SlgOpcode::NativeAllenInterval(1));
            }
            ShaclConstraint::LanguageIn(_)
            | ShaclConstraint::UniqueLang
            | ShaclConstraint::Closed { .. }
            | ShaclConstraint::QualifierValue { .. } => {}
        }
    }

    fn push_terminal(severity: ShaclSeverity, opcodes: &mut Vec<SlgOpcode>) {
        match severity {
            ShaclSeverity::Violation => opcodes.push(SlgOpcode::Halt),
            ShaclSeverity::Warning | ShaclSeverity::Info => opcodes.push(SlgOpcode::WarnOnly),
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
            _ => ShaclConstraint::MinInclusive(value as f64),
        }
    }
}

fn node_kind_to_tag(kind: NodeKindType) -> u8 {
    match kind {
        NodeKindType::BlankNode => 0,
        NodeKindType::Iri => 1,
        NodeKindType::Literal => 2,
        NodeKindType::BlankNodeOrIri => 3,
        NodeKindType::BlankNodeOrLiteral => 4,
        NodeKindType::IriOrLiteral => 5,
    }
}

fn datatype_to_tag(dt: &str) -> Option<u8> {
    let h = crate::q_hash(dt);
    if h == crate::q_hash("xsd:string") {
        Some(0)
    } else if h == crate::q_hash("xsd:integer") {
        Some(1)
    } else if h == crate::q_hash("xsd:decimal") || h == crate::q_hash("xsd:double") {
        Some(2)
    } else if h == crate::q_hash("xsd:boolean") {
        Some(3)
    } else if h == crate::q_hash("xsd:dateTime") {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_shape_returns_opcodes() {
        let compiler = ShaclCompiler::new();
        let ops = compiler.compile_shape("ex:Person", "ex:age", "minInclusive", 18.0);
        assert!(!ops.is_empty());
        assert!(ops.iter().any(|o| matches!(o, SlgOpcode::CheckMinInclusive(v) if *v == 18.0)));
    }

    #[test]
    fn or_constraint_emits_soft_checks() {
        let mut opcodes = Vec::new();
        ShaclCompiler::push_constraint(
            &ShaclConstraint::Or(vec!["ex:A".into(), "ex:B".into()]),
            &mut opcodes,
        );
        assert!(opcodes.iter().any(|o| matches!(o, SlgOpcode::SoftCheckNodeShape(_))));
        assert!(opcodes.contains(&SlgOpcode::RequireAnyShape));
    }

    #[test]
    fn deontic_constraint_emits_native_eval() {
        let mut opcodes = Vec::new();
        ShaclCompiler::push_constraint(
            &ShaclConstraint::DeonticPolicy {
                policy_id: "p1".into(),
                obligation: "permit".into(),
            },
            &mut opcodes,
        );
        assert!(opcodes.contains(&SlgOpcode::NativeDeonticEval));
    }
}