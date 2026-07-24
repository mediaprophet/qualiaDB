//! THE fail-closed gate (ADR 007) — every learned head routes weight access through this
//! (AU-LEARNED).
//!
//! The invariant this module enforces: **with no weights, a learned head returns
//! `AudioError::BackendUnavailable` — it never fabricates an output.** The gate is a plain,
//! auditable function so the fail-closed decision is impossible to miss or bypass: a head holds a
//! [`WeightState`], and the only sanctioned way to reach the weights is [`require_weights`], which
//! is `Err(BackendUnavailable)` whenever the state is `Absent`.

use super::weight_file::WeightBlob;
use crate::types::AudioError;

/// Whether a learned head has usable weights. Default construction is `Absent` (fail closed).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum WeightState {
    /// No weights loaded — the head must abstain.
    #[default]
    Absent,
    /// Weights present — the reference forward pass may run.
    Loaded(WeightBlob),
}

impl WeightState {
    #[inline]
    pub fn is_loaded(&self) -> bool {
        matches!(self, WeightState::Loaded(_))
    }
}

/// The fail-closed gate. Returns the loaded weights, or `BackendUnavailable` when absent.
///
/// This is the single choke point ADR 007 requires: a head that cannot obtain weights here must
/// propagate the error and produce nothing.
#[inline]
pub fn require_weights(state: &WeightState) -> Result<&WeightBlob, AudioError> {
    match state {
        WeightState::Loaded(blob) => Ok(blob),
        WeightState::Absent => Err(AudioError::BackendUnavailable),
    }
}

#[cfg(test)]
mod tests {
    use super::super::weight_file::make_blob;
    use super::*;

    #[test]
    fn absent_fails_closed() {
        let state = WeightState::Absent;
        assert_eq!(require_weights(&state), Err(AudioError::BackendUnavailable));
        assert!(!state.is_loaded());
    }

    #[test]
    fn default_is_absent() {
        assert_eq!(WeightState::default(), WeightState::Absent);
    }

    #[test]
    fn loaded_is_ok() {
        let blob = make_blob(vec![1], vec![0.5]);
        let state = WeightState::Loaded(blob.clone());
        assert_eq!(require_weights(&state), Ok(&blob));
        assert!(state.is_loaded());
    }
}
