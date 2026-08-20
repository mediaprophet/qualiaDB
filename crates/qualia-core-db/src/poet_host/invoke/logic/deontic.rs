//! Deontic contract scan on the live graph slice.

use crate::modalities::logic::deontic::{evaluate_deontic_contract, DeonticVerdict};
use crate::poet_host::PoetSnapshot;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

const MAX_VERDICTS: usize = 32;

pub fn evaluate(snap: &PoetSnapshot, span: Span) -> Result<Value, Diagnostic> {
    snap.with_live_quins(|quins| {
        let mut out = [DeonticVerdict::default(); MAX_VERDICTS];
        let n = evaluate_deontic_contract(quins, 0, &mut out)
            .map_err(|_| Diagnostic::new(DiagCode::E400, span, "deontic output buffer full"))?;
        let rows: Vec<Value> = out[..n]
            .iter()
            .map(|v| Value::String(format!("opcode={:#04x} status={:?}", v.opcode, v.status)))
            .collect();
        Ok(Value::List(rows))
    })
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
        match evaluate(&snap, Span { start: 0, end: 0 }).unwrap() {
            Value::List(xs) => assert_eq!(xs.len(), 1),
            other => panic!("{other:?}"),
        }
    }
}
