//! `extensions` category (reorg).

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod extension_bus;
pub mod extension_manifest;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod resource_catalog;
