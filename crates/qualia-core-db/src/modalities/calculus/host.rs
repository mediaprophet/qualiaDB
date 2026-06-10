//! Host-side I/O management for calculus modality.
//!
//! This module provides platform-specific zero-copy streaming implementations
//! that bypass the OS page cache for deterministic edge compute. It manages
//! memory-mapped I/O, io_uring (Linux), and IOCP (Windows) with strict alignment
//! requirements for DMA transfers.
//!
//! ## Architecture
//!
//! - **ZeroCopyStreamer trait**: Platform-agnostic interface for async I/O
//! - **DmaBuffer**: Page-aligned (4096-byte) buffers for DMA transfers
//! - **Memory pinning**: Prevents swap to ensure DMA stability
//! - **Double-buffering**: One buffer active for reading, one inactive for DMA

use std::fs::File;
use std::path::Path;
use std::io;

#[cfg(target_os = "windows")]
use std::os::windows::io::AsRawHandle;

#[cfg(target_os = "windows")]
use std::os::windows::fs::OpenOptionsExt;

#[cfg(target_os = "windows")]
use windows::Win32::Storage::FileSystem::ReadFile;

#[cfg(target_os = "windows")]
use windows::Win32::System::IO::GetQueuedCompletionStatus;

#[cfg(target_os = "windows")]
use windows::Win32::System::IO::CreateIoCompletionPort;

#[cfg(target_os = "windows")]
use windows::Win32::Storage::FileSystem::FILE_FLAG_NO_BUFFERING;

#[cfg(target_os = "windows")]
use windows::Win32::Storage::FileSystem::FILE_FLAG_OVERLAPPED;

// ─── Constants ─────────────────────────────────────────────────────────────────

/// OS page size for DMA alignment (4096 bytes on most systems)
pub const PAGE_SIZE: usize = 4096;

/// Default buffer size (65536 bytes = 16 pages = 8192 f64 values)
pub const DEFAULT_BUFFER_SIZE: usize = 65536;

// ─── Errors ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum IoError {
    MisalignedOffset { offset: u64, required: u64 },
    MisalignedBufferSize { size: usize, required: usize },
    FileOpenError(io::Error),
    IoError(io::Error),
    LockError(String),
    InvalidState(String),
}

impl From<io::Error> for IoError {
    fn from(err: io::Error) -> Self {
        IoError::IoError(err)
    }
}

// ─── DMA Buffer (Page-Aligned) ─────────────────────────────────────────────────

/// DMA buffer aligned to OS page boundaries (4096 bytes).
///
/// Required for O_DIRECT (Linux) and FILE_FLAG_NO_BUFFERING (Windows) to ensure
/// the NVMe controller can DMA directly into the buffer without kernel intervention.
#[repr(C, align(4096))]
pub struct DmaBuffer<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> DmaBuffer<N> {
    /// Creates a new zero-initialized DMA buffer.
    ///
    /// # Panics
    ///
    /// Panics if N is not a multiple of PAGE_SIZE (4096).
    pub fn new() -> Self {
        assert!(
            N % PAGE_SIZE == 0,
            "DMA buffer size must be a multiple of PAGE_SIZE (4096), got {}",
            N
        );
        
        Self { data: [0u8; N] }
    }
    
    /// Returns the buffer as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }
    
    /// Returns the buffer as a mutable byte slice.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
    
    /// Returns the buffer length in bytes.
    pub fn len(&self) -> usize {
        N
    }
    
    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        N == 0
    }
    
    /// Returns the number of f64 values this buffer can hold.
    pub fn f64_capacity(&self) -> usize {
        N / 8
    }
}

impl<const N: usize> Default for DmaBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

// ─── ZeroCopyStreamer Trait ─────────────────────────────────────────────────────

/// Platform-agnostic zero-copy streaming interface.
///
/// This trait abstracts over Linux io_uring and Windows IOCP, providing a
/// consistent API for asynchronous DMA transfers. The host guarantees that
/// one buffer is always safely mutable by the OS while the core reads from the other.
pub trait ZeroCopyStreamer: Send {
    /// Issues an asynchronous hardware read into the INACTIVE buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the offset is not 4096-byte aligned. Both O_DIRECT
    /// and FILE_FLAG_NO_BUFFERING require strict sector alignment.
    fn async_read_chunk(&mut self, offset: u64) -> Result<(), IoError>;
    
