//! Paged attention schedule selection for the mega pass.

use super::super::device::MultiWeightDevice;
use super::super::paged_attention::{
    PAGED_GQA_SEGMENTED_MERGE_ENTRY, PAGED_GQA_SEGMENTED_PARTIAL_ENTRY, PAGED_GQA_SEGMENTED_SRC,
    PAGED_GQA_TILED_ENTRY, PAGED_GQA_TILED_SRC,
};
use super::attention_stage::{AttentionSpec, AttentionViews};
use crate::wgsl_forge::execute::CudaPipeline;
use crate::wgsl_forge::Schedule;

pub(super) fn dispatch_sdpa(
    dev: &mut MultiWeightDevice,
    mut views: AttentionViews,
    spec: AttentionSpec,
    schedule: &Schedule,
) -> Option<()> {
    views.yq.binding = 0;
    views.kv.binding = 1;
    views.sdpa_params.binding = 3;
    views.sdpa_scale.binding = 4;
    views.layer_id.binding = 5;
    views.step.binding = 6;
    views.block_table.binding = 7;
    if spec.attention_segments == 1 {
        views.attention.binding = 2;
        let sdpa = compile(
            dev,
            PAGED_GQA_TILED_SRC,
            PAGED_GQA_TILED_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7],
        )?;
        if let Err(error) = sdpa.dispatch_async_sorted(
            &[
                views.yq,
                views.kv,
                views.attention,
                views.sdpa_params,
                views.sdpa_scale,
                views.layer_id,
                views.step,
                views.block_table,
            ],
            schedule,
            spec.n_head.saturating_mul(256),
        ) {
            log::warn!("mega_pass|sdpa|layer={}|{error:?}", spec.layer);
            return None;
        }
        return Some(());
    }

    views.attention_partials.binding = 2;
    let partial = compile(
        dev,
        PAGED_GQA_SEGMENTED_SRC,
        PAGED_GQA_SEGMENTED_PARTIAL_ENTRY,
        &[0, 1, 2, 3, 4, 5, 6, 7],
    )?;
    if let Err(error) = partial.dispatch_async_sorted(
        &[
            views.yq,
            views.kv,
            views.attention_partials,
            views.sdpa_params,
            views.sdpa_scale,
            views.layer_id,
            views.step,
            views.block_table,
        ],
        schedule,
        spec.n_head
            .saturating_mul(spec.attention_segments)
            .saturating_mul(256),
    ) {
        log::warn!("mega_pass|sdpa_partial|layer={}|{error:?}", spec.layer);
        return None;
    }

    views.attention_partials.binding = 0;
    views.attention.binding = 1;
    views.sdpa_params.binding = 2;
    let merge = compile(
        dev,
        PAGED_GQA_SEGMENTED_SRC,
        PAGED_GQA_SEGMENTED_MERGE_ENTRY,
        &[0, 1, 2],
    )?;
    if let Err(error) = merge.dispatch_async_sorted(
        &[views.attention_partials, views.attention, views.sdpa_params],
        schedule,
        spec.n_head.saturating_mul(256),
    ) {
        log::warn!("mega_pass|sdpa_merge|layer={}|{error:?}", spec.layer);
        return None;
    }
    Some(())
}

fn compile<'a>(
    dev: &'a MultiWeightDevice,
    source: &str,
    entry: &str,
    bindings: &[u32],
) -> Option<CudaPipeline<'a>> {
    CudaPipeline::compile_cuda_c_source_cached(&dev.ctx, source, entry, bindings).ok()
}
