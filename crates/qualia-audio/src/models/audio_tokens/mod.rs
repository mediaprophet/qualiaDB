//! RVQ / FSQ discrete audio token codec (quantizer ABI). Re-exports only (AU-GEN).
//!
//! The quantizer *structure* is pure Rust and real now: [`rvq_quantize`]/[`rvq_dequantize`]
//! implement a residual vector quantizer over caller-owned codebooks, and
//! [`fsq_quantize`]/[`fsq_dequantize`] implement finite scalar quantization. The neural
//! encoder/decoder that produces the latents and codebooks is `NeedsWeights` (out of scope).

pub mod fsq;
pub mod rvq;

pub use fsq::{fsq_dequantize, fsq_quantize};
pub use rvq::{rvq_dequantize, rvq_quantize};
