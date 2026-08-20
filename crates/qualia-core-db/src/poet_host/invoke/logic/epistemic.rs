//! Epistemic frame scan.

use crate::modalities::epistemic::{evaluate_epistemic_frame, EpistemicVerdict};
use crate::poet_host::PoetSnapshot;
use poet_vibe::{DiagCode, Diagnostic, Span, Value};

const MAX: usize = 32;

pub fn evaluate(snap: &PoetSnapshot, span: Span) -> Result<Value, Diagnostic> {
    snap.with_live_quins(|quins| {
        let mut out = [EpistemicVerdict {
            claim: Default::default(),
            status: crate::modalities::epistemic::EpistemicStatus::Skipped,
            certainty: 0,
        }; MAX];
        let n = evaluate_epistemic_frame(quins, 0, 0, &mut out)
            .map_err(|_| Diagnostic::new(DiagCode::E400, span, "epistemic output buffer full"))?;
        Ok(Value::U64(n as u64))
    })
}
