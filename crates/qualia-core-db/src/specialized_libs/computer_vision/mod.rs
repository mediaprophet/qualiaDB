//! Computer vision specialized library (MIG-V2).
//!
//! Pure, native algorithms formerly under `qualia-vision::{cv,ops,sr,bio,…}`.
//! Product surface (biosense consent, recipes, weights, capability registry)
//! remains in the `qualia-vision` crate and re-exports this module.
//!
//! # WASM / size gate
//! Gated off slim portal/ontology WASM (`cfg(not(target_arch = "wasm32"))` at
//! the specialized_libs parent). Sealed `.10d` browse does **not** need this
//! module — only mesh/spectral/render paths.

pub mod bio;
pub mod cv;
pub mod embeddings;
pub mod gpu;
pub mod ops;
pub mod spatial;
pub mod sr;
pub mod types;

pub use cv::{
    bicubic_u8, bilateral_denoise_u8, bilinear_u8, box_blur_u8, brief_desc_u8, canny_u8, dilate_u8,
    draw_rect_u8, equalize_hist_u8, erode_u8, fast_corners_u8, find_external_blobs,
    gaussian_blur_u8, hamming_match, histogram_u8, lanczos3_u8, lucas_kanade_step, median_blur_u8,
    rgb_to_gray_u8, sobel_mag_u8, synthetic_pulse_sequence, warp_affine_u8, warp_perspective_u8,
    BlobBox, CvError, FrameSequence, GrayView, Keypoint, Match, RgbView, DESC_LEN, MAX_SEQ_FRAMES,
};
pub use embeddings::{
    ahash_u64, color_hist_embed_rgb, cosine_distance, cosine_similarity, dhash_u64,
    hamming_distance_u64, AHASH_SIDE, COLOR_HIST_BINS, COLOR_HIST_EMBED_DIM, DHASH_HEIGHT,
    DHASH_WIDTH,
};
pub use gpu::{
    avg_pool2d_dispatch, conv2d_nchw_dispatch, max_pool2d_dispatch, resize_nearest_nchw_dispatch,
    thermal_allows_gpu_tiles, ThermalHint, VisionComputeDevice, VisionComputeReport,
    VisionVramBudget,
};
pub use ops::{avg_pool2d_nchw_f32, conv2d_nchw_f32, max_pool2d_nchw_f32, resize_nearest_nchw_f32};
pub use spatial::{
    assess_twin_eligibility, class_hash_to_sigma_base, class_id_to_sigma_base,
    class_score_to_sigma, cleanup_mesh_ir, detection_center_to_node_hint, detection_to_sigma,
    mesh_ir_to_export, mesh_ir_to_export_validated, mesh_ir_to_obj, mesh_ir_to_stl_binary,
    mesh_ir_triangles, print_readiness, refuse_fea_unless_eligible, validate_mesh_ir,
    AnalysisDomain, MeshCleanupOptions, MeshIR, MeshQualityReport, MeshValidationReport,
    MeshValidationStatus, NodeHint, PrintReadiness, RenderMeshExport, TwinEligibility, MAX_INDICES,
    MAX_VERTICES,
};
pub use sr::{
    blend_tile_into_accum, estimate_tile_count, extract_tile_rgb8, finalize_blend, plan_tiles,
    super_resolve, super_resolve_tiled, super_resolve_tiled_default,
    super_resolve_tiled_with_policy, super_resolve_with_policy, ClassicalKernel, EnhancementMode,
    SrBackend, SrReport, SrRequest, TilePolicy, TileRect, DEFAULT_OVERLAP, DEFAULT_TILE,
};
pub use types::{
    Detection, ImageView, PixelFormat, VisionError, VisualCapabilities, VisualModel,
    VisualOutputCounts, MAX_DETECTIONS, MAX_EMBED_DIM,
};
