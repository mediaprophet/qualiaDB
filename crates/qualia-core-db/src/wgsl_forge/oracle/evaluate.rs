//! The oracle driver + certification: [`evaluate_builtin`] dispatches each builtin
//! to its evaluator, the generic [`OracleContext`] cross-backend paths (affine, FFN,
//! top-k), the cooperative-matrix / CUDA tensor-core probes, and the
//! [`certify_builtin`] / [`candidate_evaluation`] wrappers that fold a run into a
//! manifest.

use crate::wgsl_forge::execute::{
    BindingUsage, OracleContext, QualiaCompute, WgpuComputeContext, WgpuPipeline,
};
use crate::wgsl_forge::{
    emit_shader, validate_wgsl, BuiltinKernel, CandidateEvaluation, CertificationManifest,
    ForgeError, GeneratedShader, Schedule, TargetBackend, TimingSource, TimingSummary,
    ValidationLevel,
};

use super::kernels::*;
use super::params::*;
use super::reference::*;
use super::report::*;

pub fn evaluate_builtin(
    context: &mut WgpuComputeContext,
    builtin: BuiltinKernel,
    schedule: Schedule,
    length: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    // Top-k uses a different oracle (per-block selection) and output sizing, so
    // dispatch to its dedicated evaluator. k defaults to min(8, block_size).
    if builtin == BuiltinKernel::TopK {
        let k = (schedule.workgroup_size as usize).clamp(1, 8);
        return evaluate_topk(context, schedule, length, k, warmups, samples);
    }
    if builtin == BuiltinKernel::FusedFfn {
        // Representative FFN dims; output_size tracks the requested length.
        let output_size = length.clamp(1, 4096);
        return evaluate_ffn(context, schedule, 64, 128, output_size, warmups, samples);
    }
    if builtin == BuiltinKernel::P64Project {
        return evaluate_p64(context, schedule, length.clamp(1, 65_536), warmups, samples);
    }
    if builtin == BuiltinKernel::RayProbe {
        // Ray-probe uses a fixed BLAS/TLAS scene and ray set (length is not a knob),
        // and an acceleration-structure binding the generic buffer path can't carry.
        return evaluate_rayprobe(context, schedule, warmups, samples);
    }
    if builtin == BuiltinKernel::TernaryGemv {
        // M output rows (tracks the requested length) over a representative K
        // activation vector; one invocation per output row.
        let m = length.clamp(1, 4096);
        return evaluate_ternary_gemv(context, schedule, m, 256, warmups, samples);
    }
    if builtin == BuiltinKernel::Gemm {
        // Fixed representative square 64×64×64 GEMM (4096 output elements, one
        // invocation each). The kernel/oracle handle arbitrary M,K,N; a fixed
        // square is used here so the certified evidence is over a stable problem
        // size independent of `length` (the generic per-element `length` knob is
        // not a natural GEMM shape parameter).
        return evaluate_gemm(context, schedule, 64, 64, 64, warmups, samples);
    }
    if builtin == BuiltinKernel::Gemv {
        // Fixed representative square 256×256 GEMV (256 output rows, one invocation
        // each). The kernel/oracle handle arbitrary M,N; a fixed square is used here
        // so the certified evidence is over a stable problem size independent of
        // `length` (the generic per-element `length` knob is not a natural GEMV shape
        // parameter).
        return evaluate_gemv(context, schedule, 256, 256, warmups, samples);
    }
    if builtin == BuiltinKernel::Fft {
        // One workgroup of n = workgroup_size threads (n a power of two, one
        // complex element per thread). The schedule's workgroup_size IS the
        // transform length; when it is not set the default workgroup_size (256)
        // gives a 256-point FFT, independent of the generic per-element `length`.
        let n = schedule.workgroup_size as usize;
        return evaluate_fft(context, schedule, n, warmups, samples);
    }
    // The only remaining builtin is the affine kernel — evaluated by the generic
    // cross-backend path (plan §7), here on the wgpu context.
    evaluate_affine(context, schedule, length, warmups, samples)
}

