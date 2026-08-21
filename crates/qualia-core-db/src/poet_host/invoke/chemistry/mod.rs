//! Chemistry modeling invoke seams.
//!
//! Exposes `specialized_libs::chemistry_modeling` functions through VibeScript
//! invoke IDs in the `Chemistry.*` namespace.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extra;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extra::parse_bse_json;

use super::args;
use poet_vibe::{Diagnostic, Span, Value};

/// `Chemistry.element_symbol` — get element symbol from atomic number.
/// Args: { atomic_number: u64 }
pub fn element_symbol(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let z = args::rec_u64(args, "atomic_number")
        .ok_or_else(|| args::bad(span, "Chemistry.element_symbol needs atomic_number"))?;
    match crate::specialized_libs::chemistry_modeling::basis_set::element_symbol(z as u32) {
        Some(sym) => Ok(args::record([
            ("symbol", Value::String(sym.to_string())),
            ("atomic_number", Value::U64(z)),
        ])),
        None => Err(args::bad(
            span,
            format!("Chemistry.element_symbol: no element for Z={z}"),
        )),
    }
}

/// `Chemistry.atomic_number` — get atomic number from element symbol.
/// Args: { symbol: string }
pub fn atomic_number(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let sym = args::rec_str(args, "symbol")
        .ok_or_else(|| args::bad(span, "Chemistry.atomic_number needs symbol"))?;
    match crate::specialized_libs::chemistry_modeling::basis_set::atomic_number(sym) {
        Some(z) => Ok(args::record([
            ("atomic_number", Value::U64(z as u64)),
            ("symbol", Value::String(sym.to_string())),
        ])),
        None => Err(args::bad(
            span,
            format!("Chemistry.atomic_number: unknown symbol '{sym}'"),
        )),
    }
}

/// `Chemistry.standard_atomic_weight` — standard atomic weight of an element.
/// Args: { element: string }
pub fn standard_atomic_weight(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let elem = args::rec_str(args, "element")
        .ok_or_else(|| args::bad(span, "Chemistry.standard_atomic_weight needs element"))?;
    match crate::specialized_libs::chemistry_modeling::standard_atomic_weight(elem) {
        Some(weight) => Ok(args::record([
            ("element", Value::String(elem.to_string())),
            ("atomic_weight", Value::F64(weight)),
        ])),
        None => Err(args::bad(
            span,
            format!("Chemistry.standard_atomic_weight: unknown element '{elem}'"),
        )),
    }
}

/// `Chemistry.lda_exchange` — LDA exchange energy and potential.
/// Args: { rho: f64 }
pub fn lda_exchange(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rho = args::rec_f64(args, "rho")
        .ok_or_else(|| args::bad(span, "Chemistry.lda_exchange needs rho"))?;
    let (energy, potential) = crate::specialized_libs::chemistry_modeling::dft::lda_exchange(rho);
    Ok(args::record([
        ("energy", Value::F64(energy)),
        ("potential", Value::F64(potential)),
    ]))
}

/// `Chemistry.lda_correlation_vwn` — VWN LDA correlation energy and potential.
/// Args: { rho: f64 }
pub fn lda_correlation_vwn(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rho = args::rec_f64(args, "rho")
        .ok_or_else(|| args::bad(span, "Chemistry.lda_correlation_vwn needs rho"))?;
    let (energy, potential) =
        crate::specialized_libs::chemistry_modeling::dft::lda_correlation_vwn(rho);
    Ok(args::record([
        ("energy", Value::F64(energy)),
        ("potential", Value::F64(potential)),
    ]))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn parse_bse_json(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Chemistry"))
}
