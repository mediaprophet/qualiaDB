//! Shared-device and host-policy adapter for the canonical Metal lane.
pub use etalon_inference::backends::metal_lane::*;
fn install_device_source() {
    #[cfg(all(target_os = "macos", feature = "gpu-runtime"))]
    {
        let _ = etalon_inference::backends::metal_lane::device_source::install(|| {
            crate::gpu_context::try_shared_gpu().map(|owner| {
                etalon_inference::backends::metal_lane::device_source::MetalDevice {
                    device: owner.device.clone(),
                    queue: owner.queue.clone(),
                }
            })
        });
    }
}
pub fn metal_lane_capability() -> MetalCapabilityState {
    install_device_source();
    etalon_inference::backends::metal_lane::metal_lane_capability()
}
pub fn metal_lane_capability_state() -> &'static str {
    install_device_source();
    etalon_inference::backends::metal_lane::metal_lane_capability_state()
}
pub fn clear_metal_kv_cache() {
    install_device_source();
    etalon_inference::backends::metal_lane::clear_metal_kv_cache()
}
pub fn project_weight_bytes(
    weight_raw: &[u8],
    in_dim: usize,
    out_dim: usize,
    x: &[f32],
    out: &mut [f32],
) -> bool {
    install_device_source();
    etalon_inference::backends::metal_lane::project_weight_bytes(
        weight_raw, in_dim, out_dim, x, out,
    )
}
pub fn execute_metal_mega_pass(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    n_layer: u32,
    token_idx: u32,
    max_context: u32,
    _layer_stride: u32,
    _slot_kv_elems: u32,
    rope_base: f32,
    rope_scale: f32,
    rms_eps: f32,
    hidden: &[f32],
    layers: &[MetalPassLayerWeights<'_>],
    layer_dims: &[MetalPassLayerDims],
    output_norm: Option<&[f32]>,
    lm_head_raw: Option<&[u8]>,
    lm_head_in: usize,
    lm_head_out: usize,
) -> Option<u32> {
    install_device_source();
    etalon_inference::backends::metal_lane::execute_metal_mega_pass(
        n_embd,
        n_head,
        n_kv,
        head_dim,
        n_layer,
        token_idx,
        max_context,
        _layer_stride,
        _slot_kv_elems,
        rope_base,
        rope_scale,
        rms_eps,
        hidden,
        layers,
        layer_dims,
        output_norm,
        lm_head_raw,
        lm_head_in,
        lm_head_out,
    )
}
pub fn try_metal_mega_pass(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    n_layer: u32,
    token_idx: u32,
    max_context: u32,
    layer_stride: u32,
    slot_kv_elems: u32,
    rope_base: f32,
    rope_scale: f32,
    rms_eps: f32,
    hidden: &[f32],
    layers: &[MetalPassLayerWeights<'_>],
    layer_dims: &[MetalPassLayerDims],
    output_norm: Option<&[f32]>,
    lm_head_raw: Option<&[u8]>,
    lm_head_in: usize,
    lm_head_out: usize,
) -> Option<u32> {
    install_device_source();
    etalon_inference::backends::metal_lane::try_metal_mega_pass(
        n_embd,
        n_head,
        n_kv,
        head_dim,
        n_layer,
        token_idx,
        max_context,
        layer_stride,
        slot_kv_elems,
        rope_base,
        rope_scale,
        rms_eps,
        hidden,
        layers,
        layer_dims,
        output_norm,
        lm_head_raw,
        lm_head_in,
        lm_head_out,
    )
}
pub fn metal_mega_pass_available() -> bool {
    install_device_source();
    etalon_inference::backends::metal_lane::metal_mega_pass_available()
}
pub fn warm_metal_context() {
    install_device_source();
    etalon_inference::backends::metal_lane::warm_metal_context()
}
pub fn metal_double_buffered_prefetch(weight_data: &[u8], dst_offset: u64) -> bool {
    install_device_source();
    etalon_inference::backends::metal_lane::metal_double_buffered_prefetch(weight_data, dst_offset)
}
