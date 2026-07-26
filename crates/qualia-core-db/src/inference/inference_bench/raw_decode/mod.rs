//! Fixed-step raw decoder benchmark.
//!
//! This runner calls `QTensorEngine` directly. Product-level retrieval, graph drafting,
//! governance, speculative acceptance, EOS, sampling and text rendering are deliberately absent.

mod config;
mod model;
mod runner;
mod stats;

pub use config::{RawDecodeConfig, RawDecodeResult};
pub use runner::run_raw_decode_blocking;
pub use stats::{median_f64, percentile_nearest_rank};
