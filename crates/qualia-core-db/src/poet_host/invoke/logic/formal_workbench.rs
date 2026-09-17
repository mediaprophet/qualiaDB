//! Bounded POET adapters for CTL, defeasible, linear, and dialectical logic.

use super::super::args;
use crate::{q_hash, NQuin};
use vibe::{Diagnostic, Span, Value};

const MAX_FACTS: usize = 128;

pub fn compute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    match args::rec_str(args_v, "mode") {
        Some("ctl") => ctl(args_v, span),
        Some("defeasible") => defeasible(args_v, span),
        Some("linear") => linear(args_v, span),
        Some("dialectical") => dialectical(args_v, span),
        Some("dialectical_counterfactual") => counterfactual(args_v, span),
        Some(mode) => Err(args::bad(
            span,
            format!("unknown formal-logic mode `{mode}`"),
        )),
        None => Err(args::bad(span, "FormalLogic.compute needs mode")),
    }
}

fn pair_names<'a>(
    value: &'a Value,
    label: &str,
    span: Span,
) -> Result<(&'a str, &'a str), Diagnostic> {
    let pair = args::list(value)
        .ok_or_else(|| args::bad(span, format!("{label} entries must be [from,to]")))?;
    if pair.len() != 2 {
        return Err(args::bad(span, format!("{label} entries need two names")));
    }
    let a = args::as_str(&pair[0])
        .ok_or_else(|| args::bad(span, format!("{label} values must be names")))?;
    let b = args::as_str(&pair[1])
        .ok_or_else(|| args::bad(span, format!("{label} values must be names")))?;
    Ok((a, b))
}

