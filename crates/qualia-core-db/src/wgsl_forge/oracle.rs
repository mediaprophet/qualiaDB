use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::wgsl_forge::execute::{
    BindingUsage, OracleContext, QualiaCompute, WgpuComputeContext, WgpuPipeline,
};
use crate::wgsl_forge::{
    emit_shader, validate_wgsl, AdapterConstraints, AdapterIdentity, BuiltinKernel,
    CandidateEvaluation, CertificationManifest, ForgeError, GeneratedShader, P64GpuWords64,
    Schedule, TargetBackend, TimingSource, TimingSummary, ValidationLevel,
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

/// 16-byte uniform block for the ternary-GEMV kernel (`k_words` == ceil(k/16)).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct TernaryGemvParams {
    pub m: u32,
    pub k: u32,
    pub k_words: u32,
    pub _pad: u32,
}

/// 16-byte uniform block for the dense GEMM kernel: row-major `C[M×N] = A[M×K]·B[K×N]`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct GemmParams {
    pub m: u32,
    pub n: u32,
    pub k: u32,
    pub _pad: u32,
}

/// 16-byte uniform block for the dense GEMV kernel: row-major `y[M] = A[M×N]·x[N]`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct GemvParams {
    pub m: u32,
    pub n: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

/// 16-byte uniform block for the radix-2 FFT kernel: `n` complex elements,
/// `log2n = log2(n)`. The kernel runs one workgroup of `n` threads.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct FftParams {
    pub n: u32,
    pub log2n: u32,
    pub _pad0: u32,
    pub _pad1: u32,
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

/// Number of 2-bit ternary codes packed into one `u32` word.
pub const TERNARY_CODES_PER_WORD: usize = 16;

/// Map a 2-bit ternary code to its value, exactly as the GPU kernel does:
/// `0 -> 0.0, 1 -> +1.0, 2 -> -1.0, 3 -> 0.0` (3 unused).
#[inline]
fn ternary_code_value(code: u32) -> f32 {
    match code & 3 {
        1 => 1.0,
        2 => -1.0,
        _ => 0.0,
    }
}

/// CPU reference for the BitNet-style ternary GEMV, the bit-for-bit mirror of the
/// emitted kernel: `out[o] = scale[o] * sum_{i<K} ternary(w[o,i]) * x[i]`.
///
/// `w_packed` holds, per output row `o`, `ceil(K/16)` `u32` words laid out
/// contiguously; each word carries 16 ternary codes in low-to-high 2-bit lanes.
/// Lanes beyond `K` (in the final word of a row) are skipped, matching the
/// kernel's `i >= k` guard.
pub fn ternary_gemv_cpu(
    x: &[f32],
    w_packed: &[u32],
    scale: &[f32],
    m: usize,
    k: usize,
) -> Vec<f32> {
    let k_words = k.div_ceil(TERNARY_CODES_PER_WORD);
    let mut out = Vec::with_capacity(m);
    for o in 0..m {
        let row_base = o * k_words;
        let mut acc = 0.0f32;
        for word_idx in 0..k_words {
            let word = w_packed[row_base + word_idx];
            let lane_base = word_idx * TERNARY_CODES_PER_WORD;
            for lane in 0..TERNARY_CODES_PER_WORD {
                let i = lane_base + lane;
                if i >= k {
                    break;
                }
                let code = (word >> (lane * 2)) & 3;
                acc += ternary_code_value(code) * x[i];
            }
        }
        out.push(scale[o] * acc);
    }
    out
}

