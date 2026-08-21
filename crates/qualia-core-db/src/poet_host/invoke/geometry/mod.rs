//! Computational geometry invoke seams.
//!
//! Exposes `specialized_libs::computational_geometry` through VibeScript
//! invoke IDs in the `ComputationalGeometry.*` namespace.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod distance;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod hull;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use distance::{
    distance_2d, distance_3d, point_segment_distance_2d, point_segment_distance_3d,
    point_triangle_distance_3d,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use hull::hull2;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn hull2(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn distance_2d(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn distance_3d(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn point_segment_distance_2d(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn point_segment_distance_3d(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn point_triangle_distance_3d(
    _args: &poet_vibe::Value,
    span: poet_vibe::Span,
) -> Result<poet_vibe::Value, poet_vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}
