//! Compatibility re-export of the lean segmented paged-attention kernel source.

pub(crate) use qualia_inference_kernel::paged_attention::kernels::{
    segments_for_position, MAX_ATTENTION_SEGMENTS, PAGED_GQA_SEGMENTED_MERGE_ENTRY,
    PAGED_GQA_SEGMENTED_PARTIAL_ENTRY, PAGED_GQA_SEGMENTED_SRC,
};
