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
mod wave24_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave25_host;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
mod wave26_host;

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
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave24_host::{
    boolean_difference_area_host as boolean_difference_area,
    boolean_intersection_area_host as boolean_intersection_area,
    boolean_union_area_host as boolean_union_area, dual_point_to_line_host as dual_point_to_line,
    dual_round_trip_host as dual_round_trip, is_convex_polygon_host as is_convex_polygon,
    point_in_or_on_polygon_host as point_in_or_on_polygon, width_coreset_host as width_coreset,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave25_host::{
    cross_ratio_1d_host as cross_ratio_1d, householder_reflect_host as householder_reflect,
    hyperplane_eval_host as hyperplane_eval, point_from_projective_host as point_from_projective,
    projective_from_point_host as projective_from_point,
    quaternion_normalize_host as quaternion_normalize, so3_exp_host as so3_exp,
    so3_log_host as so3_log,
};
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub use wave26_host::{
    barycentric_tetra_host as barycentric_tetra, frame_to_world_host as frame_to_world,
    quaternion_slerp_host as quaternion_slerp, quaternion_to_matrix_host as quaternion_to_matrix,
    schur_complement_2x2_host as schur_complement_2x2,
    separating_plane_aabb_host as separating_plane_aabb,
    solve_diagonal_quadratic_host as solve_diagonal_quadratic, world_to_frame_host as world_to_frame,
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
    nearest_segment_site,
    width_coreset,
    dual_point_to_line,
    dual_round_trip,
    is_convex_polygon,
    point_in_or_on_polygon,
    boolean_union_area,
    boolean_intersection_area,
    boolean_difference_area,
    cross_ratio_1d,
    hyperplane_eval,
    householder_reflect,
    quaternion_normalize,
    so3_exp,
    so3_log,
    projective_from_point,
    point_from_projective,
    frame_to_world,
    world_to_frame,
    barycentric_tetra,
    quaternion_slerp,
    quaternion_to_matrix,
    solve_diagonal_quadratic,
    schur_complement_2x2,
    separating_plane_aabb
);
