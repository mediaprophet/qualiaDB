//! Computational geometry invoke seams.
//!
//! Exposes `specialized_libs::computational_geometry` through VibeScript
//! invoke IDs in the `ComputationalGeometry.*` namespace.

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod distance;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod extra;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod hull;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use distance::{
    distance_2d, distance_3d, point_segment_distance_2d, point_segment_distance_3d,
    point_triangle_distance_3d,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extra::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use hull::hull2;

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn hull2(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn distance_2d(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn distance_3d(_args: &vibe::Value, span: vibe::Span) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn point_segment_distance_2d(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn point_segment_distance_3d(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn point_triangle_distance_3d(
    _args: &vibe::Value,
    span: vibe::Span,
) -> Result<vibe::Value, vibe::Diagnostic> {
    Err(super::args::need_scientific(span, "ComputationalGeometry"))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
macro_rules! geom_stub {
    ($($name:ident),*) => {
        $(
            pub fn $name(
                _args: &vibe::Value,
                span: vibe::Span,
            ) -> Result<vibe::Value, vibe::Diagnostic> {
                Err(super::args::need_scientific(span, "ComputationalGeometry"))
            }
        )*
    };
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
geom_stub!(
    triangulate_polygon,
    surface_area,
    signed_volume,
    morton_encode_2d,
    morton_decode_2d,
    morton_encode_3d,
    hilbert_encode_2d,
    orientation_2,
    circumcenter,
    line_segment_intersection_2,
    bezier_eval,
    nearest_site_brute_force,
    orient_3d
);
