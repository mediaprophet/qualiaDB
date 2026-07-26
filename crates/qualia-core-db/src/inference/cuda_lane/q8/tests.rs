use super::{
    q8_0_gemv_oracle_into, q8_rmsnorm_qkv_rope_source, q8_rmsnorm_swiglu_source, Q8_0_BLOCK_BYTES,
    Q8_0_BLOCK_ELEMS, Q8_0_RMSNORM_QKV_ROPE_ENTRY, Q8_0_RMSNORM_SWIGLU_ENTRY,
};
use crate::inference::cuda_lane::device::{ensure_device, multi_weight_device};
use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

fn q8_matrix(n_in: usize, n_out: usize, seed: usize) -> Vec<u8> {
    let row_bytes = n_in / Q8_0_BLOCK_ELEMS * Q8_0_BLOCK_BYTES;
    let mut raw = vec![0u8; row_bytes * n_out];
    for row in 0..n_out {
        for block in 0..n_in / Q8_0_BLOCK_ELEMS {
            let base = row * row_bytes + block * Q8_0_BLOCK_BYTES;
            let scale = half::f16::from_f32(0.004 + ((row + seed) % 7) as f32 * 0.0003);
            raw[base..base + 2].copy_from_slice(&scale.to_bits().to_le_bytes());
            for lane in 0..Q8_0_BLOCK_ELEMS {
                raw[base + 2 + lane] =
                    (((row * 17 + block * 11 + lane * 5 + seed) % 241) as i16 - 120) as i8 as u8;
            }
        }
    }
    raw
}

#[test]
#[serial_test::serial]
fn fused_rmsnorm_swiglu_matches_composed_oracle_when_cuda_available() {
    let n_in = 64usize;
    let n_out = 19usize;
    let eps = 1.0e-5f32;
    let x: Vec<f32> = (0..n_in)
        .map(|index| -0.1 + (index as f32 * 0.09).cos())
        .collect();
    let norm: Vec<f32> = (0..n_in)
        .map(|index| 0.7 + (index % 11) as f32 * 0.031)
        .collect();
    let mean_sq = x.iter().map(|value| value * value).sum::<f32>() / n_in as f32;
    let inv_rms = (mean_sq + eps).sqrt().recip();
    let normalized: Vec<f32> = x
        .iter()
        .zip(&norm)
        .map(|(value, weight)| value * inv_rms * weight)
        .collect();
    let gate = q8_matrix(n_in, n_out, 19);
    let up = q8_matrix(n_in, n_out, 23);
    let mut expected_gate = vec![0.0f32; n_out];
    let mut expected_up = vec![0.0f32; n_out];
    assert!(q8_0_gemv_oracle_into(
        n_in,
        n_out,
        &normalized,
        &gate,
        &mut expected_gate
    ));
    assert!(q8_0_gemv_oracle_into(
        n_in,
        n_out,
        &normalized,
        &up,
        &mut expected_up
    ));
    let expected: Vec<f32> = expected_gate
        .iter()
        .zip(expected_up)
        .map(|(gate, up)| (gate / (1.0 + (-gate).exp())) * up)
        .collect();

    set_inference_mode(InferenceMode::CudaTc);
    let Ok(mut guard) = multi_weight_device().lock() else {
        return;
    };
    if !ensure_device(&mut guard) {
        set_inference_mode(InferenceMode::Portable);
        return;
    }
    let dev = guard.as_mut().unwrap();
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let vx = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&x), 0, 0)
        .unwrap();
    let vn = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&norm), 1, 0)
        .unwrap();
    let vg = dev.ctx.allocate_and_write(&gate, 2, 0).unwrap();
    let vu = dev.ctx.allocate_and_write(&up, 3, 0).unwrap();
    let vy = dev.ctx.allocate_transient(n_out * 4, 4, 0).unwrap();
    let dims = [
        n_in as u32,
        n_out as u32,
        (n_in / 32 * 34) as u32,
        eps.to_bits(),
    ];
    let vd = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 5, 0)
        .unwrap();
    let pipeline = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        q8_rmsnorm_swiglu_source(),
        Q8_0_RMSNORM_SWIGLU_ENTRY,
        &[0, 1, 2, 3, 4, 5],
    )
    .expect("Q8 fused SwiGLU compile");
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    pipeline
        .dispatch(
            &[vx, vn, vg, vu, vy, vd],
            &schedule,
            n_out.div_ceil(8) * 256,
        )
        .expect("Q8 fused SwiGLU dispatch");
    let actual = dev.ctx.read_buffer_f32(&vy).unwrap();
    set_inference_mode(InferenceMode::Portable);
    for (index, (reference, observed)) in expected.iter().zip(actual).enumerate() {
        let tolerance = 4.0e-3 * reference.abs().max(1.0);
        assert!(
            (reference - observed).abs() <= tolerance,
            "ffn[{index}] oracle={reference} cuda={observed} tolerance={tolerance}"
        );
    }
}

fn rope_in_place(values: &mut [f32], n_heads: usize, head_dim: usize, pos: u32, base: f32) {
    for head in 0..n_heads {
        for pair in 0..head_dim / 2 {
            let offset = head * head_dim + pair * 2;
            let theta = pos as f32 * base.powf(-2.0 * pair as f32 / head_dim as f32);
            let (sin, cos) = theta.sin_cos();
            let a = values[offset];
            let b = values[offset + 1];
            values[offset] = a * cos - b * sin;
            values[offset + 1] = a * sin + b * cos;
        }
    }
}