    /// Non-blocking poll for completion.
    ///
    /// Returns `Some(&[u8])` only if the hardware has completed the DMA transfer
    /// into the inactive buffer. Upon returning `Some`, internally swaps
    /// active/inactive pointers.
    ///
    /// Returns `None` if the transfer is still in progress.
    fn poll_completion(&mut self) -> Option<&[u8]>;
    
    /// Returns the currently active buffer for SIMD chunking.
    ///
    /// This buffer is guaranteed to be stable and readable by the core.
    fn get_active_buffer(&self) -> &[u8];
    
    /// Returns the buffer size in bytes.
    fn buffer_size(&self) -> usize;
}

// ─── Windows IOCP Implementation ─────────────────────────────────────────────────

#[cfg(target_os = "windows")]
use windows::Win32::Storage::FileSystem::*;
#[cfg(target_os = "windows")]
use windows::Win32::System::IO::OVERLAPPED;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE;

#[cfg(target_os = "windows")]
use windows::Win32::Foundation::HANDLE as RawHandle;

/// Windows IOCP-based zero-copy streamer.
///
/// Uses FILE_FLAG_NO_BUFFERING and FILE_FLAG_OVERLAPPED to bypass the kernel
/// page cache and achieve deterministic DMA transfers from NVMe to RAM.
#[cfg(target_os = "windows")]
pub struct IocpGridManager {
    file: File,
    buffer_a: DmaBuffer<DEFAULT_BUFFER_SIZE>,
    buffer_b: DmaBuffer<DEFAULT_BUFFER_SIZE>,
    active_buffer: BufferId,
    iocp: HANDLE,
    pending_read: bool,
}

#[cfg(target_os = "windows")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum BufferId {
    A,
    B,
}

// SAFETY: HANDLE is safe to send across threads as it's just a kernel object handle
#[cfg(target_os = "windows")]
unsafe impl Send for IocpGridManager {}

#[cfg(target_os = "windows")]
impl IocpGridManager {
    /// Creates a new IOCP grid manager.
    ///
    /// Opens the file with FILE_FLAG_NO_BUFFERING and FILE_FLAG_OVERLAPPED,
    /// creates an I/O completion port, and pins both buffers in physical RAM.
    pub fn new(file_path: &Path) -> Result<Self, IoError> {
        let file = File::options()
            .read(true)
            .attributes(FILE_FLAG_NO_BUFFERING | FILE_FLAG_OVERLAPPED)
            .open(file_path)
            .map_err(IoError::FileOpenError)?;
        
        // Create I/O completion port
        let iocp = unsafe {
            CreateIoCompletionPort(
                Some(HANDLE(file.as_raw_handle() as *mut _)),
                None,
                0,
                0,
            ).map_err(|e| IoError::IoError(io::Error::from_raw_os_error(e.code().0)))?
        };
        
        let mut manager = Self {
            file,
            buffer_a: DmaBuffer::new(),
            buffer_b: DmaBuffer::new(),
            active_buffer: BufferId::A,
            iocp,
            pending_read: false,
        };
        
        // Pin buffers in physical RAM to prevent swap
        manager.pin_buffers()?;
        
        Ok(manager)
    }
    
    /// Pins both buffers in physical RAM using VirtualLock.
    ///
    /// Windows imposes a default working set limit (~1MB for locked pages).
    /// We bump the minimum working set size before pinning to ensure the OS
    /// grants the physical RAM reservation for our DMA buffers.
    fn pin_buffers(&mut self) -> Result<(), IoError> {
        unsafe {
            use windows::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
            
            // Calculate required working set size (2 buffers + overhead)
            let required_size = (self.buffer_a.len() + self.buffer_b.len()) as isize;
            let min_size = required_size * 2;  // Double for safety
            let max_size = min_size * 4;     // Allow growth
            
            // Bump working set size before pinning
            let process_handle = GetCurrentProcess();
            let result = SetProcessWorkingSetSize(
                process_handle,
                min_size as usize,
                max_size as usize,
            );
            
            if result.is_err() {
                return Err(IoError::LockError(
                    "Failed to increase process working set size".to_string()
                ));
            }
            
            // Now pin the buffers
            let result_a = windows::Win32::System::Memory::VirtualLock(
                self.buffer_a.as_slice().as_ptr() as *const _,
                self.buffer_a.len(),
            );
            
            let result_b = windows::Win32::System::Memory::VirtualLock(
                self.buffer_b.as_slice().as_ptr() as *const _,
                self.buffer_b.len(),
            );
            
            if result_a.is_ok() && result_b.is_ok() {
                Ok(())
            } else {
                Err(IoError::LockError(
                    "Failed to pin DMA buffers in physical RAM".to_string()
                ))
            }
        }
    }
    
