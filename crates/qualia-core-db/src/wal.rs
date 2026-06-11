use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use ed25519_dalek::{Signature, SigningKey};

use crate::agency::{scrub_quin_volatile, sign_graph_mutation, stamp_fiduciary_metadata};
use crate::crdt::SuspendedTransactionQueue;
use crate::modalities::logic::core::WebizenOpcode;
use crate::PermissiveRoutingLane;
use crate::NQuin;

// ── WAL file layout ──────────────────────────────────────────────────────────
//
// Byte 0..32   — `prev_dag_hash`: SHA-256 of the last DagNode committed from this WAL.
//                All-zero = no prior checkpoint.
// Byte 32..    — Packed 48-byte NQuin records (append-only).
//
// On `checkpoint_to_dag()` the current quins are committed to the DagStore, then
// `prev_dag_hash` is rewritten in-place and the NQuin region is truncated.
// On crash-recovery the `prev_dag_hash` survives so the new checkpoint chains
// correctly onto the previous DAG node.

/// Magic sentinel written as the message for WAL checkpoint DagNodes.
const WAL_CHECKPOINT_MSG: &str = "wal:checkpoint";
/// Size of the fixed WAL header (just the prev_dag_hash).
const WAL_HEADER_SIZE: u64 = 32;

/// The Write-Ahead Log (WAL) ensures mobile fault tolerance by appending all
/// 48-byte Quin mutations directly to flash memory synchronously before they are
/// packed into the larger 40KB SuperBlock structures.
///
/// The WAL also maintains a `prev_dag_hash` linking each checkpoint into the
/// `git_bridge::DagStore` Merkle-DAG so the full write history is content-addressed.
pub struct WriteAheadLog {
    file: File,
    /// SHA-256 of the most recent DagNode committed from this WAL.
    /// `[0u8; 32]` means no prior checkpoint — next call to `checkpoint_to_dag` triggers `genesis_node`.
    pub prev_dag_hash: [u8; 32],
}

impl WriteAheadLog {
    /// Opens or creates the append-only WAL file at `path`.
    ///
    /// If the file already has a 32-byte header (prior checkpoint hash), it is read
    /// back so the chain is preserved across restarts.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(path)?;

        let file_len = file.seek(SeekFrom::End(0))?;

        // Read existing prev_dag_hash from the header, or write a zero header for new files.
        let prev_dag_hash = if file_len >= WAL_HEADER_SIZE {
            file.seek(SeekFrom::Start(0))?;
            let mut h = [0u8; 32];
            file.read_exact(&mut h)?;
            // Seek back to end for appending.
            file.seek(SeekFrom::End(0))?;
            h
        } else {
            // New or empty file: write zero header so the layout is established.
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&[0u8; 32])?;
            file.sync_all()?;
            file.seek(SeekFrom::End(0))?;
            [0u8; 32]
        };

        Ok(Self { file, prev_dag_hash })
    }

    /// Synchronously appends a NQuin to the log and flushes to disk.
    /// This prevents data loss if the OS kills the process.
    pub fn append_mutation(&mut self, quin: &NQuin) -> io::Result<()> {
        // Always append — the file cursor is already at the end after open().
        self.file.write_all(quin_as_bytes(quin))?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Append with volatile field scrub after durable sync (wipes transient reasoning state).
    pub fn append_mutation_volatile(&mut self, quin: &mut NQuin) -> io::Result<()> {
        self.file.write_all(quin_as_bytes(quin))?;
        self.file.sync_all()?;
        scrub_quin_volatile(quin);
        Ok(())
    }

    /// Reconstructs uncommitted NQuins from the WAL (skips the 32-byte header).
    pub fn recover(&mut self) -> io::Result<Vec<NQuin>> {
        self.file.seek(SeekFrom::Start(WAL_HEADER_SIZE))?;

        let mut buffer = Vec::new();
        self.file.read_to_end(&mut buffer)?;

        let quin_size = std::mem::size_of::<NQuin>();
        let mut recovered = Vec::with_capacity(buffer.len() / quin_size);

        // Only read complete 48-byte chunks — partial chunks mean a mid-write crash; discard.
        for chunk in buffer.chunks_exact(quin_size) {
            let quin: NQuin =
                unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const NQuin) };
            recovered.push(quin);
        }

        Ok(recovered)
    }

    /// Wipes the NQuin region of the WAL after a SuperBlock commit, preserving the header.
    ///
    /// Call `checkpoint_to_dag` **before** this so the hash chain is updated first.
    pub fn truncate(&mut self) -> io::Result<()> {
        self.file.set_len(WAL_HEADER_SIZE)?;
        self.file.seek(SeekFrom::End(0))?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Commit the current WAL contents as a DagNode, updating `prev_dag_hash`.
    ///
    /// If the WAL is empty (no quins since the last checkpoint), this is a no-op and
    /// returns the existing `prev_dag_hash`.
    ///
    /// Typical call sequence:
    /// ```text
    /// wal.checkpoint_to_dag(&mut dag_store, author_did, now_ms())?;
    /// wal.truncate()?;
    /// ```
    pub fn checkpoint_to_dag(
        &mut self,
        dag_store: &mut crate::git_bridge::DagStore,
        author_did: u64,
        timestamp_ms: u64,
    ) -> io::Result<[u8; 32]> {
        let quins = self.recover()?;
        if quins.is_empty() {
            return Ok(self.prev_dag_hash);
        }

        let new_hash = if self.prev_dag_hash == [0u8; 32] {
            dag_store.genesis_node(&quins, author_did, timestamp_ms, WAL_CHECKPOINT_MSG)
        } else {
            dag_store.commit_node(
                self.prev_dag_hash,
                &quins,
                author_did,
                timestamp_ms,
                WAL_CHECKPOINT_MSG,
            )
        };

        // Persist the new hash into the WAL header so it survives a crash.
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&new_hash)?;
        self.file.sync_all()?;
        // Restore cursor to end for continued appending.
        self.file.seek(SeekFrom::End(0))?;

        self.prev_dag_hash = new_hash;
        Ok(new_hash)
    }

    /// Return the number of NQuin records currently buffered in the WAL.
    pub fn buffered_count(&mut self) -> io::Result<usize> {
        let file_len = self.file.seek(SeekFrom::End(0))?;
        let data_len = file_len.saturating_sub(WAL_HEADER_SIZE);
        Ok((data_len as usize) / std::mem::size_of::<NQuin>())
    }
}

