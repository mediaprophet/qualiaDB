//! Sort-merge join on `u64` keys. GPU radix on large sides, CPU merge always
//! (merge is sequential). Fail-closed: if `out` fills, `truncated` is true.

use super::cpu::{merge_sorted_pairs, radix_sort_u64_indices, sort_merge_join_u64};
use super::path::{AccelPath, AccelPolicy, GPU_JOIN_MIN};
use super::sort::sort_u64_indices;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JoinOutcome {
    pub path: AccelPath,
    pub written: usize,
    pub truncated: bool,
}

/// Join `probe` to `build` on equal keys. Writes `(probe_idx, build_idx)`.
pub fn hash_join_u64(build: &[u64], probe: &[u64], out: &mut [(u32, u32)]) -> JoinOutcome {
    if build.is_empty() || probe.is_empty() || out.is_empty() {
        return JoinOutcome {
            path: AccelPath::Cpu,
            written: 0,
            truncated: false,
        };
    }
    let policy = AccelPolicy::from_env();
    let large = build.len() >= GPU_JOIN_MIN && probe.len() >= GPU_JOIN_MIN;
    if policy != AccelPolicy::CpuOnly && large {
        let mut b_idx: Vec<u32> = (0..build.len() as u32).collect();
        let mut p_idx: Vec<u32> = (0..probe.len() as u32).collect();
        let sb = sort_u64_indices(build, &mut b_idx);
        let sp = sort_u64_indices(probe, &mut p_idx);
        let written = merge_sorted_pairs(build, probe, &b_idx, &p_idx, out);
        let path = if sb.path == AccelPath::Gpu || sp.path == AccelPath::Gpu {
            AccelPath::Gpu
        } else {
            AccelPath::Cpu
        };
        return JoinOutcome {
            path,
            written,
            // Full output buffer: more pairs may exist. Fail closed — caller enlarges.
            truncated: written == out.len(),
        };
    }
    let written = sort_merge_join_u64(build, probe, out);
    JoinOutcome {
        path: AccelPath::Cpu,
        written,
        truncated: written == out.len(),
    }
}

/// CPU-only oracle (tests).
pub fn hash_join_u64_cpu(build: &[u64], probe: &[u64], out: &mut [(u32, u32)]) -> usize {
    let mut b_idx: Vec<u32> = (0..build.len() as u32).collect();
    let mut p_idx: Vec<u32> = (0..probe.len() as u32).collect();
    radix_sort_u64_indices(build, &mut b_idx);
    radix_sort_u64_indices(probe, &mut p_idx);
    merge_sorted_pairs(build, probe, &b_idx, &p_idx, out)
}
