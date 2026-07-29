//! Phase 4 integration — live share requests, owner decisions, sanctuary fail-closed.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ed25519_dalek::SigningKey;
    use wellfare_core::live_share::LiveSectionRequest;

    use wellfare_core::live_share::MSG_LIVE_SECTION_DECISION;

    use crate::wellfair::api::WebizenHostApi;
    use crate::wellfair::live_share::live_section_decision_from_record;
    use crate::wellfair::policy::PolicyDecisionService;
    use crate::wellfair::vault::VaultService;

    fn test_host(dir: &tempfile::TempDir) -> WebizenHostApi {
        let wal = dir.path().join("phase4.wal");
        let vault = VaultService::open(&wal, dir.path(), 0xBEEF).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[44u8; 32]);
        WebizenHostApi::new(
            vault,
            policy,
            signing_key,
            "did:wf:owner".into(),
            "did:wf:owner".into(),
            PathBuf::from(dir.path()),
        )
    }

    fn sample_request(id: &str, kinds: Vec<&str>) -> LiveSectionRequest {
        LiveSectionRequest::new(
            id,
            "companion-phone-1",
            "Desktop WellFair",
            "live section preview",
            kinds.into_iter().map(str::to_string).collect(),
            vec!["label".into()],
            300,
        )
    }

    #[test]
    fn enqueue_and_list_pending_live_shares() {
        let dir = tempfile::tempdir().unwrap();
        let host = test_host(&dir);
        let request = sample_request("req-phase4-1", vec!["conditions", "sleep"]);
        host.submit_live_share_request(&request).unwrap();
        let pending = host.list_pending_live_shares(8).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "req-phase4-1");
        assert_eq!(pending[0].requested_kinds, vec!["conditions", "sleep"]);
    }

    #[test]
    fn approve_emits_live_share_decision_journal_kind() {
        let dir = tempfile::tempdir().unwrap();
        let host = test_host(&dir);
        let request = sample_request("req-phase4-2", vec!["conditions"]);
        let enqueued = host.submit_live_share_request(&request).unwrap();
        assert_eq!(enqueued.kind, "live_share_request");

        let decision = host
            .decide_live_share_request("req-phase4-2", true, &["conditions".into()], None)
            .unwrap();
        assert_eq!(decision.kind, "live_share_decision");
        assert!(decision
            .summary
            .as_ref()
            .is_some_and(|s| s.contains("\"approved\":true")));

        assert!(host.list_pending_live_shares(8).unwrap().is_empty());
    }

    #[test]
    fn sanctuary_protected_kind_enqueueable_decision_must_be_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let host = test_host(&dir);
        let request = sample_request(
            "req-phase4-3",
            vec!["therapy_note", "wellbeing_observation"],
        );
        let enqueued = host.submit_live_share_request(&request).unwrap();
        assert_eq!(enqueued.kind, "live_share_request");
        assert!(enqueued
            .summary
            .as_ref()
            .is_some_and(|s| s.contains("therapy_note")));
        assert_eq!(host.list_pending_live_shares(4).unwrap().len(), 1);

        // Minimum projection without classified kind is allowed while sanctuary locked.
        let partial = host
            .decide_live_share_request(
                "req-phase4-3",
                true,
                &["wellbeing_observation".into()],
                None,
            )
            .unwrap();
        assert_eq!(partial.kind, "live_share_decision");
        let partial_summary: serde_json::Value =
            serde_json::from_str(partial.summary.as_ref().unwrap()).unwrap();
        let projection = partial_summary["projection_kinds"]
            .as_array()
            .expect("projection_kinds array");
        assert_eq!(projection.len(), 1);
        assert_eq!(projection[0], "wellbeing_observation");

        let dir2 = tempfile::tempdir().unwrap();
        let host2 = test_host(&dir2);
        let request2 = sample_request("req-phase4-4", vec!["therapy_note"]);
        host2.submit_live_share_request(&request2).unwrap();

        // Explicit classified projection fails closed without sanctuary unlock.
        let blocked =
            host2.decide_live_share_request("req-phase4-4", true, &["therapy_note".into()], None);
        assert!(blocked.is_err());
        assert!(blocked
            .unwrap_err()
            .contains("sanctuary protected kind 'therapy_note'"));

        host2
            .setup_sanctuary("real-pin-phase4", "decoy-pin-phase4")
            .unwrap();
        host2.lock_sanctuary().unwrap();
        host2.unlock_sanctuary("real-pin-phase4").unwrap();

        let explicit = host2
            .decide_live_share_request("req-phase4-4", true, &["therapy_note".into()], None)
            .unwrap();
        assert_eq!(explicit.kind, "live_share_decision");
        let explicit_summary: serde_json::Value =
            serde_json::from_str(explicit.summary.as_ref().unwrap()).unwrap();
        let explicit_projection = explicit_summary["projection_kinds"]
            .as_array()
            .expect("projection_kinds array");
        assert!(explicit_projection.iter().any(|k| k == "therapy_note"));
    }

    #[test]
    fn decision_wire_round_trips_for_companion_push() {
        let dir = tempfile::tempdir().unwrap();
        let host = test_host(&dir);
        let request = sample_request("req-wire-1", vec!["conditions"]);
        host.submit_live_share_request(&request).unwrap();
        host.decide_live_share_request("req-wire-1", true, &["conditions".into()], None)
            .unwrap();
        let record = host.get_live_share_record("req-wire-1").unwrap().unwrap();
        let wire = live_section_decision_from_record(&record);
        assert_eq!(wire.msg_type, MSG_LIVE_SECTION_DECISION);
        assert!(wire.approved);
        assert_eq!(wire.projection_kinds, vec!["conditions"]);
    }
}
