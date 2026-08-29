//! Future seam: `qualia-econ` (`financial_modeling` + `computational_economics`).

#[cfg(not(target_arch = "wasm32"))]
mod black_scholes;
pub mod computational_economics;
#[cfg(not(target_arch = "wasm32"))]
mod finance_ext;
#[cfg(not(target_arch = "wasm32"))]
mod gbm_var;
#[cfg(not(target_arch = "wasm32"))]
mod portfolio;

#[cfg(not(target_arch = "wasm32"))]
pub use black_scholes::price as black_scholes;
pub use computational_economics as econ;
#[cfg(not(target_arch = "wasm32"))]
pub use finance_ext::{convert_currency, ledger_balance, multisig_check};
#[cfg(not(target_arch = "wasm32"))]
pub use gbm_var::simulate as gbm_var;
#[cfg(not(target_arch = "wasm32"))]
pub use portfolio::risk as portfolio_risk;

#[cfg(target_arch = "wasm32")]
pub fn black_scholes(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "FinancialModeling"))
}

#[cfg(target_arch = "wasm32")]
pub fn portfolio_risk(
    args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    black_scholes(args, span)
}

#[cfg(target_arch = "wasm32")]
pub fn gbm_var(args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    black_scholes(args, span)
}

#[cfg(target_arch = "wasm32")]
pub fn convert_currency(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(
        span,
        "Finance.convert_currency",
    ))
}

#[cfg(target_arch = "wasm32")]
pub fn multisig_check(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Finance.multisig_check"))
}

#[cfg(target_arch = "wasm32")]
pub fn ledger_balance(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "Finance.ledger_balance"))
}
