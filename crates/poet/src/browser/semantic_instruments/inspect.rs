//! Inspector lifecycle (SI-09/SI-10 contract, Poet host).
//!
//! Six lifecycle chips must not collapse. Collect stores bytes without
//! activating. Activate requires a closed lock. Receipt history remains
//! after revoke. Dispatch is by entry point, never a Host ID.

/// Lifecycle chips. Chrome must not collapse these into one badge.
pub const LIFECYCLE_CHIPS: [&str; 6] = [
    "authoring",
    "publication",
    "resolution",
    "installation",
    "trust",
    "activation",
];

/// Keyboard-named inspector/library actions (a11y).
pub const ACTIONS: &[(&str, &str)] = &[
    ("inspect", "Inspect instrument"),
    ("collect", "Collect without activating"),
    ("activate", "Activate closed lock"),
    ("run", "Run entry point"),
    ("cancel", "Cancel run"),
    ("view-receipt", "View receipt history"),
];

pub const HELD_COLLECT_FIRST: &str = "held / not yet — collect before activate";
pub const HELD_LOCK_OPEN: &str = "held / not yet — unresolved package is held";
pub const HELD_REVOKED: &str = "held / not yet — revoked version cannot start a new run";
pub const COLLECTED_NOT_ACTIVE: &str = "collected ≠ active";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InspectState {
    pub collected: bool,
    pub activated: bool,
    pub lock_closed: bool,
    pub revoked: bool,
    pub receipts_remain: bool,
}

/// Store bytes without activating. Does not require a closed lock.
pub fn collect_without_activate(state: &mut InspectState) {
    state.collected = true;
    state.activated = false;
}

/// Activate / new run requires collected + closed lock + not revoked.
pub fn activate_requires_closed(state: &InspectState) -> Result<(), &'static str> {
    if !state.collected {
        return Err(HELD_COLLECT_FIRST);
    }
    if state.revoked {
        return Err(HELD_REVOKED);
    }
    if !state.lock_closed {
        return Err(HELD_LOCK_OPEN);
    }
    Ok(())
}

/// Revoke stops new runs; receipt history stays readable.
pub fn revoke_keeps_receipts(state: &mut InspectState) {
    state.revoked = true;
    state.activated = false;
    state.receipts_remain = true;
}

pub fn record_run(state: &mut InspectState) -> Result<(), &'static str> {
    activate_requires_closed(state)?;
    state.activated = true;
    state.receipts_remain = true;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_chips_must_not_collapse() {
        assert_eq!(LIFECYCLE_CHIPS.len(), 6);
        for (i, chip) in LIFECYCLE_CHIPS.iter().enumerate() {
            assert!(!chip.is_empty());
            for (j, other) in LIFECYCLE_CHIPS.iter().enumerate() {
                if i != j {
                    assert_ne!(chip, other);
                }
            }
        }
        assert_eq!(LIFECYCLE_CHIPS[0], "authoring");
        assert_eq!(LIFECYCLE_CHIPS[1], "publication");
        assert_eq!(LIFECYCLE_CHIPS[2], "resolution");
        assert_eq!(LIFECYCLE_CHIPS[3], "installation");
        assert_eq!(LIFECYCLE_CHIPS[4], "trust");
        assert_eq!(LIFECYCLE_CHIPS[5], "activation");
    }

    #[test]
    fn collect_does_not_activate() {
        let mut state = InspectState::default();
        collect_without_activate(&mut state);
        assert!(state.collected);
        assert!(!state.activated);
        assert_eq!(activate_requires_closed(&state), Err(HELD_LOCK_OPEN));
    }

    #[test]
    fn activate_requires_closed_lock() {
        let mut state = InspectState::default();
        assert_eq!(activate_requires_closed(&state), Err(HELD_COLLECT_FIRST));
        collect_without_activate(&mut state);
        state.lock_closed = true;
        assert!(activate_requires_closed(&state).is_ok());
        assert!(record_run(&mut state).is_ok());
        assert!(state.activated);
        assert!(state.receipts_remain);
    }

    #[test]
    fn receipt_history_remains_after_revoke() {
        let mut state = InspectState {
            collected: true,
            activated: true,
            lock_closed: true,
            revoked: false,
            receipts_remain: true,
        };
        revoke_keeps_receipts(&mut state);
        assert!(state.revoked);
        assert!(!state.activated);
        assert!(state.receipts_remain);
        assert_eq!(activate_requires_closed(&state), Err(HELD_REVOKED));
    }

    #[test]
    fn every_inspector_action_has_accessible_name() {
        let ids: Vec<&str> = ACTIONS.iter().map(|(id, _)| *id).collect();
        for required in ["inspect", "collect", "activate", "run", "cancel", "view-receipt"] {
            assert!(ids.contains(&required), "missing action {required}");
        }
        for (id, name) in ACTIONS {
            assert!(!id.contains("Host."));
            assert!(!name.is_empty());
        }
    }
}
