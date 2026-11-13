//! Caller-buffered P64 planning, prepared-payload writing and sized streaming.
//!
//! Borrowed writers copy prepared payloads; sized streams invoke caller conversion
//! callbacks and support the legacy FFN layout with raw schedule bytes. Sorting,
//! numerical transformations, semantic manifold construction and atomic filesystem
//! publication remain caller responsibilities. Planning refusals preserve scratch;
//! streaming failures may modify scratch and leave an incomplete external sink.

mod descriptors;
mod layout;
mod metadata;
mod plan;
mod plan_error;
mod source;
mod stream;
mod stream_error;
pub(crate) mod write;

pub use descriptors::{BuildBudget, BuildError, BuildInput, BuildProfile, TensorDescriptor};
pub use layout::{LayoutInput, LayoutProfile, TensorLayout};
pub use plan::{
    plan_into, plan_into_detailed, plan_layout_into, plan_layout_into_detailed, BuildPlan,
    LayoutPlan,
};
pub use plan_error::{PlanError, PlanErrorContext, PlanField, PlanOperation};
pub use source::{
    ByteSink, PayloadSink, PayloadSource, PayloadWorkspace, PayloadWriteRefusal, StreamScratch,
    WorkspaceRequirements,
};
pub use stream::stream_into;
pub use stream_error::{StreamFailure, StreamFailureKind, StreamPhase, StreamReport};
pub use write::write_into;

#[cfg(test)]
mod plan_error_tests;
#[cfg(test)]
mod stream_tests;
#[cfg(test)]
mod tests;
