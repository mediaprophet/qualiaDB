use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::wgsl_forge::execute::{BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline};
use crate::wgsl_forge::{
    AdapterConstraints, AdapterIdentity, BuiltinKernel, CandidateEvaluation, CertificationManifest,
    ForgeError, GeneratedShader, P64GpuWords64, Schedule, TargetBackend, TimingSource, TimingSummary,
    ValidationLevel, emit_shader, validate_wgsl,
};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct AffineParams {
    pub length: u32,
    pub scale: f32,
    pub bias: f32,
    pub _pad: u32,
}

/// 16-byte uniform block for the top-k kernel (`block_size` == workgroup size).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct TopKParams {
    pub length: u32,
    pub k: u32,
    pub block_size: u32,
    pub _pad: u32,
}

/// 16-byte uniform block for the fused-FFN kernel.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct FfnParams {
    pub input_size: u32,
    pub hidden_size: u32,
    pub output_size: u32,
    pub _pad: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OracleTolerance {
    pub absolute: f32,
    pub relative: f32,
}

impl Default for OracleTolerance {
    fn default() -> Self {
        Self {
            absolute: 1.0e-6,
            relative: 1.0e-5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub compared: usize,
    pub mismatch_count: usize,
    pub first_mismatch: Option<usize>,
    pub max_absolute_error: f32,
    pub max_relative_error: f32,
}

impl ComparisonReport {
    pub const fn passed(&self) -> bool {
        self.mismatch_count == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OracleCase {
    pub seed: u64,
    pub input: Vec<f32>,
    pub expected: Vec<f32>,
    pub params: AffineParams,
}

impl OracleCase {
    pub fn affine(length: usize, seed: u64, scale: f32, bias: f32) -> Self {
        let length = length.min(u32::MAX as usize);
        let mut state = seed.max(1);
        let mut input = Vec::with_capacity(length);
        for _ in 0..length {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let unit = (state as u32) as f32 / u32::MAX as f32;
            input.push(unit.mul_add(2.0, -1.0));
        }
        let params = AffineParams {
            length: length as u32,
            scale,
            bias,
            _pad: 0,
        };
        let expected = affine_cpu(&input, params);
        Self {
            seed,
            input,
            expected,
            params,
        }
    }
}

pub fn affine_cpu(input: &[f32], params: AffineParams) -> Vec<f32> {
    input
        .iter()
        .take(params.length as usize)
        .map(|value| value.mul_add(params.scale, params.bias))
        .collect()
}

/// Sentinel reused by the GPU kernel for "below any real value" (f32::MIN).
const TOPK_SENTINEL_BITS: u32 = 0xff7f_ffff;

/// Deterministic xorshift test vector in `[-1, 1]`, matching the affine generator.
pub fn topk_inputs(length: usize, seed: u64) -> Vec<f32> {
    let mut state = seed.max(1);
    let mut input = Vec::with_capacity(length);
    for _ in 0..length {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let unit = (state as u32) as f32 / u32::MAX as f32;
        input.push(unit.mul_add(2.0, -1.0));
    }
    input
}

/// CPU reference for the per-block top-k: the `k` largest values of each
/// `block_size`-element block, in descending order. Blocks shorter than
/// `block_size` (the tail) are padded with the sentinel, mirroring the GPU
/// kernel's out-of-range loads.
pub fn topk_cpu(input: &[f32], length: usize, k: usize, block_size: usize) -> Vec<f32> {
    let sentinel = f32::from_bits(TOPK_SENTINEL_BITS);
    let num_blocks = length.div_ceil(block_size.max(1));
    let mut out = Vec::with_capacity(num_blocks * k);
    for block in 0..num_blocks {
        let start = block * block_size;
        let end = (start + block_size).min(length);
        let mut values: Vec<f32> = input[start..end].to_vec();
        values.resize(block_size, sentinel);
        values.sort_by(|a, b| b.partial_cmp(a).expect("test vectors are never NaN"));
        out.extend_from_slice(&values[..k.min(values.len())]);
    }
    out
}

fn gelu(x: f32) -> f32 {
    0.5 * x * (1.0 + (0.797_884_56 * (x + 0.044_715 * x * x * x)).tanh())
}

/// CPU reference for the fused FFN, matching the emitted kernel's op order
/// exactly (hidden outer, input inner) so GPU/CPU agree within tolerance:
/// `out[o] = sum_h w2[o,h] * gelu(sum_i w1[h,i] * input[i])`.
pub fn ffn_cpu(
    input: &[f32],
    w1: &[f32],
    w2: &[f32],
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(output_size);
    for o in 0..output_size {
        let mut acc = 0.0f32;
        for h in 0..hidden_size {
            let mut hv = 0.0f32;
            let w1_row = h * input_size;
            for i in 0..input_size {
                hv += w1[w1_row + i] * input[i];
            }
            acc += w2[o * hidden_size + h] * gelu(hv);
        }
        out.push(acc);
    }
    out
}

/// Deterministic FFN test tensors. Weights are scaled by 1/sqrt(fan_in) so the
/// pre-activations stay O(1) and GPU/CPU agree within a modest tolerance.
pub fn ffn_tensors(
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
    seed: u64,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let input = topk_inputs(input_size, seed);
    let w1_scale = 1.0 / (input_size as f32).sqrt();
    let w2_scale = 1.0 / (hidden_size as f32).sqrt();
    let w1: Vec<f32> = topk_inputs(hidden_size * input_size, seed ^ 0x1111)
        .into_iter()
        .map(|v| v * w1_scale)
        .collect();
    let w2: Vec<f32> = topk_inputs(output_size * hidden_size, seed ^ 0x2222)
        .into_iter()
        .map(|v| v * w2_scale)
        .collect();
    (input, w1, w2)
}

pub fn compare_f32(
    expected: &[f32],
    actual: &[f32],
    tolerance: OracleTolerance,
) -> ComparisonReport {
    let compared = expected.len().max(actual.len());
    let mut mismatch_count = expected.len().abs_diff(actual.len());
    let mut first_mismatch =
        (expected.len() != actual.len()).then(|| expected.len().min(actual.len()));
    let mut max_absolute_error = 0.0f32;
    let mut max_relative_error = 0.0f32;

    for (index, (&expected, &actual)) in expected.iter().zip(actual.iter()).enumerate() {
        let (matches, absolute, relative) = compare_value(expected, actual, tolerance);
        max_absolute_error = max_absolute_error.max(absolute);
        max_relative_error = max_relative_error.max(relative);
        if !matches {
            mismatch_count += 1;
            if first_mismatch
                .map(|current| index < current)
                .unwrap_or(true)
            {
                first_mismatch = Some(index);
            }
        }
    }

    ComparisonReport {
        compared,
        mismatch_count,
        first_mismatch,
        max_absolute_error,
        max_relative_error,
    }
}

fn compare_value(expected: f32, actual: f32, tolerance: OracleTolerance) -> (bool, f32, f32) {
    if expected.is_nan() || actual.is_nan() {
        return (false, f32::INFINITY, f32::INFINITY);
    }
    if expected.is_infinite() || actual.is_infinite() {
        return (
            expected.to_bits() == actual.to_bits(),
            if expected.to_bits() == actual.to_bits() {
                0.0
            } else {
                f32::INFINITY
            },
            f32::INFINITY,
        );
    }
    let absolute = (expected - actual).abs();
    let denominator = expected.abs().max(actual.abs()).max(f32::MIN_POSITIVE);
    let relative = absolute / denominator;
    (
        absolute <= tolerance.absolute || relative <= tolerance.relative,
        absolute,
        relative,
    )
}

#[derive(Debug, Clone)]
pub struct GpuEvaluation {
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    pub oracle: ComparisonReport,
    pub timing: TimingSummary,
    pub samples_ns: Vec<u64>,
}

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
    let kernel = builtin.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let case = OracleCase::affine(length, 0x5141_4C49_4157_4753, 1.618_034, -0.125);

    let input_bytes = bytemuck::cast_slice(case.input.as_slice());
    let view_input = context.allocate_and_write(input_bytes, 0, 0, BindingUsage::StorageRead)?;

    let output_bytes_len = (case.input.len() * size_of::<f32>()).max(4);
    let view_output = context.allocate_transient(output_bytes_len, 1, 0, BindingUsage::StorageReadWrite)?;

    let params_bytes = bytemuck::bytes_of(&case.params);
    let view_params = context.allocate_and_write(params_bytes, 2, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_input, view_output, view_params];

    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, case.input.len())?;
    }

    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, case.input.len())?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    let oracle = compare_f32(&case.expected, &actual, OracleTolerance::default());

