//! Explicit admitted scalar CPU and standalone developer entrypoints.
//! Consuming and independent qualification is pending; existing runtime routes
//! are not implicitly redirected to this additive profile.
#![forbid(unsafe_code)]
mod budget;
mod buffers;
mod capabilities;
mod commit;
mod developer;
mod executor;
mod facade;
mod initialize;
mod model;
mod numerical;
mod observation;
mod preparation;
mod request;
mod session;
mod state;
mod tokenizer;

pub(super) use capabilities::check_projection_program;
pub(super) use commit::DenseAcceptanceCommit;
pub(super) use session::CpuProjectionBinding;

#[cfg(all(test, windows))]
mod runtime_edges;
#[cfg(all(test, windows))]
mod runtime_fixture;
#[cfg(all(test, windows))]
mod runtime_invalidations;
#[cfg(all(test, windows))]
mod runtime_tests;
#[cfg(all(test, windows))]
mod test_actor_fixture;
#[cfg(test)]
mod test_hooks;
#[cfg(test)]
mod tests;

pub use budget::{CpuBudgetReceipt, CpuLimits, CpuTables, PromptWorkspace};
pub use buffers::JobBuffers;
pub use capabilities::RequestedCapabilities;
pub use developer::{
    prepare_developer_cpu, DeveloperCpuReview, DeveloperCpuSession, DeveloperToken,
    PreparedDeveloperCpu, StandaloneDeveloper,
};
pub use executor::{ExecutorError, ExpectedCpuArtifact};
pub use facade::{prepare_governed_cpu, CpuEvidenceStorage, CpuRequestReview, PreparedGovernedCpu};
pub use model::{ImmutableModel, ModelPreparationError};
pub use observation::TOKEN_EVENT;
pub use preparation::CpuPreparationError;
pub use request::RequestIntent;
pub use session::{AcceptedCpuToken, CpuStepError, GovernedCpuSession};
pub use tokenizer::{PromptInput, TokenizerAdmissionError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CpuAdmissionError {
    UnsupportedCapability,
    UnsupportedObservation,
    InvalidIdentity,
    InvalidRequest,
    Capacity,
    Budget,
    Overflow,
    Terminal,
}

macro_rules! error_display {
    ($($ty:ty),+ $(,)?) => {$(
        impl std::fmt::Display for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{self:?}") }
        }
        impl std::error::Error for $ty {}
    )+};
}
error_display!(
    CpuAdmissionError,
    executor::ExecutorError,
    model::ModelPreparationError,
    tokenizer::TokenizerAdmissionError,
    preparation::CpuPreparationError,
    numerical::StepError,
    session::CpuStepError
);
