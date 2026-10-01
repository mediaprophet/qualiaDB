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
mod logic;
#[allow(unused_imports)]
pub use logic::*;
mod dataio;
#[allow(unused_imports)]
pub use dataio::*;
/// Civics welfare / VaR / simulation receipts — available without GPU scientific stack.
#[cfg(any(feature = "wasm-scientific", feature = "wasm-webcivics"))]
mod compute;
#[cfg(any(feature = "wasm-scientific", feature = "wasm-webcivics"))]
pub use compute::*;
// Computational-engine exports (linear algebra, CAS, statistics, numerics, exact,
// units, transforms, graph) — the solver/CAS math surfaced to the full-wasm bundle.
#[cfg(any(feature = "wasm-scientific", feature = "wasm-webcivics"))]
mod engine;
#[cfg(any(feature = "wasm-scientific", feature = "wasm-webcivics"))]
pub use engine::*;
mod meta;
#[allow(unused_imports)]
pub use meta::*;
/// Device OPFS vault + backup-folder policy (Civics / mobile installs).
mod device_storage;
#[allow(unused_imports)]
pub use device_storage::*;
#[cfg(feature = "wasm-scientific")]
mod geometry;
#[cfg(feature = "wasm-scientific")]
pub use geometry::*;
