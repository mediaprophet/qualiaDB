//! WASM-only MC8 GPU engine: resident weight arena, fused-encoder prefill/decode,
//! async readback. Split into concern submodules (structural; no behaviour change).
//! The cfg(wasm32) `mod mc8_wasm;` in gguf_bridge/mod.rs gates the whole module.

mod encode;
mod params;
mod readback;
mod residency;
