//! Wave-19 Host binds: Audio numeric scalars.
//!
//! Pure CPU paths from `audio::dsp_kernel` — no forge / `caps()` / CUDA.

use super::super::args;
use crate::audio::dsp_kernel::epistemic_temperature_from_q;
use vibe::{Diagnostic, Span, Value};

/// `Audio.epistemic_temperature_from_q` — τ = clamp(q², 0, 4).
/// Args: `{ q: f64 }`. Out: `{ temperature: f64 }`.
pub fn epistemic_temperature_from_q_host(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let q = args::rec_f64(args_v, "q")
        .ok_or_else(|| args::bad(span, "Audio.epistemic_temperature_from_q needs q"))?;
    let t = epistemic_temperature_from_q(q as f32) as f64;
    Ok(args::record([("temperature", Value::F64(t))]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    #[test]
    fn wave19_epistemic_temperature_zero() {
        let mut m = BTreeMap::new();
        m.insert("q".into(), Value::F64(0.0));
        let out = epistemic_temperature_from_q_host(&Value::Record(m), span()).unwrap();
        assert_eq!(args::rec_f64(&out, "temperature").unwrap(), 0.0);
    }

    #[test]
    fn wave19_epistemic_temperature_squared() {
        let mut m = BTreeMap::new();
        m.insert("q".into(), Value::F64(1.5));
        let out = epistemic_temperature_from_q_host(&Value::Record(m), span()).unwrap();
        let t = args::rec_f64(&out, "temperature").unwrap();
        assert!((t - 2.25).abs() < 1e-6);
    }
}
