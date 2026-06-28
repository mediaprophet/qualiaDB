//! `foundation` category (reorg).

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod crdt;
pub mod frame_layout;
pub mod fuzz_testing;
pub mod telemetry;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod topology_draft;
