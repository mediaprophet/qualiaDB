//! Decode + equal-loudness convenience: a single "perceptual load" scalar.
//!
//! This is a **cold-path convenience** (not the zero-heap hot path): it takes a
//! complete WAV byte buffer and returns one number summarising how *loud the
//! content is to a human ear*, combining three existing primitives:
//!
//! 1. [`decode_wav`] → PCM, then [`to_mono_f32`] → mono samples.
//! 2. Non-overlapping [`BLOCK`]-length FFT magnitude spectra
//!    ([`real_fft_magnitude`]).
//! 3. [`apply_equal_loudness`] A-weighting per block.
//!
//! The returned value is the root-mean-square of the A-weighted magnitude
//! spectrum, averaged over all analysis blocks — an equal-loudness-weighted
//! spectral energy proxy. Two recordings of equal physical amplitude but
//! different spectral content (e.g. a 1 kHz tone vs. a 50 Hz rumble) yield
//! different perceptual loads, with mid-band content weighted higher.

use crate::convert::to_mono_f32;
use crate::features::fft::real_fft::real_fft_magnitude;
use crate::io::eqloud::equal_loudness::apply_equal_loudness;
use crate::types::AudioError;
use crate::wav::decode_wav;

/// FFT analysis block size (power of two) used for the perceptual-load spectrum.
pub const BLOCK: usize = 1024;

/// Compute the equal-loudness-weighted "perceptual load" of a WAV byte buffer.
///
/// Returns a single non-negative scalar (A-weighted spectral RMS averaged over
/// blocks). Propagates [`AudioError`] from decode / conversion / FFT.
pub fn perceptual_load_from_wav(bytes: &[u8]) -> Result<f32, AudioError> {
    let decoded = decode_wav(bytes)?;
    let view = decoded.view();
    let sample_rate = view.sample_rate;
    let frames = view.frames as usize;

    // Cold path: allocate a mono working buffer once.
    let mut mono = vec![0.0f32; frames];
    let got = to_mono_f32(view, &mut mono)?;
    mono.truncate(got);

    let bins = BLOCK / 2 + 1;
    let mut block = [0.0f32; BLOCK];
    let mut scratch = vec![0.0f32; 2 * BLOCK];
    let mut mags = vec![0.0f32; bins];
    let mut weighted = vec![0.0f32; bins];

    let mut acc = 0.0f64;
    let mut nblocks = 0usize;
    let total = mono.len();
    let mut start = 0usize;
    loop {
        // Fill one block, zero-padding the tail past the signal end.
        for (i, slot) in block.iter_mut().enumerate() {
            *slot = mono.get(start + i).copied().unwrap_or(0.0);
        }
        real_fft_magnitude(&block, &mut scratch, &mut mags)?;
        apply_equal_loudness(&mags, sample_rate, &mut weighted)?;
        for &w in weighted.iter() {
            acc += (w as f64) * (w as f64);
        }
        nblocks += 1;
        start += BLOCK;
        if start >= total {
            break;
        }
    }

    let denom = (nblocks * bins).max(1) as f64;
    Ok((acc / denom).sqrt() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wav::encode_wav_i16_mono;
    use core::f32::consts::TAU;

    /// Encode a mono sine WAV of `n` samples at `freq` Hz, amplitude `amp`.
    fn sine_wav(freq: f32, amp: f32, n: usize, fs: u32) -> Vec<u8> {
        let samples: Vec<i16> = (0..n)
            .map(|i| {
                let s = amp * (TAU * freq * i as f32 / fs as f32).sin();
                (s.clamp(-1.0, 1.0) * 32767.0) as i16
            })
            .collect();
        let mut buf = vec![0u8; 44 + samples.len() * 2];
        let n = encode_wav_i16_mono(&samples, fs, &mut buf).unwrap();
        buf.truncate(n);
        buf
    }

    #[test]
    fn midband_tone_has_higher_perceptual_load_than_low_rumble() {
        let fs = 8000;
        let n = 4096;
        let amp = 0.5;
        let mid = sine_wav(1000.0, amp, n, fs); // near unity A-weight
        let low = sine_wav(100.0, amp, n, fs); // ~ -19 dB A-weight

        let load_mid = perceptual_load_from_wav(&mid).unwrap();
        let load_low = perceptual_load_from_wav(&low).unwrap();

        assert!(load_mid.is_finite() && load_low.is_finite());
        assert!(load_mid > 0.0);
        // Equal physical amplitude, but the mid-band tone is perceptually louder.
        assert!(
            load_mid > 2.0 * load_low,
            "expected mid ({load_mid}) ≫ low ({load_low})"
        );
    }
}
