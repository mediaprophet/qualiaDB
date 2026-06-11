use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use ed25519_dalek::{Signature, SigningKey};

use crate::agency::{scrub_quin_volatile, sign_graph_mutation, stamp_fiduciary_metadata};
use crate::crdt::SuspendedTransactionQueue;
use crate::modalities::logic::core::WebizenOpcode;
use crate::PermissiveRoutingLane;
use crate::NQuin;

/// The Write-Ahead Log (WAL) ensures mobile fault tolerance by appending all
/// 48-byte Quin mutations directly to flash memory synchronously before they are
/// packed into the larger 40KB SuperBlock structures.
pub struct WriteAheadLog {
    file: File,
}

impl WriteAheadLog {
    /// Opens or creates the append-only WAL file at the target path.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(path)?;
        // Manually seek to the end to act as an append-only log, without locking Windows out of truncate capabilities
        file.seek(SeekFrom::End(0))?;
        Ok(Self { file })
    }

    /// Synchronously appends a NQuin to the log and flushes to disk.
    /// This prevents data loss if the OS kills the process.
    pub fn append_mutation(&mut self, quin: &NQuin) -> io::Result<()> {
        let bytes = quin_as_bytes(quin);
        self.file.write_all(bytes)?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Append with volatile field scrub after durable sync (wipes transient reasoning state).
    pub fn append_mutation_volatile(&mut self, quin: &mut NQuin) -> io::Result<()> {
        let bytes = quin_as_bytes(quin);
        self.file.write_all(bytes)?;
        self.file.sync_all()?;
        scrub_quin_volatile(quin);
        Ok(())
    }

    /// Reconstructs the uncommitted Quins from the raw WAL file.
    pub fn recover(&mut self) -> io::Result<Vec<NQuin>> {
        self.file.seek(SeekFrom::Start(0))?;

        let mut buffer = Vec::new();
        self.file.read_to_end(&mut buffer)?;

        let mut recovered_quins = Vec::new();
        let quin_size = std::mem::size_of::<NQuin>();

        // Ensure we only read complete 48-byte chunks.
        // Partial chunks mean a power failure occurred mid-write, which we discard or handle via advanced ECC recovery.
        for chunk in buffer.chunks_exact(quin_size) {
            let quin: NQuin =
                unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const NQuin) };
            recovered_quins.push(quin);
        }

        Ok(recovered_quins)
    }

    /// Wipes the WAL after the main 40KB SuperBlock successfully commits to main storage.
    pub fn truncate(&mut self) -> io::Result<()> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn qualia_validate_wal_recovery() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut wal = WriteAheadLog::open(temp_file.path()).unwrap();

        let q1 = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 4,
            metadata: 5,
            parity: 0,
        };
        let q2 = NQuin {
            subject: 10,
            predicate: 20,
            object: 30,
            context: 40,
            metadata: 50,
            parity: 0,
        };

        wal.append_mutation(&q1).unwrap();
        wal.append_mutation(&q2).unwrap();

        let recovered = wal.recover().unwrap();
        assert_eq!(
            recovered.len(),
            2,
            "WAL failed to recover the correct number of Quins"
        );
        assert_eq!(recovered[0].subject, 1, "WAL corruption on Quin 1");
        assert_eq!(recovered[1].object, 30, "WAL corruption on Quin 2");

        wal.truncate().unwrap();
        let recovered_empty = wal.recover().unwrap();
        assert_eq!(recovered_empty.len(), 0, "WAL truncation failed");
    }
}
