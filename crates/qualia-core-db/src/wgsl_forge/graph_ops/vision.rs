//! Vision op-node kernels (V3b): Pool2D, Resize2D, Conv2D — WGSL + CPU oracles.
//!
//! CPU oracles match `qualia_vision::ops` numerical contracts (NCHW f32).
//! Differential certification runs when a GPU adapter is available.

#[cfg(test)]
use crate::wgsl_forge::validate::validate_wgsl;
use crate::wgsl_forge::ForgeError;

pub const POOL2D_ENTRY: &str = "pool2d_main";
pub const RESIZE2D_ENTRY: &str = "resize2d_main";
pub const CONV2D_ENTRY: &str = "conv2d_main";

/// Max-pool 2D WGSL. Bindings: input(0), output(1), params(2) =
/// `[c, h, w, kh, kw, stride_h, stride_w, h_out, w_out, 0, 0, 0]`.
pub fn max_pool2d_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let idx = gid.x;
    let c = params[0];
    let h = params[1];
    let w = params[2];
    let kh = params[3];
    let kw = params[4];
    let sh = params[5];
    let sw = params[6];
    let ho = params[7];
    let wo = params[8];
    let n_out = c * ho * wo;
    if (idx >= n_out) {{ return; }}
    let ch = idx / (ho * wo);
    let rem = idx % (ho * wo);
    let oh = rem / wo;
    let ow = rem % wo;
    var acc = -3.402823e+38;
    for (var ky: u32 = 0u; ky < kh; ky++) {{
        for (var kx: u32 = 0u; kx < kw; kx++) {{
            let ih = oh * sh + ky;
            let iw = ow * sw + kx;
            let v = input[ch * h * w + ih * w + iw];
            acc = max(acc, v);
        }}
    }}
    output[idx] = acc;
}}
"#,
        entry = POOL2D_ENTRY,
    )
}

/// Nearest resize NCHW. params: `[c, h_in, w_in, h_out, w_out, 0, 0, 0]`.
pub fn resize2d_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read_write> output: array<f32>;
@group(0) @binding(2) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let idx = gid.x;
    let c = params[0];
    let hi = params[1];
    let wi = params[2];
    let ho = params[3];
    let wo = params[4];
    let n_out = c * ho * wo;
    if (idx >= n_out) {{ return; }}
    let ch = idx / (ho * wo);
    let rem = idx % (ho * wo);
    let oy = rem / wo;
    let ox = rem % wo;
    let iy = (oy * hi) / ho;
    let ix = (ox * wi) / wo;
    output[idx] = input[ch * hi * wi + iy * wi + ix];
}}
"#,
        entry = RESIZE2D_ENTRY,
    )
}

/// Conv2D NCHW f32. Bindings: input(0), weight(1), bias(2), output(3), params(4).
/// params: `[c_in, c_out, h, w, kh, kw, sh, sw, ph, pw, h_out, w_out]`.
/// bias may be zero-length logical (all zeros buffer of size c_out).
pub fn conv2d_wgsl(wg: u32) -> String {
    format!(
        r#"@group(0) @binding(0) var<storage, read> input: array<f32>;
@group(0) @binding(1) var<storage, read> weight: array<f32>;
@group(0) @binding(2) var<storage, read> bias: array<f32>;
@group(0) @binding(3) var<storage, read_write> output: array<f32>;
@group(0) @binding(4) var<storage, read> params: array<u32>;
@compute @workgroup_size({wg})
fn {entry}(@builtin(global_invocation_id) gid: vec3<u32>) {{
    let idx = gid.x;
    let c_in = params[0];
    let c_out = params[1];
    let h = params[2];
    let w = params[3];
    let kh = params[4];
    let kw = params[5];
    let sh = params[6];
    let sw = params[7];
    let ph = params[8];
    let pw = params[9];
    let ho = params[10];
    let wo = params[11];
    let n_out = c_out * ho * wo;
    if (idx >= n_out) {{ return; }}
    let oc = idx / (ho * wo);
    let rem = idx % (ho * wo);
    let oh = rem / wo;
    let ow = rem % wo;
    var acc = bias[oc];
    for (var ic: u32 = 0u; ic < c_in; ic++) {{
        for (var ky: u32 = 0u; ky < kh; ky++) {{
            for (var kx: u32 = 0u; kx < kw; kx++) {{
                let ih_p = oh * sh + ky;
                let iw_p = ow * sw + kx;
                if (ih_p < ph || iw_p < pw) {{ continue; }}
                let ih = ih_p - ph;
                let iw = iw_p - pw;
                if (ih >= h || iw >= w) {{ continue; }}
                let iv = input[ic * h * w + ih * w + iw];
                let wv = weight[oc * (c_in * kh * kw) + ic * (kh * kw) + ky * kw + kx];
                acc = acc + iv * wv;
            }}
        }}
    }}
    output[idx] = acc;
}}
"#,
        entry = CONV2D_ENTRY,
    )
}

