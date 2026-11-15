//! Deterministic mixed-HMC transfer plan. Bind identities, not filenames.
//! Multiplexing is a scheduler, not a container rewrite. This module does not
//! admit codecs, verify leaves, mark storage, or grant disclosure.
mod inventory;
mod readiness;
mod schedule;
pub use inventory::{
    DependencyEdge, DependencyReason, EntryFamily, EntryRef, Inventory, Obligation, PlanEntry,
    PlanEntrySpec, RationalInterval, RepresentationClass, ResourceEstimate, TimebaseId,
    MAX_PLAN_EDGES, MAX_PLAN_ENTRIES,
};
pub use readiness::{evaluate, Readiness};
pub use schedule::{
    PrincipalSelection, RecordUnit, Region, Schedule, ScheduledUnit, SelectionIntent,
    SelectionRequest, WorkClass, MAX_PRINCIPALS, MAX_SCHEDULE_ITEMS,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    Capacity,
    Limit,
    DuplicateId,
    Identity,
    Cycle,
    Graph,
    Field,
    Unsupported,
    Refused,
}
impl core::fmt::Display for PlanError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "plan {:?}", self)
    }
}
impl core::error::Error for PlanError {}
#[cfg(test)]
mod tests;
