//! NISQA — non-intrusive speech-quality learned MOS predictor. **NeedsWeights.**
//!
//! NISQA (Mittag et al., 2021) is a deep model (CNN + self-attention) trained to
//! predict speech MOS and its sub-dimensions (noisiness, coloration,
//! discontinuity, loudness) from a single degraded clip, no clean reference. As
//! with DNSMOS, the score IS the trained network; there is no principled hand
//! coded formula that yields it.
//!
//! This function therefore **fails closed**: without weights it returns
//! [`AudioError::BackendUnavailable`] and never fabricates a MOS. Fabricating a
//! learned MOS is forbidden by project rule.

use crate::types::AudioError;

/// Predict a NISQA speech-quality MOS for `signal`.
///
/// - `signal`: mono degraded speech (no reference needed).
/// - `weights`: the trained NISQA model weights. `None` (or, currently, any value)
///   means the learned backend is unavailable.
///
/// # Fails closed
///
/// Returns [`AudioError::BackendUnavailable`] when `weights` is `None`, and also
/// while the inference backend is unwired for `Some(_)`. It never returns a
/// synthesised MOS.
pub fn nisqa(signal: &[f32], weights: Option<&[u8]>) -> Result<f32, AudioError> {
    if signal.is_empty() {
        return Err(AudioError::MalformedAudio);
    }

    match weights {
        // No model → no MOS. Fail closed; NEVER fabricate a number.
        None => Err(AudioError::BackendUnavailable),
        // Weights present but learned inference is not yet wired: still fail closed.
        Some(_bytes) => Err(AudioError::BackendUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_weights_fails_closed_never_returns_mos() {
        let sig = vec![0.05f32, -0.1, 0.2, -0.15];
        let out = nisqa(&sig, None);
        assert_eq!(
            out,
            Err(AudioError::BackendUnavailable),
            "NISQA with no weights must fail closed, not return a MOS"
        );
        assert!(out.is_err());
    }

    #[test]
    fn weights_present_still_fails_closed_until_backend_wired() {
        let sig = vec![0.1f32, 0.2, 0.3];
        let dummy = [0u8; 16];
        assert_eq!(
            nisqa(&sig, Some(&dummy)),
            Err(AudioError::BackendUnavailable),
            "unwired backend must fail closed, not guess a MOS"
        );
    }

    #[test]
    fn empty_signal_is_malformed() {
        assert_eq!(nisqa(&[], None), Err(AudioError::MalformedAudio));
    }
}
