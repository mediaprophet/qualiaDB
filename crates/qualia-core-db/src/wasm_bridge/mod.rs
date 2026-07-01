//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//! Split into domain submodules; `pub use *` keeps every `wasm_bridge::fn` path stable.

#[cfg(feature = "wasm-scientific")]
mod bio;
#[cfg(feature = "wasm-scientific")]
pub use bio::*;
#[cfg(feature = "wasm-scientific")]
mod chemistry;
#[cfg(feature = "wasm-scientific")]
pub use chemistry::*;
#[cfg(feature = "wasm-scientific")]
mod medical;
#[cfg(feature = "wasm-scientific")]
pub use medical::*;
mod semantic;
#[allow(unused_imports)]
pub use semantic::*;
mod dataio;
#[allow(unused_imports)]
pub use dataio::*;
#[cfg(feature = "wasm-scientific")]
mod compute;
#[cfg(feature = "wasm-scientific")]
pub use compute::*;
// Computational-engine exports (linear algebra, CAS, statistics, numerics, exact,
// units, transforms, graph) — the solver/CAS math surfaced to the full-wasm bundle.
#[cfg(feature = "wasm-scientific")]
mod engine;
#[cfg(feature = "wasm-scientific")]
pub use engine::*;
mod meta;
#[allow(unused_imports)]
pub use meta::*;
