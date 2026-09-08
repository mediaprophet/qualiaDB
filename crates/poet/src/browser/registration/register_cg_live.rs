//! Computational Geometry Live remainder chain — Host leftovers (waves 19–23).

use super::*;

fn cg_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "lab".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "sci".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn cg_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        cg_live_tool(
            "scientific:cg_live_average_spacing_3d",
            "Average spacing 3D",
            "ComputationalGeometry.average_spacing_3d",
            "Mean kNN spacing via ComputationalGeometry.average_spacing_3d (data-k).",
        ),
        cg_live_tool(
            "scientific:cg_live_local_density_3d",
            "Local density 3D",
            "ComputationalGeometry.local_density_3d",
            "Per-point kNN density via ComputationalGeometry.local_density_3d.",
        ),
        cg_live_tool(
            "scientific:cg_live_mean_knn_distance_3d",
            "Mean kNN distance 3D",
            "ComputationalGeometry.mean_knn_distance_3d",
            "Per-point mean kNN distance via ComputationalGeometry.mean_knn_distance_3d.",
        ),
        cg_live_tool(
            "scientific:cg_live_fisher_distance",
            "Fisher distance",
            "ComputationalGeometry.fisher_distance",
            "Fisher–Rao geodesic via ComputationalGeometry.fisher_distance.",
        ),
        cg_live_tool(
            "scientific:cg_live_kl_divergence",
            "KL divergence",
            "ComputationalGeometry.kl_divergence",
            "KL(p‖q) via ComputationalGeometry.kl_divergence.",
        ),
        cg_live_tool(
            "scientific:cg_live_kl_bregman_form",
            "KL Bregman form",
            "ComputationalGeometry.kl_bregman_form",
            "Bregman KL via ComputationalGeometry.kl_bregman_form.",
        ),
        cg_live_tool(
            "scientific:cg_live_triangle_signed_area",
            "Triangle signed area",
            "ComputationalGeometry.triangle_signed_area",
            "Signed 2D triangle area via ComputationalGeometry.triangle_signed_area.",
        ),
        cg_live_tool(
            "scientific:cg_live_dist_point_to_segment",
            "Dist point–segment",
            "ComputationalGeometry.dist_point_to_segment",
            "Point-to-segment distance via ComputationalGeometry.dist_point_to_segment.",
        ),
        cg_live_tool(
            "scientific:cg_live_dist_sq_point_to_segment",
            "Dist² point–segment",
            "ComputationalGeometry.dist_sq_point_to_segment",
            "Squared point-to-segment distance via ComputationalGeometry.dist_sq_point_to_segment.",
        ),
        cg_live_tool(
            "scientific:cg_live_incircle",
            "Incircle",
            "ComputationalGeometry.incircle",
            "Exact in-circle predicate via ComputationalGeometry.incircle.",
        ),
        cg_live_tool(
            "scientific:cg_live_tukey_depth",
            "Tukey depth",
            "ComputationalGeometry.tukey_depth",
            "Tukey depth via ComputationalGeometry.tukey_depth.",
        ),
        cg_live_tool(
            "scientific:cg_live_directional_width",
            "Directional width",
            "ComputationalGeometry.directional_width",
            "Projected width via ComputationalGeometry.directional_width.",
        ),
        cg_live_tool(
            "scientific:cg_live_width",
            "Width",
            "ComputationalGeometry.width",
            "Sampled minimum width via ComputationalGeometry.width.",
        ),
        cg_live_tool(
            "scientific:cg_live_farthest_site_brute",
            "Farthest site",
            "ComputationalGeometry.farthest_site_brute",
            "Farthest site index via ComputationalGeometry.farthest_site_brute.",
        ),
        cg_live_tool(
            "scientific:cg_live_k_nearest_sites",
            "k nearest sites",
            "ComputationalGeometry.k_nearest_sites",
            "k nearest site indices via ComputationalGeometry.k_nearest_sites (data-k).",
        ),
        cg_live_tool(
            "scientific:cg_live_is_hull_site",
            "Is hull site",
            "ComputationalGeometry.is_hull_site",
            "Hull membership via ComputationalGeometry.is_hull_site (data-index).",
        ),
        cg_live_tool(
            "scientific:cg_live_diameter_and_width",
            "Diameter and width",
            "ComputationalGeometry.diameter_and_width",
            "Rotating-caliper diameter and width via ComputationalGeometry.diameter_and_width.",
        ),
        cg_live_tool(
            "scientific:cg_live_insphere",
            "Insphere",
            "ComputationalGeometry.insphere",
            "Exact in-sphere predicate via ComputationalGeometry.insphere.",
        ),
        cg_live_tool(
            "scientific:cg_live_ham_sandwich_cut",
            "Ham-sandwich cut",
            "ComputationalGeometry.ham_sandwich_cut",
            "Bisecting line via ComputationalGeometry.ham_sandwich_cut.",
        ),
        cg_live_tool(
            "scientific:cg_live_smallest_enclosing_disk",
            "Smallest enclosing disk",
            "ComputationalGeometry.smallest_enclosing_disk",
            "Welzl disk via ComputationalGeometry.smallest_enclosing_disk (data-seed).",
        ),
        cg_live_tool(
            "scientific:cg_live_polygon_signed_area",
            "Polygon signed area",
            "ComputationalGeometry.polygon_signed_area",
            "Signed polygon area via ComputationalGeometry.polygon_signed_area.",
        ),
        cg_live_tool(
            "scientific:cg_live_polygon_area",
            "Polygon area",
            "ComputationalGeometry.polygon_area",
            "Absolute polygon area via ComputationalGeometry.polygon_area.",
        ),
        cg_live_tool(
            "scientific:cg_live_point_in_polygon",
            "Point in polygon",
            "ComputationalGeometry.point_in_polygon",
            "Interior test via ComputationalGeometry.point_in_polygon.",
        ),
        cg_live_tool(
            "scientific:cg_live_minkowski_sum_convex",
            "Minkowski sum",
            "ComputationalGeometry.minkowski_sum_convex",
            "Convex Minkowski sum via ComputationalGeometry.minkowski_sum_convex.",
        ),
        cg_live_tool(
            "scientific:cg_live_nearest_segment_site",
            "Nearest segment site",
            "ComputationalGeometry.nearest_segment_site",
            "Nearest segment-site via ComputationalGeometry.nearest_segment_site.",
        ),
    ]
}
