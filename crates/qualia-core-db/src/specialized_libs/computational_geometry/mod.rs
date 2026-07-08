//! Native computational geometry for QualiaDB.
//!
//! A clean-room Rust implementation of the core computational-geometry algorithm
//! families, built directly on the QualiaDB engine (10-D tensor, `.10d`
//! container, `wgpu`/Forge, WASM, renderer). The functionality-specification
//! reference is de Berg, Cheong, van Kreveld & Overmars, *Computational
//! Geometry: Algorithms and Applications* (3rd ed.) — used as a public,
//! textbook description of the algorithms and their correctness properties, not
//! a source of code. Algorithms are expressed in Rust over caller-owned slices
//! so the same implementation runs in the native engine and browser/WASM
//! builds. Geometry is not renderer-only: the topology types are graph
//! structures usable by the query, reasoning, simulation, and 10-D manifold
//! layers.

/// P13.5 — Advancing-front surface (2-D) and volume (3-D) meshing with
/// deterministic shortest-edge/face-first ordering, self-crossing guards,
/// and typed obstructions.
pub mod advancing_front;
/// P10.3 — Allocation counter for zero-heap hot-path verification (test-only).
#[cfg(test)]
pub mod allocation_counter;
mod alpha_shape;
/// P13.8 - Anisotropic remeshing with crease/feature preservation and
/// BVH-pruned surface projection.
pub mod anisotropic_remesh;
/// P12.6 — Radial sort and Weiler 3-D arrangement model.
pub mod arrangement_3d;
/// P11.8 — Arrangements, point-line duality, and topological sweep.
pub mod arrangements;
/// P9.5 — Authoring ergonomics: primitives, transforms, scene graph, .10d export.
pub mod authoring;
/// P10.7 — Benchmark + adversarial corpus baseline (versioned corpora,
/// reproducible latency/allocation/hash reports).
pub mod benchmark_corpus;
/// P11.2 — Bentley-Ottmann sweep and output-sensitive red/blue intersection.
pub mod bentley_ottmann;
pub mod boolean_2;
pub mod boolean_3;
mod box_join;
mod bvh;
/// P11.13 — Rotating calipers (diameter, width, antipodal pairs) and
/// smallest enclosing disk (Welzl randomized incremental, seeded determinism).
pub mod calipers_enclosing_disk;
/// P9.4 — qapp/MCP capability manifests (per-op resource limits; backends).
pub mod capability_manifests;
/// P12.4 — Per-facet exact constrained Delaunay re-triangulation.
pub mod cdt_retriangulation;
/// P8.4 — CkNN density → graph Laplacian → Laplace-Beltrami.
pub mod cknn_laplacian;
mod combinatorial_map;
mod connectivity;
pub mod constrained_delaunay;
/// Convex decomposition (Hertel-Mehlhorn + triangulation-only).
pub mod convex_decomposition;
/// P12.3 — Exact 3D mesh co-refinement (split meshes along intersection curves).
pub mod corefine_3d;
mod corpus_4;
/// P12.9 — CSG/arrangement .10d sections and repair operations.
pub mod csg_section;
mod csr_adjacency;
/// P11.3 — DCEL subdivision, overlay, and full polygon-set boolean output
/// (union/intersection/difference/xor with boundary cycles + holes; Euler and
/// area identities).
pub mod dcel_overlay;
pub mod ddg_operators;
mod decimate_3;
pub mod delaunay_2;
mod delaunay_3;
/// P13.2 — Delaunay refinement for PSLGs with Steiner points (Ruppert).
pub mod delaunay_refine;
mod determinism_corpus;
pub mod deterministic_geometry;
mod distance;
/// P12.2 — Exact 2D arrangement with exact-construction intersection points.
pub mod exact_arrangement;
mod exact_construct_3;
mod exact_kernel;
mod expansion;
mod features;
pub mod fem_certificate;
/// P10.6 — Independent oracle and fixture licence registry (origin, licence,
/// checksum, permitted use; rejects copyleft; textbook = invariant reference
/// only, no copied material).
pub mod fixture_registry;
pub mod geometry_integration;
/// P10.5 — Geometry workspace: caller-owned arenas with byte budgets,
/// deterministic partition/reduction order, and cancellation.
pub mod geometry_workspace;
pub mod gpu;
pub mod gpu_3d;
/// P8.7 — GPU acceleration + CPU oracle for P8 batches.
pub mod gpu_oracle;
/// P11.9 — Half-plane intersection (sort-and-intersect + deque) and 2-D
/// randomized incremental linear programming (Seidel) with seeded determinism
/// and feasible/infeasible/unbounded certificates.
pub mod half_plane_lp;
/// P11.14 — Ham-sandwich cuts, centrepoints, and directional-width coresets.
pub mod ham_sandwich;
mod hull;
mod hull_3;
mod incircle;
mod insphere;
mod isosurface;
mod kd_tree;
mod kernel;
/// P11.7 — Kirkpatrick hierarchy for guaranteed O(log n) point location
/// in triangulated planar subdivisions.
pub mod kirkpatrick;
mod laplacian_3d;
pub mod math_geometry;
/// P13.1 — Mesh quality metrics (tri/tet min/max angle, radius-edge, aspect,
/// scaled Jacobian, dihedral) and isotropic size / anisotropic metric fields.
pub mod mesh_quality;
mod minkowski_2;
pub mod mixed_cell_topology;
pub mod motion_planning;
/// P12.7 — Arbitrary n-ary boolean-expression evaluator.
pub mod nary_boolean;
/// P12.1 — N-ary CSG operations on 2D polygons and 2D mesh co-refinement.
pub mod nary_csg;
/// P8.5 — Natural-neighbour interpolation (Sibson / Laplace weights).
pub mod natural_neighbour;
/// P8.6 — Nearest-neighbour inference query (radius + kNN).
pub mod nn_query;
mod orient3d;
pub mod parametric_cad;
/// P8.2 — Persistent homology: deterministic reduction → barcode.
pub mod persistence;
/// P11.6 — Point location in planar subdivisions (walking + slab decomposition).
pub mod point_location;
mod point_set_3d;
pub mod polygon_soup;
/// P11.4 — Simple-polygon, polygon-with-holes, and PSLG validation.
pub mod polygon_validation;
mod primitives;
/// P13.4 — Quadtree/octree balanced meshing (size-field refinement, 2:1
/// balance, conforming 2-D triangulation with hanging-node templates, 3-D
/// hex/tet extraction).
pub mod quad_octree_mesh;
pub mod query_frontend;
/// P11.12 — Simplex/halfspace range reporting with partition and cutting trees.
pub mod range_reporting;
/// P11.10 — Interval, segment, hereditary segment, priority-search and range
/// trees.
pub mod range_trees;
mod recon_section;
mod reconstruct_3d;
mod remesh_3;
pub mod screened_poisson;
/// P11.1 — Robust segment/line/ray primitives and exact intersections.
pub mod segment_intersection_2;
/// P12.8 — Coplanar-region simplification and topology-preserving snap rounding.
pub mod simplify_snap;
/// P12.2 — Simulation of Simplicity for deterministic degeneracy resolution.
pub mod sos;
mod spatial_order;
/// P8.3 — Statistical manifold: Fisher metric + KL as Bregman divergence.
pub mod statistical_manifold;
mod surface_mesh;
mod surface_mesh_processing;
mod tda;
/// P13.7 — Tetrahedral quality improvement and sliver handling
/// (flip / smooth / insert / exude passes; domain + orientation preserved;
/// monotonic quality improvement).
pub mod tet_quality_improve;
mod tool;
mod topology;
/// P11.6 gap — Trapezoidal map with randomized incremental point location
/// (search DAG, seeded determinism, O(log n) expected query).
pub mod trapezoidal_map;
mod tri_tri_3;
/// P11.5 — Monotone partition, linear monotone triangulation, ear fallback.
pub mod triangulation_2;
/// P13.3 — Optimal fixed-vertex triangulation objectives (edge-flip hill-climbing).
pub mod triangulation_opt;
mod voronoi_2;
/// P11.11 — Segment-site, farthest-site and higher-order Voronoi diagrams.
pub mod voronoi_variants;
/// P8.1 — VR / alpha filtration over Tensor10D point cloud.
pub mod vr_filtration;

