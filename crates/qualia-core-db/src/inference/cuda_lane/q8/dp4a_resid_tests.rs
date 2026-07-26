use super::{
    q8_0_gemv_oracle_into, q8_gemv_resid_source, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_ELEMS,
    Q8_0_DP4A_GEMV_RESID_ENTRY, Q8_0_DP4A_GEMV_RESID_SRC, Q8_0_GEMV_RESID_ENTRY,
    Q8_ACTIVATION_QUANT_ENTRY, Q8_ACTIVATION_QUANT_SRC,
};
use crate::inference::cuda_lane::device::{ensure_device, multi_weight_device, MultiWeightDevice};
use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::wgsl_forge::execute::CudaPipeline;

fn q8_matrix(n_in: usize, n_out: usize) -> Vec<u8> {
    let row_bytes = n_in / Q8_0_BLOCK_ELEMS * Q8_0_BLOCK_BYTES;
    let mut raw = vec![0u8; row_bytes * n_out];
    for row in 0..n_out {
        for block in 0..n_in / Q8_0_BLOCK_ELEMS {
            let base = row * row_bytes + block * Q8_0_BLOCK_BYTES;
            let scale = half::f16::from_f32(0.0028 + (row % 23) as f32 * 0.00011);
            raw[base..base + 2].copy_from_slice(&scale.to_bits().to_le_bytes());
            for lane in 0..Q8_0_BLOCK_ELEMS {
                raw[base + 2 + lane] =
                    (((row * 29 + block * 7 + lane * 17) % 249) as i16 - 124) as i8 as u8;
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

fn median(samples: &mut [f32]) -> f32 {
    samples.sort_by(f32::total_cmp);
    samples[samples.len() / 2]
}

#[test]
#[serial_test::serial]
fn dp4a_down_projection_residual_matches_oracle_and_beats_incumbent() {
    const N_IN: usize = 2560;
    const N_OUT: usize = 960;
    let x: Vec<f32> = (0..N_IN)
        .map(|index| (index as f32 * 0.023).sin() * 1.6 + (index as f32 * 0.011).cos() * 0.4)
        .collect();
    let residual: Vec<f32> = (0..N_OUT)
        .map(|index| (index as f32 * 0.037).cos() * 2.0)
        .collect();
    let weights = q8_matrix(N_IN, N_OUT);
    let row_bytes = N_IN / Q8_0_BLOCK_ELEMS * Q8_0_BLOCK_BYTES;
    let dims = [N_IN as u32, N_OUT as u32, row_bytes as u32];
    let mut expected = vec![0.0f32; N_OUT];
    assert!(q8_0_gemv_oracle_into(
        N_IN,
        N_OUT,
        &x,
        &weights,
        &mut expected
    ));
    for (value, residual) in expected.iter_mut().zip(&residual) {
        *value += residual;
    }

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
    let weights_view = allocate(device, &weights, 1);
    let incumbent_out = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_OUT]), 2);
    let candidate_out = allocate(device, bytemuck::cast_slice(&vec![0.0f32; N_OUT]), 3);
    let dims_view = allocate(device, bytemuck::cast_slice(&dims), 4);
    let residual_view = allocate(device, bytemuck::cast_slice(&residual), 5);
    let quantized = allocate(device, &vec![0u8; N_IN], 6);
    let scales = allocate(
        device,
        bytemuck::cast_slice(&vec![0.0f32; N_IN / Q8_0_BLOCK_ELEMS]),
        7,
    );
    let incumbent = CudaPipeline::compile_cuda_c_source_cached(
        &device.ctx,
        q8_gemv_resid_source(),
        Q8_0_GEMV_RESID_ENTRY,
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
        Q8_0_DP4A_GEMV_RESID_SRC,
        Q8_0_DP4A_GEMV_RESID_ENTRY,
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
    let mut incumbent_bindings = [
        x_view,
        weights_view,
        incumbent_out,
        dims_view,
        residual_view,
    ];
    let mut quant_bindings = [x_view, quantized, scales, dims_view];
    let mut candidate_bindings = [
        quantized,
        scales,
        weights_view,
        candidate_out,
        dims_view,
        residual_view,
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
    let reference_max = expected
        .iter()
        .fold(0.0f32, |maximum, value| maximum.max(value.abs()));
    let mut candidate_max_error = 0.0f32;
    for (index, ((reference, incumbent), candidate)) in expected
        .iter()
        .zip(&incumbent_values)
        .zip(&candidate_values)
        .enumerate()
    {
        assert!(
            (incumbent - reference).abs() <= 4.0e-3 * reference.abs().max(1.0),
            "incumbent[{index}]={incumbent} oracle={reference}"
        );
        candidate_max_error = candidate_max_error.max((candidate - reference).abs());
    }
    let normalized_max_error = candidate_max_error / reference_max.max(1.0);
    assert!(normalized_max_error <= 0.02);

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
    let incumbent_median = median(&mut incumbent_ms);
    let quantizer_median = median(&mut quantizer_ms);
    let candidate_median = median(&mut candidate_ms);
    let combined = quantizer_median + candidate_median;
    eprintln!(
        "q8_dp4a_down_resid incumbent_us={:.3} quantizer_us={:.3} candidate_us={:.3} \
         combined_us={:.3} speedup={:.3} normalized_max_error={normalized_max_error:.6}",
        incumbent_median * 1_000.0,
        quantizer_median * 1_000.0,
        candidate_median * 1_000.0,
        combined * 1_000.0,
        incumbent_median / combined
    );
    assert!(combined < incumbent_median);
    device.ctx.restore_checkpoint(device.permanent_end);
    set_inference_mode(InferenceMode::Portable);
}
