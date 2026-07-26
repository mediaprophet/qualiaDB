//! Native CUDA paged decode attention kernels.

mod segmented;
mod tiled_online;

pub(crate) use segmented::{
    segments_for_position, MAX_ATTENTION_SEGMENTS, PAGED_GQA_SEGMENTED_MERGE_ENTRY,
    PAGED_GQA_SEGMENTED_PARTIAL_ENTRY, PAGED_GQA_SEGMENTED_SRC,
};
pub(crate) use tiled_online::{PAGED_GQA_TILED_ENTRY, PAGED_GQA_TILED_SRC};

#[cfg(test)]
mod tests;
