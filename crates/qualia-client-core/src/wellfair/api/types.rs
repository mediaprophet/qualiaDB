//! Standalone types + free functions

use qualia_cooperative_core::agency_delegation::ConsentState;

pub struct SyncPullReport {
    /// Operations received from the transport.
    pub pulled: usize,
    /// Newly admitted as valid.
    pub validated: usize,
    /// Already-seen operation ids (idempotent replays).
    pub duplicate: usize,
    /// Failed fail-closed validation (bad signature/hash/version/oversize/Classified lane).
    pub rejected: usize,
}

/// A node health/status snapshot for support + the Sync & Backup panel.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticsReport {
    pub crate_version: String,
    pub sanctuary_configured: bool,
    pub sanctuary_keychain_wrapped: bool,
    pub journal_records: usize,
    pub outbox_queued: usize,
    pub inbox_validated: usize,
    pub data_files: usize,
    pub data_bytes: u64,
}

/// A domain of agency, flattened for a delegation-creation picker (host → Tauri → Studio).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgencyDomainInfo {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub description: String,
    pub consequential: bool,
    pub selfhood: bool,
}

/// Parse the wire form of a consent state (`"granted" | "withdrawn" | "pending" | "not_required"`)
/// into [`ConsentState`]. Used by the Tauri command layer.
pub fn agency_consent_from_str(s: &str) -> Result<ConsentState, String> {
    match s {
        "granted" => Ok(ConsentState::Granted),
        "withdrawn" => Ok(ConsentState::Withdrawn),
        "pending" => Ok(ConsentState::Pending),
        "not_required" => Ok(ConsentState::NotRequired),
        other => Err(format!("unknown consent state: {other}")),
    }
}

/// Parse the wire form of a decoy-audit retention mode (`"auto_archive"` | `"manual_triage"`) into
/// the [`RetentionMode`](qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode) enum. Used by
/// the Tauri command layer so the desktop crate needs no direct `qualia-core-db` dependency.
#[cfg(not(target_arch = "wasm32"))]
pub fn sanctuary_retention_mode_from_str(
    mode: &str,
) -> Result<qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode, String> {
    use qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode;
    match mode {
        "auto_archive" => Ok(RetentionMode::AutoArchive),
        "manual_triage" => Ok(RetentionMode::ManualTriage),
        other => Err(format!("unknown retention mode: {other}")),
    }
}

#[cfg(test)]
mod api_tests {
    use super::super::*;
    use super::*;
    use crate::wellfair::host_state::SubmitOutcome;
    use crate::wellfair::policy::PolicyDecisionService;
    use crate::wellfair::vault::VaultService;
    use ed25519_dalek::SigningKey;
    use tempfile::tempdir;
    use wellfare_core::conditions::{AllergyReport, ConditionReport};
    use wellfare_core::personal_records::{DisputedDiagnosisReport, HousingSafetyReport};

    fn test_host(dir: &std::path::Path) -> WebizenHostApi {
        let wal = dir.join("test.wal");
        let vault = VaultService::open(&wal, dir, 0xBEEF).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        WebizenHostApi::new(
            vault,
            policy,
            signing_key,
            "did:wf:owner".into(),
            "did:wf:owner".into(),
            dir.to_path_buf(),
        )
    }

