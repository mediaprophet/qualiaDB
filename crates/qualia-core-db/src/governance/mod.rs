//! `governance` category (reorg).

#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-webcivics",
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod cell_graph;
#[cfg(not(all(
    target_arch = "wasm32",
    feature = "wasm-ontology",
    not(any(
        feature = "wasm-webcivics",
        feature = "wasm-logic",
        feature = "wasm-scientific",
        feature = "wasm-full"
    ))
)))]
pub mod consent_contract;
#[cfg(any(
    not(target_arch = "wasm32"),
    feature = "wasm-webcivics",
    feature = "wasm-logic",
    feature = "wasm-scientific",
    feature = "wasm-full"
))]
pub mod coord_seams;
pub mod cohort_access_gate;
pub mod coordination;
pub mod illocution;
pub mod instrument_trace;
pub mod law_packages;
pub mod modal_kind;
pub mod provenance;
pub mod tk_bc_labels;
#[cfg(not(target_arch = "wasm32"))]
pub mod web_civics;
pub mod webizen;
pub mod webizen_bytecode;
#[cfg(not(target_arch = "wasm32"))]
pub mod webizen_sync;
pub mod webizen_validator;
pub mod worker_pool;
