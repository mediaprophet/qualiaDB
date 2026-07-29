//! Tier: **SwinRestore** — SwinIR transformer SR (heavy, tiled for large stills).
//!
//! Permissive weight source (gated ①, see `docs/plans/gated-av-jobs-2026.md` §2,
//! row "SR SwinIR-heavy"): **SwinIR** (MIT/Apache), converted to ONNX, dropped
//! into `vendor/vision/sr/swinir/`. Large stills are processed tiled by the
//! existing SR tiling runtime; this adapter runs one tile/frame per call.
//!
//! Pipeline: SwinIR is a full **RGB** restorer. We normalise RGB8 → `[0,1]`
//! planar `1×3×H×W`, run the ONNX model, and denormalise the
//! `1×3×(H·s)×(W·s)` output to RGB8 into `out`.
//!
//! This adapter targets the **restoration / `Sharpen`** SwinIR variants and is
//! therefore reported non-generative (`generative = false`): it reconstructs
//! detail without hallucinating texture, keeping it usable for the honest
//! (medical/forensic) path. (SwinIR also has GAN "real-world" variants — those
//! would be wired as a distinct generative tier, not here.)
//!
//! * `#[cfg(not(feature = "vision-onnx"))]` → **fail closed** (`FeatureDisabled`).
//! * `#[cfg(feature = "vision-onnx")]` → real `ort` inference.
//!
//! No `unwrap()` outside `#[cfg(test)]`.

use std::path::Path;

use super::onnx_sr_session::SrSessionError;
use super::super_resolve::SrReport;
use crate::specialized_libs::computer_vision::cv::buffer::RgbView;

/// Backend id reported for this tier.
pub const BACKEND_ID: &str = "swinir.onnx";

/// Super-resolve `rgb` by `scale` using a SwinIR (restoration) ONNX weight.
///
/// `out` must be at least `width*scale * height*scale * 3` bytes.
#[cfg(not(feature = "vision-onnx"))]
pub fn swin_super_resolve(
    weight_path: &Path,
    rgb: RgbView<'_>,
    scale: u8,
    out: &mut [u8],
) -> Result<SrReport, SrSessionError> {
    // Fail closed: no runtime → no fabricated image.
    let _ = (weight_path, rgb, scale, out);
    Err(SrSessionError::FeatureDisabled)
}

#[cfg(feature = "vision-onnx")]
pub fn swin_super_resolve(
    weight_path: &Path,
    rgb: RgbView<'_>,
    scale: u8,
    out: &mut [u8],
) -> Result<SrReport, SrSessionError> {
    use super::onnx_sr_session::{
        load_sr_session, planar01_to_rgb8, probe_sr_asset, rgb_to_planar01, validate_geometry,
    };

    let (out_w, out_h) = validate_geometry(rgb, scale, out.len())?;

    let _ = probe_sr_asset(weight_path, "swinir.onnx")?;
    let mut session = load_sr_session(weight_path)?;

    // Preprocess: RGB8 → [0,1] planar 1×3×H×W.
    let input = rgb_to_planar01(rgb);
    let tensor = {
        use ort::value::Tensor;
        let shape = vec![1i64, 3, rgb.height as i64, rgb.width as i64];
        Tensor::from_array((shape, input.into_boxed_slice()))
            .map_err(|_| SrSessionError::LoadFailed)?
    };

    let outputs = session
        .run(ort::inputs![tensor])
        .map_err(|_| SrSessionError::LoadFailed)?;
    let first = outputs.iter().next().ok_or(SrSessionError::BadOutput)?;
    let (_name, value) = first;
    let (oshape, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|_| SrSessionError::BadOutput)?;

    // Postprocess: planar 1×3×out_h×out_w [0,1] → RGB8 interleaved.
    planar01_to_rgb8(data, out_w, out_h, out)?;
    let _ = oshape;

    Ok(SrReport {
        backend_id: "swinir.onnx",
        device: "ort",
        scale,
        out_width: out_w,
        out_height: out_h,
        generative: false,
        tile_count: 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view_2x2(buf: &[u8]) -> RgbView<'_> {
        RgbView::new(2, 2, 6, buf).expect("valid 2x2 view")
    }

    #[test]
    fn swin_fails_closed_without_feature() {
        let buf = [255u8, 0, 0, 0, 255, 0, 0, 0, 255, 128, 128, 128];
        let view = view_2x2(&buf);
        let mut out = [0u8; 4 * 4 * 3];
        let p = Path::new("does-not-exist-swin.onnx");
        let r = swin_super_resolve(p, view, 2, &mut out);
        #[cfg(not(feature = "vision-onnx"))]
        assert_eq!(r, Err(SrSessionError::FeatureDisabled));
        #[cfg(feature = "vision-onnx")]
        assert_eq!(r, Err(SrSessionError::WeightAbsent));
        assert!(out.iter().all(|&c| c == 0));
    }
}
