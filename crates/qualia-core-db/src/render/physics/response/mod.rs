//! Bounded driven spring response for articulated scene bodies. Integration
//! delegates to the canonical Qualia calculus solver, not a renderer-local RK4.
mod spring;
mod thermal;
pub use spring::{step_spring, DrivenSpring, SpringState};
pub use thermal::{step_thermal, ThermalTargets};
pub const SOURCE_HASH: u64 = crate::q_hash(include_str!("spring.rs"))
    ^ crate::q_hash(include_str!("thermal.rs")).rotate_left(7)
    ^ crate::q_hash(include_str!("../../../solvers/calculus/mod.rs")).rotate_left(17);
pub const ALLOCATION_CLASS:crate::specialized_libs::computational_geometry::capability_manifests::AllocationClass=
    crate::specialized_libs::computational_geometry::capability_manifests::AllocationClass::HotZeroHeap;
#[cfg(test)]
mod tests;
