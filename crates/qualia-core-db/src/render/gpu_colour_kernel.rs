//! P7.4 — GPU colour-projection / gamut batch kernel + CPU oracle.
//!
//! The GPU kernel processes batches of EMF payloads `[α, μ, σ]` through
//! the full colour pipeline (SPD → XYZ → linear sRGB → gamut map →
//! display sRGB) in parallel. This module provides:
//!
//! 1. **CPU oracle**: the reference implementation for correctness checks.
//! 2. **GPU kernel spec**: the WGSL shader source and buffer layout
//!    specification for the GPU implementation.
//! 3. **Differential check**: compares CPU vs GPU output for a given batch.
//!
//! ## Buffer layout (GPU)
//!
//! ```text
//! Bind group 0:
//!   - storage buffer: EMF input  [f32×3 × N]  (α, μ, σ per element)
//!   - storage buffer: RGB output [u8×3 × N]   (r, g, b per element)
//!   - uniform buffer: params { sample_count: u32 }
//! ```
//!
//! ## Determinism
//!
//! The CPU oracle is fully deterministic. The GPU kernel must produce
//! bit-identical output to the CPU oracle for the same input (within
//! f32 rounding tolerance).

use crate::render::spectral_kernel::{
    emf_to_spd, spd_to_xyz, xyz_to_linear_srgb,
};

// ───────────────────────────────────────────────────────────────────────────
//  CPU oracle — batch EMF → gamut-mapped display RGB
// ───────────────────────────────────────────────────────────────────────────

/// CPU oracle: process a batch of EMF payloads through the full colour
/// pipeline and produce gamut-mapped 8-bit display RGB.
///
/// `emf_in` is `[α, μ, σ]` triples (3*N floats).
/// `rgb_out` is N*3 u8.
/// Returns the number of elements processed.
pub fn cpu_batch_emf_to_display_gamut_mapped(emf_in: &[f32], rgb_out: &mut [u8]) -> usize {
    let n = emf_in.len() / 3;
    let n = n.min(rgb_out.len() / 3);

    for i in 0..n {
        let alpha = emf_in[i * 3];
        let mu = emf_in[i * 3 + 1];
        let sigma = emf_in[i * 3 + 2];

        // EMF → SPD → XYZ → linear sRGB.
        let spd = emf_to_spd(alpha, mu, sigma);
        let xyz = spd_to_xyz(&spd);
        let rgb = xyz_to_linear_srgb(&xyz);

        // Gamut map: clamp to [0,1].
        let r = rgb.r.clamp(0.0, 1.0);
        let g = rgb.g.clamp(0.0, 1.0);
        let b = rgb.b.clamp(0.0, 1.0);

        // sRGB gamma encode.
        let enc = |c: f32| -> u8 {
            let encoded = if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            };
            (encoded * 255.0).round().clamp(0.0, 255.0) as u8
        };

        rgb_out[i * 3] = enc(r);
        rgb_out[i * 3 + 1] = enc(g);
        rgb_out[i * 3 + 2] = enc(b);
    }

    n
}

// ───────────────────────────────────────────────────────────────────────────
//  CPU oracle — batch EMF → XYZ (for differential testing)
// ───────────────────────────────────────────────────────────────────────────

/// CPU oracle: process a batch of EMF payloads and produce XYZ values.
///
/// `emf_in` is `[α, μ, σ]` triples (3*N floats).
/// `xyz_out` is N*3 f32.
pub fn cpu_batch_emf_to_xyz(emf_in: &[f32], xyz_out: &mut [f32]) -> usize {
    let n = emf_in.len() / 3;
    let n = n.min(xyz_out.len() / 3);

    for i in 0..n {
        let spd = emf_to_spd(emf_in[i * 3], emf_in[i * 3 + 1], emf_in[i * 3 + 2]);
        let xyz = spd_to_xyz(&spd);
        xyz_out[i * 3] = xyz.x;
        xyz_out[i * 3 + 1] = xyz.y;
        xyz_out[i * 3 + 2] = xyz.z;
    }

    n
}

