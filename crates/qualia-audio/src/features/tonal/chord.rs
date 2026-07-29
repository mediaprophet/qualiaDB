//! Simple triad chord proposal from a single HPCP frame (major/minor/dim).
//!
//! For each root (0..12) and each triad template — major {0,4,7}, minor {0,3,7},
//! diminished {0,3,6} — we score the fraction of HPCP energy captured by the three
//! chord tones, and take the best. The result is a **proposal**.
//!
//! # Abstention (hard epistemic rule)
//! Triad templates are 12-TET-specific, so an explicit `assumes_12tet` flag is
//! required. The function returns `Ok(None)` (abstain) when 12-TET is not asserted,
//! when the HPCP is not 12-bin, when the frame is flat/empty, or when the best
//! triad captures less than `min_score` of the energy. It never forces a chord.
//!
//! Zero-heap: fixed templates and scalar accumulation; no allocation.

use crate::types::AudioError;

/// Triad quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordQuality {
    Major,
    Minor,
    Diminished,
}

/// A proposed triad: root pitch class, quality, and the energy-coverage score.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChordEstimate {
    /// Root pitch class (0 = C … 11 = B).
    pub root_pc: u8,
    pub quality: ChordQuality,
    /// Fraction `[0,1]` of HPCP energy captured by the three chord tones.
    pub score: f32,
}

/// (quality, third-interval, fifth-interval) in semitones above the root.
const TEMPLATES: [(ChordQuality, usize, usize); 3] = [
    (ChordQuality::Major, 4, 7),
    (ChordQuality::Minor, 3, 7),
    (ChordQuality::Diminished, 3, 6),
];

/// Propose a triad for a 12-bin HPCP frame.
///
/// - `hpcp`: pitch-class profile (≥ 12 bins; bin 0 = C).
/// - `assumes_12tet`: caller-declared assumption; `false` ⇒ abstain (`Ok(None)`).
/// - `min_score`: coverage acceptance threshold (e.g. `0.8`). Below it, abstain.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `hpcp` is empty.
pub fn estimate_chord(
    hpcp: &[f32],
    assumes_12tet: bool,
    min_score: f32,
) -> Result<Option<ChordEstimate>, AudioError> {
    if hpcp.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    if !assumes_12tet || hpcp.len() < 12 {
        return Ok(None);
    }
    let frame = &hpcp[..12];

    let total: f32 = frame.iter().map(|v| v.max(0.0)).sum();
    if total <= 0.0 {
        return Ok(None);
    }

    let mut best = ChordEstimate {
        root_pc: 0,
        quality: ChordQuality::Major,
        score: 0.0,
    };
    for root in 0usize..12 {
        for (quality, third, fifth) in TEMPLATES {
            let e = frame[root].max(0.0)
                + frame[(root + third) % 12].max(0.0)
                + frame[(root + fifth) % 12].max(0.0);
            let score = e / total;
            if score > best.score {
                best = ChordEstimate {
                    root_pc: root as u8,
                    quality,
                    score,
                };
            }
        }
    }

    if best.score < min_score {
        Ok(None) // abstain — no triad captures enough energy.
    } else {
        Ok(Some(best))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GOLDEN: a C-E-G HPCP is proposed as a C major triad.
    #[test]
    fn c_e_g_is_c_major() {
        let mut hpcp = [0.0f32; 12];
        hpcp[0] = 1.0; // C
        hpcp[4] = 1.0; // E
        hpcp[7] = 1.0; // G
        let est = estimate_chord(&hpcp, true, 0.8)
            .expect("chord")
            .expect("some chord");
        assert_eq!(est.root_pc, 0);
        assert_eq!(est.quality, ChordQuality::Major);
        assert!(est.score > 0.99, "score = {}", est.score);
    }

    /// A-C-E is proposed as an A minor triad.
    #[test]
    fn a_c_e_is_a_minor() {
        let mut hpcp = [0.0f32; 12];
        hpcp[9] = 1.0; // A
        hpcp[0] = 1.0; // C
        hpcp[4] = 1.0; // E
        let est = estimate_chord(&hpcp, true, 0.8)
            .expect("chord")
            .expect("some chord");
        assert_eq!(est.root_pc, 9);
        assert_eq!(est.quality, ChordQuality::Minor);
    }

    /// B-D-F is proposed as a B diminished triad.
    #[test]
    fn b_d_f_is_b_diminished() {
        let mut hpcp = [0.0f32; 12];
        hpcp[11] = 1.0; // B
        hpcp[2] = 1.0; // D
        hpcp[5] = 1.0; // F
        let est = estimate_chord(&hpcp, true, 0.8)
            .expect("chord")
            .expect("some chord");
        assert_eq!(est.root_pc, 11);
        assert_eq!(est.quality, ChordQuality::Diminished);
    }

    /// ABSTAIN: a flat HPCP → no chord.
    #[test]
    fn flat_abstains() {
        let hpcp = [1.0f32; 12];
        assert!(estimate_chord(&hpcp, true, 0.8).expect("chord").is_none());
    }

    /// ABSTAIN: without 12-TET → no chord.
    #[test]
    fn non_12tet_abstains() {
        let mut hpcp = [0.0f32; 12];
        hpcp[0] = 1.0;
        hpcp[4] = 1.0;
        hpcp[7] = 1.0;
        assert!(estimate_chord(&hpcp, false, 0.8).expect("chord").is_none());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            estimate_chord(&[], true, 0.8),
            Err(AudioError::InvalidParameter)
        );
    }
}