/// Cross-backend differential-oracle evaluation of the affine kernel against
/// [`affine_cpu`] (plan §7). Generic over [`OracleContext`], so the *same* code runs
/// on wgpu (via [`WgpuComputeContext`]) and CUDA (via `CudaComputeContext`); the
/// backend only differs inside [`OracleContext::run_kernel`]. The CPU-reference
/// vectors, bindings, dispatch sizing and tolerance are identical to what the
/// wgpu-inline and `evaluate_affine_cuda` paths used before unification.
pub fn evaluate_affine<C: OracleContext>(
    context: &mut C,
    schedule: Schedule,
    length: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let kernel = BuiltinKernel::AffineF32.spec();
    schedule.validate(&kernel, context.constraints())?;
    context.constraints().supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let case = OracleCase::affine(length, 0x5141_4C49_4157_4753, 1.618_034, -0.125);

    let input_bytes = bytemuck::cast_slice(case.input.as_slice());
    let view_input = context.allocate_and_write(input_bytes, 0, 0, BindingUsage::StorageRead)?;

    let output_bytes_len = (case.input.len() * size_of::<f32>()).max(4);
    let view_output =
        context.allocate_transient(output_bytes_len, 1, 0, BindingUsage::StorageReadWrite)?;

    let params_bytes = bytemuck::bytes_of(&case.params);
    let view_params = context.allocate_and_write(params_bytes, 2, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_input, view_output, view_params];

    let timing_samples = context.run_kernel(
        &kernel,
        &schedule,
        &buffers,
        case.input.len(),
        warmups,
        samples,
    )?;
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    let tolerance = OracleTolerance::default();
    let oracle = compare_f32(&case.expected, &actual, tolerance);

    // Free transient allocations
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "{} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            oracle.max_relative_error
        )));
    }

    let source = if context.timestamp_supported() {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };

    let timing = TimingSummary::from_samples(source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter().clone(),
            constraints: *context.constraints(),
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(case.seed),
            vector_hash: vector_hash_f32(&case.expected),
        },
    ))
}

/// Differential-oracle evaluation of the cooperative-matrix (tensor-core) 8x8
/// GEMM tile `C = A * B` against [`matmul_cpu`]. All-f32 — the only coopmat
/// configuration wgpu/naga 29 implements (see [`crate::wgsl_forge::emit::coopmat`]).
/// One subgroup (32-lane NVIDIA warp) cooperatively computes the tile; the
/// row-major loads/store reproduce the row-major CPU reference, so agreement is
/// to f32 precision (a tiny tolerance covers tensor-core accumulation order).
pub fn evaluate_matmul_tc(
    context: &mut WgpuComputeContext,
) -> Result<ComparisonReport, ForgeError> {
    if !context.constraints.supports_coopmat {
        return Err(ForgeError::GpuUnavailable(
            "adapter lacks cooperative-matrix support".to_string(),
        ));
    }
    let n = crate::wgsl_forge::emit::coopmat::TILE as usize;
    let source = crate::wgsl_forge::matmul_tc_wgsl();
    validate_wgsl(&source)?;

    let a = topk_inputs(n * n, 0x4D41_545F_4141_4141);
    let b = topk_inputs(n * n, 0x4D41_545F_4242_4242);
    let expected = matmul_cpu(&a, &b, n);

    let view_a =
        context.allocate_and_write(bytemuck::cast_slice(&a), 0, 0, BindingUsage::StorageRead)?;
    let view_b =
        context.allocate_and_write(bytemuck::cast_slice(&b), 1, 0, BindingUsage::StorageRead)?;
    let zeros = vec![0.0f32; n * n];
    let view_c = context.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let buffers = vec![view_a, view_b, view_c];

    let schedule = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    let pipeline = WgpuPipeline::compile(context, &source, "matmul_tc")?;
    pipeline.dispatch(&buffers, &schedule, 1)?;
    let actual = context.read_buffer_f32(&view_c)?;

    drop(pipeline);
    context.clear_transient_allocations();
    Ok(compare_f32(
        &expected,
        &actual,
        OracleTolerance {
            absolute: 1.0e-3,
            relative: 1.0e-3,
        },
    ))
}