/// Deterministic ternary-GEMV test tensors: the activation vector `x` (length K),
/// the 2-bit-packed ternary weights (`M * ceil(K/16)` words), and the per-row
/// scales (length M). Codes are drawn from the xorshift stream and reduced into
/// `{0,1,2}` so the weights only ever decode to `{0, +1, -1}` (never the unused
/// `3`), keeping the GPU and CPU paths bit-identical.
pub fn ternary_gemv_tensors(m: usize, k: usize, seed: u64) -> (Vec<f32>, Vec<u32>, Vec<f32>) {
    let k_words = k.div_ceil(TERNARY_CODES_PER_WORD);
    let x = topk_inputs(k, seed);
    // Scales in [-1, 1] — same generator/contract as every other oracle vector.
    let scale = topk_inputs(m, seed ^ 0x3333);
    let mut w_packed = vec![0u32; m * k_words];
    let mut state = (seed ^ 0x7465_726E_6172_7900).max(1); // "ternary\0"
    for o in 0..m {
        let row_base = o * k_words;
        for i in 0..k {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // Reduce to {0,1,2}: 0->0.0, 1->+1.0, 2->-1.0 (never the unused 3).
            let code = (state % 3) as u32;
            let word_idx = i / TERNARY_CODES_PER_WORD;
            let lane = (i % TERNARY_CODES_PER_WORD) as u32;
            w_packed[row_base + word_idx] |= code << (lane * 2);
        }
    }
    (x, w_packed, scale)
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
    /// The exact correctness tolerance (absolute, relative) this kernel's GPU
    /// result was verified against, recorded so it can be folded into the reuse
    /// cache key (plan §8) — a coarser tolerance must not silently reuse evidence.
    pub tolerance: (f32, f32),
    /// Deterministic seed of the test vector this run was checked against, when the
    /// vector is seed-derived (`None` for fixed-scene kernels like ray-probe).
    pub vector_seed: Option<u64>,
    /// blake3 hex of the expected CPU-reference output bytes — pins the vector.
    pub vector_hash: String,
}

/// blake3 hex of a `&[f32]` expected vector's little-endian bytes. Used to record
/// exactly which CPU-reference output a certified manifest was checked against.
fn vector_hash_f32(expected: &[f32]) -> String {
    blake3::hash(bytemuck::cast_slice(expected))
        .to_hex()
        .to_string()
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

/// Row-major n×n matrix multiply reference: `c[i][j] = sum_k a[i][k] * b[k][j]`.
pub fn matmul_cpu(a: &[f32], b: &[f32], n: usize) -> Vec<f32> {
    gemm_cpu(a, b, n, n, n)
}

/// Row-major general dense GEMM reference, the bit-for-bit mirror of the emitted
/// `gemm` kernel: `C[M×N] = A[M×K] · B[K×N]`, i.e.
/// `C[i][j] = sum_{k<K} A[i*K + k] * B[k*N + j]`. The inner-sum order (k ascending)
/// matches the kernel's `kk` loop so GPU/CPU agree to f32 summation precision.
pub fn gemm_cpu(a: &[f32], b: &[f32], m: usize, k: usize, n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; m * n];
    for i in 0..m {
        let a_row = i * k;
        for j in 0..n {
            let mut acc = 0.0f32;
            for kk in 0..k {
                acc += a[a_row + kk] * b[kk * n + j];
            }
            c[i * n + j] = acc;
        }
    }
    c
}

/// Deterministic GEMM test tensors: A (M×K) and B (K×N), both drawn from the same
/// xorshift stream as every other oracle vector and scaled by `1/sqrt(K)` so the
/// length-K dot products stay O(1) and GPU/CPU agree within a tight tolerance.
pub fn gemm_tensors(m: usize, k: usize, n: usize, seed: u64) -> (Vec<f32>, Vec<f32>) {
    let scale = 1.0 / (k.max(1) as f32).sqrt();
    let a: Vec<f32> = topk_inputs(m * k, seed ^ 0x1111)
        .into_iter()
        .map(|v| v * scale)
        .collect();
    let b: Vec<f32> = topk_inputs(k * n, seed ^ 0x2222)
        .into_iter()
        .map(|v| v * scale)
        .collect();
    (a, b)
}

