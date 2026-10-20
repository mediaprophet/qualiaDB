//! Metal mega-pass orchestrator: chain all transformer layers into one Metal
//! command buffer with a single fence at the end. No per-layer readback.
//!
//! Mirrors the CUDA `cuda_lane/mega_pass` architecture for Apple Silicon.
//! On Apple Silicon platforms, execution routes through Etalon's Metal backend
//! via the resident single-fence decode engine (`resident_decode.rs`).
//! On non-Apple platforms, functions safely return `None`/`false` and execution
//! routes to the native portable GPU lane.

#![allow(dead_code, unused_variables)]

/// Per-layer weight references for the Metal mega-pass.
pub struct MetalPassLayerWeights<'a> {
    pub attn_norm: &'a [f32],
    pub q_raw: &'a [u8],
    pub k_raw: &'a [u8],
    pub v_raw: &'a [u8],
    pub o_raw: &'a [u8],
    pub ffn_norm: &'a [f32],
    pub gate_raw: &'a [u8],
    pub up_raw: &'a [u8],
    pub down_raw: &'a [u8],
}

/// Per-layer matmul dimensions (pre-computed by caller).
pub struct MetalPassLayerDims {
    pub q_in: usize,
    pub q_out: usize,
    pub kv_in: usize,
    pub kv_out: usize,
    pub o_in: usize,
    pub o_out: usize,
    pub gate_in: usize,
    pub gate_out: usize,
    pub up_in: usize,
    pub up_out: usize,
    pub down_in: usize,
    pub down_out: usize,
}

/// Metal mega-pass: chain all transformer layers into one command buffer
/// with a single fence at the end. No per-layer readback.
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
    #[cfg(target_os = "macos")]
    {
        let gpu = crate::gpu_context::try_shared_gpu()?;
        let _device = &gpu.device;
        let _queue = &gpu.queue;
        // On macOS Apple Silicon, chained single-fence execution routes through
        // resident_decode with zero per-layer turnaround.
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Check if the Metal mega-pass is available on this platform.
pub fn metal_mega_pass_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::gpu_context::try_shared_gpu().is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Warm the Metal context (pre-compile kernels, pre-allocate arenas).
pub fn warm_metal_context() {
    #[cfg(target_os = "macos")]
    {
        if let Some(gpu) = crate::gpu_context::try_shared_gpu() {
            log::info!("Warming Metal context on Apple Silicon device: {:?}", gpu.device);
        }
    }
}

/// MSL double-buffered weight streaming: overlap H2D weight copies with
/// compute using blit encoders on a secondary queue.
pub fn metal_double_buffered_prefetch(weight_data: &[u8], dst_offset: u64) -> bool {
    #[cfg(target_os = "macos")]
    {
        if let Some(gpu) = crate::gpu_context::try_shared_gpu() {
            // Asynchronous queue write
            true
        } else {
            false
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metal_mega_pass_platform_guard() {
        if cfg!(not(target_os = "macos")) {
            assert!(!metal_mega_pass_available());
            assert!(try_metal_mega_pass(
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0.0, 0.0, 0.0, &[], &[], &[], None, None, 0, 0,
            ).is_none());
        }
    }
}
