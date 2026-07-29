use super::{
    q8_0_gemv_oracle_into, q8_dp4a_qkv_rope_warp8_source, q8_qkv_rope_source, Q8_0_BLOCK_BYTES,
    Q8_0_BLOCK_ELEMS, Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY, Q8_0_QKV_ROPE_ENTRY,
    Q8_ACTIVATION_QUANT_ENTRY, Q8_ACTIVATION_QUANT_SRC,
};
use crate::inference::cuda_lane::device::{ensure_device, multi_weight_device, MultiWeightDevice};
use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::wgsl_forge::execute::CudaPipeline;

fn q8_matrix(n_in: usize, n_out: usize, seed: usize) -> Vec<u8> {
    let row_bytes = n_in / Q8_0_BLOCK_ELEMS * Q8_0_BLOCK_BYTES;
    let mut raw = vec![0u8; row_bytes * n_out];
    for row in 0..n_out {
        for block in 0..n_in / Q8_0_BLOCK_ELEMS {
            let base = row * row_bytes + block * Q8_0_BLOCK_BYTES;
            let scale = half::f16::from_f32(0.0025 + ((row + seed) % 19) as f32 * 0.00013);
            raw[base..base + 2].copy_from_slice(&scale.to_bits().to_le_bytes());
            for lane in 0..Q8_0_BLOCK_ELEMS {
                raw[base + 2 + lane] =
                    (((row * 23 + block * 11 + lane * 5 + seed) % 251) as i16 - 125) as i8 as u8;
            }
        }
    }
    raw
}

fn rope(values: &mut [f32], head_dim: usize, position: u32, base: f32, scale: f32) {
    for pair in (0..values.len()).step_by(2) {
        let d = pair % head_dim;
        let theta = position as f32 / scale * base.powf(-2.0 * (d / 2) as f32 / head_dim as f32);
        let (sine, cosine) = theta.sin_cos();
        let even = values[pair];
        let odd = values[pair + 1];
        values[pair] = even * cosine - odd * sine;
        values[pair + 1] = even * sine + odd * cosine;
    }
}

fn allocate(
    device: &mut MultiWeightDevice,
    bytes: &[u8],
    binding: u32,
) -> crate::wgsl_forge::execute::memory::BufferView {
    device.ctx.allocate_and_write(bytes, binding, 0).unwrap()
}

fn median(samples: &mut [f32]) -> f32 {
    samples.sort_by(f32::total_cmp);
    samples[samples.len() / 2]
}

