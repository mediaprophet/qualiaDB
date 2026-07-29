//! Vision-side mesh IR, validation, export, quality (C1/C2 pure kernels).
//!
//! Host 10d handoff / image-to-3d product adapters remain in `qualia-vision`.

pub mod export_3mf;
pub mod export_glb;
pub mod export_obj;
pub mod export_stl;
pub mod geometry_ir;
pub mod mesh_ir_quality;
pub mod mesh_ir_to_export;
pub mod print_readiness;
pub mod sigma_map;
pub mod twin_bridge;
pub mod validate;

pub use export_3mf::mesh_ir_to_3mf;
pub use export_glb::mesh_ir_to_glb;
pub use export_obj::{mesh_ir_to_obj, mesh_ir_triangles};
pub use export_stl::mesh_ir_to_stl_binary;
pub use geometry_ir::{MeshIR, MAX_INDICES, MAX_VERTICES};
pub use mesh_ir_quality::{cleanup_mesh_ir, MeshCleanupOptions, MeshQualityReport};
pub use mesh_ir_to_export::{
    detection_center_to_node_hint, mesh_ir_to_export, mesh_ir_to_export_validated, NodeHint,
    RenderMeshExport,
};
pub use print_readiness::{print_readiness, PrintReadiness};
pub use sigma_map::{
    class_hash_to_sigma_base, class_id_to_sigma_base, class_score_to_sigma, detection_to_sigma,
};
pub use twin_bridge::{
    assess_twin_eligibility, closed_form_bar_stretch, promote_elasticity_preview,
    refuse_fea_unless_eligible, run_elasticity_preview_if_eligible, AnalysisDomain,
    BarStretchInput, BarStretchResult, TwinEligibility,
};
pub use validate::{validate_mesh_ir, MeshValidationReport, MeshValidationStatus};