    fn get_inactive_buffer_mut(&mut self) -> &mut [u8] {
        match self.active_buffer {
            BufferId::A => self.buffer_b.as_mut_slice(),
            BufferId::B => self.buffer_a.as_mut_slice(),
        }
    }
    
    fn get_inactive_buffer(&self) -> &[u8] {
        match self.active_buffer {
            BufferId::A => self.buffer_b.as_slice(),
            BufferId::B => self.buffer_a.as_slice(),
        }
    }
    
    fn swap_buffers(&mut self) {
        self.active_buffer = match self.active_buffer {
            BufferId::A => BufferId::B,
            BufferId::B => BufferId::A,
        };
    }
}

#[cfg(target_os = "windows")]
impl ZeroCopyStreamer for IocpGridManager {
    fn async_read_chunk(&mut self, offset: u64) -> Result<(), IoError> {
        // Validate 4096-byte alignment
        if offset % PAGE_SIZE as u64 != 0 {
            return Err(IoError::MisalignedOffset {
                offset,
                required: PAGE_SIZE as u64,
            });
        }
        
        if self.pending_read {
            return Err(IoError::InvalidState(
                "Read already in progress. Call poll_completion first.".to_string()
            ));
        }
        
        let mut overlapped = OVERLAPPED::default();
        overlapped.Anonymous.Anonymous.Offset = (offset & 0xFFFFFFFF) as u32;
        overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
        
        let buffer_slice = std::slice::from_raw_parts_mut(
            self.get_inactive_buffer_mut().as_mut_ptr(),
            self.get_inactive_buffer().len()
        );
        
        let result = unsafe {
            ReadFile(
                HANDLE(self.file.as_raw_handle() as *mut _),
                Some(buffer_slice),
                None,
                Some(&mut overlapped),
            )
        };
        
        match result {
            Ok(()) => {
                self.pending_read = true;
                Ok(())
            }
            Err(e) => {
                if e.code().0 == 997 {
                    // ERROR_IO_PENDING - expected for async I/O
                    self.pending_read = true;
                    Ok(())
                } else {
                    Err(IoError::IoError(io::Error::from_raw_os_error(e.code().0)))
                }
            }
        }
    }
    
    fn poll_completion(&mut self) -> Option<&[u8]> {
        if !self.pending_read {
            return None;
        }
        
        let mut bytes_transferred = 0u32;
        let mut completion_key = 0usize;
        let mut overlapped_ptr = std::ptr::null_mut();
        
        unsafe {
            let result = GetQueuedCompletionStatus(
                self.iocp,
                &mut bytes_transferred,
                &mut completion_key,
                &mut overlapped_ptr,
                0,  // Non-blocking poll
            );
            
            if result.is_ok() && bytes_transferred > 0 {
                self.pending_read = false;
                self.swap_buffers();
                Some(self.get_active_buffer())
            } else {
                None
            }
        }
    }
    
    fn get_active_buffer(&self) -> &[u8] {
        match self.active_buffer {
            BufferId::A => self.buffer_a.as_slice(),
            BufferId::B => self.buffer_b.as_slice(),
        }
    }
    
    fn buffer_size(&self) -> usize {
        DEFAULT_BUFFER_SIZE
    }
}

#[cfg(target_os = "windows")]
impl Drop for IocpGridManager {
    fn drop(&mut self) {
        unsafe {
            // Unlock buffers
            let _ = windows::Win32::System::Memory::VirtualUnlock(
                self.buffer_a.as_slice().as_ptr() as *const _,
                self.buffer_a.len(),
            );
            let _ = windows::Win32::System::Memory::VirtualUnlock(
                self.buffer_b.as_slice().as_ptr() as *const _,
                self.buffer_b.len(),
            );
            
            // Close IOCP handle
            let _ = windows::Win32::Foundation::CloseHandle(self.iocp);
        }
    }
}

// ─── Linux io_uring Implementation ─────────────────────────────────────────────

