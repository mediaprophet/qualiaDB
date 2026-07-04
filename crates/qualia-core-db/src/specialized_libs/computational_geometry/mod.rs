//! Native computational geometry for QualiaDB.
//!
//! This is the shared landing zone for the package-by-package CGAL capability
//! port. Algorithms are expressed in Rust over caller-owned slices so the same
//! implementation runs in the native engine and browser/WASM builds. Geometry
//! is not renderer-only: the topology types are graph structures usable by the
//! query, reasoning, simulation, and 10D manifold layers.

mod bvh;
mod box_join;
mod combinatorial_map;
mod connectivity;
mod csr_adjacency;
mod determinism_corpus;
mod distance;
mod exact_kernel;
mod expansion;
mod features;
mod gpu;
mod hull;
mod incircle;
mod insphere;
mod kd_tree;
mod kernel;
mod orient3d;
pub mod query_frontend;
mod polygon_soup;
mod primitives;
mod spatial_order;
mod surface_mesh;
mod tool;
mod topology;

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

/// Versioned native geometry ABI. Increment only when public POD layouts or
/// caller-buffer contracts change.
pub const GEOMETRY_ABI_VERSION: u32 = 1;
