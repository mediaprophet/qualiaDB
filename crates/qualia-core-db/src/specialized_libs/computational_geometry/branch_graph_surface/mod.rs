//! Bounded closed branch-graph study using existing field/extraction owners.
//! No graft boundary, botanical identity, smooth collar or acceptance claim.
//! Hard minimum union only; nonincident expanded boxes must be separated.
mod pipeline;
pub use pipeline::{branch_graph_surface_into, branch_graph_surface_bounds, BranchGraphInput,
    BranchGraphSpec, BranchGraphLimits, BranchGraphBounds, BranchGraphBuffers,
    BranchGraphReceipt, BranchGraphError};
#[cfg(test)]
mod tests;
