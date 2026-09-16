//! SI-06: generic runner and execution receipts.
//! Dispatch is by entry-point name, never a new Host ID.

pub mod receipt;
pub mod runner;

pub use receipt::ExecutionReceipt;
pub use runner::{preflight, run_entry, RunOutcome, RunRequest};