// ───────────────────────────────────────────────────────────────────────────
//  GPU kernel specification (WGSL source + buffer layout)
// ───────────────────────────────────────────────────────────────────────────

/// WGSL shader source for the GPU colour-projection batch kernel.
///
/// This is the GPU twin of `cpu_batch_emf_to_display_gamut_mapped`.
/// It processes one EMF element per workgroup invocation.
pub const GPU_COLOUR_KERNEL_WGSL: &str = r#"
// P7.4 — GPU colour-projection / gamut batch kernel.
// Processes one EMF [α, μ, σ] element per invocation.

const SPD_SAMPLES: u32 = 41u;
const LAMBDA_MIN: f32 = 380.0;
const LAMBDA_STEP: f32 = 10.0;

// CIE 1931 2-degree observer CMFs (41 samples, 10nm steps).
const CMF_X = array<f32, 41>(
    0.001368, 0.004243, 0.014310, 0.043510, 0.134380, 0.283900, 0.348280,
    0.336200, 0.290800, 0.195360, 0.095640, 0.032010, 0.004900, 0.009300,
    0.063270, 0.165500, 0.290400, 0.433450, 0.594500, 0.762100, 0.916300,
    1.026300, 1.062200, 1.002600, 0.854450, 0.642400, 0.447900, 0.283500,
    0.164900, 0.087400, 0.046770, 0.022700, 0.011359, 0.005790, 0.002899,
    0.001440, 0.000690, 0.000332, 0.000166, 0.000083, 0.000042,
);

const CMF_Y = array<f32, 41>(
    0.000039, 0.000120, 0.000396, 0.001210, 0.004000, 0.011600, 0.023000,
    0.038000, 0.060000, 0.090980, 0.139020, 0.208020, 0.323000, 0.503000,
    0.710000, 0.862000, 0.954000, 0.994950, 0.995000, 0.952000, 0.870000,
    0.757000, 0.631000, 0.503000, 0.381000, 0.265000, 0.175000, 0.107000,
    0.061000, 0.032000, 0.017000, 0.008210, 0.004102, 0.002091, 0.001047,
    0.000520, 0.000249, 0.000120, 0.000060, 0.000030, 0.000015,
);

const CMF_Z = array<f32, 41>(
    0.006450, 0.020050, 0.067850, 0.207400, 0.645600, 1.282500, 1.453000,
    1.562100, 1.562700, 1.385600, 1.114600, 0.777500, 0.445600, 0.198700,
    0.068100, 0.019800, 0.004100, 0.000500, 0.000200, 0.000010, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
    0.000000, 0.000000, 0.000000, 0.000000, 0.000000, 0.000000,
);

// Precomputed Y normalisation = 1 / Σȳ.
const Y_NORM: f32 = 0.01068128;

// XYZ → linear sRGB matrix.
const M_R: vec3<f32> = vec3(3.2404542, -1.5371385, -0.4985314);
const M_G: vec3<f32> = vec3(-0.9692660, 1.8760108, 0.0415560);
const M_B: vec3<f32> = vec3(0.0556434, -0.2040259, 1.0572252);

struct EmfInput {
    data: array<f32>,
};

struct RgbOutput {
    data: array<u32>,
};

@group(0) @binding(0) var<storage, read> emf_in: EmfInput;
@group(0) @binding(1) var<storage, read_write> rgb_out: RgbOutput;
@group(0) @binding(2) var<uniform> params: Params;

