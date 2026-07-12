//! wgpu-concrete per-kernel differential-oracle evaluators — the paths that need
//! backend-specific wiring (uniform param blocks, ray-query acceleration
//! structures) and so are not part of the generic [`OracleContext`] cross-backend
//! set: FFT, P64 projection, ternary GEMV, dense GEMM/GEMV, and ray-probe.

use crate::wgsl_forge::execute::{BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline};
use crate::wgsl_forge::{
    emit_shader, validate_wgsl, BuiltinKernel, ForgeError, GeneratedShader, Schedule,
    TargetBackend, TimingSource, TimingSummary,
};

use super::params::*;
use super::reference::*;
use super::report::*;

/// Differential-oracle evaluation of the radix-2 FFT (`out = forward DFT(in)`)
/// against [`dft_cpu`]. One workgroup of `n = schedule.workgroup_size` threads
/// (one complex element per thread; `n` must be a power of two), mirroring the
/// single-workgroup dispatch of [`evaluate_topk`]: `element_count = n` with
/// `workgroup_size = n` launches exactly one workgroup. The input/output buffers
/// hold `2*n` interleaved f32.
pub fn evaluate_fft(
    context: &mut WgpuComputeContext,
    schedule: Schedule,
    n: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    if n == 0 || !n.is_power_of_two() {
        return Err(ForgeError::GpuValidation(format!(
            "fft length n must be a power of two; got {n}"
        )));
    }
    if n != schedule.workgroup_size as usize {
        return Err(ForgeError::GpuValidation(format!(
            "fft requires schedule.workgroup_size == n ({n}); got {}",
            schedule.workgroup_size
        )));
    }
    let kernel = BuiltinKernel::Fft.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let vector_seed = 0x46_46_54_5F_53_45_44_00u64; // "FFT_SED\0" tag
    let input = fft_inputs(n, vector_seed);
    let expected = dft_cpu(&input, n);
    let log2n = n.trailing_zeros();

    let view_input = context.allocate_and_write(
        bytemuck::cast_slice(&input),
        0,
        0,
        BindingUsage::StorageRead,
    )?;
    let output_bytes_len = (2 * n * size_of::<f32>()).max(4);
    let view_output =
        context.allocate_transient(output_bytes_len, 1, 0, BindingUsage::StorageReadWrite)?;
    let params = FftParams {
        n: n as u32,
        log2n,
        _pad0: 0,
        _pad1: 0,
    };
    let view_params =
        context.allocate_and_write(bytemuck::bytes_of(&params), 2, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_input, view_output, view_params];
    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    // One workgroup of n threads: element_count = n with workgroup_size = n.
    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, n)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, n)?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    // f32 FFT vs (f64-accumulated) DFT: for N<=1024 they agree to ~1e-3..1e-2.
    // The absolute tolerance carries near-zero bins (cancellation), the relative
    // one carries the O(1) bins.
    let tolerance = OracleTolerance {
        absolute: 1.0e-2,
        relative: 1.0e-2,
    };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "fft: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            oracle.max_relative_error
        )));
    }

    let timing_source = if context.timestamp_supported {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(vector_seed),
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}

/// Differential-oracle evaluation for the P64 projection against [`p64_project_cpu`].
pub fn evaluate_p64(
    context: &mut WgpuComputeContext,
    schedule: Schedule,
    count: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let kernel = BuiltinKernel::P64Project.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let vector_seed = 0x5036_345F_5345_4544u64;
    let records = p64_records(count, vector_seed);
    let weights = topk_inputs(16, 0x5036_345F_5742_5453);
    let expected = p64_project_cpu(&records, &weights);

    let view_input = context.allocate_and_write(
        bytemuck::cast_slice(&records),
        0,
        0,
        BindingUsage::StorageRead,
    )?;
    let view_weights = context.allocate_and_write(
        bytemuck::cast_slice(&weights),
        1,
        0,
        BindingUsage::StorageRead,
    )?;
    let output_bytes_len = (count * size_of::<f32>()).max(4);
    let view_output =
        context.allocate_transient(output_bytes_len, 2, 0, BindingUsage::StorageReadWrite)?;

    let buffers = vec![view_input, view_weights, view_output];
    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, count)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, count)?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    let tolerance = OracleTolerance {
        absolute: 1.0e-1,
        relative: 1.0e-4,
    };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "p64-project: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            oracle.max_relative_error
        )));
    }

    let timing_source = if context.timestamp_supported {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(vector_seed),
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}

