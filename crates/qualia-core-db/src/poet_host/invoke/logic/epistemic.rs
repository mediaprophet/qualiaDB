//! Epistemic frame scan. Workshop `knows`/`believes` compile to NQuins (P16.5).

use std::collections::BTreeMap;

use crate::lexicon::generate_60bit_token;
use crate::modalities::epistemic::{
    evaluate_epistemic_frame, EpistemicStatus, EpistemicVerdict, CERTAINTY_BELIEVES,
    CERTAINTY_BIT_SHIFT, CERTAINTY_KNOWS, OP_BELIEVES, OP_KNOWS,
};
use crate::poet_host::PoetSnapshot;
use crate::NQuin;
use vibe::{DiagCode, Diagnostic, Span, Value};

const MAX: usize = 32;

pub fn evaluate(snap: &PoetSnapshot, args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let compiled = compile_term_quin(args);
    snap.with_live_quins(|quins| {
        let mut scan = Vec::with_capacity(quins.len() + usize::from(compiled.is_some()));
        if let Some(q) = compiled {
            scan.push(q);
        }
        scan.extend_from_slice(quins);
        let zero = NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let mut out = [EpistemicVerdict {
            claim: zero,
            status: EpistemicStatus::Skipped,
            certainty: 0,
        }; MAX];
        let n = evaluate_epistemic_frame(&scan, 0, 0, &mut out)
            .map_err(|_| Diagnostic::new(DiagCode::E400, span, "epistemic output buffer full"))?;
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
    let (opcode, certainty) = match modality {
        "knows" => (OP_KNOWS, CERTAINTY_KNOWS),
        "believes" => (OP_BELIEVES, CERTAINTY_BELIEVES),
        _ => return None,
    };
    let body = match rec.get("body") {
        Some(Value::String(s) | Value::Iri(s)) => s.as_bytes(),
        Some(Value::Prefixed(_, l)) => l.as_bytes(),
        _ => b"vibe:claim",
    };
    let subject = generate_60bit_token(b"vibe:agent");
    let predicate = opcode as u64 | ((certainty as u64) << CERTAINTY_BIT_SHIFT);
    let object = generate_60bit_token(body);
    let context = generate_60bit_token(b"vibe:world");
    let metadata = 0;
    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
    Some(NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    })
}

fn status_name(status: EpistemicStatus) -> &'static str {
    match status {
        EpistemicStatus::Active => "Active",
        EpistemicStatus::Uncertain => "Uncertain",
        EpistemicStatus::Skipped => "Skipped",
    }
}

fn verdict_record(honesty: &'static str, n: usize, verdicts: &[EpistemicVerdict]) -> Value {
    let mut rec = BTreeMap::new();
    rec.insert("id".into(), Value::String("EpistemicLogic.evaluate".into()));
    rec.insert("evaluated".into(), Value::Bool(true));
    rec.insert("honesty".into(), Value::String(honesty.into()));
    rec.insert("verdict_count".into(), Value::U64(n as u64));
    if let Some(first) = verdicts.first() {
        rec.insert("certainty".into(), Value::U64(first.certainty as u64));
        rec.insert(
            "status".into(),
            Value::String(status_name(first.status).into()),
        );
    }
    rec.insert(
        "verdicts".into(),
        Value::List(
            verdicts
                .iter()
                .map(|v| {
                    let mut row = BTreeMap::new();
                    row.insert("certainty".into(), Value::U64(v.certainty as u64));
                    row.insert("status".into(), Value::String(status_name(v.status).into()));
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
    use crate::poet_host::PoetSnapshot;

    #[test]
    fn workshop_knows_compiles_to_nquin() {
        let mut rec = BTreeMap::new();
        rec.insert("modality".into(), Value::String("knows".into()));
        rec.insert("body".into(), Value::String("fever".into()));
        let snap = PoetSnapshot::default();
        match evaluate(&snap, &Value::Record(rec), Span { start: 0, end: 0 }).unwrap() {
            Value::Record(m) => {
                assert_eq!(m.get("evaluated"), Some(&Value::Bool(true)));
                assert_eq!(m.get("verdict_count"), Some(&Value::U64(1)));
            }
            other => panic!("{other:?}"),
        }
    }
}
