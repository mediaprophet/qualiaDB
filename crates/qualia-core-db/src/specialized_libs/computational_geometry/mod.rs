//! Native computational geometry for QualiaDB.
//!
//! This is the shared landing zone for the package-by-package CGAL capability
//! port. Algorithms are expressed in Rust over caller-owned slices so the same
//! implementation runs in the native engine and browser/WASM builds. Geometry
//! is not renderer-only: the topology types are graph structures usable by the
//! query, reasoning, simulation, and 10D manifold layers.

mod bvh;
mod boolean_2;
mod box_join;
mod combinatorial_map;
mod connectivity;
mod constrained_delaunay;
mod corpus_4;
mod csr_adjacency;
mod delaunay_2;
mod determinism_corpus;
mod distance;
mod exact_kernel;
mod expansion;
mod features;
mod gpu;
mod gpu_3d;
mod hull;
mod hull_3;
mod decimate_3;
mod delaunay_3;
mod exact_construct_3;
mod remesh_3;
mod tri_tri_3;
mod boolean_3;
mod incircle;
mod insphere;
mod kd_tree;
mod kernel;
mod minkowski_2;
mod orient3d;
pub mod query_frontend;
mod polygon_soup;
mod primitives;
mod spatial_order;
mod surface_mesh;
mod surface_mesh_processing;
mod tool;
mod topology;
mod voronoi_2;

pub mod generated;

#[cfg(test)]
mod exact_test_helper;

