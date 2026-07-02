use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use qualia_core_db::crdt::SuspendedTransactionQueue;
use qualia_core_db::git_bridge::DagStore;
use qualia_core_db::wal::{commit_semantic_mutation, WalHandoffResult, WriteAheadLog};
use qualia_core_db::NQuin;
use wellfare_core::record::{RecordEnvelope, SensitivityClass};

use super::checkpoint_store::{self, load_dag, load_meta};
use super::consent_store::ConsentStore;
use super::graph_store::GraphStore;
use super::journal::{JournalEntry, WellfairJournal};
use super::receipt::{ReceiptLog, ReceiptRecord};
use super::sync_outbox::{SyncOutbox, SyncOutboxEntry};

/// Coordinates transactions, graph materialization, and crash recovery.
pub struct VaultService {
    wal: WriteAheadLog,
    dag: DagStore,
    graph: GraphStore,
    suspended: SuspendedTransactionQueue,
    author_did: u64,
    storage_root: PathBuf,
    journal: WellfairJournal,
    receipts: ReceiptLog,
    consents: ConsentStore,
    sync_outbox: SyncOutbox,
    last_checkpoint_hash: Option<[u8; 32]>,
}

impl VaultService {
    pub fn open<W: AsRef<Path>, S: AsRef<Path>>(
        wal_path: W,
        storage_root: S,
        author_did: u64,
    ) -> std::io::Result<Self> {
        let storage_root = storage_root.as_ref().to_path_buf();
        let wal = WriteAheadLog::open(wal_path)?;
        let dag = load_dag(&storage_root);
        let graph = GraphStore::open(&storage_root)?;
        let suspended = SuspendedTransactionQueue::new();
        let journal = WellfairJournal::open(&storage_root)?;
        let receipts = ReceiptLog::open(&storage_root)?;
        let consents = ConsentStore::open(&storage_root)?;
        let sync_outbox = SyncOutbox::open(&storage_root)?;

        let last_checkpoint_hash = if wal.prev_dag_hash != [0u8; 32] {
            Some(wal.prev_dag_hash)
        } else if let Some(meta) = load_meta(&storage_root) {
            hex::decode(&meta.last_hash_hex)
                .ok()
                .and_then(|b| b.try_into().ok())
        } else {
            None
        };

        Ok(Self {
            wal,
            dag,
            graph,
            suspended,
            author_did,
            storage_root,
            journal,
            receipts,
            consents,
            sync_outbox,
            last_checkpoint_hash,
        })
    }

    pub fn graph_quin_count(&self) -> usize {
        self.graph.count()
    }

    pub fn list_graph_quins(&self, limit: usize) -> std::io::Result<Vec<NQuin>> {
        self.graph.list_recent(limit)
    }

    pub fn graph_coverage(&self, journal_limit: usize) -> std::io::Result<Vec<super::graph_query::GraphCoverageRow>> {
        let journal = self.journal.list_recent(journal_limit)?;
        let quin_limit = journal.len().saturating_mul(8).max(64);
        let quins = self.graph.list_recent(quin_limit)?;
        Ok(super::graph_query::coverage_for_journal(&journal, &quins))
    }

    pub fn journal_count(&self) -> std::io::Result<usize> {
        self.journal.count()
    }

    pub fn list_health_records(&self, limit: usize) -> std::io::Result<Vec<JournalEntry>> {
        self.journal.list_recent(limit)
    }

    pub fn list_receipts(&self, limit: usize) -> std::io::Result<Vec<ReceiptRecord>> {
        self.receipts.list_recent(limit)
    }

    pub fn list_outbox(&self, limit: usize) -> std::io::Result<Vec<SyncOutboxEntry>> {
        self.sync_outbox.list_recent(limit)
    }

    pub fn outbox_queued_count(&self) -> std::io::Result<usize> {
        self.sync_outbox.count_queued()
    }

    pub fn list_active_consents(&self, now_unix: u64) -> std::io::Result<Vec<super::consent_store::ConsentGrantRecord>> {
        self.consents.list_active(now_unix)
    }

    pub fn append_consent(
        &self,
        grant: &super::consent_store::ConsentGrantRecord,
    ) -> std::io::Result<()> {
        self.consents.append(grant)
    }

    pub fn revoke_consent(&self, grant_id: &str) -> std::io::Result<bool> {
        self.consents.revoke(grant_id)
    }

    pub fn wal_buffered_quins(&mut self) -> std::io::Result<usize> {
        self.wal.buffered_count()
    }

    /// Appends a semantic mutation to the durable WAL.
    pub fn commit_quin(
        &mut self,
        mut quin: NQuin,
        signing_key: &SigningKey,
        principal_did_hash: u64,
    ) -> std::io::Result<WalHandoffResult> {
        commit_semantic_mutation(
            &mut self.wal,
            &mut quin,
            principal_did_hash,
            self.author_did,
            signing_key,
            &mut self.suspended,
        )
    }