/// Diagnostic: cooperative-matrix load→store round-trip (no multiply). Loads `a`
/// as a role-C fragment and stores it to `c`; `c` must equal `a`. This verifies
/// `coopLoadT`/`coopStoreT` work on the adapter (they do — the `coopMultiplyAdd`
/// path is the one currently blocked on the experimental backend).
pub fn evaluate_coopmat_loadstore(
    context: &mut WgpuComputeContext,
) -> Result<ComparisonReport, ForgeError> {
    let source = r#"enable wgpu_cooperative_matrix;
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(2) var<storage, read_write> c: array<f32>;

@compute @workgroup_size(32)
fn matmul_tc() {
    let m = coopLoadT<coop_mat8x8<f32, C>>(&a[0], 8u);
    coopStoreT(m, &c[0], 8u);
}"#;
    validate_wgsl(source)?;
    let n = 8usize;
    let a = topk_inputs(n * n, 0x4D41_545F_4141_4141);
    let view_a =
        context.allocate_and_write(bytemuck::cast_slice(&a), 0, 0, BindingUsage::StorageRead)?;
    let zeros = vec![0.0f32; n * n];
    let view_c = context.allocate_and_write(
        bytemuck::cast_slice(&zeros),
        2,
        0,
        BindingUsage::StorageReadWrite,
    )?;
    let buffers = vec![view_a, view_c];
    let schedule = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    let pipeline = WgpuPipeline::compile(context, source, "matmul_tc")?;
    pipeline.dispatch(&buffers, &schedule, 1)?;
    let actual = context.read_buffer_f32(&view_c)?;
    drop(pipeline);
    context.clear_transient_allocations();
    Ok(compare_f32(&a, &actual, OracleTolerance::default()))
}

/// Cross-backend oracle (plan §7/§10): runs the affine kernel through the native
/// CUDA backend (CUDA-C compiled to PTX by NVRTC) and checks it against the *same*
/// CPU reference vectors used for the wgpu backend. Requires a CUDA device.
///
/// Thin wrapper over the generic [`evaluate_affine`]: builds a CUDA context and runs
/// the unified path with `warmups = 0, samples = 1` (one dispatch, as before),
/// returning just the [`ComparisonReport`].
#[cfg(feature = "cuda")]
pub fn evaluate_affine_cuda(length: usize) -> Result<ComparisonReport, ForgeError> {
    use crate::wgsl_forge::execute::CudaComputeContext;
    let mut context = CudaComputeContext::new(16 * 1024 * 1024)?;
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    let (_, evaluation) = evaluate_affine(&mut context, schedule, length, 0, 1)?;
    Ok(evaluation.oracle)
}

/// Cross-backend oracle for the fused FFN via the CUDA backend. Thin wrapper over
/// the generic [`evaluate_ffn`] (`warmups = 0, samples = 1`, one dispatch as before).
#[cfg(feature = "cuda")]
pub fn evaluate_ffn_cuda(
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
) -> Result<ComparisonReport, ForgeError> {
    use crate::wgsl_forge::execute::CudaComputeContext;
    let mut context = CudaComputeContext::new(64 * 1024 * 1024)?;
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    let (_, evaluation) = evaluate_ffn(
        &mut context,
        schedule,
        input_size,
        hidden_size,
        output_size,
        0,
        1,
    )?;
    Ok(evaluation.oracle)
}

