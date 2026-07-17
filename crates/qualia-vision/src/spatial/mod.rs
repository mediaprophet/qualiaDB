//! Spatial product adapters over `specialized_libs::computer_vision::spatial`.
//!
//! Pure MeshIR / export / quality live in core-db. Image→3D and 10d handoff
//! (semantic digests, detection packing) stay here.

pub mod image_to_3d;
pub mod compile_10d_handoff;

pub use qualia_core_db::specialized_libs::computer_vision::spatial::{
    assess_twin_eligibility, class_hash_to_sigma_base, class_id_to_sigma_base, class_score_to_sigma,
    cleanup_mesh_ir, closed_form_bar_stretch, detection_center_to_node_hint, detection_to_sigma,
    mesh_ir_to_export, mesh_ir_to_export_validated, mesh_ir_to_obj, mesh_ir_to_stl_binary,
    mesh_ir_triangles, print_readiness, promote_elasticity_preview, refuse_fea_unless_eligible,
    run_elasticity_preview_if_eligible, validate_mesh_ir, AnalysisDomain, BarStretchInput,
    BarStretchResult, MeshCleanupOptions, MeshIR, MeshQualityReport, MeshValidationReport,
    MeshValidationStatus, NodeHint, PrintReadiness, RenderMeshExport, TwinEligibility, MAX_INDICES,
    MAX_VERTICES,
};
pub use image_to_3d::{image_to_heightfield_mesh, ImageTo3dReceipt};
pub use compile_10d_handoff::{
    detections_to_node_hints, pack_geometry_export_for_10d, GeometryFor10d,
};
