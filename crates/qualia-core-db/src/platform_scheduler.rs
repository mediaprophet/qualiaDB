//! Platform-aware thread QoS and core-affinity binding.
//!
//! Binds the calling thread to the most appropriate processor class for its
//! workload so the OS scheduler makes the right placement decision:
//!
//! | Class              | macOS QoS                  | Linux             | Windows                    |
//! |--------------------|----------------------------|-------------------|----------------------------|
//! | UserInteractive    | QOS_CLASS_USER_INTERACTIVE | P-cores (affinity)| THREAD_PRIORITY_HIGHEST    |
//! | UserInitiated      | QOS_CLASS_USER_INITIATED   | P-cores           | THREAD_PRIORITY_ABOVE_NORMAL|
//! | Default            | QOS_CLASS_DEFAULT          | any               | THREAD_PRIORITY_NORMAL     |
//! | Utility            | QOS_CLASS_UTILITY          | any               | THREAD_PRIORITY_BELOW_NORMAL|
//! | Background         | QOS_CLASS_BACKGROUND       | E-cores (affinity)| THREAD_PRIORITY_IDLE       |
//!
//! **Apple Silicon AMP** (P-cores + E-cores) is the primary target.
//! On Intel/AMD/ARM64 the distinction collapses to thread priority.

#![cfg(not(target_arch = "wasm32"))]

// ──────────────────────────────────────────────────────────────────────────────
// Darwin QoS FFI
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
mod darwin_qos {
    /// QoS class values from <sys/qos.h> (Darwin 18+).
    pub const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21; // 33
    pub const QOS_CLASS_USER_INITIATED:   u32 = 0x19; // 25
    pub const QOS_CLASS_DEFAULT:          u32 = 0x15; // 21
    pub const QOS_CLASS_UTILITY:          u32 = 0x11; // 17
    pub const QOS_CLASS_BACKGROUND:       u32 = 0x09; // 9

    extern "C" {
        /// Set the QoS class of the calling thread.
        /// `relative_priority` must be in `[QOS_MIN_RELATIVE_PRIORITY, 0]`
        /// where `QOS_MIN_RELATIVE_PRIORITY = -15`.
        pub fn pthread_set_qos_class_self_np(
            qos_class: u32,
            relative_priority: libc::c_int,
        ) -> libc::c_int;

        /// Read back the QoS class of any thread (NULL → calling thread).
        pub fn pthread_get_qos_class_np(
            thread: libc::pthread_t,
            qos_class_out: *mut u32,
            relative_priority_out: *mut libc::c_int,
        ) -> libc::c_int;
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosClass {
    /// LLM inference, active SPARQL queries, Phase 8 decode loop.
    /// → Apple P-cores / Windows HIGHEST / Linux P-core affinity.
    UserInteractive,
    /// Active user requests, graph engine hot path.
    UserInitiated,
    /// Default — no preference.
    Default,
    /// Background sync, slow I/O.
    Utility,
    /// WAL flush, Merkle root computation, ambient orchestration.
    /// → Apple E-cores / Windows IDLE / Linux E-core affinity.
    Background,
}

#[derive(Debug)]
pub enum SchedulerError {
    Unsupported(String),
    OsError(i32),
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::Unsupported(m) => write!(f, "unsupported: {m}"),
            SchedulerError::OsError(e)     => write!(f, "OS error {e}"),
        }
    }
}

/// Bind the **calling thread** to `class`.
///
/// Returns `Ok(())` if the platform supports QoS binding, or
/// `Err(SchedulerError::Unsupported)` on platforms that have no thread
/// priority API (rare).
pub fn bind_current_thread(class: QosClass) -> Result<(), SchedulerError> {
    bind_macos(class)
        .or_else(|_| bind_linux(class))
        .or_else(|_| bind_windows(class))
}

/// Convenience wrapper: bind to UserInteractive (P-core / max priority).
/// Call from the LLM inference thread and active SPARQL query thread.
pub fn bind_inference_thread() {
    let _ = bind_current_thread(QosClass::UserInteractive);
}

/// Convenience wrapper: bind to Background (E-core / idle priority).
/// Call from WAL flush, Merkle DAG, ambient orchestration threads.
pub fn bind_background_thread() {
    let _ = bind_current_thread(QosClass::Background);
}

// ──────────────────────────────────────────────────────────────────────────────
// macOS implementation
// ──────────────────────────────────────────────────────────────────────────────

fn bind_macos(class: QosClass) -> Result<(), SchedulerError> {
    #[cfg(target_os = "macos")]
    {
        use darwin_qos::*;
        let qos = match class {
            QosClass::UserInteractive => QOS_CLASS_USER_INTERACTIVE,
            QosClass::UserInitiated   => QOS_CLASS_USER_INITIATED,
            QosClass::Default         => QOS_CLASS_DEFAULT,
            QosClass::Utility         => QOS_CLASS_UTILITY,
            QosClass::Background      => QOS_CLASS_BACKGROUND,
        };
        // SAFETY: pthread_set_qos_class_self_np is safe to call from any thread.
        let rc = unsafe { pthread_set_qos_class_self_np(qos, 0) };
        if rc == 0 {
            return Ok(());
        }
        return Err(SchedulerError::OsError(rc));
    }
    #[cfg(not(target_os = "macos"))]
    Err(SchedulerError::Unsupported("macOS QoS not available on this platform".into()))
}

