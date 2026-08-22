//! Future seam: `qualia-clinical` (`clinical_engine` + `medical/`). Native only.

#[cfg(not(target_arch = "wasm32"))]
mod framingham;

#[cfg(not(target_arch = "wasm32"))]
pub use framingham::score as framingham;

#[cfg(target_arch = "wasm32")]
pub fn framingham(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(vibe::Diagnostic::new(
        vibe::DiagCode::E300,
        span,
        "ClinicalRisk.framingham is native-only",
    ))
}
