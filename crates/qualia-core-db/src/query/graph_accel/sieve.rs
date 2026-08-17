//! Equality sieve + compact. GPU bitmask when eligible, else CPU scan.

use crate::NQuin;

use super::cpu::{sieve_eq_cpu, sieve_eq_indices_cpu, QuinField};
use super::path::{AccelPath, AccelPolicy, GPU_SIEVE_MIN};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SieveOutcome {
    pub path: AccelPath,
    pub written: usize,
}

/// Compact Quins whose `field == needle` into `out`. Returns count + path.
pub fn sieve_eq(
    quins: &[NQuin],
    field: QuinField,
    needle: u64,
    out: &mut [NQuin],
) -> SieveOutcome {
    if quins.is_empty() || out.is_empty() {
        return SieveOutcome {
            path: AccelPath::Cpu,
            written: 0,
        };
    }
    let policy = AccelPolicy::from_env();
    if policy != AccelPolicy::CpuOnly && quins.len() >= GPU_SIEVE_MIN {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(idx) = super::gpu::sieve_eq_indices_gpu(quins, field, needle) {
            let mut n = 0;
            for i in idx {
                if n >= out.len() {
                    break;
                }
                let i = i as usize;
                if i < quins.len() {
                    out[n] = quins[i];
                    n += 1;
                }
            }
            return SieveOutcome {
                path: AccelPath::Gpu,
                written: n,
            };
        }
    }
    let written = sieve_eq_cpu(quins, field, needle, out);
    SieveOutcome {
        path: AccelPath::Cpu,
        written,
    }
}

/// Indices only (caller gathers). Same fallback contract.
pub fn sieve_eq_indices(
    quins: &[NQuin],
    field: QuinField,
    needle: u64,
    out: &mut [u32],
) -> SieveOutcome {
    if quins.is_empty() || out.is_empty() {
        return SieveOutcome {
            path: AccelPath::Cpu,
            written: 0,
        };
    }
    let policy = AccelPolicy::from_env();
    if policy != AccelPolicy::CpuOnly && quins.len() >= GPU_SIEVE_MIN {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(idx) = super::gpu::sieve_eq_indices_gpu(quins, field, needle) {
            let n = idx.len().min(out.len());
            out[..n].copy_from_slice(&idx[..n]);
            return SieveOutcome {
                path: AccelPath::Gpu,
                written: n,
            };
        }
    }
    let written = sieve_eq_indices_cpu(quins, field, needle, out);
    SieveOutcome {
        path: AccelPath::Cpu,
        written,
    }
}
