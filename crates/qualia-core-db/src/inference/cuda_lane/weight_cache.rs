//! Host-side dense weight cache for CUDA tensor-core batch GEMM.
//!
//! Caches densified f32 weight matrices by FNV-1a content fingerprint so
//! repeated layers don't re-upload the same matrix. Allocations are 2 MiB-aligned
//! and backed by transparent huge pages on Linux (`MADV_HUGEPAGE`) to reduce TLB
//! pressure during H2D uploads (Phase 7C).

use std::alloc::Layout;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::wgsl_forge::dispatch::{ensure_cuda_runtime_path, gemm_f32_tc_reduced};

/// Max cached weight matrices (each can be tens of MB).
const MAX_WEIGHT_ENTRIES: usize = 24;
/// Max f32 elements per densified matrix (~192 MiB) — covers 3B FFN (~25M) with headroom.
pub const MAX_DENSE_ELEMS: usize = 48 * 1024 * 1024;

/// Alignment for huge-page-backed host weight buffers (2 MiB).
const HUGE_PAGE_ALIGN: usize = 2 * 1024 * 1024;

/// A 2 MiB-aligned f32 buffer backed by transparent huge pages on Linux.
/// Reduces TLB misses during H2D DMA transfers to the GPU.
struct HugePageF32 {
    ptr: *mut f32,
    len: usize,
    /// Byte capacity (always `len * size_of::<f32>()` rounded up to alignment).
    cap_bytes: usize,
}

// SAFETY: `HugePageF32` is a unique owner; rayon scoped threads borrow it
// mutably via `as_mut_slice` within a guaranteed-over scope. The pointer is
// never shared across threads without external synchronization (behind `Mutex`).
unsafe impl Send for HugePageF32 {}
unsafe impl Sync for HugePageF32 {}

impl HugePageF32 {
    fn new(len: usize) -> Self {
        let byte_len = len * std::mem::size_of::<f32>();
        let cap_bytes = byte_len;
        let layout = Layout::from_size_align(cap_bytes, HUGE_PAGE_ALIGN).expect("huge-page layout");
        let ptr = unsafe { std::alloc::alloc(layout) as *mut f32 };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        // On Linux, advise the kernel to back this region with huge pages.
        #[cfg(target_os = "linux")]
        unsafe {
            let ret = libc::madvise(ptr as *mut libc::c_void, cap_bytes, libc::MADV_HUGEPAGE);
            if ret != 0 {
                log::debug!("huge_page|madvise_failed|errno={ret}");
            }
        }
        Self {
            ptr,
            len,
            cap_bytes,
        }
    }

    fn as_slice(&self) -> &[f32] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl Clone for HugePageF32 {
    fn clone(&self) -> Self {
        let buf = Self::new(self.len);
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr, buf.ptr, self.len);
        }
        buf
    }
}

impl Drop for HugePageF32 {
    fn drop(&mut self) {
        let layout =
            Layout::from_size_align(self.cap_bytes, HUGE_PAGE_ALIGN).expect("huge-page layout");
        unsafe {
            std::alloc::dealloc(self.ptr as *mut u8, layout);
        }
    }
}

#[derive(Clone)]
struct WeightEntry {
    n_in: usize,
    n_out: usize,
    /// Dense f32 row-major [n_out × n_in] (GGML convention: rows = out).
    data: HugePageF32,
    last_use: u64,
}

struct WeightCache {
    entries: HashMap<u64, WeightEntry>,
    clock: u64,
}

fn cache() -> &'static Mutex<WeightCache> {
    static C: OnceLock<Mutex<WeightCache>> = OnceLock::new();
    C.get_or_init(|| {
        Mutex::new(WeightCache {
            entries: HashMap::new(),
            clock: 0,
        })
    })
}

/// FNV-1a over weight bytes + dims for cache key.
pub fn weight_fingerprint(raw: &[u8], n_in: usize, n_out: usize) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in raw {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h ^= n_in as u64;
    h = h.wrapping_mul(0x0100_0000_01b3);
    h ^= n_out as u64;
    h.wrapping_mul(0x0100_0000_01b3)
}

