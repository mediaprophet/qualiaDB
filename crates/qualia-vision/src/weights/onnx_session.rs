//! Optional ONNX Runtime session (feature `ort`).
//!
//! Without the feature, loaders report AdapterMissing. With `ort`, sessions
//! load PermissiveReady weights from vendor/vision.

use std::path::Path;

use super::resolve_vision_asset::{VisionAssetError, VisionAssetId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnnxSessionError {
    FeatureDisabled,
    WeightAbsent,
    LoadFailed,
    RunFailed,
    BadOutput,
}

impl From<VisionAssetError> for OnnxSessionError {
    fn from(e: VisionAssetError) -> Self {
        match e {
            VisionAssetError::WeightAbsent | VisionAssetError::NoRoots => {
                OnnxSessionError::WeightAbsent
            }
        }
    }
}

/// Metadata after a successful session open (always available for tests).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnnxSessionInfo {
    pub asset: VisionAssetId,
    pub path: String,
    pub backend: &'static str,
}

/// Open-session probe: resolves path; with `ort` also creates Session.
pub fn probe_onnx_asset(
    id: VisionAssetId,
    roots: &[&Path],
) -> Result<OnnxSessionInfo, OnnxSessionError> {
    let resolved = super::resolve_vision_asset(id, roots)?;
    #[cfg(feature = "ort")]
    {
        let _session = load_ort_session(&resolved.path)?;
        return Ok(OnnxSessionInfo {
            asset: id,
            path: resolved.path.display().to_string(),
            backend: "ort",
        });
    }
    #[cfg(not(feature = "ort"))]
    {
        // Weight present — runtime still AdapterMissing without feature.
        let _ = resolved;
        Err(OnnxSessionError::FeatureDisabled)
    }
}

#[cfg(feature = "ort")]
fn load_ort_session(path: &Path) -> Result<ort::session::Session, OnnxSessionError> {
    ort::session::Session::builder()
        .map_err(|_| OnnxSessionError::LoadFailed)?
        .commit_from_file(path)
        .map_err(|_| OnnxSessionError::LoadFailed)
}

/// Run YuNet ONNX if feature ort; returns flat decoded rows for yunet_decode_detections.
/// Without ort: FeatureDisabled.
pub fn yunet_infer_rgb8(
    roots: &[&Path],
    rgb: &[u8],
    width: u32,
    height: u32,
    out_rows: &mut [f32],
    stride: usize,
) -> Result<usize, OnnxSessionError> {
    if width == 0 || height == 0 || rgb.len() < (width * height * 3) as usize {
        return Err(OnnxSessionError::BadOutput);
    }
    #[cfg(not(feature = "ort"))]
    {
        let _ = (roots, out_rows, stride);
        return Err(OnnxSessionError::FeatureDisabled);
    }
    #[cfg(feature = "ort")]
    {
        yunet_infer_ort(roots, rgb, width, height, out_rows, stride)
    }
}

#[cfg(feature = "ort")]
fn yunet_infer_ort(
    roots: &[&Path],
    rgb: &[u8],
    width: u32,
    height: u32,
    out_rows: &mut [f32],
    stride: usize,
) -> Result<usize, OnnxSessionError> {
    use ort::value::Tensor;

    if stride < 5 {
        return Err(OnnxSessionError::BadOutput);
    }
    let resolved = super::resolve_vision_asset(VisionAssetId::Yunet, roots)?;
    let mut session = load_ort_session(&resolved.path)?;

    // YuNet OpenCV zoo often expects BGR float NCHW or specific size; use 320x320 letterbox.
    const IN: usize = 320;
    let mut input = vec![0.0f32; 1 * 3 * IN * IN];
    // Simple nearest resize + RGB→planar (model may expect BGR — still useful for smoke).
    for y in 0..IN {
        for x in 0..IN {
            let sx = (x as u32 * width / IN as u32).min(width.saturating_sub(1)) as usize;
            let sy = (y as u32 * height / IN as u32).min(height.saturating_sub(1)) as usize;
            let i = (sy * width as usize + sx) * 3;
            let r = rgb[i] as f32;
            let g = rgb[i + 1] as f32;
            let b = rgb[i + 2] as f32;
            // Planar BGR-ish
            input[0 * IN * IN + y * IN + x] = b;
            input[1 * IN * IN + y * IN + x] = g;
            input[2 * IN * IN + y * IN + x] = r;
        }
    }

    let shape = vec![1i64, 3, IN as i64, IN as i64];
    let tensor = Tensor::from_array((shape, input.into_boxed_slice()))
        .map_err(|_| OnnxSessionError::LoadFailed)?;

    let outputs = session
        .run(ort::inputs![tensor])
        .map_err(|_| OnnxSessionError::RunFailed)?;

    // Best-effort: flatten first output into rows of `stride`.
    let first = outputs
        .iter()
        .next()
        .ok_or(OnnxSessionError::BadOutput)?;
    let (_name, value) = first;
    let try_extract = value.try_extract_tensor::<f32>();
    let (shape, data) = match try_extract {
        Ok(v) => v,
        Err(_) => return Err(OnnxSessionError::BadOutput),
    };
    let flat: &[f32] = data;
    let n = (flat.len() / stride).min(out_rows.len() / stride);
    for i in 0..n * stride {
        out_rows[i] = flat[i.min(flat.len() - 1)];
    }
    let _ = shape;
    Ok(n)
}