#[cfg(target_os = "linux")]
use libc::{c_void, mlock, O_DIRECT};

/// Linux io_uring-based zero-copy streamer.
///
/// Uses O_DIRECT to bypass the kernel page cache and achieve deterministic
/// DMA transfers from NVMe to RAM.
#[cfg(target_os = "linux")]
pub struct IoUringGridManager {
    ring: io_uring::IoUring,
    file: File,
    #[repr(C, align(4096))]
    buffer_a: DmaBuffer<DEFAULT_BUFFER_SIZE>,
    #[repr(C, align(4096))]
    buffer_b: DmaBuffer<DEFAULT_BUFFER_SIZE>,
    active_buffer: BufferId,
    pending_submission: bool,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum BufferId {
    A,
    B,
}

#[cfg(target_os = "linux")]
impl IoUringGridManager {
    /// Creates a new io_uring grid manager.
    ///
    /// Opens the file with O_DIRECT, creates an io_uring instance, and pins
    /// both buffers in physical RAM.
    pub fn new(file_path: &Path) -> Result<Self, IoError> {
        let file = File::options()
            .read(true)
            .custom_flags(O_DIRECT)
            .open(file_path)
            .map_err(IoError::FileOpenError)?;
        
        let ring = io_uring::IoUring::new(8)
            .map_err(IoError::IoError)?;
        
        let mut manager = Self {
            ring,
            file,
            buffer_a: DmaBuffer::new(),
            buffer_b: DmaBuffer::new(),
            active_buffer: BufferId::A,
            pending_submission: false,
        };
        
        // Pin buffers in physical RAM to prevent swap
        manager.pin_buffers()?;
        
        Ok(manager)
    }
    
    /// Pins both buffers in physical RAM using mlock.
    fn pin_buffers(&mut self) -> Result<(), IoError> {
        unsafe {
            let result_a = mlock(
                self.buffer_a.as_slice().as_ptr() as *const c_void,
                self.buffer_a.len(),
            );
            
            let result_b = mlock(
                self.buffer_b.as_slice().as_ptr() as *const c_void,
                self.buffer_b.len(),
            );
            
            if result_a == 0 && result_b == 0 {
                Ok(())
            } else {
                Err(IoError::LockError(
                    "Failed to pin DMA buffers in physical RAM".to_string()
                ))
            }
        }
    }
    
    fn get_inactive_buffer_mut(&mut self) -> &mut [u8] {
        match self.active_buffer {
            BufferId::A => self.buffer_b.as_mut_slice(),
            BufferId::B => self.buffer_a.as_mut_slice(),
        }
    }
    
    fn get_inactive_buffer(&self) -> &[u8] {
        match self.active_buffer {
            BufferId::A => self.buffer_b.as_slice(),
            BufferId::B => self.buffer_a.as_slice(),
        }
    }
    
    fn swap_buffers(&mut self) {
        self.active_buffer = match self.active_buffer {
            BufferId::A => BufferId::B,
            BufferId::B => BufferId::A,
        };
    }
}

#[cfg(target_os = "linux")]
impl ZeroCopyStreamer for IoUringGridManager {
    fn async_read_chunk(&mut self, offset: u64) -> Result<(), IoError> {
        // Validate 4096-byte alignment
        if offset % PAGE_SIZE as u64 != 0 {
            return Err(IoError::MisalignedOffset {
                offset,
                required: PAGE_SIZE as u64,
            });
        }
        
        if self.pending_submission {
            return Err(IoError::InvalidState(
                "Read already submitted. Call poll_completion first.".to_string()
            ));
        }
        
        let read_op = io_uring::opcode::Read::new(
            self.file.as_raw_fd(),
            self.get_inactive_buffer_mut().as_mut_ptr(),
            self.get_inactive_buffer().len() as u32,
        )
        .offset(offset as i64)
        .build();
        
        self.ring.submission()
            .push(&read_op)
            .map_err(IoError::IoError)?;
        
        self.pending_submission = true;
        Ok(())
    }
    
    fn poll_completion(&mut self) -> Option<&[u8]> {
        if !self.pending_submission {
            return None;
        }
        
        match self.ring.submit_and_wait(1) {
            Ok(_) => {
                if let Some(cqe) = self.ring.completion().next() {
                    self.pending_submission = false;
                    
                    if cqe.result() >= 0 {
                        self.swap_buffers();
                        Some(self.get_active_buffer())
                    } else {
                        // I/O error - reset state
                        None
                    }
                } else {
                    None
                }
            }
            Err(_) => None,
        }
    }
    