/// Cross-backend oracle for top-k via the CUDA backend (CUDA-C `__shared__`). Thin
/// wrapper over the generic [`evaluate_topk`] (`warmups = 0, samples = 1`, one
/// dispatch as before; `block_size` is `schedule.workgroup_size`, i.e. 64).
#[cfg(feature = "cuda")]
pub fn evaluate_topk_cuda(length: usize, k: usize) -> Result<ComparisonReport, ForgeError> {
    use crate::wgsl_forge::execute::CudaComputeContext;
    let mut context = CudaComputeContext::new(64 * 1024 * 1024)?;
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    let (_, evaluation) = evaluate_topk(&mut context, schedule, length, k, 0, 1)?;
    Ok(evaluation.oracle)
}

/// Tensor-core oracle: runs the genuine f16-input WMMA GEMM (`C = A * B`,
/// 16x16x16) on the CUDA backend via the `nvcuda::wmma` fragment API, compiled by
/// NVRTC for the device's compute capability, and checks it against the row-major
/// CPU reference. This is the *reduced-precision* tensor-core path (f16 A/B inputs,
/// f32 accumulator) that wgpu/naga 29's cooperative-matrix backend cannot express
/// — 29 implements only all-f32 8x8x8, and even that multiply is non-functional on
/// the 29.0.3 execution path (no published fix; see [`crate::wgsl_forge::emit::coopmat`]).
/// Requires a CUDA device with compute capability >= 7.0 (Volta+).
#[cfg(feature = "cuda")]
pub fn evaluate_matmul_tc_cuda() -> Result<ComparisonReport, ForgeError> {
    use crate::wgsl_forge::emit::cuda_c::{WMMA_GEMM_16X16_ENTRY, WMMA_GEMM_16X16_SRC};
    use crate::wgsl_forge::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};
    let mut context = CudaComputeContext::new(16 * 1024 * 1024)?;
    let n = 16usize; // WMMA m16n16k16 tile.

    // f16 A/B inputs, packed as raw u16 bit patterns; f32 output. The CPU
    // reference rounds the same inputs through f16 first, so GPU/CPU agree to f16
    // input precision (the f32 accumulator keeps the K=16 sum tight).
    let a_f32 = topk_inputs(n * n, 0x574D_4D41_5F41_4141);
    let b_f32 = topk_inputs(n * n, 0x574D_4D41_5F42_4242);
    let a_bits: Vec<u16> = a_f32
        .iter()
        .map(|&x| half::f16::from_f32(x).to_bits())
        .collect();
    let b_bits: Vec<u16> = b_f32
        .iter()
        .map(|&x| half::f16::from_f32(x).to_bits())
        .collect();
    let a_round: Vec<f32> = a_bits
        .iter()
        .map(|&b| half::f16::from_bits(b).to_f32())
        .collect();
    let b_round: Vec<f32> = b_bits
        .iter()
        .map(|&b| half::f16::from_bits(b).to_f32())
        .collect();
    let expected = matmul_cpu(&a_round, &b_round, n);

    let view_a = context.allocate_and_write(bytemuck::cast_slice(&a_bits), 0, 0)?;
    let view_b = context.allocate_and_write(bytemuck::cast_slice(&b_bits), 1, 0)?;
    let zeros = vec![0.0f32; n * n];
    let view_c = context.allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0)?;
    let buffers = vec![view_a, view_b, view_c];

    // One warp computes the whole tile: workgroup_size 32, element_count 1 -> grid (1,1,1).
    let schedule = Schedule {
        workgroup_size: 32,
        ..Default::default()
    };
    let pipeline = CudaPipeline::compile_cuda_c_source(
        &context,
        WMMA_GEMM_16X16_SRC,
        WMMA_GEMM_16X16_ENTRY,
        &[0, 1, 2],
    )?;
    pipeline.dispatch(&buffers, &schedule, 1)?;
    let actual = context.read_buffer_f32(&view_c)?;
    Ok(compare_f32(
        &expected,
        &actual,
        OracleTolerance {
            absolute: 5.0e-2,
            relative: 1.0e-2,
        },
    ))
}

