//! Future seam: `qualia-engineering` (`specialized_libs/engineering_analysis`).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod kinematics;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use kinematics::run as kinematics;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn kinematics(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}
