//! Backend-neutral ownership boundary for native paged-attention kernel source.

mod segmented;
mod tiled_online;

pub use segmented::{
    segments_for_position, MAX_ATTENTION_SEGMENTS, PAGED_GQA_SEGMENTED_MERGE_ENTRY,
    PAGED_GQA_SEGMENTED_PARTIAL_ENTRY, PAGED_GQA_SEGMENTED_SRC,
};
pub use tiled_online::{PAGED_GQA_TILED_ENTRY, PAGED_GQA_TILED_SRC};
