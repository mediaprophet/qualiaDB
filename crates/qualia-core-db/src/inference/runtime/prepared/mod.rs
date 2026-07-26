//! Backend-neutral prepared decode contract.

mod decode_plan;
mod lifecycle;

pub use decode_plan::{
    DecodeStepError, DecodeStepInput, DecodeStepOutput, PreparedBackend, PreparedDecodePlan,
};
pub use lifecycle::PreparedPlanState;
