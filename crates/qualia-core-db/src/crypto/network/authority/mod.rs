//! Explicit full paired-key pin authority for QBDL publication proofs.
//!
//! LocalBootstrap is a host-admin capability, never a wire credential.
//! This candidate profile does not validate controller chains, mint semantic
//! ExecutionPermits, confirm sessions or release/export secret keys.

mod operations;
mod pins;
mod types;

pub use operations::{KeyCompletion, KeyOperation, KeyRequest};
pub use pins::PinAuthority;
pub use types::{AuthorityReceipt, AuthoritySource, LocalBootstrap, PinConfig, PinHandle};

pub const MAX_PINS: usize = 8;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod operation_tests;
#[cfg(test)]
mod test_support;
