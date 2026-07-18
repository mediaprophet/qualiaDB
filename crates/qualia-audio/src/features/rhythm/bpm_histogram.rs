//! Histogram of inter-beat-interval tempi → dominant and secondary BPM. One
//! public function; caller supplies the histogram bins (zero-heap, no internal
//! allocation).

use crate::types::AudioError;

/// Minimum bin separation between the dominant and secondary peaks, so the two
/// reported tempi are distinct modes rather than two bins of one peak.
const MIN_PEAK_SEP: usize = 2;

/// BPM at the centre of histogram bin `b` (of `n` bins over `[bpm_min, bpm_max]`).
#[inline]
fn bin_center_bpm(b: usize, n_bins: usize, bpm_min: f32, bpm_max: f32) -> f32 {
    bpm_min + (b as f32 + 0.5) / (n_bins as f32) * (bpm_max - bpm_min)
}

/// Build a BPM histogram from beat frame indices and report the dominant and
/// secondary tempi.
///
/// Each adjacent beat pair contributes one inter-beat interval `d = beat[i+1] -
/// beat[i]` frames, an instantaneous tempo `bpm = frame_rate_hz*60/d`. Tempi
/// inside `[bpm_min, bpm_max]` are binned into `out_hist` (linearly, `out_hist`
/// spanning the whole range). The **dominant** tempo is the most-populated bin;
/// the **secondary** tempo is the most-populated bin at least [`MIN_PEAK_SEP`]
/// bins away from the dominant (a distinct mode, e.g. a half/double-time
/// alias), or `0.0` if there is no such bin. Both are returned as bin-centre
/// BPMs.
///
/// Returns `(dominant_bpm, secondary_bpm)`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if fewer than two beats, `out_hist` is
///   empty, `frame_rate_hz <= 0`, either bound is non-positive, or
///   `bpm_max <= bpm_min`.
pub fn bpm_histogram(
    beat_frames: &[u32],
    frame_rate_hz: f32,
    bpm_min: f32,
    bpm_max: f32,
    out_hist: &mut [u32],
) -> Result<(f32, f32), AudioError> {
    let n_bins = out_hist.len();
    if beat_frames.len() < 2
        || n_bins == 0
        || !(frame_rate_hz > 0.0)
        || !(bpm_min > 0.0)
        || !(bpm_max > bpm_min)
    {
        return Err(AudioError::InvalidParameter);
    }

    for h in out_hist.iter_mut() {
        *h = 0;
    }

    let span = bpm_max - bpm_min;
    let mut counted = 0u32;
    for pair in beat_frames.windows(2) {
        // Guard against non-monotone / duplicate beats.
        let d = pair[1].saturating_sub(pair[0]);
        if d == 0 {
            continue;
        }
        let bpm = frame_rate_hz * 60.0 / d as f32;
        if bpm < bpm_min || bpm > bpm_max {
            continue;
        }
        let mut bin = ((bpm - bpm_min) / span * n_bins as f32) as usize;
        if bin >= n_bins {
            bin = n_bins - 1; // bpm == bpm_max lands in the last bin
        }
        out_hist[bin] += 1;
        counted += 1;
    }

    if counted == 0 {
        // Beats exist but every interval fell outside the tempo range.
        return Err(AudioError::InvalidParameter);
    }

    // Dominant = most populated bin.
    let mut dom = 0usize;
    for b in 1..n_bins {
        if out_hist[b] > out_hist[dom] {
            dom = b;
        }
    }

    // Secondary = most populated bin at least MIN_PEAK_SEP away from dominant.
    let mut sec: Option<usize> = None;
    for b in 0..n_bins {
        if out_hist[b] == 0 {
            continue;
        }
        let far = b.abs_diff(dom) >= MIN_PEAK_SEP;
        if !far {
            continue;
        }
        match sec {
            Some(s) if out_hist[b] <= out_hist[s] => {}
            _ => sec = Some(b),
        }
    }

    let dominant_bpm = bin_center_bpm(dom, n_bins, bpm_min, bpm_max);
    let secondary_bpm = match sec {
        Some(s) => bin_center_bpm(s, n_bins, bpm_min, bpm_max),
        None => 0.0,
    };
    Ok((dominant_bpm, secondary_bpm))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Steady 120 BPM beats @ 100 Hz -> dominant ~120, no secondary.
    #[test]
    fn steady_tempo_dominant() {
        let frame_rate = 100.0f32;
        // Beats every 50 frames -> 120 BPM.
        let beats: Vec<u32> = (0..20).map(|k| (k * 50) as u32).collect();
        let mut hist = [0u32; 200]; // 40..240, 1 BPM/bin
        let (dom, sec) = bpm_histogram(&beats, frame_rate, 40.0, 240.0, &mut hist).expect("hist");
        assert!((dom - 120.0).abs() <= 120.0 * 0.05, "dominant = {dom}");
        assert_eq!(sec, 0.0, "no secondary expected, got {sec}");
    }

    /// Two interleaved intervals (mostly 120 BPM, some 60 BPM) -> dominant 120,
    /// secondary 60.
    #[test]
    fn bimodal_dominant_and_secondary() {
        let frame_rate = 100.0f32;
        // Intervals in frames: 50 (120 BPM) x many, 100 (60 BPM) x few.
        let intervals = [50u32, 50, 50, 50, 100, 50, 50, 100, 50, 50];
        let mut beats = vec![0u32];
        let mut acc = 0u32;
        for d in intervals {
            acc += d;
            beats.push(acc);
        }
        let mut hist = [0u32; 200];
        let (dom, sec) = bpm_histogram(&beats, frame_rate, 40.0, 240.0, &mut hist).expect("hist");
        assert!((dom - 120.0).abs() <= 120.0 * 0.05, "dominant = {dom}");
        assert!((sec - 60.0).abs() <= 60.0 * 0.05, "secondary = {sec}");
    }

    #[test]
    fn rejects_insufficient_beats() {
        let mut hist = [0u32; 32];
        assert_eq!(
            bpm_histogram(&[10], 100.0, 40.0, 240.0, &mut hist),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn out_of_range_intervals_error() {
        // Beats 1 frame apart @ 100 Hz -> 6000 BPM, outside 40..240.
        let beats = [0u32, 1, 2, 3];
        let mut hist = [0u32; 32];
        assert_eq!(
            bpm_histogram(&beats, 100.0, 40.0, 240.0, &mut hist),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn non_monotone_beats_skipped() {
        // A duplicate/backward beat must not panic or mis-bin.
        let beats = [0u32, 50, 50, 100, 150];
        let mut hist = [0u32; 200];
        let (dom, _sec) = bpm_histogram(&beats, 100.0, 40.0, 240.0, &mut hist).expect("hist");
        assert!((dom - 120.0).abs() <= 120.0 * 0.05, "dominant = {dom}");
    }
}
