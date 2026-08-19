//! `governance` category (reorg).

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod coordination;
pub mod illocution;
pub mod instrument_trace;
pub mod modal_kind;
pub mod provenance;
#[cfg(not(target_arch = "wasm32"))]
pub mod web_civics;
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub mod webizen;
#[cfg(any(
    not(target_arch = "wasm32"),
    any(feature = "wasm-scientific", feature = "wasm-logic")
))]
pub mod webizen_bytecode;
#[cfg(not(target_arch = "wasm32"))]
pub mod webizen_sync;
pub mod webizen_validator;
