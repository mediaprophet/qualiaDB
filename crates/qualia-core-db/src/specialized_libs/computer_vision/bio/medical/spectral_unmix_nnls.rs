//! Non-negative least-squares lite for multi-channel spectral unmixing.
//!
//! Model: observed spectrum `y` (C channels) ≈ `A · x` where `A` is C×N
//! reference endmember matrix (column-major: channel-major per component),
//! and `x ≥ 0` are component abundances.
//!
//! Solver: projected multiplicative update / projected gradient (bounded iters).
//! Also supports per-pixel unmix into caller buffers and ROI-mean unmix.

use super::hu_window::MedicalError;

/// Max channels / components for stack-bounded unmix.
pub const MAX_CHANNELS: usize = 16;
pub const MAX_COMPONENTS: usize = 8;

/// Result of a single spectrum unmix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnmixResult {
    pub n_components: usize,
    /// Abundances (first `n_components` valid).
    pub abundances: [f64; MAX_COMPONENTS],
    /// ‖y − A x‖₂ residual.
    pub residual_l2: f64,
}

/// Unmix one observed spectrum `y` (length `n_channels`) against endmember
/// matrix `endmembers` laid out as column-major: component `k` occupies
/// `endmembers[k * n_channels .. (k+1) * n_channels]`.
///
/// `n_components` columns. Uses multiplicative NNLS-lite updates.
pub fn spectral_unmix_nnls(
    y: &[f32],
    endmembers: &[f32],
    n_channels: usize,
    n_components: usize,
) -> Result<UnmixResult, MedicalError> {
    if n_channels == 0
        || n_components == 0
        || n_channels > MAX_CHANNELS
        || n_components > MAX_COMPONENTS
    {
        return Err(MedicalError::InvalidParameter);
    }
    if y.len() < n_channels || endmembers.len() < n_channels * n_components {
        return Err(MedicalError::DimensionMismatch);
    }

    // x starts uniform positive
    let mut x = [0.0f64; MAX_COMPONENTS];
    let init = 1.0 / n_components as f64;
    for k in 0..n_components {
        x[k] = init;
    }

    let y64: [f64; MAX_CHANNELS] = {
        let mut a = [0.0f64; MAX_CHANNELS];
        for c in 0..n_channels {
            a[c] = y[c] as f64;
        }
        a
    };

    // Multiplicative update (Lee-Seung style for NNLS):
    // x ← x ⊙ (Aᵀ y) / (Aᵀ A x + ε)
    const ITERS: usize = 64;
    const EPS: f64 = 1e-12;

    for _ in 0..ITERS {
        // At y = Aᵀ y  (n_components)
        let mut at_y = [0.0f64; MAX_COMPONENTS];
        for k in 0..n_components {
            let mut s = 0.0f64;
            for c in 0..n_channels {
                s += endmembers[k * n_channels + c] as f64 * y64[c];
            }
            at_y[k] = s;
        }
        // A x
        let mut ax = [0.0f64; MAX_CHANNELS];
        for c in 0..n_channels {
            let mut s = 0.0f64;
            for k in 0..n_components {
                s += endmembers[k * n_channels + c] as f64 * x[k];
            }
            ax[c] = s;
        }
        // Aᵀ (A x)
        let mut at_ax = [0.0f64; MAX_COMPONENTS];
        for k in 0..n_components {
            let mut s = 0.0f64;
            for c in 0..n_channels {
                s += endmembers[k * n_channels + c] as f64 * ax[c];
            }
            at_ax[k] = s;
        }
        for k in 0..n_components {
            x[k] *= at_y[k] / (at_ax[k] + EPS);
            if x[k] < 0.0 {
                x[k] = 0.0;
            }
        }
    }

    // Residual
    let mut residual = 0.0f64;
    for c in 0..n_channels {
        let mut pred = 0.0f64;
        for k in 0..n_components {
            pred += endmembers[k * n_channels + c] as f64 * x[k];
        }
        let d = y64[c] - pred;
        residual += d * d;
    }

    Ok(UnmixResult {
        n_components,
        abundances: x,
        residual_l2: residual.sqrt(),
    })
}

