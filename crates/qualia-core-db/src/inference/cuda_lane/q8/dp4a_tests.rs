use super::{
    q8_0_gemv_oracle_into, Q8_0_BLOCK_BYTES, Q8_0_BLOCK_ELEMS, Q8_0_DP4A_GEMV_ENTRY,
    Q8_0_DP4A_GEMV_SRC, Q8_0_GEMV_ENTRY, Q8_0_GEMV_SRC, Q8_ACTIVATION_QUANT_ENTRY,
    Q8_ACTIVATION_QUANT_SRC,
};
use crate::inference::cuda_lane::device::{ensure_device, multi_weight_device};
use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::wgsl_forge::execute::CudaPipeline;

fn q8_matrix(n_in: usize, n_out: usize) -> Vec<u8> {
    let row_bytes = n_in / Q8_0_BLOCK_ELEMS * Q8_0_BLOCK_BYTES;
    let mut raw = vec![0u8; row_bytes * n_out];
    for row in 0..n_out {
        for block in 0..n_in / Q8_0_BLOCK_ELEMS {
            let base = row * row_bytes + block * Q8_0_BLOCK_BYTES;
            let scale = half::f16::from_f32(0.0035 + (row % 13) as f32 * 0.00019);
            raw[base..base + 2].copy_from_slice(&scale.to_bits().to_le_bytes());
            for lane in 0..Q8_0_BLOCK_ELEMS {
                raw[base + 2 + lane] =
                    (((row * 17 + block * 11 + lane * 5 + 23) % 241) as i16 - 120) as i8 as u8;
            }
        }
    }
    raw
}

