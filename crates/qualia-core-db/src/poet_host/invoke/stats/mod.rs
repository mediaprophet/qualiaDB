//! Future seam: `qualia-stats` (`solvers/statistics` today).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod correlation;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod descriptive;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod regression;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use correlation::pearson_r;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use descriptive::arithmetic_mean;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use regression::linear_regression;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn arithmetic_mean(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Statistics"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn pearson_r(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    arithmetic_mean(args, span)
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn linear_regression(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Statistics"))
}