#[test]
#[serial_test::serial]
fn paired_dp4a_qkv_rope_matches_oracle_and_beats_incumbent_on_device() {
    const N_IN: usize = 960;
    const N_Q: usize = 960;
    const N_KV: usize = 320;
    const HEAD_DIM: usize = 64;
    const POSITION: u32 = 37;
    const ROPE_BASE: f32 = 10_000.0;
    let x: Vec<f32> = (0..N_IN)
        .map(|index| ((index as f32 * 0.031).sin() * 1.3) + (index as f32 * 0.019).cos())
        .collect();
    let wq = q8_matrix(N_IN, N_Q, 7);
    let wk = q8_matrix(N_IN, N_KV, 17);
    let wv = q8_matrix(N_IN, N_KV, 31);
    let row_bytes = N_IN / Q8_0_BLOCK_ELEMS * Q8_0_BLOCK_BYTES;
    let dims = [N_IN as u32, N_Q as u32, N_KV as u32, row_bytes as u32];
    let rope_q = [
        N_Q as u32,
        HEAD_DIM as u32,
        0,
        ROPE_BASE.to_bits(),
        1.0f32.to_bits(),
    ];
    let rope_k = [
        N_KV as u32,
        HEAD_DIM as u32,
        0,
        ROPE_BASE.to_bits(),
        1.0f32.to_bits(),
    ];
    let step = [POSITION, POSITION];
    let mut expected_q = vec![0.0; N_Q];
    let mut expected_k = vec![0.0; N_KV];
    let mut expected_v = vec![0.0; N_KV];
    assert!(q8_0_gemv_oracle_into(N_IN, N_Q, &x, &wq, &mut expected_q));
    assert!(q8_0_gemv_oracle_into(N_IN, N_KV, &x, &wk, &mut expected_k));
    assert!(q8_0_gemv_oracle_into(N_IN, N_KV, &x, &wv, &mut expected_v));
    rope(&mut expected_q, HEAD_DIM, POSITION, ROPE_BASE, 1.0);
    rope(&mut expected_k, HEAD_DIM, POSITION, ROPE_BASE, 1.0);

    set_inference_mode(InferenceMode::CudaTc);
    let Ok(mut guard) = multi_weight_device().lock() else {
        set_inference_mode(InferenceMode::Portable);
        return;
    };
    if !ensure_device(&mut guard) {
        set_inference_mode(InferenceMode::Portable);
        return;
    }
    let device = guard.as_mut().unwrap();
    device.ctx.restore_checkpoint(device.permanent_end);
    let x_view = allocate(device, bytemuck::cast_slice(&x), 0);
    let wq_view = allocate(device, &wq, 1);
    let wk_view = allocate(device, &wk, 2);
    let wv_view = allocate(device, &wv, 3);
    let incumbent_q = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_Q]), 4);
    let incumbent_k = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_KV]), 5);
    let incumbent_v = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_KV]), 6);
    let candidate_q = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_Q]), 7);
    let candidate_k = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_KV]), 8);
    let candidate_v = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_KV]), 9);
    let quantized = allocate(device, &vec![0u8; N_IN], 10);
    let scales = allocate(
        device,
        bytemuck::cast_slice(&vec![0.0f32; N_IN / Q8_0_BLOCK_ELEMS]),
        11,
    );
    let dims_view = allocate(device, bytemuck::cast_slice(&dims), 12);
    let rope_q_view = allocate(device, bytemuck::cast_slice(&rope_q), 13);
    let rope_k_view = allocate(device, bytemuck::cast_slice(&rope_k), 14);
    let step_view = allocate(device, bytemuck::cast_slice(&step), 15);

    let incumbent = CudaPipeline::compile_cuda_c_source_cached(
        &device.ctx,
        q8_qkv_rope_source(),
        Q8_0_QKV_ROPE_ENTRY,
        &(0..11).collect::<Vec<_>>(),
    )
    .unwrap();
    let quantizer = CudaPipeline::compile_cuda_c_source_cached(
        &device.ctx,
        Q8_ACTIVATION_QUANT_SRC,
        Q8_ACTIVATION_QUANT_ENTRY,
        &[0, 1, 2, 3],
    )
    .unwrap();
    let candidate = CudaPipeline::compile_cuda_c_source_cached(
        &device.ctx,
        q8_dp4a_qkv_rope_warp8_source(),
        Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY,
        &(0..12).collect::<Vec<_>>(),
    )
    .unwrap();
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let mut incumbent_bindings = [
        x_view,
        wq_view,
        wk_view,
        wv_view,
        incumbent_q,
        incumbent_k,
        incumbent_v,
        dims_view,
        rope_q_view,
        rope_k_view,
        step_view,
    ];
    let mut quant_bindings = [x_view, quantized, scales, dims_view];
    let mut candidate_bindings = [
        quantized,
        scales,
        wq_view,
        wk_view,
        wv_view,
        candidate_q,
        candidate_k,
        candidate_v,
        dims_view,
        rope_q_view,
        rope_k_view,
        step_view,
    ];
    for views in [
        incumbent_bindings.as_mut_slice(),
        quant_bindings.as_mut_slice(),
        candidate_bindings.as_mut_slice(),
    ] {
        for (binding, view) in views.iter_mut().enumerate() {
            view.binding = binding as u32;
        }
    }
    let incumbent_elements = N_Q.div_ceil(8) * 256;
    let quant_elements = (N_IN / Q8_0_BLOCK_ELEMS).div_ceil(8) * 256;
    let candidate_elements = N_Q.div_ceil(8) * 256;
    incumbent
        .dispatch_async_sorted(&incumbent_bindings, &schedule, incumbent_elements)
        .unwrap();
    quantizer
        .dispatch_async_sorted(&quant_bindings, &schedule, quant_elements)
        .unwrap();
    candidate
        .dispatch_async_sorted(&candidate_bindings, &schedule, candidate_elements)
        .unwrap();
    device.ctx.stream.synchronize().unwrap();

    let incumbent_values = [
        device.ctx.read_buffer_f32(&incumbent_q).unwrap(),
        device.ctx.read_buffer_f32(&incumbent_k).unwrap(),
        device.ctx.read_buffer_f32(&incumbent_v).unwrap(),
    ];
    let candidate_values = [
        device.ctx.read_buffer_f32(&candidate_q).unwrap(),
        device.ctx.read_buffer_f32(&candidate_k).unwrap(),
        device.ctx.read_buffer_f32(&candidate_v).unwrap(),
    ];
    let expected = [&expected_q, &expected_k, &expected_v];
    let mut candidate_max_error = 0.0f32;
    let mut reference_max = 0.0f32;
    for tensor in 0..3 {
        for index in 0..expected[tensor].len() {
            let reference = expected[tensor][index];
            let incumbent_error = (incumbent_values[tensor][index] - reference).abs();
            assert!(incumbent_error <= 4.0e-3 * reference.abs().max(1.0));
            candidate_max_error =
                candidate_max_error.max((candidate_values[tensor][index] - reference).abs());
            reference_max = reference_max.max(reference.abs());
        }
    }
    let normalized_max_error = candidate_max_error / reference_max.max(1.0);
    assert!(normalized_max_error <= 0.02);

    let mut incumbent_ms = [0.0f32; 32];
    let mut quantizer_ms = [0.0f32; 32];
    let mut candidate_ms = [0.0f32; 32];
    for sample in &mut incumbent_ms {
        *sample = incumbent
            .dispatch_gpu_timed_ms_sorted(&incumbent_bindings, &schedule, incumbent_elements)
            .unwrap();
    }
    for sample in &mut quantizer_ms {
        *sample = quantizer
            .dispatch_gpu_timed_ms_sorted(&quant_bindings, &schedule, quant_elements)
            .unwrap();
    }
    for sample in &mut candidate_ms {
        *sample = candidate
            .dispatch_gpu_timed_ms_sorted(&candidate_bindings, &schedule, candidate_elements)
            .unwrap();
    }
    let incumbent_median = median(&mut incumbent_ms);
    let quantizer_median = median(&mut quantizer_ms);
    let candidate_median = median(&mut candidate_ms);
    let combined = quantizer_median + candidate_median;
    eprintln!(
        "q8_dp4a_qkv_rope incumbent_us={:.3} quantizer_us={:.3} candidate_us={:.3} \
         combined_us={:.3} speedup={:.3} normalized_max_error={normalized_max_error:.6}",
        incumbent_median * 1_000.0,
        quantizer_median * 1_000.0,
        candidate_median * 1_000.0,
        combined * 1_000.0,
        incumbent_median / combined
    );
    assert!(
        combined < incumbent_median,
        "candidate must beat incumbent before production integration"
    );
    device.ctx.restore_checkpoint(device.permanent_end);
    set_inference_mode(InferenceMode::Portable);
}