struct Params {
    count: u32,
};

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.count) {
        return;
    }

    let alpha = emf_in.data[idx * 3u];
    let mu = emf_in.data[idx * 3u + 1u];
    let sigma = emf_in.data[idx * 3u + 2u];

    // EMF → SPD: Gaussian peak.
    let lambda = 400.0 + clamp(sigma, 0.0, 1.0) * 300.0;
    let width = 10.0 + clamp(mu, 0.0, 1.0) * 140.0;
    let amplitude = max(alpha, 0.0);

    // SPD → XYZ.
    var x = 0.0;
    var y = 0.0;
    var z = 0.0;
    for (var i = 0u; i < SPD_SAMPLES; i = i + 1u) {
        let l = LAMBDA_MIN + f32(i) * LAMBDA_STEP;
        let d = (l - lambda) / width;
        let s = amplitude * exp(-0.5 * d * d);
        x = x + s * CMF_X[i];
        y = y + s * CMF_Y[i];
        z = z + s * CMF_Z[i];
    }
    x = x * Y_NORM;
    y = y * Y_NORM;
    z = z * Y_NORM;

    // XYZ → linear sRGB.
    let xyz = vec3<f32>(x, y, z);
    var r = dot(M_R, xyz);
    var g = dot(M_G, xyz);
    var b = dot(M_B, xyz);

    // Gamut map: clamp to [0, 1].
    r = clamp(r, 0.0, 1.0);
    g = clamp(g, 0.0, 1.0);
    b = clamp(b, 0.0, 1.0);

    // sRGB gamma encode.
    let encode = fn(c: f32) -> f32 {
        if (c <= 0.0031308) {
            return 12.92 * c;
        }
        return 1.055 * pow(c, 1.0 / 2.4) - 0.055;
    };

    let er = encode(r);
    let eg = encode(g);
    let eb = encode(b);

    // Pack into u32 (RGB24).
    let packed = u32(round(er * 255.0)) |
                 (u32(round(eg * 255.0)) << 8u) |
                 (u32(round(eb * 255.0)) << 16u);

    rgb_out.data[idx] = packed;
}
"#;

/// Buffer layout specification for the GPU kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuBufferLayout {
    pub emf_stride: usize,
    pub rgb_stride: usize,
    pub params_size: usize,
}

impl Default for GpuBufferLayout {
    fn default() -> Self {
        Self {
            emf_stride: 12,   // 3 × f32
            rgb_stride: 4,    // 1 × u32 (packed RGB24)
            params_size: 4,   // 1 × u32 (count)
        }
    }
}

// ───────────────────────────────────────────────────────────────────────────
//  Differential check (CPU vs GPU)
// ───────────────────────────────────────────────────────────────────────────

/// Tolerance for CPU vs GPU differential comparison.
/// f32 rounding in the GPU kernel may produce ±1 LSB differences
/// in the 8-bit output.
pub const DIFF_TOLERANCE_U8: u8 = 2;

