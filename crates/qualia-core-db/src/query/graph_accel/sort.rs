//! Sort NQuins / u64 keys: GPU radix when eligible, else CPU radix floor.

use crate::NQuin;

use super::cpu::{radix_sort_by_key, radix_sort_u64_indices, sort_quins_by_object_cpu};
use super::path::{AccelPath, AccelPolicy, GPU_SORT_MIN};

/// Result of a sort: which path ran. Items are mutated in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortOutcome {
    pub path: AccelPath,
    pub n: usize,
}

/// Sort `quins` by `object` (ingest run order). Never fails.
pub fn sort_quins_by_object(quins: &mut [NQuin]) -> SortOutcome {
    let n = quins.len();
    if n < 2 {
        return SortOutcome {
            path: AccelPath::Cpu,
            n,
        };
    }
    let policy = AccelPolicy::from_env();
    if policy != AccelPolicy::CpuOnly && n >= GPU_SORT_MIN {
        if try_gpu_sort_quins(quins) {
            return SortOutcome {
                path: AccelPath::Gpu,
                n,
            };
        }
    }
    sort_quins_by_object_cpu(quins);
    SortOutcome {
        path: AccelPath::Cpu,
        n,
    }
}

/// Sort `indices` so `keys[indices[i]]` is non-decreasing.
pub fn sort_u64_indices(keys: &[u64], indices: &mut [u32]) -> SortOutcome {
    let n = keys.len();
    debug_assert_eq!(n, indices.len());
    if n < 2 {
        return SortOutcome {
            path: AccelPath::Cpu,
            n,
        };
    }
    let policy = AccelPolicy::from_env();
    if policy != AccelPolicy::CpuOnly && n >= GPU_SORT_MIN {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(gpu_idx) = super::gpu::radix_sort_u64_indices_gpu(keys) {
            if gpu_idx.len() == n && permutation_covers(&gpu_idx, n) {
                indices.copy_from_slice(&gpu_idx);
                return SortOutcome {
                    path: AccelPath::Gpu,
                    n,
                };
            }
        }
    }
    if indices.iter().enumerate().any(|(i, &v)| v as usize != i) {
        // caller supplied a permutation — still correct on CPU
    }
    radix_sort_u64_indices(keys, indices);
    SortOutcome {
        path: AccelPath::Cpu,
        n,
    }
}

fn try_gpu_sort_quins(quins: &mut [NQuin]) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let keys: Vec<u64> = quins.iter().map(|q| q.object).collect();
        if let Some(idx) = super::gpu::radix_sort_u64_indices_gpu(&keys) {
            if idx.len() != quins.len() || !permutation_covers(&idx, quins.len()) {
                return false;
            }
            let mut out = Vec::with_capacity(quins.len());
            for &i in &idx {
                out.push(quins[i as usize]);
            }
            quins.copy_from_slice(&out);
            return true;
        }
    }
    let _ = quins;
    false
}

fn permutation_covers(idx: &[u32], n: usize) -> bool {
    if idx.len() != n {
        return false;
    }
    let mut seen = vec![false; n];
    for &i in idx {
        let i = i as usize;
        if i >= n || seen[i] {
            return false;
        }
        seen[i] = true;
    }
    true
}

/// CPU-only radix (tests / oracles).
pub fn sort_quins_by_object_cpu_only(quins: &mut [NQuin]) {
    radix_sort_by_key(quins, |q| q.object);
}