// ──────────────────────────────────────────────────────────────────────────────
// Linux implementation — core_affinity for asymmetric multiprocessing
// ──────────────────────────────────────────────────────────────────────────────

fn bind_linux(class: QosClass) -> Result<(), SchedulerError> {
    #[cfg(target_os = "linux")]
    {
        use core_affinity::CoreId;

        let all_cores = core_affinity::get_core_ids()
            .ok_or_else(|| SchedulerError::Unsupported("core_affinity unavailable".into()))?;

        if all_cores.is_empty() {
            return Err(SchedulerError::Unsupported("no cores found".into()));
        }

        // Heuristic for big.LITTLE / Alder Lake / Sapphire Rapids:
        // Lower-numbered cores are P-cores; higher-numbered are E-cores.
        // We split at the midpoint as a conservative estimate.
        let mid = all_cores.len() / 2;
        let target = match class {
            QosClass::UserInteractive | QosClass::UserInitiated => {
                // P-cores: first half (or all if symmetric)
                &all_cores[..mid.max(1)]
            }
            QosClass::Background | QosClass::Utility => {
                // E-cores: second half (or all if symmetric)
                &all_cores[mid..]
            }
            QosClass::Default => &all_cores[..],
        };

        // Pin to the first core in the target set; real production code would
        // use `sched_setaffinity` with a full mask, but core_affinity exposes
        // single-core pinning which is sufficient for the inference split.
        if let Some(core) = target.first() {
            if core_affinity::set_for_current(*core) {
                return Ok(());
            }
        }

        // Also set Linux thread nice / scheduling policy.
        let nice_val: libc::c_int = match class {
            QosClass::UserInteractive => -10,
            QosClass::UserInitiated   => -5,
            QosClass::Default         => 0,
            QosClass::Utility         => 5,
            QosClass::Background      => 19,
        };
        // SAFETY: getpid() always succeeds; setpriority is safe with valid args.
        unsafe {
            libc::setpriority(libc::PRIO_PROCESS, 0, nice_val);
        }
        return Ok(());
    }
    #[cfg(not(target_os = "linux"))]
    Err(SchedulerError::Unsupported("Linux sched_setaffinity not available".into()))
}

// ──────────────────────────────────────────────────────────────────────────────
// Windows implementation — SetThreadPriority
// ──────────────────────────────────────────────────────────────────────────────

fn bind_windows(class: QosClass) -> Result<(), SchedulerError> {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        use windows::Win32::System::Threading::{
            GetCurrentThread, SetThreadPriority,
            THREAD_PRIORITY_HIGHEST, THREAD_PRIORITY_ABOVE_NORMAL,
            THREAD_PRIORITY_NORMAL, THREAD_PRIORITY_BELOW_NORMAL,
            THREAD_PRIORITY_IDLE,
        };
        let priority = match class {
            QosClass::UserInteractive => THREAD_PRIORITY_HIGHEST,
            QosClass::UserInitiated   => THREAD_PRIORITY_ABOVE_NORMAL,
            QosClass::Default         => THREAD_PRIORITY_NORMAL,
            QosClass::Utility         => THREAD_PRIORITY_BELOW_NORMAL,
            QosClass::Background      => THREAD_PRIORITY_IDLE,
        };
        // SAFETY: GetCurrentThread() returns a pseudo-handle that is always valid.
        let ok = unsafe { SetThreadPriority(GetCurrentThread(), priority) };
        return if ok.is_ok() {
            Ok(())
        } else {
            Err(SchedulerError::OsError(
                unsafe { windows::Win32::Foundation::GetLastError() }.0 as i32,
            ))
        };
    }
    #[cfg(not(all(target_os = "windows", target_arch = "x86_64")))]
    Err(SchedulerError::Unsupported("Windows SetThreadPriority not available".into()))
}

// ──────────────────────────────────────────────────────────────────────────────
// Query current QoS (macOS only)
// ──────────────────────────────────────────────────────────────────────────────

/// Read the QoS class of the calling thread.  Returns `None` on non-macOS.
pub fn current_qos_class() -> Option<u32> {
    #[cfg(target_os = "macos")]
    {
        let mut cls: u32 = 0;
        let mut rel: libc::c_int = 0;
        // SAFETY: pthread_get_qos_class_np with null thread → calling thread.
        let rc = unsafe {
            darwin_qos::pthread_get_qos_class_np(
                std::ptr::null_mut(),
                &mut cls,
                &mut rel,
            )
        };
        if rc == 0 { Some(cls) } else { None }
    }
    #[cfg(not(target_os = "macos"))]
    None
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_does_not_panic() {
        // On every platform this must complete without panic.
        // It may return Err(Unsupported) on platforms with no QoS API.
        let _ = bind_current_thread(QosClass::UserInteractive);
        let _ = bind_current_thread(QosClass::Background);
    }

    #[test]
    fn test_inference_background_helpers() {
        bind_inference_thread();
        bind_background_thread();
    }

    #[test]
    fn test_current_qos_no_panic() {
        let _ = current_qos_class(); // May return None on non-macOS
    }
}
