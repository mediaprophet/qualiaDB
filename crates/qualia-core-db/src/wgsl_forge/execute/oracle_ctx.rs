//! Backend-agnostic context trait for the differential oracle (plan §7).
//!
//! The differential oracle ([`crate::wgsl_forge::oracle`]) verifies a kernel by
//! running it on real hardware and comparing the result against a CPU reference.
//! Plan §7 requires that the oracle "interacts with hardware only via the trait;
//! agnostic to wgpu vs cudarc" — yet the cross-backend kernels (affine, fused-FFN,
//! top-k) previously had a wgpu implementation *and* a parallel `*_cuda` copy that
//! duplicated the allocation/dispatch/compare logic with a concrete
//! [`super::CudaComputeContext`].
//!
//! [`OracleContext`] unifies the two. It is the minimal surface the oracle needs:
//! the four buffer/allocator operations both backends already expose, the three
//! adapter accessors, and a single [`OracleContext::run_kernel`] that hides the
//! backend-specific *compile + dispatch* — WGSL via [`super::WgpuPipeline`] for
//! wgpu, CUDA-C via [`super::CudaPipeline`] for cudarc. With this in place each
//! cross-backend kernel is one generic `fn evaluate_*<C: OracleContext>` instead of
//! two divergent copies.

use crate::wgsl_forge::{AdapterConstraints, AdapterIdentity, ForgeError, KernelSpec, Schedule};

use super::memory::{BindingUsage, BufferView};

/// The hardware surface the differential oracle drives. Implemented by both the
/// wgpu ([`super::WgpuComputeContext`]) and the CUDA ([`super::CudaComputeContext`])
/// backends so each cross-backend kernel evaluator is written once, generically.
///
/// The allocator/read methods mirror the inherent methods both contexts already
/// have; [`Self::run_kernel`] is the one method that abstracts the backend-specific
/// kernel compile + warmup/sample dispatch loop, returning the per-sample timing
/// (nanoseconds).
pub trait OracleContext {
    /// Allocate a transient slab sub-range and upload `data` into it. `usage`
    /// selects the backing slab on wgpu (read-only/uniform vs read-write); the
    /// CUDA backend addresses one slab and ignores it.
    fn allocate_and_write(
        &mut self,
        data: &[u8],
        binding: u32,
        group: u32,
        usage: BindingUsage,
    ) -> Result<BufferView, ForgeError>;

    /// Allocate a transient (uninitialised) slab sub-range. `usage` is honoured by
    /// the wgpu backend and ignored by CUDA, as for [`Self::allocate_and_write`].
    fn allocate_transient(
        &mut self,
        size_bytes: usize,
        binding: u32,
        group: u32,
        usage: BindingUsage,
    ) -> Result<BufferView, ForgeError>;

    /// Read a device buffer back as `f32`s.
    fn read_buffer_f32(&self, view: &BufferView) -> Result<Vec<f32>, ForgeError>;

    /// Release every transient allocation (resets the ring's read head to the
    /// write head). Only valid once all device work on those ranges has completed.
    fn clear_transient_allocations(&mut self);

    /// The adapter identity (vendor/device/driver) backing this context.
    fn adapter(&self) -> &AdapterIdentity;

    /// The adapter's intrinsic capability/limit constraints.
    fn constraints(&self) -> &AdapterConstraints;

    /// Whether GPU-timestamp timing is available (wgpu only; CUDA times on the
    /// host wall clock, so this is always `false` there).
    fn timestamp_supported(&self) -> bool;

    /// Compile `kernel` for *this* backend (WGSL → [`super::WgpuPipeline`] for wgpu;
    /// CUDA-C → [`super::CudaPipeline`] for cudarc), then dispatch it `warmups` times
    /// (untimed) followed by `samples` timed dispatches over `element_count`
    /// elements, returning the per-sample timing in nanoseconds.
    ///
    /// This is the single method that hides the backend-specific compile + dispatch:
    /// the buffers (and their bindings/usages) are provided by the caller, identical
    /// across backends, so only the pipeline construction and the timing source differ.
    fn run_kernel(
        &mut self,
        kernel: &KernelSpec,
        schedule: &Schedule,
        buffers: &[BufferView],
        element_count: usize,
        warmups: usize,
        samples: usize,
    ) -> Result<Vec<u64>, ForgeError>;
}
