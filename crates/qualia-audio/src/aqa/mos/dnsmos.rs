//! DNSMOS — non-intrusive (no-reference) learned MOS predictor. **NeedsWeights.**
//!
//! DNSMOS (Microsoft, ICASSP 2021) is a *learned* model: a CNN trained on
//! crowd-sourced ratings predicts a Mean Opinion Score from a single degraded
//! clip, with no clean reference. There is no closed-form, hand-written way to
//! produce that number — the score IS the network. Without the trained weights we
//! cannot compute a MOS, and the project rule is absolute: **never fabricate a
//! MOS.**
//!
//! Therefore this function **fails closed**: with no weights it returns
//! [`AudioError::BackendUnavailable`]. It does not, under any circumstance, return
//! an invented number. When weights are supplied the loader is likewise not yet
//! present, so it also fails closed rather than guessing — surfaced as
//! `BackendUnavailable` (backend not wired), not as a fake score.

use crate::types::AudioError;

/// Predict a DNSMOS quality score for `signal`.
///
/// - `signal`: mono degraded audio (no reference needed).
/// - `weights`: the trained DNSMOS model weights. `None` (or, currently, any
///   value) means the learned backend is unavailable.
///
/// # Fails closed
///
/// Returns [`AudioError::BackendUnavailable`] when `weights` is `None`. A learned
/// MOS cannot be synthesised without the model, and fabricating one is forbidden.
/// A future weights loader will replace the `Some(_)` arm; until then it too fails
/// closed rather than returning a guessed MOS.
pub fn dnsmos(signal: &[f32], weights: Option<&[u8]>) -> Result<f32, AudioError> {
    // Reject obviously empty input before considering the backend.
    if signal.is_empty() {
        return Err(AudioError::MalformedAudio);
    }

    match weights {
        // No model → no MOS. Fail closed; NEVER fabricate a number.
        None => Err(AudioError::BackendUnavailable),
        // Weights provided but the learned inference backend is not yet wired.
        // Still fail closed — do not invent a score from unvalidated bytes.
        Some(_bytes) => Err(AudioError::BackendUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_weights_fails_closed_never_returns_mos() {
        let sig = vec![0.1f32, -0.2, 0.3, -0.4, 0.5];
        let out = dnsmos(&sig, None);
        assert_eq!(
            out,
            Err(AudioError::BackendUnavailable),
            "DNSMOS with no weights must fail closed, not return a MOS"
        );
        // Explicitly assert it is NOT any numeric MOS.
        assert!(out.is_err());
    }

    #[test]
    fn weights_present_still_fails_closed_until_backend_wired() {
        let sig = vec![0.1f32, 0.2, 0.3];
        let dummy_weights = [0u8; 8];
        assert_eq!(
            dnsmos(&sig, Some(&dummy_weights)),
            Err(AudioError::BackendUnavailable),
            "unwired backend must fail closed, not guess a MOS"
        );
    }

    #[test]
    fn empty_signal_is_malformed() {
        assert_eq!(dnsmos(&[], None), Err(AudioError::MalformedAudio));
    }
}
