use super::*;
use crate::ggml_quants::GGML_TYPE_F32;
use crate::gguf_bridge::QTensorEngine;
use crate::gguf_sharder::GgufTensorInfo;

fn expert_tensor(offset: u64, dims: [u64; 4]) -> GgufTensorInfo {
    GgufTensorInfo {
        dims,
        n_dims: 3,
        ggml_type: GGML_TYPE_F32,
        byte_offset: offset,
    }
}

fn write_f32s(dst: &mut [u8], values: &[f32]) {
    for (index, value) in values.iter().enumerate() {
        let start = index * core::mem::size_of::<f32>();
        dst[start..start + core::mem::size_of::<f32>()].copy_from_slice(&value.to_le_bytes());
    }
}

#[test]
fn ordinary_gguf_moe_uses_the_selected_expert_batch() {
    // Each tensor has shape [input=2, rows=2, experts=2].  Expert 1 has
    // deliberately different weights, proving that a 3-D tensor is neither
    // truncated to expert 0 nor addressed with the wrong flattened stride.
    let mut mmap = [0u8; 96];
    write_f32s(&mut mmap[0..32], &[1.0, 0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0]);
    write_f32s(&mut mmap[32..64], &[1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0]);
    write_f32s(&mut mmap[64..96], &[1.0, 0.0, 0.0, 1.0, 4.0, 0.0, 0.0, 5.0]);

    let gate = expert_tensor(0, [2, 2, 2, 0]);
    let up = expert_tensor(32, [2, 2, 2, 0]);
    let down = expert_tensor(64, [2, 2, 2, 0]);
    let input = [1.0, 0.0];
    let mut gate_buf = [0.0; 2];
    let mut up_buf = [0.0; 2];
    let mut swiglu_buf = [0.0; 2];
    let mut out = [0.0; 2];

    assert!(evaluate_gguf_swiglu_expert(
        &mmap,
        0,
        &gate,
        &up,
        &down,
        1,
        &input,
        &mut gate_buf,
        &mut up_buf,
        &mut swiglu_buf,
        &mut out,
    ));

    assert!((out[0] - 4.0 * crate::inference::moe::dispatch::silu(2.0)).abs() < 1e-6);
    assert!((out[1] - 5.0 * crate::inference::moe::dispatch::silu(3.0)).abs() < 1e-6);
}

#[test]
fn fused_gate_up_gguf_expert_preserves_gate_and_up_row_halves() {
    // Gemma stores [gate rows; up rows] in one [input=2, rows=4, experts=1] tensor.
    let mut mmap = [0u8; 48];
    write_f32s(
        &mut mmap[..32],
        &[
            1.0, 0.0, // gate row 0
            2.0, 0.0, // gate row 1
            3.0, 0.0, // up row 0
            4.0, 0.0, // up row 1
        ],
    );
    write_f32s(&mut mmap[32..], &[1.0, 0.0, 0.0, 1.0]);
    let gate_up = expert_tensor(0, [2, 4, 1, 0]);
    let down = expert_tensor(32, [2, 2, 1, 0]);
    let input = [1.0, 0.0];
    let mut gate_buf = [0.0; 2];
    let mut up_buf = [0.0; 2];
    let mut swiglu_buf = [0.0; 2];
    let mut out = [0.0; 2];

    assert!(evaluate_gguf_swiglu_expert_fused_gate_up(
        &mmap,
        0,
        &gate_up,
        &down,
        0,
        &input,
        &mut gate_buf,
        &mut up_buf,
        &mut swiglu_buf,
        &mut out,
    ));
    assert!((out[0] - 3.0 * crate::inference::moe::dispatch::silu(1.0)).abs() < 1e-6);
    assert!((out[1] - 4.0 * crate::inference::moe::dispatch::silu(2.0)).abs() < 1e-6);
}

