//! A0 — native LLM benchmark harness (STELLAR §A; decisions D17 + D22).
//!
//! This is the **shared** measurement surface for the performance push. It drives
//! the *real* inference path ([`LocalLlmAgent::infer_local_model_streaming`]) and
//! reads per-phase timing recorded *inside that same path* — so the existing
//! F16/Q8 path and the future ternary/top-k paths are measured by **one** harness
//! rather than a forked benchmark loop (D22 "shared-improvement" rule). A speedup
//! that shows up here is a real, attributable, end-to-end number, not a kernel
//! microbenchmark.
//!
//! What it reports (per model / weight policy):
//!   * **cold TTFT** — model not resident: wall-clock from call to first token
//!     (bundles mmap load + pipeline create + prefill + first decode);
//!   * **warm TTFT** — model resident (mmap adopted, pipelines still rebuilt per
//!     call in the current architecture — that cost is intentionally *included*,
//!     it is what A7 will attack);
//!   * **prefill / decode tok/s** from the internal phase split;
//!   * the **load / prefill / decode** wall-clock breakdown.
//!
//! Honest scope of *this* increment (A0.1): timings are **host wall-clock**.
//! GPU timestamp-query kernel isolation (D17) — requesting `TIMESTAMP_QUERY` on
//! the shared device and wrapping passes with `timestamp_writes` — is the A0.2
//! follow-on; [`BenchResult::gpu_timestamp_supported`] is `false` until then.
//!
//! Native-only: the WASM decode path is a different beast and is benchmarked in
//! the browser harness.
//!
//! Library-ized (CLAUDE.md §11) from a single `inference_bench.rs` into cohesive
//! submodules — pure code motion, no behaviour change. The public surface is
//! re-exported unchanged, so `crate::inference::inference_bench::<Item>` resolves
//! exactly as before.
#![cfg(not(target_arch = "wasm32"))]

mod counters;
mod metrics;
mod probes;
pub mod raw_decode;
mod reporting;
mod runner;
mod toggles;
mod types;

#[cfg(test)]
mod tests;

pub use counters::*;
pub use metrics::*;
pub use probes::*;
pub use raw_decode::*;
pub use reporting::*;
pub use runner::*;
pub use toggles::*;
pub use types::*;
