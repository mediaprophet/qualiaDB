//! Crash/replay integration tests for WellFair vault durability.

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ed25519_dalek::SigningKey;
    use wellfare_core::record::{EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass};

    use super::super::api::WebizenHostApi;
    use super::super::checkpoint_store;
    use super::super::policy::PolicyDecisionService;
    use super::super::vault::VaultService;

    fn sample_weight_envelope(id: &str) -> RecordEnvelope {
        RecordEnvelope {
            id: id.into(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:owner".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::DeviceMeasured,
            sensitivity: SensitivityClass::Restricted,
            asserted_time_unix: 1_700_000_000,
            asserted_instant: None,
            valid_time_start_unix: None,
            valid_time_start_instant: None,
            valid_time_end_unix: None,
            valid_time_end_instant: None,
            predecessor_id: None,
            blob_hash: Some("abc".into()),
            tombstone: false,
        }
    }

    fn open_vault(dir: &tempfile::TempDir, wal_name: &str) -> VaultService {
        let wal_path = dir.path().join(wal_name);
        VaultService::open(&wal_path, dir.path(), 0xBEEF).unwrap()
    }

    fn test_host(dir: &tempfile::TempDir) -> WebizenHostApi {
        let vault = open_vault(dir, "test.wal");
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

    fn seed_vault_with_checkpoint(vault: &mut VaultService) -> [u8; 32] {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let envelope = sample_weight_envelope("urn:wellfair:weight:replay");
        vault
            .commit_envelope(&envelope, &signing_key, 1, "replay-test", None)
            .unwrap();
        vault.checkpoint().unwrap()
    }

    #[test]
    fn reopen_after_checkpoint_preserves_graph_and_journal() {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");
        let mut vault = VaultService::open(&wal_path, dir.path(), 0xBEEF).unwrap();
        let hash = seed_vault_with_checkpoint(&mut vault);

        let graph_before = vault.graph_quin_count();
        let journal_before = vault.list_health_records(10).unwrap();
        let meta_before = vault.checkpoint_meta().unwrap();
        assert!(graph_before > 0);
        assert_eq!(journal_before.len(), 1);
        assert_ne!(hash, [0u8; 32]);

        let wal_path2 = dir.path().join("test2.wal");
        std::fs::copy(&wal_path, &wal_path2).unwrap();
        let reopened = VaultService::open(&wal_path2, dir.path(), 0xBEEF).unwrap();

        assert_eq!(reopened.graph_quin_count(), graph_before);
        assert_eq!(reopened.journal_count().unwrap(), journal_before.len());
        let journal_after = reopened.list_health_records(10).unwrap();
        assert_eq!(journal_after, journal_before);

        let meta_after = reopened.checkpoint_meta().unwrap();
        assert_eq!(meta_after.graph_quin_count, meta_before.graph_quin_count);
        assert_eq!(meta_after.last_hash_hex, meta_before.last_hash_hex);
        assert_eq!(reopened.last_checkpoint_hash(), Some(hash));
        assert!(dir.path().join(checkpoint_store::META_FILE).exists());
        assert!(dir.path().join(checkpoint_store::DAG_FILE).exists());
        #[cfg(not(target_arch = "wasm32"))]
        assert!(dir.path().join(checkpoint_store::Q42_FILE).exists());
    }

    #[test]
    fn double_checkpoint_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = open_vault(&dir, "test.wal");
        seed_vault_with_checkpoint(&mut vault);

        let hash_first = vault.last_checkpoint_hash().unwrap();
        let graph_first = vault.graph_quin_count();
        let journal_first = vault.journal_count().unwrap();
        let meta_first = vault.checkpoint_meta().unwrap();

        let hash_second = vault.checkpoint().unwrap();
        assert_eq!(hash_second, hash_first);
        assert_eq!(vault.last_checkpoint_hash(), Some(hash_first));
        assert_eq!(vault.graph_quin_count(), graph_first);
        assert_eq!(vault.journal_count().unwrap(), journal_first);

        let meta_second = vault.checkpoint_meta().unwrap();
        assert_eq!(meta_second.graph_quin_count, meta_first.graph_quin_count);
        assert_eq!(meta_second.last_hash_hex, meta_first.last_hash_hex);
        assert_eq!(meta_second.dag_node_count, meta_first.dag_node_count);
    }

    #[test]
    fn policy_denied_write_does_not_grow_journal() {
        let dir = tempfile::tempdir().unwrap();
        let mut host = test_host(&dir);
        assert_eq!(host.list_health_records(10).unwrap().len(), 0);
        assert_eq!(host.graph_quin_count(), 0);

        let classified = RecordEnvelope {
            sensitivity: SensitivityClass::Classified,
            ..sample_weight_envelope("urn:wellfair:weight:denied-classified")
        };
        let err = host
            .submit_record("wellfair-health", classified, "replay-test")
            .unwrap_err();
        assert!(err.contains("Policy denied"));
        assert_eq!(host.list_health_records(10).unwrap().len(), 0);
        assert_eq!(host.graph_quin_count(), 0);

        let refuted = RecordEnvelope {
            epistemic_status: EpistemicStatus::Refuted,
            ..sample_weight_envelope("urn:wellfair:weight:denied-refuted")
        };
        let err = host
            .submit_record("wellfair-health", refuted, "replay-test")
            .unwrap_err();
        assert!(err.contains("Policy denied"));
        assert_eq!(host.list_health_records(10).unwrap().len(), 0);
        assert_eq!(host.graph_quin_count(), 0);
    }
}
