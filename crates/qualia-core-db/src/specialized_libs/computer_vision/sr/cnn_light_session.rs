//! Tier: **CnnLight** — lightweight learned SR (FSRCNN / ESPCN).
//!
//! Permissive weight source (gated ①, see `docs/plans/gated-av-jobs-2026.md` §2,
//! row "SR CNN-Light"): **FSRCNN / ESPCN** from the OpenCV zoo (Apache-2.0),
//! dropped into `vendor/vision/sr/{fsrcnn,espcn}/` and converted to ONNX.
//!
//! Pipeline (the classic FSRCNN/ESPCN convention): these networks super-resolve
//! only the **luma (Y)** channel. We convert RGB8 → YCbCr (BT.601), run the ONNX
//! model on a single-channel `1×1×H×W` Y tensor normalised to `[0,1]`, upscale
//! the chroma (Cb/Cr) classically with bilinear interpolation, then recombine
//! YCbCr → RGB8 into `out`. This is non-generative (`generative = false`): it
//! recovers luminance detail without hallucinating texture, which is why it is
//! the honest `Sharpen`-class tier for medical/forensic use.
//!
//! * `#[cfg(not(feature = "vision-onnx"))]` → **fail closed** (`FeatureDisabled`);
//!   never invents pixels.
//! * `#[cfg(feature = "vision-onnx")]` → real `ort` inference.
//!
//! No `unwrap()` outside `#[cfg(test)]`.

use std::path::Path;

use super::onnx_sr_session::SrSessionError;
use super::super_resolve::SrReport;
use crate::specialized_libs::computer_vision::cv::buffer::RgbView;

/// Backend id reported for this tier.
pub const BACKEND_ID: &str = "cnn_light.onnx";

/// Super-resolve `rgb` by `scale` using a CnnLight (FSRCNN/ESPCN) ONNX weight.
///
/// `out` must be at least `width*scale * height*scale * 3` bytes.
#[cfg(not(feature = "vision-onnx"))]
pub fn cnn_light_super_resolve(
    weight_path: &Path,
    rgb: RgbView<'_>,
    scale: u8,
    out: &mut [u8],
) -> Result<SrReport, SrSessionError> {
    // Fail closed: without the runtime we do NOT fall back to a fabricated image.
    let _ = (weight_path, rgb, scale, out);
    Err(SrSessionError::FeatureDisabled)
}

#[cfg(feature = "vision-onnx")]
pub fn cnn_light_super_resolve(
    weight_path: &Path,
    rgb: RgbView<'_>,
    scale: u8,
    out: &mut [u8],
) -> Result<SrReport, SrSessionError> {
    use super::onnx_sr_session::{load_sr_session, probe_sr_asset};
    use ort::value::Tensor;

    if scale < 2 || scale > 4 {
        return Err(SrSessionError::InvalidParameter);
    }
    let w = rgb.width;
    let h = rgb.height;
    if w == 0 || h == 0 {
        return Err(SrSessionError::InvalidParameter);
    }
    let out_w = w
        .checked_mul(scale as u32)
        .ok_or(SrSessionError::InvalidParameter)?;
    let out_h = h
        .checked_mul(scale as u32)
        .ok_or(SrSessionError::InvalidParameter)?;
    let need = (out_w as usize)
        .checked_mul(out_h as usize)
        .and_then(|n| n.checked_mul(3))
        .ok_or(SrSessionError::InvalidParameter)?;
    if out.len() < need {
        return Err(SrSessionError::InvalidParameter);
    }

    // Probe (WeightAbsent if missing) then build the session.
    let _ = probe_sr_asset(weight_path, "cnn_light.onnx")?;
    let mut session = load_sr_session(weight_path)?;

    // Preprocess: RGB8 → Y (luma, [0,1]) planar 1×1×H×W; keep Cb/Cr for later.
    let (yin, cb, cr) = rgb_to_ycbcr(rgb);
    let shape = vec![1i64, 1, h as i64, w as i64];
    let tensor = Tensor::from_array((shape, yin.into_boxed_slice()))
        .map_err(|_| SrSessionError::LoadFailed)?;

    let outputs = session
        .run(ort::inputs![tensor])
        .map_err(|_| SrSessionError::LoadFailed)?;
    let first = outputs.iter().next().ok_or(SrSessionError::BadOutput)?;
    let (_name, value) = first;
    let (oshape, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|_| SrSessionError::BadOutput)?;

    // Expect a single-channel super-resolved luma plane of out_w × out_h.
    let expect = (out_w as usize)
        .checked_mul(out_h as usize)
        .ok_or(SrSessionError::BadOutput)?;
    if data.len() < expect {
        return Err(SrSessionError::BadOutput);
    }
    let _ = oshape;

    // Chroma: classical bilinear upscale of the low-res Cb/Cr planes.
    // Recombine Y'(model) + Cb'/Cr'(bilinear) → RGB8.
    for oy in 0..out_h {
        for ox in 0..out_w {
            let y = data[(oy * out_w + ox) as usize].clamp(0.0, 1.0) * 255.0;
            let (fcb, fcr) = sample_bilinear(&cb, &cr, w, h, ox, oy, scale);
            let (r, g, b) = ycbcr_to_rgb(y, fcb, fcr);
            let doff = ((oy * out_w + ox) * 3) as usize;
            out[doff] = r;
            out[doff + 1] = g;
            out[doff + 2] = b;
        }
    }

    Ok(SrReport {
        backend_id: "cnn_light.onnx",
        device: "ort",
        scale,
        out_width: out_w,
        out_height: out_h,
        generative: false,
        tile_count: 1,
    })
}

