//! Bounded native plaintext sparse candidates, live-session scope only.
//! File barriers are observations, not directory durability or serving rights.
//! The public effect wrapper owns actual policy acceptance through final checks.
mod binding;
mod checkpoint;
mod error;
mod geometry;
mod owner;
mod read;
mod resume;
mod write;
pub use binding::{SelectedStoreBinding, StorePartition};
pub use error::StoreError;
pub use geometry::{
    StoreLayout, CHECKPOINT_BANK_BYTES, CHECKPOINT_PAGE_BYTES, PROOF_SLOT_BYTES,
    READ_SCRATCH_BYTES, WRITE_SCRATCH_BYTES,
};
pub use owner::{LeafBarrierObservation, NativeSparseStore, StoreState};
pub use resume::ResumeHintObservation;
#[cfg(test)]
mod tests;

mod recovery;
