//! The open backend registry (HARDWARE_BACKEND_AUTOSELECT_PLAN.md §2.2, §4).
//!
//! The bridge's expansion point: backends are **not** a closed enum. Each
//! acceleration method (CPU, the wgpu backends, later CUDA / ROCm / oneAPI / NPU
//! runtimes) is one `impl ProbeableBackend` registered into a [`BackendRegistry`].
//! The benchmark loop, the ranking, the passport schema and `ComputePolicy::select`
//! all iterate the registry — so **adding a backend is one `register()` call and
//! never edits the decision tree** (the load-bearing requirement). A backend that
//! is not `available()` on this machine simply contributes no rows.
//!
//! `BackendId` is a `Copy` `&'static str` so it is zero-heap to pass around and
//! string-keyed in the passport (forward-compatible: a passport written by a core
//! build is still readable by an expansion build — the new backend is just absent
//! and gets probed on next boot).

use crate::device_benchmark::CircuitBench;
use super::kernel_class::KernelClass;

/// Stable, string-keyed backend identifier. `Copy`, zero-heap. Built-ins:
/// `"cpu"`, `"wgpu"` (which itself reports per-adapter circuits via wgpu's
/// `Vulkan`/`Dx12`/`Metal`/`Gl`); expansion ids: `"cuda"`, `"rocm"`, `"oneapi"`,
/// `"npu-directml"`, …
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BackendId(pub &'static str);

impl BackendId {
    pub const CPU: BackendId = BackendId("cpu");
    pub const WGPU: BackendId = BackendId("wgpu");

    pub fn as_str(self) -> &'static str {
        self.0
    }
}

impl core::fmt::Display for BackendId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.0)
    }
}

/// Why a dispatch could not run on the requested backend. The dispatcher must
/// degrade to CPU on any of these, never panic (plan §7: CPU never hard-fails).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// The backend exists but cannot run this kernel class.
    UnsupportedClass(KernelClass),
    /// The backend's runtime/SDK is not present on this host.
    Unavailable(BackendId),
    /// A backend-internal failure (driver, allocation, …) — caller falls back to CPU.
    BackendFailure(String),
}

impl core::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DispatchError::UnsupportedClass(c) => write!(f, "backend cannot run kernel class {}", c.label()),
            DispatchError::Unavailable(b) => write!(f, "backend {b} is not available on this host"),
            DispatchError::BackendFailure(m) => write!(f, "backend failure: {m}"),
        }
    }
}
impl std::error::Error for DispatchError {}

/// Per-class problem sizes for the measurement panel. Kept small; cached in the
/// passport so the heavy pass runs once per machine (plan §3 cost guard). `quick()`
/// shrinks the sizes for low-tier devices / fast boot.
#[derive(Debug, Clone, Copy)]
pub struct KernelPanel {
    /// GEMV/GEMM side length (DenseLinear).
    pub dense_n: usize,
    /// Vector length for ElementwiseMap / Reduction / Scan.
    pub vector_len: usize,
    /// Grid length for the 1-D Stencil pass.
    pub grid_n: usize,
    /// Particle count for the AllPairs (N-body) pass.
    pub nbody_n: usize,
    /// Transform length for the FFT (must be a power of two).
    pub fft_n: usize,
    /// Monte-Carlo steps for the Divergent pass.
    pub mc_steps: usize,
}

impl Default for KernelPanel {
    fn default() -> Self {
        Self {
            dense_n: 1024,
            vector_len: 1 << 20,
            grid_n: 1 << 20,
            nbody_n: 2048,
            fft_n: 1 << 16,
            mc_steps: 1 << 20,
        }
    }
}

impl KernelPanel {
    /// Smaller panel for fast boot / Tier-0 devices (plan §3 `--quick`).
    pub fn quick() -> Self {
        Self {
            dense_n: 256,
            vector_len: 1 << 16,
            grid_n: 1 << 16,
            nbody_n: 512,
            fft_n: 1 << 12,
            mc_steps: 1 << 16,
        }
    }
}

