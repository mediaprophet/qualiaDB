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

mod cas;
mod exact;
mod graph;
mod linalg;
mod numerics;
mod stats;
mod transforms;
mod units;

pub use cas::*;
pub use exact::*;
pub use graph::*;
pub use linalg::*;
pub use numerics::*;
pub use stats::*;
pub use transforms::*;
pub use units::*;

/// Map any `Display` error into a JS string error value.
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(crate) fn jserr<E: core::fmt::Display>(e: E) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(&e.to_string())
}
