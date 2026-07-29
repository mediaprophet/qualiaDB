//! Spectral peaks from a magnitude spectrum (parabolic sub-bin refinement).

use crate::types::AudioError;

/// Parabolic sub-bin offset and interpolated peak height for the three
/// magnitude ordinates `(ym1, y0, yp1)` centred on a local maximum bin.
/// Returns `(offset, peak_value)` with `offset` in `(-0.5, 0.5)`.
#[inline]
fn parabolic(ym1: f32, y0: f32, yp1: f32) -> (f32, f32) {
    let denom = ym1 - 2.0 * y0 + yp1;
    if denom == 0.0 || !denom.is_finite() {
        return (0.0, y0);
    }
    let off = (0.5 * (ym1 - yp1) / denom).clamp(-0.5, 0.5);
    // Interpolated vertex height y0 - 0.25*(ym1 - yp1)*offset.
    let peak = y0 - 0.25 * (ym1 - yp1) * off;
    (off, peak)
}

/// Extract the strongest spectral peaks from a magnitude spectrum `mag`.
///
/// `mag` is a one-sided magnitude spectrum covering DC..Nyquist inclusive, so a
/// spectrum of `N` bins was produced by an FFT of size `2*(N-1)` and the bin
/// spacing is `sample_rate / (2*(N-1))` Hz. Each interior local maximum is
/// parabolically interpolated to a sub-bin frequency. The up-to-`max_peaks`
/// strongest peaks are written **magnitude-descending** (strongest first):
/// `out_freq[k]` the interpolated frequency in Hz, `out_mag[k]` the interpolated
/// peak magnitude.
///
/// Returns the number of peaks written.
///
/// Zero-heap: the output arrays hold an insertion-sorted top-K; no allocation.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `sample_rate` is not a positive finite.
pub fn spectral_peaks(
    mag: &[f32],
    sample_rate: f32,
    max_peaks: usize,
    out_freq: &mut [f32],
    out_mag: &mut [f32],
) -> Result<usize, AudioError> {
    if sample_rate <= 0.0 || !sample_rate.is_finite() {
        return Err(AudioError::InvalidParameter);
    }
    let cap = out_freq.len().min(out_mag.len()).min(max_peaks);
    if mag.len() < 3 || cap == 0 {
        return Ok(0);
    }
    let bin_hz = sample_rate / (2.0 * (mag.len() - 1) as f32);

    let mut count: usize = 0;
    for k in 1..mag.len() - 1 {
        let y0 = mag[k];
        if !(y0 > mag[k - 1] && y0 >= mag[k + 1] && y0 > 0.0) {
            continue;
        }
        let (off, peak) = parabolic(mag[k - 1], y0, mag[k + 1]);
        let freq = (k as f32 + off) * bin_hz;

        // Insert into the magnitude-descending top-K held in the out arrays.
        if count < cap {
            // Shift down to open the insertion slot.
            let mut j = count;
            while j > 0 && out_mag[j - 1] < peak {
                out_mag[j] = out_mag[j - 1];
                out_freq[j] = out_freq[j - 1];
                j -= 1;
            }
            out_mag[j] = peak;
            out_freq[j] = freq;
            count += 1;
        } else if peak > out_mag[cap - 1] {
            // Displace the current weakest, then bubble into place.
            let mut j = cap - 1;
            while j > 0 && out_mag[j - 1] < peak {
                out_mag[j] = out_mag[j - 1];
                out_freq[j] = out_freq[j - 1];
                j -= 1;
            }
            out_mag[j] = peak;
            out_freq[j] = freq;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden: a downward parabola with vertex at bin 100.3 in a 513-bin
    /// (fft_size=1024) spectrum at 44100 Hz. Parabolic interpolation recovers
    /// the vertex frequency exactly (parabola in, parabola out).
    #[test]
    fn golden_peak_frequency() {
        let sr = 44100.0f32;
        let n = 513usize; // fft_size = 1024
        let bin_hz = sr / (2.0 * (n - 1) as f32); // 43.066 Hz
        let center = 100.3f32;
        let mut mag = vec![0.0f32; n];
        for (k, m) in mag.iter_mut().enumerate() {
            let v = 1.0 - 0.01 * (k as f32 - center) * (k as f32 - center);
            *m = v.max(0.0);
        }
        let mut fr = [0.0f32; 4];
        let mut mg = [0.0f32; 4];
        let np = spectral_peaks(&mag, sr, 4, &mut fr, &mut mg).expect("peaks");
        assert_eq!(np, 1);
        let true_hz = center * bin_hz; // 4319.56 Hz
        assert!(
            (fr[0] - true_hz).abs() < 2.0,
            "freq={} want={}",
            fr[0],
            true_hz
        );
        assert!(mg[0] > 0.99, "peak mag={}", mg[0]);
    }

    /// Two peaks -> strongest returned first, ordering is magnitude-descending.
    #[test]
    fn strongest_first() {
        let sr = 8000.0f32;
        let n = 129usize; // fft_size 256, bin_hz = 8000/256 = 31.25
        let mut mag = vec![0.0f32; n];
        // Weaker peak at bin 20, stronger at bin 60.
        mag[19] = 0.5;
        mag[20] = 1.0;
        mag[21] = 0.5;
        mag[59] = 1.5;
        mag[60] = 3.0;
        mag[61] = 1.5;
        let mut fr = [0.0f32; 4];
        let mut mg = [0.0f32; 4];
        let np = spectral_peaks(&mag, sr, 4, &mut fr, &mut mg).expect("peaks");
        assert_eq!(np, 2);
        assert!(mg[0] > mg[1]);
        // Strongest is bin 60 -> 60 * 31.25 = 1875 Hz (symmetric -> no offset).
        assert!((fr[0] - 1875.0).abs() < 1.0, "freq0={}", fr[0]);
        assert!((fr[1] - 625.0).abs() < 1.0, "freq1={}", fr[1]);
    }

    /// max_peaks bounds the output to the strongest K.
    #[test]
    fn max_peaks_limits() {
        let sr = 8000.0f32;
        let n = 129usize;
        let mut mag = vec![0.0f32; n];
        for c in [10usize, 30, 50, 70] {
            mag[c - 1] = 0.5;
            mag[c] = c as f32; // taller for larger bins
            mag[c + 1] = 0.5;
        }
        let mut fr = [0.0f32; 2];
        let mut mg = [0.0f32; 2];
        let np = spectral_peaks(&mag, sr, 2, &mut fr, &mut mg).expect("peaks");
        assert_eq!(np, 2);
        // Two tallest are bins 70 and 50.
        assert!((fr[0] - 70.0 * (sr / 256.0)).abs() < 1.0, "f0={}", fr[0]);
        assert!((fr[1] - 50.0 * (sr / 256.0)).abs() < 1.0, "f1={}", fr[1]);
    }

    #[test]
    fn rejects_bad_rate() {
        let mag = [0.0f32, 1.0, 0.0];
        let mut fr = [0.0f32; 2];
        let mut mg = [0.0f32; 2];
        assert_eq!(
            spectral_peaks(&mag, 0.0, 2, &mut fr, &mut mg),
            Err(AudioError::InvalidParameter)
        );
    }
}