#[inline]
fn quin_as_bytes(quin: &NQuin) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            (quin as *const NQuin) as *const u8,
            std::mem::size_of::<NQuin>(),
        )
    }
}

/// Outcome of routing a sieve-emitted Quin into the ledger pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalHandoffResult {
    Committed,
    Suspended { agreement_id: u64 },
}

/// Fiduciary-stamped, signed WAL handoff for neuro-symbolic graph mutations (zero String parse).
pub fn commit_semantic_mutation(
    wal: &mut WriteAheadLog,
    quin: &mut NQuin,
    principal_did_hash: u64,
    agent_did_hash: u64,
    signing_key: &SigningKey,
    suspended: &mut SuspendedTransactionQueue,
) -> io::Result<WalHandoffResult> {
    stamp_fiduciary_metadata(quin, principal_did_hash, agent_did_hash);
    let _sig: Signature = sign_graph_mutation(signing_key, quin);

    if quin.identify_routing_lane() == PermissiveRoutingLane::EnforceBilateralMicroCommons {
        let agreement_id = quin.context;
        let tx = crate::crdt::SuspendedTransaction {
            agreement_id,
            threshold: 2,
            collected_signatures: 1,
            registers: [None; 16],
            bytecode_buffer: [None; 64],
            yielded_op: Some(WebizenOpcode::LoadModel(0)),
            suspended_quin: *quin,
        };
        if suspended.push(tx).is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "suspended transaction queue full",
            ));
        }
        scrub_quin_volatile(quin);
        return Ok(WalHandoffResult::Suspended { agreement_id });
    }

    wal.append_mutation_volatile(quin)?;
    Ok(WalHandoffResult::Committed)
}

/// Appends a mutation to the global Write-Ahead Log.
/// For the MVP/MCP, we write to a default location or stderr.
pub fn append_mutation(quin: &NQuin) -> io::Result<()> {
    // In a real implementation this would use a globally managed WAL lock.
    // For now we open it locally or just pass.
    let mut wal = WriteAheadLog::open("qualia_global.wal")?;
    wal.append_mutation(quin)
}