#[cfg(test)]
mod exact_test_helper;

pub use advancing_front::{
    advancing_front_tetrahedralise_3d, advancing_front_triangulate_2d, FrontError, FrontOptions2d,
    FrontOptions3d, FrontResult2d, FrontResult3d,
};
pub use alpha_shape::{
    alpha_shape_2d, alpha_shape_3d, alpha_shape_hash, max_triangles, AlphaEdge, AlphaShapeError,
    AlphaShapeReport, EdgeClass, TriangleClass,
};
pub use anisotropic_remesh::{
    anisotropic_remesh, required_anisotropic_output_capacity, AnisotropicRemeshError,
    AnisotropicRemeshOptions, AnisotropicRemeshReport, FeatureEdge,
};
pub use arrangement_3d::{
    build_arrangement_3d, radial_sort_around_edge, validate_arrangement, Arrangement3D,
    ArrangementError as Arrangement3DError, EdgeKey, Region, Shell,
};
pub use benchmark_corpus::{
    compute_p10_corpus_baseline_hash, run_p10_corpus, CorpusReport, CORPUS_VERSION,
};
pub use bentley_ottmann::{
    bentley_ottmann_intersections, brute_force_intersections, brute_force_red_blue_intersections,
    red_blue_intersections, SweepSegment,
};
pub use boolean_2::{
    boolean_difference_area, boolean_intersection_area, boolean_union_area, point_in_polygon,
    polygon_area, polygon_signed_area, verify_area_conservation, BooleanError, BooleanOp,
};
pub use boolean_3::{
    boolean_3, boolean_3_exact, boolean_3_with_kernel, required_triangles_3, required_vertices_3,
    Boolean3Error, Boolean3Op,
};
pub use box_join::{box_join_brute_force, box_join_bvh, BoxJoinError, BoxPair};
pub use bvh::{
    build_bvh_recursive, query_closest, query_overlap, BvhError, BvhNode, BVH_NODE_SIZE,
    MAX_BVH_DEPTH,
};
pub use calipers_enclosing_disk::{
    diameter_and_width, rotating_calipers, smallest_enclosing_disk, AntipodalPair, CalipersError,
    CalipersResult, Disk, EnclosingDisk,
};
pub use cdt_retriangulation::{cdt_retriangulate_facet, CdtError};
pub use combinatorial_map::{
    darts_to_half_edges, half_edges_to_darts, validate_combinatorial_map, CMapError, Dart,
};
pub use connectivity::{
    compute_connectivity, count_boundary_loops, euler_characteristic, genus_from_euler,
    label_components, ConnectivityError, ConnectivitySummary,
};
pub use constrained_delaunay::{
    conforming_delaunay_2, constraint_edge_present, ConstrainedDelaunayError,
};
pub use convex_decomposition::{
    convex_decomposition_hm, convex_decomposition_triangulation, is_convex_polygon,
    verify_convex_decomposition,
};
pub use corefine_3d::{
    corefine_3d, count_shared_vertices, verify_refinement_preserves_triangles,
    CorefinementResult3D, Mesh3D as Mesh3DCorefine,
};
pub use corpus_4::{
    corpus_hash, run_delaunay_corpus, run_full_corpus, run_incircle_corpus, run_orientation_corpus,
};
pub use csg_section::{
    decode_csg_section, deserialize_expr, encode_csg_section, repair_mesh, serialize_expr,
    CsgSection, CsgSectionError, DecodedCsgSection, RepairReport as MeshRepairReport, CSG_MAGIC,
    CSG_TYPE_ARRANGEMENT, CSG_TYPE_EXPRESSION, CSG_TYPE_REPAIR_REPORT, CSG_VERSION,
};
pub use csr_adjacency::{
    build_face_adjacency_csr, build_vertex_adjacency_csr, required_face_neighbours,
    required_face_offsets, required_vertex_neighbours, required_vertex_offsets, CsrError,
    CsrHeader, CsrSummary,
};
pub use ddg_operators::{
    boundary_of_boundary_is_zero, boundary_of_simplex, cotangent_laplacian, curvature_angle_defect,
    encode_ddg_operator_section, face_vector_areas, geodesic_distances_dijkstra,
    harmonic_parameterize_disk, heat_step, hodge_decomposition_summary, mean_curvature_flow_step,
    parallel_transport_angle, solve_poisson_jacobi, surface_area_gradient, vertex_normals,
    ChainTerm, CurvatureSample, DdgError, DdgReport, DdgSectionHeader, HodgeSummary, SparseEntry,
    DDG_SECTION_MAGIC, DDG_SECTION_VERSION,
};
pub use decimate_3::{
    decimate_qem, decimate_qem_with_kernel, required_triangles, required_vertices, DecimateError,
    DecimateOptions, DecimateReport,
};
pub use delaunay_2::{
    delaunay_triangulation_2, triangulation_hash, verify_delaunay, DelaunayError,
};
pub use delaunay_3::{
    delaunay_tetrahedralization_3, delaunay_tetrahedralization_3_with_kernel,
    required_tetrahedra_3, tetrahedralization_hash, verify_delaunay_3,
    verify_delaunay_3_with_kernel, Delaunay3Error,
};
pub use delaunay_refine::{
    delaunay_refine_2, verify_refined_mesh, RefineError, RefineOptions,
    RUPPERT_TERMINATION_BOUND_DEG,
};
pub use determinism_corpus::compute_corpus_hash;
pub use deterministic_geometry::{
    all_nearest_neighbours, beta_skeleton_graph, bichromatic_nearest_pair,
    build_batch_dynamic_kd_tree, conflict_graph_pairs, deterministic_pack_u32,
    deterministic_reduce_f64, deterministic_scan_u64, dynamic_kd_compact, dynamic_kd_delete,
    dynamic_kd_insert, dynamic_kd_nearest, euclidean_mst, external_memory_nearest_tiles,
    gabriel_graph, generate_spatial_points, greedy_spanner, knn_graph, list_rank_successors,
    nearest_pair, reproducible_benchmark_report, reservation_batch_hull_2d,
    seeded_incremental_order, smallest_enclosing_ball, tree_contract_roots, well_separated_pairs,
    BatchDynamicKdTree, BenchmarkReport, ConflictEdge, DeterministicGeometryError, DynamicKdRecord,
    EnclosingBall, NearestPair, SpatialGeneratorKind, WeightedEdge, WellSeparatedPair,
};
pub use distance::{
    distance_2d, distance_3d, distance_sq_2d, distance_sq_3d, point_line_distance_sq_2d,
    point_segment_distance_2d, point_segment_distance_sq_2d, point_segment_distance_sq_3d,
    point_triangle_distance_sq_3d, ray_triangle_intersect_3d, segment_segment_intersect_2d, Aabb,
    RayTriangleHit, RayTriangleResult, SegmentIntersection2d,
};
pub use exact_arrangement::{
    build_exact_arrangement, max_coordinate_error, verify_euler, verify_general_position_counts,
    zone_traversal, ArrangementEdge, ArrangementError, ArrangementVertex, ExactArrangement,
    ExactLine2, ZoneTraversal,
};
pub use exact_construct_3::{
    construct_segment_plane_intersection_3, construct_segment_triangle_intersection_3,
    orient_3d_exact_3, segment_plane_parameter_sign, Exact3Error, ExactPoint3, ParameterSpan,
    TriangleContainment,
};
pub use exact_kernel::{
    construct_segment_intersection, orientation_2_exact, ExactConstructionKernel, ExactPoint2,
};
pub use expansion::{
    compress_expansion, expansion_sum, fast_two_sum, grow_expansion, negate_expansion,
    scalar_product, scalar_sum, scale_expansion, sign_of_expansion, two_diff, two_product, two_sum,
    ExpansionError, Sign, MAX_EXPANSION_INCIRCLE, MAX_EXPANSION_INSPHERE, MAX_EXPANSION_ORIENT2,
    MAX_EXPANSION_ORIENT3,
};
pub use features::{
    encode_topology_features_10d, encode_topology_features_10d_with_connectivity, FeatureError,
};
pub use fem_certificate::{
    build_fem_certificate, decode_fem_certificate, encode_fem_certificate, BoundaryMarker,
    FemCertificate, FemCertificateError, MaterialRegion, FEM_CERT_ENCODED_LEN, FEM_CERT_MAGIC,
    FEM_CERT_VERSION,
};
pub use fixture_registry::{
    validate_records, FixtureOrigin, FixtureRecord, FixtureRegistry, FixtureRegistryError,
    LicenceKind, UsePermission, SEED_FIXTURES,
};
pub use geometry_integration::{
    build_section_header, cancel_stream_at, closeout_summary, encode_section_header,
    plan_stream_chunks, renderer_descriptor, schema_descriptor, validate_conformance,
    validate_conformance_matrix, validate_operation_descriptor, GeometryBackendKind,
    GeometryCloseoutRecord, GeometryConformanceMatrixRow, GeometryConformanceRecord,
    GeometryEvidenceLevel, GeometryOperationDescriptor, GeometryRenderableDescriptor,
    GeometrySchemaDescriptor, GeometrySchemaKind, GeometrySectionHeader, GeometryStreamChunk,
    IntegrationError, GEOMETRY_SECTION_MAGIC,
};
pub use geometry_workspace::{
    deterministic_partition, deterministic_reduce, Cancellation, GeometryWorkspace, WorkspaceError,
    DEFAULT_WORKSPACE_BUDGET,
};
pub use gpu::{
    emit_geometry_wgsl, evaluate_orientation_batch_f32, GeometryGpuError, GeometryGpuKernel,
    GeometryGpuSchedule, GPU_ORIENTATION_UNCERTAIN,
};
pub use gpu::{
    evaluate_aabb_overlap_batch_f32, gpu_candidate_box_join, merge_aabb_overlap_results,
    GPU_OVERLAP_NO, GPU_OVERLAP_UNCERTAIN, GPU_OVERLAP_YES,
};
pub use gpu_3d::{
    evaluate_point_in_tetra_batch_f32, gpu_filter_point_in_tetra_f32, point_in_tetra_wgsl,
    Gpu3dError, POINT_IN_TETRA_BOUNDARY, POINT_IN_TETRA_INSIDE, POINT_IN_TETRA_OUTSIDE,
    POINT_IN_TETRA_STRIDE, POINT_IN_TETRA_UNCERTAIN,
};
pub use half_plane_lp::{
    half_plane_intersection, linear_program_2d, HalfPlane, HalfPlaneIntersection, LpResult2d,
};
pub use ham_sandwich::{
    centrepoint, directional_width, ham_sandwich_cut, tukey_depth, width, width_coreset,
    Centrepoint, HamSandwichCut, WidthCoreset,
};
pub use hull::{
    convex_hull_2, convex_hull_indices_2, convex_hull_indices_2_with_kernel, convex_hull_tensor_xy,
    convex_hull_tensor_xy_with_kernel, is_ccw_strongly_convex_2,
    is_ccw_strongly_convex_2_with_kernel, HullError,
};
pub use hull_3::{convex_hull_3, convex_hull_3_with_kernel, required_hull_3_faces, Hull3Error};
pub use incircle::incircle;
pub use insphere::insphere;
pub use isosurface::{isosurface_hash, marching_cubes, IsosurfaceError};
pub use kd_tree::{
    build_kd_tree_3d, query_nearest_3d, query_radius_3d, KdNode, KdTreeError, KD_NODE_SIZE,
    MAX_KD_DEPTH,
};
pub use kernel::{
    ConstructionKernel, ExactPoint2 as KernelExactPoint2, FilteredF64Kernel, GeometryKernel,
    Unsupported,
};
pub use kirkpatrick::{KirkpatrickError, KirkpatrickHierarchy};
pub use laplacian_3d::{
    cknn_laplacian_3d, cknn_laplacian_normalised_3d, verify_laplacian_properties, LaplacianError,
};
pub use math_geometry::{
    barycentric_tetra, best_translation_registration, caratheodory_reduce_3d,
    convexity_certificate_polyline, cross_ratio_1d, curve_differential, frame_to_world,
    householder_reflect, hyperplane_eval, point_from_projective, project_points_to_plane,
    projective_from_point, quaternion_normalize, quaternion_slerp, quaternion_to_matrix,
    schur_complement_2x2, separating_plane_aabb, so3_exp, so3_log, solve_diagonal_quadratic,
    surface_patch_differential, world_to_frame, AffineFrame3, ConvexityCertificate,
    CurveDifferential, HomogeneousPoint3, Hyperplane3, MathGeometryError, QuadraticSolution,
    Quaternion, SeparatingPlane, SurfaceDifferential,
};
pub use mesh_quality::{
    check_field_conformance_tet, check_field_conformance_tri, tet_mesh_quality_slice, tet_quality,
    tet_quality_points, tri_mesh_quality_2d, tri_quality, tri_quality_points, tri_signed_area_2d,
    AnisotropyField, FieldConformance, MeshQualityError, MetricTensor, SizeField,
    TetMeshQualityStats, TetQuality, TriMeshQualityStats, TriQuality,
};
pub use minkowski_2::{
    minkowski_difference_2, minkowski_sum_2, minkowski_sum_brute_force, minkowski_sum_convex,
    minkowski_sum_non_convex, MinkowskiError,
};
pub use mixed_cell_topology::{
    extract_boundary_faces, hex_to_tetrahedra, quad_to_triangles, tets_to_mixed,
    triangles_to_mixed, validate_mixed_cells, BoundaryFace, MixedCell, MixedTopologyError,
    MixedTopologyReport, CELL_KIND_HEX, CELL_KIND_QUAD, CELL_KIND_TETRA, CELL_KIND_TRIANGLE,
};
pub use motion_planning::{
    belief_update_2d, configuration_obstacle_translate, continuous_segment_collision,
    coverage_lawnmower, feedback_vector, forward_kinematics_2d, interpolate_pose2, jacobian_2d,
    kinodynamic_propagate, multi_robot_conflict_free, normalize_pose, normalize_pose3,
    pursuit_step, rrt_plan, seeded_roadmap, segment_collision_free, time_parameterize_path,
    visibility_graph_path, BeliefState2, CollisionReport, Control2, Joint2, PlannedPath,
    PlanningError, PolygonObstacle, Pose2, Pose3, TimedPose2,
};
pub use nary_boolean::{
    evaluate_expr, nary_boolean, BoolExpr, MeshInput, NaryBoolError, RegionMask, MAX_OPERANDS,
};
pub use nary_csg::{
    corefine_2d, nary_csg, verify_pairwise_inclusion_exclusion, CorefinementResult2D, Mesh2D,
    NaryCsgError, NaryCsgResult, NaryOp, PolygonWithHoles as NaryPolygonWithHoles,
};
pub use orient3d::orient_3d;
pub use parametric_cad::{
    bezier_derivative_eval, bezier_eval, bspline_eval, bspline_insert_uniform_knot,
    budgeted_evolution_optimize, classify_trim_uv, continuity_between_curves, helical_lattice,
    latin_hypercube_samples, loft_profiles, nurbs_eval, offset_polyline, rbf_surrogate_predict,
    revolve_profile, sdf_compose, sdf_eval, shape_distance, tensor_surface_eval,
    tensor_surface_sample, tube_along_polyline, validate_parameters,
    vascular_lattice_adapter_notice, CadError, ContinuityReport, DomainAdapterNotice,
    NurbsControlPoint, OptimizationReport, ParameterConstraint, SdfOp, SdfPrimitive, ShapeDistance,
    SurfaceSample, SurrogatePrediction,
};
pub use point_location::{
    build_slab_map, locate_point, point_in_triangle, point_strictly_in_triangle,
    triangulation_to_subdivision, walk_locate, LocateResult, PointLocationError, SlabMap,
    SubdivisionEdge,
};
pub use point_set_3d::{
    average_spacing_3d, cknn_graph_3d, cknn_hash, knn_all_brute_force_3d, knn_brute_force_3d,
    knn_hash, knn_search_3d, local_density_3d, mean_knn_distance_3d, remove_outliers_3d, CknnEdge,
    KnnEntry, PointSetError, MAX_K,
};
pub use polygon_soup::{
    build_face_adjacency, count_flipped, filter_degenerate_faces, merge_vertices,
    orient_consistently, repair_polygon_soup, FaceAdjacencyEntry, RepairReport, SoupError,
};
pub use polygon_validation::{
    canonicalize_polygon_with_holes, canonicalize_simple_polygon, repair_for,
    validate_polygon_with_holes, validate_pslg, validate_simple_polygon, PolygonWithHoles,
    PslgEdge, RepairSuggestion, ValidationIssue, ValidationReport,
};
pub use primitives::{orientation_2, orientation_2_tensor_xy, Orientation, Point2, Point3};
pub use quad_octree_mesh::{
    balance_octtree_2to1, balance_quadtree_2to1, build_octtree, build_quadtree, const_size_fn_3d,
    octtree_to_hexahedra, octtree_to_tetrahedra, quadtree_to_triangles, size_field_2d_fn,
    size_target_refiner_2d, size_target_refiner_3d, OctLeaf, OctMeshError, OctMeshOptions, OctNode,
    OctTree, QuadLeaf, QuadMeshError, QuadMeshOptions, QuadNode, QuadTree, OCT_MAX_LEVEL,
    QUAD_MAX_LEVEL,
};
pub use query_frontend::{QueryFrontendError, QueryStats, SpatialIndexQuery};
pub use range_reporting::{CuttingTree, Halfspace2, KdTree2, PartitionTree};
pub use recon_section::{
    crc32c, decode_recon_section, encode_recon_section, recon_hash, DecodedRecon,
    ReconSectionError, RECON_HEADER_SIZE, RECON_MAGIC, RECON_TYPE_ALPHA_SHAPE_2D,
    RECON_TYPE_ALPHA_SHAPE_3D, RECON_TYPE_ISOSURFACE, RECON_TYPE_LAPLACIAN, RECON_TYPE_PERSISTENCE,
    RECON_VERSION,
};
pub use reconstruct_3d::{poisson_reconstruct_3d, ReconstructionError};
pub use remesh_3::{
    isotropic_remesh, isotropic_remesh_with_kernel, required_output_capacity, RemeshError,
    RemeshOptions, RemeshReport,
};
pub use screened_poisson::{
    required_screened_poisson_capacity, screened_poisson_reconstruct_3d, ScreenedPoissonError,
    ScreenedPoissonOptions, ScreenedPoissonReport,
};
pub use segment_intersection_2::{
    classify_and_construct, classify_segment_intersection_2, line_segment_intersection_2,
    ray_segment_intersection_2, SegmentIntersectionClass, SegmentIntersectionResult, TJunctionSide,
};
pub use simplify_snap::{
    simplify_coplanar_regions, snap_round_3d, SimplifyError, SimplifyOptions, SimplifyResult,
};
pub use sos::orient_3d_sos;
pub use spatial_order::{
    hilbert_encode_2d, morton_decode_2d, morton_encode_2d, morton_encode_3d, sort_by_hilbert_2d,
    sort_by_morton_2d, sort_by_morton_3d, SpatialOrderError, SpatialOrderHeader,
};
pub use surface_mesh::{
    build_surface_mesh_maps, BoundaryLoopWalker, FaceLoopCirculator, OneRingCirculator,
    SurfaceMeshError, SurfaceMeshView,
};
pub use surface_mesh_processing::{signed_volume, surface_area, MeshMeasureError};
pub use tda::{
    alpha_filtration_2d, compute_persistence, persistence_hash, PersistencePair, Simplex, TdaError,
};
pub use tet_quality_improve::{
    improve_tet_mesh, verify_improvement, TetImproveError, TetImproveObjective, TetImproveOptions,
    TetImproveResult,
};
pub use tool::{execute_geometry_tool_json, GeometryToolError};
pub use topology::{
    build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge, TopologyError,
    TopologySummary, INVALID_INDEX,
};
pub use trapezoidal_map::{TmSegment, Trapezoid, TrapezoidalMap, TrapezoidalMapError};
pub use tri_tri_3::{
    required_self_intersection_pairs, self_intersecting_pairs, self_intersecting_pairs_with_kernel,
    tri_tri_intersect_3, tri_tri_intersect_3_exact, tri_tri_intersect_3_with_kernel,
    ExactTriTriSegment, TriPair, TriTriError, TriTriSegment,
};
pub use triangulation_2::{
    triangulate_ear_clipping, triangulate_monotone, triangulate_polygon, verify_triangulation,
    Triangle,
};
pub use triangulation_opt::{
    evaluate_objective, optimise_and_evaluate, optimise_triangulation, TriObjective,
    TriangulationOptError,
};
pub use voronoi_2::{
    circumcenter, nearest_site_brute_force, nearest_site_via_delaunay, verify_voronoi_vertices,
    voronoi_diagram_2, voronoi_hash, VoronoiEdge, VoronoiError, VoronoiVertex,
};

/// Versioned native geometry ABI. Increment only when public POD layouts or
/// caller-buffer contracts change.
pub const GEOMETRY_ABI_VERSION: u32 = 1;
