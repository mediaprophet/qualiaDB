//! N3Logic → SHACL → Sentinel Bytecode compiler (CogAI symbolic layer).
//!
//! The LLM emits N3 assertions on the cold path; this module validates them against
//! compiled SHACL shapes from [`crate::shacl_compiler`] and lowers surviving rules to
//! [`SlgOpcode`] sequences for the Core-1 Webizen VM. Hot-path execution uses only
//! fixed caller-supplied buffers.

use crate::n3_parser::{Formula, Rule, RuleType, Term, Triple};
use crate::q_hash;
use crate::shacl_compiler::{CompiledShape, ShaclCompiler, ShaclConstraint, ShaclSeverity};
use crate::webizen::{execute_vm_frame, SlgArena, SlgOpcode, VmFrame};
use crate::QualiaQuin;

pub const MAX_COMPILED_OPCODES: usize = 256;
pub const MAX_COMPILED_QUINS: usize = 64;
pub const MAX_INTENT_SCOPE_SLOTS: usize = 16;
pub const MAX_CONTEXT_NAMESPACE_SLOTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum N3OutputMode {
    FreeText,
    N3Assertions,
    GraphMutation,
    SummarizeOnly,
}

impl Default for N3OutputMode {
    fn default() -> Self {
        Self::FreeText
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum N3CompileError {
    EmptyRule,
    MalformedTriple,
    UnsupportedRuleType,
    ShapeViolation,
    OpcodeBufferFull,
    QuinBufferFull,
    SentinelMemoryOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentinelError {
    MemoryOverflow,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct N3CompiledProgram {
    pub opcode_count: usize,
    pub quin_count: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AgentIntentFrame {
    pub intent_predicate: u64,
    pub principal_did_hash: u64,
    pub mcp_intent_frame_hash: u64,
    pub ilp_offer_micro_cents: u64,
    pub scope_count: u8,
    pub context_namespace_count: u8,
    pub requires_network: bool,
    pub output_mode: N3OutputMode,
    pub clearance_ceiling: u8,
    pub max_sentinel_depth: u8,
    pub graph_scope: [u64; MAX_INTENT_SCOPE_SLOTS],
    pub context_namespaces: [u64; MAX_CONTEXT_NAMESPACE_SLOTS],
}

fn term_hash(term: &Term) -> Result<u64, N3CompileError> {
    match term {
        Term::Uri(uri) => Ok(q_hash(uri)),
        Term::Literal(lit) => Ok(q_hash(lit)),
        Term::Variable(_) => Err(N3CompileError::MalformedTriple),
    }
}

fn triple_to_quin(triple: &Triple, context: u64) -> Result<QualiaQuin, N3CompileError> {
    let mut quin = QualiaQuin::default();
    quin.subject = term_hash(&triple.subject)?;
    quin.predicate = term_hash(&triple.predicate)?;
    quin.object = term_hash(&triple.object)?;
    quin.context = context;
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
    Ok(quin)
}

fn first_triple(formula: &Formula) -> Result<&Triple, N3CompileError> {
    formula
        .triples
        .first()
        .ok_or(N3CompileError::MalformedTriple)
}

/// Returns true when every conclusion triple property path matches a compiled SHACL shape.
pub fn validate_rule_against_shapes(
    rule: &Rule,
    shapes: &[&CompiledShape],
) -> Result<(), N3CompileError> {
    if shapes.is_empty() {
        return Ok(());
    }
    let conclusion = first_triple(&rule.conclusion)?;
    let property_hash = term_hash(&conclusion.predicate)?;
    let mut matched = false;
    for shape in shapes {
        if q_hash(&shape.property_path) == property_hash {
            matched = true;
            if let Term::Literal(lit) = &conclusion.object {
                if let Ok(value) = lit.parse::<f64>() {
                    if !shape.evaluate_numeric(value) {
                        return Err(N3CompileError::ShapeViolation);
                    }
                }
            }
        }
    }
    if matched {
        Ok(())
    } else {
        Err(N3CompileError::ShapeViolation)
    }
}

fn push_opcode(
    out: &mut [SlgOpcode],
    count: &mut usize,
    opcode: SlgOpcode,
) -> Result<(), N3CompileError> {
    if *count >= out.len() {
        return Err(N3CompileError::OpcodeBufferFull);
    }
    out[*count] = opcode;
    *count += 1;
    Ok(())
}

/// Lower one N3 rule into Sentinel opcodes (reuses SHACL terminal semantics).
pub fn compile_rule_to_opcodes(
    rule: &Rule,
    out: &mut [SlgOpcode],
) -> Result<usize, N3CompileError> {
    let mut count = 0usize;
    match rule.rule_type {
        RuleType::Strict => {
            push_opcode(out, &mut count, SlgOpcode::Unify)?;
            push_opcode(out, &mut count, SlgOpcode::Call)?;
            push_opcode(out, &mut count, SlgOpcode::Halt)?;
        }
        RuleType::Defeasible => {
            push_opcode(out, &mut count, SlgOpcode::CheckDefeaters)?;
            push_opcode(out, &mut count, SlgOpcode::Unify)?;
            push_opcode(out, &mut count, SlgOpcode::Call)?;
            push_opcode(out, &mut count, SlgOpcode::WarnOnly)?;
        }
        RuleType::Defeater => {
            push_opcode(out, &mut count, SlgOpcode::NativeUnless)?;
            push_opcode(out, &mut count, SlgOpcode::Halt)?;
        }
        RuleType::Linear => {
            push_opcode(out, &mut count, SlgOpcode::NativeLinearConsume)?;
            push_opcode(out, &mut count, SlgOpcode::Unify)?;
            push_opcode(out, &mut count, SlgOpcode::Call)?;
            push_opcode(out, &mut count, SlgOpcode::Halt)?;
        }
    }
    Ok(count)
}

pub fn compile_rule_to_quin(
    rule: &Rule,
    contract_hash: u64,
    out: &mut [QualiaQuin],
) -> Result<usize, N3CompileError> {
    if let Some(norm) = crate::deontic_logic::compile_n3_rule_to_norm(rule, contract_hash, 0) {
        if out.is_empty() {
            return Err(N3CompileError::QuinBufferFull);
        }
        out[0] = norm;
        return Ok(1);
    }

    let mut count = 0usize;
    let triples = [&rule.premise, &rule.conclusion];
    for formula in triples {
        if let Ok(triple) = first_triple(formula) {
            if count >= out.len() {
                return Err(N3CompileError::QuinBufferFull);
            }
            out[count] = triple_to_quin(triple, contract_hash)?;
            count += 1;
        }
    }
    if count == 0 {
        return Err(N3CompileError::EmptyRule);
    }
    Ok(count)
}

/// SHACL-gated batch compile: validate each rule, then emit opcodes into a fixed buffer.
pub fn compile_rules_with_shacl_gate(
    rules: &[Rule],
    shapes: &[&CompiledShape],
    opcodes_out: &mut [SlgOpcode],
    quins_out: &mut [QualiaQuin],
    contract_hash: u64,
) -> Result<N3CompiledProgram, N3CompileError> {
    let mut opcode_offset = 0usize;
    let mut quin_offset = 0usize;

    for rule in rules {
        validate_rule_against_shapes(rule, shapes)?;
        let written = compile_rule_to_opcodes(rule, &mut opcodes_out[opcode_offset..])?;
        opcode_offset += written;

        let quins_written =
            compile_rule_to_quin(rule, contract_hash, &mut quins_out[quin_offset..])?;
        quin_offset += quins_written;
    }

    Ok(N3CompiledProgram {
        opcode_count: opcode_offset,
        quin_count: quin_offset,
    })
}

/// Execute compiled opcodes inside the 42 MB `SlgArena` without heap growth in the eval loop.
pub fn execute_compiled_program(
    arena: &mut SlgArena,
    opcodes: &[SlgOpcode],
    frame: &mut VmFrame,
    max_depth: u8,
) -> Result<Option<QualiaQuin>, SentinelError> {
    if opcodes.len() > max_depth as usize {
        return Err(SentinelError::MemoryOverflow);
    }
    Ok(execute_vm_frame(arena, opcodes, frame))
}

/// Build a default health-observation SHACL gate for LLM-emitted N3 (cold path helper).
pub fn default_observation_shape() -> CompiledShape {
    ShaclCompiler::new().compile(
        "fhir:Observation",
        "health:restingHeartRate",
        ShaclConstraint::MinInclusive(20.0),
        ShaclSeverity::Violation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::n3_parser::{Formula, Rule, RuleType, Triple};

    fn sample_strict_rule() -> Rule {
        Rule {
            id: Some("hr-observation".into()),
            rule_type: RuleType::Strict,
            weight: None,
            premise: Formula {
                triples: vec![Triple {
                    subject: Term::Uri("ex:Patient1".into()),
                    predicate: Term::Uri("health:restingHeartRate".into()),
                    object: Term::Literal("72".into()),
                }],
            },
            conclusion: Formula {
                triples: vec![Triple {
                    subject: Term::Uri("ex:Patient1".into()),
                    predicate: Term::Uri("health:restingHeartRate".into()),
                    object: Term::Literal("72".into()),
                }],
            },
        }
    }

    #[test]
    fn compiles_strict_rule_to_opcodes() {
        let mut opcodes = [SlgOpcode::Call; MAX_COMPILED_OPCODES];
        let count = compile_rule_to_opcodes(&sample_strict_rule(), &mut opcodes).unwrap();
        assert_eq!(count, 3);
        assert_eq!(opcodes[0], SlgOpcode::Unify);
        assert_eq!(opcodes[1], SlgOpcode::Call);
        assert_eq!(opcodes[2], SlgOpcode::Halt);
    }

    #[test]
    fn shacl_gate_rejects_out_of_range_numeric() {
        let mut rule = sample_strict_rule();
        rule.conclusion.triples[0].object = Term::Literal("12".into());
        let shape = default_observation_shape();
        let shapes = [&shape];
        assert_eq!(
            validate_rule_against_shapes(&rule, &shapes),
            Err(N3CompileError::ShapeViolation)
        );
    }

    #[test]
    fn zero_heap_compile_rules_with_shacl_gate() {
        let _profiler = dhat::Profiler::builder().testing().build();
        let rules = [sample_strict_rule()];
        let shape = default_observation_shape();
        let shapes = [&shape];
        let mut opcodes = [SlgOpcode::Call; MAX_COMPILED_OPCODES];
        let mut quins = [QualiaQuin::default(); MAX_COMPILED_QUINS];

        let result = compile_rules_with_shacl_gate(
            &rules,
            &shapes,
            &mut opcodes,
            &mut quins,
            q_hash("did:test:contract"),
        );
        assert!(result.is_ok());
        assert!(result.unwrap().opcode_count > 0);

        let stats = dhat::HeapStats::get();
        assert_eq!(
            stats.curr_blocks, 0,
            "compile_rules_with_shacl_gate must not allocate"
        );
        assert_eq!(stats.curr_bytes, 0);
    }
}
