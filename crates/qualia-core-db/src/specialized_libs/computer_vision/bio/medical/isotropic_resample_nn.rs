//! Nearest-neighbour resample of anisotropic 2D/3D grids to isotropic spacing.

use super::hu_window::MedicalError;

/// Resample a 2D image from anisotropic pixel spacing to isotropic.
///
/// `in_spacing = (sx, sy)` physical size of input pixels.
/// Target isotropic spacing is `target_spacing` (same units).
/// Output dimensions are chosen so physical extent is preserved:
///   `out_w = round(in_w * sx / target)`, etc.
///
/// Layout: row-major `in_w * in_h`. Returns `(out_w, out_h)`.
pub fn isotropic_resample_2d_nn(
    input: &[f32],
    in_w: usize,
    in_h: usize,
    in_spacing: (f64, f64),
    target_spacing: f64,
    out: &mut [f32],
) -> Result<(usize, usize), MedicalError> {
    if in_w == 0 || in_h == 0 || input.len() < in_w * in_h {
        return Err(MedicalError::DimensionMismatch);
    }
    let (sx, sy) = in_spacing;
    if sx <= 0.0 || sy <= 0.0 || target_spacing <= 0.0 {
        return Err(MedicalError::InvalidParameter);
    }

    let out_w = ((in_w as f64 * sx / target_spacing).round() as usize).max(1);
    let out_h = ((in_h as f64 * sy / target_spacing).round() as usize).max(1);
    if out.len() < out_w * out_h {
        return Err(MedicalError::BufferTooSmall);
    }

    for oy in 0..out_h {
        // Map output physical center → input index
        let py = (oy as f64 + 0.5) * target_spacing;
        let iy = ((py / sy) - 0.5).round().clamp(0.0, (in_h - 1) as f64) as usize;
        for ox in 0..out_w {
            let px = (ox as f64 + 0.5) * target_spacing;
            let ix = ((px / sx) - 0.5).round().clamp(0.0, (in_w - 1) as f64) as usize;
            out[oy * out_w + ox] = input[iy * in_w + ix];
        }
    }
    Ok((out_w, out_h))
}

/// Resample a 3D volume (stacked slices) to isotropic voxel spacing.
///
/// Layout: `voxels[z * in_h * in_w + y * in_w + x]`.
/// `in_spacing = (sx, sy, sz)`. Returns `(out_w, out_h, out_d)`.
pub fn isotropic_resample_3d_nn(
    input: &[f32],
    in_w: usize,
    in_h: usize,
    in_d: usize,
    in_spacing: (f64, f64, f64),
    target_spacing: f64,
    out: &mut [f32],
) -> Result<(usize, usize, usize), MedicalError> {
    if in_w == 0 || in_h == 0 || in_d == 0 {
        return Err(MedicalError::InvalidParameter);
    }
    let expected = in_w.saturating_mul(in_h).saturating_mul(in_d);
    if input.len() < expected {
        return Err(MedicalError::DimensionMismatch);
    }
    let (sx, sy, sz) = in_spacing;
    if sx <= 0.0 || sy <= 0.0 || sz <= 0.0 || target_spacing <= 0.0 {
        return Err(MedicalError::InvalidParameter);
    }

    let out_w = ((in_w as f64 * sx / target_spacing).round() as usize).max(1);
    let out_h = ((in_h as f64 * sy / target_spacing).round() as usize).max(1);
    let out_d = ((in_d as f64 * sz / target_spacing).round() as usize).max(1);
    let out_len = out_w * out_h * out_d;
    if out.len() < out_len {
        return Err(MedicalError::BufferTooSmall);
    }

    for oz in 0..out_d {
        let pz = (oz as f64 + 0.5) * target_spacing;
        let iz = ((pz / sz) - 0.5).round().clamp(0.0, (in_d - 1) as f64) as usize;
        for oy in 0..out_h {
            let py = (oy as f64 + 0.5) * target_spacing;
            let iy = ((py / sy) - 0.5).round().clamp(0.0, (in_h - 1) as f64) as usize;
            for ox in 0..out_w {
                let px = (ox as f64 + 0.5) * target_spacing;
                let ix = ((px / sx) - 0.5).round().clamp(0.0, (in_w - 1) as f64) as usize;
                out[oz * out_h * out_w + oy * out_w + ox] =
                    input[iz * in_h * in_w + iy * in_w + ix];
            }
        }
    }
    Ok((out_w, out_h, out_d))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_isotropic_2d_identity_dims() {
        let img = [1.0f32, 2.0, 3.0, 4.0];
        let mut out = [0.0f32; 4];
        let (ow, oh) = isotropic_resample_2d_nn(&img, 2, 2, (1.0, 1.0), 1.0, &mut out).unwrap();
        assert_eq!((ow, oh), (2, 2));
        assert_eq!(out, img);
    }

    #[test]
    fn stretch_x_spacing() {
        // 2×1 image, sx=2, sy=1 → isotropic 1 → out_w=4, out_h=1
        let img = [10.0f32, 20.0];
        let mut out = [0.0f32; 8];
        let (ow, oh) = isotropic_resample_2d_nn(&img, 2, 1, (2.0, 1.0), 1.0, &mut out).unwrap();
        assert_eq!((ow, oh), (4, 1));
        // First half ≈ 10, second ≈ 20
        assert!((out[0] - 10.0).abs() < 1e-6);
        assert!((out[3] - 20.0).abs() < 1e-6);
    }

    #[test]
    fn volume_z_anisotropic() {
        // 1×1×2, sz=2 → isotropic 1 → depth 4
        let vol = [1.0f32, 9.0];
        let mut out = [0.0f32; 8];
        let (ow, oh, od) =
            isotropic_resample_3d_nn(&vol, 1, 1, 2, (1.0, 1.0, 2.0), 1.0, &mut out).unwrap();
        assert_eq!((ow, oh, od), (1, 1, 4));
        assert!((out[0] - 1.0).abs() < 1e-6);
        assert!((out[3] - 9.0).abs() < 1e-6);
    }

    #[test]
    fn bad_spacing() {
        let img = [1.0f32];
        let mut out = [0.0f32; 1];
        assert_eq!(
            isotropic_resample_2d_nn(&img, 1, 1, (0.0, 1.0), 1.0, &mut out).unwrap_err(),
            MedicalError::InvalidParameter
        );
    }
}
