//! Global-tempo estimation and beat placement from an onset-strength envelope.
//! Estimates BPM from the autocorrelation tempogram (peak-picked with
//! [`detect_peaks`]) and places beats with a comb-filter phase search. One
//! public function; the only working state is bounded fixed-size stack scratch.

use crate::features::peaks::detect_peaks;
use crate::features::rhythm::tempogram::tempogram;
use crate::types::AudioError;

/// Tempo search range (BPM). Covers the musically common band; the low bound is
/// raised automatically when the frame rate would need a lag beyond `ACF_CAP`.
const BPM_MIN: f32 = 40.0;
const BPM_MAX: f32 = 240.0;
/// Salience bins over `[BPM_MIN, BPM_MAX]` → 1 BPM resolution.
const SAL_BINS: usize = 201;
/// Autocorrelation lag ceiling (frames). Bounds stack scratch to 16 KiB.
const ACF_CAP: usize = 4096;
/// Peak-list capacity: 201 samples admit at most 100 local maxima.
const MAX_PEAKS: usize = 101;

/// Estimate global tempo and place beat frames for `onset_env`.
///
/// `onset_env` is a per-frame onset-strength curve (e.g. the spectral-flux
/// novelty behind [`super::spectral_flux_onset::onset_detection`]); `frame_rate_hz`
/// is its sampling rate (`sample_rate / hop`). The tempo is the dominant peak of
/// the autocorrelation [`tempogram`] (highest-salience [`detect_peaks`] maximum,
/// falling back to the salience argmax). Given the beat period
/// `P = frame_rate_hz*60/bpm` frames, a comb filter scores every integer phase
/// offset `o ∈ [0, round(P))` by the summed onset strength at
/// `o, o+P, o+2P, …` and keeps the best; beats are then laid at that phase and
/// period and written (as frame indices) to `out_beat_frames`.
///
/// If the frame rate is so high that `BPM_MIN`'s lag exceeds `ACF_CAP`, the
/// effective low bound is raised to keep the lag in range (documented, not
/// silent failure — the returned BPM reflects the searched band).
///
/// Returns `(n_beats, bpm)`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `onset_env` is empty, `frame_rate_hz`
///   is non-positive, or the frame rate is so extreme the search band collapses.
/// - [`AudioError::OutputBufferTooSmall`] if more beats are found than
///   `out_beat_frames` can hold.
pub fn track_beats(
    onset_env: &[f32],
    frame_rate_hz: f32,
    out_beat_frames: &mut [u32],
) -> Result<(usize, f32), AudioError> {
    if onset_env.is_empty() || !(frame_rate_hz > 0.0) {
        return Err(AudioError::InvalidParameter);
    }

    // Keep the slowest-tempo lag within the stack scratch.
    let eff_bpm_min = BPM_MIN.max(frame_rate_hz * 60.0 / ((ACF_CAP - 2) as f32));
    if !(eff_bpm_min < BPM_MAX) {
        return Err(AudioError::InvalidParameter);
    }

    let mut sal = [0.0f32; SAL_BINS];
    let mut scratch = [0.0f32; ACF_CAP];
    let bins = tempogram(
        onset_env,
        frame_rate_hz,
        eff_bpm_min,
        BPM_MAX,
        &mut sal,
        &mut scratch,
    )?;

    // Dominant tempo: strongest local maximum of the salience curve.
    let mut pos = [0.0f32; MAX_PEAKS];
    let mut mag = [0.0f32; MAX_PEAKS];
    let peak_bin = match detect_peaks(&sal[..bins], 0.0, 2, &mut pos, &mut mag) {
        Ok(np) if np > 0 => {
            let mut best = 0usize;
            for k in 1..np {
                if mag[k] > mag[best] {
                    best = k;
                }
            }
            pos[best]
        }
        // No clean peak (or overflow) → fall back to the salience argmax.
        _ => {
            let mut best = 0usize;
            for b in 1..bins {
                if sal[b] > sal[best] {
                    best = b;
                }
            }
            best as f32
        }
    };

    let bpm = if bins > 1 {
        eff_bpm_min + (BPM_MAX - eff_bpm_min) * peak_bin / ((bins - 1) as f32)
    } else {
        eff_bpm_min
    };
    if !(bpm > 0.0) {
        return Err(AudioError::InvalidParameter);
    }

    let period = frame_rate_hz * 60.0 / bpm;
    let n_beats = place_beats(onset_env, period, out_beat_frames)?;
    Ok((n_beats, bpm))
}

