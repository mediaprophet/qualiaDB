//! Compute bridge — the engine's one shared "which hardware runs this kernel"
//! surface (HARDWARE_BACKEND_AUTOSELECT_PLAN.md; CLAUDE.md §13).
//!
//! Every accelerable STEM function classifies itself into a [`KernelClass`], and at
//! its call site asks [`ComputePolicy::select`] for a [`Plan`] — which backend
//! (over the **open** [`BackendRegistry`]), which precision, how to tile, how to move
//! data — then runs it. No module hardcodes a device; they all defer to the
//! **measured** per-class capability matrix. CPU is always present and never fails.
//!
//! Layers (each its own submodule, plan §11):
//! - [`kernel_class`] — the fixed kernel-shape taxonomy routed per class.
//! - [`backend`] — the open `ProbeableBackend` registry (the expansion point: a new
//!   accelerator is one `register()`, never an edit to the decision tree).
//! - [`reference`] — correct CPU microkernels: the always-present path AND the
//!   correctness reference a GPU/NPU/vendor kernel must match before it may default.
//! - [`matrix`] — the per-class capability matrix + the built-in CPU/wgpu backends.
//! - [`policy`] — `ComputePolicy::select` → `Plan` (wraps `hetero_dispatch`).
//!
//! Native only (it reuses the native wgpu probe + `rayon` CPU path). A WASM/browser
//! build registers a narrower set (WebGPU-or-CPU); that target-conditional registry
//! is a follow-on (plan §4b).
#![cfg(not(target_arch = "wasm32"))]

pub mod backend;
pub mod execute;
pub mod gpu_gemm;
pub mod kernel_class;
pub mod matrix;
pub mod policy;
pub mod reference;

pub use backend::{BackendId, BackendRegistry, DispatchError, KernelPanel, ProbeableBackend};
pub use execute::{accelerated_gemm_f32, shared_policy, RanOn};
pub use gpu_gemm::WgpuGemm;
pub use kernel_class::KernelClass;
pub use matrix::{probe_class_matrix, ClassMatrix, CpuBackend, WgpuBackend};
pub use policy::{ComputePolicy, Plan};

/// The default backend registry: the always-present native CPU path plus the
/// portable wgpu path. Expansion backends (`cuda`, `rocm`, `oneapi`, NPU runtimes)
/// register here behind their Cargo features — one `register()` each, no change to
/// ranking or `select`.
pub fn default_registry() -> BackendRegistry {
    let mut reg = BackendRegistry::new();
    reg.register(Box::new(CpuBackend));
    reg.register(Box::new(WgpuBackend));
    // #[cfg(feature = "cuda")] reg.register(Box::new(CudaBackend));   // plan P7
    // #[cfg(feature = "npu-directml")] reg.register(Box::new(...));   // plan P6
    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_cpu_and_wgpu() {
        let reg = default_registry();
        let ids: Vec<_> = reg.iter().map(|b| b.id()).collect();
        assert!(ids.contains(&BackendId::CPU));
        assert!(ids.contains(&BackendId::WGPU));
        // CPU is always available even when headless.
        assert!(reg
            .iter()
            .any(|b| b.id() == BackendId::CPU && b.available()));
    }
}
