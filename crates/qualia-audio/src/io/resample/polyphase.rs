//! Polyphase windowed-sinc resampler for rational rate ratios.
//!
//! Delegated to [`etalon_signal::resample::resample_polyphase_into`].

use crate::types::AudioError;

/// Resample mono f32 `src` (`src_rate`) → `dst_rate` into caller-buffered `out`
/// via the reduced rational factors `L/M`, with `half_taps` per side.
pub fn resample_polyphase(
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
    Ok(etalon_signal::resample::resample_polyphase_into(
        src, src_rate, dst_rate, half_taps, out,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::TAU;

    #[test]
    fn rational_ratio_output_length() {
        // 44.1 kHz → 48 kHz: L/M = 160/147. 1470 in → 1600 out.
        let s = vec![0.0f32; 1470];
        let mut o = vec![0.0f32; 2000];
        let n = resample_polyphase(&s, 44100, 48000, &mut o, 16).unwrap();
        assert_eq!(n, 1470 * 160 / 147);
    }

    #[test]
    fn dc_is_preserved() {
        let s = vec![0.25f32; 512];
        let mut o = vec![0.0f32; 600];
        let n = resample_polyphase(&s, 44100, 48000, &mut o, 16).unwrap();
        assert!(n > 0);
        // Interior avoids edge truncation of the kernel support.
        for &v in &o[64..n - 64] {
            assert!((v - 0.25).abs() < 1e-3, "DC not preserved: {v}");
        }
    }

    #[test]
    fn identity_rate_returns_input() {
        let s = [0.1f32, -0.2, 0.3];
        let mut o = [0.0f32; 3];
        let n = resample_polyphase(&s, 8000, 8000, &mut o, 8).unwrap();
        assert_eq!(n, 3);
        assert_eq!(o, s);
    }

    #[test]
    fn anti_aliasing_downsample_rejects_out_of_band() {
        let n_in = 4800usize;
        let amp = 0.8;
        let out_band: Vec<f32> = (0..n_in)
            .map(|i| (amp * (TAU * 12000.0 * i as f64 / 48000.0).sin()) as f32)
            .collect();
        let mut out = vec![0.0f32; 1700];
        let n = resample_polyphase(&out_band, 48000, 16000, &mut out, 32).unwrap();

        let mid = &out[100..n - 100];
        let w = TAU * 4000.0 / 16000.0;
        let coeff = 2.0 * w.cos();
        let (mut s1, mut s2) = (0.0f64, 0.0f64);
        for &x in mid {
            let s0 = x as f64 + coeff * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        let power = (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0);
        let alias = power.sqrt() * 2.0 / mid.len() as f64;
        assert!(alias < 0.1, "alias tone should be suppressed, got {alias}");
    }
}
