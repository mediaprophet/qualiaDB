//! Shared ONNX Runtime session helper for learned super-resolution tiers
//! (feature `vision-onnx`).
//!
//! This mirrors `qualia-vision/src/weights/onnx_session.rs` exactly, but for the
//! SR tier adapters (`cnn_light_session`, `esrgan_session`, `swin_session`).
//!
//! * Without the `vision-onnx` feature the whole learned path **fails closed** —
//!   probing/inference returns `FeatureDisabled` and no pixels are ever invented.
//! * With `vision-onnx`, permissive published SR weights (FSRCNN/ESPCN,
//!   Real-ESRGAN, SwinIR — see `docs/plans/gated-av-jobs-2026.md` §2) become
//!   drop-in: point a tier adapter at the weight file and it loads an
//!   `ort::session::Session` and runs it.
//!
//! No `unwrap()` outside `#[cfg(test)]`.

use std::path::Path;

use crate::specialized_libs::computer_vision::cv::error::CvError;

/// Errors for the learned SR session layer. Mirrors vision's `OnnxSessionError`
/// with an added `InvalidParameter` for bad request geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrSessionError {
    /// The `vision-onnx` feature is not compiled in — learned SR is unavailable.
    FeatureDisabled,
    /// The weight file does not exist on disk.
    WeightAbsent,
    /// ONNX Runtime failed to build a session / tensor from the weight.
    LoadFailed,
    /// The model produced an output whose shape/size did not match expectation.
    BadOutput,
    /// The caller passed invalid geometry (scale, dimensions, buffer size).
    InvalidParameter,
}

impl From<CvError> for SrSessionError {
    fn from(e: CvError) -> Self {
        match e {
            CvError::BufferTooSmall
            | CvError::DimensionMismatch
            | CvError::InvalidParameter
            | CvError::EmptyInput => SrSessionError::InvalidParameter,
        }
    }
}

/// Metadata after a session probe (always available for tests, feature or not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SrSessionInfo {
    /// Tier backend id, e.g. `"cnn_light.onnx"`.
    pub backend: &'static str,
    /// Resolved weight path (present when the file exists).
    pub weight_path: Option<String>,
    /// True only when an `ort::session::Session` was actually created.
    pub session_ready: bool,
}

/// Probe an SR weight file for a tier.
///
/// * If the file is absent → `WeightAbsent`.
/// * Under `vision-onnx` the weight is also loaded into an `ort` session
///   (`LoadFailed` on failure) and `session_ready` is set to `true`.
/// * Under `not(vision-onnx)` → `FeatureDisabled` (fail closed; the weight may
///   be present on disk but the runtime cannot use it).
pub fn probe_sr_asset(path: &Path, backend: &'static str) -> Result<SrSessionInfo, SrSessionError> {
    if !path.exists() {
        return Err(SrSessionError::WeightAbsent);
    }
    #[cfg(feature = "vision-onnx")]
    {
        let _session = load_sr_session(path)?;
        Ok(SrSessionInfo {
            backend,
            weight_path: Some(path.display().to_string()),
            session_ready: true,
        })
    }
    #[cfg(not(feature = "vision-onnx"))]
    {
        // Weight present, but the runtime is unavailable without the feature.
        let _ = backend;
        Err(SrSessionError::FeatureDisabled)
    }
}

/// Build an `ort` session from a weight file. Mirrors vision's `load_ort_session`.
#[cfg(feature = "vision-onnx")]
pub fn load_sr_session(path: &Path) -> Result<ort::session::Session, SrSessionError> {
    ort::session::Session::builder()
        .map_err(|_| SrSessionError::LoadFailed)?
        .commit_from_file(path)
        .map_err(|_| SrSessionError::LoadFailed)
}

// ---------------------------------------------------------------------------
// Shared RGB planar pre/post-processing for the 3-channel tiers (ESRGAN, Swin).
// Gated on the feature so the default build has no dead code.
// ---------------------------------------------------------------------------

#[cfg(feature = "vision-onnx")]
use crate::specialized_libs::computer_vision::cv::buffer::RgbView;

/// Validate request geometry for a 3-channel SR pass and return the output dims.
///
/// * `scale` must be 2..=4.
/// * `out_len` must hold `w*scale * h*scale * 3` bytes.
#[cfg(feature = "vision-onnx")]
pub(crate) fn validate_geometry(
    rgb: RgbView<'_>,
    scale: u8,
    out_len: usize,
) -> Result<(u32, u32), SrSessionError> {
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
    if out_len < need {
        return Err(SrSessionError::InvalidParameter);
    }
    Ok((out_w, out_h))
}

/// RGB8 interleaved → `[0,1]` planar `C×H×W` (R plane, then G, then B).
#[cfg(feature = "vision-onnx")]
pub(crate) fn rgb_to_planar01(rgb: RgbView<'_>) -> Vec<f32> {
    let w = rgb.width as usize;
    let h = rgb.height as usize;
    let plane = w * h;
    let mut out = vec![0.0f32; 3 * plane];
    for yy in 0..h {
        for xx in 0..w {
            let (r, g, b) = rgb.pixel(xx as u32, yy as u32);
            let i = yy * w + xx;
            out[i] = r as f32 / 255.0;
            out[plane + i] = g as f32 / 255.0;
            out[2 * plane + i] = b as f32 / 255.0;
        }
    }
    out
}

/// `[0,1]` planar `3×out_h×out_w` → RGB8 interleaved into `out`.
///
/// `data` must hold at least `3 * out_w * out_h` values or `BadOutput` is returned.
#[cfg(feature = "vision-onnx")]
pub(crate) fn planar01_to_rgb8(
    data: &[f32],
    out_w: u32,
    out_h: u32,
    out: &mut [u8],
) -> Result<(), SrSessionError> {
    let plane = (out_w as usize)
        .checked_mul(out_h as usize)
        .ok_or(SrSessionError::BadOutput)?;
    if data.len() < 3 * plane {
        return Err(SrSessionError::BadOutput);
    }
    if out.len() < 3 * plane {
        return Err(SrSessionError::InvalidParameter);
    }
    for i in 0..plane {
        let r = (data[i].clamp(0.0, 1.0) * 255.0).round() as u8;
        let g = (data[plane + i].clamp(0.0, 1.0) * 255.0).round() as u8;
        let b = (data[2 * plane + i].clamp(0.0, 1.0) * 255.0).round() as u8;
        let o = i * 3;
        out[o] = r;
        out[o + 1] = g;
        out[o + 2] = b;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn nonexistent() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("does-not-exist")
            .join("sr_weight.onnx")
    }

    #[test]
    fn probe_absent_weight_reports_weight_absent_or_feature_disabled() {
        let p = nonexistent();
        let r = probe_sr_asset(&p, "cnn_light.onnx");
        // Absent file → WeightAbsent on every build (checked before the feature
        // branch). We accept FeatureDisabled too, matching vision's tolerance,
        // in case a real weight ever lands at the probe path on a feature build.
        assert!(matches!(
            r,
            Err(SrSessionError::WeightAbsent) | Err(SrSessionError::FeatureDisabled)
        ));
    }

    #[test]
    fn cverror_maps_to_invalid_parameter() {
        assert_eq!(
            SrSessionError::from(CvError::BufferTooSmall),
            SrSessionError::InvalidParameter
        );
    }
}
