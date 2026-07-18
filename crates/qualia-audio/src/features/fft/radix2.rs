//! In-place iterative radix-2 Cooley–Tukey FFT (power-of-two `N`).
//!
//! Operates on interleaved complex data `[re0, im0, re1, im1, …]`, so the slice
//! length is `2 * N`. Zero-heap: the permutation is done in place and twiddle
//! factors are computed on the fly (in `f64` for accuracy, stored back as `f32`).

use crate::types::AudioError;

/// Forward (`inverse == false`) or inverse (`inverse == true`) radix-2 FFT,
/// in place, over interleaved complex samples.
///
/// `data.len()` must equal `2 * N` where `N` is a power of two. The inverse
/// transform is normalised by `1/N`, so `fft_radix2(x, false)` followed by
/// `fft_radix2(x, true)` reproduces the original `x`.
///
/// Returns [`AudioError::InvalidParameter`] if the length is odd, zero, or the
/// implied `N` is not a power of two.
pub fn fft_radix2(data: &mut [f32], inverse: bool) -> Result<(), AudioError> {
    let len = data.len();
    if len == 0 || !len.is_multiple_of(2) {
        return Err(AudioError::InvalidParameter);
    }
    let n = len / 2;
    if !n.is_power_of_two() {
        return Err(AudioError::InvalidParameter);
    }
    if n == 1 {
        // Single point: DFT is identity, inverse divides by 1 → no-op.
        return Ok(());
    }

    // --- Bit-reversal permutation over complex indices [0, n). ---
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            data.swap(2 * i, 2 * j);
            data.swap(2 * i + 1, 2 * j + 1);
        }
    }

    // --- Danielson–Lanczos butterflies. ---
    // Forward transform uses e^{-j2πk/L}; inverse uses e^{+j2πk/L}.
    let sign: f64 = if inverse { 1.0 } else { -1.0 };
    let mut sub = 2usize;
    while sub <= n {
        let half = sub / 2;
        let theta = sign * core::f64::consts::TAU / sub as f64;
        let mut base = 0usize;
        while base < n {
            for k in 0..half {
                // Twiddle w = e^{sign·j·2π·k/sub}, computed directly for accuracy.
                let ang = theta * k as f64;
                let wr = ang.cos() as f32;
                let wi = ang.sin() as f32;

                let a = base + k;
                let b = base + k + half;
                let ar = data[2 * a];
                let ai = data[2 * a + 1];
                let br = data[2 * b];
                let bi = data[2 * b + 1];

                // t = w · b
                let tr = wr * br - wi * bi;
                let ti = wr * bi + wi * br;

                data[2 * b] = ar - tr;
                data[2 * b + 1] = ai - ti;
                data[2 * a] = ar + tr;
                data[2 * a + 1] = ai + ti;
            }
            base += sub;
        }
        sub <<= 1;
    }

    // --- Inverse normalisation by 1/N. ---
    if inverse {
        let inv_n = 1.0f32 / n as f32;
        for v in data.iter_mut() {
            *v *= inv_n;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    fn magnitudes(data: &[f32]) -> Vec<f32> {
        data.chunks_exact(2)
            .map(|c| (c[0] * c[0] + c[1] * c[1]).sqrt())
            .collect()
    }

    #[test]
    fn peak_bin_matches_cosine_frequency() {
        // Golden: FFT of cos(2π·k0·n/N) peaks at bins k0 and N-k0.
        let n = 64usize;
        let k0 = 5usize;
        let mut data = vec![0.0f32; 2 * n];
        for i in 0..n {
            data[2 * i] = (TAU * k0 as f32 * i as f32 / n as f32).cos();
            data[2 * i + 1] = 0.0;
        }
        fft_radix2(&mut data, false).unwrap();
        let mags = magnitudes(&data);

        // Peak over lower half (excluding DC) must be at k0.
        let mut peak = 1usize;
        for (b, m) in mags.iter().enumerate().take(n / 2 + 1).skip(1) {
            if *m > mags[peak] {
                peak = b;
            }
        }
        assert_eq!(peak, k0, "expected peak at k0={k0}, got {peak}");
        // Mirror image at N-k0 must also be strong.
        assert!(mags[n - k0] > mags[k0] * 0.5);
    }

    #[test]
    fn round_trip_reconstructs_input() {
        let n = 128usize;
        let mut data = vec![0.0f32; 2 * n];
        let orig: Vec<f32> = (0..n)
            .map(|i| (0.3 * i as f32).sin() + 0.5 * (0.05 * i as f32).cos())
            .collect();
        for i in 0..n {
            data[2 * i] = orig[i];
            data[2 * i + 1] = 0.0;
        }
        fft_radix2(&mut data, false).unwrap();
        fft_radix2(&mut data, true).unwrap();
        for i in 0..n {
            assert!(
                (data[2 * i] - orig[i]).abs() < 1e-4,
                "re[{i}] {} vs {}",
                data[2 * i],
                orig[i]
            );
            assert!(data[2 * i + 1].abs() < 1e-4, "im[{i}] leaked {}", data[2 * i + 1]);
        }
    }

    #[test]
    fn rejects_non_power_of_two() {
        let mut data = vec![0.0f32; 2 * 6]; // N = 6
        assert_eq!(fft_radix2(&mut data, false), Err(AudioError::InvalidParameter));
    }

    #[test]
    fn rejects_odd_length() {
        let mut data = vec![0.0f32; 7];
        assert_eq!(fft_radix2(&mut data, false), Err(AudioError::InvalidParameter));
    }
}