/// True if a dense weight for `key` is already in the host TC cache.
pub fn dense_weight_cached(key: u64) -> bool {
    cache()
        .lock()
        .ok()
        .map(|g| g.entries.contains_key(&key))
        .unwrap_or(false)
}

/// Insert or refresh a dense weight matrix (row-major n_out × n_in).
pub fn cache_dense_weight(key: u64, n_in: usize, n_out: usize, data: Vec<f32>) {
    if data.len() != n_in.saturating_mul(n_out) {
        return;
    }
    let len = data.len();
    let buf = HugePageF32::new(len);
    unsafe {
        std::ptr::copy_nonoverlapping(data.as_ptr(), buf.ptr, len);
    }
    let Ok(mut g) = cache().lock() else {
        return;
    };
    g.clock = g.clock.wrapping_add(1);
    if g.entries.len() >= MAX_WEIGHT_ENTRIES && !g.entries.contains_key(&key) {
        // Evict oldest.
        if let Some(old_k) = g
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_use)
            .map(|(k, _)| *k)
        {
            g.entries.remove(&old_k);
            log::info!("cuda_lane|weight_evict|key={old_k:#x}");
        }
    }
    let clock = g.clock;
    g.entries.insert(
        key,
        WeightEntry {
            n_in,
            n_out,
            data: buf,
            last_use: clock,
        },
    );
    let n_ent = g.entries.len();
    log::debug!("cuda_lane|weight_cache|key={key:#x}|n_in={n_in}|n_out={n_out}|entries={n_ent}");
}

/// Allocate a 2 MiB-aligned buffer and fill it via `fill` — skips the
/// intermediate `Vec<f32>` that [`cache_dense_weight`] requires. The closure
/// receives a row-major `n_out × n_in` slice to write into directly.
pub fn cache_dense_weight_direct<F>(key: u64, n_in: usize, n_out: usize, fill: F) -> bool
where
    F: FnOnce(&mut [f32]) -> bool,
{
    let total = n_in.saturating_mul(n_out);
    if total == 0 || total > MAX_DENSE_ELEMS {
        return false;
    }
    let mut buf = HugePageF32::new(total);
    let ok = fill(buf.as_mut_slice());
    if !ok {
        return false;
    }
    let Ok(mut g) = cache().lock() else {
        return false;
    };
    g.clock = g.clock.wrapping_add(1);
    if g.entries.len() >= MAX_WEIGHT_ENTRIES && !g.entries.contains_key(&key) {
        if let Some(old_k) = g
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_use)
            .map(|(k, _)| *k)
        {
            g.entries.remove(&old_k);
            log::info!("cuda_lane|weight_evict|key={old_k:#x}");
        }
    }
    let clock = g.clock;
    g.entries.insert(
        key,
        WeightEntry {
            n_in,
            n_out,
            data: buf,
            last_use: clock,
        },
    );
    let n_ent = g.entries.len();
    log::debug!(
        "cuda_lane|weight_cache_direct|key={key:#x}|n_in={n_in}|n_out={n_out}|entries={n_ent}"
    );
    true
}

fn pad16(x: usize) -> usize {
    ((x + 15) / 16) * 16
}

