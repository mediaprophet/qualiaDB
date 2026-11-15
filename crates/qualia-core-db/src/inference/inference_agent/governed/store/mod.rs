//! Explicit persistence version dispatch. Loading never upgrades a signed record.
mod legacy;
mod versioned;
pub(crate) use versioned::Record;