/// Differential-oracle evaluation for the ternary GEMV against [`ternary_gemv_cpu`].
/// One workgroup-thread per output row; the output buffer is `m` elements.
pub fn evaluate_ternary_gemv(
    context: &mut WgpuComputeContext,
    schedule: Schedule,
    m: usize,
    k: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let kernel = BuiltinKernel::TernaryGemv.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let vector_seed = 0x5445_524E_5345_4544u64; // "TERNSED" tag
    let (x, w_packed, scale) = ternary_gemv_tensors(m, k, vector_seed);
    let expected = ternary_gemv_cpu(&x, &w_packed, &scale, m, k);
    let k_words = k.div_ceil(TERNARY_CODES_PER_WORD);

    let view_x =
        context.allocate_and_write(bytemuck::cast_slice(&x), 0, 0, BindingUsage::StorageRead)?;
    let view_w = context.allocate_and_write(
        bytemuck::cast_slice(&w_packed),
        1,
        0,
        BindingUsage::StorageRead,
    )?;
    let view_scale = context.allocate_and_write(
        bytemuck::cast_slice(&scale),
        2,
        0,
        BindingUsage::StorageRead,
    )?;
    let output_bytes_len = (m * size_of::<f32>()).max(4);
    let view_output =
        context.allocate_transient(output_bytes_len, 3, 0, BindingUsage::StorageReadWrite)?;
    let params = TernaryGemvParams {
        m: m as u32,
        k: k as u32,
        k_words: k_words as u32,
        _pad: 0,
    };
    let view_params =
        context.allocate_and_write(bytemuck::bytes_of(&params), 4, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_x, view_w, view_scale, view_output, view_params];
    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, m)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, m)?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    // The ternary path is exact arithmetic (±1/0 weights, no transcendentals); a
    // tight tolerance covers only f32 summation-order differences across the K sum.
    let tolerance = OracleTolerance {
        absolute: 1.0e-3,
        relative: 1.0e-3,
    };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "ternary-gemv: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            oracle.max_relative_error
        )));
    }

    let timing_source = if context.timestamp_supported {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(vector_seed),
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}

/// Differential-oracle evaluation for the dense GEMM against [`gemm_cpu`]. One
/// workgroup-thread per output element; the output buffer is `m * n` elements.
/// Row-major `C[M×N] = A[M×K] · B[K×N]`, all f32.
pub fn evaluate_gemm(
    context: &mut WgpuComputeContext,
    schedule: Schedule,
    m: usize,
    k: usize,
    n: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let kernel = BuiltinKernel::Gemm.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let vector_seed = 0x47_45_4D_4D_53_45_44_00u64; // "GEMMSED\0" tag
    let (a, b) = gemm_tensors(m, k, n, vector_seed);
    let expected = gemm_cpu(&a, &b, m, k, n);
    let element_count = m * n;

    let view_a =
        context.allocate_and_write(bytemuck::cast_slice(&a), 0, 0, BindingUsage::StorageRead)?;
    let view_b =
        context.allocate_and_write(bytemuck::cast_slice(&b), 1, 0, BindingUsage::StorageRead)?;
    let output_bytes_len = (element_count * size_of::<f32>()).max(4);
    let view_c =
        context.allocate_transient(output_bytes_len, 2, 0, BindingUsage::StorageReadWrite)?;
    let params = GemmParams {
        m: m as u32,
        n: n as u32,
        k: k as u32,
        _pad: 0,
    };
    let view_params =
        context.allocate_and_write(bytemuck::bytes_of(&params), 3, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_a, view_b, view_c, view_params];
    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, element_count)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, element_count)?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_c)?;
    // Dense f32 GEMM: only the length-K summation order differs between GPU and
    // CPU (no transcendentals), so a tight tolerance covers accumulation drift.
    let tolerance = OracleTolerance {
        absolute: 1.0e-3,
        relative: 1.0e-3,
    };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "gemm: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            oracle.max_relative_error
        )));
    }

    let timing_source = if context.timestamp_supported {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(vector_seed),
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}

