//! Windows host-memory sampling and low-memory notification for inference.
//!
//! This is deliberately a small FFI boundary.  `GlobalMemoryStatusEx` supplies
//! physical headroom, `GetProcessMemoryInfo` supplies this process's private
//! commit, and the kernel low-memory notification supplies an asynchronous
//! early-warning signal.  None of these APIs changes a working set or claims
//! immunity from allocation failure.

use super::memory_guard::{HostMemorySample, MemoryPressureAction, MemoryPressureGuard};
use super::scheduler::RequestScheduler;

const LOW_MEMORY_RESOURCE_NOTIFICATION: u32 = 0;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 0x0000_0102;

#[repr(C)]
#[derive(Clone, Copy)]
struct MemoryStatusEx {
    length: u32,
    memory_load: u32,
    total_phys: u64,
    avail_phys: u64,
    total_page_file: u64,
    avail_page_file: u64,
    total_virtual: u64,
    avail_virtual: u64,
    avail_extended_virtual: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ProcessMemoryCountersEx {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
    private_usage: usize,
}

#[link(name = "kernel32")]
extern "system" {
    fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    fn GetCurrentProcess() -> isize;
    fn CreateMemoryResourceNotification(notification_type: u32) -> isize;
    fn WaitForSingleObject(handle: isize, milliseconds: u32) -> u32;
    fn CloseHandle(handle: isize) -> i32;
}

#[link(name = "psapi")]
extern "system" {
    fn GetProcessMemoryInfo(process: isize, counters: *mut ProcessMemoryCountersEx, cb: u32)
        -> i32;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsMemoryError {
    SystemSampleUnavailable,
    ProcessSampleUnavailable,
    NotificationUnavailable,
}

impl core::fmt::Display for WindowsMemoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SystemSampleUnavailable => write!(f, "GlobalMemoryStatusEx failed"),
            Self::ProcessSampleUnavailable => write!(f, "GetProcessMemoryInfo failed"),
            Self::NotificationUnavailable => write!(f, "CreateMemoryResourceNotification failed"),
        }
    }
}

impl std::error::Error for WindowsMemoryError {}

/// Kernel-backed notification handle.  Poll it at a scheduler token boundary;
/// the wait has zero timeout and never blocks a decode worker.
pub struct WindowsLowMemoryNotification {
    handle: isize,
}

impl WindowsLowMemoryNotification {
    pub fn create() -> Result<Self, WindowsMemoryError> {
        // SAFETY: `LOW_MEMORY_RESOURCE_NOTIFICATION` is the documented enum
        // value; the returned handle is owned by this RAII wrapper.
        let handle = unsafe { CreateMemoryResourceNotification(LOW_MEMORY_RESOURCE_NOTIFICATION) };
        if handle == 0 || handle == -1 {
            return Err(WindowsMemoryError::NotificationUnavailable);
        }
        Ok(Self { handle })
    }

    /// `true` means Windows has entered its low-memory condition.  Unexpected
    /// wait failures are conservatively treated as signalled so admission does
    /// not continue on an unknown memory state.
    pub fn is_signalled(&self) -> bool {
        // SAFETY: `handle` was returned by CreateMemoryResourceNotification and
        // remains valid until this wrapper is dropped.
        let wait = unsafe { WaitForSingleObject(self.handle, 0) };
        match wait {
            WAIT_TIMEOUT => false,
            WAIT_OBJECT_0 => true,
            _ => true,
        }
    }
}

impl Drop for WindowsLowMemoryNotification {
    fn drop(&mut self) {
        // SAFETY: releasing the handle is the documented ownership action;
        // CloseHandle is idempotent at process termination, and errors cannot
        // be usefully recovered during Drop.
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

/// Read a current-process sample without allocating.  `private_usage` is the
/// Windows private commit counter, not working-set size or mmap address space.
pub fn sample_current_process(
    active_requests: u32,
) -> Result<HostMemorySample, WindowsMemoryError> {
    let mut memory = MemoryStatusEx {
        length: core::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page_file: 0,
        avail_page_file: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended_virtual: 0,
    };
    // SAFETY: points to a valid initialized `MEMORYSTATUSEX` equivalent with
    // its required length field set.
    if unsafe { GlobalMemoryStatusEx(&mut memory) } == 0 {
        return Err(WindowsMemoryError::SystemSampleUnavailable);
    }
    let mut counters = ProcessMemoryCountersEx {
        cb: core::mem::size_of::<ProcessMemoryCountersEx>() as u32,
        page_fault_count: 0,
        peak_working_set_size: 0,
        working_set_size: 0,
        quota_peak_paged_pool_usage: 0,
        quota_paged_pool_usage: 0,
        quota_peak_non_paged_pool_usage: 0,
        quota_non_paged_pool_usage: 0,
        pagefile_usage: 0,
        peak_pagefile_usage: 0,
        private_usage: 0,
    };
    // SAFETY: GetCurrentProcess returns a valid pseudo-handle, and counters is
    // a correctly sized writable `PROCESS_MEMORY_COUNTERS_EX` equivalent.
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            core::mem::size_of::<ProcessMemoryCountersEx>() as u32,
        )
    };
    if ok == 0 {
        return Err(WindowsMemoryError::ProcessSampleUnavailable);
    }
    Ok(HostMemorySample {
        available_host_bytes: memory.avail_phys,
        inference_commit_bytes: counters.private_usage as u64,
        active_requests,
    })
}

/// Feed an actual Windows sample and low-memory notification into the
/// scheduler before a new prefill/decode round.  The low-memory signal is
/// conservatively converted to zero available bytes, which requests drain
/// before the next allocation.  Cache shedding/checkpoint callbacks remain
/// owned by the runtime lifecycle after this returns its action.
pub fn apply_windows_memory_pressure<const REQUESTS: usize>(
    scheduler: &mut RequestScheduler<REQUESTS>,
    guard: &MemoryPressureGuard,
    notification: &WindowsLowMemoryNotification,
    current_epoch: u64,
) -> Result<MemoryPressureAction, WindowsMemoryError> {
    let mut sample = sample_current_process(scheduler.active_count() as u32)?;
    if notification.is_signalled() {
        sample.available_host_bytes = 0;
    }
    Ok(scheduler.apply_memory_pressure(guard, sample, current_epoch))
}