    drop(pipeline); // drop immutable borrow before mutating context

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

    let source = if context.timestamp_supported {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };

    let timing = TimingSummary::from_samples(source, &timing_samples).ok_or_else(|| {
        ForgeError::GpuValidation("GPU produced no timing samples".to_string())
    })?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
        },
    ))
}

/// Row-major n×n matrix multiply reference: `c[i][j] = sum_k a[i][k] * b[k][j]`.
pub fn matmul_cpu(a: &[f32], b: &[f32], n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; n * n];
    for i in 0..n {
        for j in 0..n {
            let mut acc = 0.0f32;
            for k in 0..n {
                acc += a[i * n + k] * b[k * n + j];
            }
            c[i * n + j] = acc;
        }
    }
    c
}

/// Differential-oracle evaluation of the cooperative-matrix (tensor-core) 16x16
/// GEMM tile against [`matmul_cpu`]. Requires an adapter with cooperative-matrix
/// support; tensor-core matmul may run at reduced precision, hence a loose tolerance.
pub fn evaluate_matmul_tc(context: &mut WgpuComputeContext) -> Result<ComparisonReport, ForgeError> {
    if !context.constraints.supports_coopmat {
        return Err(ForgeError::GpuUnavailable(
            "adapter lacks cooperative-matrix support".to_string(),
        ));
    }
    let n = crate::wgsl_forge::emit::coopmat::TILE as usize;
    let source = crate::wgsl_forge::matmul_tc_wgsl("f32");
    validate_wgsl(&source)?;

    let a = topk_inputs(n * n, 0x4D41_545F_4141_4141);
    let b = topk_inputs(n * n, 0x4D41_545F_4242_4242);
    let expected = matmul_cpu(&a, &b, n);

    let view_a = context.allocate_and_write(bytemuck::cast_slice(&a), 0, 0, BindingUsage::StorageRead)?;
    let view_b = context.allocate_and_write(bytemuck::cast_slice(&b), 1, 0, BindingUsage::StorageRead)?;
    let zeros = vec![0.0f32; n * n];
    let view_c = context.allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0, BindingUsage::StorageReadWrite)?;
    let buffers = vec![view_a, view_b, view_c];

    let schedule = Schedule { workgroup_size: 32, ..Default::default() };
    let pipeline = WgpuPipeline::compile(context, &source, "matmul_tc")?;
    pipeline.dispatch(&buffers, &schedule, 1)?;
    let actual = context.read_buffer_f32(&view_c)?;

    drop(pipeline);
    context.clear_transient_allocations();
    Ok(compare_f32(&expected, &actual, OracleTolerance { absolute: 1.0e-2, relative: 1.0e-2 }))
}

