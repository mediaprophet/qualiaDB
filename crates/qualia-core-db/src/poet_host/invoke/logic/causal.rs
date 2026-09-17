//! Causal reachability and Gödel t-norm. Full modalities only (not wasm-ontology lite).

use super::super::args;
use crate::poet_host::{hash_val, PoetSnapshot};
use vibe::{Diagnostic, Span, Value};

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub fn caused(snap: &PoetSnapshot, args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::causal::caused as caused_of;
    let effect = rec_hash(args_v, "effect")
        .or_else(|| hash_val(args_v))
        .ok_or_else(|| args::bad(span, "CausalFuzzyAndControl.caused needs effect"))?;
    let mut roots = [0u64; 32];
    let mut n = 0usize;
    if let Some(xs) = args::rec(args_v, "roots").and_then(args::list) {
        for x in xs {
            if n >= roots.len() {
                break;
            }
            if let Some(h) = hash_val(x) {
                roots[n] = h;
                n += 1;
            }
        }
    }
    if n == 0 {
        return Err(args::bad(span, "caused needs roots: [...]"));
    }
    Ok(Value::Bool(snap.with_live_quins(|quins| {
        caused_of(quins, &roots[..n], effect)
    })))
}

#[cfg(not(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
)))]
pub fn caused(_snap: &PoetSnapshot, _args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(vibe::Diagnostic::new(
        vibe::DiagCode::E300,
        span,
        "CausalFuzzyAndControl is not in the wasm-ontology lite profile",
    ))
}

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub fn t_norm(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::modalities::fuzzy::t_norm_godel;
    let (a, b) = args::pair_f64(args_v, span, "t_norm")?;
    Ok(Value::F64(t_norm_godel(a as f32, b as f32) as f64))
}

#[cfg(not(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
)))]
pub fn t_norm(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    caused(&PoetSnapshot::default(), _args, span)
}

fn rec_hash(v: &Value, key: &str) -> Option<u64> {
    args::rec(v, key).and_then(hash_val)
}
