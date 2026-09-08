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
mod wave19_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave20_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave21_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave22_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave23_host;

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use distance::{
    distance_2d, distance_3d, point_segment_distance_2d, point_segment_distance_3d,
    point_triangle_distance_3d,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use extra::*;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use hull::hull2;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave19_host::{
    average_spacing_3d_host as average_spacing_3d, local_density_3d_host as local_density_3d,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave20_host::mean_knn_distance_3d_host as mean_knn_distance_3d;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave21_host::{
    dist_point_to_segment_host as dist_point_to_segment,
    dist_sq_point_to_segment_host as dist_sq_point_to_segment,
    fisher_distance_host as fisher_distance,
    kl_bregman_form_host as kl_bregman_form,
    kl_divergence_host as kl_divergence,
    triangle_signed_area_host as triangle_signed_area,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave22_host::{
    diameter_and_width_host as diameter_and_width,
    directional_width_host as directional_width, farthest_site_brute_host as farthest_site_brute,
    incircle_host as incircle, is_hull_site_host as is_hull_site,
    k_nearest_sites_host as k_nearest_sites, tukey_depth_host as tukey_depth, width_host as width,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave23_host::{
    ham_sandwich_cut_host as ham_sandwich_cut, insphere_host as insphere,
    minkowski_sum_convex_host as minkowski_sum_convex,
    nearest_segment_site_host as nearest_segment_site,
    point_in_polygon_host as point_in_polygon, polygon_area_host as polygon_area,
    polygon_signed_area_host as polygon_signed_area,
    smallest_enclosing_disk_host as smallest_enclosing_disk,
};

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
    orient_3d,
    average_spacing_3d,
    local_density_3d,
    mean_knn_distance_3d,
    dist_point_to_segment,
    dist_sq_point_to_segment,
    fisher_distance,
    kl_bregman_form,
    kl_divergence,
    triangle_signed_area,
    incircle,
    tukey_depth,
    directional_width,
    width,
    farthest_site_brute,
    k_nearest_sites,
    is_hull_site,
    diameter_and_width,
    insphere,
    ham_sandwich_cut,
    smallest_enclosing_disk,
    polygon_signed_area,
    polygon_area,
    point_in_polygon,
    minkowski_sum_convex,
    nearest_segment_site
);