/// Mean the ROI spectra then unmix once (cheap path for bulk regions).
///
/// `pixels` is planar or interleaved? **Interleaved** channel-major per pixel:
/// `pixels[p * n_channels + c]`. `n_pixels` pixels.
pub fn spectral_unmix_roi_mean(
    pixels: &[f32],
    n_pixels: usize,
    endmembers: &[f32],
    n_channels: usize,
    n_components: usize,
) -> Result<UnmixResult, MedicalError> {
    if n_pixels == 0 {
        return Err(MedicalError::EmptyInput);
    }
    if n_channels == 0 || n_channels > MAX_CHANNELS {
        return Err(MedicalError::InvalidParameter);
    }
    if pixels.len() < n_pixels * n_channels {
        return Err(MedicalError::DimensionMismatch);
    }

    let mut mean = [0.0f32; MAX_CHANNELS];
    for p in 0..n_pixels {
        for c in 0..n_channels {
            mean[c] += pixels[p * n_channels + c];
        }
    }
    let inv = 1.0 / n_pixels as f32;
    for c in 0..n_channels {
        mean[c] *= inv;
    }
    spectral_unmix_nnls(&mean[..n_channels], endmembers, n_channels, n_components)
}

/// Per-pixel unmix: writes abundances into `out_abundances` laid out as
/// `pixel-major` × `n_components`. Returns number of pixels written.
pub fn spectral_unmix_per_pixel(
    pixels: &[f32],
    n_pixels: usize,
    endmembers: &[f32],
    n_channels: usize,
    n_components: usize,
    out_abundances: &mut [f32],
) -> Result<usize, MedicalError> {
    if n_pixels == 0 {
        return Err(MedicalError::EmptyInput);
    }
    if out_abundances.len() < n_pixels * n_components {
        return Err(MedicalError::BufferTooSmall);
    }
    for p in 0..n_pixels {
        let start = p * n_channels;
        let y = &pixels[start..start + n_channels];
        let r = spectral_unmix_nnls(y, endmembers, n_channels, n_components)?;
        for k in 0..n_components {
            out_abundances[p * n_components + k] = r.abundances[k] as f32;
        }
    }
    Ok(n_pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_endmember_recovers() {
        // 2 channels, 2 components: identity endmembers
        // A = [[1,0],[0,1]] column-major: [1,0, 0,1]
        let endmembers = [1.0f32, 0.0, 0.0, 1.0];
        let y = [0.3f32, 0.7];
        let r = spectral_unmix_nnls(&y, &endmembers, 2, 2).unwrap();
        assert!((r.abundances[0] - 0.3).abs() < 0.05);
        assert!((r.abundances[1] - 0.7).abs() < 0.05);
        assert!(r.residual_l2 < 0.1);
    }

    #[test]
    fn mixture_of_two() {
        // endmembers: e0=[1,0,0], e1=[0,1,0]  (3 ch, 2 comp)
        // y = 0.4 e0 + 0.6 e1 = [0.4, 0.6, 0]
        let endmembers = [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0];
        let y = [0.4f32, 0.6, 0.0];
        let r = spectral_unmix_nnls(&y, &endmembers, 3, 2).unwrap();
        assert!((r.abundances[0] - 0.4).abs() < 0.08);
        assert!((r.abundances[1] - 0.6).abs() < 0.08);
        assert!(r.abundances[0] >= 0.0 && r.abundances[1] >= 0.0);
    }

    #[test]
    fn roi_mean_path() {
        let endmembers = [1.0f32, 0.0, 0.0, 1.0];
        // two pixels averaging to [0.5, 0.5]
        let pixels = [1.0f32, 0.0, 0.0, 1.0];
        let r = spectral_unmix_roi_mean(&pixels, 2, &endmembers, 2, 2).unwrap();
        assert!((r.abundances[0] - 0.5).abs() < 0.08);
        assert!((r.abundances[1] - 0.5).abs() < 0.08);
    }

    #[test]
    fn rejects_zero_channels() {
        assert_eq!(
            spectral_unmix_nnls(&[], &[], 0, 1).unwrap_err(),
            MedicalError::InvalidParameter
        );
    }
}
