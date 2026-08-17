//! Future seam: `qualia-ml` (`solvers/learning` today).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod ols;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use ols::fit_ols;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn fit_ols(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "MachineLearning"))
}
