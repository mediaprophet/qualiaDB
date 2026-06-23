//! STELLAR §A A1a — GPU top-K reduction: CPU oracle, host merge, and the WGSL kernel.
//!
//! Decode is memory-bandwidth-bound; the sentinel/sampler only needs the high-probability
//! mass, not the 49 k near-zero tail. Instead of reading back the full logit vector
//! (~196 KB/token) and doing a CPU argmax, the GPU reduces each block of the vocabulary to
//! its top-K candidates; the host merges those into the global top-K. The CPU oracle here is
//! the byte-for-byte reference the on-device kernel is verified against (see `topk_gpu.rs`),
//! exactly as `ternary.rs` anchors `ternary_gpu.rs`.
//!
//! Contract (must match `shaders/topk_reduction.wgsl`): NaN → −∞ (never selected); ties broken
//! toward the LOWER token id (deterministic); K=1 == argmax.

/// One top-K entry: a token id and its raw logit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopKItem {
    pub token_id: u32,
    pub logit: f32,
}

/// The block-reduction kernel (auto bind-group layout; entry `topk_block`).
pub const TOPK_REDUCTION_WGSL: &str = include_str!("shaders/topk_reduction.wgsl");

/// Block size each workgroup reduces — must equal `MAX_BLOCK` in the WGSL (`var<workgroup>` cap).
pub const TOPK_BLOCK_SIZE: usize = 1024;

/// Largest K the host paths support (kept generous; the kernel itself is K-agnostic per round).
pub const TOPK_MAX_K: usize = 64;

/// 16-byte `Params` uniform: `n, k, block_size, _pad`.
pub fn topk_params_bytes(n: u32, k: u32, block_size: u32) -> [u8; 16] {
    let mut b = [0u8; 16];
    b[0..4].copy_from_slice(&n.to_le_bytes());
    b[4..8].copy_from_slice(&k.to_le_bytes());
    b[8..12].copy_from_slice(&block_size.to_le_bytes());
    b
}

/// Normalize: NaN → −∞ so it can never win a comparison.
#[inline]
fn clean(x: f32) -> f32 {
    if x.is_nan() {
        f32::NEG_INFINITY
    } else {
        x
    }
}

/// Order two `(id, logit)` candidates: higher logit first, lower id on ties.
#[inline]
fn cmp_desc(a: &(u32, f32), b: &(u32, f32)) -> std::cmp::Ordering {
    b.1.partial_cmp(&a.1)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then(a.0.cmp(&b.0))
}

/// CPU reference top-K over a full logit vector. Drops −∞ (e.g. masked) entries.
pub fn topk_cpu(logits: &[f32], k: usize) -> Vec<TopKItem> {
    let mut v: Vec<(u32, f32)> = logits
        .iter()
        .enumerate()
        .map(|(i, &x)| (i as u32, clean(x)))
        .collect();
    v.sort_by(cmp_desc);
    v.into_iter()
        .filter(|(_, val)| *val > f32::NEG_INFINITY)
        .take(k)
        .map(|(token_id, logit)| TopKItem { token_id, logit })
        .collect()
}

/// Merge per-block GPU candidates (`num_blocks × k` pairs) into the global top-K.
/// Blocks cover disjoint index ranges, so candidate ids are unique. Drops −∞ entries
/// and any id in `masked` (the governance/sieve veto — "a masked token never returned").
pub fn merge_block_candidates(
    cand_val: &[f32],
    cand_idx: &[u32],
    k: usize,
    masked: Option<&dyn Fn(u32) -> bool>,
) -> Vec<TopKItem> {
    let mut v: Vec<(u32, f32)> = cand_val
        .iter()
        .zip(cand_idx.iter())
        .map(|(val, idx)| (*idx, clean(*val)))
        .filter(|(idx, val)| *val > f32::NEG_INFINITY && masked.map(|m| !m(*idx)).unwrap_or(true))
        .collect();
    v.sort_by(cmp_desc);
    v.truncate(k);
    v.into_iter()
        .map(|(token_id, logit)| TopKItem { token_id, logit })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_topk_orders_and_breaks_ties() {
        // Two 5.0s at ids 1 and 4 → lower id (1) wins the tie.
        let logits = [1.0, 5.0, 3.0, 2.0, 5.0, f32::NAN, -1.0];
        let top = topk_cpu(&logits, 3);
        assert_eq!(top[0], TopKItem { token_id: 1, logit: 5.0 });
        assert_eq!(top[1], TopKItem { token_id: 4, logit: 5.0 });
        assert_eq!(top[2], TopKItem { token_id: 2, logit: 3.0 });
    }

    #[test]
    fn cpu_topk_k1_is_argmax() {
        let logits = [0.1, -2.0, 9.9, 9.9, 1.0];
        let top = topk_cpu(&logits, 1);
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].token_id, 2); // first of the tied maxima
    }

    #[test]
    fn merge_drops_masked_and_neg_inf() {
        // Simulate 2 blocks × k=2 candidates.
        let cand_val = [9.0, 7.0, 8.0, f32::NEG_INFINITY];
        let cand_idx = [3u32, 10, 42, 99];
        // Mask id 3 (a governance veto) → next-best survives.
        let masked = |id: u32| id == 3;
        let merged = merge_block_candidates(&cand_val, &cand_idx, 2, Some(&masked));
        assert_eq!(merged[0], TopKItem { token_id: 42, logit: 8.0 });
        assert_eq!(merged[1], TopKItem { token_id: 10, logit: 7.0 });
    }
}
