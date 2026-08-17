//! Future seam: `qualia-clinical` (`clinical_engine` + `medical/`). Native only.

#[cfg(not(target_arch = "wasm32"))]
mod framingham;

#[cfg(not(target_arch = "wasm32"))]
pub use framingham::score as framingham;

#[cfg(target_arch = "wasm32")]
pub fn framingham(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(poet_vibe::Diagnostic::new(
        poet_vibe::DiagCode::E300,
        span,
        "ClinicalRisk.framingham is native-only",
    ))
}