/// RGB8 → (Y[0,1] planar, Cb[0,255], Cr[0,255]) using BT.601 full-range.
#[cfg(feature = "vision-onnx")]
fn rgb_to_ycbcr(rgb: RgbView<'_>) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let w = rgb.width as usize;
    let h = rgb.height as usize;
    let mut y = vec![0.0f32; w * h];
    let mut cb = vec![0.0f32; w * h];
    let mut cr = vec![0.0f32; w * h];
    for yy in 0..h {
        for xx in 0..w {
            let (r, g, b) = rgb.pixel(xx as u32, yy as u32);
            let (rf, gf, bf) = (r as f32, g as f32, b as f32);
            let yv = 0.299 * rf + 0.587 * gf + 0.114 * bf;
            let cbv = 128.0 - 0.168736 * rf - 0.331264 * gf + 0.5 * bf;
            let crv = 128.0 + 0.5 * rf - 0.418688 * gf - 0.081312 * bf;
            let i = yy * w + xx;
            y[i] = (yv / 255.0).clamp(0.0, 1.0);
            cb[i] = cbv;
            cr[i] = crv;
        }
    }
    (y, cb, cr)
}

/// Bilinear-sample the low-res Cb/Cr planes at an output pixel.
#[cfg(feature = "vision-onnx")]
fn sample_bilinear(
    cb: &[f32],
    cr: &[f32],
    w: u32,
    h: u32,
    ox: u32,
    oy: u32,
    scale: u8,
) -> (f32, f32) {
    let s = scale as f32;
    // Map output centre back into source coordinates.
    let sx = (ox as f32 + 0.5) / s - 0.5;
    let sy = (oy as f32 + 0.5) / s - 0.5;
    let x0 = sx.floor().max(0.0) as u32;
    let y0 = sy.floor().max(0.0) as u32;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let dx = (sx - x0 as f32).clamp(0.0, 1.0);
    let dy = (sy - y0 as f32).clamp(0.0, 1.0);
    let at = |p: &[f32], x: u32, y: u32| p[(y * w + x) as usize];
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let cb_v = lerp(
        lerp(at(cb, x0, y0), at(cb, x1, y0), dx),
        lerp(at(cb, x0, y1), at(cb, x1, y1), dx),
        dy,
    );
    let cr_v = lerp(
        lerp(at(cr, x0, y0), at(cr, x1, y0), dx),
        lerp(at(cr, x0, y1), at(cr, x1, y1), dx),
        dy,
    );
    (cb_v, cr_v)
}

/// (Y[0,255], Cb[0,255], Cr[0,255]) → RGB8 (BT.601 full-range).
#[cfg(feature = "vision-onnx")]
fn ycbcr_to_rgb(y: f32, cb: f32, cr: f32) -> (u8, u8, u8) {
    let cbc = cb - 128.0;
    let crc = cr - 128.0;
    let r = y + 1.402 * crc;
    let g = y - 0.344136 * cbc - 0.714136 * crc;
    let b = y + 1.772 * cbc;
    (
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view_2x2(buf: &[u8]) -> RgbView<'_> {
        RgbView::new(2, 2, 6, buf).expect("valid 2x2 view")
    }

    #[test]
    fn cnn_light_fails_closed_without_feature() {
        let buf = [255u8, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
        let view = view_2x2(&buf);
        let mut out = [0u8; 4 * 4 * 3];
        let p = Path::new("does-not-exist-cnn.onnx");
        let r = cnn_light_super_resolve(p, view, 2, &mut out);
        #[cfg(not(feature = "vision-onnx"))]
        assert_eq!(r, Err(SrSessionError::FeatureDisabled));
        #[cfg(feature = "vision-onnx")]
        // With the feature the weight is absent → WeightAbsent (still no pixels).
        assert_eq!(r, Err(SrSessionError::WeightAbsent));
        // Output buffer must remain untouched (no fabricated image).
        assert!(out.iter().all(|&c| c == 0));
    }
}
