//! Polyphase windowed-sinc resampler for rational rate ratios.
//!
//! For a rational conversion `src_rate : dst_rate`, reducing by the greatest
//! common divisor gives interpolation factor `L = dst/gcd` and decimation
//! factor `M = src/gcd`. Output sample `i` then corresponds to the *exact*
//! source position `i·M/L`, whose fractional part `(i·M) mod L` selects one of
//! `L` polyphase kernel phases. Evaluating the same band-limited windowed-sinc
//! kernel as [`super::windowed_sinc`] at that exact phase gives an
//! anti-aliased, allocation-free resampler well suited to fixed studio ratios
//! (e.g. 44.1 kHz ↔ 48 kHz, `L/M = 160/147`).
//!
//! Like the direct path this is **zero-heap**: the output is caller-buffered
//! and every tap is computed from stack scalars.

use crate::io::resample::anti_alias::antialias_cutoff;
use crate::types::AudioError;

/// Resample mono f32 `src` (`src_rate`) → `dst_rate` into caller-buffered `out`
/// via the reduced rational factors `L/M`, with `half_taps` per side.
///
/// Semantics and errors mirror [`super::windowed_sinc::resample_sinc`]; this
/// path drives the kernel from exact integer phase indices instead of a
/// floating step. Returns output frames written (clamped to `out.len()`).
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
    if src_rate == dst_rate {
        let n = src.len().min(out.len());
        out[..n].copy_from_slice(&src[..n]);
        return Ok(n);
    }

    let g = gcd(src_rate, dst_rate);
    let l = (dst_rate / g) as u64; // interpolation factor
    let m = (src_rate / g) as u64; // decimation factor

    let out_len_total = (src.len() as u64 * l / m) as usize;
    let out_n = out_len_total.min(out.len());
    if out_n == 0 {
        return Ok(0);
    }

    let filt_scale = (2.0 * antialias_cutoff(src_rate, dst_rate)) as f64; // = min(1, dst/src)
    let radius = half_taps as f64 / filt_scale;
    let len = src.len() as i64;

    for (i, o) in out.iter_mut().enumerate().take(out_n) {
        // Exact source position: num/l = i·M/L. Split into integer base + phase.
        let num = i as u64 * m;
        let base = (num / l) as i64;
        let phase = (num % l) as f64 / l as f64; // fractional source offset in [0,1)
        let center = base as f64 + phase;

        let j_min = (center - radius).ceil() as i64;
        let j_max = (center + radius).floor() as i64;
        let mut acc = 0.0f64;
        let mut wsum = 0.0f64;
        let mut j = j_min;
        while j <= j_max {
            if j >= 0 && j < len {
                let d = center - j as f64;
                let tap = filt_scale * sinc(filt_scale * d) * sinc(d / radius);
                acc += src[j as usize] as f64 * tap;
                wsum += tap;
            }
            j += 1;
        }
        *o = if wsum.abs() < 1e-12 {
            0.0
        } else {
            (acc / wsum) as f32
        };
    }
    Ok(out_n)
}

/// Greatest common divisor (Euclid).
fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Normalised sinc: `sin(πx) / (πx)`, with `sinc(0) = 1`.
#[inline]
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let px = core::f64::consts::PI * x;
        px.sin() / px
    }
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
        // 48 kHz → 16 kHz (dst Nyquist 8 kHz). 12 kHz tone is out of band and
        // would alias to |12000 - 16000| = 4000 Hz under naive decimation.
        let n_in = 4800usize;
        let amp = 0.8;
        let out_band: Vec<f32> = (0..n_in)
            .map(|i| (amp * (TAU * 12000.0 * i as f64 / 48000.0).sin()) as f32)
            .collect();
        let mut out = vec![0.0f32; 1700];
        let n = resample_polyphase(&out_band, 48000, 16000, &mut out, 32).unwrap();

        // Goertzel at the alias frequency should be small.
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
