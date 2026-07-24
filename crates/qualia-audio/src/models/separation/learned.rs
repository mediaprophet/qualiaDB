//! Learned (demucs / open-unmix / band-split RNN class) source separation — **fail-closed stub**.
//!
//! A learned separator estimates per-source masks (or waveforms) from a neural network whose
//! weights are not present in this build. Per the project's honesty rule, this module must never
//! fabricate stems: with no weights loaded there is no faithful separation to return, so the only
//! correct behaviour is to abstain. [`separate_learned`] therefore always returns
//! [`AudioError::BackendUnavailable`] (the `NeedsWeights` signal).
//!
//! When real weights are wired (a P64 head, gated on the principal per ADR 007 / delivery plan
//! Waves 7–8), this stub is replaced by the actual inference path. Until then, callers get an
//! explicit, honest failure rather than plausible-looking noise. For weight-free separation use
//! the classical masking in [`crate::models::separation::apply_soft_mask`].

use crate::types::AudioError;

/// Attempt learned, demucs-class source separation of a mixture magnitude spectrum into
/// `n_stems` stems written contiguously into `out_stems` (`n_stems × mixture_mag.len()`).
///
/// **Always fails closed.** No weights are bundled, so this returns
/// [`AudioError::BackendUnavailable`] without touching `out_stems` — it never fabricates stems.
///
/// # Errors
/// Always [`AudioError::BackendUnavailable`] (NeedsWeights).
pub fn separate_learned(
    mixture_mag: &[f32],
    n_stems: usize,
    out_stems: &mut [f32],
) -> Result<usize, AudioError> {
    // Bind the parameters so the signature is honest about what a real backend would consume,
    // and so the stub cannot silently write partial output.
    let _ = (mixture_mag, n_stems, &out_stems);
    Err(AudioError::BackendUnavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_fails_closed_with_backend_unavailable() {
        let mix = [1.0f32, 2.0, 3.0];
        let mut out = [0.0f32; 6]; // room for 2 stems
        let r = separate_learned(&mix, 2, &mut out);
        assert_eq!(r, Err(AudioError::BackendUnavailable));
    }

    #[test]
    fn never_writes_fabricated_stems() {
        let mix = [1.0f32, 2.0, 3.0];
        let mut out = [7.0f32; 6];
        let _ = separate_learned(&mix, 2, &mut out);
        // Output buffer untouched — no invented audio.
        assert!(
            out.iter().all(|&x| x == 7.0),
            "stub must not fabricate stems"
        );
    }
}