pub fn certify_builtin(
    context: &mut WgpuComputeContext,
    builtin: BuiltinKernel,
    schedule: Schedule,
    length: usize,
    warmups: usize,
    samples: usize,
) -> Result<CertificationManifest, ForgeError> {
    let (generated, evaluation) =
        evaluate_builtin(context, builtin, schedule, length, warmups, samples)?;
    let validation = validate_wgsl(&generated.source)?;
    // Fold the exact tolerance this run was verified against into the reuse key
    // (plan §8) so evidence is not reused under a coarser correctness bar.
    let cache_key = evaluation.adapter.cache_key(
        &generated.semantic_hash,
        &generated.source_hash,
        schedule,
        evaluation.tolerance,
    )?;
    // Provenance timestamp (plan §8). SystemTime is fine here — this is runtime
    // certification, not a deterministic build artifact. Source-commit provenance
    // would need build-time plumbing we don't have, so it is out of scope for now.
    let certified_at_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs());
    Ok(CertificationManifest {
        forge_schema_version: crate::wgsl_forge::FORGE_SCHEMA_VERSION,
        crate_version: env!("CARGO_PKG_VERSION").to_string(),
        wgpu_api_version: crate::wgsl_forge::WGPU_API_VERSION.to_string(),
        naga_api_version: crate::wgsl_forge::NAGA_API_VERSION.to_string(),
        kernel_id: generated.kernel_id,
        semantic_hash: generated.semantic_hash,
        source_hash: generated.source_hash,
        schedule,
        validation_level: ValidationLevel::Certified,
        validation,
        adapter: Some(evaluation.adapter),
        oracle: Some(evaluation.oracle),
        timing: Some(evaluation.timing),
        cache_key: Some(cache_key),
        vector_seed: evaluation.vector_seed,
        vector_hash: Some(evaluation.vector_hash),
        certified_at_unix,
    })
}

pub fn candidate_evaluation(
    context: &mut WgpuComputeContext,
    builtin: BuiltinKernel,
    schedule: Schedule,
    length: usize,
    warmups: usize,
    samples: usize,
) -> Result<CandidateEvaluation, ForgeError> {
    let (_, evaluation) = evaluate_builtin(context, builtin, schedule, length, warmups, samples)?;
    let timing_source = evaluation.timing.source;
    Ok(CandidateEvaluation {
        oracle: evaluation.oracle,
        timing_source,
        samples_ns: evaluation.samples_ns,
    })
}

/// Cross-backend differential-oracle evaluation for the fused FFN against
/// [`ffn_cpu`] (plan §7). Generic over [`OracleContext`] — the same code runs on
/// wgpu and CUDA; only [`OracleContext::run_kernel`] differs. One workgroup-thread
/// per output element; the output buffer is `output_size`. Tensors, bindings,
/// dispatch sizing and tolerance are identical to the prior wgpu/`evaluate_ffn_cuda`
/// paths.
pub fn evaluate_ffn<C: OracleContext>(
    context: &mut C,
    schedule: Schedule,
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let kernel = BuiltinKernel::FusedFfn.spec();
    schedule.validate(&kernel, context.constraints())?;
    context.constraints().supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let vector_seed = 0x4646_4E5F_5345_4544u64;
    let (input, w1, w2) = ffn_tensors(input_size, hidden_size, output_size, vector_seed);
    let expected = ffn_cpu(&input, &w1, &w2, input_size, hidden_size, output_size);

    let view_input = context.allocate_and_write(
        bytemuck::cast_slice(&input),
        0,
        0,
        BindingUsage::StorageRead,
    )?;
    let view_w1 =
        context.allocate_and_write(bytemuck::cast_slice(&w1), 1, 0, BindingUsage::StorageRead)?;
    let view_w2 =
        context.allocate_and_write(bytemuck::cast_slice(&w2), 2, 0, BindingUsage::StorageRead)?;
    let output_bytes_len = (output_size * size_of::<f32>()).max(4);
    let view_output =
        context.allocate_transient(output_bytes_len, 3, 0, BindingUsage::StorageReadWrite)?;
    let params = FfnParams {
        input_size: input_size as u32,
        hidden_size: hidden_size as u32,
        output_size: output_size as u32,
        _pad: 0,
    };
    let view_params =
        context.allocate_and_write(bytemuck::bytes_of(&params), 4, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_input, view_w1, view_w2, view_output, view_params];
    let timing_samples =
        context.run_kernel(&kernel, &schedule, &buffers, output_size, warmups, samples)?;
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    // FFN accumulates over the hidden dim and uses tanh, so allow a modest
    // tolerance for GPU/CPU transcendental + accumulation differences.
    let tolerance = OracleTolerance {
        absolute: 2.0e-3,
        relative: 2.0e-3,
    };
    let oracle = compare_f32(&expected, &actual, tolerance);

    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "fused-ffn: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count,
            oracle.first_mismatch,
            oracle.max_absolute_error,
            oracle.max_relative_error
        )));
    }

    let timing_source = if context.timestamp_supported() {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter().clone(),
            constraints: *context.constraints(),
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(vector_seed),
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}

