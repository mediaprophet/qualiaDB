//! CPU floors — always present, never broken. GPU paths must match these results
//! (permutation of equal keys may differ; join pairs are compared as sets).

use crate::NQuin;

/// LSD radix sort of `items` by `key`. Unstable. Exact for all `u64` keys.
pub fn radix_sort_by_key<T, F>(items: &mut [T], mut key: F)
where
    T: Copy,
    F: FnMut(&T) -> u64,
{
    let n = items.len();
    if n < 2 {
        return;
    }
    let keys: Vec<u64> = items.iter().map(|t| key(t)).collect();
    let mut idx: Vec<u32> = (0..n as u32).collect();
    radix_sort_u64_indices(&keys, &mut idx);
    let mut out = Vec::with_capacity(n);
    let mut out_keys = Vec::with_capacity(n);
    for &i in &idx {
        out.push(items[i as usize]);
        out_keys.push(keys[i as usize]);
    }
    items.copy_from_slice(&out);
    let _ = out_keys;
}

/// Sort `indices` so `keys[indices[i]]` is non-decreasing. `indices` must be a
/// permutation of `0..keys.len()` (caller may pass identity).
pub fn radix_sort_u64_indices(keys: &[u64], indices: &mut [u32]) {
    let n = keys.len();
    debug_assert_eq!(n, indices.len());
    if n < 2 {
        return;
    }
    let mut src_idx = indices.to_vec();
    let mut dst_idx = vec![0u32; n];
    let mut src_key: Vec<u64> = src_idx.iter().map(|&i| keys[i as usize]).collect();
    let mut dst_key = vec![0u64; n];
    for shift in (0..64).step_by(8) {
        let mut hist = [0u32; 256];
        for &k in &src_key {
            hist[((k >> shift) & 0xff) as usize] += 1;
        }
        let mut sum = 0u32;
        for bin in &mut hist {
            let c = *bin;
            *bin = sum;
            sum += c;
        }
        for i in 0..n {
            let k = src_key[i];
            let d = ((k >> shift) & 0xff) as usize;
            let pos = hist[d] as usize;
            hist[d] += 1;
            dst_key[pos] = k;
            dst_idx[pos] = src_idx[i];
        }
        std::mem::swap(&mut src_key, &mut dst_key);
        std::mem::swap(&mut src_idx, &mut dst_idx);
    }
    indices.copy_from_slice(&src_idx);
}

pub fn sort_quins_by_object_cpu(quins: &mut [NQuin]) {
    radix_sort_by_key(quins, |q| q.object);
}

/// Compact Quins whose `field` equals `needle`. Returns count written to `out`.
pub fn sieve_eq_cpu(quins: &[NQuin], field: QuinField, needle: u64, out: &mut [NQuin]) -> usize {
    let mut n = 0;
    for q in quins {
        if field.get(q) == needle {
            if n >= out.len() {
                break;
            }
            out[n] = *q;
            n += 1;
        }
    }
    n
}

/// Write matching *indices* (not Quins) into `out`.
pub fn sieve_eq_indices_cpu(
    quins: &[NQuin],
    field: QuinField,
    needle: u64,
    out: &mut [u32],
) -> usize {
    let mut n = 0;
    for (i, q) in quins.iter().enumerate() {
        if field.get(q) == needle {
            if n >= out.len() {
                break;
            }
            out[n] = i as u32;
            n += 1;
        }
    }
    n
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QuinField {
    Subject = 0,
    Predicate = 1,
    Object = 2,
    Context = 3,
    Metadata = 4,
}

impl QuinField {
    #[inline]
    pub fn get(self, q: &NQuin) -> u64 {
        match self {
            Self::Subject => q.subject,
            Self::Predicate => q.predicate,
            Self::Object => q.object,
            Self::Context => q.context,
            Self::Metadata => q.metadata,
        }
    }
}

/// Sort-merge join on `u64` keys. Writes `(probe_idx, build_idx)` pairs.
/// Equal keys emit the cartesian product, fail-closed when `out` fills
/// (returns the count written; caller sees truncation if count == out.len()
/// and more matches exist — use a larger buffer).
pub fn sort_merge_join_u64(
    build: &[u64],
    probe: &[u64],
    out: &mut [(u32, u32)],
) -> usize {
    if build.is_empty() || probe.is_empty() || out.is_empty() {
        return 0;
    }
    let mut b_idx: Vec<u32> = (0..build.len() as u32).collect();
    let mut p_idx: Vec<u32> = (0..probe.len() as u32).collect();
    radix_sort_u64_indices(build, &mut b_idx);
    radix_sort_u64_indices(probe, &mut p_idx);
    merge_sorted_pairs(build, probe, &b_idx, &p_idx, out)
}

pub fn merge_sorted_pairs(
    build: &[u64],
    probe: &[u64],
    b_idx: &[u32],
    p_idx: &[u32],
    out: &mut [(u32, u32)],
) -> usize {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut n = 0usize;
    while i < b_idx.len() && j < p_idx.len() && n < out.len() {
        let bk = build[b_idx[i] as usize];
        let pk = probe[p_idx[j] as usize];
        match bk.cmp(&pk) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                let mut i2 = i;
                while i2 < b_idx.len() && build[b_idx[i2] as usize] == bk {
                    i2 += 1;
                }
                let mut j2 = j;
                while j2 < p_idx.len() && probe[p_idx[j2] as usize] == pk {
                    j2 += 1;
                }
                for &bi in &b_idx[i..i2] {
                    for &pj in &p_idx[j..j2] {
                        if n >= out.len() {
                            return n;
                        }
                        out[n] = (pj, bi);
                        n += 1;
                    }
                }
                i = i2;
                j = j2;
            }
        }
    }
    n
}
