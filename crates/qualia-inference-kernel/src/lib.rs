#![forbid(unsafe_code)]
//! Lean inference-kernel contracts shared by QualiaDB runtime backends.
//!
//! This crate deliberately has no dependencies on the database, renderer, networking,
//! cryptography, or application crates. Kernel policies and scalar correctness oracles can
//! therefore be tested without linking the complete QualiaDB product graph.

pub mod paged_attention;
