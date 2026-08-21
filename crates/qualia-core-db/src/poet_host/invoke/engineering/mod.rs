//! Future seam: `qualia-engineering` (`specialized_libs/engineering_analysis`).

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extra;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod kinematics;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod survival;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extra::{analyze_conduction, fem_static};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use kinematics::run as kinematics;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use survival::{cauchy_stress, drag_force, fatigue_cycles, miner_damage, reynolds_number};

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn kinematics(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn cauchy_stress(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn drag_force(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn reynolds_number(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn fatigue_cycles(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn miner_damage(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn analyze_conduction(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn fem_static(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}
