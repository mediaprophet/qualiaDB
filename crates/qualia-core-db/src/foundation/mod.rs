//! `foundation` category (reorg).

pub mod frame_layout;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod crdt;
pub mod telemetry;
pub mod fuzz_testing;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod topology_draft;