/// Cross-backend differential-oracle evaluation for the top-k kernel against
/// [`topk_cpu`] (plan §7). Generic over [`OracleContext`] — the same code runs on
/// wgpu and CUDA; only [`OracleContext::run_kernel`] differs.
///
/// `block_size` is fixed to `schedule.workgroup_size`; the dispatch launches one
/// workgroup per block. The output buffer is sized to `num_blocks * k`, not the
/// input length. Seeds, bindings, dispatch sizing and tolerance match the prior
/// wgpu/`evaluate_topk_cuda` paths.
pub fn evaluate_topk<C: OracleContext>(
    context: &mut C,
    schedule: Schedule,
    length: usize,
    k: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation(
            "sample count must be non-zero".to_string(),
        ));
    }
    let block_size = schedule.workgroup_size as usize;
    if k == 0 || k > block_size {
        return Err(ForgeError::GpuValidation(format!(
            "k must be in 1..=block_size ({block_size}); got {k}"
        )));
    }

    let kernel = BuiltinKernel::TopK.spec();
    schedule.validate(&kernel, context.constraints())?;
    context.constraints().supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let vector_seed = 0x5031_4B5F_5345_4544u64;
    let input = topk_inputs(length, vector_seed);
    let expected = topk_cpu(&input, length, k, block_size);

    let input_bytes = bytemuck::cast_slice(input.as_slice());
    let view_input = context.allocate_and_write(input_bytes, 0, 0, BindingUsage::StorageRead)?;

    let output_len = expected.len();
    let output_bytes_len = (output_len * size_of::<f32>()).max(4);
    let view_output =
        context.allocate_transient(output_bytes_len, 1, 0, BindingUsage::StorageReadWrite)?;

    let params = TopKParams {
        length: length as u32,
        k: k as u32,
        block_size: block_size as u32,
        _pad: 0,
    };
    let view_params =
        context.allocate_and_write(bytemuck::bytes_of(&params), 2, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_input, view_output, view_params];
    let timing_samples =
        context.run_kernel(&kernel, &schedule, &buffers, length, warmups, samples)?;
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    let tolerance = OracleTolerance::default();
    let oracle = compare_f32(&expected, &actual, tolerance);

    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "top-k: {} mismatches; first={:?}, max_abs={}",
            oracle.mismatch_count, oracle.first_mismatch, oracle.max_absolute_error
        )));
    }

    let timing_source = if context.timestamp_supported() {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples)
        .ok_or_else(|| ForgeError::GpuValidation("GPU produced no timing samples".to_string()))?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter().clone(),
            constraints: *context.constraints(),
            oracle,
            timing,
            samples_ns: timing_samples,
            tolerance: (tolerance.absolute, tolerance.relative),
            vector_seed: Some(vector_seed),
            vector_hash: vector_hash_f32(&expected),
        },
    ))
}
