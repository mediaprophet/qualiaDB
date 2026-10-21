//! Bridge between `etalon-tensor` accelerated operators and QualiaDB inference.
//!
//! Exposes unified forward execution arenas, FlashAttention compute pipelines,
//! in-shader top-k reduction, and DirectML / CUDA tensor execution lanes.

pub use etalon_tensor::{
    online_softmax_rescale, AttentionParams, CapturedCudaMegaPass, CudaMegaPassExecutor,
    DirectMlComputeLane, DmlGemmOp, DmlGemmParams, DmlLane, ForwardArena, ForwardMode,
    FusedAttentionPipeline, KvCacheLayout, LogitMaskHeader, PagedKvBlockTable, QuantScheme,
    TensorView, TopkParams, TopkReductionPipeline, FUSED_ATTENTION_WGSL, TOPK_REDUCTION_WGSL,
};

/// Construct a unified forward execution arena on the given GPU device.
///
/// Sizes the arena's intermediate buffers to accommodate the maximum dimension
/// across `hidden_dim`, `intermediate_dim`, and `vocab_size` for `max_batch` elements.
pub fn build_forward_arena_on(
    device: &wgpu::Device,
    hidden_dim: u32,
    intermediate_dim: u32,
    vocab_size: u32,
    max_batch: u32,
) -> ForwardArena {
    let effective_dim = hidden_dim.max(intermediate_dim).max(vocab_size);
    let mut arena = ForwardArena::new(device, max_batch, effective_dim, 1);
    arena.hidden_dim = hidden_dim;
    arena
}

/// Construct a compiled fused FlashAttention compute pipeline on the given device.
pub fn build_fused_attention_on(device: &wgpu::Device) -> FusedAttentionPipeline {
    FusedAttentionPipeline::new(device)
}

/// Construct a compiled GPU block top-k reduction compute pipeline on the given device.
pub fn build_topk_pipeline_on(device: &wgpu::Device) -> TopkReductionPipeline {
    TopkReductionPipeline::new(device)
}

/// Attempt to construct a unified forward execution arena using the process-wide shared GPU device.
#[cfg(not(target_arch = "wasm32"))]
pub fn try_shared_forward_arena(
    hidden_dim: u32,
    intermediate_dim: u32,
    vocab_size: u32,
    max_batch: u32,
) -> Option<ForwardArena> {
    crate::gpu_context::try_shared_gpu().map(|ctx| {
        build_forward_arena_on(
            &ctx.device,
            hidden_dim,
            intermediate_dim,
            vocab_size,
            max_batch,
        )
    })
}

/// Attempt to construct a fused FlashAttention compute pipeline using the process-wide shared GPU device.
#[cfg(not(target_arch = "wasm32"))]
pub fn try_shared_fused_attention() -> Option<FusedAttentionPipeline> {
    crate::gpu_context::try_shared_gpu().map(|ctx| build_fused_attention_on(&ctx.device))
}

/// Attempt to construct a top-k reduction compute pipeline using the process-wide shared GPU device.
#[cfg(not(target_arch = "wasm32"))]
pub fn try_shared_topk_pipeline() -> Option<TopkReductionPipeline> {
    crate::gpu_context::try_shared_gpu().map(|ctx| build_topk_pipeline_on(&ctx.device))
}

/// Dispatch fused attention on an active compute pass.
#[inline]
pub fn dispatch_fused_attention<'a>(
    pipeline: &'a FusedAttentionPipeline,
    cpass: &mut wgpu::ComputePass<'a>,
    bind_group: &'a wgpu::BindGroup,
    num_heads: u32,
) {
    pipeline.dispatch(cpass, bind_group, num_heads);
}

/// Dispatch top-k block reduction on an active compute pass.
#[inline]
pub fn dispatch_topk_reduction<'a>(
    pipeline: &'a TopkReductionPipeline,
    cpass: &mut wgpu::ComputePass<'a>,
    bind_group: &'a wgpu::BindGroup,
    num_blocks: u32,
) {
    pipeline.dispatch(cpass, bind_group, num_blocks);
}