    /// Commits buffered WAL events into the content-addressed DAG, materializes graph, persists checkpoint.
    pub fn checkpoint(&mut self) -> std::io::Result<[u8; 32]> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let hash = self
            .wal
            .checkpoint_to_dag(&mut self.dag, self.author_did, timestamp_ms)?;
        let committed = self.wal.recover()?;
        if !committed.is_empty() {
            self.graph.append_quins(&committed)?;
            checkpoint_store::persist_checkpoint(
                &self.storage_root,
                &self.dag,
                hash,
                self.graph.count(),
                &committed,
                self.author_did,
            )?;
        } else if self.dag.nodes().len() > 0 {
            checkpoint_store::save_dag(&self.storage_root, &self.dag)?;
        }
        self.wal.truncate()?;
        self.last_checkpoint_hash = Some(hash);
        Ok(hash)
    }

    pub fn last_checkpoint_hash(&self) -> Option<[u8; 32]> {
        self.last_checkpoint_hash
    }

    pub fn checkpoint_meta(&self) -> Option<checkpoint_store::CheckpointMeta> {
        load_meta(&self.storage_root)
    }

    /// Converts a wellfare RecordEnvelope into NQuins and commits them.
    pub fn commit_envelope(
        &mut self,
        envelope: &RecordEnvelope,
        signing_key: &SigningKey,
        principal_did_hash: u64,
        source: &str,
        summary: Option<String>,
    ) -> std::io::Result<usize> {
        let mut buffer = [wellfare_core::record::NQuin::default(); 8];
        let count = envelope.compile_to_quins(&mut buffer);
        let mut committed = 0;
        for i in 0..count {
            let src = &buffer[i];
            let q = NQuin {
                subject: src.subject,
                predicate: src.predicate,
                object: src.object,
                context: src.context,
                metadata: src.metadata,
                parity: src.parity,
            };
            self.commit_quin(q, signing_key, principal_did_hash)?;
            committed += 1;
        }

        let committed_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        let entry = JournalEntry::from_envelope(envelope, source, committed_unix, summary);
        self.journal.append(&entry)?;
        // Sanctuary/Classified records are excluded from the ordinary sync outbox
        // before an operation can enter it (master plan §5.2). Their existence,
        // record id, and kind never leak onto the ordinary routing lane; a
        // dedicated Sanctuary transport (future ADR) is the only permitted path.
        if envelope.sensitivity != SensitivityClass::Classified {
            let outbox_entry = SyncOutboxEntry::from_envelope(envelope, committed_unix);
            self.sync_outbox.enqueue(&outbox_entry)?;
        }
        Ok(committed)
    }

    pub fn append_receipt(&self, receipt: &ReceiptRecord) -> std::io::Result<()> {
        self.receipts.append(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use wellfare_core::record::{
        EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass,
    };

    #[test]
    fn checkpoint_persists_graph_and_meta() {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");
        let mut vault = VaultService::open(&wal_path, dir.path(), 0xBEEF).unwrap();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let envelope = RecordEnvelope {
            id: "urn:wellfair:weight:chk".into(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:owner".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::DeviceMeasured,
            sensitivity: SensitivityClass::Restricted,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: None,
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: Some("abc".into()),
            tombstone: false,
        };
        vault
            .commit_envelope(&envelope, &signing_key, 1, "test", None)
            .unwrap();
        let hash = vault.checkpoint().unwrap();
        assert_ne!(hash, [0u8; 32]);
        assert!(vault.graph_quin_count() > 0);
        assert!(dir.path().join(checkpoint_store::META_FILE).exists());
        assert!(dir.path().join(checkpoint_store::DAG_FILE).exists());

        let wal_path2 = dir.path().join("test2.wal");
        std::fs::copy(&wal_path, &wal_path2).unwrap();
        let graph_before = vault.graph_quin_count();
        let meta_before = vault.checkpoint_meta().unwrap();
        let reopened = VaultService::open(&wal_path2, dir.path(), 0xBEEF).unwrap();
        assert_eq!(reopened.graph_quin_count(), graph_before);
        let meta_after = reopened.checkpoint_meta().unwrap();
        assert_eq!(meta_after.graph_quin_count, meta_before.graph_quin_count);
        assert_eq!(meta_after.last_hash_hex, meta_before.last_hash_hex);
        #[cfg(not(target_arch = "wasm32"))]
        assert!(dir.path().join(checkpoint_store::Q42_FILE).exists());
    }

    #[test]
    fn classified_records_excluded_from_sync_outbox() {
        let dir = tempfile::tempdir().unwrap();
        let wal_path = dir.path().join("sanct.wal");
        let mut vault = VaultService::open(&wal_path, dir.path(), 0xBEEF).unwrap();
        let signing_key = SigningKey::from_bytes(&[3u8; 32]);

        let restricted = RecordEnvelope {
            id: "urn:wellfair:weight:r1".into(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:owner".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::DeviceMeasured,
            sensitivity: SensitivityClass::Restricted,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: None,
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };
        let classified = RecordEnvelope {
            id: "urn:wellfair:sanctuary_note:s1".into(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:owner".into(),
            proxy_did: None,
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Classified,
            asserted_time_unix: 1_700_000_100,
            valid_time_start_unix: None,
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        };

        vault
            .commit_envelope(&restricted, &signing_key, 1, "test", None)
            .unwrap();
        vault
            .commit_envelope(&classified, &signing_key, 1, "test", None)
            .unwrap();

        // Both records are durably journaled...
        assert_eq!(vault.journal_count().unwrap(), 2);
        // ...but only the Restricted record reaches the ordinary sync outbox (§5.2).
        let outbox = vault.list_outbox(16).unwrap();
        assert_eq!(outbox.len(), 1);
        assert_eq!(outbox[0].record_id, "urn:wellfair:weight:r1");
        assert!(outbox
            .iter()
            .all(|e| e.record_id != "urn:wellfair:sanctuary_note:s1"));
    }
}