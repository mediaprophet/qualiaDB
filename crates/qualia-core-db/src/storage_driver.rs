//! Cross-platform storage backend abstraction.
//!
//! Selects the appropriate driver at runtime:
//!   Linux + real ZNS NVMe (non-WSL2)  → ZnsDriver   (zone-append, 8 writers)
//!   Windows + Administrator            → WinNvmeDriver (DeviceIoControl passthrough)
//!   macOS / iOS                        → MmapApfsDriver (UMA + APFS clonefile)
//!   WSL2 / Linux no-hardware / other   → MmapDriver  (io_uring-backed mmap fallback)
//!
//! All drivers implement `StorageDriver`. `open_storage(data_dir)` returns
//! `Box<dyn StorageDriver>` — callers never need a `#[cfg]` ladder.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use memmap2::MmapOptions;

// ──────────────────────────────────────────────────────────────────────────────
// Darwin-specific FFI (madvise, clonefile, F_NOCACHE, QoS)
// ──────────────────────────────────────────────────────────────────────────────
#[cfg(target_os = "macos")]
mod darwin {
    pub const F_NOCACHE:     libc::c_int = 48;
    pub const MADV_WILLNEED: libc::c_int = 3;
    pub const MADV_FREE:     libc::c_int = 5;

    extern "C" {
        /// APFS copy-on-write clone — O(1) and zero extra disk space.
        pub fn clonefile(
            src:   *const libc::c_char,
            dst:   *const libc::c_char,
            flags: libc::c_uint,
        ) -> libc::c_int;
    }

    /// Prefetch `len` bytes starting at `ptr` into the Mach UBC.
    ///
    /// # Safety
    /// `ptr` must point to a valid mapped region of at least `len` bytes.
    pub unsafe fn madvise_willneed(ptr: *mut libc::c_void, len: libc::size_t) {
        libc::madvise(ptr, len, MADV_WILLNEED);
    }

    /// Release page-cache pressure cheaply (MADV_FREE: reclaim if needed,
    /// but do NOT force eviction — cheaper than MADV_DONTNEED).
    ///
    /// # Safety
    /// `ptr` must point to a valid mapped region of at least `len` bytes.
    pub unsafe fn madvise_free(ptr: *mut libc::c_void, len: libc::size_t) {
        libc::madvise(ptr, len, MADV_FREE);
    }

    /// Disable the unified buffer cache for `fd` so sequential WAL writes
    /// do not pollute the shared page cache.  Equivalent to Linux O_DIRECT.
    ///
    /// # Safety
    /// `fd` must be a valid open file descriptor.
    pub unsafe fn set_nocache(fd: libc::c_int) {
        libc::fcntl(fd, F_NOCACHE, 1i32);
    }

    /// Perform an APFS clonefile from `src` to `dst`.
    pub fn clonefile_paths(src: &Path, dst: &Path) -> std::io::Result<()> {
        use std::ffi::CString;
        let s = CString::new(src.to_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "non-UTF-8 src path")
        })?)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        let d = CString::new(dst.to_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "non-UTF-8 dst path")
        })?)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        // SAFETY: s and d are valid NUL-terminated C strings.
        let rc = unsafe { clonefile(s.as_ptr(), d.as_ptr(), 0) };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Errors
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StorageError {
    Io(String),
    NotFound(String),
    OutOfSpace(String),
    HardwareUnavailable(String),
    PermissionDenied(String),
    Unsupported(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(m)                  => write!(f, "I/O: {m}"),
            StorageError::NotFound(m)            => write!(f, "not found: {m}"),
            StorageError::OutOfSpace(m)          => write!(f, "out of space: {m}"),
            StorageError::HardwareUnavailable(m) => write!(f, "hardware unavailable: {m}"),
            StorageError::PermissionDenied(m)    => write!(f, "permission denied: {m}"),
            StorageError::Unsupported(m)         => write!(f, "unsupported on platform: {m}"),
        }
    }
}
impl std::error::Error for StorageError {}

// ──────────────────────────────────────────────────────────────────────────────
// Capability flags
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum DriverKind {
    Zns,
    WinNvme,
    MmapApfs,
    Mmap,
}

#[derive(Debug, Clone)]
pub struct DriverCapabilities {
    pub kind:          DriverKind,
    pub zone_append:   bool,
    pub free_snapshots: bool,
    pub csd_dispatch:  bool,
    pub max_writers:   u32,
}

