use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::wgsl_forge::execute::{BindingUsage, QualiaCompute, WgpuComputeContext, WgpuPipeline};
use crate::wgsl_forge::{
    AdapterConstraints, AdapterIdentity, BuiltinKernel, CandidateEvaluation, CertificationManifest,
    ForgeError, GeneratedShader, Schedule, TargetBackend, TimingSource, TimingSummary, ValidationLevel,
    emit_shader,
    validate_wgsl,
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
