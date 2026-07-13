//! Physics of artefacts (Phase 2, `RENDERER_IMPLEMENTATION_PLAN.md`).
//!
//! Deterministic, zero-alloc operators over rendered artefacts, built on the PGA motor oracle
//! (`render::pga`) so physics and the GPU projector share one geometry:
//!   * [`aabb`] — axis-aligned extent + rigid/scale transform of an artefact's box.
//!   * [`admission`] — **deterministic** admission of a proposed transform; refuses contraction
//!     below a material floor or movement outside world bounds (no probabilistic guess).
//!   * [`joint`] — kinematic joints as PGA motors animated over time `t` (composable into chains).
//!   * [`material`] — mass / material / momentum (the `P` in the Manifold-Coordinate).
//!
//! Rail-check: deterministic prevention; zero-heap operators (fixed arrays, no `Vec` in the path).

pub mod aabb;
pub mod admission;
pub mod joint;
pub mod material;

pub use aabb::Aabb;
pub use admission::{Admission, Refusal};
pub use joint::{chain_motor_at, Joint, JointKind};
pub use material::{Body, Material};
