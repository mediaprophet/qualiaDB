//! Phase 3 integration — life, wellbeing, sanctuary projection.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ed25519_dalek::SigningKey;
    use wellfare_core::life_records::{CaseTaskReport, LifeEventReport, WelfareCaseReport};
    use wellfare_core::mental_wellbeing::{TherapyNote, WellbeingObservation};

    use crate::wellfair::api::WebizenHostApi;
    use crate::wellfair::policy::PolicyDecisionService;
    use crate::wellfair::sanctuary::is_sanctuary_protected_kind;
    use crate::wellfair::vault::VaultService;

    fn host(dir: &tempfile::TempDir) -> WebizenHostApi {
        let wal = dir.path().join("phase3.wal");
        let vault = VaultService::open(&wal, dir.path(), 0xBEEF).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[33u8; 32]);
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
    fn life_event_and_welfare_case_journal_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host(&dir);
        let life = host
            .add_life_event(&LifeEventReport::new("Job loss"))
            .unwrap();
        assert_eq!(life.kind, "life_event");
        let case = host
            .add_welfare_case(&WelfareCaseReport::new("Rent arrears"))
            .unwrap();
        assert_eq!(case.kind, "welfare_case");
        assert!(is_sanctuary_protected_kind("welfare_case"));
    }

    #[test]
    fn case_task_links_to_welfare_case() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host(&dir);
        let case = host
            .add_welfare_case(&WelfareCaseReport::new("Housing review"))
            .unwrap();
        assert_eq!(case.kind, "welfare_case");
        let case_uuid = case.id.rsplit(':').next().expect("case uuid");
        let task = host
            .add_case_task(&CaseTaskReport::new(case_uuid, "Gather tenancy documents"))
            .unwrap();
        assert_eq!(task.kind, "case_task");
        assert!(task.summary.as_ref().is_some_and(|s| s.contains(case_uuid)));
    }

    #[test]
    fn sanctuary_lock_hides_therapy_from_projection() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = host(&dir);
        host.setup_sanctuary("real-pin-99", "decoy-pin-88").unwrap();
        host.add_wellbeing_observation(&WellbeingObservation::new("low"))
            .unwrap();
        host.add_therapy_note(&TherapyNote::new("private session notes"))
            .unwrap();
        host.lock_sanctuary().unwrap();
        let visible = host.list_health_records(32).unwrap();
        assert!(visible.iter().any(|e| e.kind == "wellbeing_observation"));
        assert!(!visible.iter().any(|e| e.kind == "therapy_note"));
        host.unlock_sanctuary("real-pin-99").unwrap();
        let unlocked = host.list_health_records(32).unwrap();
        assert!(unlocked.iter().any(|e| e.kind == "therapy_note"));
    }
}
