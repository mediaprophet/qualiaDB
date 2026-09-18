//! Bounded request scheduling for paged native inference.
//!
//! Construction is cold and sizes every request table. Admission, prefix attachment, runnable
//! collection, page COW, cancellation, and completion are zero-growth hot operations.

pub mod admission;
mod batch;
pub mod policy;
pub mod queue;
mod request_table;

pub use admission::{AdmissionError, AdmittedCurrencies};
pub use batch::{
    RaggedBackendError, RaggedBatchItem, RaggedBatchOutput, RaggedBatchReceipt, RaggedDecodeBackend,
};
pub use policy::{PrefillProgress, SchedulingPolicy};
pub use queue::{BoundedIntakeQueue, IntakeError, IntakeRequest, IntakeState};
pub use request_table::{
    Admission, DecodeRoundError, RequestScheduler, RequestState, RequestView, SchedulerError,
};

#[cfg(test)]
mod tests;
