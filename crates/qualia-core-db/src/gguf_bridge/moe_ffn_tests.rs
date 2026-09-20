use super::*;
use crate::ggml_quants::GGML_TYPE_F32;
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