fn ctl(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::ctl::{
        all_finally, all_until, always_globally, always_next, exists_finally, exists_globally,
        exists_next, exists_until,
    };
    let operator = args::rec_str(args_v, "operator")
        .ok_or_else(|| args::bad(span, "CTL needs operator"))?
        .to_ascii_uppercase();
    let start =
        q_hash(args::rec_str(args_v, "start").ok_or_else(|| args::bad(span, "CTL needs start"))?);
    let prop = q_hash(
        args::rec_str(args_v, "proposition")
            .ok_or_else(|| args::bad(span, "CTL needs proposition"))?,
    );
    let phi = q_hash(args::rec_str(args_v, "phi").unwrap_or("q42:true"));
    let next = q_hash("q42:next");
    let holds = q_hash("q42:holds");
    let mut graph = [NQuin::default(); MAX_FACTS];
    let mut count = 0usize;
    for value in args::rec(args_v, "transitions")
        .and_then(args::list)
        .unwrap_or(&[])
    {
        if count >= graph.len() {
            return Err(args::bad(span, "CTL frame exceeds 128 facts"));
        }
        let (from, to) = pair_names(value, "transitions", span)?;
        graph[count] = NQuin {
            subject: q_hash(from),
            predicate: next,
            object: q_hash(to),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        count += 1;
    }
    for value in args::rec(args_v, "holds")
        .and_then(args::list)
        .unwrap_or(&[])
    {
        if count >= graph.len() {
            return Err(args::bad(span, "CTL frame exceeds 128 facts"));
        }
        let (state, proposition) = pair_names(value, "holds", span)?;
        graph[count] = NQuin {
            subject: q_hash(state),
            predicate: holds,
            object: q_hash(proposition),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        count += 1;
    }
    if count == 0 {
        return Err(args::bad(
            span,
            "CTL frame needs transitions or holds facts",
        ));
    }
    let frame = &graph[..count];
    let result = match operator.as_str() {
        "EF" => exists_finally(frame, start, prop, next, holds),
        "AG" => always_globally(frame, start, prop, next, holds),
        "EX" => exists_next(frame, start, prop, next, holds),
        "AX" => always_next(frame, start, prop, next, holds),
        "EU" => exists_until(frame, start, phi, prop, next, holds),
        "AU" => all_until(frame, start, phi, prop, next, holds),
        "EG" => exists_globally(frame, start, prop, next, holds),
        "AF" => all_finally(frame, start, prop, next, holds),
        _ => {
            return Err(args::bad(
                span,
                "CTL operator must be EF, AG, EX, AX, EU, AU, EG, or AF",
            ))
        }
    };
    Ok(args::record([
        ("operator", Value::String(operator)),
        ("satisfied", Value::Bool(result)),
        ("fact_count", Value::U64(count as u64)),
    ]))
}

fn rule_kind(
    name: &str,
    span: Span,
) -> Result<crate::modalities::defeasible::RuleKind, Diagnostic> {
    use crate::modalities::defeasible::RuleKind;
    match name.to_ascii_lowercase().as_str() {
        "strict" => Ok(RuleKind::Strict),
        "defeasible" => Ok(RuleKind::Defeasible),
        "defeater" => Ok(RuleKind::Defeater),
        _ => Err(args::bad(
            span,
            "rule kind must be strict, defeasible, or defeater",
        )),
    }
}

fn defeasible(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::defeasible::{
        grounded_justified_rules, resolve_conflict, rules_conflict, AmbiguityMode, DefeasibleRule,
    };
    let literal = q_hash(
        args::rec_str(args_v, "literal")
            .ok_or_else(|| args::bad(span, "defeasible mode needs literal"))?,
    );
    let rule_a_name = args::rec_str(args_v, "rule_a")
        .ok_or_else(|| args::bad(span, "defeasible mode needs rule_a"))?;
    let rule_b_name = args::rec_str(args_v, "rule_b")
        .ok_or_else(|| args::bad(span, "defeasible mode needs rule_b"))?;
    let a = DefeasibleRule {
        id: q_hash(rule_a_name),
        kind: rule_kind(
            args::rec_str(args_v, "kind_a").unwrap_or("defeasible"),
            span,
        )?,
        literal,
        positive: args::rec_bool(args_v, "positive_a").unwrap_or(true),
    };
    let b = DefeasibleRule {
        id: q_hash(rule_b_name),
        kind: rule_kind(
            args::rec_str(args_v, "kind_b").unwrap_or("defeasible"),
            span,
        )?,
        literal,
        positive: args::rec_bool(args_v, "positive_b").unwrap_or(false),
    };
    let superiority = match args::rec_str(args_v, "superior").unwrap_or("none") {
        "none" => Vec::new(),
        name if name == rule_a_name => vec![(a.id, b.id)],
        name if name == rule_b_name => vec![(b.id, a.id)],
        _ => {
            return Err(args::bad(
                span,
                "superior must name rule_a, rule_b, or none",
            ))
        }
    };
    let ambiguity = match args::rec_str(args_v, "ambiguity").unwrap_or("blocking") {
        "blocking" => AmbiguityMode::Blocking,
        "propagating" => AmbiguityMode::Propagating,
        _ => return Err(args::bad(span, "ambiguity must be blocking or propagating")),
    };
    let conclusion = resolve_conflict(&a, &b, &superiority, ambiguity);
    let justified = grounded_justified_rules(&[a, b], &superiority);
    Ok(args::record([
        ("conflict", Value::Bool(rules_conflict(&a, &b))),
        ("conclusion", Value::String(format!("{conclusion:?}"))),
        ("rule_a_justified", Value::Bool(justified.contains(&a.id))),
        ("rule_b_justified", Value::Bool(justified.contains(&b.id))),
    ]))
}

fn linear(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::linear::{
        consume_quin, is_consumed, structural_rule_licensed, tensor_consume, StructuralRule,
    };
    let mut a = NQuin {
        subject: q_hash(
            args::rec_str(args_v, "resource_a")
                .ok_or_else(|| args::bad(span, "linear mode needs resource_a"))?,
        ),
        ..NQuin::default()
    };
    let mut b = NQuin {
        subject: q_hash(
            args::rec_str(args_v, "resource_b")
                .ok_or_else(|| args::bad(span, "linear mode needs resource_b"))?,
        ),
        ..NQuin::default()
    };
    if args::rec_bool(args_v, "consumed_a").unwrap_or(false) {
        consume_quin(&mut a);
    }
    if args::rec_bool(args_v, "consumed_b").unwrap_or(false) {
        consume_quin(&mut b);
    }
    let reusable_a = args::rec_bool(args_v, "reusable_a").unwrap_or(false);
    let reusable_b = args::rec_bool(args_v, "reusable_b").unwrap_or(false);
    let consumed = tensor_consume(&mut a, reusable_a, &mut b, reusable_b);
    let structural = match args::rec_str(args_v, "structural_rule").unwrap_or("exchange") {
        "weakening" => StructuralRule::Weakening,
        "contraction" => StructuralRule::Contraction,
        "exchange" => StructuralRule::Exchange,
        _ => {
            return Err(args::bad(
                span,
                "structural_rule must be weakening, contraction, or exchange",
            ))
        }
    };
    Ok(args::record([
        ("tensor_consumed", Value::Bool(consumed)),
        ("resource_a_consumed", Value::Bool(is_consumed(&a))),
        ("resource_b_consumed", Value::Bool(is_consumed(&b))),
        (
            "structural_rule_licensed_for_a",
            Value::Bool(structural_rule_licensed(structural, reusable_a)),
        ),
    ]))
}

fn dialectical(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::dialectical::{
        ibis_position_favoured, synthesis_coherence, synthesize_dialectical,
    };
    let subject = q_hash(
        args::rec_str(args_v, "subject")
            .ok_or_else(|| args::bad(span, "dialectical mode needs subject"))?,
    );
    let predicate = q_hash(
        args::rec_str(args_v, "predicate")
            .ok_or_else(|| args::bad(span, "dialectical mode needs predicate"))?,
    );
    let thesis = NQuin {
        subject,
        predicate,
        object: q_hash(
            args::rec_str(args_v, "thesis")
                .ok_or_else(|| args::bad(span, "dialectical mode needs thesis"))?,
        ),
        context: q_hash("urn:poet:thesis"),
        metadata: 0,
        parity: 0,
    };
    let antithesis = NQuin {
        subject,
        predicate,
        object: q_hash(
            args::rec_str(args_v, "antithesis")
                .ok_or_else(|| args::bad(span, "dialectical mode needs antithesis"))?,
        ),
        context: q_hash("urn:poet:antithesis"),
        metadata: 0,
        parity: 0,
    };
    let synthesis = synthesize_dialectical(&thesis, &antithesis)
        .ok_or_else(|| args::bad(span, "thesis and antithesis are not contradictory"))?;
    Ok(args::record([
        ("synthesized", Value::Bool(true)),
        ("synthesis_object_hash", Value::U64(synthesis.object)),
        ("synthesis_context", Value::U64(synthesis.context)),
        (
            "coherence",
            Value::F64(synthesis_coherence(&thesis, &antithesis, &synthesis) as f64),
        ),
        (
            "position_favoured",
            Value::Bool(ibis_position_favoured(
                args::rec_u64(args_v, "supporting").unwrap_or(0) as u32,
                args::rec_u64(args_v, "objecting").unwrap_or(0) as u32,
            )),
        ),
    ]))
}

fn counterfactual(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::dialectical::counterfactual_query;
    let edges = args::rec(args_v, "causal_edges")
        .and_then(args::list)
        .ok_or_else(|| args::bad(span, "counterfactual needs causal_edges"))?;
    if edges.is_empty() || edges.len() > MAX_FACTS {
        return Err(args::bad(span, "causal_edges must contain 1..=128 pairs"));
    }
    let mut graph = Vec::with_capacity(edges.len());
    for edge in edges {
        let (from, to) = pair_names(edge, "causal_edges", span)?;
        graph.push(NQuin {
            subject: q_hash(from),
            predicate: q_hash("q42:causes"),
            object: q_hash(to),
            context: 0,
            metadata: 0,
            parity: 0,
        });
    }
    let result = counterfactual_query(
        &graph,
        q_hash(
            args::rec_str(args_v, "factual_outcome")
                .ok_or_else(|| args::bad(span, "counterfactual needs factual_outcome"))?,
        ),
        q_hash(
            args::rec_str(args_v, "intervention")
                .ok_or_else(|| args::bad(span, "counterfactual needs intervention"))?,
        ),
        q_hash(
            args::rec_str(args_v, "intervention_value")
                .ok_or_else(|| args::bad(span, "counterfactual needs intervention_value"))?,
        ),
        q_hash(
            args::rec_str(args_v, "target")
                .ok_or_else(|| args::bad(span, "counterfactual needs target"))?,
        ),
    )
    .ok_or_else(|| args::bad(span, "the intervention has no causal path to the target"))?;
    Ok(args::record([
        ("target_hash", Value::U64(result.subject)),
        (
            "counterfactual_probability",
            Value::F64(result.object as f64 / 1000.0),
        ),
        ("counterfactual", Value::Bool(true)),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn ctl_and_linear_modes_delegate_to_native_evaluators() {
        let ctl = args::record([
            ("mode", Value::String("ctl".into())),
            ("operator", Value::String("EF".into())),
            ("start", Value::String("s0".into())),
            ("proposition", Value::String("approved".into())),
            (
                "transitions",
                Value::List(vec![Value::List(vec![
                    Value::String("s0".into()),
                    Value::String("s1".into()),
                ])]),
            ),
            (
                "holds",
                Value::List(vec![Value::List(vec![
                    Value::String("s1".into()),
                    Value::String("approved".into()),
                ])]),
            ),
        ]);
        match compute(&ctl, span()).unwrap() {
            Value::Record(r) => assert_eq!(r.get("satisfied"), Some(&Value::Bool(true))),
            _ => panic!(),
        }

        let linear = args::record([
            ("mode", Value::String("linear".into())),
            ("resource_a", Value::String("a".into())),
            ("resource_b", Value::String("b".into())),
            ("structural_rule", Value::String("exchange".into())),
        ]);
        match compute(&linear, span()).unwrap() {
            Value::Record(r) => assert_eq!(r.get("tensor_consumed"), Some(&Value::Bool(true))),
            _ => panic!(),
        }
    }

    #[test]
    fn defeasible_and_dialectical_modes_are_resolved() {
        let defeasible = args::record([
            ("mode", Value::String("defeasible".into())),
            ("literal", Value::String("eligible".into())),
            ("rule_a", Value::String("r1".into())),
            ("kind_a", Value::String("defeasible".into())),
            ("positive_a", Value::Bool(true)),
            ("rule_b", Value::String("r2".into())),
            ("kind_b", Value::String("defeasible".into())),
            ("positive_b", Value::Bool(false)),
            ("superior", Value::String("r1".into())),
            ("ambiguity", Value::String("blocking".into())),
        ]);
        match compute(&defeasible, span()).unwrap() {
            Value::Record(r) => {
                assert_eq!(r.get("conclusion"), Some(&Value::String("Positive".into())));
                assert_eq!(r.get("rule_a_justified"), Some(&Value::Bool(true)));
            }
            _ => panic!(),
        }

        let dialectical = args::record([
            ("mode", Value::String("dialectical".into())),
            ("subject", Value::String("decision".into())),
            ("predicate", Value::String("stack".into())),
            ("thesis", Value::String("rust".into())),
            ("antithesis", Value::String("python".into())),
            ("supporting", Value::U64(2)),
            ("objecting", Value::U64(1)),
        ]);
        match compute(&dialectical, span()).unwrap() {
            Value::Record(r) => assert_eq!(r.get("synthesized"), Some(&Value::Bool(true))),
            _ => panic!(),
        }
    }
}