/// Cross-backend oracle (plan §7/§10): runs the affine kernel through the native
/// CUDA backend (CUDA-C compiled to PTX by NVRTC) and checks it against the *same*
/// CPU reference vectors used for the wgpu backend. Requires a CUDA device.
#[cfg(feature = "cuda")]
pub fn evaluate_affine_cuda(length: usize) -> Result<ComparisonReport, ForgeError> {
    use crate::wgsl_forge::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};
    let mut context = CudaComputeContext::new(16 * 1024 * 1024)?;
    let schedule = Schedule { workgroup_size: 64, ..Default::default() };
    let kernel = BuiltinKernel::AffineF32.spec();

    let case = OracleCase::affine(length, 0x5141_4C49_4157_4753, 1.618_034, -0.125);
    let view_input = context.allocate_and_write(bytemuck::cast_slice(&case.input), 0, 0)?;
    let view_output = context.allocate_transient((case.input.len() * size_of::<f32>()).max(4), 1, 0)?;
    let view_params = context.allocate_and_write(bytemuck::bytes_of(&case.params), 2, 0)?;
    let buffers = vec![view_input, view_output, view_params];

    let pipeline = CudaPipeline::compile_cuda_c(&context, &kernel, schedule)?;
    pipeline.dispatch(&buffers, &schedule, case.input.len())?;
    let actual = context.read_buffer_f32(&view_output)?;
    Ok(compare_f32(&case.expected, &actual, OracleTolerance::default()))
}