/// Row-major dense GEMV reference, the bit-for-bit mirror of the emitted `gemv`
/// kernel: `y[M] = A[M×N] · x[N]`, i.e. `y[i] = sum_{j<N} A[i*N + j] * x[j]`. The
/// inner-sum order (j ascending) matches the kernel's `j` loop so GPU/CPU agree to
/// f32 summation precision.
pub fn gemv_cpu(a: &[f32], x: &[f32], m: usize, n: usize) -> Vec<f32> {
    let mut y = vec![0.0f32; m];
    for i in 0..m {
        let a_row = i * n;
        let mut acc = 0.0f32;
        for j in 0..n {
            acc += a[a_row + j] * x[j];
        }
        y[i] = acc;
    }
    y
}

/// Deterministic GEMV test tensors: A (M×N) and x (N), both drawn from the same
/// xorshift stream as every other oracle vector and scaled by `1/sqrt(N)` so the
/// length-N dot products stay O(1) and GPU/CPU agree within a tight tolerance.
pub fn gemv_tensors(m: usize, n: usize, seed: u64) -> (Vec<f32>, Vec<f32>) {
    let scale = 1.0 / (n.max(1) as f32).sqrt();
    let a: Vec<f32> = topk_inputs(m * n, seed ^ 0x1111)
        .into_iter()
        .map(|v| v * scale)
        .collect();
    let x: Vec<f32> = topk_inputs(n, seed ^ 0x2222)
        .into_iter()
        .map(|v| v * scale)
        .collect();
    (a, x)
}

/// Naive `O(N²)` forward Discrete Fourier Transform, the reference the GPU
/// radix-2 FFT is differentially checked against. Complex data is interleaved
/// f32: element `j` is `(input[2*j], input[2*j+1]) = (real, imag)`, so both the
/// `input` slice and the returned vector hold `2*N` f32.
///
/// `X[k] = sum_{j<N} x[j] * exp(-2*pi*i * k * j / N)` — the SAME forward sign
/// convention `exp(-2*pi*i*...)` the emitted kernel's twiddle uses, so the CPU
/// reference and the GPU FFT compute the identical transform. Angles are
/// accumulated in f64 for a clean reference; the comparison tolerance covers the
/// f32-vs-f64 and FFT-vs-DFT summation differences.
pub fn dft_cpu(input_interleaved: &[f32], n: usize) -> Vec<f32> {
    use std::f64::consts::PI;
    let mut out = vec![0.0f32; 2 * n];
    for k in 0..n {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for j in 0..n {
            let xr = input_interleaved[2 * j] as f64;
            let xi = input_interleaved[2 * j + 1] as f64;
            let ang = -2.0 * PI * (k as f64) * (j as f64) / (n as f64);
            let (s, c) = ang.sin_cos();
            // x * (c + i s): real = xr*c - xi*s, imag = xr*s + xi*c.
            re += xr * c - xi * s;
            im += xr * s + xi * c;
        }
        out[2 * k] = re as f32;
        out[2 * k + 1] = im as f32;
    }
    out
}

/// Deterministic complex test signal as interleaved f32 (`2*n` values), drawn
/// from the same xorshift stream as every other oracle vector so it is
/// reproducible. Both the real and imaginary parts land in `[-1, 1]`.
pub fn fft_inputs(n: usize, seed: u64) -> Vec<f32> {
    // 2*n interleaved (real, imag) samples in [-1, 1].
    topk_inputs(2 * n, seed)
}

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

// ── Ray-query (ray-probe) differential oracle ──────────────────────────────

