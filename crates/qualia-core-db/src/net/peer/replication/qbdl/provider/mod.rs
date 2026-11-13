//! Cold bounded proof lookup over an already-built caller-owned outboard.
//! This module provides no source availability, policy, or disclosure authority.
mod outboard;
pub use outboard::{OutboardCatalog, OutboardError, OutboardLimits};

#[cfg(test)]
mod outboard_tests;
