//! CPU reference implementations and deterministic test-vector generators — the
//! differential-oracle "truth" each GPU kernel is checked against, plus the private
//! math helpers (gelu, ternary decode, Möller–Trumbore) they build on.

use super::params::AffineParams;
use crate::wgsl_forge::P64GpuWords64;

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

pub(super) fn gelu(x: f32) -> f32 {
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
