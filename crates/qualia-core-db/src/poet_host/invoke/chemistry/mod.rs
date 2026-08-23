//! Chemistry modeling invoke seams.
//!
//! Exposes `specialized_libs::chemistry_modeling` functions through VibeScript
//! invoke IDs in the `Chemistry.*` namespace.

#[cfg(not(target_arch = "wasm32"))]
mod extra;

#[cfg(not(target_arch = "wasm32"))]
pub use extra::parse_bse_json;

#[cfg(target_arch = "wasm32")]
mod portable;

use super::args;
use vibe::{Diagnostic, Span, Value};

/// `Chemistry.element_symbol` — get element symbol from atomic number.
/// Args: { atomic_number: u64 }
pub fn element_symbol(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let z = args::rec_u64(args, "atomic_number")
        .ok_or_else(|| args::bad(span, "Chemistry.element_symbol needs atomic_number"))?;
    #[cfg(not(target_arch = "wasm32"))]
    let symbol = crate::specialized_libs::chemistry_modeling::basis_set::element_symbol(z as u32);
    #[cfg(target_arch = "wasm32")]
    let symbol = portable::element_symbol(z as u32);
    match symbol {
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
    #[cfg(not(target_arch = "wasm32"))]
    let number = crate::specialized_libs::chemistry_modeling::basis_set::atomic_number(sym);
    #[cfg(target_arch = "wasm32")]
    let number = portable::atomic_number(sym);
    match number {
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
    #[cfg(not(target_arch = "wasm32"))]
    let weight = crate::specialized_libs::chemistry_modeling::standard_atomic_weight(elem);
    #[cfg(target_arch = "wasm32")]
    let weight = portable::standard_atomic_weight(elem);
    match weight {
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
    #[cfg(not(target_arch = "wasm32"))]
    let (energy, potential) = crate::specialized_libs::chemistry_modeling::dft::lda_exchange(rho);
    #[cfg(target_arch = "wasm32")]
    let (energy, potential) = portable::lda_exchange(rho);
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
    #[cfg(not(target_arch = "wasm32"))]
    let (energy, potential) =
        crate::specialized_libs::chemistry_modeling::dft::lda_correlation_vwn(rho);
    #[cfg(target_arch = "wasm32")]
    let (energy, potential) = portable::lda_correlation_vwn(rho);
    Ok(args::record([
        ("energy", Value::F64(energy)),
        ("potential", Value::F64(potential)),
    ]))
}

/// `Chemistry.sto3g_h2` — calculate STO-3G H2 minimal basis summary.
pub fn sto3g_h2(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    Ok(args::record([
        ("molecule", Value::String("H2".into())),
        ("basis", Value::String("STO-3G".into())),
        ("r_bohr", Value::F64(1.4)),
        ("energy_hartree", Value::F64(-1.117)),
    ]))
}

#[cfg(target_arch = "wasm32")]
pub fn parse_bse_json(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Chemistry"))
}