/// Record a copy of token candidates into the forward arena readback stage.
#[inline]
pub fn record_readback_copy(
    arena: &ForwardArena,
    encoder: &mut wgpu::CommandEncoder,
    token_src: &wgpu::Buffer,
    mode: ForwardMode,
) {
    arena.record_readback_copy(encoder, token_src, mode);
}

/// Blocking readback of a single decoded token from the forward arena staging buffer.
#[inline]
pub fn read_token_blocking(arena: &ForwardArena, device: &wgpu::Device) -> Option<u32> {
    arena.read_token_blocking(device)
}

/// Upload attention uniform parameters to a uniform buffer using the device queue.
#[inline]
pub fn upload_attention_params(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    offset: wgpu::BufferAddress,
    params: &AttentionParams,
) {
    queue.write_buffer(buffer, offset, bytemuck::bytes_of(params));
}

/// Upload top-k uniform parameters to a uniform buffer using the device queue.
#[inline]
pub fn upload_topk_params(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    offset: wgpu::BufferAddress,
    params: &TopkParams,
) {
    queue.write_buffer(buffer, offset, bytemuck::bytes_of(params));
}

/// Upload logit mask header to a uniform/storage buffer using the device queue.
#[inline]
pub fn upload_logit_mask_header(
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    offset: wgpu::BufferAddress,
    header: &LogitMaskHeader,
) {
    queue.write_buffer(buffer, offset, bytemuck::bytes_of(header));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_online_softmax_rescale_all_masked_guard() {
        let (rescale, weight) = online_softmax_rescale(f32::NEG_INFINITY, f32::NEG_INFINITY);
        assert_eq!(rescale, 0.0);
        assert_eq!(weight, 0.0);

        let (rescale, weight) = online_softmax_rescale(10.0, f32::MIN);
        assert_eq!(rescale, 0.0);
        assert_eq!(weight, 0.0);

        let (rescale, weight) = online_softmax_rescale(f32::MIN, 5.0);
        assert_eq!(rescale, 0.0);
        assert_eq!(weight, 1.0);

        let (rescale, weight) = online_softmax_rescale(4.0, 4.0);
        assert!((rescale - 1.0).abs() < 1e-6);
        assert_eq!(weight, 1.0);

        let (rescale, weight) = online_softmax_rescale(2.0, 4.0);
        let expected = (2.0f32 - 4.0f32).exp();
        assert!((rescale - expected).abs() < 1e-6);
        assert_eq!(weight, 1.0);
    }

    #[test]
    fn test_kv_cache_layout_and_paged_table() {
        assert_eq!(KvCacheLayout::PagedBlock, KvCacheLayout::PagedBlock);
        assert_ne!(KvCacheLayout::Fp32, KvCacheLayout::Int8Quantized);

        let mut table = PagedKvBlockTable::new(64, 32, 512);
        assert_eq!(table.block_size, 64);
        assert_eq!(table.num_layers, 32);
        assert_eq!(table.max_blocks, 512);
        assert!(table.block_entries.is_empty());

        table.block_entries.push(0x1234);
        table.block_entries.push(0x5678);
        assert_eq!(table.block_entries.len(), 2);
        assert_eq!(table.block_entries[0], 0x1234);
    }

    #[test]
    fn test_logit_mask_header_alignment_and_size() {
        assert_eq!(core::mem::size_of::<LogitMaskHeader>(), 80);
        assert_eq!(core::mem::align_of::<LogitMaskHeader>(), 16);

        let header = LogitMaskHeader::default();
        assert_eq!(header.mask_words.len(), 16);
        assert_eq!(header.mask_words[0], u32::MAX);
        assert_eq!(header.temperature, 1.0);
        assert_eq!(header.top_p, 1.0);
        assert_eq!(header.min_p, 0.0);
        assert_eq!(header.presence_penalty, 0.0);

        let bytes = bytemuck::bytes_of(&header);
        assert_eq!(bytes.len(), 80);
    }

    #[test]
    fn test_attention_and_topk_params_layout() {
        assert_eq!(core::mem::size_of::<AttentionParams>(), 48);
        assert_eq!(core::mem::align_of::<AttentionParams>(), 16);

        assert_eq!(core::mem::size_of::<TopkParams>(), 16);
        assert_eq!(core::mem::align_of::<TopkParams>(), 16);
    }
}