/// Cross-backend oracle for the fused FFN via the CUDA backend.
#[cfg(feature = "cuda")]
pub fn evaluate_ffn_cuda(
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
) -> Result<ComparisonReport, ForgeError> {
    use crate::wgsl_forge::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};
    let mut context = CudaComputeContext::new(64 * 1024 * 1024)?;
    let schedule = Schedule { workgroup_size: 64, ..Default::default() };
    let kernel = BuiltinKernel::FusedFfn.spec();

    let (input, w1, w2) = ffn_tensors(input_size, hidden_size, output_size, 0x4646_4E5F_5345_4544);
    let expected = ffn_cpu(&input, &w1, &w2, input_size, hidden_size, output_size);
    let view_input = context.allocate_and_write(bytemuck::cast_slice(&input), 0, 0)?;
    let view_w1 = context.allocate_and_write(bytemuck::cast_slice(&w1), 1, 0)?;
    let view_w2 = context.allocate_and_write(bytemuck::cast_slice(&w2), 2, 0)?;
    let view_output = context.allocate_transient((output_size * size_of::<f32>()).max(4), 3, 0)?;
    let params = FfnParams {
        input_size: input_size as u32,
        hidden_size: hidden_size as u32,
        output_size: output_size as u32,
        _pad: 0,
    };
    let view_params = context.allocate_and_write(bytemuck::bytes_of(&params), 4, 0)?;
    let buffers = vec![view_input, view_w1, view_w2, view_output, view_params];

    let pipeline = CudaPipeline::compile_cuda_c(&context, &kernel, schedule)?;
    pipeline.dispatch(&buffers, &schedule, output_size)?;
    let actual = context.read_buffer_f32(&view_output)?;
    Ok(compare_f32(&expected, &actual, OracleTolerance { absolute: 2.0e-3, relative: 2.0e-3 }))
}

