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

    fn make_test_expert<'a>(
        gate_up_packed: &'a mut [u8],
        gate_up_scale: &'a mut [u8],
        down_packed: &'a mut [u8],
        down_scale: &'a mut [u8],
        val_byte: u8,
        emb_dim: usize,
        intermediate_dim: usize,
    ) -> ExpertWeightView<'a> {
        for b in gate_up_packed.iter_mut() {
            *b = val_byte;
        }
        for s in gate_up_scale.iter_mut() {
            *s = 0x38; // 1.0 in FP8 E4M3
        }

        for b in down_packed.iter_mut() {
            *b = val_byte;
        }
        for s in down_scale.iter_mut() {
            *s = 0x38;
        }

        ExpertWeightView {
            gate_up_packed,
            gate_up_scale,
            gate_up_global: 1.0,
            down_packed,
            down_scale,
            down_global: 1.0,
            intermediate_dim,
            emb_dim,
        }
    }

    #[test]
    fn test_swiglu_expert_nvfp4_matches_oracle() {
        let emb_dim = 16;
        let inter_dim = 16;
        let mut gu_packed = vec![0u8; 2 * inter_dim * (emb_dim / 2)];
        let mut gu_scale = vec![0u8; 2 * inter_dim * (emb_dim / 16)];
        let mut d_packed = vec![0u8; emb_dim * (inter_dim / 2)];
        let mut d_scale = vec![0u8; emb_dim * (inter_dim / 16)];

        // val_byte = 0x42: low nibble = 2 (1.0), high nibble = 4 (2.0)
        let expert = make_test_expert(
            &mut gu_packed,
            &mut gu_scale,
            &mut d_packed,
            &mut d_scale,
            0x42,
            emb_dim,
            inter_dim,
        );

        let input = vec![1.0f32; emb_dim];
        let mut gate_buf = [0.0f32; 16];
        let mut up_buf = [0.0f32; 16];
        let mut swiglu_buf = [0.0f32; 16];
        let mut out = [0.0f32; 16];

        evaluate_swiglu_expert_nvfp4(
            &input,
            &expert,
            &mut gate_buf,
            &mut up_buf,
            &mut swiglu_buf,
            &mut out,
        )
        .unwrap();

        // Calculate expected:
        // Each row of gate and up: 8 pairs of (1.0, 2.0). With input all 1.0:
        // dot = 8 * (1.0 * 1.0 + 2.0 * 1.0) = 8 * 3.0 = 24.0
        let expected_gate = 24.0f32;
        let expected_up = 24.0f32;
        let expected_swiglu = silu(expected_gate) * expected_up;

        // Down projection: 8 pairs of (1.0, 2.0) with swiglu vector (all expected_swiglu):
        // out[r] = 8 * (1.0 * expected_swiglu + 2.0 * expected_swiglu) = 24.0 * expected_swiglu
        let expected_out = 24.0f32 * expected_swiglu;

        for i in 0..inter_dim {
            assert!((gate_buf[i] - expected_gate).abs() < 1e-4);
            assert!((up_buf[i] - expected_up).abs() < 1e-4);
            assert!((swiglu_buf[i] - expected_swiglu).abs() < 1e-4);
            assert!((out[i] - expected_out).abs() < 1e-3);
        }
    }

    #[test]
    fn test_dispatch_moe_step_routed_and_shared_experts_matches_oracle() {
        let emb_dim = 16;
        let inter_dim = 16;
        let num_experts = 4;
        let topk = 2;

        // Shared expert: val_byte 0x22 -> low 2 (1.0), high 2 (1.0). dot = 16 * 1.0 = 16.0
        let mut shared_gu_packed = vec![0u8; 2 * inter_dim * (emb_dim / 2)];
        let mut shared_gu_scale = vec![0u8; 2 * inter_dim * (emb_dim / 16)];
        let mut shared_d_packed = vec![0u8; emb_dim * (inter_dim / 2)];
        let mut shared_d_scale = vec![0u8; emb_dim * (inter_dim / 16)];

        let shared_expert = make_test_expert(
            &mut shared_gu_packed,
            &mut shared_gu_scale,
            &mut shared_d_packed,
            &mut shared_d_scale,
            0x22,
            emb_dim,
            inter_dim,
        );

        // Router weights designed to favor expert 1 and expert 3
        let mut router_weights = vec![0.0f32; num_experts * emb_dim];
        for d in 0..emb_dim {
            router_weights[0 * emb_dim + d] = 0.1;
            router_weights[1 * emb_dim + d] = 2.0; // High
            router_weights[2 * emb_dim + d] = 0.2;
            router_weights[3 * emb_dim + d] = 3.0; // Highest
        }

        let input = vec![1.0f32; emb_dim];
        let mut scratch_accum = vec![0.0f32; emb_dim];

        // Fetcher returns expert 0..3 with val_byte 0x42
        let expert_fetcher = |_id: u16| -> Option<ExpertWeightView<'static>> {
            // Leaking for the lifetime of test
            let gu_p: &'static mut [u8] = Box::leak(vec![0x42u8; 2 * 16 * 8].into_boxed_slice());
            let gu_s: &'static mut [u8] = Box::leak(vec![0x38u8; 2 * 16 * 1].into_boxed_slice());
            let d_p: &'static mut [u8] = Box::leak(vec![0x42u8; 16 * 8].into_boxed_slice());
            let d_s: &'static mut [u8] = Box::leak(vec![0x38u8; 16 * 1].into_boxed_slice());
            Some(ExpertWeightView {
                gate_up_packed: gu_p,
                gate_up_scale: gu_s,
                gate_up_global: 1.0,
                down_packed: d_p,
                down_scale: d_s,
                down_global: 1.0,
                intermediate_dim: 16,
                emb_dim: 16,
            })
        };

        dispatch_moe_step(
            &input,
            emb_dim,
            &router_weights,
            num_experts,
            topk,
            expert_fetcher,
            Some(&shared_expert),
            0.5,
            &mut scratch_accum,
        )
        .unwrap();

        // 1. Routed experts: 1 and 3 are selected.
        // Dot product with router weights:
        // logit[1] = 16 * 2.0 = 32.0
        // logit[3] = 16 * 3.0 = 48.0
        // Softmax over top-2: w1 = exp(32)/(exp(32)+exp(48)), w3 = exp(48)/(exp(32)+exp(48))
        // Note: softmax normalized sum w1 + w3 == 1.0.
        // Since both experts 1 and 3 have the identical output val (24.0 * silu(24.0) * 24.0 = ~576.0),
        // the weighted sum of routed experts is simply 1.0 * expert_out = 24.0 * silu(24.0) * 24.0.
        let routed_single_out = 24.0f32 * silu(24.0) * 24.0;

        // 2. Shared expert: 0x22 -> low 1.0, high 1.0 -> dot = 16.0
        // gate = 16.0, up = 16.0 -> swiglu = silu(16.0) * 16.0
        // down = 16.0 * swiglu -> shared_out = 16.0 * silu(16.0) * 16.0
        let shared_single_out = 16.0f32 * silu(16.0) * 16.0;
        let expected_total = routed_single_out + 0.5 * shared_single_out;

        for d in 0..emb_dim {
            assert!(
                (scratch_accum[d] - expected_total).abs() < 1e-2,
                "Dim {d} mismatch: actual={}, expected={}",
                scratch_accum[d],
                expected_total
            );
        }
    }
}
