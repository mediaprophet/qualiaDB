//! Key estimation by Krumhansl–Schmuckler profile correlation over an HPCP.
//!
//! The Krumhansl–Kessler major/minor tonal-hierarchy profiles are correlated
//! against the (12-bin) HPCP at all 12 rotations; the best-correlating
//! (tonic, mode) is the proposed key.
//!
//! # Proposal, with abstention (hard epistemic rule)
//! The K–S profiles are **12-TET-specific**. This function therefore takes an
//! explicit `assumes_12tet` flag and **abstains** (`Ok(None)`) when 12-TET is not
//! asserted, when the HPCP is not 12-bin, or when the best correlation falls below
//! `min_correlation` (flat / ambiguous / atonal material). It never forces a key
//! label onto content that does not support one.
//!
//! Zero-heap: rotation happens in a stack `[f32; 12]`; no allocation.

use crate::types::AudioError;

/// Krumhansl–Kessler major-key tonal-hierarchy profile (tonic at index 0).
const KS_MAJOR: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
/// Krumhansl–Kessler minor-key tonal-hierarchy profile (tonic at index 0).
const KS_MINOR: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// A proposed key: tonic pitch class (0 = C … 11 = B), mode, and the correlation
/// that supports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyEstimate {
    pub tonic_pc: u8,
    pub is_major: bool,
    pub correlation: f32,
}

/// Pearson correlation of two equal-length slices; `0.0` if either has no
/// variance (e.g. a flat HPCP), which drives abstention rather than a spurious key.
fn pearson(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    if n == 0 || b.len() != n {
        return 0.0;
    }
    let inv = 1.0 / n as f32;
    let ma = a.iter().sum::<f32>() * inv;
    let mb = b.iter().sum::<f32>() * inv;
    let (mut num, mut da, mut db) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..n {
        let x = a[i] - ma;
        let y = b[i] - mb;
        num += x * y;
        da += x * x;
        db += y * y;
    }
    let den = (da * db).sqrt();
    if den <= 0.0 || !den.is_finite() {
        return 0.0;
    }
    num / den
}

/// Estimate the musical key of a 12-bin HPCP frame/profile.
///
/// - `hpcp`: pitch-class profile; must be at least 12 bins (bin 0 = C).
/// - `assumes_12tet`: caller-declared assumption. If `false`, abstains.
/// - `min_correlation`: acceptance threshold; below it the result is `Ok(None)`
///   (abstain). A value around `0.6`–`0.7` is typical for confident keys.
///
/// Returns `Ok(Some(KeyEstimate))` when a key is proposed, `Ok(None)` when the
/// function abstains.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `hpcp` is empty.
pub fn estimate_key(
    hpcp: &[f32],
    assumes_12tet: bool,
    min_correlation: f32,
) -> Result<Option<KeyEstimate>, AudioError> {
    if hpcp.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    // Abstain unless the 12-TET assumption holds and the profile is chromatic.
    if !assumes_12tet || hpcp.len() < 12 {
        return Ok(None);
    }
    let frame = &hpcp[..12];

    let mut best = KeyEstimate {
        tonic_pc: 0,
        is_major: true,
        correlation: f32::MIN,
    };
    for tonic in 0usize..12 {
        for (profile, is_major) in [(&KS_MAJOR, true), (&KS_MINOR, false)] {
            // Rotate the profile so its tonic aligns with `tonic` (stack buffer).
            let mut rot = [0.0f32; 12];
            for (pc, slot) in rot.iter_mut().enumerate() {
                *slot = profile[(pc + 12 - tonic) % 12];
            }
            let c = pearson(frame, &rot);
            if c > best.correlation {
                best = KeyEstimate {
                    tonic_pc: tonic as u8,
                    is_major,
                    correlation: c,
                };
            }
        }
    }

    if best.correlation < min_correlation {
        Ok(None) // abstain — no key confident enough.
    } else {
        Ok(Some(best))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GOLDEN: an HPCP equal to the C-major profile itself yields tonic C, major,
    /// with a strongly positive correlation.
    #[test]
    fn c_major_profile_returns_c_major() {
        let est = estimate_key(&KS_MAJOR, true, 0.5)
            .expect("key")
            .expect("some key");
        assert_eq!(est.tonic_pc, 0, "tonic should be C");
        assert!(est.is_major, "should be major");
        assert!(est.correlation > 0.9, "corr = {}", est.correlation);
    }

    /// A C-major *triad* HPCP (energy only at C, E, G) still proposes C major with
    /// positive correlation.
    #[test]
    fn c_major_triad_returns_c_major() {
        let mut hpcp = [0.0f32; 12];
        hpcp[0] = 1.0; // C
        hpcp[4] = 1.0; // E
        hpcp[7] = 1.0; // G
        let est = estimate_key(&hpcp, true, 0.3)
            .expect("key")
            .expect("some key");
        assert_eq!(est.tonic_pc, 0);
        assert!(est.is_major);
        assert!(est.correlation > 0.0, "corr = {}", est.correlation);
    }

    /// An A-minor triad (A, C, E) proposes A minor (tonic 9, minor).
    #[test]
    fn a_minor_triad_returns_a_minor() {
        let mut hpcp = [0.0f32; 12];
        hpcp[9] = 1.0; // A
        hpcp[0] = 1.0; // C
        hpcp[4] = 1.0; // E
        let est = estimate_key(&hpcp, true, 0.3)
            .expect("key")
            .expect("some key");
        assert_eq!(est.tonic_pc, 9, "tonic should be A");
        assert!(!est.is_major, "should be minor");
    }

    /// ABSTAIN: a flat HPCP has no key.
    #[test]
    fn flat_hpcp_abstains() {
        let hpcp = [1.0f32; 12];
        assert!(estimate_key(&hpcp, true, 0.5).expect("key").is_none());
    }

    /// ABSTAIN: without the 12-TET assumption the function refuses to guess.
    #[test]
    fn abstains_without_12tet() {
        assert!(estimate_key(&KS_MAJOR, false, 0.1).expect("key").is_none());
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(
            estimate_key(&[], true, 0.5),
            Err(AudioError::InvalidParameter)
        );
    }
}
