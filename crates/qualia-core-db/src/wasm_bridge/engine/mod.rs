//! WASM-bindgen exports for the QualiaDB **computational engine** — the
//! solver / CAS / statistics math surfaced to the browser.
//!
//! Design notes (read once):
//! * We wrap the **solver layer** (`crate::solvers::*`) and the pure free-function
//!   facades in `crate::specialized_libs::*`, NOT the `*Library` struct wrappers —
//!   those measure `Instant::now()` for telemetry, which *panics* on
//!   `wasm32-unknown-unknown`. The solver math is timing/IO/thread-free, so it runs
//!   identically in the browser and natively (the demos call exactly the code the
//!   native MCP tools and the 600+ solver unit tests exercise).
//! * Every export takes one JS object and returns one JS object via
//!   `serde_wasm_bindgen`; errors come back as `Err(JsValue::from_str(..))`.
//! * The whole module is gated on `feature = "wasm-scientific"` (so it ships in the
//!   full-wasm *playground* bundle), and every `#[wasm_bindgen]` fn is additionally
//!   gated on `target_arch = "wasm32"` — on a native `wasm-scientific` build the
//!   module is simply empty (the engine is reached through MCP there).

#[cfg(target_arch = "wasm32")]
mod cas;
#[cfg(target_arch = "wasm32")]
mod crypto;
mod exact;
#[cfg(target_arch = "wasm32")]
mod graph;
#[cfg(target_arch = "wasm32")]
mod linalg;
#[cfg(target_arch = "wasm32")]
mod numerics;
#[cfg(target_arch = "wasm32")]
mod stats;
#[cfg(target_arch = "wasm32")]
mod stats_regression;
#[cfg(target_arch = "wasm32")]
mod transforms;
#[cfg(target_arch = "wasm32")]
mod units;

#[cfg(target_arch = "wasm32")]
pub use cas::*;
#[cfg(target_arch = "wasm32")]
pub use crypto::*;
#[allow(unused_imports)]
pub use exact::*;
#[cfg(target_arch = "wasm32")]
pub use graph::*;
#[cfg(target_arch = "wasm32")]
pub use linalg::*;
#[cfg(target_arch = "wasm32")]
pub use numerics::*;
#[cfg(target_arch = "wasm32")]
pub use stats::*;
#[cfg(target_arch = "wasm32")]
pub use stats_regression::*;
#[cfg(target_arch = "wasm32")]
pub use transforms::*;
#[cfg(target_arch = "wasm32")]
pub use units::*;

/// Map any `Display` error into a JS string error value.
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(crate) fn jserr<E: core::fmt::Display>(e: E) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(&e.to_string())
}
