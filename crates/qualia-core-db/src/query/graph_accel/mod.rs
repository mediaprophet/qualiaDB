//! Graph acceleration: GPU-first integer kernels over NQuins / u64 keys, with
//! a CPU radix / scan / sort-merge floor that always works.
//!
//! Chain: **GPU (shared wgpu) → CPU**. NPU is a reserved [`AccelPath`] slot;
//! `platform::npu_ffi` today is wgpu (`sieve.wgsl`), not a third backend.
//!
//! Override: `QUALIA_GRAPH_ACCEL=cpu|gpu|auto` (default auto).

mod cpu;
#[cfg(not(target_arch = "wasm32"))]
mod gpu;
mod join;
mod path;
// Q42 volume files use the native mmap-backed reader. Browser builds work on
// caller-provided Quin frames and must not pull that filesystem surface in.
#[cfg(not(target_arch = "wasm32"))]
mod segment;
mod sieve;
mod sort;

pub use cpu::QuinField;
pub use join::{hash_join_u64, hash_join_u64_cpu, JoinOutcome};
pub use path::{npu_available, AccelPath, AccelPolicy, GPU_JOIN_MIN, GPU_SIEVE_MIN, GPU_SORT_MIN};
#[cfg(not(target_arch = "wasm32"))]
pub use segment::sieve_volume_file;
pub use sieve::{sieve_eq, sieve_eq_indices, SieveOutcome};
pub use sort::{
    sort_quins_by_object, sort_quins_by_object_cpu_only, sort_u64_indices, SortOutcome,
};

#[cfg(test)]
mod tests;
