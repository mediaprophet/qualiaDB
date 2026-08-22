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

// The browser profile keeps clinical formulae and record processing available.
// The rule-based SMILES screen is enabled with the scientific profile because
// it depends on the broader organic-chemistry domain tree.
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
mod cheminformatics;
mod clinical_formulas;
mod compliance;
mod diagnosis;
mod differential;
mod drug_discovery;
mod errors;
mod image_dsp;
mod imaging;
mod library;
mod privacy;
mod records;
mod types;

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "portal",
    feature = "wasm-logic",
    feature = "wasm-scientific"
))]
pub use cheminformatics::*;
pub use clinical_formulas::*;
pub use compliance::*;
pub use diagnosis::*;
pub use differential::*;
pub use drug_discovery::*;
pub use errors::*;
pub use image_dsp::*;
pub use imaging::*;
pub use library::*;
pub use privacy::*;
pub use records::*;
pub use types::*;

#[cfg(test)]
mod tests;