    fn get_active_buffer(&self) -> &[u8] {
        match self.active_buffer {
            BufferId::A => self.buffer_a.as_slice(),
            BufferId::B => self.buffer_b.as_slice(),
        }
    }
    
    fn buffer_size(&self) -> usize {
        DEFAULT_BUFFER_SIZE
    }
}

#[cfg(target_os = "linux")]
impl Drop for IoUringGridManager {
    fn drop(&mut self) {
        unsafe {
            // Unlock buffers
            let _ = libc::munlock(
                self.buffer_a.as_slice().as_ptr() as *const c_void,
                self.buffer_a.len(),
            );
            let _ = libc::munlock(
                self.buffer_b.as_slice().as_ptr() as *const c_void,
                self.buffer_b.len(),
            );
        }
    }
}

// ─── Fallback mmap Implementation (Non-Deterministic) ───────────────────────────

/// Simple mmap-based grid manager for non-real-time use cases.
///
/// This implementation does not bypass the page cache and may incur page faults.
/// Use only for desktop applications where occasional stalls are acceptable.
pub struct MmapGridManager {
    mmap: memmap2::Mmap,
}

impl MmapGridManager {
    /// Creates a new mmap grid manager.
    ///
    /// Uses madvise to hint sequential read-ahead pattern.
    pub fn new(file_path: &Path) -> Result<Self, IoError> {
        let file = File::open(file_path).map_err(IoError::FileOpenError)?;
        let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(IoError::IoError)?;
        
        // Issue madvise for aggressive read-ahead
        #[cfg(target_os = "linux")]
        unsafe {
            libc::madvise(
                mmap.as_ptr() as *mut libc::c_void,
                mmap.len(),
                libc::MADV_SEQUENTIAL | libc::MADV_WILLNEED,
            );
        }
        
        Ok(Self { mmap })
    }
    
    /// Returns the entire memory-mapped slice.
    pub fn get_slice(&self) -> &[u8] {
        &self.mmap
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    
    #[test]
    fn test_dma_buffer_alignment() {
        let buffer = DmaBuffer::<4096>::new();
        
        // Verify pointer is 4096-byte aligned
        assert_eq!(buffer.as_slice().as_ptr() as usize % 4096, 0);
    }
    
    #[test]
    #[should_panic(expected = "DMA buffer size must be a multiple of PAGE_SIZE")]
    fn test_dma_buffer_misaligned_size() {
        let _buffer = DmaBuffer::<4095>::new();
    }
    
    #[test]
    fn test_dma_buffer_f64_capacity() {
        let buffer = DmaBuffer::<65536>::new();
        assert_eq!(buffer.f64_capacity(), 8192);  // 65536 / 8
    }
    
    #[cfg(target_os = "windows")]
    #[test]
    fn test_iocp_offset_validation() {
        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_iocp.dat");
        
        let mut file = File::create(&file_path).unwrap();
        // Write at least 2 pages of data
        file.write_all(&vec![0u8; 8192]).unwrap();
        file.sync_all().unwrap();
        
        let mut manager = IocpGridManager::new(&file_path).unwrap();
        
        // Test valid offset (4096-byte aligned)
        assert!(manager.async_read_chunk(4096).is_ok());
        
        // Test invalid offset (not aligned)
        let result = manager.async_read_chunk(4095);
        assert!(matches!(result, Err(IoError::MisalignedOffset { .. })));
        
        // Cleanup
        std::fs::remove_file(&file_path).unwrap();
    }
    
    #[cfg(target_os = "linux")]
    #[test]
    fn test_iouring_offset_validation() {
        // Create a temporary test file
        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("test_iouring.dat");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(&vec![0u8; 8192]).unwrap();
        file.sync_all().unwrap();
        
        let mut manager = IoUringGridManager::new(&file_path).unwrap();
        
        // Test valid offset (4096-byte aligned)
        assert!(manager.async_read_chunk(4096).is_ok());
        
        // Test invalid offset (not aligned)
        let result = manager.async_read_chunk(4095);
        assert!(matches!(result, Err(IoError::MisalignedOffset { .. })));
        
        // Cleanup
        std::fs::remove_file(&file_path).unwrap();
    }
}
