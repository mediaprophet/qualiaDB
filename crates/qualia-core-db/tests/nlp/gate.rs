//! Release-gate evaluation. Plumbing success is not a quality pass.

use super::manifest::{BenchmarkManifest, GateStatus};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGateOutcome {
    pub succeeded: bool,
    pub effective_status: GateStatus,
    pub reason: String,
}

/// Whether this manifest may succeed a **release** gate.
///
/// `UNEVALUATED` and `unavailable` never succeed. A missing `minimum_score`
/// is `UNEVALUATED` when a threshold is required (`release_required` or a
/// recorded `pass`). This function does not compute F1/accuracy.
pub fn evaluate_release_gate(manifest: &BenchmarkManifest) -> ReleaseGateOutcome {
    if manifest.gate_status == GateStatus::Unavailable {
        return ReleaseGateOutcome {
            succeeded: false,
            effective_status: GateStatus::Unavailable,
            reason: manifest
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "unavailable without reason".into()),
        };
    }

    let threshold_unset = manifest.minimum_score.is_none();
    let pass_without_threshold = manifest.gate_status == GateStatus::Pass && threshold_unset;
    let required_without_threshold = manifest.release_required && threshold_unset;

    if pass_without_threshold || required_without_threshold || threshold_unset {
        return ReleaseGateOutcome {
            succeeded: false,
            effective_status: GateStatus::Unevaluated,
            reason: "unset required threshold is UNEVALUATED and never passing".into(),
        };
    }

    match manifest.gate_status {
        GateStatus::Pass => {
            let min = manifest.minimum_score.expect("threshold checked above");
            match manifest.achieved_score {
                Some(achieved) if achieved >= min => ReleaseGateOutcome {
                    succeeded: true,
                    effective_status: GateStatus::Pass,
                    reason: "measured achieved_score >= frozen minimum_score".into(),
                },
                Some(_) => ReleaseGateOutcome {
                    succeeded: false,
                    effective_status: GateStatus::Fail,
                    reason: "gate_status is pass but achieved_score is below minimum_score".into(),
                },
                None => ReleaseGateOutcome {
                    succeeded: false,
                    effective_status: GateStatus::Unevaluated,
                    reason: "gate_status pass requires a measured achieved_score (NLP-006)".into(),
                },
            }
        }
        GateStatus::Fail => ReleaseGateOutcome {
            succeeded: false,
            effective_status: GateStatus::Fail,
            reason: "scored below frozen threshold".into(),
        },
        GateStatus::Blocked => ReleaseGateOutcome {
            succeeded: false,
            effective_status: GateStatus::Blocked,
            reason: "blocked".into(),
        },
        GateStatus::Unevaluated => ReleaseGateOutcome {
            succeeded: false,
            effective_status: GateStatus::Unevaluated,
            reason: "UNEVALUATED never passing".into(),
        },
        GateStatus::Unavailable => unreachable!("handled above"),
    }
}

pub fn release_gate_succeeds(manifest: &BenchmarkManifest) -> bool {
    evaluate_release_gate(manifest).succeeded
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::manifest::{load_manifest, SMOKE_MANIFEST_JSON, UNAVAILABLE_MANIFEST_JSON};

    #[test]
    fn smoke_stays_unevaluated_and_does_not_pass_release_gate() {
        let m = load_manifest(SMOKE_MANIFEST_JSON).unwrap();
        let out = evaluate_release_gate(&m);
        assert!(!out.succeeded);
        assert_eq!(out.effective_status, GateStatus::Unevaluated);
    }

    #[test]
    fn unavailable_external_stub_does_not_succeed_release_gate() {
        let m = load_manifest(UNAVAILABLE_MANIFEST_JSON).unwrap();
        let out = evaluate_release_gate(&m);
        assert_eq!(m.gate_status, GateStatus::Unavailable);
        assert!(!out.succeeded);
        assert_eq!(out.effective_status, GateStatus::Unavailable);
        assert!(out.reason.contains("licensed") || out.reason.contains("not present"));
        assert!(!release_gate_succeeds(&m));
    }

    #[test]
    fn recorded_pass_without_threshold_is_unevaluated() {
        let mut m = load_manifest(SMOKE_MANIFEST_JSON).unwrap();
        m.gate_status = GateStatus::Pass;
        m.release_required = true;
        m.minimum_score = None;
        let out = evaluate_release_gate(&m);
        assert!(!out.succeeded);
        assert_eq!(out.effective_status, GateStatus::Unevaluated);
    }

    #[test]
    fn pass_without_measured_score_is_unevaluated() {
        let mut m = load_manifest(SMOKE_MANIFEST_JSON).unwrap();
        m.gate_status = GateStatus::Pass;
        m.minimum_score = Some(0.99);
        m.achieved_score = None;
        let out = evaluate_release_gate(&m);
        assert!(!out.succeeded);
        assert_eq!(out.effective_status, GateStatus::Unevaluated);
    }

    #[test]
    fn pass_requires_achieved_at_least_minimum() {
        let mut m = load_manifest(SMOKE_MANIFEST_JSON).unwrap();
        m.gate_status = GateStatus::Pass;
        m.minimum_score = Some(0.99);
        m.achieved_score = Some(0.50);
        let out = evaluate_release_gate(&m);
        assert!(!out.succeeded);
        assert_eq!(out.effective_status, GateStatus::Fail);
        m.achieved_score = Some(0.99);
        let out = evaluate_release_gate(&m);
        assert!(out.succeeded);
        assert_eq!(out.effective_status, GateStatus::Pass);
    }
}