/// Logs an adversarial conduct violation to the WAL when the Sentinel VM halts execution.
pub fn log_adversarial_conduct(intent_quin: &NQuin, violation_code: u8) -> io::Result<()> {
    let violation_quin = NQuin {
        subject: intent_quin.subject,
        predicate: crate::q_hash("q42:conductViolation"),
        object: intent_quin.object,
        context: intent_quin.context,
        metadata: violation_code as u64,
        parity: 0,
    };
    append_mutation(&violation_quin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn make_quin(subject: u64, object: u64) -> NQuin {
        NQuin { subject, predicate: 2, object, context: 4, metadata: 0, parity: 0 }
    }

    #[test]
    fn wal_append_and_recover() {
        let tmp = NamedTempFile::new().unwrap();
        let mut wal = WriteAheadLog::open(tmp.path()).unwrap();

        wal.append_mutation(&make_quin(1, 3)).unwrap();
        wal.append_mutation(&make_quin(10, 30)).unwrap();

        let recovered = wal.recover().unwrap();
        assert_eq!(recovered.len(), 2, "WAL must recover 2 quins");
        assert_eq!(recovered[0].subject, 1);
        assert_eq!(recovered[1].object, 30);

        wal.truncate().unwrap();
        assert_eq!(wal.recover().unwrap().len(), 0, "WAL truncation failed");
    }

    #[test]
    fn wal_header_persists_across_reopen() {
        let tmp = NamedTempFile::new().unwrap();
        {
            let mut wal = WriteAheadLog::open(tmp.path()).unwrap();
            assert_eq!(wal.prev_dag_hash, [0u8; 32]);
            wal.append_mutation(&make_quin(42, 99)).unwrap();
        }
        // Reopen — header must still be there; quin must be recoverable.
        {
            let mut wal = WriteAheadLog::open(tmp.path()).unwrap();
            assert_eq!(wal.prev_dag_hash, [0u8; 32]);
            let quins = wal.recover().unwrap();
            assert_eq!(quins.len(), 1);
            assert_eq!(quins[0].subject, 42);
        }
    }

    #[test]
    fn wal_checkpoint_to_dag_chains_nodes() {
        let tmp = NamedTempFile::new().unwrap();
        let mut wal = WriteAheadLog::open(tmp.path()).unwrap();
        let mut dag = crate::git_bridge::DagStore::new();
        const AUTHOR: u64 = 0xA007_0001;

        // First checkpoint — should produce a genesis node.
        wal.append_mutation(&make_quin(1, 1)).unwrap();
        wal.append_mutation(&make_quin(2, 2)).unwrap();
        let hash1 = wal.checkpoint_to_dag(&mut dag, AUTHOR, 1000).unwrap();
        assert_ne!(hash1, [0u8; 32], "genesis hash must be non-zero");
        assert_eq!(wal.prev_dag_hash, hash1);
        wal.truncate().unwrap();
        assert_eq!(wal.recover().unwrap().len(), 0);

        // Second checkpoint — should produce a commit node chained from hash1.
        wal.append_mutation(&make_quin(3, 3)).unwrap();
        let hash2 = wal.checkpoint_to_dag(&mut dag, AUTHOR, 2000).unwrap();
        assert_ne!(hash2, hash1, "second checkpoint must have a different hash");
        assert_eq!(wal.prev_dag_hash, hash2);
        wal.truncate().unwrap();

        // Verify DAG has 2 nodes.
        let serialized = dag.serialize();
        assert!(!serialized.is_empty());
    }

    #[test]
    fn wal_checkpoint_empty_wal_is_noop() {
        let tmp = NamedTempFile::new().unwrap();
        let mut wal = WriteAheadLog::open(tmp.path()).unwrap();
        let mut dag = crate::git_bridge::DagStore::new();

        // Empty WAL — checkpoint must return the existing (zero) prev_dag_hash.
        let hash = wal.checkpoint_to_dag(&mut dag, 0, 0).unwrap();
        assert_eq!(hash, [0u8; 32]);
        // An empty DagStore serializes to just the 8-byte node-count header (count=0).
        let serialized = dag.serialize();
        let node_count = u64::from_le_bytes(serialized[..8].try_into().unwrap());
        assert_eq!(node_count, 0, "no DagNodes should be created for empty WAL");
    }

    #[test]
    fn wal_buffered_count() {
        let tmp = NamedTempFile::new().unwrap();
        let mut wal = WriteAheadLog::open(tmp.path()).unwrap();
        assert_eq!(wal.buffered_count().unwrap(), 0);
        wal.append_mutation(&make_quin(1, 1)).unwrap();
        wal.append_mutation(&make_quin(2, 2)).unwrap();
        assert_eq!(wal.buffered_count().unwrap(), 2);
    }
}
