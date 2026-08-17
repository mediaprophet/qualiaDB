//! Future seam: `qualia-econ` (`financial_modeling` + `computational_economics`).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod black_scholes;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod portfolio;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use black_scholes::price as black_scholes;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use portfolio::risk as portfolio_risk;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn black_scholes(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "FinancialModeling"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn portfolio_risk(
    args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    black_scholes(args, span)
}
