//! Hybrid operator converter module (W5: EOS-050…EOS-052).
//!
//! Provides bounded activation statistics, hybrid candidate generation ($W \approx Q + AB + S$),
//! and held-out validation with explicit conversion receipts:
//! - `activation_stats`: $H = E[x x^T]$ covariance sketching under Sentinel 42 MiB limits.
//! - `hybrid_fit`: Activation-weighted rank-$r$ + sparse outlier residual fitting.
//! - `eval`: Held-out validation and conversion quality receipts.

pub mod activation_stats;
pub mod eval;
pub mod hybrid_fit;

pub use activation_stats::{ActivationStats, ConverterError, SketchKind};
pub use eval::{evaluate_conversion, ConversionReceipt};
pub use hybrid_fit::{
    fit_hybrid_decomposition, DenseEscapeTile, HybridDecomposition, HybridFitConfig, SparseEntry,
};
