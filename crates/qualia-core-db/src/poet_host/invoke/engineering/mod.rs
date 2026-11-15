//! Future seam: `qualia-engineering` (`specialized_libs/engineering_analysis`).

#[cfg(not(target_arch = "wasm32"))]
mod extra;
#[cfg(not(target_arch = "wasm32"))]
mod kinematics;
#[cfg(not(target_arch = "wasm32"))]
mod survival;
#[cfg(not(target_arch = "wasm32"))]
mod wave16_host;

#[cfg(not(target_arch = "wasm32"))]
pub use extra::{analyze_conduction, fem_static, natural_frequency_sdof};
#[cfg(not(target_arch = "wasm32"))]
pub use kinematics::run as kinematics;
#[cfg(not(target_arch = "wasm32"))]
pub use survival::{cauchy_stress, drag_force, fatigue_cycles, miner_damage, reynolds_number};
#[cfg(not(target_arch = "wasm32"))]
pub use wave16_host::{analyze_euler, analyze_harmonic_sdof, compute_reliability_index};

#[cfg(target_arch = "wasm32")]
/// WASM gate — engineering analysis requires the scientific profile; native builds call `kinematics::run` (D1).
pub fn kinematics(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native builds delegate to `survival::cauchy_stress` (D1).
pub fn cauchy_stress(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `survival::drag_force` (D1).
pub fn drag_force(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `survival::reynolds_number` (D1).
pub fn reynolds_number(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `survival::fatigue_cycles` (D1).
pub fn fatigue_cycles(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `survival::miner_damage` (D1).
pub fn miner_damage(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `extra::analyze_conduction` (D1).
pub fn analyze_conduction(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `extra::fem_static` (D1).
pub fn fem_static(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `extra::natural_frequency_sdof` (D1).
pub fn natural_frequency_sdof(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `wave16_host::analyze_harmonic_sdof` (D1).
pub fn analyze_harmonic_sdof(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `wave16_host::analyze_euler` (D1).
pub fn analyze_euler(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}

#[cfg(target_arch = "wasm32")]
/// WASM gate — requires scientific profile; native: `wave16_host::compute_reliability_index` (D1).
pub fn compute_reliability_index(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "EngineeringAnalysis"))
}