/// Comb-filter phase alignment + beat emission. Chooses the integer phase offset
/// maximising summed onset strength on the pulse grid, then writes beat frame
/// indices spaced by `period`.
fn place_beats(
    onset_env: &[f32],
    period: f32,
    out: &mut [u32],
) -> Result<usize, AudioError> {
    let len = onset_env.len();
    let step = period.max(1.0);
    let p_round = (step.round() as usize).max(1);

    // Best phase offset in [0, p_round).
    let mut best_off = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    for off in 0..p_round {
        let mut score = 0.0f32;
        let mut p = off as f32;
        loop {
            let idx = p.round() as usize;
            if idx >= len {
                break;
            }
            score += onset_env[idx];
            p += step;
        }
        if score > best_score {
            best_score = score;
            best_off = off;
        }
    }

    // Emit beats at the chosen phase and period.
    let mut w = 0usize;
    let mut p = best_off as f32;
    loop {
        let idx = p.round() as usize;
        if idx >= len {
            break;
        }
        if w == out.len() {
            return Err(AudioError::OutputBufferTooSmall);
        }
        out[w] = idx as u32;
        w += 1;
        p += step;
    }
    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Impulse train onset envelope at a known BPM.
    fn click_train(n: usize, period: usize) -> Vec<f32> {
        let mut env = vec![0.0f32; n];
        let mut i = 0;
        while i < n {
            env[i] = 1.0;
            i += period;
        }
        env
    }

    /// GOLDEN: 120 BPM click train @ 100 Hz frame rate -> recover ~120 BPM.
    #[test]
    fn recovers_120_bpm() {
        let frame_rate = 100.0f32; // 100 frames/s
        let period = 50usize; // 0.5 s -> 120 BPM
        let env = click_train(1000, period);
        let mut beats = [0u32; 64];
        let (n_beats, bpm) = track_beats(&env, frame_rate, &mut beats).expect("track");
        assert!((bpm - 120.0).abs() <= 120.0 * 0.05, "bpm = {bpm} (want ~120)");
        // 1000 frames / 50 -> 20 beats.
        assert_eq!(n_beats, 20, "n_beats = {n_beats}");
        // Beats land on the click grid.
        assert_eq!(beats[0], 0);
        assert_eq!(beats[1], 50);
        assert_eq!(beats[2], 100);
    }

    /// A second tempo to prove it is not hard-wired to 120.
    #[test]
    fn recovers_100_bpm() {
        let frame_rate = 100.0f32;
        let period = 60usize; // 0.6 s -> 100 BPM
        let env = click_train(1200, period);
        let mut beats = [0u32; 64];
        let (_n, bpm) = track_beats(&env, frame_rate, &mut beats).expect("track");
        assert!((bpm - 100.0).abs() <= 100.0 * 0.05, "bpm = {bpm} (want ~100)");
    }

    /// Beats sit near real onsets even with an off-grid starting phase.
    #[test]
    fn phase_locks_to_offset_onsets() {
        let frame_rate = 100.0f32;
        let period = 50usize;
        // First click at frame 17, then every 50.
        let mut env = vec![0.0f32; 1000];
        let mut i = 17usize;
        while i < 1000 {
            env[i] = 1.0;
            i += period;
        }
        let mut beats = [0u32; 64];
        let (n_beats, bpm) = track_beats(&env, frame_rate, &mut beats).expect("track");
        assert!((bpm - 120.0).abs() <= 120.0 * 0.05, "bpm = {bpm}");
        assert!(n_beats > 0);
        assert_eq!(beats[0], 17, "first beat phase = {}", beats[0]);
    }

    #[test]
    fn rejects_bad_input() {
        let mut beats = [0u32; 8];
        assert_eq!(
            track_beats(&[], 100.0, &mut beats),
            Err(AudioError::InvalidParameter)
        );
        let env = [1.0f32, 0.0, 1.0];
        assert_eq!(
            track_beats(&env, 0.0, &mut beats),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn beat_buffer_too_small_errors() {
        let env = click_train(1000, 50);
        let mut beats = [0u32; 4]; // 20 beats will not fit
        assert_eq!(
            track_beats(&env, 100.0, &mut beats),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn silence_bounds_gracefully() {
        // Flat non-zero envelope: autocorrelation exists but carries no clear
        // period; must still return a valid (n_beats, bpm) without panicking.
        let env = vec![1.0f32; 500];
        let mut beats = [0u32; 512];
        let res = track_beats(&env, 100.0, &mut beats);
        assert!(res.is_ok());
        let (_n, bpm) = res.expect("ok");
        assert!(bpm >= BPM_MIN && bpm <= BPM_MAX, "bpm = {bpm}");
    }
}
