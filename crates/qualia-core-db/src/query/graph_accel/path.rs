//! Which accelerator actually ran. NPU is reserved: today's `npu_ffi` sieve is wgpu,
//! not a distinct integer backend. The live chain is GPU → CPU.

use std::fmt;

/// Best-path that produced a result. Callers never fail because GPU is missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelPath {
    Gpu,
    /// Reserved. `npu_available()` is false until a real NPU integer path exists.
    Npu,
    Cpu,
}

impl AccelPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gpu => "gpu",
            Self::Npu => "npu",
            Self::Cpu => "cpu",
        }
    }
}

impl fmt::Display for AccelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How the dispatcher should choose. `QUALIA_GRAPH_ACCEL=cpu|gpu|auto` overrides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelPolicy {
    Auto,
    GpuPreferred,
    CpuOnly,
}

impl AccelPolicy {
    pub fn from_env() -> Self {
        match std::env::var("QUALIA_GRAPH_ACCEL")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("cpu") => Self::CpuOnly,
            Some("gpu") => Self::GpuPreferred,
            _ => Self::Auto,
        }
    }
}

/// Distinct NPU integer backend (DirectML/Hexagon/CoreML as *compute*, not wgpu).
/// Today's `platform::npu_ffi` dispatches `sieve.wgsl` on wgpu — that is [`AccelPath::Gpu`].
#[inline]
pub fn npu_available() -> bool {
    false
}

/// Prefer GPU when a shared device exists and policy allows it.
///
/// Without native `gpu-runtime` (and on wasm32) this is the CPU floor: the
/// fail-closed [`crate::gpu_context::try_shared_gpu`] stub returns `None`.
/// Never panics.
pub fn gpu_available() -> bool {
    crate::gpu_context::try_shared_gpu().is_some()
}

/// Below this many records, PCIe + dispatch lose to the CPU radix floor.
pub const GPU_SORT_MIN: usize = 65_536;
pub const GPU_SIEVE_MIN: usize = 4_096;
pub const GPU_JOIN_MIN: usize = 16_384;

#[cfg(test)]
mod tests {
    #[test]
    fn gpu_available_never_panics() {
        let _ = super::gpu_available();
    }

    #[cfg(not(all(not(target_arch = "wasm32"), feature = "gpu-runtime")))]
    #[test]
    fn gpu_available_is_cpu_floor_without_gpu_runtime() {
        assert!(!super::gpu_available());
    }
}
