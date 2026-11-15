//! Bounded storage jobs outside owner locks. Original transfer charges remain
//! until actual I/O completion. This is not a second ReservationLedger.
mod complete;
mod submit;
mod ticket;
pub use submit::StorageJobs;
pub use ticket::{StorageJobTicket, MAX_STORAGE_JOBS};

#[cfg(test)]
mod tests;
