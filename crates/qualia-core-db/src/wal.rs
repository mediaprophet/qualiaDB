use std::fs::{File, OpenOptions};
use std::io::{self, Write, Read, Seek, SeekFrom};
use std::path::Path;
use crate::QualiaQuin;

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

    /// Synchronously appends a QualiaQuin to the log and flushes to disk.
    /// This prevents data loss if the OS kills the process.
    pub fn append_mutation(&mut self, quin: &QualiaQuin) -> io::Result<()> {
        // Convert the 48-byte Quin into raw bytes safely.
        // Since QualiaQuin is #[repr(C, align(16))] and contains 6 u64s, it is precisely 48 bytes without padding traps.
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (quin as *const QualiaQuin) as *const u8,
                std::mem::size_of::<QualiaQuin>()
            )
        };
        
        self.file.write_all(bytes)?;
        // Flush synchronously ensures the flash memory commits the transaction
        self.file.sync_all()?;
        Ok(())
    }

    /// Reconstructs the uncommitted Quins from the raw WAL file.
    pub fn recover(&mut self) -> io::Result<Vec<QualiaQuin>> {
        self.file.seek(SeekFrom::Start(0))?;
        
        let mut buffer = Vec::new();
        self.file.read_to_end(&mut buffer)?;
        
        let mut recovered_quins = Vec::new();
        let quin_size = std::mem::size_of::<QualiaQuin>();
        
        // Ensure we only read complete 48-byte chunks.
        // Partial chunks mean a power failure occurred mid-write, which we discard or handle via advanced ECC recovery.
        for chunk in buffer.chunks_exact(quin_size) {
            let quin: QualiaQuin = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const QualiaQuin) };
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn qualia_validate_wal_recovery() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut wal = WriteAheadLog::open(temp_file.path()).unwrap();
        
        let q1 = QualiaQuin { subject: 1, predicate: 2, object: 3, context: 4, metadata: 5, parity: 0 };
        let q2 = QualiaQuin { subject: 10, predicate: 20, object: 30, context: 40, metadata: 50, parity: 0 };
        
        wal.append_mutation(&q1).unwrap();
        wal.append_mutation(&q2).unwrap();
        
        let recovered = wal.recover().unwrap();
        assert_eq!(recovered.len(), 2, "WAL failed to recover the correct number of Quins");
        assert_eq!(recovered[0].subject, 1, "WAL corruption on Quin 1");
        assert_eq!(recovered[1].object, 30, "WAL corruption on Quin 2");
        
        wal.truncate().unwrap();
        let recovered_empty = wal.recover().unwrap();
        assert_eq!(recovered_empty.len(), 0, "WAL truncation failed");
    }
}