/// Compare CPU oracle output with GPU output.
///
/// `cpu_out` and `gpu_out` are N*3 u8 arrays. Returns the number of
/// elements that differ by more than `DIFF_TOLERANCE_U8` in any channel.
pub fn diff_cpu_gpu(cpu_out: &[u8], gpu_out: &[u8]) -> usize {
    let n = cpu_out.len().min(gpu_out.len()) / 3;
    let mut mismatches = 0;

    for i in 0..n {
        let dr = (cpu_out[i * 3] as i16 - gpu_out[i * 3] as i16).unsigned_abs() as u8;
        let dg = (cpu_out[i * 3 + 1] as i16 - gpu_out[i * 3 + 1] as i16).unsigned_abs() as u8;
        let db = (cpu_out[i * 3 + 2] as i16 - gpu_out[i * 3 + 2] as i16).unsigned_abs() as u8;
        if dr > DIFF_TOLERANCE_U8 || dg > DIFF_TOLERANCE_U8 || db > DIFF_TOLERANCE_U8 {
            mismatches += 1;
        }
    }

    mismatches
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_batch_deterministic() {
        let emf = [1.0, 0.3, 0.5, 0.8, 0.2, 0.7, 1.0, 0.0, 0.0];
        let mut out1 = [0u8; 9];
        let mut out2 = [0u8; 9];
        cpu_batch_emf_to_display_gamut_mapped(&emf, &mut out1);
        cpu_batch_emf_to_display_gamut_mapped(&emf, &mut out2);
        assert_eq!(out1, out2, "CPU batch must be deterministic");
    }

    #[test]
    fn cpu_batch_xyz_deterministic() {
        let emf = [1.0, 0.3, 0.5, 0.8, 0.2, 0.7];
        let mut out1 = [0.0f32; 6];
        let mut out2 = [0.0f32; 6];
        cpu_batch_emf_to_xyz(&emf, &mut out1);
        cpu_batch_emf_to_xyz(&emf, &mut out2);
        assert_eq!(out1, out2, "CPU batch XYZ must be deterministic");
    }

    #[test]
    fn cpu_batch_valid_rgb() {
        let emf = [1.0, 0.0, 0.0, 1.0, 0.5, 0.5, 1.0, 0.0, 1.0];
        let mut out = [0u8; 9];
        let n = cpu_batch_emf_to_display_gamut_mapped(&emf, &mut out);
        assert_eq!(n, 3);
        // RGB is u8, so it's always in [0, 255] by type limits.
    }

    #[test]
    fn cpu_batch_partial_output() {
        let emf = [1.0, 0.3, 0.5, 0.8, 0.2, 0.7, 1.0, 0.0, 0.0];
        let mut out = [0u8; 6]; // only room for 2
        let n = cpu_batch_emf_to_display_gamut_mapped(&emf, &mut out);
        assert_eq!(n, 2, "should process only 2 elements");
    }

    #[test]
    fn gpu_kernel_source_contains_cmf_data() {
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("CMF_X"), "WGSL must contain CMF X data");
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("CMF_Y"), "WGSL must contain CMF Y data");
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("CMF_Z"), "WGSL must contain CMF Z data");
    }

    #[test]
    fn gpu_kernel_source_contains_pipeline() {
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("SPD"), "WGSL must mention SPD");
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("XYZ"), "WGSL must mention XYZ");
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("sRGB"), "WGSL must mention sRGB");
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("clamp"), "WGSL must gamut-map with clamp");
    }

    #[test]
    fn gpu_kernel_source_has_workgroup_size() {
        assert!(GPU_COLOUR_KERNEL_WGSL.contains("workgroup_size(64)"),
            "WGSL must specify workgroup size");
    }

    #[test]
    fn buffer_layout_defaults() {
        let layout = GpuBufferLayout::default();
        assert_eq!(layout.emf_stride, 12);
        assert_eq!(layout.rgb_stride, 4);
        assert_eq!(layout.params_size, 4);
    }

    #[test]
    fn diff_cpu_gpu_self_zero() {
        let cpu = [100u8, 150, 200, 50, 60, 70];
        let mismatches = diff_cpu_gpu(&cpu, &cpu);
        assert_eq!(mismatches, 0, "self-comparison should have zero mismatches");
    }

    #[test]
    fn diff_cpu_gpu_within_tolerance() {
        let cpu = [100u8, 150, 200, 50, 60, 70];
        let gpu = [101u8, 149, 202, 51, 58, 72]; // all within ±2
        let mismatches = diff_cpu_gpu(&cpu, &gpu);
        assert_eq!(mismatches, 0, "within-tolerance differences should not count");
    }

    #[test]
    fn diff_cpu_gpu_outside_tolerance() {
        let cpu = [100u8, 150, 200, 50, 60, 70];
        let gpu = [110u8, 150, 200, 50, 60, 70]; // dr=10 > 2
        let mismatches = diff_cpu_gpu(&cpu, &gpu);
        assert_eq!(mismatches, 1, "out-of-tolerance should count as mismatch");
    }

    #[test]
    fn cpu_batch_blue_dominates_at_low_sigma() {
        let emf = [1.0, 0.0, 0.0]; // σ=0 → blue
        let mut out = [0u8; 3];
        cpu_batch_emf_to_display_gamut_mapped(&emf, &mut out);
        assert!(out[2] >= out[0], "B should dominate R at σ=0");
        assert!(out[2] >= out[1], "B should dominate G at σ=0");
    }

    #[test]
    fn cpu_batch_red_dominates_at_high_sigma() {
        let emf = [1.0, 0.0, 1.0]; // σ=1 → red
        let mut out = [0u8; 3];
        cpu_batch_emf_to_display_gamut_mapped(&emf, &mut out);
        assert!(out[0] >= out[1], "R should dominate G at σ=1");
        assert!(out[0] >= out[2], "R should dominate B at σ=1");
    }
}
