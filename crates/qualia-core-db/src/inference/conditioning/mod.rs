//! Portable conditioning compiler (Prompt Precision P1C).
//!
//! Request-scoped, caller-buffered summaries. Cold construction may allocate
//! within explicit bounds; Tier-1 decode paths remain elsewhere.

mod authority;
mod budget;
mod capabilities;
mod codec;
mod compile;
mod identity;
pub mod inspector;
mod receipt;
pub mod registry;
mod render;
mod requirement;
mod select;
mod spec;
mod validate;
#[cfg(test)]
mod tests;

pub use authority::AuthorityView;
pub use budget::ConditioningBudget;
pub use capabilities::{BackendCapabilities, SupportLevel};
pub use codec::{decode_plan_cbor, encode_plan_cbor, CODEC_VERSION};
pub use compile::{compile_into, CompileBuffers, CompiledPlanSummary};
pub use identity::plan_identity;
pub use inspector::{inspect_spec, RedactedInspectionTrace, RedactedRequirement};
pub use receipt::{RequirementDisposition, RequirementOutcome};
pub use registry::{
    global_registry, ConditioningRegistry, ProfileLifecycleState, ProfileVersionEntry,
};
pub use render::{render_into, RenderTarget, RenderedRequestSummary};
pub use requirement::{RequirementClass, RequirementRef};
pub use select::{select_evidence_into, EvidencePart};
pub use spec::{ConditioningError, ConditioningSpec, OutputContractRef};
pub use validate::validate_spec;
