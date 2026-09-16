//! Studio SI-10 scripted collect→run→revoke walkthrough (pure, no Dioxus).
//! Composes inspect/run/receipts/lifecycle helpers. No Host IDs.

use super::inspect::card_for;
use super::lifecycle::{can_run, suspend, LibraryLife};
use super::receipts::{revoke_keeps_rows, visible_rows, SEED_RECEIPTS};
use super::run::{cancel, run_step, RunPhase, PERMISSION_DENIED};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkStep {
    pub id: &'static str,
    pub detail: String,
}

/// Inspect → collect ≠ activate → run → permission deny → revoke keeps receipts → suspend.
pub fn collect_activate_run_revoke() -> Result<Vec<WalkStep>, &'static str> {
    let mut steps = Vec::new();
    let card = card_for("unit-convert").ok_or("held / not yet — inspect seed missing")?;
    if card.endorsed {
        return Err("held / not yet — catalogue inclusion is not endorsement");
    }
    steps.push(WalkStep {
        id: "inspect",
        detail: format!("inspect before fetch · {}", card.slug),
    });

    let mut life = LibraryLife {
        collected: true,
        ..LibraryLife::default()
    };
    if life.activated {
        return Err("held / not yet — collect must not activate");
    }
    steps.push(WalkStep {
        id: "collect",
        detail: "collected ≠ active".into(),
    });

    life.activated = true;
    can_run(&life)?;
    let mut phase = run_step(RunPhase::Idle, "assess", "1m", true);
    if phase != RunPhase::Running {
        return Err("held / not yet — run did not start");
    }
    phase = run_step(phase, "assess", "1m", true);
    if phase != RunPhase::Completed {
        return Err("held / not yet — run did not complete");
    }
    steps.push(WalkStep {
        id: "run",
        detail: "completed".into(),
    });

    let denied = run_step(RunPhase::Idle, "assess", "1m", false);
    if denied != RunPhase::Denied {
        return Err("held / not yet — permission denial must be usable");
    }
    steps.push(WalkStep {
        id: "permission-deny",
        detail: PERMISSION_DENIED.into(),
    });

    let mut revoked = false;
    let kept = revoke_keeps_rows(&mut revoked, SEED_RECEIPTS);
    if !revoked || visible_rows(revoked, SEED_RECEIPTS).len() != kept {
        return Err("held / not yet — old receipts must remain readable");
    }
    steps.push(WalkStep {
        id: "revoke-receipts",
        detail: format!("{kept} receipts remain"),
    });

    suspend(&mut life);
    if can_run(&life).is_ok() {
        return Err("held / not yet — suspended pack cannot start a new run");
    }
    steps.push(WalkStep {
        id: "suspend",
        detail: "held".into(),
    });

    let _ = cancel(RunPhase::Running);
    Ok(steps)
}

/// Host entry points are refused on the Studio run path.
pub fn host_entry_refused() -> bool {
    run_step(RunPhase::Idle, "Host.assess", "1m", true) == RunPhase::Held
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_collect_run_revoke() {
        let steps = collect_activate_run_revoke().expect("walkthrough");
        assert!(steps.iter().any(|s| s.id == "inspect"));
        assert!(steps.iter().any(|s| s.id == "permission-deny"));
        assert!(steps.iter().any(|s| s.id == "revoke-receipts"));
        for s in &steps {
            assert!(!s.id.contains("Host."));
            assert!(!s.detail.contains("Host."));
        }
    }

    #[test]
    fn host_entry_is_held() {
        assert!(host_entry_refused());
    }
}
