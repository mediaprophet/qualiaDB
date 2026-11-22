//! Indexed positive N3 consequences scoped to one authorised motion action.

use crate::{
    bundle::semantic_model::valid_iri,
    modalities::logic::{
        core::{WebizenOpcode, WebizenVM},
        n3_compiler::{compile_triple, CompiledTerm},
        n3_parser::{StackFormula, Term, Triple},
    },
    q42_volume::Q42RangeQueryCursor,
    q_hash, NQuin,
};

use super::{graph::{read_key, KEY}, MacroKeyframe, MotionGraph, SemanticMotionError as Error};

const RULE_SOURCE: u64 = q_hash("https://qualiadb.org/ontology/hmc#n3RuleSource");
const SEMANTIC_MODEL: u64 = q_hash("https://qualiadb.org/ontology/hmc#SemanticModel");
const MAX_RULE_LITERAL: usize = 4096;
const MAX_PREMISES: usize = 4;
pub(super) const RULE_SCRATCH_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy)]
enum Operand {
    Action,
    Frame,
    Iri(u64),
}

impl Operand {
    fn resolve(self, action: u64, frame: u64) -> u64 {
        match self {
            Self::Action => action,
            Self::Frame => frame,
            Self::Iri(iri) => iri,
        }
    }
}

#[derive(Clone, Copy)]
struct Guard {
    subject: Operand,
    predicate: u64,
    object: Operand,
}

pub(super) struct CompiledMacroRule {
    selector: u64,
    guards: [Option<Guard>; MAX_PREMISES - 1],
}

impl CompiledMacroRule {
    pub(super) fn admit(graph: &mut MotionGraph<'_, '_, '_>, source: &str) -> Result<Self, Error> {
        let mut literal = [0u8; MAX_RULE_LITERAL];
        let canonical = encode_rule_literal(source, &mut literal)?;
        let length = canonical.len();
        let rule_hash = q_hash(canonical);
        if !graph.contains(graph.context, RULE_SOURCE, rule_hash, SEMANTIC_MODEL)? {
            return Err(Error::UnboundTarget);
        }
        let resolved = graph.model.lookup_lexicon_hash_into(rule_hash, graph.lex_page, graph.lex_value)
            .map_err(Error::Model)?;
        if resolved != Some(length) || graph.lex_value.get(..length) != Some(canonical.as_bytes()) {
            return Err(Error::UnknownIdentity);
        }

        let rule = source.trim().strip_suffix('.').ok_or(Error::InvalidFact)?;
        let (premise, conclusion) = rule.split_once("=>").ok_or(Error::InvalidFact)?;
        let premise = parse_formula(premise)?;
        let conclusion = parse_formula(conclusion)?;
        if premise.len == 0 || conclusion.len != 1 {
            return Err(Error::InvalidFact);
        }
        let head = conclusion.triples[0];
        let (Term::Variable(action), Term::Variable(frame)) = (head.subject, head.object)
            else { return Err(Error::InvalidFact); };
        if action == frame || verified_iri(graph, head.predicate)? != KEY {
            return Err(Error::InvalidFact);
        }
        let main = premise.triples[0];
        if main.subject != head.subject || main.object != head.object {
            return Err(Error::InvalidFact);
        }
        let selector = verified_iri(graph, main.predicate)?;
        let lowered = compile_triple(&main);
        if lowered.predicate != CompiledTerm::Uri(selector) || selector == KEY {
            return Err(Error::InvalidFact);
        }
        let mut guards = [None; MAX_PREMISES - 1];
        for (slot, term) in premise.as_slice()[1..].iter().enumerate() {
            guards[slot] = Some(Guard {
                subject: operand(graph, term.subject, action, frame)?,
                predicate: verified_iri(graph, term.predicate)?,
                object: operand(graph, term.object, action, frame)?,
            });
        }
        Ok(Self { selector, guards })
    }