fn rp_sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn rp_cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn rp_dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Möller–Trumbore ray/triangle intersection. Returns the hit distance `t` along
/// `dir` (which need not be normalised — `t` is in units of `dir`) when the ray
/// crosses the triangle interior, else `None`. This is the CPU mirror of the GPU's
/// committed-intersection `t`.
fn ray_triangle_intersect(
    origin: [f32; 3],
    dir: [f32; 3],
    v0: [f32; 3],
    v1: [f32; 3],
    v2: [f32; 3],
) -> Option<f32> {
    const EPS: f32 = 1.0e-7;
    let e1 = rp_sub(v1, v0);
    let e2 = rp_sub(v2, v0);
    let p = rp_cross(dir, e2);
    let det = rp_dot(e1, p);
    if det.abs() < EPS {
        return None; // ray parallel to the triangle plane
    }
    let inv = 1.0 / det;
    let tvec = rp_sub(origin, v0);
    let u = rp_dot(tvec, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = rp_cross(tvec, e1);
    let v = rp_dot(dir, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    Some(rp_dot(e2, q) * inv)
}

/// The fixed ray-probe scene: three world-space triangles as a flat `f32` list
/// (9 floats/triangle = 3 verts × xyz). Two coplanar triangles tile the quad
/// `[0,2]²` at `z = 2`; a third sits behind them at `z = 4`, so rays through the
/// lower-left region hit two triangles and must commit the nearer one (t at z=2).
pub fn rayprobe_scene() -> Vec<f32> {
    vec![
        // T0 @ z=2 (lower-left half of the quad: x>=0, y>=0, x+y<=2)
        0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 0.0, 2.0, 2.0, //
        // T1 @ z=2 (upper-right half: x<=2, y<=2, x+y>=2)
        2.0, 2.0, 2.0, 0.0, 2.0, 2.0, 2.0, 0.0, 2.0, //
        // T2 @ z=4 (behind T0; same lower-left footprint)
        0.0, 0.0, 4.0, 2.0, 0.0, 4.0, 0.0, 2.0, 4.0,
    ]
}

/// The fixed ray set as the 8-float-per-ray layout the emitter expects
/// (`origin.xyz, dir.xyz, t_min, t_max`). All rays originate at `z = -1` and point
/// along `+z`; hit rays target clear triangle interiors (away from edges) so GPU
/// BVH traversal and the CPU reference agree, and miss rays point well outside.
pub fn rayprobe_rays() -> Vec<f32> {
    // (x, y) at z=-1; expected committed t = 3.0 for hits at z=2, else -1.0.
    let xy: [(f32, f32); 12] = [
        (0.5, 0.5),    // T0 (also over T2 -> commit nearer T0)
        (0.3, 0.3),    // T0 (over T2)
        (1.0, 0.5),    // T0
        (0.5, 1.0),    // T0
        (1.5, 1.7),    // T1
        (1.7, 1.5),    // T1
        (1.6, 1.6),    // T1
        (5.0, 5.0),    // miss
        (-2.0, 0.5),   // miss
        (0.5, -2.0),   // miss
        (3.0, 3.0),    // miss
        (10.0, -10.0), // miss
    ];
    let mut rays = Vec::with_capacity(xy.len() * 8);
    for (x, y) in xy {
        rays.extend_from_slice(&[x, y, -1.0, 0.0, 0.0, 1.0, 0.001, 100.0]);
    }
    rays
}

/// CPU reference for the ray-probe kernel: for each ray, the nearest committed
/// triangle hit `t` within `[t_min, t_max]`, or `-1.0` on a miss — matching the
/// emitter's `hits[i] = committed.t else -1.0`.
pub fn rayprobe_cpu(rays: &[f32], scene: &[f32]) -> Vec<f32> {
    let tri = |k: usize| -> ([f32; 3], [f32; 3], [f32; 3]) {
        let b = k * 9;
        (
            [scene[b], scene[b + 1], scene[b + 2]],
            [scene[b + 3], scene[b + 4], scene[b + 5]],
            [scene[b + 6], scene[b + 7], scene[b + 8]],
        )
    };
    let triangles = scene.len() / 9;
    let mut out = Vec::with_capacity(rays.len() / 8);
    for r in rays.chunks_exact(8) {
        let origin = [r[0], r[1], r[2]];
        let dir = [r[3], r[4], r[5]];
        let (t_min, t_max) = (r[6], r[7]);
        let mut nearest = f32::INFINITY;
        for k in 0..triangles {
            let (v0, v1, v2) = tri(k);
            if let Some(t) = ray_triangle_intersect(origin, dir, v0, v1, v2) {
                if t >= t_min && t <= t_max && t < nearest {
                    nearest = t;
                }
            }
        }
        out.push(if nearest.is_finite() { nearest } else { -1.0 });
    }
    out
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
    fn rayprobe_cpu_reference_is_sane() {
        let scene = rayprobe_scene();
        let rays = rayprobe_rays();
        let hits = rayprobe_cpu(&rays, &scene);
        assert_eq!(hits.len(), 12);
        // First 7 rays hit the z=2 quad (committed t = 3.0); last 5 miss (-1.0).
        for (i, &t) in hits.iter().enumerate() {
            if i < 7 {
                assert!((t - 3.0).abs() < 1.0e-4, "ray {i} expected t=3, got {t}");
            } else {
                assert_eq!(t, -1.0, "ray {i} expected a miss");
            }
        }
    }

    #[test]
    #[ignore = "requires a ray-query capable adapter (RT cores)"]
    fn rayprobe_certifies_on_real_gpu() {
        // Feasibility + correctness: build a BLAS/TLAS and run the emitted ray_probe
        // shader, checking committed hit distances against the Möller–Trumbore CPU
        // reference. This is the gate for whether wgpu 29.0.3 ray-query *executes*.
        let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            ..Default::default()
        };
        let (_, eval) =
            evaluate_rayprobe(&mut context, schedule, 1, 3).expect("ray-probe evaluation");
        assert!(
            eval.oracle.passed(),
            "ray-probe GPU/oracle mismatch: {:?}",
            eval.oracle
        );
    }

    #[test]
    #[ignore = "requires a ray-query capable adapter (RT cores)"]
    fn rayprobe_certify_builtin_on_real_gpu() {
        // Full certification pipeline: certify_builtin -> evaluate_builtin ->
        // evaluate_rayprobe -> manifest, proving RayProbe is a first-class
        // certifiable builtin (not just a standalone test).
        let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            ..Default::default()
        };
        let manifest = certify_builtin(&mut context, BuiltinKernel::RayProbe, schedule, 12, 1, 3)
            .expect("ray-probe certification");
        assert_eq!(manifest.validation_level, ValidationLevel::Certified);
        assert!(manifest.oracle.as_ref().is_some_and(|o| o.passed()));
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
        let out = ffn_cpu(
            &[1.0, 2.0, 3.0],
            &vec![0.0; 4 * 3],
            &vec![0.5; 2 * 4],
            3,
            4,
            2,
        );
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
    fn ternary_gemv_cpu_matches_hand_checked_2x4() {
        // A 2x4 case computed by hand. K=4 => 1 u32 word per row.
        // Row 0 codes [1,2,0,1] -> ternary [+1,-1,0,+1]:
        //   1<<0 | 2<<2 | 0<<4 | 1<<6 = 1 + 8 + 0 + 64 = 73.
        // Row 1 codes [2,1,1,3] -> ternary [-1,+1,+1,0] (3 -> 0.0):
        //   2<<0 | 1<<2 | 1<<4 | 3<<6 = 2 + 4 + 16 + 192 = 214.
        // x = [1,2,3,4], scale = [2.0, 10.0].
        //   out[0] = 2.0  * ( +1*1 -1*2 +0*3 +1*4 ) = 2.0  * 3 = 6.0
        //   out[1] = 10.0 * ( -1*1 +1*2 +1*3 +0*4 ) = 10.0 * 4 = 40.0
        let x = [1.0f32, 2.0, 3.0, 4.0];
        let w_packed = [73u32, 214u32];
        let scale = [2.0f32, 10.0];
        let out = ternary_gemv_cpu(&x, &w_packed, &scale, 2, 4);
        assert_eq!(out, vec![6.0, 40.0]);
    }

    #[test]
    fn ternary_gemv_tensors_are_deterministic_and_decode_in_range() {
        // The generated tensors must be reproducible and the packed codes must only
        // ever decode to {0, +1, -1} (never the unused code 3), so GPU == CPU.
        let (x0, w0, s0) = ternary_gemv_tensors(7, 40, 123);
        let (x1, w1, s1) = ternary_gemv_tensors(7, 40, 123);
        assert_eq!((x0.clone(), w0.clone(), s0.clone()), (x1, w1, s1));
        assert_eq!(x0.len(), 40);
        assert_eq!(s0.len(), 7);
        assert_eq!(w0.len(), 7 * 40usize.div_ceil(TERNARY_CODES_PER_WORD));
        for word in &w0 {
            for lane in 0..TERNARY_CODES_PER_WORD {
                let code = (word >> (lane * 2)) & 3;
                assert_ne!(
                    code, 3,
                    "generator must never emit the unused ternary code 3"
                );
            }
        }
    }

    #[test]
    fn ternary_gemv_cpu_zero_codes_yield_zero() {
        // All-zero packed words => every ternary weight is 0.0 => output all zero
        // regardless of x and scale.
        let x = topk_inputs(20, 9);
        let scale = [3.0f32, -2.0, 7.0];
        let w_packed = vec![0u32; 3 * 20usize.div_ceil(TERNARY_CODES_PER_WORD)];
        let out = ternary_gemv_cpu(&x, &w_packed, &scale, 3, 20);
        assert_eq!(out, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn gemm_cpu_matches_hand_checked_2x3_3x2() {
        // A (2×3) · B (3×2) = C (2×2), all computed by hand. Row-major.
        //   A = [[1, 2, 3],      B = [[ 7,  8],
        //        [4, 5, 6]]           [ 9, 10],
        //                             [11, 12]]
        //   C[0][0] = 1*7 + 2*9  + 3*11 =  7 + 18 + 33 =  58
        //   C[0][1] = 1*8 + 2*10 + 3*12 =  8 + 20 + 36 =  64
        //   C[1][0] = 4*7 + 5*9  + 6*11 = 28 + 45 + 66 = 139
        //   C[1][1] = 4*8 + 5*10 + 6*12 = 32 + 50 + 72 = 154
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let b = [7.0f32, 8.0, 9.0, 10.0, 11.0, 12.0];
        let c = gemm_cpu(&a, &b, 2, 3, 2);
        assert_eq!(c, vec![58.0, 64.0, 139.0, 154.0]);
        // matmul_cpu is the n×n special case of gemm_cpu; a 2×2 identity confirms
        // the generalization is consistent with the pre-existing reference.
        let id = [1.0f32, 0.0, 0.0, 1.0];
        let mat = [2.0f32, 3.0, 4.0, 5.0];
        assert_eq!(matmul_cpu(&id, &mat, 2), gemm_cpu(&id, &mat, 2, 2, 2));
        assert_eq!(matmul_cpu(&id, &mat, 2), vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn gemm_tensors_are_deterministic() {
        assert_eq!(gemm_tensors(8, 16, 4, 7), gemm_tensors(8, 16, 4, 7));
        let (a, b) = gemm_tensors(8, 16, 4, 7);
        assert_eq!(a.len(), 8 * 16);
        assert_eq!(b.len(), 16 * 4);
    }

    #[test]
    fn gemv_cpu_matches_hand_checked_2x3() {
        // A (2×3) · x (3) = y (2), computed by hand. Row-major.
        //   A = [[1, 2, 3],      x = [1, 1, 1]
        //        [4, 5, 6]]
        //   y[0] = 1*1 + 2*1 + 3*1 =  6
        //   y[1] = 4*1 + 5*1 + 6*1 = 15
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let x = [1.0f32, 1.0, 1.0];
        let y = gemv_cpu(&a, &x, 2, 3);
        assert_eq!(y, vec![6.0, 15.0]);
        // GEMV is the N-column special case of GEMM with N_gemm = 1; a 2×3 · 3×1
        // GEMM must produce the same vector, confirming the references are consistent.
        let x_col = gemm_cpu(&a, &x, 2, 3, 1);
        assert_eq!(x_col, y);
    }

    #[test]
    fn gemv_tensors_are_deterministic() {
        assert_eq!(gemv_tensors(8, 16, 7), gemv_tensors(8, 16, 7));
        let (a, x) = gemv_tensors(8, 16, 7);
        assert_eq!(a.len(), 8 * 16);
        assert_eq!(x.len(), 16);
    }

    #[test]
    fn dft_cpu_impulse_is_all_ones() {
        // A unit impulse at index 0 (x[0] = 1, rest 0) has a flat spectrum:
        // X[k] = sum_j x[j] e^{-2pi i kj/N} = x[0] = 1 for every k. So every
        // output bin is exactly (1, 0). Hand-verified, exact.
        let n = 8usize;
        let mut input = vec![0.0f32; 2 * n];
        input[0] = 1.0; // real impulse at j=0
        let out = dft_cpu(&input, n);
        assert_eq!(out.len(), 2 * n);
        for k in 0..n {
            assert!((out[2 * k] - 1.0).abs() < 1e-5, "bin {k} real should be 1");
            assert!(out[2 * k + 1].abs() < 1e-5, "bin {k} imag should be 0");
        }
    }

    #[test]
    fn dft_cpu_dc_signal_concentrates_in_bin_zero() {
        // A constant real signal x[j] = 1 for all j has all its energy in bin 0:
        // X[0] = N, X[k>0] = 0. For N=4 input [1,0, 1,0, 1,0, 1,0]:
        //   X[0] = 1+1+1+1 = 4
        //   X[1] = 1 + e^{-i pi/2} + e^{-i pi} + e^{-i 3pi/2} = 1 - i - 1 + i = 0
        //   X[2], X[3] = 0 by the same cancellation.
        // Hand-verified -> expected interleaved [4,0, 0,0, 0,0, 0,0].
        let n = 4usize;
        let input = [1.0f32, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let out = dft_cpu(&input, n);
        assert!((out[0] - 4.0).abs() < 1e-5, "X[0] real should be 4");
        assert!(out[1].abs() < 1e-5, "X[0] imag should be 0");
        for k in 1..n {
            assert!(out[2 * k].abs() < 1e-5, "bin {k} real should be 0");
            assert!(out[2 * k + 1].abs() < 1e-5, "bin {k} imag should be 0");
        }
    }

    #[test]
    fn fft_inputs_are_deterministic_interleaved() {
        let a = fft_inputs(16, 7);
        let b = fft_inputs(16, 7);
        assert_eq!(a, b);
        assert_eq!(a.len(), 2 * 16, "interleaved complex => 2*n f32");
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_fft_matches_oracle_on_real_gpu() {
        // N=256-point forward FFT, one workgroup of 256 threads, checked against
        // the O(N²) DFT reference (same forward sign convention).
        let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 256,
            items_per_invocation: 1,
            vector_width: 1,
            ..Default::default()
        };
        let (_, evaluation) =
            evaluate_fft(&mut context, schedule, 256, 2, 5).expect("fft evaluation");
        assert!(
            evaluation.oracle.passed(),
            "fft GPU/oracle mismatch: {:?}",
            evaluation.oracle
        );
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_ternary_gemv_matches_oracle_on_real_gpu() {
        let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
            ..Default::default()
        };
        let (_, evaluation) = evaluate_ternary_gemv(&mut context, schedule, 256, 256, 2, 5)
            .expect("ternary-gemv evaluation");
        assert!(
            evaluation.oracle.passed(),
            "ternary-gemv GPU/oracle mismatch: {:?}",
            evaluation.oracle
        );
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_gemm_matches_oracle_on_real_gpu() {
        let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
            ..Default::default()
        };
        let (_, evaluation) =
            evaluate_gemm(&mut context, schedule, 64, 64, 64, 2, 5).expect("gemm evaluation");
        assert!(
            evaluation.oracle.passed(),
            "gemm GPU/oracle mismatch: {:?}",
            evaluation.oracle
        );
    }

    #[test]
    #[ignore = "requires a native wgpu adapter"]
    fn generated_gemv_matches_oracle_on_real_gpu() {
        let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
        let schedule = Schedule {
            workgroup_size: 64,
            items_per_invocation: 1,
            vector_width: 1,
            ..Default::default()
        };
        let (_, evaluation) =
            evaluate_gemv(&mut context, schedule, 256, 256, 2, 5).expect("gemv evaluation");
        assert!(
            evaluation.oracle.passed(),
            "gemv GPU/oracle mismatch: {:?}",
            evaluation.oracle
        );
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
        let (_, evaluation) =
            evaluate_p64(&mut context, schedule, 1000, 2, 5).expect("p64 evaluation");
        assert!(
            evaluation.oracle.passed(),
            "p64 GPU/oracle mismatch: {:?}",
            evaluation.oracle
        );
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
        assert!(
            evaluation.oracle.passed(),
            "fused-ffn GPU/oracle mismatch: {:?}",
            evaluation.oracle
        );
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
        assert!(
            evaluation.oracle.passed(),
            "top-k GPU/oracle mismatch: {:?}",
            evaluation.oracle
        );
    }

    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device"]
    fn affine_oracle_matches_across_cuda_backend() {
        let report = evaluate_affine_cuda(4099).expect("cuda affine evaluation");
        assert!(
            report.passed(),
            "CUDA affine GPU/oracle mismatch: {report:?}"
        );
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

    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "requires a CUDA device with tensor cores (compute capability >= 7.0)"]
    fn wmma_matmul_certifies_on_cuda_tensor_cores() {
        // The genuine tensor-core path: f16-input WMMA GEMM through NVRTC, the
        // reduced-precision config wgpu 29's coopmat cannot run.
        let report = evaluate_matmul_tc_cuda().expect("cuda wmma evaluation");
        assert!(report.passed(), "CUDA WMMA C=A*B mismatch: {report:?}");
    }

    #[test]
    #[ignore = "requires a cooperative-matrix capable adapter"]
    fn coopmat_loadstore_roundtrips_on_real_gpu() {
        // Diagnostic: coopLoadT/coopStoreT round-trip correctly on the adapter (c == a).
        let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
        let report = evaluate_coopmat_loadstore(&mut context).expect("coopmat round-trip");
        assert!(
            report.passed(),
            "coopmat load/store round-trip mismatch: {report:?}"
        );
    }

    // NOTE: there is intentionally no GPU test asserting the *WGSL* coopmat
    // `coopMultiplyAdd` (`evaluate_matmul_tc`) computes C = A * B. The emitter
    // produces the correct, naga-validated all-f32 8x8x8 kernel
    // (`cooperative_matrix_tile_validates`, above), but wgpu/naga 29.0.3's
    // experimental cooperative-matrix *execution* path returns all-zeros from the
    // multiply (the load/store round-trip works — see below), and no published
    // wgpu release fixes it (29.0.3 is the newest on crates.io; the fix is on
    // unreleased git main). The genuine tensor-core multiply is proven instead via
    // CUDA WMMA (`wmma_matmul_certifies_on_cuda_tensor_cores`). When wgpu ships the
    // coopmat execution fix, add the `evaluate_matmul_tc` assertion back here.

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