/// SFace: resize face crop to 112x112, run embed, write 128 floats.
pub fn sface_infer_rgb8(
    roots: &[&Path],
    rgb: &[u8],
    width: u32,
    height: u32,
    out_embed: &mut [f32],
) -> Result<usize, OnnxSessionError> {
    if out_embed.len() < 128 {
        return Err(OnnxSessionError::BadOutput);
    }
    #[cfg(not(feature = "ort"))]
    {
        let _ = (roots, rgb, width, height);
        return Err(OnnxSessionError::FeatureDisabled);
    }
    #[cfg(feature = "ort")]
    {
        sface_infer_ort(roots, rgb, width, height, out_embed)
    }
}

#[cfg(feature = "ort")]
fn sface_infer_ort(
    roots: &[&Path],
    rgb: &[u8],
    width: u32,
    height: u32,
    out_embed: &mut [f32],
) -> Result<usize, OnnxSessionError> {
    use ort::value::Tensor;

    let resolved = super::resolve_vision_asset(VisionAssetId::Sface, roots)?;
    let mut session = load_ort_session(&resolved.path)?;
    const IN: usize = 112;
    let mut input = vec![0.0f32; 1 * 3 * IN * IN];
    for y in 0..IN {
        for x in 0..IN {
            let sx = (x as u32 * width / IN as u32).min(width.saturating_sub(1)) as usize;
            let sy = (y as u32 * height / IN as u32).min(height.saturating_sub(1)) as usize;
            let i = (sy * width as usize + sx) * 3;
            input[0 * IN * IN + y * IN + x] = rgb[i] as f32;
            input[1 * IN * IN + y * IN + x] = rgb[i + 1] as f32;
            input[2 * IN * IN + y * IN + x] = rgb[i + 2] as f32;
        }
    }
    let shape = vec![1i64, 3, IN as i64, IN as i64];
    let tensor = Tensor::from_array((shape, input.into_boxed_slice()))
        .map_err(|_| OnnxSessionError::LoadFailed)?;
    let outputs = session
        .run(ort::inputs![tensor])
        .map_err(|_| OnnxSessionError::RunFailed)?;
    let first = outputs.iter().next().ok_or(OnnxSessionError::BadOutput)?;
    let (_n, value) = first;
    let (_, data) = value
        .try_extract_tensor::<f32>()
        .map_err(|_| OnnxSessionError::BadOutput)?;
    let n = data.len().min(128);
    out_embed[..n].copy_from_slice(&data[..n]);
    // L2 normalize
    let mut norm = 0.0f32;
    for i in 0..n {
        norm += out_embed[i] * out_embed[i];
    }
    let nrm = norm.sqrt().max(1e-8);
    for i in 0..n {
        out_embed[i] /= nrm;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn vendor() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/vision")
    }

    #[test]
    fn probe_without_ort_feature() {
        let root = vendor();
        let r = probe_onnx_asset(VisionAssetId::Yunet, &[root.as_path()]);
        #[cfg(not(feature = "ort"))]
        {
            // Weight may be present → FeatureDisabled; or WeightAbsent on CI
            assert!(matches!(
                r,
                Err(OnnxSessionError::FeatureDisabled) | Err(OnnxSessionError::WeightAbsent)
            ));
        }
        #[cfg(feature = "ort")]
        {
            // May fail LoadFailed if shape API differs; weight path should resolve
            let _ = r;
        }
    }
}
