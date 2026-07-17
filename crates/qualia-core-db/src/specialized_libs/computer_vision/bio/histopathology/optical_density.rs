//! RGB8 → optical density (Beer–Lambert): OD = −log₁₀((I+1)/255).
//!
//! Per-channel, packed RGB interleaved. Caller supplies `out` of length ≥ `rgb.len()`.

use super::HistoError;

/// Convert packed RGB8 intensities to optical density (f32, same layout).
///
/// `rgb` and `out` are interleaved RGB (length multiple of 3). Uses `(I+1)/255`
/// so pure black does not produce +∞.
pub fn optical_density_rgb(rgb: &[u8], out: &mut [f32]) -> Result<(), HistoError> {
    if rgb.is_empty() {
        return Err(HistoError::EmptyInput);
    }
    if rgb.len() % 3 != 0 {
        return Err(HistoError::InvalidParameter);
    }
    if out.len() < rgb.len() {
        return Err(HistoError::BufferTooSmall);
    }
    // log10(x) = ln(x) / ln(10)
    const LN10: f32 = 2.302_585_092_994_046;
    for i in 0..rgb.len() {
        // Clamp transmittance to (0,1] so OD is always ≥ 0.
        let t = ((rgb[i] as f32 + 1.0) / 256.0).clamp(1e-6, 1.0);
        out[i] = -(t.ln() / LN10);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn od_monotonic_darker_higher() {
        // Darker pixel → higher OD.
        let bright = [200u8, 200, 200];
        let dark = [20u8, 20, 20];
        let mut od_b = [0f32; 3];
        let mut od_d = [0f32; 3];
        optical_density_rgb(&bright, &mut od_b).unwrap();
        optical_density_rgb(&dark, &mut od_d).unwrap();
        assert!(od_d[0] > od_b[0]);
        assert!(od_d[1] > od_b[1]);
        assert!(od_d[2] > od_b[2]);
    }

    #[test]
    fn od_white_near_zero() {
        let white = [255u8, 255, 255];
        let mut od = [0f32; 3];
        optical_density_rgb(&white, &mut od).unwrap();
        // −log10(256/255) is tiny positive.
        // Near-white → OD near zero (allow tiny FP noise).
        assert!(od[0].abs() < 0.02);
    }

    #[test]
    fn od_buffer_too_small() {
        let rgb = [10u8, 20, 30];
        let mut od = [0f32; 2];
        assert_eq!(
            optical_density_rgb(&rgb, &mut od),
            Err(HistoError::BufferTooSmall)
        );
    }
}
