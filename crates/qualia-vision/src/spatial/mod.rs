//! Swarm S — spatial / mesh IR, validation, image-to-3D handoff.
//!
//! Geometry here is vision-side reconstruction + validation. Canonical GLB→Q42
//! dense compile remains in `qualia-core-db::render`; this module produces
//! bounded meshes that can be handed off after validation.

pub mod geometry_ir;
pub mod image_to_3d;
pub mod validate;
pub mod export_obj;
pub mod export_stl;
pub mod print_readiness;
pub mod twin_bridge;

pub use geometry_ir::{MeshIR, MAX_INDICES, MAX_VERTICES};
pub use image_to_3d::{image_to_heightfield_mesh, ImageTo3dReceipt};
pub use validate::{validate_mesh_ir, MeshValidationReport, MeshValidationStatus};
pub use export_obj::{mesh_ir_to_obj, mesh_ir_triangles};
pub use export_stl::mesh_ir_to_stl_binary;
pub use print_readiness::{print_readiness, PrintReadiness};
pub use twin_bridge::{
    assess_twin_eligibility, closed_form_bar_stretch, promote_elasticity_preview,
    refuse_fea_unless_eligible, run_elasticity_preview_if_eligible, AnalysisDomain,
    BarStretchInput, BarStretchResult, TwinEligibility,
};
