//! Per-frame voice-activity decision from a fusion of three cheap, orthogonal
//! cues:
//!
//! 1. **Energy above an adaptive noise floor** — the primary gate. Activity
//!    requires the frame RMS to clear the (caller-supplied) noise floor by a
//!    margin; the size of the margin, in dB, sets the confidence.
//! 2. **Spectral flatness** — tonal/harmonic content (low flatness) is
//!    structured signal; a flat (white-ish) spectrum is noise-like. This
//!    *raises confidence* for structured frames but never vetoes a clearly
//!    above-floor frame, so a genuine noise burst is still reported as active.
//! 3. **Zero-crossing rate** — a coarse spectral-tilt proxy; voiced/percussive
//!    content sits in a plausible mid range, extreme values are down-weighted.
//!
//! EPISTEMIC RULE: the output is a *proposal*, not ground truth. A frame at or
//! below the noise floor is returned unvoiced with zero confidence — **silence
//! is never forced into "speech"**. Confidence is fused so that the energy gate
//! dominates and the spectral/temporal cues only modulate a frame that has
//! already cleared the floor.
//!
//! Zero-heap hot path: spectral flatness is computed through an in-place FFT
//! over fixed **stack** scratch buffers (no allocation). Frames whose length is
//! not a power of two, or longer than [`MAX_VAD_FFT`], skip the spectral cue and
//! fall back to a neutral tonality — the energy gate still applies.

use crate::features::energy::{frame_energy, frame_zcr};
use crate::features::fft::real_fft_magnitude;
use crate::features::spectral::spectral_flatness_db;

/// Largest frame length (samples) for which the stack-buffered spectral-flatness
/// cue is computed. Covers the usual 256/512/1024-sample VAD frames; longer
/// frames fall back to the energy+ZCR decision with neutral tonality.
pub const MAX_VAD_FFT: usize = 1024;

/// SNR (dB) at which a frame *starts* to count as active above the floor.
const ONSET_DB: f32 = 3.0;
/// SNR (dB) at which energy confidence saturates to 1.0.
const FULL_DB: f32 = 15.0;
/// Confidence below which a frame is declared unvoiced by [`frame_is_voiced`].
const VOICE_DECISION: f32 = 0.5;
/// Floor of the structure multiplier: even a perfectly flat (noise-like) but
/// clearly above-floor frame keeps this fraction of its energy confidence, so
/// noise bursts are still detected as activity.
const STRUCT_FLOOR: f32 = 0.6;
/// Guards against a zero/unknown floor being read as "everything is voiced".
const FLOOR_EPS: f32 = 1e-7;
/// ZCR value treated as most plausible for voiced/structured content.
const ZCR_CENTER: f32 = 0.15;
/// Half-width of the plausible ZCR band.
const ZCR_SPREAD: f32 = 0.4;

/// Binary voiced/unvoiced proposal for one frame.
///
/// Convenience wrapper over [`frame_voicing_score`]: `true` iff the fused
/// confidence reaches the decision threshold. `noise_floor` is the adaptive
/// floor (e.g. from [`crate::features::vad::noise_floor_min_stat`]); a frame at
/// or below it is always unvoiced.
pub fn frame_is_voiced(frame: &[f32], noise_floor: f32) -> bool {
    frame_voicing_score(frame, noise_floor) >= VOICE_DECISION
}

/// Fused voice-activity **confidence** in `[0, 1]` for one frame.
///
/// `0.0` means "at/below the noise floor" (silence — never voiced). Values grow
/// with how far the frame clears the floor (energy cue) and are scaled up by
/// spectral structure (low flatness) and a plausible zero-crossing rate. This is
/// the scored variant; [`frame_is_voiced`] thresholds it.
///
/// Zero-heap: uses fixed stack scratch for the optional FFT cue.
pub fn frame_voicing_score(frame: &[f32], noise_floor: f32) -> f32 {
    if frame.len() < 2 {
        return 0.0;
    }
    let floor = noise_floor.max(FLOOR_EPS);
    let e = frame_energy(frame);
    // Epistemic guard: at or below the estimated floor is silence, not speech.
    if !(e > floor) {
        return 0.0;
    }

    // Energy confidence from headroom over the floor, in dB.
    let snr_db = 20.0 * (e / floor).log10();
    let energy_conf = ((snr_db - ONSET_DB) / (FULL_DB - ONSET_DB)).clamp(0.0, 1.0);
    if energy_conf <= 0.0 {
        // Above the floor but within the onset margin: treat as not (yet) voiced.
        return 0.0;
    }

    // Spectral structure: tonality = 1 - flatness (falls back to neutral 0.5).
    let tonality = match frame_tonality(frame) {
        Some(t) => t,
        None => 0.5,
    };

    // Temporal plausibility from ZCR: peaks near ZCR_CENTER, decays outward.
    let z = frame_zcr(frame);
    let zcr_plaus = (1.0 - (z - ZCR_CENTER).abs() / ZCR_SPREAD).clamp(0.0, 1.0);

    let structure = 0.7 * tonality + 0.3 * zcr_plaus;
    let score = energy_conf * (STRUCT_FLOOR + (1.0 - STRUCT_FLOOR) * structure);
    score.clamp(0.0, 1.0)
}

