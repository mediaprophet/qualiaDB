use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use qualia_core_db::crdt::SuspendedTransactionQueue;
use qualia_core_db::git_bridge::DagStore;
use qualia_core_db::wal::{commit_semantic_mutation, WalHandoffResult, WriteAheadLog};
use qualia_core_db::NQuin;
use wellfare_core::record::RecordEnvelope;

use super::journal::{JournalEntry, WellfairJournal};
use super::receipt::{ReceiptLog, ReceiptRecord};

/// Coordinates transactions, graph materialization, and crash recovery.
pub struct VaultService {
    wal: WriteAheadLog,
    dag: DagStore,
    suspended: SuspendedTransactionQueue,
    author_did: u64,
    journal: WellfairJournal,
    receipts: ReceiptLog,
    last_checkpoint_hash: Option<[u8; 32]>,
}

impl VaultService {
    pub fn open<W: AsRef<Path>, S: AsRef<Path>>(
        wal_path: W,
        storage_root: S,
        author_did: u64,
    ) -> std::io::Result<Self> {
        let wal = WriteAheadLog::open(wal_path)?;
        let dag = DagStore::new();
        let suspended = SuspendedTransactionQueue::new();
        let journal = WellfairJournal::open(&storage_root)?;
        let receipts = ReceiptLog::open(&storage_root)?;
        Ok(Self {
            wal,
            dag,
            suspended,
            author_did,
            journal,
            receipts,
            last_checkpoint_hash: None,
        })
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

    /// Commits buffered WAL events into the content-addressed DAG and truncates the WAL.
    pub fn checkpoint(&mut self) -> std::io::Result<[u8; 32]> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let hash = self
            .wal
            .checkpoint_to_dag(&mut self.dag, self.author_did, timestamp_ms)?;
        self.wal.truncate()?;
        self.last_checkpoint_hash = Some(hash);
        Ok(hash)
    }

    pub fn last_checkpoint_hash(&self) -> Option<[u8; 32]> {
        self.last_checkpoint_hash
    }

    /// Converts a wellfare RecordEnvelope into NQuins and commits them.
    pub fn commit_envelope(
        &mut self,
        envelope: &RecordEnvelope,
        signing_key: &SigningKey,
        principal_did_hash: u64,
        source: &str,
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
        let entry = JournalEntry::from_envelope(envelope, source, committed_unix);
        self.journal.append(&entry)?;
        Ok(committed)
    }

    pub fn append_receipt(&self, receipt: &ReceiptRecord) -> std::io::Result<()> {
        self.receipts.append(receipt)
    }
}