//! Mixed-family QBDL-profile HMC. Many entries per family; inner bytes admit
//! through existing format views. Integrator routes `pub mod mixed`.
mod bundle;
pub use bundle::{
    mixed_plan, publish_mixed, MixedEntry, MixedError, MixedHmc, MixedPlan, MIXED_ID_EMF_ALIGN,
    MIXED_ID_EMF_OPAQUE, MIXED_ID_M64, MIXED_ID_OPAQUE, MIXED_ID_P64, MIXED_ID_Q42,
    MIXED_ID_QAC_OPAQUE, MIXED_ID_QAC_PCM, MIXED_ID_TEND_A, MIXED_ID_TEND_B, MIXED_OPAQUE_BYTES,
};
mod transfer;
pub use transfer::{
    bind_mixed_clients, bind_mixed_entries, bind_mixed_entry, compile_from_mixed, compile_mixed_mux,
    mixed_intents, mixed_outboard, mixed_present_ids, mixed_selection, select_mixed, transfer_mixed,
};
#[cfg(all(test, not(target_arch = "wasm32")))]
pub use bundle::default_mixed_entries;
#[cfg(test)]
mod tests;
