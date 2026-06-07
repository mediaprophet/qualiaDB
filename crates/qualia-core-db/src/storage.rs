//! SuperBlock Storage Engine
//! Handles flushing 850 48-byte Quins into perfectly aligned 40,960-byte 
//! QualiaSuperBlock structures onto the NVMe disk format (`.qla`).

pub mod mmap;

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

        #[cfg(not(any(target_family = "unix", target_family = "windows")))]
        {
            use std::io::Write;
            self.file.write_all(bytes)?;
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

// -----------------------------------------------------------------------------
// Phase 5: Virtual File System (VFS) Abstraction
// -----------------------------------------------------------------------------
// Provides a unified storage interface for `.q42.bidx` offline index sync.
// - Uses standard `std::fs` on native/Tauri targets.
// - Uses Origin Private File System (OPFS) on `wasm32-unknown-unknown` targets.

use std::future::Future;
use std::pin::Pin;

pub trait VirtualFileSystem {
    /// Reads a chunk of data from the local storage hierarchy
    #[cfg(not(target_arch = "wasm32"))]
    fn read_chunk(&self, path: &str) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send>>;
    #[cfg(target_arch = "wasm32")]
    fn read_chunk(&self, path: &str) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>>>>;
    
    /// Writes a chunk of data to the local storage hierarchy
    #[cfg(not(target_arch = "wasm32"))]
    fn write_chunk(&self, path: &str, data: &[u8]) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;
    #[cfg(target_arch = "wasm32")]
    fn write_chunk(&self, path: &str, data: &[u8]) -> Pin<Box<dyn Future<Output = Result<(), String>>>>;
}

#[cfg(not(target_arch = "wasm32"))]
pub struct NativeVfs;

#[cfg(not(target_arch = "wasm32"))]
impl VirtualFileSystem for NativeVfs {
    fn read_chunk(&self, path: &str) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>> + Send>> {
        let path = path.to_string();
        Box::pin(async move {
            std::fs::read(&path).map_err(|e| e.to_string())
        })
    }

    fn write_chunk(&self, path: &str, data: &[u8]) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> {
        let path = path.to_string();
        let data = data.to_vec();
        Box::pin(async move {
            std::fs::write(&path, data).map_err(|e| e.to_string())
        })
    }
}

#[cfg(target_arch = "wasm32")]
pub struct OpfsVfs;

#[cfg(target_arch = "wasm32")]
impl VirtualFileSystem for OpfsVfs {
    fn read_chunk(&self, path: &str) -> Pin<Box<dyn Future<Output = Result<Vec<u8>, String>>>> {
        let path = path.to_string();
        Box::pin(async move {
            use wasm_bindgen::JsCast;
            use wasm_bindgen_futures::JsFuture;

            let window = web_sys::window().ok_or("No global window")?;
            let navigator = window.navigator();
            let storage = navigator.storage();
            
            let dir_handle_val = JsFuture::from(storage.get_directory()).await.map_err(|e| format!("{:?}", e))?;
            let dir_handle: web_sys::FileSystemDirectoryHandle = dir_handle_val.unchecked_into();
            
            let file_handle_val = JsFuture::from(dir_handle.get_file_handle(&path)).await.map_err(|e| format!("{:?}", e))?;
            let file_handle: web_sys::FileSystemFileHandle = file_handle_val.unchecked_into();
            
            let file_val = JsFuture::from(file_handle.get_file()).await.map_err(|e| format!("{:?}", e))?;
            let file: web_sys::File = file_val.unchecked_into();
            
            let array_buffer_val = JsFuture::from(file.array_buffer()).await.map_err(|e| format!("{:?}", e))?;
            let array_buffer: js_sys::ArrayBuffer = array_buffer_val.unchecked_into();
            
            let uint8_array = js_sys::Uint8Array::new(&array_buffer);
            Ok(uint8_array.to_vec())
        })
    }

    fn write_chunk(&self, path: &str, data: &[u8]) -> Pin<Box<dyn Future<Output = Result<(), String>>>> {
        let path = path.to_string();
        let data = data.to_vec();
        Box::pin(async move {
            use wasm_bindgen::JsCast;
            use wasm_bindgen_futures::JsFuture;

            let window = web_sys::window().ok_or("No global window")?;
            let navigator = window.navigator();
            let storage = navigator.storage();
            
            let dir_handle_val = JsFuture::from(storage.get_directory()).await.map_err(|e| format!("{:?}", e))?;
            let dir_handle: web_sys::FileSystemDirectoryHandle = dir_handle_val.unchecked_into();
            
            let options = web_sys::FileSystemGetFileOptions::new();
            options.set_create(true);
            let file_handle_val = JsFuture::from(dir_handle.get_file_handle_with_options(&path, &options))
                .await.map_err(|e| format!("{:?}", e))?;
            let file_handle: web_sys::FileSystemFileHandle = file_handle_val.unchecked_into();
            
            let writable_val = JsFuture::from(file_handle.create_writable()).await.map_err(|e| format!("{:?}", e))?;
            let writable: web_sys::FileSystemWritableFileStream = writable_val.unchecked_into();
            
            let uint8_array = js_sys::Uint8Array::from(data.as_slice());
            JsFuture::from(writable.write_with_buffer_source(&uint8_array).map_err(|e| format!("{:?}", e))?).await.map_err(|e| format!("{:?}", e))?;
            JsFuture::from(writable.close()).await.map_err(|e| format!("{:?}", e))?;
            
            Ok(())
        })
    }
}
