//! Vision excellence capability registry (machine-readable ledger).

pub mod status;
pub mod entry;
pub mod registry;

pub use status::CapabilityStatus;
pub use entry::CapabilityEntry;
pub use registry::{all_capabilities, by_id, count_by_status};