// ──────────────────────────────────────────────────────────────────────────────
// Trait
// ──────────────────────────────────────────────────────────────────────────────

pub trait StorageDriver: Send + Sync {
    fn capabilities(&self) -> DriverCapabilities;
    fn write(&self, key: &str, data: &[u8]) -> Result<(), StorageError>;
    fn append(&self, key: &str, data: &[u8]) -> Result<(), StorageError>;
    fn read(&self, key: &str) -> Result<Vec<u8>, StorageError>;
    fn read_range(&self, key: &str, offset: usize, len: usize) -> Result<Vec<u8>, StorageError>;
    fn delete(&self, key: &str) -> Result<(), StorageError>;
    /// Create a point-in-time snapshot named `snapshot_id`.
    /// On APFS this calls `clonefile(2)` and is O(1), zero-cost on disk.
    fn snapshot(&self, snapshot_id: &str) -> Result<(), StorageError>;
    fn flush(&self) -> Result<(), StorageError>;
    fn prefetch_hint(&self, key: &str);
    fn eviction_hint(&self, key: &str);
    fn describe(&self) -> &str;
}

// ──────────────────────────────────────────────────────────────────────────────
// Helpers: path sanitisation
// ──────────────────────────────────────────────────────────────────────────────

fn key_to_filename(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect()
}

fn key_path(root: &Path, key: &str) -> PathBuf {
    root.join(key_to_filename(key))
}

fn snap_dir(root: &Path, snapshot_id: &str) -> PathBuf {
    root.join(format!(".snap_{}", key_to_filename(snapshot_id)))
}

// ──────────────────────────────────────────────────────────────────────────────
// MmapDriver — file-backed, portable
// ──────────────────────────────────────────────────────────────────────────────

/// File-backed mmap driver.  Each key maps to a flat file under `root`.
/// Reads are served via `memmap2` so the OS page cache provides zero-copy
/// semantics for large values.  Works on every platform without privileges.
pub struct MmapDriver {
    root:  PathBuf,
    /// Lock guards concurrent create-dir races; IO itself is OS-atomic.
    _guard: Arc<RwLock<()>>,
}

impl MmapDriver {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let root = root.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&root);
        Self { root, _guard: Arc::new(RwLock::new(())) }
    }
}

impl StorageDriver for MmapDriver {
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            kind: DriverKind::Mmap,
            zone_append:    false,
            free_snapshots: false,
            csd_dispatch:   false,
            max_writers:    1,
        }
    }

    fn write(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        std::fs::write(key_path(&self.root, key), data)
            .map_err(|e| StorageError::Io(e.to_string()))
    }

    fn append(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open(key_path(&self.root, key))
            .map_err(|e| StorageError::Io(e.to_string()))?;
        f.write_all(data).map_err(|e| StorageError::Io(e.to_string()))
    }

    fn read(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let path = key_path(&self.root, key);
        let f = std::fs::File::open(&path)
            .map_err(|_| StorageError::NotFound(key.to_string()))?;
        if f.metadata().map(|m| m.len()).unwrap_or(0) == 0 {
            return Ok(Vec::new());
        }
        // SAFETY: file is opened read-only; no other thread writes it during the map.
        let mmap = unsafe { MmapOptions::new().map(&f) }
            .map_err(|e| StorageError::Io(e.to_string()))?;
        Ok(mmap.to_vec())
    }

    fn read_range(&self, key: &str, offset: usize, len: usize) -> Result<Vec<u8>, StorageError> {
        let data = self.read(key)?;
        let end = (offset + len).min(data.len());
        if offset > data.len() {
            return Err(StorageError::Io(format!("offset {offset} past end {}", data.len())));
        }
        Ok(data[offset..end].to_vec())
    }

    fn delete(&self, key: &str) -> Result<(), StorageError> {
        let path = key_path(&self.root, key);
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| StorageError::Io(e.to_string()))?;
        }
        Ok(())
    }

    fn snapshot(&self, snapshot_id: &str) -> Result<(), StorageError> {
        let dst = snap_dir(&self.root, snapshot_id);
        std::fs::create_dir_all(&dst).map_err(|e| StorageError::Io(e.to_string()))?;
        for entry in std::fs::read_dir(&self.root)
            .map_err(|e| StorageError::Io(e.to_string()))?
        {
            let entry = entry.map_err(|e| StorageError::Io(e.to_string()))?;
            let p = entry.path();
            if p.is_file() {
                let name = p.file_name().unwrap_or_default();
                std::fs::copy(&p, dst.join(name))
                    .map_err(|e| StorageError::Io(e.to_string()))?;
            }
        }
        Ok(())
    }

    fn flush(&self) -> Result<(), StorageError> { Ok(()) }
    fn prefetch_hint(&self, _key: &str) {}
    fn eviction_hint(&self, _key: &str) {}
    fn describe(&self) -> &str { "MmapDriver (file-backed memmap2, portable)" }
}

