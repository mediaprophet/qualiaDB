//! P10 — Authoritative physical-state interface and evidence.
//!
//! Constitutive, load, time, units and mesh-authority records stay distinct
//! from visual LOD identities. The scoped kernel is `pbd_softbody` (Verlet +
//! Gauss–Seidel hard distance/volume projection). Independent tetrahedra reuse
//! that projection on unshared vertices and are never labelled a race-free
//! connected global solver or a universal collision guarantee.
//!
//! FEMC remains surface-to-tetra correspondence only. Contact, friction and
//! global coupling claims are explicit `unsupported` outcomes.
//! Parent `computational_geometry/mod.rs` must declare `pub mod physical_authority`.

mod state;
mod transfer;

pub use state::*;
pub use transfer::*;

#[cfg(test)]
mod tests;