/// Tonality in `[0, 1]` (`1 - spectral_flatness`) via a stack-buffered FFT.
///
/// Returns `None` when the spectral cue is unavailable (frame length not a power
/// of two, longer than [`MAX_VAD_FFT`], or an FFT/flatness error) so the caller
/// can fall back to a neutral value. Zero-heap: fixed-size stack scratch.
fn frame_tonality(frame: &[f32]) -> Option<f32> {
    let n = frame.len();
    if n < 2 || n > MAX_VAD_FFT || !n.is_power_of_two() {
        return None;
    }
    let bins = n / 2 + 1;
    let mut scratch = [0.0f32; 2 * MAX_VAD_FFT];
    let mut mags = [0.0f32; MAX_VAD_FFT / 2 + 1];
    real_fft_magnitude(frame, &mut scratch[..2 * n], &mut mags[..bins]).ok()?;
    // Nudge off exact zeros so flatness reflects the true peakiness rather than
    // collapsing to 0 on a single empty bin.
    for m in mags[..bins].iter_mut() {
        *m += 1e-9;
    }
    let flat = spectral_flatness_db(&mags[..bins], false).ok()?;
    Some((1.0 - flat).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::TAU;

    fn tone(n: usize, freq: f32, sr: f32, amp: f32) -> Vec<f32> {
        (0..n).map(|i| amp * (TAU * freq * i as f32 / sr).sin()).collect()
    }

    #[test]
    fn silent_frame_is_unvoiced() {
        let frame = [0.0f32; 512];
        assert!(!frame_is_voiced(&frame, 0.0));
        assert_eq!(frame_voicing_score(&frame, 0.0), 0.0);
        // Also unvoiced against a nonzero floor.
        assert!(!frame_is_voiced(&frame, 0.01));
    }

    #[test]
    fn strong_tone_above_floor_is_voiced() {
        let frame = tone(512, 440.0, 16_000.0, 0.5);
        let score = frame_voicing_score(&frame, 0.001);
        assert!(score >= VOICE_DECISION, "tone score={score}");
        assert!(frame_is_voiced(&frame, 0.001));
        assert!(score <= 1.0);
    }

    #[test]
    fn quiet_tone_at_floor_stays_unvoiced() {
        // Tone energy sits essentially at the floor → within onset margin → unvoiced.
        let frame = tone(512, 440.0, 16_000.0, 0.5);
        let e = frame_energy(&frame);
        assert!(!frame_is_voiced(&frame, e), "must not force a floor-level frame voiced");
        assert_eq!(frame_voicing_score(&frame, e), 0.0);
    }

    #[test]
    fn noise_burst_above_floor_is_still_voiced() {
        // Pseudo-random noise well above the floor: flat spectrum, but activity.
        let mut state = 0x1234_5678u32;
        let frame: Vec<f32> = (0..512)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 8_388_608.0) - 1.0 // ~[-1,1)
            })
            .collect();
        assert!(frame_is_voiced(&frame, 0.001), "noise burst should be detected as active");
    }

    #[test]
    fn confidence_grows_with_headroom() {
        let frame = tone(512, 440.0, 16_000.0, 0.5);
        let near = frame_voicing_score(&frame, 0.05); // little headroom
        let far = frame_voicing_score(&frame, 0.0005); // lots of headroom
        assert!(far >= near, "far={far} near={near}");
    }

    #[test]
    fn tonality_available_for_power_of_two() {
        let frame = tone(512, 440.0, 16_000.0, 0.5);
        let t = frame_tonality(&frame).expect("tonality for power-of-two frame");
        assert!(t > 0.5, "a pure tone should read as strongly tonal, got {t}");
    }

    #[test]
    fn tonality_none_for_non_power_of_two() {
        let frame = tone(500, 440.0, 16_000.0, 0.5);
        assert!(frame_tonality(&frame).is_none());
    }
}
