//! Max Intensity Projection (MIP) along a volume axis.
//!
//! Volume is flat slices stacked: for axis Z (default clinical MIP), layout is
//! `depth` contiguous `height * width` slices (row-major within slice).

use super::hu_window::MedicalError;

/// Projection axis for a 3D volume stored as stacked 2D slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MipAxis {
    /// Along width (X) — output is `height × depth`.
    X = 0,
    /// Along height (Y) — output is `width × depth`.
    Y = 1,
    /// Along depth (Z) — output is `width × height` (classic axial MIP).
    Z = 2,
}

/// Max-intensity project a 3D volume of `f32` voxels.
///
/// Layout: `voxels[z * height * width + y * width + x]`.
/// `out` must hold the full projected plane (size depends on axis).
///
/// Returns `(out_width, out_height)` of the projection.
pub fn mip_project_axis(
    voxels: &[f32],
    width: usize,
    height: usize,
    depth: usize,
    axis: MipAxis,
    out: &mut [f32],
) -> Result<(usize, usize), MedicalError> {
    if width == 0 || height == 0 || depth == 0 {
        return Err(MedicalError::InvalidParameter);
    }
    let expected = width.saturating_mul(height).saturating_mul(depth);
    if voxels.len() < expected {
        return Err(MedicalError::DimensionMismatch);
    }

    let (ow, oh) = match axis {
        MipAxis::Z => (width, height),
        MipAxis::Y => (width, depth),
        MipAxis::X => (height, depth),
    };
    let out_len = ow * oh;
    if out.len() < out_len {
        return Err(MedicalError::BufferTooSmall);
    }

    // Init to -inf so first sample wins
    for i in 0..out_len {
        out[i] = f32::NEG_INFINITY;
    }

    match axis {
        MipAxis::Z => {
            for z in 0..depth {
                let base = z * height * width;
                for y in 0..height {
                    for x in 0..width {
                        let v = voxels[base + y * width + x];
                        let o = y * width + x;
                        if v > out[o] {
                            out[o] = v;
                        }
                    }
                }
            }
        }
        MipAxis::Y => {
            for z in 0..depth {
                for y in 0..height {
                    for x in 0..width {
                        let v = voxels[z * height * width + y * width + x];
                        let o = z * width + x;
                        if v > out[o] {
                            out[o] = v;
                        }
                    }
                }
            }
        }
        MipAxis::X => {
            for z in 0..depth {
                for y in 0..height {
                    for x in 0..width {
                        let v = voxels[z * height * width + y * width + x];
                        let o = z * height + y;
                        if v > out[o] {
                            out[o] = v;
                        }
                    }
                }
            }
        }
    }

    // Replace residual -inf (shouldn't happen if dims > 0) with 0
    for i in 0..out_len {
        if !out[i].is_finite() {
            out[i] = 0.0;
        }
    }

    Ok((ow, oh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mip_z_picks_max() {
        // 2×2×2: slice0 all 1, slice1 has a 9 at (0,0)
        let mut vol = [1.0f32; 8];
        vol[4] = 9.0; // z=1, y=0, x=0
        let mut out = [0.0f32; 4];
        let (ow, oh) = mip_project_axis(&vol, 2, 2, 2, MipAxis::Z, &mut out).unwrap();
        assert_eq!((ow, oh), (2, 2));
        assert!((out[0] - 9.0).abs() < 1e-6);
        assert!((out[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn mip_x_dims() {
        let vol = [1.0f32, 2.0, 3.0, 4.0]; // 2w × 1h × 2d
        let mut out = [0.0f32; 2];
        let (ow, oh) = mip_project_axis(&vol, 2, 1, 2, MipAxis::X, &mut out).unwrap();
        assert_eq!((ow, oh), (1, 2)); // height × depth
        assert!((out[0] - 2.0).abs() < 1e-6); // max of 1,2
        assert!((out[1] - 4.0).abs() < 1e-6); // max of 3,4
    }

    #[test]
    fn short_buffer() {
        let vol = [1.0f32; 8];
        let mut out = [0.0f32; 2];
        assert_eq!(
            mip_project_axis(&vol, 2, 2, 2, MipAxis::Z, &mut out).unwrap_err(),
            MedicalError::BufferTooSmall
        );
    }
}
