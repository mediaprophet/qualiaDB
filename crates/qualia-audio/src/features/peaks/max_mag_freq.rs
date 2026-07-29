//! Frequency of the maximum-magnitude bin (parabolic-refined).

/// Frequency in Hz of the strongest bin in a one-sided magnitude spectrum
/// `mag` (DC..Nyquist inclusive; bin spacing `sample_rate / (2*(N-1))`).
///
/// The peak bin is refined with a parabolic fit against its two neighbours when
/// it is interior, giving a sub-bin frequency estimate. Returns `0.0` for an
/// empty spectrum, a non-positive/non-finite `sample_rate`, or a peak that sits
/// on the DC / Nyquist boundary (no interpolation possible — the bin-centre
/// frequency is returned in that case).
pub fn max_magnitude_frequency(mag: &[f32], sample_rate: f32) -> f32 {
    if mag.is_empty() || sample_rate <= 0.0 || !sample_rate.is_finite() {
        return 0.0;
    }
    if mag.len() == 1 {
        return 0.0; // DC only
    }
    let bin_hz = sample_rate / (2.0 * (mag.len() - 1) as f32);

    // Argmax.
    let mut kmax = 0usize;
    let mut vmax = mag[0];
    for (k, &v) in mag.iter().enumerate() {
        if v > vmax {
            vmax = v;
            kmax = k;
        }
    }

    // Parabolic refinement when the peak is interior.
    let mut refined = kmax as f32;
    if kmax > 0 && kmax < mag.len() - 1 {
        let ym1 = mag[kmax - 1];
        let y0 = mag[kmax];
        let yp1 = mag[kmax + 1];
        let denom = ym1 - 2.0 * y0 + yp1;
        if denom != 0.0 && denom.is_finite() {
            refined += (0.5 * (ym1 - yp1) / denom).clamp(-0.5, 0.5);
        }
    }
    refined * bin_hz
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden: a single-bin spike returns exactly that bin's frequency.
    #[test]
    fn single_bin_spike() {
        let sr = 16000.0f32; // n = 9 bins (fft_size 16) -> bin_hz = 1000
        let mut mag = [0.0f32; 9];
        mag[3] = 1.0;
        let f = max_magnitude_frequency(&mag, sr);
        // Neighbours are zero -> symmetric -> no offset -> exactly 3 * 1000.
        assert!((f - 3000.0).abs() < 1e-3, "f={f}");
    }

    /// Sub-bin: an asymmetric parabola vertex is recovered between bins.
    #[test]
    fn sub_bin_refinement() {
        let sr = 2000.0f32; // n = 11 bins -> bin_hz = 100
        let center = 5.4f32;
        let mut mag = [0.0f32; 11];
        for (k, m) in mag.iter_mut().enumerate() {
            let v = 1.0 - 0.02 * (k as f32 - center) * (k as f32 - center);
            *m = v.max(0.0);
        }
        let f = max_magnitude_frequency(&mag, sr);
        assert!((f - center * 100.0).abs() < 1.0, "f={f}");
    }

    #[test]
    fn empty_is_zero() {
        assert_eq!(max_magnitude_frequency(&[], 44100.0), 0.0);
    }

    #[test]
    fn bad_rate_is_zero() {
        assert_eq!(max_magnitude_frequency(&[0.0, 1.0, 0.0], -1.0), 0.0);
    }
}
