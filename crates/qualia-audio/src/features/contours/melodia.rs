//! Predominant-melody contour selection (Melodia-class).
//!
//! After tracking (`contour_tracking`), a frame may be covered by several
//! competing contours. Melodia's melody stage chooses the contour that is both
//! **salient** (loud, harmonically strong) and **smooth** (low pitch variance —
//! melodies move stepwise, not erratically), rejecting octave-jumping or noisy
//! candidates.
//!
//! This selects, among a set of candidate contours summarised by their mean
//! salience and their pitch variance, the index of the predominant-melody
//! contour. It encodes an explicitly **monophonic** reading: exactly one melody
//! line is chosen. In genuinely polyphonic material that assumption is a
//! *proposal*, not a claim that only one voice exists.
//!
//! Zero-heap: reads two caller slices, returns an index; allocates nothing.

use crate::types::AudioError;

/// Choose the predominant-melody contour among `n` candidates.
///
/// Each candidate `i` is summarised by `mean_salience[i]` (higher = stronger)
/// and `pitch_variance[i]` in squared bins (higher = more erratic pitch). The
/// selection score normalises both across the candidate set and combines them:
///
/// ```text
/// score(i) = salience_weight * (salience[i] / max_salience)
///          + (1 - salience_weight) * (1 - variance[i] / max_variance)
/// ```
///
/// so a candidate is preferred when it is loud *and* smooth. The index of the
/// highest score is returned (ties resolve to the lowest index).
///
/// - `salience_weight`: relative importance of salience vs. smoothness, in
///   `[0, 1]`. `1.0` selects purely on salience; `0.0` purely on smoothness.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `n == 0`, either slice is shorter than
///   `n`, `salience_weight` is not a finite value in `[0, 1]`, or any summarised
///   value is non-finite / negative.
pub fn select_melody_contour(
    mean_salience: &[f32],
    pitch_variance: &[f32],
    n: usize,
    salience_weight: f32,
) -> Result<usize, AudioError> {
    if n == 0
        || mean_salience.len() < n
        || pitch_variance.len() < n
        || !salience_weight.is_finite()
        || !(0.0..=1.0).contains(&salience_weight)
    {
        return Err(AudioError::InvalidParameter);
    }

    let mut max_sal = 0.0f32;
    let mut max_var = 0.0f32;
    for i in 0..n {
        let s = mean_salience[i];
        let v = pitch_variance[i];
        if !(s.is_finite() && s >= 0.0 && v.is_finite() && v >= 0.0) {
            return Err(AudioError::InvalidParameter);
        }
        if s > max_sal {
            max_sal = s;
        }
        if v > max_var {
            max_var = v;
        }
    }

    let mut best_i = 0usize;
    let mut best_score = f32::NEG_INFINITY;
    for i in 0..n {
        // Normalise; guard degenerate (all-equal) sets.
        let s_norm = if max_sal > 0.0 {
            mean_salience[i] / max_sal
        } else {
            0.0
        };
        let smoothness = if max_var > 0.0 {
            1.0 - pitch_variance[i] / max_var
        } else {
            1.0 // all equally smooth
        };
        let score = salience_weight * s_norm + (1.0 - salience_weight) * smoothness;
        if score > best_score {
            best_score = score;
            best_i = i;
        }
    }
    Ok(best_i)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With equal weight, a loud-but-erratic contour loses to a slightly quieter
    /// but far smoother one — the melody heuristic favours smooth pitch.
    #[test]
    fn prefers_smooth_over_erratic() {
        // idx0: loud (1.0) but very erratic (var 100). idx1: medium (0.85) smooth (var 1).
        let sal = [1.0f32, 0.85];
        let var = [100.0f32, 1.0];
        let i = select_melody_contour(&sal, &var, 2, 0.5).expect("select");
        assert_eq!(i, 1, "smooth medium-salience contour is the melody");
    }

    /// Pure-salience weight ignores smoothness and takes the loudest.
    #[test]
    fn salience_only_takes_loudest() {
        let sal = [0.3f32, 0.9, 0.5];
        let var = [1.0f32, 50.0, 2.0];
        let i = select_melody_contour(&sal, &var, 3, 1.0).expect("select");
        assert_eq!(i, 1);
    }

    /// Pure-smoothness weight takes the lowest-variance contour.
    #[test]
    fn smoothness_only_takes_flattest() {
        let sal = [0.9f32, 0.2, 0.5];
        let var = [40.0f32, 0.5, 10.0];
        let i = select_melody_contour(&sal, &var, 3, 0.0).expect("select");
        assert_eq!(i, 1);
    }

    #[test]
    fn rejects_bad_params() {
        assert_eq!(
            select_melody_contour(&[], &[], 0, 0.5),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            select_melody_contour(&[1.0], &[1.0], 1, 2.0),
            Err(AudioError::InvalidParameter)
        );
    }
}