#[test]
fn qtensor_engine_dispatches_moe_step_routed_with_shared_expert() {
    let mut engine = QTensorEngine::new();
    let emb_dim = 16;
    let intermediate_dim = 16;
    let num_experts = 4;
    let topk = 2;

    let shared_gu_p = vec![0x42u8; 2 * intermediate_dim * (emb_dim / 2)];
    let shared_gu_s = vec![0x38u8; 2 * intermediate_dim * (emb_dim / 16)];
    let shared_d_p = vec![0x42u8; emb_dim * (intermediate_dim / 2)];
    let shared_d_s = vec![0x38u8; emb_dim * (intermediate_dim / 16)];

    let shared_view = crate::inference::moe::dispatch::ExpertWeightView {
        gate_up_packed: &shared_gu_p,
        gate_up_scale: &shared_gu_s,
        gate_up_global: 1.0,
        down_packed: &shared_d_p,
        down_scale: &shared_d_s,
        down_global: 1.0,
        intermediate_dim,
        emb_dim,
    };

    let router_weights = vec![1.0f32; num_experts * emb_dim];
    let input = vec![1.0f32; emb_dim];
    let mut scratch_a = vec![0.0f32; emb_dim];

    let expert_fetcher = |_id: u16| -> Option<crate::inference::moe::dispatch::ExpertWeightView<'static>> {
        let gu_p: &'static mut [u8] = Box::leak(vec![0x42u8; 2 * 16 * 8].into_boxed_slice());
        let gu_s: &'static mut [u8] = Box::leak(vec![0x38u8; 2 * 16 * 1].into_boxed_slice());
        let d_p: &'static mut [u8] = Box::leak(vec![0x42u8; 16 * 8].into_boxed_slice());
        let d_s: &'static mut [u8] = Box::leak(vec![0x38u8; 16 * 1].into_boxed_slice());
        Some(crate::inference::moe::dispatch::ExpertWeightView {
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

    let ok = engine.dispatch_moe_step_routed(
        emb_dim,
        &router_weights,
        num_experts,
        topk,
        expert_fetcher,
        Some(&shared_view),
        0.5,
        &input,
        &mut scratch_a,
    );

    assert!(ok);
    assert!(scratch_a.iter().all(|&x| x > 0.0));
}

fn make_synthetic_q4k_blocks(n_blocks: usize, seed: u8) -> Vec<u8> {
    let mut raw = Vec::with_capacity(n_blocks * 144);
    for b in 0..n_blocks {
        let mut blk = [0u8; 144];
        blk[0] = 0x00;
        blk[1] = 0x14; // d
        blk[2] = 0x00;
        blk[3] = 0x0c; // dmin
        for i in 4..16 {
            blk[i] = ((seed as usize + b + i) & 0x3f) as u8;
        }
        for i in 16..144 {
            blk[i] = seed.wrapping_add((b as u8).wrapping_add(i as u8));
        }
        raw.extend_from_slice(&blk);
    }
    raw
}

#[test]
fn qtensor_engine_dispatches_clustered_moe_operator() {
    let mut engine = QTensorEngine::new();
    let model_dim = 256;
    let hidden_dim = 256;

    let num_blocks = (hidden_dim * model_dim) / 256;
    let base_gate = make_synthetic_q4k_blocks(num_blocks, 41);
    let base_up = make_synthetic_q4k_blocks(num_blocks, 42);
    let base_down = make_synthetic_q4k_blocks(num_blocks, 43);

    let anchor = crate::inference::operator_runtime::ClusterAnchor {
        cluster_id: 0,
        model_dim,
        hidden_dim,
        base_gate,
        base_up,
        base_down,
        expert_ids: vec![0],
        expert_gate_deltas: vec![None],
        expert_up_deltas: vec![None],
        expert_down_deltas: vec![None],
    };

    let op = crate::inference::operator_runtime::ClusteredMoEOperator::new(vec![anchor]);

    let routed = [
        crate::inference::operator_runtime::RoutedExpert {
            expert_id: 0,
            weight: 1.0,
        },
    ];

    let input = vec![0.1f32; model_dim];
    let mut output = vec![0.0f32; model_dim];

    let ws_gate_len = qualia_inference_kernel::operators::q4k_lookup_workspace_floats(model_dim).unwrap();
    let ws_down_len = qualia_inference_kernel::operators::q4k_lookup_workspace_floats(hidden_dim).unwrap();

    let mut gb = vec![0.0f32; hidden_dim];
    let mut ub = vec![0.0f32; hidden_dim];
    let mut hb = vec![0.0f32; hidden_dim];
    let mut accum = vec![0.0f32; hidden_dim];
    let mut down = vec![0.0f32; model_dim];
    let mut rank = vec![0.0f32; 16];
    let mut ws_gate = vec![0.0f32; ws_gate_len];
    let mut ws_down = vec![0.0f32; ws_down_len];

    let mut scratch = crate::inference::operator_runtime::ClusteredMoeScratch {
        gate_buf: &mut gb,
        up_buf: &mut ub,
        hidden_buf: &mut hb,
        cluster_accum_hidden: &mut accum,
        cluster_down_out: &mut down,
        rank_scratch: &mut rank,
        ws_gate_up: &mut ws_gate,
        ws_down: &mut ws_down,
    };

    let ok = engine.dispatch_clustered_moe_operator(
        &op,
        &routed,
        &input,
        &mut output,
        &mut scratch,
    );

    assert!(ok);
    assert!(output.iter().any(|&v| v.abs() > 1e-5));
}


