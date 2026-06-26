//! `core` category — consolidated from crate-root modules (reorg).

pub mod frame_layout;
pub mod crdt;
pub mod telemetry;
pub mod fuzz_testing;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-llm"))]
pub mod topology_draft;
