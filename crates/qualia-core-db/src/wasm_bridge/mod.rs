//! WASM-bindgen API surface — exposes Qualia engine functions to JavaScript.
//! Split into domain submodules; `pub use *` keeps every `wasm_bridge::fn` path stable.

mod bio;
pub use bio::*;
mod chemistry;
pub use chemistry::*;
mod medical;
pub use medical::*;
mod semantic;
pub use semantic::*;
mod dataio;
pub use dataio::*;
mod compute;
pub use compute::*;
mod meta;
pub use meta::*;
