//! Bounded cold authoring of thin guards with circular wheel-clearance edges.
//! Native closed lofts generate topology; public outputs remain caller-owned.
mod build;
pub use build::{wheel_clearance_panel_into, WheelPanel};
/// Cold authoring classification; compilation/test maturity remains unclaimed
/// until the deferred validation gates have run.
pub const ALLOCATION_CLASS: super::capability_manifests::AllocationClass =
    super::capability_manifests::AllocationClass::ColdBounded;
#[cfg(test)]
mod tests;
