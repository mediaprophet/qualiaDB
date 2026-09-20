//! Zero-heap MoE layer dispatcher and SwiGLU accumulator (Work Package F9/F10).
//!
//! Evaluates top-k routed experts and shared experts using zero-heap NVFP4 GEMV
//! on caller-supplied stack scratch, strictly conforming to the 42MB Sentinel
//! and Rule 0 (zero heap in hot paths).

use super::nvfp4::{nvfp4_gemv_zero_heap, Nvfp4Error};
use super::routing::{route_topk, MAX_MOE_TOPK};

pub const MAX_ROUTED_EXPERTS: usize = 256;
pub const MAX_INTERMEDIATE_DIM: usize = 4096;

/// SiLU (Swish) activation function: x * sigmoid(x).
#[inline(always)]
pub fn silu(x: f32) -> f32 {
    x / (1.0 + (-x).exp())
}

/// Description of one expert's raw NVFP4 matrices.
pub struct ExpertWeightView<'a> {
    /// Gate + Up packed matrix (NVFP4, 2 nibbles per byte)
    pub gate_up_packed: &'a [u8],
    /// Per-16 FP8 block scales
    pub gate_up_scale: &'a [u8],
    pub gate_up_global: f32,
    /// Down packed matrix (NVFP4)
    pub down_packed: &'a [u8],
    pub down_scale: &'a [u8],
    pub down_global: f32,
    pub intermediate_dim: usize,
    pub emb_dim: usize,
}

/// Zero-heap evaluation of one SwiGLU expert:
/// y = (SiLU(x * W_gate) * (x * W_up)) * W_down
pub fn evaluate_swiglu_expert_nvfp4(
    input: &[f32],
    weights: &ExpertWeightView<'_>,
    gate_buf: &mut [f32],
    up_buf: &mut [f32],
    swiglu_buf: &mut [f32],
    out: &mut [f32],
) -> Result<(), Nvfp4Error> {
    let emb_dim = weights.emb_dim;
    let inter_dim = weights.intermediate_dim;

    if input.len() < emb_dim || out.len() < emb_dim {
        return Err(Nvfp4Error::BufferTooSmall);
    }
    if gate_buf.len() < inter_dim || up_buf.len() < inter_dim || swiglu_buf.len() < inter_dim {
        return Err(Nvfp4Error::BufferTooSmall);
    }

    // In packed gate_up, gate is rows 0..inter_dim, up is rows inter_dim..2*inter_dim
    let bytes_per_row = emb_dim / 2;
    let blocks_per_row = emb_dim / super::nvfp4::NVFP4_BLOCK_SIZE;

    let gate_packed = &weights.gate_up_packed[..inter_dim * bytes_per_row];
    let gate_scales = &weights.gate_up_scale[..inter_dim * blocks_per_row];
    nvfp4_gemv_zero_heap(
        input,
        gate_packed,
        gate_scales,
        weights.gate_up_global,
        inter_dim,
        emb_dim,
        &mut gate_buf[..inter_dim],
    )?;

    let up_packed =
        &weights.gate_up_packed[inter_dim * bytes_per_row..2 * inter_dim * bytes_per_row];
    let up_scales =
        &weights.gate_up_scale[inter_dim * blocks_per_row..2 * inter_dim * blocks_per_row];
    nvfp4_gemv_zero_heap(
        input,
        up_packed,
        up_scales,
        weights.gate_up_global,
        inter_dim,
        emb_dim,
        &mut up_buf[..inter_dim],
    )?;

    // SwiGLU activation: silu(gate) * up
    for i in 0..inter_dim {
        swiglu_buf[i] = silu(gate_buf[i]) * up_buf[i];
    }

    // Down projection: swiglu * W_down -> out (emb_dim)
    nvfp4_gemv_zero_heap(
        &swiglu_buf[..inter_dim],
        weights.down_packed,
        weights.down_scale,
        weights.down_global,
        emb_dim,
        inter_dim,
        &mut out[..emb_dim],
    )?;

    Ok(())
}

/// Dispatch MoE layer step:
/// 1. Compute router logits
/// 2. Select top-k experts
/// 3. Compute SwiGLU for each selected expert and accumulate weighted output
pub fn dispatch_moe_step<F>(
    input: &[f32],
    emb_dim: usize,
    router_weights: &[f32], // [num_experts, emb_dim]
    num_experts: usize,
    topk: usize,
    mut expert_fetcher: F,
    shared_expert: Option<&ExpertWeightView<'_>>,
    shared_gate_weight: f32,
    scratch_accum: &mut [f32],
) -> Result<(), String>
where
    F: FnMut(u16) -> Option<ExpertWeightView<'static>>,
{
    if input.len() < emb_dim || scratch_accum.len() < emb_dim {
        return Err("MoE dispatch: input or accumulator buffer too small".into());
    }
    if num_experts > MAX_ROUTED_EXPERTS {
        return Err("MoE dispatch: num_experts exceeds MAX_ROUTED_EXPERTS".into());
    }

    // 1. Router logits: g_e = input . W_router[e]
    let mut gate_logits = [0.0f32; MAX_ROUTED_EXPERTS];
    for e in 0..num_experts {
        let mut dot = 0.0f32;
        let w_row = &router_weights[e * emb_dim..(e + 1) * emb_dim];
        for i in 0..emb_dim {
            dot += input[i] * w_row[i];
        }
        gate_logits[e] = dot;
    }

    // 2. Route top-k
    let mut expert_indices = [0u16; MAX_MOE_TOPK];
    let mut expert_weights = [0.0f32; MAX_MOE_TOPK];
    let selected = route_topk(
        &gate_logits[..num_experts],
        topk,
        true,
        &mut expert_indices,
        &mut expert_weights,
    )
    .map_err(|e| format!("route_topk failed: {}", e))?;

    // Clear accumulator
    for x in scratch_accum[..emb_dim].iter_mut() {
        *x = 0.0;
    }

    // Scratch buffers for intermediate SwiGLU
    let mut gate_buf = [0.0f32; MAX_INTERMEDIATE_DIM];
    let mut up_buf = [0.0f32; MAX_INTERMEDIATE_DIM];
    let mut swiglu_buf = [0.0f32; MAX_INTERMEDIATE_DIM];
    let mut expert_out = [0.0f32; 8192];

    // 3. Accumulate routed experts
    for i in 0..selected {
        let expert_id = expert_indices[i];
        let weight = expert_weights[i];

        if let Some(view) = expert_fetcher(expert_id) {
            evaluate_swiglu_expert_nvfp4(
                input,
                &view,
                &mut gate_buf,
                &mut up_buf,
                &mut swiglu_buf,
                &mut expert_out[..emb_dim],
            )
            .map_err(|e| format!("evaluate_swiglu_expert failed: {}", e))?;

            for d in 0..emb_dim {
                scratch_accum[d] += weight * expert_out[d];
            }
        }
    }

    // 4. Evaluate shared expert if present
    if let Some(shared) = shared_expert {
        evaluate_swiglu_expert_nvfp4(
            input,
            shared,
            &mut gate_buf,
            &mut up_buf,
            &mut swiglu_buf,
            &mut expert_out[..emb_dim],
        )
        .map_err(|e| format!("evaluate shared expert failed: {}", e))?;

        for d in 0..emb_dim {
            scratch_accum[d] += shared_gate_weight * expert_out[d];
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silu() {
        assert_eq!(silu(0.0), 0.0);
        assert!((silu(1.0) - 0.7310586).abs() < 1e-5);
    }
}
