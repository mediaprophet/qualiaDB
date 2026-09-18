//! Backend-neutral prepared decode contract.

pub mod capability;
mod decode_plan;
pub mod generation;
mod lifecycle;

pub use capability::{
    BackendCapabilities, KvEncoding, SupportedArchitecture,
};
pub use decode_plan::{
    DecodeStepError, DecodeStepInput, DecodeStepOutput, PreparedBackend, PreparedDecodePlan,
};
pub use generation::{ModelResidencyIdentity, ResidencyGenerationTracker};
pub use lifecycle::PreparedPlanState;