/// Cross-backend oracle for top-k via the CUDA backend (CUDA-C `__shared__`).
#[cfg(feature = "cuda")]
pub fn evaluate_topk_cuda(length: usize, k: usize) -> Result<ComparisonReport, ForgeError> {
    use crate::wgsl_forge::execute::{CudaComputeContext, CudaPipeline, QualiaCompute};
    let mut context = CudaComputeContext::new(64 * 1024 * 1024)?;
    let schedule = Schedule { workgroup_size: 64, ..Default::default() };
    let block_size = schedule.workgroup_size as usize;
    let kernel = BuiltinKernel::TopK.spec();

    let input = topk_inputs(length, 0x5031_4B5F_5345_4544);
    let expected = topk_cpu(&input, length, k, block_size);
    let view_input = context.allocate_and_write(bytemuck::cast_slice(&input), 0, 0)?;
    let view_output = context.allocate_transient((expected.len() * size_of::<f32>()).max(4), 1, 0)?;
    let params = TopKParams {
        length: length as u32,
        k: k as u32,
        block_size: block_size as u32,
        _pad: 0,
    };
    let view_params = context.allocate_and_write(bytemuck::bytes_of(&params), 2, 0)?;
    let buffers = vec![view_input, view_output, view_params];

    let pipeline = CudaPipeline::compile_cuda_c(&context, &kernel, schedule)?;
    pipeline.dispatch(&buffers, &schedule, length)?;
    let actual = context.read_buffer_f32(&view_output)?;
    Ok(compare_f32(&expected, &actual, OracleTolerance::default()))
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
    let cache_key =
        evaluation
            .adapter
            .cache_key(&generated.semantic_hash, &generated.source_hash, schedule)?;
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

/// Deterministic P64 descriptors with small (f32-exact) u32 words.
pub fn p64_records(count: usize, seed: u64) -> Vec<P64GpuWords64> {
    let mut state = seed.max(1);
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let mut words = [0u32; 16];
        for word in words.iter_mut() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            *word = (state as u32) % 1000;
        }
        records.push(P64GpuWords64 {
            lanes: [
                [words[0], words[1], words[2], words[3]],
                [words[4], words[5], words[6], words[7]],
                [words[8], words[9], words[10], words[11]],
                [words[12], words[13], words[14], words[15]],
            ],
        });
    }
    records
}

/// CPU reference for the P64 projection: `out[r] = sum_w weights[w] * f32(word_w)`,
/// reading the 16 packed u32 words in the same lane order as the kernel.
pub fn p64_project_cpu(records: &[P64GpuWords64], weights: &[f32]) -> Vec<f32> {
    records
        .iter()
        .map(|record| {
            let words: &[u32; 16] = bytemuck::cast_ref(record);
            let mut acc = 0.0f32;
            for w in 0..16 {
                acc += weights[w] * words[w] as f32;
            }
            acc
        })
        .collect()
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
        return Err(ForgeError::GpuValidation("sample count must be non-zero".to_string()));
    }
    let kernel = BuiltinKernel::P64Project.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let records = p64_records(count, 0x5036_345F_5345_4544);
    let weights = topk_inputs(16, 0x5036_345F_5742_5453);
    let expected = p64_project_cpu(&records, &weights);

    let view_input = context.allocate_and_write(bytemuck::cast_slice(&records), 0, 0, BindingUsage::StorageRead)?;
    let view_weights = context.allocate_and_write(bytemuck::cast_slice(&weights), 1, 0, BindingUsage::StorageRead)?;
    let output_bytes_len = (count * size_of::<f32>()).max(4);
    let view_output = context.allocate_transient(output_bytes_len, 2, 0, BindingUsage::StorageReadWrite)?;

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
        return Err(ForgeError::GpuValidation("GPU produced a zero-duration timing sample".to_string()));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    let tolerance = OracleTolerance { absolute: 1.0e-1, relative: 1.0e-4 };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "p64-project: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count, oracle.first_mismatch, oracle.max_absolute_error, oracle.max_relative_error
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
        },
    ))
}