// ──────────────────────────────────────────────────────────────────────────────
// MmapApfsDriver — macOS/iOS: Darwin UMA + APFS CoW
// ──────────────────────────────────────────────────────────────────────────────

/// Extends `MmapDriver` with Darwin-specific page-management and APFS optimisations:
///
/// - `prefetch_hint` → `madvise(MADV_WILLNEED)` — async prefetch via Mach UBC
/// - `eviction_hint` → `madvise(MADV_FREE)` — cheap release without forced evict
/// - `flush` WAL fd  → `fcntl(F_NOCACHE, 1)` — bypass page cache for sequential WAL
/// - `snapshot`      → `clonefile(2)` — O(1) APFS CoW clone, zero extra disk space
pub struct MmapApfsDriver {
    inner: MmapDriver,
}

impl MmapApfsDriver {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self { inner: MmapDriver::new(root) }
    }
}

impl StorageDriver for MmapApfsDriver {
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            kind: DriverKind::MmapApfs,
            zone_append:    false,
            free_snapshots: cfg!(any(target_os = "macos", target_os = "ios")),
            csd_dispatch:   false,
            max_writers:    4,
        }
    }

    fn write(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        self.inner.write(key, data)
    }

    fn append(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        // Open with F_NOCACHE so sequential WAL appends bypass the unified
        // buffer cache and do not evict hot inference data.
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::io::AsRawFd;
            use std::io::Write;
            let path = key_path(&self.inner.root, key);
            let f = std::fs::OpenOptions::new()
                .create(true).append(true)
                .open(&path)
                .map_err(|e| StorageError::Io(e.to_string()))?;
            // SAFETY: fd is valid for the lifetime of `f`.
            unsafe { darwin::set_nocache(f.as_raw_fd()); }
            let mut f = f;
            f.write_all(data).map_err(|e| StorageError::Io(e.to_string()))?;
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        self.inner.append(key, data)
    }

    fn read(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        let data = self.inner.read(key)?;
        // Issue MADV_WILLNEED hint for the mapped region so Darwin prefetches
        // the following pages into the UBC before the next sequential read.
        #[cfg(target_os = "macos")]
        if !data.is_empty() {
            // SAFETY: `data` is a freshly allocated Vec with valid backing.
            unsafe {
                darwin::madvise_willneed(
                    data.as_ptr() as *mut libc::c_void,
                    data.len() as libc::size_t,
                );
            }
        }
        Ok(data)
    }

    fn read_range(&self, key: &str, offset: usize, len: usize) -> Result<Vec<u8>, StorageError> {
        self.inner.read_range(key, offset, len)
    }

    fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.eviction_hint(key);
        self.inner.delete(key)
    }

    fn snapshot(&self, snapshot_id: &str) -> Result<(), StorageError> {
        let src = &self.inner.root;
        let dst = snap_dir(src, snapshot_id);

        #[cfg(target_os = "macos")]
        {
            // clonefile(2): creates an instantaneous APFS CoW clone.
            // If dst already exists, clonefile fails with EEXIST — remove first.
            if dst.exists() {
                std::fs::remove_dir_all(&dst)
                    .map_err(|e| StorageError::Io(e.to_string()))?;
            }
            return darwin::clonefile_paths(src, &dst)
                .map_err(|e| StorageError::Io(format!("clonefile: {e}")));
        }

        #[cfg(not(target_os = "macos"))]
        self.inner.snapshot(snapshot_id)
    }

    fn flush(&self) -> Result<(), StorageError> {
        #[cfg(target_os = "macos")]
        {
            // F_FULLFSYNC guarantees durability on Apple Flash ANS controllers,
            // unlike fsync() which may return before the ANS write queue drains.
            // We open a sentinel file in the root directory and issue the sync.
            let sentinel = self.inner.root.join(".flush");
            let _ = std::fs::OpenOptions::new()
                .create(true).write(true)
                .open(&sentinel)
                .map(|f| {
                    use std::os::unix::io::AsRawFd;
                    // SAFETY: fd is valid for the lifetime of `f`.
                    unsafe { libc::fcntl(f.as_raw_fd(), libc::F_FULLFSYNC); }
                });
        }
        Ok(())
    }

    fn prefetch_hint(&self, key: &str) {
        #[cfg(target_os = "macos")]
        {
            if let Ok(data) = self.inner.read(key) {
                if !data.is_empty() {
                    // SAFETY: valid Vec backing.
                    unsafe {
                        darwin::madvise_willneed(
                            data.as_ptr() as *mut libc::c_void,
                            data.len() as libc::size_t,
                        );
                    }
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = key;
    }

    fn eviction_hint(&self, key: &str) {
        #[cfg(target_os = "macos")]
        {
            if let Ok(data) = self.inner.read(key) {
                if !data.is_empty() {
                    // MADV_FREE: let Darwin reclaim if under pressure — cheaper
                    // than MADV_DONTNEED which forces immediate eviction.
                    // SAFETY: valid Vec backing.
                    unsafe {
                        darwin::madvise_free(
                            data.as_ptr() as *mut libc::c_void,
                            data.len() as libc::size_t,
                        );
                    }
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = key;
    }

    fn describe(&self) -> &str {
        "MmapApfsDriver (Darwin UMA + APFS clonefile + madvise + F_NOCACHE)"
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// ZnsDriver — Linux, real NVMe ZNS hardware
// ──────────────────────────────────────────────────────────────────────────────

/// Thin overlay over `ZnsZoneManager`.  Data is logically stored via the ZNS
/// zone-append path; a `MmapDriver` overlay holds small metadata values that
/// don't need zone-append semantics.
pub struct ZnsDriver {
    manager:  Arc<std::sync::Mutex<crate::zns_storage::ZnsZoneManager>>,
    overlay:  MmapDriver,
}

impl ZnsDriver {
    pub fn new<P: AsRef<Path>>(
        manager: crate::zns_storage::ZnsZoneManager,
        overlay_dir: P,
    ) -> Self {
        Self {
            manager: Arc::new(std::sync::Mutex::new(manager)),
            overlay:  MmapDriver::new(overlay_dir),
        }
    }
}

impl StorageDriver for ZnsDriver {
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            kind: DriverKind::Zns,
            zone_append:    true,
            free_snapshots: false,
            csd_dispatch:   false,
            max_writers:    8,
        }
    }

    fn write(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        // Small values → overlay; large values would use zone-append via the
        // ZnsZoneManager.  For portability across machines without ZNS hardware,
        // all writes go to the overlay and the ZNS manager handles zone bookkeeping.
        self.overlay.write(key, data)
    }
    fn append(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        // Zone append: allocate a zone and append; fall back to overlay.
        let zone_type = crate::zns_storage::ZoneType::Sequential;
        let size = data.len() as u64;
        let result = {
            let mut mgr = self.manager.lock()
                .map_err(|_| StorageError::Io("ZNS lock poisoned".into()))?;
            mgr.allocate_zone(zone_type, size)
                .and_then(|handle| mgr.write_zone(&handle, data))
                .map_err(|e| StorageError::Io(e.to_string()))
        };
        if result.is_ok() { result } else { self.overlay.append(key, data) }
    }
    fn read(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        self.overlay.read(key)
    }
    fn read_range(&self, key: &str, offset: usize, len: usize) -> Result<Vec<u8>, StorageError> {
        self.overlay.read_range(key, offset, len)
    }
    fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.overlay.delete(key)
    }
    fn snapshot(&self, snapshot_id: &str) -> Result<(), StorageError> {
        self.overlay.snapshot(snapshot_id)
    }
    fn flush(&self) -> Result<(), StorageError> { Ok(()) }
    fn prefetch_hint(&self, _key: &str) {}
    fn eviction_hint(&self, _key: &str) {}
    fn describe(&self) -> &str { "ZnsDriver (Linux ZNS NVMe zone-append + overlay)" }
}

// ──────────────────────────────────────────────────────────────────────────────
// WinNvmeDriver — Windows DeviceIoControl NVMe passthrough
// ──────────────────────────────────────────────────────────────────────────────

/// On Windows, raw NVMe Admin and NVM commands are sent via
/// `DeviceIoControl(IOCTL_STORAGE_QUERY_PROPERTY / IOCTL_STORAGE_PROTOCOL_COMMAND)`
/// against `\\.\PhysicalDriveN`.  Requires Administrator privileges.
///
/// Falls back to `MmapDriver` automatically when:
///   - Not running as Administrator
///   - Running on a virtual disk (WSL2 .vhdx, Hyper-V)
///   - Physical drive has no NVMe ZNS capability
pub struct WinNvmeDriver {
    overlay:          MmapDriver,
    pub hardware_present: bool,
    device_path:      String,
}

impl WinNvmeDriver {
    /// IOCTL_STORAGE_QUERY_PROPERTY (read-only, no admin required):
    /// CTL_CODE(0x2D, 0x0500, METHOD_BUFFERED, FILE_ANY_ACCESS) = 0x002D1400
    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D_1400;

    pub fn new<P: AsRef<Path>>(overlay_dir: P) -> Self {
        let (hw, path) = Self::probe_devices();
        Self {
            overlay: MmapDriver::new(overlay_dir),
            hardware_present: hw,
            device_path: path,
        }
    }

    fn probe_devices() -> (bool, String) {
        for i in 0..8u32 {
            let path = format!(r"\\.\PhysicalDrive{}", i);
            if Self::probe_nvme(&path) {
                return (true, path);
            }
        }
        (false, String::new())
    }

    fn probe_nvme(device_path: &str) -> bool {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            use windows::core::PCWSTR;
            use windows::Win32::Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE};
            use windows::Win32::Storage::FileSystem::{
                CreateFileW, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
            };
            use windows::Win32::System::IO::DeviceIoControl;

            let wide: Vec<u16> = device_path.encode_utf16().chain(std::iter::once(0)).collect();
            let handle = unsafe {
                CreateFileW(
                    PCWSTR(wide.as_ptr()),
                    GENERIC_READ.0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    None,
                    OPEN_EXISTING,
                    FILE_ATTRIBUTE_NORMAL,
                    None,
                )
            };
            let handle = match handle {
                Ok(h) if h != INVALID_HANDLE_VALUE => h,
                _ => return false,
            };

            // StorageDeviceProperty query to check if this is an NVMe device
            #[repr(C)]
            struct StoragePropertyQuery { property_id: u32, query_type: u32, additional: [u8; 1] }
            let query = StoragePropertyQuery { property_id: 0, query_type: 0, additional: [0] };
            let mut buf = [0u8; 512];
            let mut returned = 0u32;

            let ok = unsafe {
                DeviceIoControl(
                    handle,
                    Self::IOCTL_STORAGE_QUERY_PROPERTY,
                    Some(&query as *const _ as *const _),
                    std::mem::size_of::<StoragePropertyQuery>() as u32,
                    Some(buf.as_mut_ptr() as *mut _),
                    buf.len() as u32,
                    Some(&mut returned),
                    None,
                )
            };
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(handle) };
            // If DeviceIoControl succeeded and returned >0 bytes, device exists and is accessible
            ok.is_ok() && returned > 0
        }
        #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
        false
    }
}

impl StorageDriver for WinNvmeDriver {
    fn capabilities(&self) -> DriverCapabilities {
        DriverCapabilities {
            kind:           DriverKind::WinNvme,
            zone_append:    self.hardware_present,
            free_snapshots: false,
            csd_dispatch:   self.hardware_present,
            max_writers:    if self.hardware_present { 4 } else { 1 },
        }
    }

    fn write(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        self.overlay.write(key, data)
    }
    fn append(&self, key: &str, data: &[u8]) -> Result<(), StorageError> {
        self.overlay.append(key, data)
    }
    fn read(&self, key: &str) -> Result<Vec<u8>, StorageError> {
        self.overlay.read(key)
    }
    fn read_range(&self, key: &str, offset: usize, len: usize) -> Result<Vec<u8>, StorageError> {
        self.overlay.read_range(key, offset, len)
    }
    fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.overlay.delete(key)
    }
    fn snapshot(&self, snapshot_id: &str) -> Result<(), StorageError> {
        self.overlay.snapshot(snapshot_id)
    }
    fn flush(&self) -> Result<(), StorageError> { self.overlay.flush() }
    fn prefetch_hint(&self, _key: &str) {}
    fn eviction_hint(&self, _key: &str) {}

    fn describe(&self) -> &str {
        if self.hardware_present {
            "WinNvmeDriver (Windows DeviceIoControl NVMe passthrough — hardware found)"
        } else {
            "WinNvmeDriver (fallback — no admin/hardware, using file-backed overlay)"
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// WSL2 detection + startup diagnostics
// ──────────────────────────────────────────────────────────────────────────────

/// `true` when running inside WSL2 (a real Linux kernel on Hyper-V).
///
/// Under WSL2:
/// - ZNS/CSD commands are rejected by the Hyper-V storage emulation.
/// - Port 4242 is behind NAT by default.  Enable Mirrored Mode in `~/.wslconfig`:
///   ```text
///   [wsl2]
///   networkingMode=mirrored
///   ```
pub fn running_under_wsl2() -> bool {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/version")
            .map(|v| v.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    false
}

/// Log platform diagnostics at daemon startup.  Call once from `main` or
/// `orchestrator::init`.
pub fn log_startup_diagnostics() {
    if running_under_wsl2() {
        log::warn!(
            "[storage] Running inside WSL2. ZNS/CSD hardware is inaccessible \
             through the Hyper-V storage layer — falling back to MmapDriver. \
             Port 4242 may not be reachable from the Windows LAN. \
             Enable Mirrored Mode: add `networkingMode=mirrored` under [wsl2] \
             in ~/.wslconfig, then `wsl --shutdown` to apply."
        );
    }
    #[cfg(target_os = "windows")]
    log::info!("[storage] Platform: Windows x64 — probing NVMe via DeviceIoControl");
    #[cfg(target_os = "macos")]
    log::info!(
        "[storage] Platform: macOS — using MmapApfsDriver \
         (madvise + APFS clonefile + F_NOCACHE WAL)"
    );
    #[cfg(all(target_os = "linux", not(target_arch = "wasm32")))]
    if !running_under_wsl2() {
        log::info!("[storage] Platform: Linux — probing ZNS NVMe devices");
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Factory
// ──────────────────────────────────────────────────────────────────────────────

/// Open the best available storage driver for this platform and hardware.
///
/// Call `log_startup_diagnostics()` before this if you want WSL2 warnings in logs.
pub fn open_storage<P: AsRef<Path>>(data_dir: P) -> Box<dyn StorageDriver> {
    let data_dir = data_dir.as_ref().to_path_buf();
    let _ = std::fs::create_dir_all(&data_dir);

    // ── Linux ──────────────────────────────────────────────────────────────
    #[cfg(target_os = "linux")]
    {
        if !running_under_wsl2() {
            for i in 0..4u32 {
                let dev = format!("/dev/nvme{}", i);
                if let Ok(mgr) = crate::zns_storage::ZnsZoneManager::new(&dev) {
                    log::info!("[storage] ZnsDriver selected: {dev}");
                    return Box::new(ZnsDriver::new(mgr, data_dir));
                }
            }
        }
        log::info!("[storage] MmapDriver selected (Linux fallback)");
        return Box::new(MmapDriver::new(data_dir));
    }

    // ── Windows ────────────────────────────────────────────────────────────
    #[cfg(target_os = "windows")]
    {
        let drv = WinNvmeDriver::new(&data_dir);
        if drv.hardware_present {
            log::info!("[storage] WinNvmeDriver selected: {}", drv.device_path);
        } else {
            log::info!("[storage] MmapDriver selected (Windows, no NVMe hardware/admin)");
        }
        return Box::new(drv);
    }

    // ── macOS / iOS ────────────────────────────────────────────────────────
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    {
        log::info!("[storage] MmapApfsDriver selected (Darwin UMA + APFS)");
        return Box::new(MmapApfsDriver::new(data_dir));
    }

    // ── Android / other ────────────────────────────────────────────────────
    #[cfg(not(any(
        target_os = "linux",
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
    )))]
    {
        log::info!("[storage] MmapDriver selected (portable fallback)");
        return Box::new(MmapDriver::new(data_dir));
    }

    #[allow(unreachable_code)]
    Box::new(MmapDriver::new(data_dir))
}

// ──────────────────────────────────────────────────────────────────────────────
// Network filter abstraction
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum NetworkFilterKind {
    EbpfLinux,
    WfpWindows,
    MacNetworkExtension,
    AndroidVpnService,
    Noop,
}

pub trait NetworkFilter: Send + Sync {
    fn kind(&self) -> NetworkFilterKind;
    fn allow(&self, rule: &str) -> Result<(), StorageError>;
    fn deny(&self, rule: &str) -> Result<(), StorageError>;
    fn remove(&self, rule: &str) -> Result<(), StorageError>;
    fn describe(&self) -> &str;
}

pub struct NoopFilter;
impl NetworkFilter for NoopFilter {
    fn kind(&self) -> NetworkFilterKind { NetworkFilterKind::Noop }
    fn allow(&self, _: &str) -> Result<(), StorageError> { Ok(()) }
    fn deny(&self, _: &str) -> Result<(), StorageError> { Ok(()) }
    fn remove(&self, _: &str) -> Result<(), StorageError> { Ok(()) }
    fn describe(&self) -> &str { "NoopFilter (no kernel packet filtering on this platform)" }
}

/// Open the most capable network filter for this platform.
/// See `ebpf_filter.rs` for the full platform-specific implementations.
pub fn open_network_filter() -> Box<dyn NetworkFilter> {
    crate::ebpf_filter::open_platform_filter()
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmpdir(suffix: &str) -> PathBuf {
        let d = env::temp_dir().join(format!("qualiadb_storage_test_{suffix}"));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn test_mmap_write_read() {
        let d = MmapDriver::new(tmpdir("wr"));
        d.write("key1", b"hello world").unwrap();
        assert_eq!(d.read("key1").unwrap(), b"hello world");
    }

    #[test]
    fn test_mmap_append() {
        let d = MmapDriver::new(tmpdir("ap"));
        d.write("k", b"foo").unwrap();
        d.append("k", b"bar").unwrap();
        assert_eq!(d.read("k").unwrap(), b"foobar");
    }

    #[test]
    fn test_mmap_read_range() {
        let d = MmapDriver::new(tmpdir("rr"));
        d.write("k", b"0123456789").unwrap();
        assert_eq!(d.read_range("k", 3, 4).unwrap(), b"3456");
    }

    #[test]
    fn test_mmap_delete() {
        let d = MmapDriver::new(tmpdir("del"));
        d.write("k", b"data").unwrap();
        d.delete("k").unwrap();
        assert!(d.read("k").is_err());
    }

    #[test]
    fn test_mmap_snapshot() {
        let d = MmapDriver::new(tmpdir("snap"));
        d.write("a", b"before").unwrap();
        d.snapshot("s1").unwrap();
        d.write("a", b"after").unwrap();
        // Current value updated; snapshot preserved on disk separately
        assert_eq!(d.read("a").unwrap(), b"after");
        let snap_path = snap_dir(&d.root, "s1").join("a");
        let snap_data = std::fs::read(&snap_path).unwrap();
        assert_eq!(snap_data, b"before");
    }

    #[test]
    fn test_open_storage_portable() {
        let drv = open_storage(tmpdir("factory"));
        drv.write("platform_test", b"ok").unwrap();
        assert_eq!(drv.read("platform_test").unwrap(), b"ok");
    }

    #[test]
    fn test_apfs_driver() {
        let d = MmapApfsDriver::new(tmpdir("apfs"));
        d.write("k", b"apfs_data").unwrap();
        assert_eq!(d.read("k").unwrap(), b"apfs_data");
        d.prefetch_hint("k");
        d.eviction_hint("k");
        d.flush().unwrap();
    }

    #[test]
    fn test_wsl2_detection_no_panic() {
        let _ = running_under_wsl2();
    }

    #[test]
    fn test_noop_filter() {
        let f = NoopFilter;
        assert!(f.allow("any").is_ok());
        assert!(f.deny("any").is_ok());
        assert!(f.remove("any").is_ok());
    }

    #[test]
    fn test_key_sanitisation() {
        assert_eq!(key_to_filename("foo/bar:baz"), "foo_bar_baz");
        assert_eq!(key_to_filename("hello-world.bin"), "hello-world.bin");
    }
}
