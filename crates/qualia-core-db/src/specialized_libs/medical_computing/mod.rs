//! Medical Computing Library - Healthcare Data Processing and Medical Analytics
//!
//! This module provides high-performance medical computing operations leveraging Phase 2 enhancements:
//! - Fiduciary Cryptography (ML-DSA) for secure medical data protection
//! - Zero-Knowledge Semantic Proofs for privacy-preserving medical research
//! - Hardware-Sympathetic Storage (ZNS) for zero-copy medical data
//! - Statistical Computing Library for advanced medical analytics

// Library-ized per CLAUDE.md §11: this module was a ~6180-line monolith,
// split into single-concern submodules. mod.rs retains the module doc and
// re-exports the full public surface unchanged.

mod types;
mod errors;
mod library;
mod clinical_formulas;
mod records;
mod privacy;
mod diagnosis;
mod differential;
mod imaging;
mod image_dsp;
mod drug_discovery;
mod cheminformatics;
mod compliance;

pub use types::*;
pub use errors::*;
pub use library::*;
pub use clinical_formulas::*;
pub use records::*;
pub use privacy::*;
pub use diagnosis::*;
pub use differential::*;
pub use imaging::*;
pub use image_dsp::*;
pub use drug_discovery::*;
pub use cheminformatics::*;
pub use compliance::*;

#[cfg(test)]
mod tests;
