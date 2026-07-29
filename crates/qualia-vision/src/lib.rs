//! Qualia Vision — product surface for visual intelligence + biosense.
//!
//! # Library home (MIG-V2)
//! Pure algorithms live in
//! `qualia_core_db::specialized_libs::computer_vision` (`cv`, `ops`, `sr`,
//! `bio`, `embeddings`, `spatial` kernels, `gpu` dispatch). This crate
//! **re-exports** those kernels and owns product layers: biosense consent,
//! recipes, weights, capability registry, semantic quins, media store.
//!
//! Plans: `native-visual-intelligence-and-generative-3d.md`,
//! `vision-10d-browser-excellence-programme-2026.md` §9-B.
//!
//! # Design rules
//! - Hot path is caller-buffered, no hidden heap in `infer`.
//! - Model outputs are **epistemic observations**, not ground truth.
//! - Dense pixels / biometric templates never in NQuins.
//! - **No Python**; no OpenCV product link.

#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod bio;
pub mod biosense;
pub mod capability;
pub mod classifier;
pub mod cv;
pub mod detector;
pub mod embeddings;
pub mod generator;
pub mod gpu;
pub mod media_store;
pub mod metrics;
pub mod ops;
pub mod overlay;
pub mod preprocess;
pub mod recipes;
pub mod semantic;
pub mod spatial;
pub mod sr;
pub mod synthetic;
pub mod tracker;
pub mod types;
pub mod weights;

pub mod ade20k_taxonomy;
pub mod cifar_taxonomy;
pub mod cityscapes_taxonomy;
pub mod coco_taxonomy;
#[cfg(feature = "cpu-reference")]
pub mod cpu_reference;
pub mod kitti_taxonomy;
pub mod openimages_taxonomy;
pub mod pascal_voc_taxonomy;
pub mod vggface2_taxonomy;

pub use ade20k_taxonomy::{
    ade20k_category_count, lookup_ade20k_class_by_hash, lookup_ade20k_class_by_id,
    q_hash_ade20k_class, ADE20K_150_CLASSES,
};
pub use cifar_taxonomy::{
    lookup_cifar100_class_by_id, lookup_cifar10_class_by_id, lookup_cifar_class_by_hash,
    q_hash_cifar_class, CIFAR_100_CLASSES, CIFAR_10_CLASSES,
};
pub use cityscapes_taxonomy::{
    cityscapes_category_count, lookup_cityscapes_class_by_hash, lookup_cityscapes_class_by_id,
    q_hash_cityscapes_class, CITYSCAPES_CLASSES,
};
pub use coco_taxonomy::{
    coco_category_count, lookup_coco_class_by_hash, lookup_coco_class_by_id, q_hash_coco_class,
    COCO_80_CLASSES,
};
pub use kitti_taxonomy::{
    kitti_category_count, lookup_kitti_class_by_hash, lookup_kitti_class_by_id, q_hash_kitti_class,
    KITTI_8_CLASSES,
};
pub use openimages_taxonomy::{
    lookup_openimages_class_by_hash, lookup_openimages_class_by_mid, openimages_category_count,
    q_hash_openimages_class, OpenImagesClassEntry, OPENIMAGES_600_CLASSES,
};
pub use pascal_voc_taxonomy::{
    lookup_pascal_voc_class_by_hash, lookup_pascal_voc_class_by_id, pascal_voc_category_count,
    q_hash_pascal_voc_class, PASCAL_VOC_20_CLASSES,
};
pub use vggface2_taxonomy::{
    format_vggface2_subject_id, lookup_vggface2_subject_hash, q_hash_vggface2_subject,
    vggface2_identity_count, VGGFace2Pose, VGGFACE2_IDENTITY_COUNT,
};

