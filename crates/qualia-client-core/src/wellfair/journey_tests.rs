//! §8.1 first-usable-release journey — API-level integration test.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ed25519_dalek::SigningKey;
    use wellfare_core::companion_sync::{CompanionCsvFile, CompanionHealthBundle};
    use wellfare_core::conditions::ConditionReport;
    use wellfare_core::medication::AdministrationStatus;
    use wellfare_core::record::{
        EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass,
    };

    use super::super::host_state::AccessibilityPreferences;
    use super::super::api::WebizenHostApi;
    use super::super::checkpoint_store;
    use super::super::host_state::ConsentGrantDraft;
    use super::super::policy::PolicyDecisionService;
    use super::super::vault::VaultService;

    const WEIGHT_CSV: &str = "uuid,start_time,end_time,time_offset,weight,body_fat,muscle_mass,body_water,skeletal_muscle,bmi\n\
a1000001-0000-4000-8000-000000000001,1777632000000,1777632060000,60,72.0,18.5,32.1,55.2,30.5,23.1\n";

    fn journey_host(dir: &tempfile::TempDir) -> WebizenHostApi {
        let wal = dir.path().join("journey.wal");
        let vault = VaultService::open(&wal, dir.path(), 0xBEEF).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[11u8; 32]);
        WebizenHostApi::new(
            vault,
            policy,
            signing_key,
            "did:wf:owner".into(),
            "did:wf:owner".into(),
            PathBuf::from(dir.path()),
        )
    }

    fn sample_bundle() -> CompanionHealthBundle {
        CompanionHealthBundle::new(
            "phone-journey-1",
            1_700_000_000,
            vec![CompanionCsvFile {
                filename: "weight.csv".into(),
                csv_content: WEIGHT_CSV.into(),
            }],
        )
    }

    #[test]
    fn section_8_1_first_usable_journey_offline_and_restart() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = journey_host(&dir);

        // Step 3 — profile + accessibility
        let prefs = AccessibilityPreferences {
            high_contrast: true,
            reduced_motion: true,
            screen_reader_hints: false,
            text_scale_percent: 110,
        };
        host.save_accessibility(&prefs).unwrap();
        assert!(host.load_accessibility().high_contrast);

        // Step 4 — Samsung companion ingest
        let bundle = sample_bundle();
        let import_report = host.ingest_companion_health_bundle(&bundle);
        assert!(import_report.records_committed > 0, "{import_report:?}");

        // Step 6 — medication, administration, diet
        host.add_medication("Metformin", "500mg", "oral", vec!["08:00".into()])
            .unwrap();
        host.record_administration(
            "urn:wellfair:medication:ignored",
            "Metformin",
            AdministrationStatus::Taken,
            None,
        )
        .unwrap();
        host.add_diet_entry("Oatmeal", "breakfast", Some(320)).unwrap();

        // Step 3 (personal) — self-reported condition
        host.add_condition(&ConditionReport::new("Hypertension"))
            .unwrap();

        // Step 7–8 — contact relationship + scoped sharing request (consent grant)
        let draft = ConsentGrantDraft {
            recipient: "wellfair-care".into(),
            purpose: "care_coordination".into(),
            fields: vec!["weight".into(), "medication".into()],
            expires_at_unix: Some(4_000_000_000),
        };
        let grant = host.grant_consent(&draft, "read_record").unwrap();
        assert!(!grant.revoked);

        let decision = host
            .evaluate_policy(
                "wellfair-care",
                "read_record",
                SensitivityClass::Restricted,
                EpistemicStatus::Asserted,
            )
            .unwrap();
        assert!(matches!(
            decision,
            super::super::host_state::PolicyDecisionDto::Permit { .. }
        ));

        // Checkpoint + graph coverage
        let hash_hex = host.finalize_batch().unwrap();
        assert!(!hash_hex.is_empty());
        let coverage = host.query_graph_coverage(32).unwrap();
        assert!(!coverage.is_empty());
        for row in &coverage {
            assert!(
                row.quin_count > 0,
                "record {} should have materialized quins",
                row.record_id
            );
        }

        // Step 9 — export package + receipt
        let (pkg, export_receipt) = host.export_health_package(64).unwrap();
        assert!(pkg.record_count >= 4);
        assert!(pkg.turtle_body.contains("@prefix wf:"));
        assert!(!export_receipt.export_sha256_hex.is_empty());
        let receipts = host.list_receipts(16).unwrap();
        assert!(receipts.iter().any(|r| r.obligations.contains(&"standards_readable_export".into())));

        let graph_before = host.graph_quin_count();
        let journal_before = host.list_health_records(64).unwrap().len();
        let outbox_before = host.list_outbox(32).unwrap().len();

        // Step 10 — restart offline: reopen vault from same storage
        let wal_path2 = dir.path().join("journey2.wal");
        std::fs::copy(dir.path().join("journey.wal"), &wal_path2).unwrap();
        let reopened = VaultService::open(&wal_path2, dir.path(), 0xBEEF).unwrap();
        assert_eq!(reopened.graph_quin_count(), graph_before);
        assert_eq!(reopened.journal_count().unwrap(), journal_before);
        assert_eq!(reopened.list_outbox(32).unwrap().len(), outbox_before);
        assert!(dir.path().join(checkpoint_store::META_FILE).exists());
        assert!(dir.path().join(checkpoint_store::DAG_FILE).exists());

        // Policy still fail-closed on classified writes after restart
        let mut host2 = journey_host(&dir);
        let denied = RecordEnvelope {
            sensitivity: SensitivityClass::Classified,
            id: "urn:wellfair:weight:denied".into(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:owner".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::DeviceMeasured,
            asserted_time_unix: 1,
            valid_time_start_unix: None,
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };
        assert!(host2
            .submit_record("wellfair-health", denied, "journey")
            .is_err());
    }
}