#[test]
#[serial_test::serial]
fn fused_qkv_rope_matches_composed_oracle_when_cuda_available() {
    let n_in = 64usize;
    let n_q = 16usize;
    let n_kv = 8usize;
    let head_dim = 8usize;
    let position = 3u32;
    let rope_base = 10_000.0f32;
    let eps = 1.0e-5f32;
    let x: Vec<f32> = (0..n_in)
        .map(|index| 0.2 + (index as f32 * 0.13).sin())
        .collect();
    let norm: Vec<f32> = (0..n_in)
        .map(|index| 0.8 + (index % 9) as f32 * 0.025)
        .collect();
    let mean_sq = x.iter().map(|value| value * value).sum::<f32>() / n_in as f32;
    let inv_rms = (mean_sq + eps).sqrt().recip();
    let normalized: Vec<f32> = x
        .iter()
        .zip(&norm)
        .map(|(value, weight)| value * inv_rms * weight)
        .collect();
    let wq = q8_matrix(n_in, n_q, 1);
    let wk = q8_matrix(n_in, n_kv, 2);
    let wv = q8_matrix(n_in, n_kv, 3);
    let mut expected_q = vec![0.0f32; n_q];
    let mut expected_k = vec![0.0f32; n_kv];
    let mut expected_v = vec![0.0f32; n_kv];
    assert!(q8_0_gemv_oracle_into(
        n_in,
        n_q,
        &normalized,
        &wq,
        &mut expected_q
    ));
    assert!(q8_0_gemv_oracle_into(
        n_in,
        n_kv,
        &normalized,
        &wk,
        &mut expected_k
    ));
    assert!(q8_0_gemv_oracle_into(
        n_in,
        n_kv,
        &normalized,
        &wv,
        &mut expected_v
    ));
    rope_in_place(
        &mut expected_q,
        n_q / head_dim,
        head_dim,
        position,
        rope_base,
    );
    rope_in_place(
        &mut expected_k,
        n_kv / head_dim,
        head_dim,
        position,
        rope_base,
    );

    set_inference_mode(InferenceMode::CudaTc);
    let Ok(mut guard) = multi_weight_device().lock() else {
        return;
    };
    if !ensure_device(&mut guard) {
        set_inference_mode(InferenceMode::Portable);
        return;
    }
    let dev = guard.as_mut().unwrap();
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let mut alloc = |bytes: &[u8], binding| {
        dev.ctx
            .allocate_and_write(bytes, binding, 0)
            .expect("CUDA test allocation")
    };
    let vx = alloc(bytemuck::cast_slice(&x), 0);
    let vn = alloc(bytemuck::cast_slice(&norm), 1);
    let vq = alloc(&wq, 2);
    let vk = alloc(&wk, 3);
    let vv = alloc(&wv, 4);
    let yq = alloc(bytemuck::cast_slice(&vec![0.0f32; n_q]), 5);
    let yk = alloc(bytemuck::cast_slice(&vec![0.0f32; n_kv]), 6);
    let yv = alloc(bytemuck::cast_slice(&vec![0.0f32; n_kv]), 7);
    let dims = [
        n_in as u32,
        n_q as u32,
        n_kv as u32,
        (n_in / 32 * 34) as u32,
        eps.to_bits(),
    ];
    let vd = alloc(bytemuck::cast_slice(&dims), 8);
    let rope_q = [
        (n_q / head_dim) as u32,
        head_dim as u32,
        position,
        rope_base.to_bits(),
        1.0f32.to_bits(),
    ];
    let rope_k = [
        (n_kv / head_dim) as u32,
        head_dim as u32,
        position,
        rope_base.to_bits(),
        1.0f32.to_bits(),
    ];
    let vrq = alloc(bytemuck::cast_slice(&rope_q), 9);
    let vrk = alloc(bytemuck::cast_slice(&rope_k), 10);
    let step = [position, position];
    let vstep = alloc(bytemuck::cast_slice(&step), 11);
    let pipeline = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        q8_rmsnorm_qkv_rope_source(),
        Q8_0_RMSNORM_QKV_ROPE_ENTRY,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    )
    .expect("Q8 fused QKV compile");
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    pipeline
        .dispatch(
            &[vx, vn, vq, vk, vv, yq, yk, yv, vd, vrq, vrk, vstep],
            &schedule,
            n_q.div_ceil(8) * 256,
        )
        .expect("Q8 fused QKV dispatch");
    let actual_q = dev.ctx.read_buffer_f32(&yq).unwrap();
    let actual_k = dev.ctx.read_buffer_f32(&yk).unwrap();
    let actual_v = dev.ctx.read_buffer_f32(&yv).unwrap();
    set_inference_mode(InferenceMode::Portable);
    for (label, expected, actual) in [
        ("q", expected_q.as_slice(), actual_q.as_slice()),
        ("k", expected_k.as_slice(), actual_k.as_slice()),
        ("v", expected_v.as_slice(), actual_v.as_slice()),
    ] {
        for (index, (reference, observed)) in expected.iter().zip(actual).enumerate() {
            let tolerance = 3.0e-3 * reference.abs().max(1.0);
            assert!(
                (reference - observed).abs() <= tolerance,
                "{label}[{index}] oracle={reference} cuda={observed} tolerance={tolerance}"
            );
        }
    }
}
