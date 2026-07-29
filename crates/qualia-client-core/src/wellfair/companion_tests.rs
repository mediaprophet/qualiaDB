//! Companion ingest path — phone bundle → journal (§8.1 step 4, Phase 2 closeout).

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ed25519_dalek::SigningKey;
    use wellfare_core::companion_sync::{CompanionCsvFile, CompanionHealthBundle};

    use crate::wellfair::api::WebizenHostApi;
    use crate::wellfair::policy::PolicyDecisionService;
    use crate::wellfair::vault::VaultService;

    const WEIGHT_CSV: &str = "uuid,start_time,end_time,time_offset,weight,body_fat,muscle_mass,body_water,skeletal_muscle,bmi\n\
a2000001-0000-4000-8000-000000000002,1777632000000,1777632060000,60,71.5,18.0,31.8,55.0,30.2,22.9\n";

    fn host(dir: &tempfile::TempDir) -> WebizenHostApi {
        let wal = dir.path().join("companion.wal");
        let vault = VaultService::open(&wal, dir.path(), 0xCAFE).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[22u8; 32]);
        WebizenHostApi::new(
            vault,
            policy,
            signing_key,
            "did:wf:owner".into(),
            "did:wf:owner".into(),
            PathBuf::from(dir.path()),
        )
    }

    #[test]
    fn companion_bundle_ingest_commits_weight_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host(&dir);
        let bundle = CompanionHealthBundle::new(
            "phone-closeout-1",
            1_700_000_100,
            vec![CompanionCsvFile {
                filename: "weight.csv".into(),
                csv_content: WEIGHT_CSV.into(),
            }],
        );
        let report = host.ingest_companion_health_bundle(&bundle);
        assert!(
            report.records_committed > 0,
            "expected commits, got {:?}",
            report
        );
        let journal = host.list_health_records(32).unwrap();
        assert!(
            journal.iter().any(|e| e.kind == "weight"),
            "journal should contain companion-derived rows: {:?}",
            journal
        );
    }

    #[test]
    fn companion_bundle_survives_checkpoint_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host(&dir);
        let bundle = CompanionHealthBundle::new(
            "phone-closeout-2",
            1_700_000_200,
            vec![CompanionCsvFile {
                filename: "weight.csv".into(),
                csv_content: WEIGHT_CSV.into(),
            }],
        );
        host.ingest_companion_health_bundle(&bundle);
        host.finalize_batch().unwrap();

        let wal = dir.path().join("companion.wal");
        let vault2 = VaultService::open(&wal, dir.path(), 0xCAFE).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[22u8; 32]);
        let host2 = WebizenHostApi::new(
            vault2,
            policy,
            signing_key,
            "did:wf:owner".into(),
            "did:wf:owner".into(),
            PathBuf::from(dir.path()),
        );
        let count = host2.list_health_records(64).unwrap().len();
        assert!(count > 0, "records should persist after checkpoint reopen");
    }
}
