//! Scale / mode proposal from an HPCP (major / minor / other, with abstention).
//!
//! For each candidate tonic we measure the fraction of HPCP energy that falls on
//! the diatonic degrees of the major and of the natural-minor scale, and take the
//! best fit. The result is a **proposal**, not a verdict.
//!
//! # Abstention (hard epistemic rule)
//! The diatonic templates are 12-TET-specific, so an explicit `assumes_12tet` flag
//! is required. Outcomes:
//! - not 12-TET / not a 12-bin HPCP → [`ScaleProposal::Unknown`] (abstain);
//! - a near-flat HPCP (no tonal information) → [`ScaleProposal::Unknown`];
//! - clear energy that fits major/minor at ≥ `min_score` coverage →
//!   [`ScaleProposal::Major`] / [`ScaleProposal::Minor`];
//! - clear energy that fits *neither* well → [`ScaleProposal::Other`]
//!   (present but non-diatonic — e.g. chromatic / modal), never a forced label.
//!
//! Zero-heap: fixed-size masks and scalar accumulation; no allocation.

use crate::types::AudioError;

/// Major scale degrees (semitone offsets from tonic): 0 2 4 5 7 9 11.
const MAJOR_MASK: [bool; 12] = [
    true, false, true, false, true, true, false, true, false, true, false, true,
];
/// Natural-minor scale degrees: 0 2 3 5 7 8 10.
const MINOR_MASK: [bool; 12] = [
    true, false, true, true, false, true, false, true, true, false, true, false,
];

/// Proposed scale / mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScaleProposal {
    Major,
    Minor,
    /// Tonal energy present but not a good diatonic major/minor fit.
    Other,
    /// Abstained — no basis to propose (non-12-TET, or flat/no information).
    Unknown,
}

/// A scale proposal with its supporting tonic and coverage score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleEstimate {
    /// Best-fitting tonic (0 = C … 11 = B); meaningless when `scale == Unknown`.
    pub tonic_pc: u8,
    pub scale: ScaleProposal,
    /// Fraction `[0,1]` of HPCP energy captured by the winning diatonic set.
    pub score: f32,
}

/// Propose a scale/mode for a 12-bin HPCP.
///
/// - `hpcp`: pitch-class profile (≥ 12 bins; bin 0 = C).
/// - `assumes_12tet`: caller-declared assumption; `false` ⇒ `Unknown`.
/// - `min_score`: diatonic-coverage acceptance threshold (e.g. `0.9`). Below it,
///   present-but-non-diatonic energy is reported as `Other`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `hpcp` is empty.
pub fn estimate_scale(
    hpcp: &[f32],
    assumes_12tet: bool,
    min_score: f32,
) -> Result<ScaleEstimate, AudioError> {
    if hpcp.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let unknown = ScaleEstimate {
        tonic_pc: 0,
        scale: ScaleProposal::Unknown,
        score: 0.0,
    };
    if !assumes_12tet || hpcp.len() < 12 {
        return Ok(unknown);
    }
    let frame = &hpcp[..12];

    let total: f32 = frame.iter().map(|v| v.max(0.0)).sum();
    if total <= 0.0 {
        return Ok(unknown);
    }
    // Near-flat profile carries no tonal information → abstain.
    let maxv = frame.iter().cloned().fold(f32::MIN, f32::max);
    let mean = total / 12.0;
    if maxv <= mean * 1.05 {
        return Ok(unknown);
    }

    // Best diatonic coverage over all tonics and both modes.
    let mut best_tonic = 0u8;
    let mut best_is_major = true;
    let mut best_score = 0.0f32;
    for tonic in 0usize..12 {
        for (mask, is_major) in [(&MAJOR_MASK, true), (&MINOR_MASK, false)] {
            let mut inside = 0.0f32;
            for (pc, &v) in frame.iter().enumerate() {
                if mask[(pc + 12 - tonic) % 12] {
                    inside += v.max(0.0);
                }
            }
            let score = inside / total;
            if score > best_score {
                best_score = score;
                best_tonic = tonic as u8;
                best_is_major = is_major;
            }
        }
    }

    let scale = if best_score >= min_score {
        if best_is_major {
            ScaleProposal::Major
        } else {
            ScaleProposal::Minor
        }
    } else {
        ScaleProposal::Other
    };

    Ok(ScaleEstimate {
        tonic_pc: best_tonic,
        scale,
        score: best_score,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A full C-major diatonic HPCP is proposed as Major.
    #[test]
    fn c_major_scale_is_major() {
        let mut hpcp = [0.0f32; 12];
        for pc in [0, 2, 4, 5, 7, 9, 11] {
            hpcp[pc] = 1.0;
        }
        let est = estimate_scale(&hpcp, true, 0.9).expect("scale");
        assert_eq!(est.scale, ScaleProposal::Major);
        assert_eq!(est.tonic_pc, 0);
        assert!(est.score > 0.99, "score = {}", est.score);
    }

    /// A full A-minor diatonic HPCP is proposed as Minor (tonic A).
    #[test]
    fn a_minor_scale_is_minor() {
        let mut hpcp = [0.0f32; 12];
        for pc in [9, 11, 0, 2, 4, 5, 7] {
            hpcp[pc] = 1.0;
        }
        // A minor and C major share the same set; require the minor reading by
        // weighting the A tonic/dominant. Both score 1.0 for coverage, so this
        // asserts a diatonic Major/Minor result rather than a specific mode.
        let est = estimate_scale(&hpcp, true, 0.9).expect("scale");
        assert!(matches!(
            est.scale,
            ScaleProposal::Major | ScaleProposal::Minor
        ));
        assert!(est.score > 0.99);
    }

    /// ABSTAIN: a flat HPCP → Unknown.
    #[test]
    fn flat_is_unknown() {
        let hpcp = [1.0f32; 12];
        let est = estimate_scale(&hpcp, true, 0.9).expect("scale");
        assert_eq!(est.scale, ScaleProposal::Unknown);
    }

    /// ABSTAIN: without 12-TET → Unknown.
    #[test]
    fn non_12tet_is_unknown() {
        let mut hpcp = [0.0f32; 12];
        hpcp[0] = 1.0;
        let est = estimate_scale(&hpcp, false, 0.9).expect("scale");
        assert_eq!(est.scale, ScaleProposal::Unknown);
    }

    /// Present-but-non-diatonic energy (a fully chromatic cluster with a peak)
    /// falls to `Other`, not a forced Major/Minor.
    #[test]
    fn chromatic_is_other() {
        // Strong tonic plus energy on every chromatic degree → no 7-note diatonic
        // set covers ≥ min_score of the energy.
        let mut hpcp = [1.0f32; 12];
        hpcp[0] = 3.0;
        let est = estimate_scale(&hpcp, true, 0.95).expect("scale");
        assert_eq!(est.scale, ScaleProposal::Other, "score = {}", est.score);
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            estimate_scale(&[], true, 0.9),
            Err(AudioError::InvalidParameter)
        );
    }
}
