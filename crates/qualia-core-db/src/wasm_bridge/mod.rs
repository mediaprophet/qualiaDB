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
pub use semantic::*;
mod dataio;
pub use dataio::*;
#[cfg(feature = "wasm-scientific")]
mod compute;
#[cfg(feature = "wasm-scientific")]
pub use compute::*;
mod meta;
pub use meta::*;