/// Differential-oracle evaluation for the fused FFN against [`ffn_cpu`].
/// One workgroup-thread per output element; the output buffer is `output_size`.
pub fn evaluate_ffn(
    context: &mut WgpuComputeContext,
    schedule: Schedule,
    input_size: usize,
    hidden_size: usize,
    output_size: usize,
    warmups: usize,
    samples: usize,
) -> Result<(GeneratedShader, GpuEvaluation), ForgeError> {
    if samples == 0 {
        return Err(ForgeError::GpuValidation("sample count must be non-zero".to_string()));
    }
    let kernel = BuiltinKernel::FusedFfn.spec();
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let (input, w1, w2) = ffn_tensors(input_size, hidden_size, output_size, 0x4646_4E5F_5345_4544);
    let expected = ffn_cpu(&input, &w1, &w2, input_size, hidden_size, output_size);

    let view_input = context.allocate_and_write(bytemuck::cast_slice(&input), 0, 0, BindingUsage::StorageRead)?;
    let view_w1 = context.allocate_and_write(bytemuck::cast_slice(&w1), 1, 0, BindingUsage::StorageRead)?;
    let view_w2 = context.allocate_and_write(bytemuck::cast_slice(&w2), 2, 0, BindingUsage::StorageRead)?;
    let output_bytes_len = (output_size * size_of::<f32>()).max(4);
    let view_output = context.allocate_transient(output_bytes_len, 3, 0, BindingUsage::StorageReadWrite)?;
    let params = FfnParams {
        input_size: input_size as u32,
        hidden_size: hidden_size as u32,
        output_size: output_size as u32,
        _pad: 0,
    };
    let view_params = context.allocate_and_write(bytemuck::bytes_of(&params), 4, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_input, view_w1, view_w2, view_output, view_params];
    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, output_size)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, output_size)?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation("GPU produced a zero-duration timing sample".to_string()));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    // FFN accumulates over the hidden dim and uses tanh, so allow a modest
    // tolerance for GPU/CPU transcendental + accumulation differences.
    let tolerance = OracleTolerance { absolute: 2.0e-3, relative: 2.0e-3 };
    let oracle = compare_f32(&expected, &actual, tolerance);

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "fused-ffn: {} mismatches; first={:?}, max_abs={}, max_rel={}",
            oracle.mismatch_count, oracle.first_mismatch, oracle.max_absolute_error, oracle.max_relative_error
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
        },
    ))
}

