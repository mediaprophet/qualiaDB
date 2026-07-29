//! Bounded request scheduling for paged native inference.
//!
//! Construction is cold and sizes every request table. Admission, prefix attachment, runnable
//! collection, page COW, cancellation, and completion are zero-growth hot operations.

mod batch;
mod request_table;

pub use batch::{
    RaggedBackendError, RaggedBatchItem, RaggedBatchOutput, RaggedBatchReceipt, RaggedDecodeBackend,
};
pub use request_table::{
    Admission, DecodeRoundError, RequestScheduler, RequestState, RequestView, SchedulerError,
};

#[cfg(test)]
mod tests;
