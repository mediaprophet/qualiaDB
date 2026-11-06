//! Band-limited windowed-sinc (Lanczos) resampler — the quality path.
//!
//! Delegated to [`etalon_signal::resample::windowed_sinc_resample_into`].

use crate::types::AudioError;

/// Resample mono f32 `src` (`src_rate`) → `dst_rate` into caller-buffered `out`,
/// using a Lanczos-windowed-sinc kernel with `half_taps` on each side.
pub fn resample_sinc(
    src: &[f32],
    src_rate: u32,
    dst_rate: u32,
    out: &mut [f32],
    half_taps: usize,
) -> Result<usize, AudioError> {
    if src_rate == 0 || dst_rate == 0 || half_taps == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if src.is_empty() {
        return Err(AudioError::MalformedAudio);
    }
    Ok(etalon_signal::resample::windowed_sinc_resample_into(
        src,
        f64::from(src_rate),
        f64::from(dst_rate),
        half_taps,
        0.0,
        out,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::TAU;

    /// Goertzel amplitude estimate at `freq` Hz over `sig` sampled at `fs`.
    fn goertzel_amp(sig: &[f32], freq: f64, fs: f64) -> f64 {
        let n = sig.len();
        if n == 0 {
            return 0.0;
        }
        let w = TAU * freq / fs;
        let coeff = 2.0 * w.cos();
        let mut s1 = 0.0f64;
        let mut s2 = 0.0f64;
        for &x in sig {
            let s0 = x as f64 + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let power = s1 * s1 + s2 * s2 - coeff * s1 * s2;
        power.max(0.0).sqrt() * 2.0 / n as f64
    }

    fn rms(sig: &[f32]) -> f64 {
        if sig.is_empty() {
            return 0.0;
        }
        let s: f64 = sig.iter().map(|&x| x as f64 * x as f64).sum();
        (s / sig.len() as f64).sqrt()
    }

    fn sine(freq: f64, amp: f64, n: usize, fs: f64) -> Vec<f32> {
        (0..n)
            .map(|i| (amp * (TAU * freq * i as f64 / fs).sin()) as f32)
            .collect()
    }

    #[test]
    fn identity_rate_returns_input() {
        let s = [0.1f32, 0.2, 0.3, 0.4];
        let mut o = [0.0f32; 4];
        let n = resample_sinc(&s, 16000, 16000, &mut o, 16).unwrap();
        assert_eq!(n, 4);
        assert_eq!(o, s);
    }

    #[test]
    fn dc_is_preserved() {
        let s = vec![0.5f32; 256];
        let mut o = vec![0.0f32; 128];
        let n = resample_sinc(&s, 48000, 16000, &mut o, 16).unwrap();
        assert!(n > 0);
        for (k, &v) in o[..n].iter().enumerate() {
            assert!((v - 0.5).abs() < 1e-3, "bin {k} = {v}, expected 0.5");
        }
    }

    #[test]
    fn ramp_value_is_preserved_interior() {
        let s: Vec<f32> = (0..200).map(|j| 0.01 * j as f32).collect();
        let mut o = vec![0.0f32; 100];
        let half = 16usize;
        let n = resample_sinc(&s, 48000, 24000, &mut o, half).unwrap();
        for i in 25..=75 {
            let center = 2.0 * i as f32;
            let expected = 0.01 * center;
            assert!(
                (o[i] - expected).abs() < 1e-2,
                "ramp[{i}] = {}, expected {expected}",
                o[i]
            );
        }
        let _ = n;
    }

    #[test]
    fn anti_aliasing_downsample_rejects_out_of_band_tone() {
        let src_rate = 48000u32;
        let dst_rate = 8000u32;
        let fs_out = dst_rate as f64;
        let n_in = 4800usize; // 0.1 s
        let amp = 0.8;
        let half = 32usize;

        let in_band = sine(1000.0, amp, n_in, src_rate as f64);
        let mut out_in = vec![0.0f32; 900];
        let n_in_out = resample_sinc(&in_band, src_rate, dst_rate, &mut out_in, half).unwrap();
        let mid_in = &out_in[100..n_in_out.saturating_sub(100)];
        let amp_1k = goertzel_amp(mid_in, 1000.0, fs_out);
        assert!(
            amp_1k > 0.6,
            "in-band 1 kHz should survive ~unity (0.8), got {amp_1k}"
        );

        let out_band = sine(6000.0, amp, n_in, src_rate as f64);
        let mut out_ob = vec![0.0f32; 900];
        let n_ob_out = resample_sinc(&out_band, src_rate, dst_rate, &mut out_ob, half).unwrap();
        let mid_ob = &out_ob[100..n_ob_out.saturating_sub(100)];

        let alias_2k = goertzel_amp(mid_ob, 2000.0, fs_out);
        assert!(
            alias_2k < 0.1,
            "aliased 2 kHz tone should be suppressed (< 0.1), got {alias_2k}"
        );
        let rms_ob = rms(mid_ob);
        let rms_in = rms(mid_in);
        assert!(
            rms_ob < 0.15 * rms_in,
            "out-of-band residual rms {rms_ob} should be ≪ in-band rms {rms_in}"
        );
    }

    #[test]
    fn rejects_bad_params() {
        let s = [0.0f32; 4];
        let mut o = [0.0f32; 4];
        assert_eq!(
            resample_sinc(&s, 0, 8000, &mut o, 8),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            resample_sinc(&s, 8000, 8000, &mut o, 0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            resample_sinc(&[], 8000, 16000, &mut o, 8),
            Err(AudioError::MalformedAudio)
        );
    }
}
