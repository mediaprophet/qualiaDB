//! Unified Analytical SDF + B-Rep 3D Physics Engine (`physics3d`).
//!
//! Provides `QualiaDB`'s Tier-1 (`HotZeroHeap`) 3D rigid-body, SDF, joint, fracture, and
//! continuum-coupling physics engine:
//! - [`sdf_collider`]: Exact analytical SDF evaluation (`Sphere`, `RoundedBox`, `Capsule`, `Torus`,
//!   `SmoothUnionPair`) with tetrahedron gradient normals $\nabla\text{sdf}(\mathbf{x})$.
//! - [`broadphase_bvh`] & [`query`]: Dynamic fat-AABB BVH (`BroadphaseBvh`), `cast_ray`
//!   (sphere-tracing + Möller–Trumbore), `cast_sphere`, `cast_capsule`, and `overlap_aabb`.
//! - [`gjk_epa`] & [`manifold`]: PGA [`Motor`] support mappings, zero-heap GJK/EPA narrowphase,
//!   and persistent 4-point [`ContactManifold3D`] with warm-starting.
//! - [`fracture_csg`]: Principled `.10d` `PROC1` tensile stress-cleavage dynamic fracture
//!   (`evaluate_and_fracture_body`) conserving linear and angular momentum.
//! - [`xpbd_solver`], [`joints`], & [`ragdoll`]: 64 Hz deterministic XPBD rigid-body solver,
//!   6-DOF PGA [`Motor`] joints (`BallMotorJoint`, `RevoluteMotorJoint`, `FixedMotorJoint`,
//!   `SpringSuspensionJoint`), and `ArticulatedRagdoll`.
//! - [`continuum_bridge`]: Two-way force coupling with `geomechanics.rs`, `hydro.rs`, and
//!   `metamer_fspm.rs`.

pub mod broadphase_bvh;
pub mod continuum_bridge;
pub mod fracture_csg;
pub mod gjk_epa;
pub mod joints;
pub mod manifold;
pub mod query;
pub mod ragdoll;
pub mod sdf_collider;
pub mod xpbd_solver;

pub use broadphase_bvh::{Aabb3D, BroadphaseBvh, BvhLeafEntry};
pub use continuum_bridge::{
    couple_rigid_body_with_botanical_stem, couple_rigid_body_with_hydro,
    couple_rigid_body_with_soil, BotanicalImpactOutcome, HydroCouplingOutcome,
    SoilCouplingOutcome,
};
pub use fracture_csg::{evaluate_and_fracture_body, FractureOutcome3D};
pub use gjk_epa::{
    compute_narrowphase_contact, gjk_intersect, ColliderShape3D, NarrowphaseContact, SupportVertex,
    MAX_CONVEX_HULL_POINTS,
};
pub use joints::{
    solve_motor_joint_xpbd, BallMotorJoint, FixedMotorJoint, MotorJoint3D, RevoluteMotorJoint,
    SpringSuspensionJoint,
};
pub use manifold::{
    ContactManifold3D, ContactPoint3D, MAX_MANIFOLD_POINTS, PERSISTENT_CONTACT_THRESHOLD_SQ,
};
pub use query::{
    cast_capsule, cast_ray, cast_sphere, overlap_aabb, ray_sdf_local, ray_sphere_local,
    ray_triangle_moller_trumbore, raycast_collider, RayHit3D, ShapeCastHit3D,
};
pub use ragdoll::{ArticulatedRagdoll, RagdollLimbBinding, RagdollMode};
pub use sdf_collider::SdfPrimitive;
pub use xpbd_solver::{step_xpbd_world, RigidBody3D, RigidBodyType3D};
