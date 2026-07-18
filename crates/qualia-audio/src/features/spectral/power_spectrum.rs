//! Power spectrum of a windowed real frame (magnitude spectrum, squared).

use crate::features::fft::real_fft_magnitude;
use crate::types::AudioError;

/// Power spectrum of `N` real (already-windowed) samples.
///
/// A thin wrapper over [`real_fft_magnitude`]: it produces the `N/2 + 1`
/// one-sided magnitude bins `|X[k]|` and then squares each in place, so
/// `out_power[k] == |X[k]|^2`.
///
/// - `input`: `N` real samples, `N` a power of two.
/// - `scratch`: exactly `2 * N` floats, clobbered on return.
/// - `out_power`: at least `N/2 + 1` floats; bins `0..=N/2` are written.
///
/// Zero-heap: all working storage is caller-supplied.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `N` is 0 or not a power of two.
/// - [`AudioError::OutputBufferTooSmall`] if `scratch` or `out_power` are short.
pub fn power_spectrum(
    input: &[f32],
    scratch: &mut [f32],
    out_power: &mut [f32],
) -> Result<(), AudioError> {
    real_fft_magnitude(input, scratch, out_power)?;
    let bins = input.len() / 2 + 1;
    for p in out_power.iter_mut().take(bins) {
        *p *= *p;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    #[test]
    fn dc_power_is_magnitude_squared() {
        let n = 16usize;
        let input = vec![1.0f32; n];
        let mut scratch = vec![0.0f32; 2 * n];
        let mut power = vec![0.0f32; n / 2 + 1];
        power_spectrum(&input, &mut scratch, &mut power).expect("power");
        // DC magnitude == n, so DC power == n^2.
        assert!((power[0] - (n * n) as f32).abs() < 1e-2, "dc={}", power[0]);
        for p in &power[1..] {
            assert!(*p < 1e-3);
        }
    }

    #[test]
    fn tone_power_concentrates_in_bin() {
        let n = 1024usize;
        let fs = 44_100.0f32;
        let k0 = 40usize;
        let freq = k0 as f32 * fs / n as f32;
        let input: Vec<f32> = (0..n).map(|i| (TAU * freq * i as f32 / fs).cos()).collect();
        let mut scratch = vec![0.0f32; 2 * n];
        let mut power = vec![0.0f32; n / 2 + 1];
        power_spectrum(&input, &mut scratch, &mut power).expect("power");
        let mut peak = 1usize;
        for (b, p) in power.iter().enumerate().skip(1) {
            if *p > power[peak] {
                peak = b;
            }
        }
        assert_eq!(peak, k0, "peak power in bin {k0}, found {peak}");
        // Power is non-negative everywhere.
        assert!(power.iter().all(|p| *p >= 0.0));
    }

    #[test]
    fn rejects_non_power_of_two() {
        let input = vec![0.0f32; 6];
        let mut scratch = vec![0.0f32; 12];
        let mut power = vec![0.0f32; 4];
        assert_eq!(
            power_spectrum(&input, &mut scratch, &mut power),
            Err(AudioError::InvalidParameter)
        );
    }
}