// ── CPU oracles (same math as qualia_vision::ops) ────────────────────────────

pub fn max_pool2d_cpu(
    input: &[f32],
    c: usize,
    h: usize,
    w: usize,
    kh: usize,
    kw: usize,
    sh: usize,
    sw: usize,
) -> Result<Vec<f32>, ForgeError> {
    let ho = (h - kh) / sh + 1;
    let wo = (w - kw) / sw + 1;
    let mut out = vec![0.0f32; c * ho * wo];
    for ch in 0..c {
        for oh in 0..ho {
            for ow in 0..wo {
                let mut acc = f32::NEG_INFINITY;
                for ky in 0..kh {
                    for kx in 0..kw {
                        let ih = oh * sh + ky;
                        let iw = ow * sw + kx;
                        acc = acc.max(input[ch * h * w + ih * w + iw]);
                    }
                }
                out[ch * ho * wo + oh * wo + ow] = acc;
            }
        }
    }
    Ok(out)
}

pub fn resize2d_cpu(
    input: &[f32],
    c: usize,
    hi: usize,
    wi: usize,
    ho: usize,
    wo: usize,
) -> Result<Vec<f32>, ForgeError> {
    let mut out = vec![0.0f32; c * ho * wo];
    for ch in 0..c {
        for oy in 0..ho {
            let iy = (oy * hi) / ho;
            for ox in 0..wo {
                let ix = (ox * wi) / wo;
                out[ch * ho * wo + oy * wo + ox] = input[ch * hi * wi + iy * wi + ix];
            }
        }
    }
    Ok(out)
}

#[allow(clippy::too_many_arguments)]
pub fn conv2d_cpu(
    input: &[f32],
    c_in: usize,
    h: usize,
    w: usize,
    weight: &[f32],
    c_out: usize,
    kh: usize,
    kw: usize,
    bias: &[f32],
    sh: usize,
    sw: usize,
    ph: usize,
    pw: usize,
) -> Result<Vec<f32>, ForgeError> {
    let ho = (h + 2 * ph - kh) / sh + 1;
    let wo = (w + 2 * pw - kw) / sw + 1;
    let mut out = vec![0.0f32; c_out * ho * wo];
    for oc in 0..c_out {
        for oh in 0..ho {
            for ow in 0..wo {
                let mut acc = if bias.is_empty() { 0.0 } else { bias[oc] };
                for ic in 0..c_in {
                    for ky in 0..kh {
                        for kx in 0..kw {
                            let ih_p = oh * sh + ky;
                            let iw_p = ow * sw + kx;
                            if ih_p < ph || iw_p < pw {
                                continue;
                            }
                            let ih = ih_p - ph;
                            let iw = iw_p - pw;
                            if ih >= h || iw >= w {
                                continue;
                            }
                            let iv = input[ic * h * w + ih * w + iw];
                            let wv =
                                weight[oc * (c_in * kh * kw) + ic * (kh * kw) + ky * kw + kx];
                            acc += iv * wv;
                        }
                    }
                }
                out[oc * ho * wo + oh * wo + ow] = acc;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgsl_validates() {
        for src in [max_pool2d_wgsl(64), resize2d_wgsl(64), conv2d_wgsl(64)] {
            validate_wgsl(&src).expect("naga");
        }
    }

    #[test]
    fn pool_cpu_matches_identity_case() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let out = max_pool2d_cpu(&input, 1, 2, 2, 2, 2, 2, 2).unwrap();
        assert_eq!(out, vec![4.0]);
    }

    #[test]
    fn conv_identity_1x1() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let w = [1.0f32];
        let out = conv2d_cpu(&input, 1, 2, 2, &w, 1, 1, 1, &[], 1, 1, 0, 0).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn resize_cpu() {
        let input = [1.0f32, 2.0, 3.0, 4.0];
        let out = resize2d_cpu(&input, 1, 2, 2, 4, 4).unwrap();
        assert_eq!(out[0], 1.0);
        assert_eq!(out[15], 4.0);
    }
}
