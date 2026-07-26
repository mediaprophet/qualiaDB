use super::{
    q8_0_gemv_oracle_into, q8_dp4a_swiglu_source, q8_swiglu_source, Q8_0_BLOCK_BYTES,
    Q8_0_BLOCK_ELEMS, Q8_0_DP4A_SWIGLU_ENTRY, Q8_0_SWIGLU_ENTRY, Q8_ACTIVATION_QUANT_ENTRY,
    Q8_ACTIVATION_QUANT_SRC,
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
            let scale = half::f16::from_f32(0.003 + ((row + seed) % 17) as f32 * 0.00017);
            raw[base..base + 2].copy_from_slice(&scale.to_bits().to_le_bytes());
            for lane in 0..Q8_0_BLOCK_ELEMS {
                raw[base + 2 + lane] =
                    (((row * 19 + block * 13 + lane * 7 + seed) % 241) as i16 - 120) as i8 as u8;
            }
        }
    }
    raw
}

fn allocate(
    device: &mut MultiWeightDevice,
    bytes: &[u8],
    binding: u32,
) -> crate::wgsl_forge::execute::memory::BufferView {
    device.ctx.allocate_and_write(bytes, binding, 0).unwrap()
}

#[test]
#[serial_test::serial]
fn two_stage_dp4a_swiglu_matches_oracle_and_reports_device_time_when_cuda_available() {
    const N_IN: usize = 960;
    const N_OUT: usize = 2560;
    let x: Vec<f32> = (0..N_IN)
        .map(|index| ((index as f32 * 0.043).sin() * 1.4) + ((index as f32 * 0.017).cos() * 0.6))
        .collect();
    let gate = q8_matrix(N_IN, N_OUT, 29);
    let up = q8_matrix(N_IN, N_OUT, 47);
    let row_bytes = N_IN / Q8_0_BLOCK_ELEMS * Q8_0_BLOCK_BYTES;
    let dims = [N_IN as u32, N_OUT as u32, row_bytes as u32];
    let mut expected_gate = vec![0.0f32; N_OUT];
    let mut expected_up = vec![0.0f32; N_OUT];
    assert!(q8_0_gemv_oracle_into(
        N_IN,
        N_OUT,
        &x,
        &gate,
        &mut expected_gate
    ));
    assert!(q8_0_gemv_oracle_into(
        N_IN,
        N_OUT,
        &x,
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
    let gate_view = allocate(device, &gate, 1);
    let up_view = allocate(device, &up, 2);
    let incumbent_out = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_OUT]), 3);
    let candidate_out = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_OUT]), 4);
    let quantized_view = allocate(device, &vec![0u8; N_IN], 5);
    let scale_view = allocate(
        device,
        bytemuck::cast_slice(&vec![0.0f32; N_IN / Q8_0_BLOCK_ELEMS]),
        6,
    );
    let dims_view = allocate(device, bytemuck::cast_slice(&dims), 7);

    let incumbent = CudaPipeline::compile_cuda_c_source_cached(
        &device.ctx,
        q8_swiglu_source(),
        Q8_0_SWIGLU_ENTRY,
        &[0, 1, 2, 3, 4],
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
        q8_dp4a_swiglu_source(),
        Q8_0_DP4A_SWIGLU_ENTRY,
        &[0, 1, 2, 3, 4, 5],
    )
    .unwrap();
    let schedule256 = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let schedule128 = crate::wgsl_forge::Schedule {
        workgroup_size: 128,
        ..Default::default()
    };
    let mut incumbent_bindings = [x_view, gate_view, up_view, incumbent_out, dims_view];
    let mut quant_bindings = [x_view, quantized_view, scale_view, dims_view];
    let mut candidate_bindings = [
        quantized_view,
        scale_view,
        gate_view,
        up_view,
        candidate_out,
        dims_view,
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
    let incumbent_elements = N_OUT.div_ceil(8) * 256;
    let quant_elements = (N_IN / Q8_0_BLOCK_ELEMS).div_ceil(8) * 256;
    let candidate_elements = N_OUT * 128;
    incumbent
        .dispatch_async_sorted(&incumbent_bindings, &schedule256, incumbent_elements)
        .unwrap();
    quantizer
        .dispatch_async_sorted(&quant_bindings, &schedule256, quant_elements)
        .unwrap();
    candidate
        .dispatch_async_sorted(&candidate_bindings, &schedule128, candidate_elements)
        .unwrap();
    device.ctx.stream.synchronize().unwrap();

    let incumbent_values = device.ctx.read_buffer_f32(&incumbent_out).unwrap();
    let candidate_values = device.ctx.read_buffer_f32(&candidate_out).unwrap();
    let mut candidate_max_error = 0.0f32;
    let max_reference = expected
        .iter()
        .fold(0.0f32, |maximum, value| maximum.max(value.abs()));
    for (index, ((reference, incumbent), candidate)) in expected
        .iter()
        .zip(&incumbent_values)
        .zip(&candidate_values)
        .enumerate()
    {
        let incumbent_tolerance = 4.0e-3 * reference.abs().max(1.0);
        assert!(
            (incumbent - reference).abs() <= incumbent_tolerance,
            "incumbent[{index}]={incumbent} oracle={reference}"
        );
        candidate_max_error = candidate_max_error.max((candidate - reference).abs());
    }
    let normalized_max_error = candidate_max_error / max_reference.max(1.0);
    assert!(normalized_max_error <= 0.02);

    for _ in 0..8 {
        incumbent
            .dispatch_async_sorted(&incumbent_bindings, &schedule256, incumbent_elements)
            .unwrap();
        quantizer
            .dispatch_async_sorted(&quant_bindings, &schedule256, quant_elements)
            .unwrap();
        candidate
            .dispatch_async_sorted(&candidate_bindings, &schedule128, candidate_elements)
            .unwrap();
    }
    device.ctx.stream.synchronize().unwrap();
    let mut incumbent_ms = [0.0f32; 32];
    let mut quantizer_ms = [0.0f32; 32];
    let mut candidate_ms = [0.0f32; 32];
    for sample in &mut incumbent_ms {
        *sample = incumbent
            .dispatch_gpu_timed_ms_sorted(&incumbent_bindings, &schedule256, incumbent_elements)
            .unwrap();
    }
    for sample in &mut quantizer_ms {
        *sample = quantizer
            .dispatch_gpu_timed_ms_sorted(&quant_bindings, &schedule256, quant_elements)
            .unwrap();
    }
    for sample in &mut candidate_ms {
        *sample = candidate
            .dispatch_gpu_timed_ms_sorted(&candidate_bindings, &schedule128, candidate_elements)
            .unwrap();
    }
    incumbent_ms.sort_by(f32::total_cmp);
    quantizer_ms.sort_by(f32::total_cmp);
    candidate_ms.sort_by(f32::total_cmp);
    let combined_ms = quantizer_ms[16] + candidate_ms[16];
    eprintln!(
        "q8_dp4a_swiglu_960x2560 incumbent_us={:.3} quantizer_us={:.3} \
         candidate_us={:.3} combined_us={:.3} speedup={:.3} normalized_max_error={:.6}",
        incumbent_ms[16] * 1_000.0,
        quantizer_ms[16] * 1_000.0,
        candidate_ms[16] * 1_000.0,
        combined_ms * 1_000.0,
        incumbent_ms[16] / combined_ms,
        normalized_max_error
    );
    set_inference_mode(InferenceMode::Portable);
}