/// Batch dense GEMM: for each of `batch` rows of hidden (n_in), compute out (n_out).
///
/// `hidden` is packed `batch × n_in` contiguous. `weight` is row-major `n_out × n_in`
/// (same layout as GGML / our GEMV rows). Writes `batch × n_out` into `out`.
///
/// Pads batch/n_in/n_out to multiples of 16 for WMMA; trims result.
/// Returns false if mode/caps/dims ineligible or TC fails (caller keeps wgpu path).
pub fn try_cuda_batch_gemv(
    hidden: &[f32],
    batch: usize,
    n_in: usize,
    n_out: usize,
    weight: &[f32],
    out: &mut [f32],
) -> bool {
    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    if batch == 0 || n_in == 0 || n_out == 0 {
        return false;
    }
    if hidden.len() < batch * n_in || weight.len() < n_out * n_in || out.len() < batch * n_out {
        return false;
    }
    ensure_cuda_runtime_path();

    // C[m×n] = A[m×k] · B[k×n]  with m=batch, k=n_in, n=n_out
    // weight is n_out × n_in rows → need B as k×n = n_in × n_out (transpose of weight rows).
    let m = pad16(batch);
    let k = pad16(n_in);
    let n = pad16(n_out);

    let mut a = vec![0.0f32; m * k];
    for b in 0..batch {
        a[b * k..b * k + n_in].copy_from_slice(&hidden[b * n_in..b * n_in + n_in]);
    }
    // B[k×n]: B[i,j] = weight[j, i]  (weight row j, col i)
    let mut bmat = vec![0.0f32; k * n];
    for j in 0..n_out {
        for i in 0..n_in {
            bmat[i * n + j] = weight[j * n_in + i];
        }
    }

    // Prefer f32-faithful TC/floor (`gemm_f32_tc`), not `gemm_f32_tc_reduced`.
    // Reduced f16-WMMA on pad16 single-token decode was measured as incoherent garbage
    // (2026-07-24). Lab may still force reduced via QUALIA_LLM_CUDA_TC_REDUCED=1.
    let use_reduced = matches!(
        std::env::var("QUALIA_LLM_CUDA_TC_REDUCED").ok().as_deref(),
        Some("1") | Some("true")
    );
    let c = if use_reduced {
        match gemm_f32_tc_reduced(m, k, n, &a, &bmat) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("cuda_lane|batch_gemv|tc_reduced_fail|{e:?}");
                return false;
            }
        }
    } else {
        match crate::wgsl_forge::dispatch::gemm_f32_tc(m, k, n, &a, &bmat) {
            Ok(v) => v,
            Err(e) => {
                log::warn!("cuda_lane|batch_gemv|tc_fail|{e:?}");
                return false;
            }
        }
    };
    for b in 0..batch {
        out[b * n_out..b * n_out + n_out].copy_from_slice(&c[b * n..b * n + n_out]);
    }
    true
}

/// Lookup-only: run GEMV if `key` is already densified in the cache.
pub fn try_cuda_batch_gemv_cached_only(
    key: u64,
    hidden: &[f32],
    batch: usize,
    out: &mut [f32],
) -> bool {
    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    let Ok(mut g) = cache().lock() else {
        return false;
    };
    g.clock = g.clock.wrapping_add(1);
    let clock = g.clock;
    let Some(e) = g.entries.get_mut(&key) else {
        return false;
    };
    e.last_use = clock;
    let (wi, wo) = (e.n_in, e.n_out);
    let w = e.data.clone();
    drop(g);
    try_cuda_batch_gemv(hidden, batch, wi, wo, w.as_slice(), out)
}

/// Like [`try_cuda_batch_gemv`] but uses a cached weight by fingerprint if present;
/// otherwise caches `weight` under `key` then multiplies.
pub fn try_cuda_batch_gemv_cached(
    key: u64,
    hidden: &[f32],
    batch: usize,
    n_in: usize,
    n_out: usize,
    weight: &[f32],
    out: &mut [f32],
) -> bool {
    if try_cuda_batch_gemv_cached_only(key, hidden, batch, out) {
        return true;
    }
    if weight.len() == n_in.saturating_mul(n_out) {
        cache_dense_weight(key, n_in, n_out, weight.to_vec());
        return try_cuda_batch_gemv(hidden, batch, n_in, n_out, weight, out);
    }
    false
}

/// How many weight matrices are currently cached.
pub fn weight_cache_len() -> usize {
    cache().lock().map(|g| g.entries.len()).unwrap_or(0)
}

/// Clear weight cache (tests / model swap).
pub fn clear_weight_cache() {
    if let Ok(mut g) = cache().lock() {
        g.entries.clear();
        g.clock = 0;
    }
}