    #[test]
    fn add_condition_writes_restricted_journal_entry() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let report = ConditionReport::new("Hypertension");
        let entry = host.add_condition(&report).unwrap();
        assert_eq!(entry.kind, "condition");
        assert_eq!(entry.source, SOURCE_PERSONAL);
        assert!(entry.id.contains(":condition:"));
        let listed = host.list_health_records(8).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, "condition");
    }

    #[test]
    fn agency_delegation_create_list_evaluate_supersede() {
        use qualia_cooperative_core::agency_delegation::AgencyDelegation;
        use qualia_cooperative_core::agency_domain::ids as dom;

        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());

        // 17 seeded domains available to the picker.
        assert_eq!(host.list_agency_domains().len(), 17);

        // Create a consented MEDICAL delegation to a carer.
        let mut d = AgencyDelegation::new("did:wf:alice", dom::MEDICAL, "urn:un:hr:udhr", 100);
        d.agent_dids = vec!["did:wf:carer".into()];
        d.consent = ConsentState::Granted;
        let entry = host.add_agency_delegation(&d).unwrap();
        assert_eq!(entry.kind, "agency_delegation");
        assert!(entry.id.contains(":agency-delegation:"));

        // It lists back losslessly (agent + domain preserved).
        let listed = host.list_agency_delegations(16).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].domain, dom::MEDICAL);
        assert_eq!(listed[0].agent_dids, vec!["did:wf:carer".to_string()]);

        // A read is permitted; a consequential (medical) *decide* is denied without provenance.
        assert!(host
            .evaluate_agency_access(&d.id, "read", "diagnosis")
            .unwrap()
            .is_permit());
        assert!(!host
            .evaluate_agency_access(&d.id, "decide", "diagnosis")
            .unwrap()
            .is_permit());

        // Withdraw consent → supersedes; the projection shows one delegation, now non-permitting.
        host.set_agency_delegation_consent(&d.id, ConsentState::Withdrawn)
            .unwrap();
        let listed = host.list_agency_delegations(16).unwrap();
        assert_eq!(
            listed.len(),
            1,
            "supersede must not create a second logical delegation"
        );
        assert_eq!(listed[0].consent, ConsentState::Withdrawn);
        assert!(!host
            .evaluate_agency_access(&d.id, "read", "diagnosis")
            .unwrap()
            .is_permit());

        // Revoke → still one logical delegation, now revoked.
        host.revoke_agency_delegation(&d.id).unwrap();
        let listed = host.list_agency_delegations(16).unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].revoked);
    }

    #[test]
    fn wellbeing_assessment_record_score_and_list() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());

        // Ships PHQ-9 and GAD-7.
        let insts = host.list_assessment_instruments();
        assert_eq!(insts.len(), 2);
        assert!(insts.iter().any(|i| i.id == "phq9"));

        // Record a PHQ-9 that trips the self-harm flag (item 9 endorsed).
        let mut resp = vec![0u8; 9];
        resp[8] = 2;
        let result = host.record_assessment("phq9", resp).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.band_label, "Minimal");
        assert_eq!(result.flags.len(), 1, "self-harm flag must surface");

        // Persisted + reconstructed via the journal.
        let listed = host.list_assessments(16).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].instrument_id, "phq9");
        assert_eq!(listed[0].flags.len(), 1);

        // Fail-closed: unknown instrument and bad response count are rejected (no record written).
        assert!(host.record_assessment("bdi2", vec![0; 9]).is_err());
        assert!(host.record_assessment("gad7", vec![0; 3]).is_err());
        assert_eq!(host.list_assessments(16).unwrap().len(), 1);
    }

    #[test]
    fn sync_push_pull_round_trip_over_in_memory_relay() {
        use crate::wellfair::sync_outbox::SyncOutbox;
        use crate::wellfair::sync_transport::InMemoryRelay;

        let relay = InMemoryRelay::new();
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        let mut host_a = test_host(dir_a.path());
        let host_b = test_host(dir_b.path());

        // Host A commits a (Restricted) record — the vault auto-enqueues it to A's outbox.
        host_a
            .add_condition(&ConditionReport::new("Asthma"))
            .unwrap();
        let queued_before = SyncOutbox::open(dir_a.path())
            .unwrap()
            .count_queued()
            .unwrap();
        assert!(queued_before >= 1);

        // Push drains the outbox onto the relay and marks entries Sent.
        let pushed = host_a.sync_push_via(&relay, 32).unwrap();
        assert_eq!(pushed, queued_before);
        assert_eq!(
            SyncOutbox::open(dir_a.path())
                .unwrap()
                .count_queued()
                .unwrap(),
            0
        );
        assert_eq!(relay.len(), pushed);

        // Host B pulls + admits — the op lands in B's validated set.
        let report = host_b.sync_pull_via(&relay, 0).unwrap();
        assert_eq!(report.pulled, pushed);
        assert_eq!(report.validated, pushed);
        assert_eq!(report.rejected, 0);
        let validated = host_b.validated_sync_operations().unwrap();
        assert!(validated.iter().any(|o| o.kind == "condition"));

        // Re-pull is idempotent: duplicates, never double-admitted.
        let again = host_b.sync_pull_via(&relay, 0).unwrap();
        assert_eq!(again.validated, 0);
        assert_eq!(again.duplicate, pushed);
        assert_eq!(
            host_b.validated_sync_operations().unwrap().len(),
            validated.len()
        );
    }

    #[test]
    fn backup_export_restore_reconstructs_a_working_node() {
        let src = tempdir().unwrap();
        let mut host = test_host(src.path());
        host.add_condition(&ConditionReport::new("Eczema")).unwrap();
        host.add_allergy(&AllergyReport::new("Penicillin")).unwrap();
        let before = host.list_health_records(16).unwrap();
        assert!(before.len() >= 2);

        // Export, then restore into a brand-new, empty node.
        let archive = host.export_backup_bytes().unwrap();
        let dst = tempdir().unwrap();
        let restored_host = test_host(dst.path());
        assert!(restored_host.list_health_records(16).unwrap().is_empty());

        let report = restored_host.import_backup_bytes(&archive).unwrap();
        assert!(report.files >= 1);

        // A fresh host over the restored storage reads the same records back.
        let reopened = test_host(dst.path());
        let after = reopened.list_health_records(16).unwrap();
        assert_eq!(after.len(), before.len());
        assert!(after.iter().any(|e| e.kind == "condition"));
        assert!(after.iter().any(|e| e.kind == "allergy"));
    }

    #[test]
    fn diagnostics_report_reflects_node_state() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_condition(&ConditionReport::new("Migraine"))
            .unwrap();

        let report = host.diagnostics_report().unwrap();
        assert!(!report.crate_version.is_empty());
        assert!(report.journal_records >= 1);
        assert!(
            report.outbox_queued >= 1,
            "a committed record auto-enqueues to the outbox"
        );
        assert!(report.data_files >= 1);
        assert!(report.data_bytes > 0);
        // No Sanctuary vault set up in this test.
        assert!(!report.sanctuary_configured);
    }

    #[test]
    fn add_allergy_writes_allergy_journal_entry() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let report = AllergyReport::new("Shellfish");
        let entry = host.add_allergy(&report).unwrap();
        assert_eq!(entry.kind, "allergy");
        assert!(entry.id.contains(":allergy:"));
    }

    #[test]
    fn add_disputed_diagnosis_journal_kind() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let report = DisputedDiagnosisReport::new("Bipolar disorder");
        let entry = host.add_disputed_diagnosis(&report).unwrap();
        assert_eq!(entry.kind, "disputed_diagnosis");
        assert!(entry.id.contains(":disputed_diagnosis:"));
    }

    #[test]
    fn add_housing_safety_journal_kind() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let mut report = HousingSafetyReport::new();
        report.dwelling_type = wellfare_core::personal_records::DwellingType::MobileShelter;
        report.homelessness = true;
        let entry = host.add_housing_safety(&report).unwrap();
        assert_eq!(entry.kind, "housing_safety");
    }

    #[test]
    fn ledger_entries_commit_and_balance_by_currency() {
        use wellfare_core::finance::LedgerEntry;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_ledger_entry(&LedgerEntry::new("Wage", 250_000, "AUD", 1_700_000_000))
            .unwrap();
        host.add_ledger_entry(&LedgerEntry::new(
            "Groceries",
            -42_000,
            "AUD",
            1_700_000_100,
        ))
        .unwrap();
        host.add_ledger_entry(&LedgerEntry::new(
            "Grant (USD)",
            100_000,
            "usd",
            1_700_000_200,
        ))
        .unwrap();

        let rows = host.list_ledger_entries(16).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|r| r.kind == "ledger_entry"));

        let balance = host.ledger_balance(64).unwrap();
        assert_eq!(balance.total_entries, 3);
        let aud = balance
            .by_currency
            .iter()
            .find(|c| c.currency == "AUD")
            .unwrap();
        assert_eq!(aud.net_cents, 208_000);
        assert_eq!(aud.entry_count, 2);
        let usd = balance
            .by_currency
            .iter()
            .find(|c| c.currency == "USD")
            .unwrap();
        assert_eq!(usd.net_cents, 100_000);
    }

    #[test]
    fn contributions_commit_and_obligations_derive_through_journal() {
        use wellfare_core::projects::{Contribution, Project};
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let p = Project::new("Community Garden", "shared beds", vec![], 1_700_000_000);
        host.add_project(&p).unwrap();
        host.add_contribution(&Contribution::new(
            &p.id,
            "did:wf:owner",
            "dig",
            60,
            0,
            1.0,
            wellfare_core::projects::ContributionPrivacy::Public,
            1_700_000_050,
        ))
        .unwrap();
        host.add_contribution(&Contribution::new(
            &p.id,
            "did:wf:owner",
            "plant",
            30,
            0,
            1.0,
            wellfare_core::projects::ContributionPrivacy::Public,
            1_700_000_100,
        ))
        .unwrap();

        let obligations = host.project_obligations(64).unwrap();
        let owner = obligations
            .iter()
            .find(|o| o.project_id == p.id && o.contributor_did == "did:wf:owner")
            .unwrap();
        assert_eq!(owner.total_effort_minutes, 90);
        assert_eq!(owner.contribution_count, 2);
    }

    #[test]
    fn credential_commits_with_credential_kind() {
        use wellfare_core::credentials::CredentialRecord;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let cred = CredentialRecord::new(
            "did:wf:issuer",
            "did:wf:owner",
            "ProofOfAddress",
            1_700_000_000,
        )
        .with_claim("postcode", "3000");
        let entry = host.add_credential(&cred).unwrap();
        assert_eq!(entry.kind, "credential");
        assert_eq!(host.list_credentials(8).unwrap().len(), 1);
    }

    fn proxy_condition_envelope(id: &str) -> wellfare_core::record::RecordEnvelope {
        use wellfare_core::record::{
            EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass,
        };
        RecordEnvelope {
            id: id.to_string(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:supporter".into(),
            proxy_did: Some("did:wf:supporter".into()),
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Restricted,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: Some(1_700_000_000),
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        }
    }

    #[test]
    fn proxy_write_suspends_then_ratifies_and_commits() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());

        // A supporter (proxy) drafts a protected record on the principal's behalf.
        let env = proxy_condition_envelope("urn:wellfair:condition:proxy1");
        let outcome = host
            .submit_proxy_record(
                QAPP_CLINICAL,
                env,
                SOURCE_CLINICAL,
                Some("{\"label\":\"BP\"}".into()),
            )
            .unwrap();
        let proposal_id = match outcome {
            SubmitOutcome::Suspended {
                proposal_id,
                threshold,
            } => {
                assert_eq!(threshold, 2);
                proposal_id
            }
            other => panic!("expected suspension, got {other:?}"),
        };

        // The escrowed record is NOT yet committed.
        assert!(host
            .list_journal_by_kind("condition", 64)
            .unwrap()
            .is_empty());

        // One approval → still pending.
        let v1 = host
            .vote_guardianship_proposal(&proposal_id, "did:wf:guardianA", true, None)
            .unwrap();
        assert_eq!(v1.state, "pending");
        assert!(!v1.committed);
        assert!(host
            .list_journal_by_kind("condition", 64)
            .unwrap()
            .is_empty());

        // Second distinct approval → ratified; the escrowed record commits.
        let v2 = host
            .vote_guardianship_proposal(&proposal_id, "did:wf:guardianB", true, None)
            .unwrap();
        assert_eq!(v2.state, "ratified");
        assert!(v2.committed);
        let committed = host.list_journal_by_kind("condition", 64).unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(committed[0].id, "urn:wellfair:condition:proxy1");

        // A replayed final vote must not double-commit (idempotent).
        let v3 = host
            .vote_guardianship_proposal(&proposal_id, "did:wf:guardianB", true, None)
            .unwrap();
        assert!(v3.committed);
        assert_eq!(host.list_journal_by_kind("condition", 64).unwrap().len(), 1);

        // The tray shows it ratified + committed.
        let tray = host.list_guardianship_proposals(64).unwrap();
        let view = tray.iter().find(|p| p.proposal_id == proposal_id).unwrap();
        assert_eq!(view.state, "ratified");
        assert!(view.committed);
    }

    #[test]
    fn guardian_objection_denies_and_blocks_commit() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let env = proxy_condition_envelope("urn:wellfair:condition:proxy2");
        let outcome = host
            .submit_proxy_record(QAPP_CLINICAL, env, SOURCE_CLINICAL, None)
            .unwrap();
        let proposal_id = match outcome {
            SubmitOutcome::Suspended { proposal_id, .. } => proposal_id,
            other => panic!("expected suspension, got {other:?}"),
        };
        host.vote_guardianship_proposal(&proposal_id, "did:wf:guardianA", true, None)
            .unwrap();
        let denied = host
            .vote_guardianship_proposal(
                &proposal_id,
                "did:wf:guardianB",
                false,
                Some("not in her interest".into()),
            )
            .unwrap();
        assert_eq!(denied.state, "denied");
        assert!(!denied.committed);
        assert_eq!(denied.denied_by.as_deref(), Some("did:wf:guardianB"));
        assert!(host
            .list_journal_by_kind("condition", 64)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn non_proxy_write_is_unaffected_by_escrow() {
        // Regression guard: an ordinary (non-proxy) write still commits directly, no proposal.
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let entry = host.add_condition(&ConditionReport::new("Asthma")).unwrap();
        assert_eq!(entry.kind, "condition");
        assert!(host.list_guardianship_proposals(64).unwrap().is_empty());
    }

    #[test]
    fn credential_claims_persist_and_presentation_selects_fields() {
        use wellfare_core::credentials::CredentialRecord;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let cred = CredentialRecord::new(
            "did:wf:issuer",
            "did:wf:owner",
            "ProofOfAddress",
            1_700_000_000,
        )
        .with_claim("full_name", "Jane Roe")
        .with_claim("street", "1 Camper Lane")
        .with_claim("postcode", "3000");
        let entry = host.add_credential(&cred).unwrap();

        // Full claims survive via the content-addressed blob (not just claim_count).
        let loaded = host.get_credential(&entry.id).unwrap().unwrap();
        assert_eq!(loaded.claims.len(), 3);
        assert_eq!(loaded.issuer_did, "did:wf:issuer");

        // Presentation discloses only the selected claim keys; the rest never appear.
        let pres = host
            .present_credential(&entry.id, &["full_name".into(), "postcode".into()])
            .unwrap();
        assert_eq!(pres.disclosed_claims.len(), 2);
        assert!(pres.disclosed_claims.iter().any(|(k, _)| k == "full_name"));
        assert!(pres.disclosed_claims.iter().all(|(k, _)| k != "street"));
    }

    #[test]
    fn outbound_sync_operation_signed_and_inbox_dedupes_replay() {
        use wellfare_core::finance::LedgerEntry;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let entry = host
            .add_ledger_entry(&LedgerEntry::new("Wage", 100_000, "AUD", 1_700_000_000))
            .unwrap();
        let op = host
            .build_outbound_operation(&entry, 1)
            .expect("a Restricted entry yields an outbound operation");
        assert!(op.signature.as_deref().is_some_and(|s| !s.is_empty()));
        assert_eq!(op.kind, "ledger_entry");

        // A separate peer receives it: first admit validates, replays are idempotent.
        let dir2 = tempdir().unwrap();
        let peer = test_host(dir2.path());
        assert!(peer.admit_sync_operation(&op).unwrap().is_validated());
        assert_eq!(
            peer.admit_sync_operation(&op).unwrap(),
            crate::wellfair::sync_protocol::AdmitOutcome::Duplicate
        );
        assert_eq!(peer.validated_sync_operations().unwrap().len(), 1);
    }

    #[test]
    fn clinical_report_commits_with_clinical_kind() {
        use wellfare_core::clinical::ClinicalReportType;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let entry = host
            .add_clinical_report(
                "Full blood count",
                ClinicalReportType::Pathology,
                1_700_000_000,
                "Hb 140 g/L",
                Some("Dr Smith".into()),
            )
            .unwrap();
        assert_eq!(entry.kind, "clinical_report");
        assert_eq!(host.list_clinical_reports(8).unwrap().len(), 1);
    }

    #[test]
    fn work_items_commit_and_board_derives_replay_safe() {
        use qualia_cooperative_core::work_item::{
            WorkItem, WorkItemStatus, WorkItemStatusEvent, WorkItemType,
        };
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let wi = WorkItem::new("proj-A", WorkItemType::Task, "Write tests", 1_700_000_000);
        host.add_work_item(&wi).unwrap();
        host.add_work_item_status(&WorkItemStatusEvent::new(
            &wi.id,
            WorkItemStatus::InProgress,
            1_700_000_100,
        ))
        .unwrap();

        let board = host.work_item_board("proj-A", 64).unwrap();
        let in_progress = board
            .iter()
            .find(|c| c.status == WorkItemStatus::InProgress)
            .unwrap();
        assert_eq!(in_progress.cards.len(), 1);
        assert_eq!(in_progress.cards[0].title, "Write tests");
        // No card left in the default Todo column.
        let todo = board
            .iter()
            .find(|c| c.status == WorkItemStatus::Todo)
            .unwrap();
        assert!(todo.cards.is_empty());

        // A different project's board has no cards.
        assert!(host
            .work_item_board("proj-B", 64)
            .unwrap()
            .iter()
            .all(|c| c.cards.is_empty()));
    }

    #[test]
    fn clinical_attachment_stores_and_retrieves_bytes() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let bytes = b"%PDF-1.4 pretend pathology report bytes";
        let entry = host
            .add_clinical_attachment("path_report.pdf", "application/pdf", bytes)
            .unwrap();
        assert_eq!(entry.kind, "clinical_attachment");
        assert_eq!(host.list_clinical_attachments(8).unwrap().len(), 1);
        // The bytes round-trip out of the blob store, integrity-verified.
        let got = host.attachment_bytes(&entry.id).unwrap().unwrap();
        assert_eq!(got, bytes);
    }

    #[test]
    fn government_letter_attachment_stores_and_retrieves_bytes() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let bytes = b"%PDF-1.4 letter from the agency";
        let entry = host
            .add_government_letter_attachment("Services Australia", "Payment review", true, bytes)
            .unwrap();
        assert_eq!(entry.kind, "government_letter");
        // The generalized attachment_bytes reads any record's blob back.
        let got = host.attachment_bytes(&entry.id).unwrap().unwrap();
        assert_eq!(got, bytes);
    }

    #[test]
    fn welfare_records_commit_and_list() {
        use wellfare_core::welfare_support::{StreamStatus, Urgency};
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_assistance_need("housing", "emergency accommodation", Urgency::Critical)
            .unwrap();
        host.add_welfare_stream("JobSeeker", Some("ref-42".into()), StreamStatus::Active)
            .unwrap();
        host.add_government_letter("Services Australia", "Payment review", true)
            .unwrap();
        let rows = host.list_welfare_records(16).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().any(|r| r.kind == "assistance_need"));
        assert!(rows.iter().any(|r| r.kind == "welfare_stream"));
        assert!(rows.iter().any(|r| r.kind == "government_letter"));
    }

    #[test]
    fn synced_obligations_fold_in_validated_remote_contributions_replay_safe() {
        use wellfare_core::projects::{Contribution, Project};
        // Peer A commits a contribution and emits a signed sync operation for it.
        let dir_a = tempdir().unwrap();
        let mut peer_a = test_host(dir_a.path());
        let pa = Project::new("Shared Garden", "beds", vec![], 1_700_000_000);
        peer_a.add_project(&pa).unwrap();
        let a_entry = peer_a
            .add_contribution(&Contribution::new(
                &pa.id,
                "did:wf:alice",
                "dig",
                60,
                0,
                1.0,
                wellfare_core::projects::ContributionPrivacy::Public,
                1_700_000_050,
            ))
            .unwrap();
        let remote_op = peer_a.build_outbound_operation(&a_entry, 5).unwrap();

        // Peer B has its own local contribution to the same project id.
        let dir_b = tempdir().unwrap();
        let mut peer_b = test_host(dir_b.path());
        peer_b
            .add_contribution(&Contribution::new(
                &pa.id,
                "did:wf:bob",
                "plant",
                30,
                0,
                1.0,
                wellfare_core::projects::ContributionPrivacy::Public,
                1_700_000_100,
            ))
            .unwrap();

        // Local-only view: just Bob's 30 min.
        let local = peer_b.project_obligations(64).unwrap();
        assert_eq!(
            local.iter().map(|o| o.total_effort_minutes).sum::<u64>(),
            30
        );

        // Admit the remote op, then the synced view includes Alice's 60 min too.
        assert!(peer_b
            .admit_sync_operation(&remote_op)
            .unwrap()
            .is_validated());
        let synced = peer_b.synced_project_obligations(64).unwrap();
        let alice = synced
            .iter()
            .find(|o| o.contributor_did == "did:wf:alice")
            .unwrap();
        assert_eq!(alice.total_effort_minutes, 60);
        let bob = synced
            .iter()
            .find(|o| o.contributor_did == "did:wf:bob")
            .unwrap();
        assert_eq!(bob.total_effort_minutes, 30);

        // Replaying the remote op does not double-count: the synced view is unchanged.
        assert_eq!(
            peer_b.admit_sync_operation(&remote_op).unwrap(),
            crate::wellfair::sync_protocol::AdmitOutcome::Duplicate
        );
        let after_replay = peer_b.synced_project_obligations(64).unwrap();
        assert_eq!(after_replay, synced);
    }

    #[test]
    fn classified_record_produces_no_outbound_operation() {
        use wellfare_core::mental_wellbeing::TherapyNote;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        // therapy_note is a Classified, sanctuary-protected kind (the encrypted vault holds
        // free-text sanctuary notes; the journal still carries other Classified kinds).
        let entry = host
            .add_therapy_note(&TherapyNote::new("private contingency"))
            .unwrap();
        assert_eq!(entry.sensitivity, "Classified");
        assert!(host.build_outbound_operation(&entry, 1).is_none());
    }

    #[test]
    fn locked_sanctuary_hides_protected_kinds_from_graph_coverage() {
        use wellfare_core::mental_wellbeing::TherapyNote;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_condition(&ConditionReport::new("Hypertension"))
            .unwrap();
        host.add_therapy_note(&TherapyNote::new("private contingency"))
            .unwrap();
        host.finalize_batch().unwrap();

        // Before Sanctuary is set up, coverage lists every kind.
        let unlocked = host.query_graph_coverage(32).unwrap();
        assert!(unlocked.iter().any(|r| r.kind == "therapy_note"));
        assert!(unlocked.iter().any(|r| r.kind == "condition"));

        // Once set up and locked, the protected kind is withheld from the coverage view.
        host.setup_sanctuary("real-pin-cov", "decoy-pin-cov")
            .unwrap();
        host.lock_sanctuary().unwrap();
        let locked = host.query_graph_coverage(32).unwrap();
        assert!(locked.iter().all(|r| r.kind != "therapy_note"));
        assert!(locked.iter().any(|r| r.kind == "condition"));
    }
}
