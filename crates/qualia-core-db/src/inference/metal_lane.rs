//! Metal mega-pass orchestrator: chain all transformer layers into one Metal
//! command buffer with a single fence at the end. No per-layer readback.
//!
//! This mirrors the CUDA `cuda_lane/mega_pass.rs` architecture but targets
//! Apple Silicon via `metal-rs` or wgpu's Metal backend. On non-Apple
//! platforms, all functions return `None`/`false` — the wgpu WGSL path
//! handles inference instead.
//!
//! ## Architecture
//!
//! 1. Upload hidden state to device buffer (once).
//! 2. For each layer:
//!    a. RMSNorm + QKV + RoPE (fused dispatch via `fused-qkv-rope` MSL kernel)
//!    b. KV cache write
//!    c. SDPA decode (via `sdpa-decode` MSL kernel)
//!    d. O-proj GEMV + residual add
//!    e. RMSNorm + SwiGLU (fused)
//!    f. Down GEMV + residual add
//! 3. Output norm + logits GEMV + argmax
//! 4. Single readback of the final token
//!
//! All dispatches share one command buffer — the GPU never idles waiting
//! for CPU between layers.

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

/// Metal mega-pass: chain all transformer layers into one Metal command buffer
/// with a **single fence** at the end. No per-layer readback. The hidden state
/// stays on-device for the entire forward pass; only the final logits/token
/// come back to host.
///
/// This is the Apple Silicon equivalent of `try_cuda_mega_pass`.
///
/// **Requirements:**
/// - `metal-rs` crate (or wgpu Metal backend) must be available.
/// - Device KV cache must be initialized.
/// - All weights must be resident on-device.
///
/// Returns `Some(token)` on success, `None` if Metal is unavailable or
/// any kernel fails.
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
    // Metal is only available on macOS. On other platforms, return None
    // and let the wgpu WGSL path handle inference.
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
    #[cfg(target_os = "macos")]
    {
        // TODO: implement via metal-rs when available on the build target.
        // The MSL kernels (rmsnorm, fused-qkv-rope, sdpa-decode, gemv-simd-matrix)
        // are already emitted by the MSL emitter. The execution bridge needs
        // `WgpuPipeline::compile_msl` to be implemented with metal-rs.
        log::info!("metal_mega_pass: not yet implemented on this build");
        None
    }
}

/// Check if the Metal mega-pass is available on this platform.
pub fn metal_mega_pass_available() -> bool {
    cfg!(target_os = "macos")
}

/// Warm the Metal context (pre-compile kernels, pre-allocate arenas).
/// On non-Apple platforms, this is a no-op.
pub fn warm_metal_context() {
    #[cfg(target_os = "macos")]
    {
        // TODO: pre-compile MSL kernels and cache them
    }
}

/// MSL double-buffered weight streaming: overlap H2D weight copies with
/// compute using `MTLBlitCommandEncoder` on a separate Metal command buffer.
/// This is the Metal equivalent of CUDA's `write_view_prefetch` + `join_prefetch`
/// on a secondary stream.
///
/// **Architecture:**
/// 1. Issue async H2D copies for next layer's weights via `MTLBlitCommandEncoder`
///    on a dedicated command buffer (prefetch buffer).
/// 2. Encode compute kernels for current layer on the main command buffer.
/// 3. Insert a `MTLEvent` signal so compute waits for prefetch to complete.
/// 4. Repeat for each layer — compute and prefetch overlap.
///
/// **Requirements:** `metal-rs` crate (macOS only). Not implementable on
/// non-Apple platforms.
pub fn metal_double_buffered_prefetch(_weight_data: &[u8], _dst_offset: u64) -> bool {
    #[cfg(target_os = "macos")]
    {
        // TODO: implement via metal-rs MTLBlitCommandEncoder
        false
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
    fn metal_mega_pass_unavailable_on_non_macos() {
        if cfg!(not(target_os = "macos")) {
            assert!(!metal_mega_pass_available());
            assert!(try_metal_mega_pass(
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0.0,
                0.0,
                0.0,
                &[],
                &[],
                &[],
                None,
                None,
                0,
                0,
            )
            .is_none());
        }
    }
}
