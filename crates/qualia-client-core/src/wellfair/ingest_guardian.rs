//! **Guardian notification on flagged ingest** — the guardianship hook of the hypermedia semantic library
//! (Timothy, 2026-07-07: *"if there's a guardianship relation, it might notify the guardian if the content
//! raises particular flags"*).
//!
//! When an asset is ingested, a processor may raise [`Flag`]s (a semantic descriptor bound to the asset — see
//! `qualia_core_db::hypermedia`). If the **principal is under a guardianship relation**, this layer turns the
//! flags at or above a chosen severity into [`GuardianNotification`]s **and records each in the tamper-evident
//! accountability ledger** — so a flagged ingest is both a notification to the guardian *and* an auditable,
//! un-erasable event (who was notified, about what, when). The "is the principal under guardianship + who is
//! the guardian" lookup is the guardianship / care-relationship layer's job; this takes the resolved guardian
//! and does the honest, recordable thing with the flags.

use qualia_core_db::hypermedia::{Flag, FlagSeverity};
use serde::{Deserialize, Serialize};

/// A notification to a guardian that a flagged asset was ingested for a principal under their guardianship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianNotification {
    pub guardian_did: String,
    pub principal_did: String,
    pub asset_uri: String,
    pub flag_kind: String,
    /// 0 Info · 1 Notice · 2 Concern · 3 Urgent.
    pub severity_level: u64,
    pub detail: String,
    pub time_unix: u64,
}

/// Produce guardian notifications for the flags that meet or exceed `min_severity`. Pure — the caller (host)
/// records each to the ledger and delivers it. A principal *not* under guardianship yields none (the caller
/// passes no guardian).
pub fn guardian_notifications(
    flags: &[Flag],
    asset_uri: &str,
    guardian_did: &str,
    principal_did: &str,
    min_severity: FlagSeverity,
    now_unix: u64,
) -> Vec<GuardianNotification> {
    flags
        .iter()
        .filter(|f| f.severity.level() >= min_severity.level())
        .map(|f| GuardianNotification {
            guardian_did: guardian_did.to_string(),
            principal_did: principal_did.to_string(),
            asset_uri: asset_uri.to_string(),
            flag_kind: f.kind.clone(),
            severity_level: f.severity.level(),
            detail: f.detail.clone(),
            time_unix: now_unix,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_flags_at_or_above_the_threshold_notify_the_guardian() {
        let flags = vec![
            Flag {
                kind: "sensitive-medical".into(),
                severity: FlagSeverity::Concern,
                detail: "x-ray".into(),
            },
            Flag {
                kind: "minor-note".into(),
                severity: FlagSeverity::Info,
                detail: String::new(),
            },
        ];
        let ns = guardian_notifications(
            &flags,
            "urn:doc:scan",
            "did:wf:guardian",
            "did:wf:child",
            FlagSeverity::Notice,
            1_000,
        );
        // The Concern flag (level 2 ≥ Notice level 1) notifies; the Info flag (0) does not.
        assert_eq!(ns.len(), 1);
        assert_eq!(ns[0].flag_kind, "sensitive-medical");
        assert_eq!(ns[0].guardian_did, "did:wf:guardian");
        assert_eq!(ns[0].principal_did, "did:wf:child");
        assert_eq!(ns[0].severity_level, 2);
    }

    #[test]
    fn urgent_only_threshold_filters_out_concern() {
        let flags = vec![Flag {
            kind: "x".into(),
            severity: FlagSeverity::Concern,
            detail: String::new(),
        }];
        assert!(guardian_notifications(&flags, "u", "g", "p", FlagSeverity::Urgent, 0).is_empty());
    }
}
