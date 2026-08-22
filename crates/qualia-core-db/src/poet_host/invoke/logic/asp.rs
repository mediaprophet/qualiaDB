//! Answer-set context worlds from the live graph.

use crate::modalities::asp::{enumerate_stable_models, MAX_STABLE_MODELS};
use crate::poet_host::PoetSnapshot;
use vibe::{Diagnostic, Span, Value};

pub fn enumerate(snap: &PoetSnapshot, span: Span) -> Result<Value, Diagnostic> {
    let _ = span;
    snap.with_live_quins(|quins| {
        let Some(base) = quins.first() else {
            return Ok(Value::List(Vec::new()));
        };
        let mut worlds = [0u64; MAX_STABLE_MODELS];
        let n = enumerate_stable_models(base, quins, &mut worlds);
        Ok(Value::List(
            worlds[..n].iter().copied().map(Value::U64).collect(),
        ))
    })
}
