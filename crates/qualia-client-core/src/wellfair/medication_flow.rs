//! Integration tests for medication host API path.

#[cfg(test)]
mod tests {
    use ed25519_dalek::SigningKey;
    use std::path::PathBuf;

    use wellfare_core::medication::AdministrationStatus;

    use super::super::api::WebizenHostApi;
    use super::super::policy::PolicyDecisionService;
    use super::super::vault::VaultService;

    fn test_host(dir: &tempfile::TempDir) -> WebizenHostApi {
        let wal = dir.path().join("test.wal");
        let vault = VaultService::open(&wal, dir.path(), 0xBEEF).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
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
    fn medication_diet_and_administration_journal_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = test_host(&dir);

        let med = host
            .add_medication("Ibuprofen", "200mg", "oral", vec!["08:00".into()])
            .expect("add med");
        assert_eq!(med.kind, "medication");

        let admin = host
            .record_administration(
                &med.id,
                "Ibuprofen",
                AdministrationStatus::Taken,
                None,
            )
            .expect("admin");
        assert_eq!(admin.kind, "med_administration");

        let diet = host
            .add_diet_entry("Oats and fruit", "breakfast", Some(350))
            .expect("diet");
        assert_eq!(diet.kind, "diet");

        let meds = host.list_journal_by_kind("medication", 10).unwrap();
        assert!(!meds.is_empty());
    }

    #[test]
    fn sleep_analytics_empty_journal() {
        let dir = tempfile::tempdir().unwrap();
        let host = test_host(&dir);
        let (debt, hm) = host.default_sleep_analytics().unwrap();
        assert_eq!(debt.nights_analyzed, 0);
        assert!(hm.cells.is_empty());
    }
}