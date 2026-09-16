//! Scripted SI-09 author-to-publish and SI-10 inspect lifecycle walkthroughs.
//!
//! Pure state machines — no Document, no Host IDs, no clinical kernel.

use super::inspect::{
    activate_requires_closed, collect_without_activate, record_run, revoke_keeps_receipts,
    InspectState, ACTIONS as INSPECT_ACTIONS, HELD_REVOKED,
};
use super::manufacture::{can_publish, next_action, Draft, ACTIONS as MFG_ACTIONS, NEXT_PUBLISH};
use super::manufacture_keys::keyboard_only_author_to_publish;
use super::round_trip::{loss_reasons, source_visual_round_trip_ok, sync_round_trip_flag};
use super::validate::{evidence_for, validation_held};

/// One recorded step in a scripted walkthrough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub id: &'static str,
    pub status: &'static str,
}

/// Author → validate evidence → publish. Invalid mid-path stays held.
pub fn author_to_publish() -> Result<Vec<Step>, &'static str> {
    let mut draft = Draft::default();
    let mut steps = Vec::new();

    steps.push(Step {
        id: "start",
        status: next_action(&draft),
    });
    assert_held_evidence(&draft);

    draft.source_ok = true;
    sync_round_trip_flag(&mut draft);
    steps.push(Step {
        id: "add-definition-n3",
        status: next_action(&draft),
    });
    // Source without graph must list concrete loss before save.
    if loss_reasons(&draft).is_empty() {
        return Err("held / not yet — expected source-without-graph loss");
    }

    draft.graph_ok = true;
    sync_round_trip_flag(&mut draft);
    steps.push(Step {
        id: "compile-graph",
        status: next_action(&draft),
    });

    draft.executable_ok = true;
    steps.push(Step {
        id: "build-executable",
        status: next_action(&draft),
    });

    draft.labelled = true;
    steps.push(Step {
        id: "label-category",
        status: next_action(&draft),
    });

    if !source_visual_round_trip_ok(&draft) {
        return Err("held / not yet — source/visual round-trip incomplete");
    }
    if validation_held(&draft).is_some() {
        return Err("held / not yet — validation still held after ready draft");
    }
    for row in evidence_for(&draft) {
        if !row.ok {
            return Err("held / not yet — validation evidence not all ok");
        }
    }
    can_publish(&draft)?;
    steps.push(Step {
        id: "publish",
        status: NEXT_PUBLISH,
    });
    Ok(steps)
}

fn assert_held_evidence(draft: &Draft) {
    let _ = evidence_for(draft);
    let _ = validation_held(draft);
}

/// Collect → activate → run → revoke → receipts remain; revoked blocks new run.
pub fn collect_to_receipt_walkthrough() -> Result<Vec<Step>, &'static str> {
    let mut state = InspectState::default();
    let mut steps = Vec::new();

    collect_without_activate(&mut state);
    steps.push(Step {
        id: "collect",
        status: "collected ≠ active",
    });
    if state.activated {
        return Err("held / not yet — collect must not activate");
    }

    state.lock_closed = true;
    activate_requires_closed(&state)?;
    steps.push(Step {
        id: "activate",
        status: "closed lock",
    });

    record_run(&mut state)?;
    steps.push(Step {
        id: "run",
        status: "receipt recorded",
    });
    if !state.receipts_remain {
        return Err("held / not yet — receipt missing after run");
    }

    revoke_keeps_receipts(&mut state);
    steps.push(Step {
        id: "revoke",
        status: HELD_REVOKED,
    });
    if !state.receipts_remain {
        return Err("held / not yet — old receipts must remain readable");
    }
    match activate_requires_closed(&state) {
        Err(HELD_REVOKED) => Ok(steps),
        Ok(()) => Err("held / not yet — revoked version must not start a new run"),
        Err(other) => Err(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::a11y::action_name_ok;

    #[test]
    fn scripted_author_to_publish_completes() {
        let steps = author_to_publish().expect("walkthrough");
        assert!(steps.len() >= 5);
        assert_eq!(steps.last().unwrap().id, "publish");
        assert_eq!(steps.last().unwrap().status, NEXT_PUBLISH);
        for step in &steps {
            assert!(!step.id.contains("Host."));
            assert!(!step.status.contains("Host."));
        }
    }

    #[test]
    fn invalid_mid_path_cannot_publish() {
        let mut draft = Draft {
            source_ok: true,
            ..Draft::default()
        };
        sync_round_trip_flag(&mut draft);
        assert!(can_publish(&draft).is_err());
        assert!(!loss_reasons(&draft).is_empty());
    }

    #[test]
    fn keyboard_only_path_matches_scripted_publish() {
        let mut draft = Draft::default();
        keyboard_only_author_to_publish(&mut draft).expect("keyboard");
        assert!(can_publish(&draft).is_ok());
    }

    #[test]
    fn collect_run_revoke_keeps_receipts() {
        let steps = collect_to_receipt_walkthrough().expect("si-10");
        assert!(steps.iter().any(|s| s.id == "revoke"));
        assert_eq!(steps.last().unwrap().status, HELD_REVOKED);
    }

    #[test]
    fn every_primary_action_has_accessible_name() {
        for (id, name) in MFG_ACTIONS.iter().chain(INSPECT_ACTIONS.iter()) {
            assert!(!id.is_empty());
            assert!(action_name_ok(name), "{id}");
        }
    }
}
