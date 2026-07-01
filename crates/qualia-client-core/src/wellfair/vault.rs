use std::path::Path;
use qualia_core_db::wal::{WriteAheadLog, WalHandoffResult, commit_semantic_mutation};
use qualia_core_db::git_bridge::DagStore;
use qualia_core_db::crdt::SuspendedTransactionQueue;
use qualia_core_db::NQuin;
use ed25519_dalek::SigningKey;
use wellfare_core::record::RecordEnvelope;

/// Coordinates transactions, graph materialization, and crash recovery.
pub struct VaultService {
    wal: WriteAheadLog,
    dag: DagStore,
    suspended: SuspendedTransactionQueue,
    author_did: u64,
}

impl VaultService {
    pub fn open<P: AsRef<Path>>(wal_path: P, author_did: u64) -> std::io::Result<Self> {
        let wal = WriteAheadLog::open(wal_path)?;
        let dag = DagStore::new();
        // Assume default initialization for SuspendedTransactionQueue
        // (will adjust if needed by compiler)
        let suspended = SuspendedTransactionQueue::new();
        Ok(Self { wal, dag, suspended, author_did })
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

    /// Commits buffered WAL events into the content-addressed DAG.
    pub fn checkpoint(&mut self, timestamp_ms: u64) -> std::io::Result<[u8; 32]> {
        let hash = self.wal.checkpoint_to_dag(&mut self.dag, self.author_did, timestamp_ms)?;
        self.wal.truncate()?;
        Ok(hash)
    }
    
    /// Converts a wellfare RecordEnvelope into NQuins and commits them.
    pub fn commit_envelope(
        &mut self,
        envelope: &RecordEnvelope,
        signing_key: &SigningKey,
        principal_did_hash: u64,
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
        Ok(committed)
    }
}
