//! Vision excellence capability registry (machine-readable ledger).

pub mod entry;
pub mod registry;
pub mod status;

pub use entry::CapabilityEntry;
pub use registry::{all_capabilities, by_id, count_by_status};
pub use status::CapabilityStatus;
