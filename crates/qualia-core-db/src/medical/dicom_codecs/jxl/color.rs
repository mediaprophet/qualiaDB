//! ISO/IEC 18181-1 JPEG-XL XYB -> LMS -> Linear sRGB inverse color space,
//! sRGB/PQ/HLG transfer functions, YCbCr -> RGB inverse, and clamped pixel packing.
use super::container::{JxlError, JxlImageHeader, JxlTransferFunction};

#[allow(dead_code)]
pub const DEFAULT_XYB_CBRT_BIAS: f64 = 0.003_793_073_255_275_449_3;

/// Exact inverse of XYB cube-root opponency and `OpsinInverseMatrix` (ISO 18181-1 G.3).
pub fn xyb_to_linear_srgb(
    x: f64,
    y: f64,
    b: f64,
    inv_opsin: &[[f64; 3]; 3],
    bias: &[f64; 3],
) -> [f64; 3] {
    let l_g = y + x;
    let m_g = y - x;
    let s_g = b;
    let c0 = bias[0].abs().cbrt().copysign(bias[0]);
    let c1 = bias[1].abs().cbrt().copysign(bias[1]);
    let c2 = bias[2].abs().cbrt().copysign(bias[2]);
    let l = (l_g + c0).powi(3) - bias[0];
    let m = (m_g + c1).powi(3) - bias[1];
    let s = (s_g + c2).powi(3) - bias[2];
    [
        inv_opsin[0][0] * l + inv_opsin[0][1] * m + inv_opsin[0][2] * s,
        inv_opsin[1][0] * l + inv_opsin[1][1] * m + inv_opsin[1][2] * s,
        inv_opsin[2][0] * l + inv_opsin[2][1] * m + inv_opsin[2][2] * s,
    ]
}

/// Forward sRGB transfer curve `[0, 1] -> [0, 1]` from linear light.
pub fn linear_to_transfer(v: f64, tf: JxlTransferFunction) -> f64 {
    let x = v.clamp(0.0, 1.0);
    match tf {
        JxlTransferFunction::Linear => x,
        JxlTransferFunction::Srgb => {
            if x <= 0.0031308 {
                12.92 * x
            } else {
                1.055 * x.powf(1.0 / 2.4) - 0.055
            }
        }
        JxlTransferFunction::Pq => {
            let m1 = 2610.0 / 16384.0;
            let m2 = 2523.0 / 4096.0 * 128.0;
            let c1 = 3424.0 / 4096.0;
            let c2 = 2413.0 / 4096.0 * 32.0;
            let c3 = 2392.0 / 4096.0 * 32.0;
            let ym1 = x.powf(m1);
            ((c1 + c2 * ym1) / (1.0 + c3 * ym1)).powf(m2)
        }
        JxlTransferFunction::Hlg => {
            let a = 0.17883277;
            let b = 0.28466892;
            let c = 0.55991073;
            if x <= 1.0 / 12.0 {
                (3.0 * x).sqrt()
            } else {
                a * (12.0 * x - b).ln() + c
            }
        }
        JxlTransferFunction::Gamma(g) => {
            let gamma = (g as f64) / 10_000_000.0;
            if gamma > 0.0 {
                x.powf(gamma)
            } else {
                x
            }
        }
    }
}

/// Exact inverse of floating-point YCbCr (ISO 18181-1) when frame `do_ycbcr == true`.
#[allow(dead_code)]
pub fn ycbcr_to_rgb(y: f64, cb: f64, cr: f64) -> [f64; 3] {
    [
        y + 1.402 * cr,
        y - 0.34413628620102214 * cb - 0.7141362862010221 * cr,
        y + 1.772 * cb,
    ]
}

/// Convert decoded color planes (`[c][y*w + x]` either integer-scaled Modular or linear/XYB f64 VarDCT)
/// into caller DICOM `output` (`bits_allocated = 8` or `16`, little-endian u16 when `> 8`).
pub fn pack_jxl_planes_into_output(
    hdr: &JxlImageHeader,
    planes: &[Vec<f64>],
    is_integer_modular: bool,
    expected_nc: u16,
    expected_ba: u16,
    expected_bs: u16,
    out: &mut [u8],
) -> Result<(), JxlError> {
    let w = hdr.width as usize;
    let h = hdr.height as usize;
    let npix = w.checked_mul(h).ok_or(JxlError::Overflow)?;
    let nc = expected_nc as usize;
    if planes.len() < nc || planes[..nc].iter().any(|p| p.len() != npix) {
        return Err(JxlError::DimensionMismatch);
    }
    let bps = if expected_ba <= 8 { 1usize } else { 2usize };
    let need = npix.checked_mul(nc).and_then(|v| v.checked_mul(bps)).ok_or(JxlError::Overflow)?;
    if out.len() < need {
        return Err(JxlError::OutputTooSmall);
    }
    let max_val = ((1u32 << expected_bs) - 1) as f64;
    let mod_max = ((1u32 << hdr.bits_per_sample.min(16)) - 1).max(1) as f64;
    let scale = max_val / mod_max;

    for i in 0..npix {
        let mut samples = [0.0f64; 4];
        if hdr.xyb_encoded && !is_integer_modular && nc >= 3 {
            let lin = xyb_to_linear_srgb(
                planes[0][i],
                planes[1][i],
                planes[2][i],
                &hdr.opsin_inverse_matrix,
                &hdr.opsin_bias,
            );
            samples[0] = linear_to_transfer(lin[0], hdr.transfer) * max_val;
            samples[1] = linear_to_transfer(lin[1], hdr.transfer) * max_val;
            samples[2] = linear_to_transfer(lin[2], hdr.transfer) * max_val;
            if nc == 4 {
                samples[3] = planes[3][i] * max_val;
            }
        } else if hdr.xyb_encoded && !is_integer_modular && nc == 1 {
            let lin = xyb_to_linear_srgb(
                0.0,
                planes[1][i],
                planes[2][i],
                &hdr.opsin_inverse_matrix,
                &hdr.opsin_bias,
            );
            let luma = 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2];
            samples[0] = linear_to_transfer(luma, hdr.transfer) * max_val;
        } else {
            for c in 0..nc.min(4) {
                samples[c] = if is_integer_modular {
                    planes[c][i] * scale
                } else {
                    planes[c][i] * max_val
                };
            }
        }

        for c in 0..nc {
            let q = samples[c].round().clamp(0.0, max_val) as u16;
            let idx = (i * nc + c) * bps;
            if bps == 1 {
                out[idx] = q as u8;
            } else {
                let le = q.to_le_bytes();
                out[idx] = le[0];
                out[idx + 1] = le[1];
            }
        }
    }
    Ok(())
}