/// Differential-oracle evaluation for the top-k kernel against [`topk_cpu`].
///
/// `block_size` is fixed to `schedule.workgroup_size`; the dispatch launches one
/// workgroup per block. Requires a native adapter. The output buffer is sized to
/// `num_blocks * k`, not the input length.
pub fn evaluate_topk(
    context: &mut WgpuComputeContext,
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
    schedule.validate(&kernel, &context.constraints)?;
    context.constraints.supports_kernel(&kernel)?;
    let generated = emit_shader(&kernel, schedule, TargetBackend::Wgsl)?;
    validate_wgsl(&generated.source)?;

    let input = topk_inputs(length, 0x5031_4B5F_5345_4544);
    let expected = topk_cpu(&input, length, k, block_size);

    let input_bytes = bytemuck::cast_slice(input.as_slice());
    let view_input = context.allocate_and_write(input_bytes, 0, 0, BindingUsage::StorageRead)?;

    let output_len = expected.len();
    let output_bytes_len = (output_len * size_of::<f32>()).max(4);
    let view_output = context.allocate_transient(output_bytes_len, 1, 0, BindingUsage::StorageReadWrite)?;

    let params = TopKParams {
        length: length as u32,
        k: k as u32,
        block_size: block_size as u32,
        _pad: 0,
    };
    let view_params = context.allocate_and_write(bytemuck::bytes_of(&params), 2, 0, BindingUsage::Uniform)?;

    let buffers = vec![view_input, view_output, view_params];
    let pipeline = WgpuPipeline::compile(context, &generated.source, &kernel.entry_point)?;

    for _ in 0..warmups {
        pipeline.dispatch(&buffers, &schedule, length)?;
    }
    let mut timing_samples = Vec::with_capacity(samples);
    for _ in 0..samples {
        timing_samples.push(pipeline.dispatch(&buffers, &schedule, length)?);
    }
    if timing_samples.iter().any(|s| *s == 0) {
        return Err(ForgeError::GpuValidation(
            "GPU produced a zero-duration timing sample".to_string(),
        ));
    }

    let actual = context.read_buffer_f32(&view_output)?;
    let oracle = compare_f32(&expected, &actual, OracleTolerance::default());

    drop(pipeline);
    context.clear_transient_allocations();

    if !oracle.passed() {
        return Err(ForgeError::OracleMismatch(format!(
            "top-k: {} mismatches; first={:?}, max_abs={}",
            oracle.mismatch_count, oracle.first_mismatch, oracle.max_absolute_error
        )));
    }

    let timing_source = if context.timestamp_supported {
        TimingSource::GpuTimestamp
    } else {
        TimingSource::CompletionClock
    };
    let timing = TimingSummary::from_samples(timing_source, &timing_samples).ok_or_else(|| {
        ForgeError::GpuValidation("GPU produced no timing samples".to_string())
    })?;

    Ok((
        generated,
        GpuEvaluation {
            adapter: context.adapter.clone(),
            constraints: context.constraints,
            oracle,
            timing,
            samples_ns: timing_samples,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oracle_vectors_are_reproducible() {
        let first = OracleCase::affine(17, 42, 1.5, -0.25);
        let second = OracleCase::affine(17, 42, 1.5, -0.25);
        assert_eq!(first, second);
        assert!(compare_f32(
            &first.expected,
            &second.expected,
            OracleTolerance::default()
        )
        .passed());
    }

    #[test]
    fn comparator_reports_tail_and_numeric_errors() {
        let report = compare_f32(&[1.0, 2.0, 3.0], &[1.0, 2.1], OracleTolerance::default());
        assert_eq!(report.mismatch_count, 2);
        assert_eq!(report.first_mismatch, Some(1));
    }

    #[test]
    fn nan_never_certifies() {
        assert!(!compare_f32(&[f32::NAN], &[f32::NAN], OracleTolerance::default()).passed());
    }

    #[test]
    fn topk_cpu_matches_bruteforce_full_blocks() {
        let block_size = 8usize;
        let k = 3usize;
        let blocks = 4usize;
        let length = block_size * blocks;
        let input = topk_inputs(length, 99);
        let got = topk_cpu(&input, length, k, block_size);
        assert_eq!(got.len(), blocks * k);
        for b in 0..blocks {
            let mut block = input[b * block_size..(b + 1) * block_size].to_vec();
            block.sort_by(|a, c| c.partial_cmp(a).unwrap());
            for i in 0..k {
                assert_eq!(got[b * k + i], block[i], "block {b} rank {i}");
            }
            for i in 1..k {
                assert!(got[b * k + i] <= got[b * k + i - 1], "descending order");
            }
        }
    }

    #[test]
    fn topk_cpu_handles_partial_tail_block() {
        let block_size = 8usize;
        let k = 2usize;
        let length = block_size * 2 + 3; // two full blocks + a 3-element tail
        let input = topk_inputs(length, 7);
        let got = topk_cpu(&input, length, k, block_size);
        assert_eq!(got.len(), 3 * k); // tail counts as a third block
        let mut tail = input[16..19].to_vec();
        tail.sort_by(|a, c| c.partial_cmp(a).unwrap());
        assert_eq!(got[2 * k], tail[0]);
        assert_eq!(got[2 * k + 1], tail[1]);
    }

    #[test]
    fn ffn_cpu_zero_weights_yield_zero() {
        // gelu(0) = 0, so all-zero w1 forces every output to 0 regardless of w2.
        let out = ffn_cpu(&[1.0, 2.0, 3.0], &vec![0.0; 4 * 3], &vec![0.5; 2 * 4], 3, 4, 2);
        assert_eq!(out, vec![0.0, 0.0]);
    }

    #[test]
    fn ffn_cpu_matches_single_neuron() {
        // input=[2], w1=[3], w2=[4]: out = 4 * gelu(3*2).
        let out = ffn_cpu(&[2.0], &[3.0], &[4.0], 1, 1, 1);
        assert!((out[0] - 4.0 * gelu(6.0)).abs() < 1e-6);
    }

    #[test]
    fn ffn_tensors_are_deterministic() {
        assert_eq!(ffn_tensors(8, 16, 4, 7), ffn_tensors(8, 16, 4, 7));
    }

    #[test]
    fn p64_project_cpu_matches_manual() {
        let record = p64_records(1, 5)[0];
        let words: &[u32; 16] = bytemuck::cast_ref(&record);
        let mut weights = vec![0.0f32; 16];
        weights[0] = 2.0;
        weights[5] = -1.0;
        let out = p64_project_cpu(&[record], &weights);
        assert_eq!(out, vec![2.0 * words[0] as f32 - words[5] as f32]);
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_p64_matches_oracle_on_real_gpu() {
        let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
            ..Default::default()
        };
        let (_, evaluation) = evaluate_p64(&mut context, schedule, 1000, 2, 5).expect("p64 evaluation");
        assert!(evaluation.oracle.passed(), "p64 GPU/oracle mismatch: {:?}", evaluation.oracle);
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_ffn_matches_oracle_on_real_gpu() {
        let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
            ..Default::default()
        };
        let (_, evaluation) =
            evaluate_ffn(&mut context, schedule, 64, 128, 256, 2, 5).expect("ffn evaluation");
        assert!(evaluation.oracle.passed(), "fused-ffn GPU/oracle mismatch: {:?}", evaluation.oracle);
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_topk_matches_oracle_on_real_gpu() {
        let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
            ..Default::default()
        };
        let (_, evaluation) =
            evaluate_topk(&mut context, schedule, 64 * 10, 4, 2, 5).expect("topk evaluation");
        assert!(evaluation.oracle.passed(), "top-k GPU/oracle mismatch: {:?}", evaluation.oracle);
    }

    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device"]
    fn affine_oracle_matches_across_cuda_backend() {
        let report = evaluate_affine_cuda(4099).expect("cuda affine evaluation");
        assert!(report.passed(), "CUDA affine GPU/oracle mismatch: {report:?}");
    }

    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device"]
    fn ffn_oracle_matches_across_cuda_backend() {
        let report = evaluate_ffn_cuda(64, 128, 256).expect("cuda ffn evaluation");
        assert!(report.passed(), "CUDA fused-ffn mismatch: {report:?}");
    }

    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device"]
    fn topk_oracle_matches_across_cuda_backend() {
        let report = evaluate_topk_cuda(64 * 10, 4).expect("cuda topk evaluation");
        assert!(report.passed(), "CUDA top-k mismatch: {report:?}");
    }

    #[test]
    #[ignore = "requires a cooperative-matrix capable adapter"]
    fn cooperative_matrix_tile_runs_on_real_gpu() {
        // Verifies the emitted cooperative-matrix (tensor-core) kernel compiles and
        // executes on the adapter producing finite output — i.e. the 8x8 f32 config
        // is supported and does not hit the experimental-UB path (16x16 f32 returns
        // inf). Bit-exact GPU correctness vs. the CPU oracle is still being resolved
        // (the committed result currently reads zero); tracked in the plan ledger.
        let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
        let report = evaluate_matmul_tc(&mut context).expect("coopmat evaluation");
        assert!(
            report.max_absolute_error.is_finite(),
            "coopmat output must be finite (no experimental UB): {report:?}"
        );
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_affine_certifies_on_real_gpu() {
        let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
        let manifest = certify_builtin(
            &mut context,
            BuiltinKernel::AffineF32,
            Schedule {
                workgroup_size: 64,
                items_per_invocation: 2,
                vector_width: 4,
                ..Default::default()
            },
            4_099,
            2,
            5,
        )
        .expect("certification");
        assert_eq!(manifest.validation_level, ValidationLevel::Certified);
        assert!(manifest.oracle.unwrap().passed());
    }
}
