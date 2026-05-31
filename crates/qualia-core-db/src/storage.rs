//! SuperBlock Storage Engine
//! Handles flushing 850 48-byte Quins into perfectly aligned 40,960-byte 
//! QualiaSuperBlock structures onto the NVMe disk format (`.qla`).

use std::fs::{File, OpenOptions};
use std::io::{self, Seek, SeekFrom};
use std::path::Path;
use crate::{QualiaQuin, QualiaSuperBlock, QUINS_PER_BLOCK, BLOCK_MULTIPLIER_SIZE};

/// The Physical I/O Persistence Writer.
pub struct SuperBlockWriter {
    file: File,
    current_offset: u64,
}

impl SuperBlockWriter {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .open(path)?;
            
        let current_offset = file.seek(SeekFrom::End(0))?;
        
        Ok(Self { file, current_offset })
    }

    /// Flushes an array of exact 850 Quins into a hardware-aligned block on disk.
    pub fn flush_block(&mut self, sequence_id: u64, owner_did: u64, quins: &[QualiaQuin; QUINS_PER_BLOCK]) -> io::Result<()> {
        let mut block = Box::new(unsafe { std::mem::zeroed::<QualiaSuperBlock>() });
        
        block.block_sequence_id = sequence_id;
        block.storage_owner_did = owner_did;
        block.active_quin_count = QUINS_PER_BLOCK as u64;
        block.validation_checksum = 0xABCD; // Mock checksum
        block.hardware_profile_flags = 0x01; // Edge device default profile
        
        // Copy the array iteratively to avoid unaligned access panics
        for i in 0..QUINS_PER_BLOCK {
            block.quin_ledger[i] = quins[i];
        }

        // Convert the aligned struct directly into a byte slice
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (block.as_ref() as *const QualiaSuperBlock) as *const u8,
                BLOCK_MULTIPLIER_SIZE
            )
        };

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::FileExt;
            self.file.write_all_at(bytes, self.current_offset)?;
        }

        #[cfg(target_family = "windows")]
        {
            use std::os::windows::fs::FileExt;
            let mut written = 0;
            while written < BLOCK_MULTIPLIER_SIZE {
                let n = self.file.seek_write(&bytes[written..], self.current_offset + written as u64)?;
                if n == 0 {
                    return Err(io::Error::new(io::ErrorKind::WriteZero, "Failed to write whole block"));
                }
                written += n;
            }
        }
        
        // Sync to guarantee physical sector write
        self.file.sync_data()?;
        self.current_offset += BLOCK_MULTIPLIER_SIZE as u64;
        
        crate::telemetry::SUPERBLOCK_IO_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_flush_superblock() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut writer = SuperBlockWriter::open(temp_file.path()).unwrap();
        
        // Create an array of exactly 850 mock quins
        let quins = [QualiaQuin { subject: 0, predicate: 0, object: 0, context: 0, metadata: 0, parity: 0 }; QUINS_PER_BLOCK];
        
        let result = writer.flush_block(1, 42, &quins);
        assert!(result.is_ok(), "Failed to flush SuperBlock");
        
        // Verify the file size is exactly 40,960 bytes
        let metadata = temp_file.as_file().metadata().unwrap();
        assert_eq!(metadata.len(), BLOCK_MULTIPLIER_SIZE as u64, "File size is not page aligned to 40,960 bytes");
    }
}
