//! Swarm S — spatial / mesh IR, validation, image-to-3D handoff.
//!
//! Geometry here is vision-side reconstruction + validation. Canonical GLB→Q42
//! dense compile remains in `qualia-core-db::render`; this module produces
//! bounded meshes that can be handed off after validation.

pub mod geometry_ir;
pub mod image_to_3d;
pub mod validate;
pub mod export_obj;

pub use geometry_ir::{MeshIR, MAX_INDICES, MAX_VERTICES};
pub use image_to_3d::{image_to_heightfield_mesh, ImageTo3dReceipt};
pub use validate::{validate_mesh_ir, MeshValidationReport, MeshValidationStatus};
pub use export_obj::{mesh_ir_to_obj, mesh_ir_triangles};
