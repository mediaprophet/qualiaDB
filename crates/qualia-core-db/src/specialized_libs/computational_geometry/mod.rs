//! Native computational geometry for QualiaDB.
//!
//! This is the shared landing zone for the package-by-package CGAL capability
//! port. Algorithms are expressed in Rust over caller-owned slices so the same
//! implementation runs in the native engine and browser/WASM builds. Geometry
//! is not renderer-only: the topology types are graph structures usable by the
//! query, reasoning, simulation, and 10D manifold layers.

mod determinism_corpus;
mod exact_kernel;
mod expansion;
mod features;
mod gpu;
mod hull;
mod incircle;
mod insphere;
mod kernel;
mod orient3d;
mod primitives;
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
pub use features::{encode_topology_features_10d, FeatureError};
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

/// Versioned native geometry ABI. Increment only when public POD layouts or
/// caller-buffer contracts change.
pub const GEOMETRY_ABI_VERSION: u32 = 1;
