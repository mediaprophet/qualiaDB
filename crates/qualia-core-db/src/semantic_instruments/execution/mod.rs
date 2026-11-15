//! SI-06: generic runner and execution receipts.
//! Dispatch is by entry-point name, never a new Host ID.

pub mod receipt;
pub mod runner;

pub use receipt::ExecutionReceipt;
pub use runner::{
    evaluate_core_semantic_instrument, execute_reference_instrument, preflight, run_entry,
    RunOutcome, RunRequest,
};