pub use expansion::{
    compress_expansion, expansion_sum, fast_two_sum, grow_expansion, negate_expansion,
    scale_expansion, scalar_product, scalar_sum, sign_of_expansion, two_diff, two_product,
    two_sum, ExpansionError, Sign, MAX_EXPANSION_INCIRCLE, MAX_EXPANSION_INSPHERE,
    MAX_EXPANSION_ORIENT2, MAX_EXPANSION_ORIENT3,
};
pub use features::{encode_topology_features_10d, encode_topology_features_10d_with_connectivity, FeatureError};
pub use gpu::{
    emit_geometry_wgsl, evaluate_orientation_batch_f32, GeometryGpuError, GeometryGpuKernel,
    GeometryGpuSchedule, GPU_ORIENTATION_UNCERTAIN,
};
pub use hull::{
    convex_hull_2, convex_hull_indices_2, convex_hull_indices_2_with_kernel,
    convex_hull_tensor_xy, convex_hull_tensor_xy_with_kernel, is_ccw_strongly_convex_2,
    is_ccw_strongly_convex_2_with_kernel, HullError,
};
pub use determinism_corpus::compute_corpus_hash;
pub use exact_kernel::{construct_segment_intersection, orientation_2_exact, ExactConstructionKernel, ExactPoint2};
pub use incircle::incircle;
pub use insphere::insphere;
pub use kernel::{FilteredF64Kernel, GeometryKernel};
pub use orient3d::orient_3d;
pub use primitives::{orientation_2, orientation_2_tensor_xy, Orientation, Point2, Point3};
pub use tool::{execute_geometry_tool_json, GeometryToolError};
pub use topology::{
    build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge, TopologyError,
    TopologySummary, INVALID_INDEX,
};
pub use surface_mesh::{
    build_surface_mesh_maps, BoundaryLoopWalker, FaceLoopCirculator, OneRingCirculator,
    SurfaceMeshError, SurfaceMeshView,
};
pub use surface_mesh_processing::{signed_volume, surface_area, MeshMeasureError};
pub use gpu_3d::{
    evaluate_point_in_tetra_batch_f32, gpu_filter_point_in_tetra_f32, point_in_tetra_wgsl,
    Gpu3dError, POINT_IN_TETRA_BOUNDARY, POINT_IN_TETRA_INSIDE, POINT_IN_TETRA_OUTSIDE,
    POINT_IN_TETRA_STRIDE, POINT_IN_TETRA_UNCERTAIN,
};
pub use polygon_soup::{
    build_face_adjacency, count_flipped, filter_degenerate_faces, merge_vertices,
    orient_consistently, repair_polygon_soup, FaceAdjacencyEntry, RepairReport, SoupError,
};
pub use combinatorial_map::{
    darts_to_half_edges, half_edges_to_darts, validate_combinatorial_map, CMapError, Dart,
};
pub use csr_adjacency::{
    build_face_adjacency_csr, build_vertex_adjacency_csr, required_face_neighbours,
    required_face_offsets, required_vertex_neighbours, required_vertex_offsets, CsrError,
    CsrHeader, CsrSummary,
};
pub use connectivity::{
    compute_connectivity, count_boundary_loops, euler_characteristic, genus_from_euler,
    label_components, ConnectivityError, ConnectivitySummary,
};
pub use spatial_order::{
    hilbert_encode_2d, morton_decode_2d, morton_encode_2d, morton_encode_3d,
    sort_by_hilbert_2d, sort_by_morton_2d, sort_by_morton_3d, SpatialOrderError,
    SpatialOrderHeader,
};
pub use distance::{
    distance_2d, distance_3d, distance_sq_2d, distance_sq_3d, point_segment_distance_2d,
    point_segment_distance_sq_2d, point_segment_distance_sq_3d, point_line_distance_sq_2d,
    point_triangle_distance_sq_3d, segment_segment_intersect_2d, ray_triangle_intersect_3d,
    Aabb, RayTriangleHit, RayTriangleResult, SegmentIntersection2d,
};
pub use bvh::{
    build_bvh_recursive, query_closest, query_overlap, BvhError, BvhNode, BVH_NODE_SIZE,
    MAX_BVH_DEPTH,
};
pub use kd_tree::{
    build_kd_tree_3d, query_nearest_3d, query_radius_3d, KdNode, KdTreeError, KD_NODE_SIZE,
    MAX_KD_DEPTH,
};
pub use box_join::{box_join_brute_force, box_join_bvh, BoxJoinError, BoxPair};
pub use gpu::{
    evaluate_aabb_overlap_batch_f32, gpu_candidate_box_join, merge_aabb_overlap_results,
    GPU_OVERLAP_NO, GPU_OVERLAP_UNCERTAIN, GPU_OVERLAP_YES,
};
pub use query_frontend::{QueryFrontendError, QueryStats, SpatialIndexQuery};
pub use delaunay_2::{
    delaunay_triangulation_2, triangulation_hash, verify_delaunay, DelaunayError,
};
pub use voronoi_2::{
    circumcenter, nearest_site_brute_force, nearest_site_via_delaunay, voronoi_diagram_2,
    voronoi_hash, verify_voronoi_vertices, VoronoiEdge, VoronoiError, VoronoiVertex,
};
pub use corpus_4::{
    run_full_corpus, corpus_hash, run_orientation_corpus, run_incircle_corpus,
    run_delaunay_corpus,
};
pub use constrained_delaunay::{
    conforming_delaunay_2, constraint_edge_present, ConstrainedDelaunayError,
};
pub use boolean_2::{
    boolean_union_area, boolean_intersection_area, boolean_difference_area,
    polygon_area, polygon_signed_area, point_in_polygon, verify_area_conservation,
    BooleanOp, BooleanError,
};
pub use minkowski_2::{
    minkowski_sum_2, minkowski_difference_2, minkowski_sum_brute_force, MinkowskiError,
};
pub use hull_3::{convex_hull_3, convex_hull_3_with_kernel, required_hull_3_faces, Hull3Error};
pub use delaunay_3::{
    delaunay_tetrahedralization_3, delaunay_tetrahedralization_3_with_kernel,
    required_tetrahedra_3, tetrahedralization_hash, verify_delaunay_3,
    verify_delaunay_3_with_kernel, Delaunay3Error,
};
pub use tri_tri_3::{
    required_self_intersection_pairs, self_intersecting_pairs,
    self_intersecting_pairs_with_kernel, tri_tri_intersect_3, tri_tri_intersect_3_with_kernel,
    TriPair, TriTriError, TriTriSegment,
};
pub use boolean_3::{
    boolean_3, boolean_3_with_kernel, required_triangles_3, required_vertices_3,
    Boolean3Error, Boolean3Op,
};
pub use decimate_3::{
    decimate_qem, decimate_qem_with_kernel, required_triangles, required_vertices,
    DecimateError, DecimateOptions, DecimateReport,
};
pub use exact_construct_3::{
    construct_segment_plane_intersection_3, construct_segment_triangle_intersection_3,
    orient_3d_exact_3, segment_plane_parameter_sign, Exact3Error, ExactPoint3,
    ParameterSpan, TriangleContainment,
};
pub use remesh_3::{
    isotropic_remesh, isotropic_remesh_with_kernel, required_output_capacity,
    RemeshError, RemeshOptions, RemeshReport,
};

/// Versioned native geometry ABI. Increment only when public POD layouts or
/// caller-buffer contracts change.
pub const GEOMETRY_ABI_VERSION: u32 = 1;
