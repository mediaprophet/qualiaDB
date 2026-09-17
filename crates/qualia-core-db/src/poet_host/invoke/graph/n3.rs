//! N3 parse and bounded rule-evaluation adapter.

use super::super::args;
use crate::modalities::logic::n3_compiler::{compile_triple, triple_to_quin};
use crate::modalities::logic::n3_parser::{N3Event, N3Parser};
use crate::modalities::logic::rules::RuleEngine;
use crate::q_hash;
use std::collections::BTreeMap;
use vibe::{Diagnostic, Span, Value};

const MAX_STATIC_TRIPLES: usize = 256;

pub fn evaluate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source = args::rec_str(args_v, "source")
        .ok_or_else(|| args::bad(span, "N3Logic.evaluate needs `source`"))?;
    let mode = args::rec_str(args_v, "mode").unwrap_or("evaluate");
    if !matches!(mode, "parse" | "evaluate" | "infer") {
        return Err(args::bad(
            span,
            "N3 mode must be `parse`, `evaluate`, or `infer`",
        ));
    }
    let context = args::rec_str(args_v, "context")
        .map(q_hash)
        .unwrap_or_else(|| q_hash("urn:poet:n3:workbench"));
    let mut parser = N3Parser::new(source);
    let mut static_quins = Vec::new();
    let mut rules = 0usize;
    let mut asp_blocks = 0usize;
    let mut diffuse_blocks = 0usize;
    parser
        .parse_all(|event| {
            match event {
                N3Event::StaticTriple(triple) => {
                    if static_quins.len() >= MAX_STATIC_TRIPLES {
                        return Err(crate::modalities::logic::n3_parser::N3ParserError(
                            "N3 workbench static-triple limit exceeded",
                        ));
                    }
                    let compiled = compile_triple(&triple);
                    let quin = triple_to_quin(&compiled, context).map_err(|_| {
                        crate::modalities::logic::n3_parser::N3ParserError(
                            "N3 triple could not be compiled",
                        )
                    })?;
                    static_quins.push(quin);
                }
                N3Event::LogicRule(_) => rules += 1,
                N3Event::AspBlock(_) => asp_blocks += 1,
                N3Event::DiffuseBlock(_) => diffuse_blocks += 1,
            }
            Ok(())
        })
        .map_err(|error| args::bad(span, format!("N3 parse failed: {error}")))?;

    let mut fired = 0usize;
    let mut matched = 0usize;
    let explain = args::rec_bool(args_v, "explain").unwrap_or(false);
    let mut derivations = Vec::new();
    if matches!(mode, "evaluate" | "infer") && rules > 0 {
        let mut engine = RuleEngine::with_contract(context);
        fired = engine.load_n3("poet-workbench", source);
        for (fact_index, quin) in static_quins.iter().enumerate() {
            for result in engine.evaluate_silent(quin) {
                if result.passed {
                    matched += 1;
                    if explain {
                        let mut record = BTreeMap::new();
                        record.insert("fact_index".into(), Value::U64(fact_index as u64));
                        record.insert("ruleset".into(), Value::String(result.ruleset_name));
                        record.insert("rule".into(), Value::String(result.rule_name));
                        record.insert("evidence".into(), Value::String(result.message));
                        record.insert("subject_hash".into(), Value::U64(quin.subject));
                        record.insert("predicate_hash".into(), Value::U64(quin.predicate));
                        record.insert("object_hash".into(), Value::U64(quin.object));
                        derivations.push(Value::Record(record));
                    }
                }
            }
        }
    }
    Ok(args::record([
        ("mode", Value::String(mode.to_string())),
        ("static_triples", Value::U64(static_quins.len() as u64)),
        ("rules", Value::U64(rules as u64)),
        ("asp_blocks", Value::U64(asp_blocks as u64)),
        ("diffuse_blocks", Value::U64(diffuse_blocks as u64)),
        ("rules_fired", Value::U64(fired as u64)),
        ("static_fact_matches", Value::U64(matched as u64)),
        ("derived_matches", Value::U64(matched as u64)),
        ("explain", Value::Bool(explain)),
        ("derivations", Value::List(derivations)),
        ("context_hash", Value::U64(context)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_fires_n3_rule() {
        let source = "{ <did:alice> <must> <act> } => { <contract> <obligate> <act> }.";
        let input = args::record([
            ("source", Value::String(source.into())),
            ("mode", Value::String("evaluate".into())),
        ]);
        let Value::Record(result) = evaluate(&input, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        assert_eq!(args::as_u64(result.get("rules").unwrap()), Some(1));
        assert_eq!(args::as_u64(result.get("rules_fired").unwrap()), Some(1));
    }

    #[test]
    fn infer_mode_returns_derivation_evidence() {
        let source = "<did:alice> <must> <act> .\n{ <did:alice> <must> <act> } => { <contract> <obligate> <act> }.";
        let input = args::record([
            ("source", Value::String(source.into())),
            ("mode", Value::String("infer".into())),
            ("explain", Value::Bool(true)),
        ]);
        let Value::Record(result) = evaluate(&input, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        assert_eq!(
            args::as_u64(result.get("derived_matches").unwrap()),
            Some(1)
        );
        let Value::List(derivations) = result.get("derivations").unwrap() else {
            panic!("expected derivations")
        };
        assert_eq!(derivations.len(), 1);
    }
}