#[test]
#[serial_test::serial]
fn two_stage_dp4a_candidate_matches_oracle_and_reports_device_time_when_cuda_available() {
    const N_IN: usize = 960;
    const N_OUT: usize = 2560;
    let x: Vec<f32> = (0..N_IN)
        .map(|index| ((index as f32 * 0.071).sin() * 1.7) + ((index as f32 * 0.013).cos() * 0.4))
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

    set_inference_mode(InferenceMode::CudaTc);
    let Ok(mut guard) = multi_weight_device().lock() else {
        set_inference_mode(InferenceMode::Portable);
        return;
    };
    if !ensure_device(&mut guard) {
        set_inference_mode(InferenceMode::Portable);
        return;
    }
    let dev = guard.as_mut().unwrap();
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let x_view = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&x), 0, 0)
        .unwrap();
    let weight_view = dev.ctx.allocate_and_write(&weights, 1, 0).unwrap();
    let float_out = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&vec![0.0f32; N_OUT]), 2, 0)
        .unwrap();
    let dp4a_out = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&vec![0.0f32; N_OUT]), 2, 0)
        .unwrap();
    let quantized_view = dev.ctx.allocate_and_write(&vec![0u8; N_IN], 1, 0).unwrap();
    let scale_view = dev
        .ctx
        .allocate_and_write(
            bytemuck::cast_slice(&vec![0.0f32; N_IN / Q8_0_BLOCK_ELEMS]),
            2,
            0,
        )
        .unwrap();
    let dims_view = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)
        .unwrap();

    let incumbent = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q8_0_GEMV_SRC,
        Q8_0_GEMV_ENTRY,
        &[0, 1, 2, 3],
    )
    .unwrap();
    let quantizer = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q8_ACTIVATION_QUANT_SRC,
        Q8_ACTIVATION_QUANT_ENTRY,
        &[0, 1, 2, 3],
    )
    .unwrap();
    let candidate = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q8_0_DP4A_GEMV_SRC,
        Q8_0_DP4A_GEMV_ENTRY,
        &[0, 1, 2, 3, 4],
    )
    .unwrap();
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let candidate_schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 128,
        ..Default::default()
    };
    let elements = N_OUT.div_ceil(8) * 256;
    let candidate_elements = N_OUT * 128;
    let quant_elements = (N_IN / Q8_0_BLOCK_ELEMS).div_ceil(8) * 256;
    let mut incumbent_bindings = [x_view, weight_view, float_out, dims_view];
    for (binding, view) in incumbent_bindings.iter_mut().enumerate() {
        view.binding = binding as u32;
    }
    let mut quant_bindings = [x_view, quantized_view, scale_view, dims_view];
    for (binding, view) in quant_bindings.iter_mut().enumerate() {
        view.binding = binding as u32;
    }
    let mut candidate_bindings = [quantized_view, scale_view, weight_view, dp4a_out, dims_view];
    for (binding, view) in candidate_bindings.iter_mut().enumerate() {
        view.binding = binding as u32;
    }
    incumbent
        .dispatch_async_sorted(&incumbent_bindings, &schedule, elements)
        .unwrap();
    quantizer
        .dispatch_async_sorted(&quant_bindings, &schedule, quant_elements)
        .unwrap();
    candidate
        .dispatch_async_sorted(&candidate_bindings, &candidate_schedule, candidate_elements)
        .unwrap();
    dev.ctx.stream.synchronize().unwrap();

    let float_values = dev.ctx.read_buffer_f32(&float_out).unwrap();
    let dp4a_values = dev.ctx.read_buffer_f32(&dp4a_out).unwrap();
    let mut max_abs_error = 0.0f32;
    let mut max_relative_error = 0.0f32;
    let mut max_reference = 0.0f32;
    let mut squared_error = 0.0f64;
    for (index, ((reference, float_value), dp4a_value)) in expected
        .iter()
        .zip(&float_values)
        .zip(&dp4a_values)
        .enumerate()
    {
        let float_tolerance = 2.0e-3 * reference.abs().max(1.0);
        assert!(
            (float_value - reference).abs() <= float_tolerance,
            "incumbent[{index}] cuda={float_value} oracle={reference}"
        );
        let abs_error = (dp4a_value - reference).abs();
        max_abs_error = max_abs_error.max(abs_error);
        max_reference = max_reference.max(reference.abs());
        squared_error += f64::from(abs_error) * f64::from(abs_error);
        max_relative_error = max_relative_error.max(abs_error / reference.abs().max(1.0));
    }
    let normalized_max_error = max_abs_error / max_reference.max(1.0);
    let root_mean_square_error = (squared_error / N_OUT as f64).sqrt();
    assert!(
        normalized_max_error <= 0.01,
        "dp4a normalized maximum error {normalized_max_error} (abs {max_abs_error}, \
         reference max {max_reference})"
    );

    for _ in 0..8 {
        incumbent
            .dispatch_async_sorted(&incumbent_bindings, &schedule, elements)
            .unwrap();
        quantizer
            .dispatch_async_sorted(&quant_bindings, &schedule, quant_elements)
            .unwrap();
        candidate
            .dispatch_async_sorted(&candidate_bindings, &candidate_schedule, candidate_elements)
            .unwrap();
    }
    dev.ctx.stream.synchronize().unwrap();
    let mut incumbent_ms = [0.0f32; 32];
    let mut quantizer_ms = [0.0f32; 32];
    let mut candidate_ms = [0.0f32; 32];
    for sample in &mut incumbent_ms {
        *sample = incumbent
            .dispatch_gpu_timed_ms_sorted(&incumbent_bindings, &schedule, elements)
            .unwrap();
    }
    for sample in &mut quantizer_ms {
        *sample = quantizer
            .dispatch_gpu_timed_ms_sorted(&quant_bindings, &schedule, quant_elements)
            .unwrap();
    }
    for sample in &mut candidate_ms {
        *sample = candidate
            .dispatch_gpu_timed_ms_sorted(
                &candidate_bindings,
                &candidate_schedule,
                candidate_elements,
            )
            .unwrap();
    }
    incumbent_ms.sort_by(f32::total_cmp);
    quantizer_ms.sort_by(f32::total_cmp);
    candidate_ms.sort_by(f32::total_cmp);
    let combined_ms = quantizer_ms[16] + candidate_ms[16];
    eprintln!(
        "q8_dp4a_960x2560 incumbent_us={:.3} quantizer_us={:.3} dp4a_us={:.3} \
         combined_us={:.3} projection_speedup={:.3} combined_speedup={:.3} \
         max_scaled_rel_error={:.6} normalized_max_error={:.6} rmse={:.6}",
        incumbent_ms[16] * 1_000.0,
        quantizer_ms[16] * 1_000.0,
        candidate_ms[16] * 1_000.0,
        combined_ms * 1_000.0,
        incumbent_ms[16] / candidate_ms[16],
        incumbent_ms[16] / combined_ms,
        max_relative_error,
        normalized_max_error,
        root_mean_square_error
    );
    set_inference_mode(InferenceMode::Portable);
}