/// Differential-oracle evaluation for the dense GEMV against [`gemv_cpu`]. One
/// workgroup-thread per output ROW; the output buffer is `m` elements. Row-major
/// `y[M] = A[M×N] · x[N]`, all f32.
pub fn evaluate_gemv(
    context: &mut WgpuComputeContext,
    schedule: Schedule,
    m: usize,
    n: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let kernel = BuiltinKernel::Gemv.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let vector_seed = 0x47_45_4D_56_53_45_44_00u64; // "GEMVSED\0" tag
    let (a, x) = gemv_tensors(m, n, vector_seed);
    let expected = gemv_cpu(&a, &x, m, n);
    let element_count = m;

    let view_a =
        context.allocate_and_write(bytemuck::cast_slice(&a), 0, 0, BindingUsage::StorageRead)?;
    let view_x =
        context.allocate_and_write(bytemuck::cast_slice(&x), 1, 0, BindingUsage::StorageRead)?;
    let output_bytes_len = (element_count * size_of::<f32>()).max(4);
    let view_y =
        context.allocate_transient(output_bytes_len, 2, 0, BindingUsage::StorageReadWrite)?;
    let params = GemvParams {
        m: m as u32,
        n: n as u32,
        _pad0: 0,
        _pad1: 0,
    };
    let view_params =
        context.allocate_and_write(bytemuck::bytes_of(&params), 3, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_a, view_x, view_y, view_params];
    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, element_count)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, element_count)?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_y)?;
    // Dense f32 GEMV: only the length-N summation order differs between GPU and CPU
    // (no transcendentals), so a tight tolerance covers accumulation drift.
    let tolerance = OracleTolerance {
        absolute: 1.0e-3,
        relative: 1.0e-3,
    };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "gemv: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            oracle.max_relative_error
        )));
    }

    let timing_source = if context.timestamp_supported {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(vector_seed),
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}

/// Differential-oracle evaluation of the ray-query (ray-probe) kernel: builds a
/// BLAS+TLAS for [`rayprobe_scene`], dispatches the emitted `ray_probe` WGSL over
/// [`rayprobe_rays`] on the GPU, and checks the committed hit distances against
/// [`rayprobe_cpu`]. Requires a ray-query-capable adapter (RT cores).
///
/// Intentionally concrete on [`WgpuComputeContext`] (not generic over
/// [`OracleContext`]): it needs the wgpu-only acceleration-structure build
/// ([`WgpuComputeContext::build_triangle_scene`]) and the dedicated
/// [`WgpuPipeline::dispatch_rayprobe`] binding path, which have no CUDA analogue —
/// it is not a §7 cross-backend kernel.
pub fn evaluate_rayprobe(
    context: &mut WgpuComputeContext,
    schedule: Schedule,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let kernel = BuiltinKernel::RayProbe.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let scene = rayprobe_scene();
    let rays = rayprobe_rays();
    let n_rays = rays.len() / 8;
    let expected = rayprobe_cpu(&rays, &scene);

    // The acceleration structure is binding 0 (not a slab buffer); rays are binding
    // 1 (read), hits are binding 2 (read-write).
    let (_blas, tlas) = context.build_triangle_scene(&scene)?;
    let view_rays =
        context.allocate_and_write(bytemuck::cast_slice(&rays), 1, 0, BindingUsage::StorageRead)?;
    let output_bytes_len = (n_rays * size_of::<f32>()).max(4);
    let view_hits =
        context.allocate_transient(output_bytes_len, 2, 0, BindingUsage::StorageReadWrite)?;
    let buffers = vec![view_rays, view_hits];

    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;
    for _ in 0..warmups {
        pipeline.dispatch_rayprobe(&tlas, &buffers, &schedule, n_rays)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch_rayprobe(&tlas, &buffers, &schedule, n_rays)?);
    }

    let actual = context.read_buffer_f32(&view_hits)?;
    let tolerance = OracleTolerance {
        absolute: 1.0e-2,
        relative: 1.0e-2,
    };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "ray-probe: {} mismatches; first={:?}, max_abs={}; expected={:?} actual={:?}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            expected,
            actual
        )));
    }

    // dispatch_rayprobe uses a wall-clock completion timer (no timestamp pass).
    let timing = TimingSummary::from_samples(TimingSource::CompletionClock, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            // The ray-probe vectors are a fixed scene/ray set, not seed-derived; the
            // hash still pins the exact expected committed-hit vector.
            vector_seed: None,
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}
