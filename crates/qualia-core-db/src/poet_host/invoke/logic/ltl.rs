//! LTL evaluation over an explicit bounded trace or the live Quin trace.

use super::super::args;
use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula, SafetyMonitor};
use crate::poet_host::{hash_val, PoetSnapshot};
use crate::{q_hash, NQuin};
use vibe::{DiagCode, Diagnostic, Span, Value};

const MAX_TRACE: usize = 256;

pub fn globally(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    run(snap, args, span, true)
}

pub fn finally(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    run(snap, args, span, false)
}

/// Evaluate G/F/X/U/R against a caller-supplied trace of predicate IRIs. Safety mode
/// additionally reports the first violation observed by the streaming monitor.
pub fn evaluate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let operator = args::rec_str(args_v, "operator")
        .ok_or_else(|| args::bad(span, "LTL evaluation needs `operator`"))?
        .to_ascii_uppercase();
    let events = args::rec_str_list(args_v, "trace")
        .ok_or_else(|| args::bad(span, "LTL evaluation needs a string `trace` list"))?;
    if events.is_empty() || events.len() > MAX_TRACE {
        return Err(args::bad(
            span,
            format!("LTL trace must contain 1..={MAX_TRACE} events"),
        ));
    }
    let trace = events
        .iter()
        .enumerate()
        .map(|(index, event)| NQuin {
            predicate: q_hash(event),
            metadata: index as u64,
            ..NQuin::default()
        })
        .collect::<Vec<_>>();

    let primary = args::rec_str(args_v, "predicate").map(q_hash);
    let left = args::rec_str(args_v, "left").map(q_hash);
    let right = args::rec_str(args_v, "right").map(q_hash);
    let formula = match operator.as_str() {
        "G" => LtlFormula::Globally(primary.ok_or_else(|| args::bad(span, "G needs `predicate`"))?),
        "F" => LtlFormula::Finally(primary.ok_or_else(|| args::bad(span, "F needs `predicate`"))?),
        "X" => LtlFormula::Next(primary.ok_or_else(|| args::bad(span, "X needs `predicate`"))?),
        "U" => LtlFormula::Until {
            ante: left.ok_or_else(|| args::bad(span, "U needs `left`"))?,
            consequent: right.ok_or_else(|| args::bad(span, "U needs `right`"))?,
        },
        "R" => LtlFormula::Release {
            trigger: left.ok_or_else(|| args::bad(span, "R needs `left`"))?,
            invariant: right.ok_or_else(|| args::bad(span, "R needs `right`"))?,
        },
        _ => return Err(args::bad(span, "LTL operator must be G, F, X, U, or R")),
    };
    let holds = evaluate_ltl_trace(&trace, &formula);
    let safety = args::rec_bool(args_v, "safety").unwrap_or(false);
    let mut first_violation = None;
    if safety {
        if operator != "G" {
            return Err(args::bad(span, "streaming safety mode requires operator=G"));
        }
        let mut monitor = SafetyMonitor::new(primary.expect("G predicate checked above"));
        for (index, event) in trace.iter().enumerate() {
            if !monitor.step(event.predicate) && first_violation.is_none() {
                first_violation = Some(index as u64);
            }
        }
    }
    Ok(args::record([
        ("operator", Value::String(operator)),
        ("trace_length", Value::U64(trace.len() as u64)),
        ("holds", Value::Bool(holds)),
        ("safety", Value::Bool(safety)),
        (
            "first_violation",
            first_violation.map(Value::U64).unwrap_or(Value::Null),
        ),
    ]))
}

fn run(snap: &PoetSnapshot, args: &Value, span: Span, globally: bool) -> Result<Value, Diagnostic> {
    let pred = predicate_hash(args).ok_or_else(|| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            "ltl needs a predicate IRI, hash, or modal body",
        )
    })?;
    let formula = if globally {
        LtlFormula::Globally(pred)
    } else {
        LtlFormula::Finally(pred)
    };
    Ok(Value::Bool(snap.with_live_quins(|quins| {
        evaluate_ltl_trace(quins, &formula)
    })))
}

fn predicate_hash(args: &Value) -> Option<u64> {
    if let Some(h) = hash_val(args) {
        return Some(h);
    }
    match args {
        Value::Record(m) => m
            .get("body")
            .and_then(hash_val)
            .or_else(|| m.get("predicate").and_then(hash_val)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_trace_supports_until_and_safety_evidence() {
        let until = args::record([
            ("operator", Value::String("U".into())),
            ("left", Value::String("safe".into())),
            ("right", Value::String("done".into())),
            (
                "trace",
                Value::List(
                    ["safe", "safe", "done"]
                        .into_iter()
                        .map(|v| Value::String(v.into()))
                        .collect(),
                ),
            ),
        ]);
        let Value::Record(result) = evaluate(&until, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        assert_eq!(result.get("holds"), Some(&Value::Bool(true)));

        let safety = args::record([
            ("operator", Value::String("G".into())),
            ("predicate", Value::String("safe".into())),
            ("safety", Value::Bool(true)),
            (
                "trace",
                Value::List(
                    ["safe", "unsafe", "safe"]
                        .into_iter()
                        .map(|v| Value::String(v.into()))
                        .collect(),
                ),
            ),
        ]);
        let Value::Record(result) = evaluate(&safety, Span::new(0, 0)).unwrap() else {
            panic!("expected record")
        };
        assert_eq!(result.get("holds"), Some(&Value::Bool(false)));
        assert_eq!(result.get("first_violation"), Some(&Value::U64(1)));
    }
}