pub use bio::{
    anonymize_tag_map, apply_background_correct, apply_hu_window_i16, background_intensity_sample,
    centroid_from_bbox, centroids_from_binary, centroids_from_labels, crocker_grier_link,
    extended_minima, first_order_stats, glcm_features, glcm_features_d1, hu_window_i16,
    intensity_to_od_u8, isotropic_resample_2d_nn, isotropic_resample_nn_2d, lab_to_rgb,
    link_particles, macenko_deconvolution, mip_project_axis, mip_project_z, morphological_tophat,
    nucleus_features, optical_density_rgb, otsu_threshold_from_hist, positive_od_threshold,
    reinhard_normalize, rgb_to_lab, shape_2d_features, shape_2d_from_mask, shape_3d_from_voxels,
    snmf_unmix_lite, spectral_unmix_nnls, suv_from_activity, voronoi_otsu_label, watershed_markers,
    CrockerGrierLinker, CrockerGrierParams, Detection2, FirstOrderStats, GlcmFeatures, HistoError,
    LabStats, LinkedParticle, MacenkoResult, NucleusFeature, OdPositiveIndex, ParticleCentroid,
    RadiomicsError, RgbBg, Shape2d, Shape2dFeatures, Shape3dFeatures, StainBasis, TopHatKind,
    TrackLink, DEFAULT_HE_TARGET_MEAN, DEFAULT_HE_TARGET_STD, GLCM_LEVELS_16, GLCM_LEVELS_32,
    MAX_FRAME_DETS, MAX_NUCLEUS_LABELS, MAX_OD_LABELS, MAX_PARTICLES_PER_FRAME,
    MAX_PARTICLE_TRACKS, NO_TRACK_ID,
};
pub use biosense::{
    blendshape_affect_proposal, cctv_stages_allowed, colour_evm_yiq, design_bandpass_iir,
    energy_ms, ensemble_hr, ensemble_respiration, eulerian_color_magnify,
    eulerian_color_magnify_consented, eulerian_color_magnify_ex, eulerian_motion_magnify,
    eulerian_motion_magnify_consented, eulerian_motion_magnify_ex, evaluate_challenge_pad,
    evaluate_landmark_pad, evaluate_pad_from_mediapipe_trace, evaluate_processing_act,
    evaluate_profile_asymmetry, evm_snr_gate, face_roi_center, frame_blur_score,
    gaussian_pyramid_build, issue_challenge, issue_rotation_challenge, landmarks_from_normalized,
    laplacian_pyramid_build, motion_energy, pack_landmark_frame, profile_asymmetry_ratio,
    pyramid_reconstruct, reject_low_quality, respiration_from_motion,
    respiration_from_rppg_harmonic, respiration_rate_from_motion_trace, roi_mean_rgb,
    spectral_hr_peak, template_hash_from_roi, templates_match, temporal_bandpass_iir,
    temporal_bandpass_series, valence_arousal_proposal, AffectProposal, BandpassIir, BandpassState,
    BiometricTemplate, BiosenseConsent, BiosensePurpose, BlendshapeProxy, CameraStreamAttestation,
    ChallengeKind, ColourEvmParams, EvmRefuse, EvmSnrVerdict, FaceRoi, HeadPose, HrEstimate,
    Landmark2, LandmarkBufferLayout, LandmarkFrame, MeshBlendProxies, MeshFrameSignals,
    MotionEvmParams, PadLandmarkId, PadReason, PadResult, PadThresholds, ParSample, ParVerdict,
    PolicyDecision, ProcessingAct, PyramidLevelMeta, QualityReject, RrEstimate, TemporalWindow,
    DEFAULT_EVM_MIN_SNR, DEFAULT_PAR_TAU, MAX_PYRAMID_LEVELS, MEDIAPIPE_FACE_MESH_COUNT,
    RR_F_HI_HZ, RR_F_LO_HZ, RR_MIN_SNR_DEFAULT,
};
pub use capability::{all_capabilities, by_id, count_by_status, CapabilityEntry, CapabilityStatus};
pub use classifier::{fit_two_class_centroids, LinearHead, LinearProbeVision};
pub use cv::{
    bicubic_u8, bilateral_denoise_u8, bilinear_u8, box_blur_u8, brief_desc_u8, canny_u8, dilate_u8,
    draw_rect_u8, equalize_hist_u8, erode_u8, fast_corners_u8, find_external_blobs,
    gaussian_blur_u8, hamming_match, histogram_u8, lanczos3_u8, lucas_kanade_step, median_blur_u8,
    rgb_to_gray_u8, sobel_mag_u8, synthetic_pulse_sequence, warp_affine_u8, warp_perspective_u8,
    BlobBox, CvError, FrameSequence, GrayView, Keypoint, Match, RgbView, DESC_LEN, MAX_SEQ_FRAMES,
}; // GrayView + RgbView for embeddings / recipes
pub use detector::{sample_frame_indices, GridMultiObjectDetector, MAX_GRID};
pub use embeddings::{
    ahash_u64, color_hist_embed_rgb, cosine_distance, cosine_similarity, dhash_u64,
    hamming_distance_u64, AHASH_SIDE, COLOR_HIST_BINS, COLOR_HIST_EMBED_DIM, DHASH_HEIGHT,
    DHASH_WIDTH,
};
pub use generator::{
    compile_generation_receipt_quins, CancelFlag, GenerationReceipt, NativeImageGenerator,
    CTX_GENERATION, GENERATOR_MODEL_ID, P_GENERATED_IMAGE, P_GEN_PROMPT, P_GEN_SEED,
};
pub use gpu::{
    avg_pool2d_dispatch, conv2d_nchw_dispatch, max_pool2d_dispatch, resize_nearest_nchw_dispatch,
    thermal_allows_gpu_tiles, ThermalHint, VisionComputeDevice, VisionComputeReport,
    VisionVramBudget,
};
pub use media_store::{MediaRecord, MediaStore, RetentionClass};
pub use metrics::{evaluate_real_held_out, evaluate_synthetic, mean_best_iou, MetricsReport};
pub use ops::{avg_pool2d_nchw_f32, conv2d_nchw_f32, max_pool2d_nchw_f32, resize_nearest_nchw_f32};
pub use overlay::{
    box_css_percent, box_pixel_bounds, compose_rgb_overlay_rgba8, draw_boxes_rgba8,
    encode_bmp_rgba8,
};
pub use preprocess::{
    iou_u16, letterbox_rgb8, letterbox_workspace_bytes, nms_class_agnostic,
    normalize_rgb8_to_f32_chw, resize_nearest_rgb8,
};
pub use recipes::{
    challenge_pad_from_landmark_frames, challenge_pad_from_mesh_trace,
    compile_hr_observation_quins, respiration_monitor, respiration_monitor_motion_only,
    self_monitor_pulse, self_monitor_pulse_evm, PulseAbstain, PulseEvmResult,
};
pub use semantic::{
    bbox_quin, class_proposal_quin, compile_observation_quins, compile_observation_quins_full,
    human_correct_quin, human_reject_quin, media_digest, model_digest_quin, observation_quin,
    pack_bbox_u64, q_hash, query_by_frame_range, query_by_model, query_instances_in_region,
    track_quin, unpack_bbox_u64, MediaDigest, VisionQuin, CTX_HUMAN_ATTESTATION, CTX_VISION,
    MAX_OBS_QUINS, P_HAS_BBOX, P_HAS_TRACK, P_HUMAN_CORRECTS, P_HUMAN_REJECTS, P_MODEL_DIGEST,
    P_PROPOSES_CLASS, P_VISUAL_OBSERVATION,
};
pub use spatial::{
    assess_twin_eligibility, class_hash_to_sigma_base, class_id_to_sigma_base,
    class_score_to_sigma, cleanup_mesh_ir, detection_center_to_node_hint, detection_to_sigma,
    detections_to_node_hints, image_to_heightfield_mesh, mesh_ir_to_export,
    mesh_ir_to_export_validated, mesh_ir_to_obj, mesh_ir_to_stl_binary, mesh_ir_triangles,
    pack_geometry_export_for_10d, print_readiness, refuse_fea_unless_eligible, validate_mesh_ir,
    AnalysisDomain, GeometryFor10d, ImageTo3dReceipt, MeshCleanupOptions, MeshIR,
    MeshQualityReport, MeshValidationReport, MeshValidationStatus, NodeHint, PrintReadiness,
    RenderMeshExport, TwinEligibility, MAX_INDICES, MAX_VERTICES,
};
pub use sr::{
    blend_tile_into_accum, estimate_tile_count, extract_tile_rgb8, finalize_blend, plan_tiles,
    super_resolve, super_resolve_tiled, super_resolve_tiled_default,
    super_resolve_tiled_with_policy, super_resolve_with_policy, ClassicalKernel, EnhancementMode,
    SrBackend, SrReport, SrRequest, TilePolicy, TileRect, DEFAULT_OVERLAP, DEFAULT_TILE,
};
pub use synthetic::{
    generate_scene_rgb8, match_accuracy, sample_id, train_test_disjoint, DatasetSplit,
    SyntheticSampleId, TEST_SEED_BASE, TRAIN_SEED_BASE,
};
pub use tracker::{BoundedTracker, FLAG_TRACK_OVERFLOW, MAX_TRACKS};
pub use types::{
    Detection, ImageView, PixelFormat, VisionError, VisualCapabilities, VisualModel,
    VisualOutputCounts, MAX_DETECTIONS, MAX_EMBED_DIM,
};
pub use weights::{
    load_litert_file, probe_onnx_asset, resolve_vision_asset, sface_infer_rgb8,
    validate_litert_bytes, yunet_infer_rgb8, AssetLicenceTag, LiteRtFileMeta, LiteRtLoadError,
    OnnxSessionError, ProductionVision, ResolvedAsset, VisionAssetError, VisionAssetId,
    VisionBackendKind, VisionWeightBundle, LITER_TFLITE_MAGIC, QVWT_MAGIC, QVWT_VERSION,
};

#[cfg(feature = "cpu-reference")]
pub use cpu_reference::CpuReferenceVision;
