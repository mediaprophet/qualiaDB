//! **The person's own physiological-state declaration** — where they are on the reproductive continuum.
//!
//! The reproductive-continuum layer (P1, `wellfare-core::anatomy::physiology`) models the continuum as
//! whole-body physiological states. The `StateModulator` re-parameterises all body systems by the current
//! state — but the person must be able to **declare** where they are on the continuum, so the score-card
//! reads them at their current life stage, not a neutral baseline.
//!
//! This is the person's own, self-declared state — **forum-internum / Sanctuary-class** selfhood content
//! (their inward knowledge of their own body), stored under the same prefs-style prefix as the weight model.
//! Absence of a stored state means "the person has not declared one" — the caller falls back to
//! [`PhysiologicalState::Baseline`], never an assumption about their body.

use std::path::Path;

use wellfare_core::anatomy::PhysiologicalState;

/// Where the person's declared physiological state lives (prefs-style, under the `wellfair/` prefix).
pub const PHYSIOLOGY_STATE_FILE: &str = "wellfair/physiology_state.json";

/// Load the person's **declared** physiological state, or `None` if they have not declared one (→ the
/// caller uses [`PhysiologicalState::Baseline`]).
pub fn load(storage_root: impl AsRef<Path>) -> Option<PhysiologicalState> {
    let path = storage_root.as_ref().join(PHYSIOLOGY_STATE_FILE);
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Persist the person's declared physiological state — their own statement of where they are on the
/// reproductive continuum. Forum-internum / Sanctuary-class.
pub fn save(storage_root: impl AsRef<Path>, state: &PhysiologicalState) -> Result<(), String> {
    let path = storage_root.as_ref().join(PHYSIOLOGY_STATE_FILE);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

/// Clear the person's declared state — revert to the implicit [`PhysiologicalState::Baseline`]. Idempotent
/// (no-op if none exists).
pub fn clear(storage_root: impl AsRef<Path>) -> Result<(), String> {
    let path = storage_root.as_ref().join(PHYSIOLOGY_STATE_FILE);
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wellfare_core::anatomy::{CyclePhase, PhysiologicalState, ReproductiveState, Trimester};

    #[test]
    fn declared_state_persists_and_clears_back_to_none() {
        let dir = tempfile::tempdir().unwrap();
        // Nothing declared yet → None (caller uses Baseline).
        assert!(load(dir.path()).is_none());

        // The person declares they're in the third trimester.
        let state = PhysiologicalState::Reproductive(ReproductiveState::Pregnant(Trimester::Third));
        save(dir.path(), &state).unwrap();
        assert_eq!(load(dir.path()), Some(state), "the person's state is theirs, persisted");

        // They can clear it back to the implicit baseline.
        clear(dir.path()).unwrap();
        assert!(load(dir.path()).is_none(), "cleared returns to the implicit baseline");
    }

    #[test]
    fn every_continuum_state_round_trips_through_serde() {
        let states = vec![
            PhysiologicalState::Baseline,
            PhysiologicalState::Reproductive(ReproductiveState::PreMenarche),
            PhysiologicalState::Reproductive(ReproductiveState::Cycling(CyclePhase::Menstrual)),
            PhysiologicalState::Reproductive(ReproductiveState::Cycling(CyclePhase::Follicular)),
            PhysiologicalState::Reproductive(ReproductiveState::Cycling(CyclePhase::Ovulatory)),
            PhysiologicalState::Reproductive(ReproductiveState::Cycling(CyclePhase::Luteal)),
            PhysiologicalState::Reproductive(ReproductiveState::Pregnant(Trimester::First)),
            PhysiologicalState::Reproductive(ReproductiveState::Pregnant(Trimester::Second)),
            PhysiologicalState::Reproductive(ReproductiveState::Pregnant(Trimester::Third)),
            PhysiologicalState::Reproductive(ReproductiveState::Postpartum),
            PhysiologicalState::Reproductive(ReproductiveState::Lactating),
            PhysiologicalState::Reproductive(ReproductiveState::Perimenopause),
            PhysiologicalState::Reproductive(ReproductiveState::PostMenopause),
        ];
        let dir = tempfile::tempdir().unwrap();
        for state in &states {
            save(dir.path(), state).unwrap();
            let loaded = load(dir.path()).unwrap();
            assert_eq!(&loaded, state, "round-trip failed for {:?}", state);
        }
    }
}
