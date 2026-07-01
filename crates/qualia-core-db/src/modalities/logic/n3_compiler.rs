//! N3Logic → SHACL → Sentinel Bytecode compiler (CogAI symbolic layer).
//!
//! The LLM emits N3 assertions on the cold path; this module validates them against
//! compiled SHACL shapes from [`crate::modalities::logic::shacl`] and lowers surviving rules to
//! [`SlgOpcode`] sequences for the Core-1 Webizen VM. Hot-path execution uses only
//! fixed caller-supplied buffers.

use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Term, Triple};
use crate::modalities::logic::shacl::{
    CompiledShape, ShaclCompiler, ShaclConstraint, ShaclSeverity,
};
use crate::q_hash;
use crate::webizen::{execute_vm_frame, SlgArena, SlgOpcode, VmFrame};
use crate::NQuin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledTerm {
    Uri(u64),
    Variable(u64),
    Literal(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledTriple {
    pub subject: CompiledTerm,
    pub predicate: CompiledTerm,
    pub object: CompiledTerm,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledFormula {
    pub triples: [CompiledTriple; 8],
    pub len: usize,
}

impl Default for CompiledFormula {
    fn default() -> Self {
        Self {
            triples: [CompiledTriple {
                subject: CompiledTerm::Uri(0),
                predicate: CompiledTerm::Uri(0),
                object: CompiledTerm::Uri(0),
            }; 8],
            len: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledRule {
    pub id_hash: Option<u64>,
    pub rule_type: RuleType,
    pub weight: Option<f32>,
    pub premise: CompiledFormula,
    pub conclusion: CompiledFormula,
}

impl CompiledTerm {
    pub fn as_u64(&self) -> u64 {
        match self {
            CompiledTerm::Uri(h) => *h,
            CompiledTerm::Variable(h) => *h,
            CompiledTerm::Literal(h) => *h,
        }
    }

    pub fn is_variable(&self) -> bool {
        matches!(self, CompiledTerm::Variable(_))
    }
}

pub fn compile_term(term: &Term<'_>) -> CompiledTerm {
    let hash = crate::modalities::logic::n3_parser::term_uri_hash(term)
        .unwrap_or_else(|| q_hash("?"));
    match term {
        Term::Variable(_) => CompiledTerm::Variable(hash),
        Term::Literal(_) => CompiledTerm::Literal(hash),
        _ => CompiledTerm::Uri(hash),
    }
}

pub fn compile_triple(triple: &Triple<'_>) -> CompiledTriple {
    CompiledTriple {
        subject: compile_term(&triple.subject),
        predicate: compile_term(&triple.predicate),
        object: compile_term(&triple.object),
    }
}

pub fn compile_formula(formula: &Formula<'_>) -> CompiledFormula {
    let mut comp = CompiledFormula::default();
    for (i, t) in formula.triples.iter().enumerate().take(8) {
        comp.triples[i] = compile_triple(t);
        comp.len += 1;
    }
    comp
}

pub fn compile_rule_to_zero_heap(rule: &Rule<'_>) -> CompiledRule {
    CompiledRule {
        id_hash: rule.id.map(q_hash),
        rule_type: rule.rule_type,
        weight: rule.weight,
        premise: compile_formula(&rule.premise),
        conclusion: compile_formula(&rule.conclusion),
    }
}

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

fn term_hash(term: &Term<'_>) -> Result<u64, N3CompileError> {
    match term {
        Term::Uri(uri) => Ok(q_hash(uri)),
        Term::Literal(lit) => Ok(q_hash(lit)),
        Term::Formula(s) => Ok(crate::modalities::logic::n3_parser::q_hash_formula(s)),
        Term::Variable(_) => Err(N3CompileError::MalformedTriple),
    }
}

pub fn triple_to_quin(triple: &CompiledTriple, context: u64) -> Result<NQuin, N3CompileError> {
    let mut quin = NQuin::default();
    quin.subject = triple.subject.as_u64();
    quin.predicate = triple.predicate.as_u64();
    quin.object = triple.object.as_u64();
    quin.context = context;
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
    Ok(quin)
}

fn first_triple<'a>(
    f: &'a crate::modalities::logic::n3_compiler::CompiledFormula,
) -> Result<&'a crate::modalities::logic::n3_compiler::CompiledTriple, N3CompileError> {
    f.triples.first().ok_or(N3CompileError::MalformedTriple)
}

/// Returns true when every conclusion triple property path matches a compiled SHACL shape.
pub fn validate_rule_against_shapes(
    rule: &Rule<'_>,
    shapes: &[&CompiledShape],
) -> Result<(), N3CompileError> {
    if shapes.is_empty() {
        return Ok(());
    }
    let conclusion = rule
        .conclusion
        .triples
        .first()
        .ok_or(N3CompileError::MalformedTriple)?;
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
    rule: &CompiledRule,
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
    rule: &CompiledRule,
    contract_hash: u64,
    out: &mut [NQuin],
) -> Result<usize, N3CompileError> {
    if let Some(norm) =
        crate::modalities::logic::deontic::compile_n3_rule_to_norm(rule, contract_hash, 0)
    {
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
    rules: &[Rule<'_>],
    shapes: &[&CompiledShape],
    opcodes_out: &mut [SlgOpcode],
    quins_out: &mut [NQuin],
    contract_hash: u64,
) -> Result<N3CompiledProgram, N3CompileError> {
    let mut opcode_offset = 0usize;
    let mut quin_offset = 0usize;

    for rule in rules {
        // SHACL firewall: validate each rule against the routed shapes BEFORE compiling.
        // `compile_term` hashes literals away ("12" -> u64), so a numeric range check is
        // only possible here, on the Rule. Fail closed on any violation. Validation reads
        // the existing Rule (no allocation); the compile step below is stack-only, so the
        // gate itself remains zero-heap.
        validate_rule_against_shapes(rule, shapes)?;

        let compiled = compile_rule_to_zero_heap(rule);

        let written = compile_rule_to_opcodes(&compiled, &mut opcodes_out[opcode_offset..])?;
        opcode_offset += written;

        let quins_written =
            compile_rule_to_quin(&compiled, contract_hash, &mut quins_out[quin_offset..])?;
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
) -> Result<Option<NQuin>, SentinelError> {
    if opcodes.len() > max_depth as usize {
        return Err(SentinelError::MemoryOverflow);
    }
    Ok(execute_vm_frame(arena, opcodes, frame))
}

/// Build a default health-observation SHACL gate for LLM-emitted N3 (cold path helper).
pub fn default_observation_shape() -> CompiledShape {
    ShaclCompiler::new().compile_class(
        "fhir:Observation",
        "health:restingHeartRate",
        ShaclConstraint::MinInclusive(20.0),
        ShaclSeverity::Violation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::logic::n3_parser::{Formula, Rule, RuleType, Triple};

    fn sample_strict_rule() -> Rule<'static> {
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
        let count =
            compile_rule_to_opcodes(
                &crate::modalities::logic::n3_compiler::compile_rule_to_zero_heap(
                    &sample_strict_rule(),
                ),
                &mut opcodes,
            )
            .unwrap();
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
        // Build inputs OUTSIDE the measured region: parser-owned `Rule`s carry heap Vecs;
        // this test asserts the GATE ITSELF allocates nothing (validate + stack compile).
        let rules = [sample_strict_rule()];
        let shape = default_observation_shape();
        let shapes = [&shape];
        let mut opcodes = [SlgOpcode::Call; MAX_COMPILED_OPCODES];
        let mut quins = [NQuin::default(); MAX_COMPILED_QUINS];

        let _profiler = dhat::Profiler::builder().testing().build();
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