    pub(super) fn collect(
        &self, graph: &mut MotionGraph<'_, '_, '_>, action: u64, keys: &mut [MacroKeyframe],
    ) -> Result<usize, Error> {
        let mut cursor = Q42RangeQueryCursor::default();
        let mut rows = [NQuin::default(); 4];
        let mut vm = WebizenVM::new();
        let mut written = 0;
        loop {
            let page = graph.page(action, self.selector, cursor, &mut rows)?;
            for fact in &rows[..page.returned] {
                let mut satisfied = true;
                for guard in self.guards.iter().flatten() {
                    if !graph.contains(guard.subject.resolve(action, fact.object), guard.predicate,
                        guard.object.resolve(action, fact.object), graph.context)?
                    {
                        satisfied = false;
                        break;
                    }
                }
                if !satisfied { continue; }
                vm.load_bytecode(&[
                    WebizenOpcode::MatchSubject(action), WebizenOpcode::HaltIfFalse,
                    WebizenOpcode::MatchPredicate(self.selector), WebizenOpcode::HaltIfFalse,
                    WebizenOpcode::BindRegister { vector_id: 0, register_index: 0 },
                    WebizenOpcode::BindRegister { vector_id: 3, register_index: 1 },
                    WebizenOpcode::EmitQuin { subject_reg: 0, predicate: KEY, object: fact.object, context_reg: 1 },
                ]);
                let inferred = vm.execute_implication(fact).ok_or(Error::InvalidFact)?;
                if inferred.subject != action || inferred.context != graph.context || inferred.predicate != KEY {
                    return Err(Error::InvalidFact);
                }
                let key = keys.get_mut(written).ok_or(Error::OutputTooSmall)?;
                *key = read_key(graph, inferred.object)?;
                written += 1;
            }
            match page.next_cursor {
                Some(next) => cursor = next,
                None => return Ok(written),
            }
        }
    }
}

/// Match the JSON lexical envelope emitted by `SemanticModelBuilder`. The
/// positive fragment admits conventional N3 whitespace and absolute IRIs.
fn encode_rule_literal<'buffer>(source: &str, out: &'buffer mut [u8]) -> Result<&'buffer str, Error> {
    if source.is_empty() || out.len() < 2 || source.len() > out.len() - 2 {
        return Err(Error::InvalidFact);
    }
    let mut index = 1;
    out[0] = b'"';
    for byte in source.bytes() {
        let escaped: &[u8] = match byte {
            b'\t' => b"\\t",
            b'\n' => b"\\n",
            b'\r' => b"\\r",
            0..=31 | b'"' | b'\\' => return Err(Error::InvalidFact),
            _ => core::slice::from_ref(&byte),
        };
        let end = index + escaped.len();
        if end >= out.len() {
            return Err(Error::InvalidFact);
        }
        out[index..end].copy_from_slice(escaped);
        index = end;
    }
    out[index] = b'"';
    std::str::from_utf8(&out[..=index]).map_err(|_| Error::InvalidFact)
}

fn verified_iri(graph: &mut MotionGraph<'_, '_, '_>, term: Term<'_>) -> Result<u64, Error> {
    let Term::Uri(iri) = term else { return Err(Error::InvalidFact); };
    let hash = q_hash(iri);
    if graph.iri(hash)? != iri { return Err(Error::UnknownIdentity); }
    Ok(hash)
}

fn operand(graph: &mut MotionGraph<'_, '_, '_>, term: Term<'_>, action: &str, frame: &str) -> Result<Operand, Error> {
    match term {
        Term::Variable(name) if name == action => Ok(Operand::Action),
        Term::Variable(name) if name == frame => Ok(Operand::Frame),
        _ => verified_iri(graph, term).map(Operand::Iri),
    }
}

/// Parse only the positive fragment into the native fixed-size N3 AST.
/// Complete tokens require whitespace separators and absolute angle IRIs.
fn parse_formula(formula: &str) -> Result<StackFormula<'_>, Error> {
    let body = formula.trim().strip_prefix('{').and_then(|body| body.strip_suffix('}'))
        .ok_or(Error::InvalidFact)?;
    let mut parsed = StackFormula::new();
    let mut fields = [Term::Uri(""); 3];
    let mut position = 0;
    for (count, token) in body.split_whitespace().enumerate() {
        if count >= 4 * MAX_PREMISES { return Err(Error::BudgetExceeded); }
        position = count % 4;
        if position == 3 {
            if token != "." { return Err(Error::InvalidFact); }
            continue;
        }
        fields[position] = if let Some(name) = token.strip_prefix('?') {
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(Error::InvalidFact);
            }
            Term::Variable(name)
        } else {
            let iri = token.strip_prefix('<').and_then(|v| v.strip_suffix('>')).ok_or(Error::InvalidFact)?;
            if !valid_iri(iri) || iri.contains(['<', '>', '{', '}']) { return Err(Error::InvalidFact); }
            Term::Uri(iri)
        };
        if position == 2 {
            let [subject, predicate, object] = fields;
            parsed.triples[parsed.len] = Triple { subject, predicate, object };
            parsed.len += 1;
        }
    }
    match position {
        2 | 3 => Ok(parsed),
        _ => Err(Error::InvalidFact),
    }
}
