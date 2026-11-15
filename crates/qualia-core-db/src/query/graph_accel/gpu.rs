//! wgpu dispatch on the process-wide shared device. Any failure returns `None`
//! so the caller uses the CPU floor. Does not create a second adapter.

#![cfg(feature = "gpu-runtime")]

use std::sync::OnceLock;

use crate::gpu_context::try_shared_gpu;
use crate::NQuin;

use super::cpu::QuinField;
#[allow(unused_imports)]
use super::path::gpu_available;

#[allow(dead_code)]
struct Kernels {
    radix: etalon_nquin::GpuRadixSorter,
    sieve: etalon_nquin::GpuQuinSieve,
}

#[allow(dead_code)]
fn kernels() -> Option<&'static Kernels> {
    static CELL: OnceLock<Kernels> = OnceLock::new();
    if let Some(existing) = CELL.get() {
        return Some(existing);
    }
    let gpu = try_shared_gpu()?;
    let device = &gpu.device;
    let built = Kernels {
        radix: etalon_nquin::GpuRadixSorter::new(device),
        sieve: etalon_nquin::GpuQuinSieve::new(device),
    };
    let _ = CELL.set(built);
    CELL.get()
}

/// GPU LSD radix of `keys` → permutation `indices` (`indices[i]` is original row).
pub fn radix_sort_u64_indices_gpu(keys: &[u64]) -> Option<Vec<u32>> {
    #[cfg(target_arch = "wasm32")]
    {
        // On wasm32, synchronous spin-polling cannot yield to the browser microtask loop to complete map_async.
        // Return None immediately to allow zero-latency synchronous CPU fallback without spinning.
        let _ = keys;
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !gpu_available() {
            return None;
        }
        let n = keys.len();
        if n == 0 {
            return Some(Vec::new());
        }
        let k = kernels()?;
        let gpu = try_shared_gpu()?;
        let device = &gpu.device;
        let queue = &gpu.queue;

        let idx = k.radix.sort_indices(device, queue, keys)?;
        if idx.len() != n {
            return None;
        }
        // Refuse a wrong permutation or unsorted order — caller uses the CPU floor.
        let mut seen = vec![false; n];
        let mut prev = 0u64;
        for (step, &i) in idx.iter().enumerate() {
            let i = i as usize;
            if i >= n || seen[i] {
                return None;
            }
            seen[i] = true;
            let k = keys[i];
            if step > 0 && k < prev {
                return None;
            }
            prev = k;
        }
        Some(idx)
    }
}

/// GPU sieve of equality indices.
pub fn sieve_eq_indices_gpu(quins: &[NQuin], field: QuinField, needle: u64) -> Option<Vec<u32>> {
    #[cfg(target_arch = "wasm32")]
    {
        // On wasm32, return None immediately to allow zero-latency synchronous CPU fallback.
        let _ = (quins, field, needle);
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !gpu_available() || quins.is_empty() {
            return None;
        }
        let etalon_field = match field {
            QuinField::Subject => etalon_nquin::QuinField::Subject,
            QuinField::Predicate => etalon_nquin::QuinField::Predicate,
            QuinField::Object => etalon_nquin::QuinField::Object,
            QuinField::Context => etalon_nquin::QuinField::Context,
            _ => return None,
        };
        let k = kernels()?;
        let gpu = try_shared_gpu()?;
        let device = &gpu.device;
        let queue = &gpu.queue;
        let wire_quins: &[etalon_nquin::NQuinWire] =
            bytemuck::cast_slice::<NQuin, etalon_nquin::NQuinWire>(quins);
        k.sieve
            .sieve_eq_indices(device, queue, wire_quins, etalon_field, needle)
    }
}