/// One acceleration method. Implementors register into a [`BackendRegistry`]; the
/// rest of the bridge only ever sees them through this trait, which is why adding a
/// backend never edits `select()`.
pub trait ProbeableBackend: Send + Sync {
    /// Stable identifier (`"cpu"`, `"wgpu"`, `"cuda"`, …).
    fn id(&self) -> BackendId;

    /// Is this backend's runtime/SDK present and usable on THIS machine? A `false`
    /// backend contributes no rows and is never selected.
    fn available(&self) -> bool;

    /// Measure this backend on one kernel class, returning a row per physical
    /// circuit it can drive (e.g. wgpu returns one row per adapter). Empty when the
    /// backend cannot run the class or is unavailable — recorded honestly as "no
    /// rows," never a fabricated number.
    fn probe_class(&self, class: KernelClass, panel: &KernelPanel) -> Vec<CircuitBench>;
}

/// The registry of acceleration methods. Heap-using and boot-time only (the heavy
/// probe runs once and is cached in the passport — never on a hot path).
#[derive(Default)]
pub struct BackendRegistry {
    backends: Vec<Box<dyn ProbeableBackend>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self { backends: Vec::new() }
    }

    /// Register a backend. Adding one here is the *only* change needed to bring a
    /// new acceleration method into the benchmark, ranking and policy.
    pub fn register(&mut self, backend: Box<dyn ProbeableBackend>) -> &mut Self {
        self.backends.push(backend);
        self
    }

    /// All registered backends (including currently-unavailable ones).
    pub fn iter(&self) -> impl Iterator<Item = &dyn ProbeableBackend> {
        self.backends.iter().map(|b| b.as_ref())
    }

    /// Backends actually usable on this host.
    pub fn available(&self) -> impl Iterator<Item = &dyn ProbeableBackend> {
        self.backends.iter().map(|b| b.as_ref()).filter(|b| b.available())
    }

    pub fn len(&self) -> usize {
        self.backends.len()
    }

    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_benchmark::CircuitKind;

    /// A synthetic backend proving the registry is open: it registers and is iterated
    /// without any change to the registry, panel, matrix, or policy code.
    struct StubBackend {
        id: BackendId,
        up: bool,
    }
    impl ProbeableBackend for StubBackend {
        fn id(&self) -> BackendId {
            self.id
        }
        fn available(&self) -> bool {
            self.up
        }
        fn probe_class(&self, _class: KernelClass, _panel: &KernelPanel) -> Vec<CircuitBench> {
            if !self.up {
                return Vec::new();
            }
            vec![CircuitBench {
                label: self.id.as_str().to_string(),
                kind: CircuitKind::Other,
                backend: self.id.as_str().to_string(),
                ms_per_gemv: 1.0,
                gflops: 1.0,
                upload_gbps: 1.0,
                rel_score: 1.0,
            }]
        }
    }

    #[test]
    fn registry_is_open_and_iterates_members() {
        let mut reg = BackendRegistry::new();
        reg.register(Box::new(StubBackend { id: BackendId("alpha"), up: true }))
            .register(Box::new(StubBackend { id: BackendId("beta"), up: false }));
        assert_eq!(reg.len(), 2);
        // Only the available backend is offered for work.
        let avail: Vec<_> = reg.available().map(|b| b.id()).collect();
        assert_eq!(avail, vec![BackendId("alpha")]);
        // Unavailable backend contributes no rows (honest "not probed").
        let rows: Vec<_> = reg
            .iter()
            .flat_map(|b| b.probe_class(KernelClass::DenseLinear, &KernelPanel::quick()))
            .collect();
        assert_eq!(rows.len(), 1, "only the available backend yields a row");
    }

    #[test]
    fn backend_id_is_copy_and_string_keyed() {
        let a = BackendId::CPU;
        let b = a; // Copy
        assert_eq!(a, b);
        assert_eq!(BackendId::WGPU.as_str(), "wgpu");
    }
}
