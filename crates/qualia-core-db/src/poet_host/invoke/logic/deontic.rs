//! Deontic contract scan on the live graph slice.
//!
//! Workshop `obligate { … }` lowers to `DeonticLogic.evaluate` with a term
//! record. This seam compiles that term to an `NQuin` and scans it with the
//! live graph — no JSON round-trip (P16.5).

use std::collections::BTreeMap;

use crate::lexicon::generate_60bit_token;
use crate::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID,
    OP_OBLIGATE, OP_PERMIT,
};
use crate::poet_host::PoetSnapshot;
use crate::NQuin;
use vibe::{DiagCode, Diagnostic, Span, Value};

const MAX_VERDICTS: usize = 32;

pub fn evaluate(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let compiled = compile_term_quin(args);
    snap.with_live_quins(|quins| {
        let mut scan = Vec::with_capacity(quins.len() + usize::from(compiled.is_some()));
        if let Some(q) = compiled {
            scan.push(q);
        }
        scan.extend_from_slice(quins);
        let mut out = [DeonticVerdict::default(); MAX_VERDICTS];
        let n = evaluate_deontic_contract(&scan, 0, &mut out)
            .map_err(|_| Diagnostic::new(DiagCode::E400, span, "deontic output buffer full"))?;
        Ok(verdict_record(snap.honesty(), n, &out[..n]))
    })
}

fn compile_term_quin(args: &Value) -> Option<NQuin> {
    let rec = match args {
        Value::Record(m) => m,
        _ => return None,
    };
    let modality = match rec.get("modality") {
        Some(Value::String(s)) => s.as_str(),
        _ => return None,
    };
    let opcode = match modality {
        "obligate" => OP_OBLIGATE,
        "permit" => OP_PERMIT,
        "forbid" => OP_FORBID,
        _ => return None,
    };
    let body = match rec.get("body") {
        Some(Value::String(s) | Value::Iri(s)) => s.as_bytes(),
        Some(Value::Prefixed(_, l)) => l.as_bytes(),
        _ => b"vibe:modal",
    };
    Some(compile_norm_quin(
        generate_60bit_token(b"vibe:party"),
        opcode,
        generate_60bit_token(b"vibe:action"),
        generate_60bit_token(body),
        generate_60bit_token(b"vibe:contract"),
        0,
        false,
    ))
}

fn status_name(status: DeonticStatus) -> &'static str {
    match status {
        DeonticStatus::Active => "Active",
        DeonticStatus::Defeated => "Defeated",
        DeonticStatus::Expired => "Expired",
        DeonticStatus::Malformed => "Malformed",
        DeonticStatus::Pending => "Pending",
        DeonticStatus::Violated => "Violated",
        DeonticStatus::Discharged => "Discharged",
    }
}

fn verdict_record(honesty: &'static str, n: usize, verdicts: &[DeonticVerdict]) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String("DeonticLogic.evaluate".into()));
    rec.insert("evaluated".into(), Value::Bool(true));
    rec.insert("honesty".into(), Value::String(honesty.into()));
    rec.insert("verdict_count".into(), Value::U64(n as u64));
    if let Some(first) = verdicts.first() {
        rec.insert("opcode".into(), Value::U64(first.opcode as u64));
        rec.insert(
            "status".into(),
            Value::String(status_name(first.status).into()),
        );
        rec.insert("status_code".into(), Value::U64(first.status as u8 as u64));
    }
    rec.insert(
        "verdicts".into(),
        Value::List(
            verdicts
                .iter()
                .map(|v| {
                    let mut row = BTreeMap::new();
                    row.insert("opcode".into(), Value::U64(v.opcode as u64));
                    row.insert("status_code".into(), Value::U64(v.status as u8 as u64));
                    row.insert(
                        "status".into(),
                        Value::String(status_name(v.status).into()),
                    );
                    Value::Record(row)
                })
                .collect(),
        ),
    );
    Value::Record(rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::generate_60bit_token;
    use crate::modalities::logic::deontic::{compile_norm_quin, OP_OBLIGATE};
    use crate::poet_host::PoetSnapshot;

    #[test]
    fn compiled_obligation_is_scanned() {
        let q = compile_norm_quin(
            generate_60bit_token(b"did:qualia:timothy_charles_holborn"),
            OP_OBLIGATE,
            generate_60bit_token(b"clinic:mustReport"),
            generate_60bit_token(b"clinic:Overheat"),
            generate_60bit_token(b"clinic:alerts"),
            0,
            false,
        );
        let snap = PoetSnapshot::with_seed(vec![q]);
        match evaluate(&snap, &Value::Null, Span { start: 0, end: 0 }).unwrap() {
            Value::Record(m) => {
                assert_eq!(m.get("evaluated"), Some(&Value::Bool(true)));
                assert_eq!(m.get("verdict_count"), Some(&Value::U64(1)));
                match m.get("verdicts") {
                    Some(Value::List(xs)) => assert_eq!(xs.len(), 1),
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn workshop_term_compiles_to_nquin_not_json() {
        let mut rec = BTreeMap::new();
        rec.insert("modality".into(), Value::String("obligate".into()));
        rec.insert("body".into(), Value::String("sign".into()));
        rec.insert("kind".into(), Value::String("term".into()));
        rec.insert("evaluated".into(), Value::Bool(false));
        let snap = PoetSnapshot::default();
        match evaluate(&snap, &Value::Record(rec), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(m) => {
                assert_eq!(m.get("evaluated"), Some(&Value::Bool(true)));
                assert_eq!(m.get("verdict_count"), Some(&Value::U64(1)));
                assert_eq!(m.get("opcode"), Some(&Value::U64(OP_OBLIGATE as u64)));
                match m.get("verdicts") {
                    Some(Value::List(xs)) => {
                        assert!(!xs.iter().any(|v| matches!(v, Value::String(s) if s.contains("opcode="))));
                    }
                    other => panic!("{other:?}"),
                }
            }
            other => panic!("{other:?}"),
        }
    }
}
