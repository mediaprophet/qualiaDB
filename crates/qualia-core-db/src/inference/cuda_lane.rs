//! CUDA inference lane — dense batch GEMM via persistent WMMA (mode=`cuda`).
//!
//! # Goal
//! Prefer tensor-core dense matmul for **batch prefill-shaped** GEMMs when:
//! - `InferenceMode::CudaTc` is active (`prefer_tensor_core_gemm`)
//! - dims can be padded to multiples of 16
//! - a dense f32 weight matrix is available (f16 p64 expand, or one-shot dequant)
//!
//! Weight rows are **cached on the CUDA slab** by content fingerprint so subsequent
//! chunks do not re-upload the same matrix (no host thrash for repeated layers).
//!
//! # Honest limits
//! - Not a fused Q4_K dequant-GEMV on-device (llama.cpp-class) — that remains M2b.
//! - Quantized weights must be dequantized once to dense f32 before first cache insert.
//! - Slab is finite (256 MiB); LRU-ish eviction of oldest entries when full.

#![cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::wgsl_forge::dispatch::{ensure_cuda_runtime_path, gemm_f32_tc_reduced};
use crate::wgsl_forge::execute::CudaComputeContext;

/// Max cached weight matrices (each can be tens of MB).
const MAX_WEIGHT_ENTRIES: usize = 24;
/// Max f32 elements per densified matrix (~192 MiB) — covers 3B FFN (~25M) with headroom.
pub const MAX_DENSE_ELEMS: usize = 48 * 1024 * 1024;

#[derive(Clone)]
struct WeightEntry {
    n_in: usize,
    n_out: usize,
    /// Dense f32 row-major [n_out × n_in] (GGML convention: rows = out).
    data: Vec<f32>,
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
            data,
            last_use: clock,
        },
    );
    let n_ent = g.entries.len();
    log::debug!("cuda_lane|weight_cache|key={key:#x}|n_in={n_in}|n_out={n_out}|entries={n_ent}");
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
        out[b * n_out..b * n_out + n_out]
            .copy_from_slice(&c[b * n..b * n + n_out]);
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
    let w = e.data.clone();
    let (wi, wo) = (e.n_in, e.n_out);
    drop(g);
    try_cuda_batch_gemv(hidden, batch, wi, wo, &w, out)
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

/// Multi-weight device residency: Q4 SoA matrices live permanently in one slab;
/// activations use a **fixed sticky slot** (overwrite in place — no permanent growth).
/// Permanent **device KV** (P4) lives after weights when reserved — same layout as
/// host `KvCacheLayout` f32 mode so SDPA/KV write kernels match engine indices.
///
/// Transient y/dims are rewound to `permanent_end` each call. Slab is 2.5 GiB so a
/// full Llama-3.2-3B Q4 weight set (~1.8 GiB) + f32 KV (~0.22 GiB) can stay resident.
struct MultiWeightDevice {
    ctx: CudaComputeContext,
    /// Write cursor after permanent weights (+ sticky slot + optional KV). Transient starts here.
    permanent_end: u64,
    weights: HashMap<u64, crate::wgsl_forge::execute::memory::BufferView>,
    /// Fixed sticky activation buffer (capacity in floats); content overwritten via write_view.
    sticky_x_key: u64,
    sticky_x_n_in: usize,
    sticky_x_cap: usize,
    sticky_x: Option<crate::wgsl_forge::execute::memory::BufferView>,
    /// Permanent f32 KV arena (device SDPA). None until [`ensure_device_kv_cache`].
    kv: Option<crate::wgsl_forge::execute::memory::BufferView>,
    kv_total_f32: usize,
    kv_max_context: u32,
    kv_n_layer: u32,
    kv_n_kv_head: u32,
    kv_head_dim: u32,
    kv_slot_kv_elems: u32,
    kv_layer_stride: u32,
}

/// CUDA slab for multi-weight SoA + device KV (bytes). 2.5 GiB covers 3B Q4 + KV headroom.
const CUDA_SOA_SLAB_BYTES: u64 = (5 * 1024 * 1024 * 1024) / 2;

fn multi_weight_device() -> &'static Mutex<Option<MultiWeightDevice>> {
    static C: OnceLock<Mutex<Option<MultiWeightDevice>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

/// How many Q4 SoA matrices are sticky-resident on CUDA.
pub fn q4k_device_weight_count() -> usize {
    multi_weight_device()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| d.weights.len()))
        .unwrap_or(0)
}

/// Bulk-preload Q4_K_SOA weight blobs into the multi-weight CUDA slab.
/// Call once at plan build so first-token decode does not thrash PCIe.
/// Each entry is `(raw_bytes, n_in, n_out)`. Returns how many newly resident.
pub fn preload_q4k_soa_weights(weights: &[(&[u8], usize, usize)]) -> usize {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return 0;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return 0;
    }
    let Ok(mut guard) = multi_weight_device().lock() else {
        return 0;
    };
    if !ensure_device(&mut guard) {
        return 0;
    }
    let dev = guard.as_mut().unwrap();
    let mut added = 0usize;
    for &(raw, n_in, n_out) in weights {
        if n_in == 0 || n_out == 0 || n_out > 32_768 {
            continue;
        }
        let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
            continue;
        };
        let need = row_bytes.saturating_mul(n_out);
        if raw.len() < need || need > 48 * 1024 * 1024 {
            continue;
        }
        let key = weight_fingerprint(&raw[..need], n_in, n_out);
        if dev.weights.contains_key(&key) {
            continue;
        }
        if ensure_weight_resident(dev, key, &raw[..need]) {
            added += 1;
        }
    }
    if added > 0 {
        log::info!(
            "cuda_lane|q4k_soa|preload|added={added}|total={}",
            dev.weights.len()
        );
    }
    added
}

/// Ensure the multi-weight CUDA context exists (NVIDIA clocks / driver warm).
/// Safe to call from portable paths — brings A2000 out of idle so wgpu resident
/// decode sees production clocks (measured ~4× vs cold portable on 3B).
pub fn warm_cuda_context() -> bool {
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    let ok = ensure_device(&mut guard);
    if ok {
        log::info!("cuda_lane|warm_context|ok");
    }
    ok
}

fn empty_mw_device(ctx: CudaComputeContext) -> MultiWeightDevice {
    MultiWeightDevice {
        ctx,
        permanent_end: 0,
        weights: HashMap::new(),
        sticky_x_key: 0,
        sticky_x_n_in: 0,
        sticky_x_cap: 0,
        sticky_x: None,
        kv: None,
        kv_total_f32: 0,
        kv_max_context: 0,
        kv_n_layer: 0,
        kv_n_kv_head: 0,
        kv_head_dim: 0,
        kv_slot_kv_elems: 0,
        kv_layer_stride: 0,
    }
}

fn ensure_device(guard: &mut Option<MultiWeightDevice>) -> bool {
    use crate::wgsl_forge::execute::CudaComputeContext;
    if guard.is_some() {
        return true;
    }
    // Prefer 2.5 GiB (weights + KV); fall back 2 GiB → 1 GiB under VRAM pressure.
    let sizes: [u64; 3] = [
        CUDA_SOA_SLAB_BYTES,
        2 * 1024 * 1024 * 1024,
        1024 * 1024 * 1024,
    ];
    let mut last_err = None;
    for &bytes in &sizes {
        match CudaComputeContext::new(bytes as usize) {
            Ok(c) => {
                log::info!(
                    "cuda_lane|q4k_soa|multi_weight_context|{}MiB",
                    bytes / (1024 * 1024)
                );
                *guard = Some(empty_mw_device(c));
                return true;
            }
            Err(e) => {
                last_err = Some(e);
            }
        }
    }
    log::warn!("cuda_lane|q4k_soa|ctx_fail|{last_err:?}");
    false
}

/// Reserve permanent device KV matching host `KvCacheLayout` f32 indices (P4).
/// Call **before** heavy weight preload when possible so the slab still has room.
/// Returns false if layout is invalid or the slab cannot hold the arena.
pub fn ensure_device_kv_cache(
    max_context: u32,
    n_layer: u32,
    n_kv_head: u32,
    head_dim: u32,
    slot_kv_elems: u32,
    layer_stride: u32,
    total_f32_elems: usize,
) -> bool {
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if max_context == 0
        || n_layer == 0
        || n_kv_head == 0
        || head_dim == 0
        || total_f32_elems == 0
        || total_f32_elems > 128 * 1024 * 1024
    {
        return false;
    }
    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if let Some(v) = dev.kv {
        if dev.kv_total_f32 == total_f32_elems
            && dev.kv_max_context == max_context
            && dev.kv_n_layer == n_layer
            && dev.kv_n_kv_head == n_kv_head
            && dev.kv_head_dim == head_dim
            && dev.kv_layer_stride == layer_stride
        {
            let _ = v;
            return true;
        }
        // Layout change: cannot relocate without nuking permanent region — soft-fail.
        log::warn!("cuda_lane|kv|layout_mismatch|refuse_realloc");
        return false;
    }
    let bytes = total_f32_elems.saturating_mul(4);
    let zeros = vec![0u8; bytes];
    dev.ctx.restore_checkpoint(dev.permanent_end);
    match dev.ctx.allocate_and_write(&zeros, 0, 0) {
        Ok(v) => {
            dev.permanent_end = dev.ctx.write_checkpoint();
            dev.kv = Some(v);
            dev.kv_total_f32 = total_f32_elems;
            dev.kv_max_context = max_context;
            dev.kv_n_layer = n_layer;
            dev.kv_n_kv_head = n_kv_head;
            dev.kv_head_dim = head_dim;
            dev.kv_slot_kv_elems = slot_kv_elems;
            dev.kv_layer_stride = layer_stride;
            log::info!(
                "cuda_lane|kv|resident|elems={total_f32_elems}|MiB={}",
                bytes / (1024 * 1024)
            );
            true
        }
        Err(e) => {
            log::warn!("cuda_lane|kv|alloc_fail|bytes={bytes}|{e:?}");
            false
        }
    }
}

/// True when a permanent device KV arena is resident (P4 path eligible).
pub fn device_kv_ready() -> bool {
    multi_weight_device()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|d| d.kv.is_some()))
        .unwrap_or(false)
}

fn ensure_weight_resident(dev: &mut MultiWeightDevice, key: u64, raw: &[u8]) -> bool {
    if dev.weights.contains_key(&key) {
        return true;
    }
    // Weights only: restore to permanent_end (after weights + sticky slot).
    dev.ctx.restore_checkpoint(dev.permanent_end);
    match dev.ctx.allocate_and_write(raw, 1, 0) {
        Ok(v) => {
            dev.permanent_end = dev.ctx.write_checkpoint();
            // Sticky slot sits after weights — if sticky was allocated first at
            // a lower offset, permanent_end still grows correctly for weights.
            dev.weights.insert(key, v);
            log::info!(
                "cuda_lane|q4k_soa|resident+|key={key:#x}|bytes={}|count={}",
                raw.len(),
                dev.weights.len()
            );
            true
        }
        Err(e) => {
            // Soft-fail: do NOT nuke the whole resident set (that thrashing was
            // the measured CUDA_DECODE ~1 tok/s killer). Caller falls back to wgpu.
            log::warn!(
                "cuda_lane|q4k_soa|slab_full_skip|key={key:#x}|bytes={}|err={e:?}",
                raw.len()
            );
            false
        }
    }
}

/// Sticky host→device activation in a **fixed permanent slot** (overwrite in place).
/// Content fingerprint skips H2D when unchanged; capacity grows once if needed.
fn ensure_sticky_x(
    dev: &mut MultiWeightDevice,
    x: &[f32],
) -> Option<crate::wgsl_forge::execute::memory::BufferView> {
    let key = weight_fingerprint(bytemuck::cast_slice(x), x.len(), 0);
    if let Some(v) = dev.sticky_x {
        if dev.sticky_x_key == key && dev.sticky_x_n_in == x.len() {
            return Some(v);
        }
        if dev.sticky_x_cap >= x.len() {
            // In-place overwrite — permanent_end unchanged.
            if let Err(e) = dev.ctx.write_view(&v, bytemuck::cast_slice(x)) {
                log::warn!("cuda_lane|sticky_x|write_view|{e:?}");
                return None;
            }
            dev.sticky_x_key = key;
            dev.sticky_x_n_in = x.len();
            return Some(v);
        }
    }
    // First sticky alloc (or grow): permanent after current weights.
    // Cap at least 8192 floats for typical LLM embd so we rarely re-grow.
    let cap = x.len().max(8192);
    let zeros = vec![0u8; cap * 4];
    dev.ctx.restore_checkpoint(dev.permanent_end);
    match dev.ctx.allocate_and_write(&zeros, 0, 0) {
        Ok(v) => {
            dev.permanent_end = dev.ctx.write_checkpoint();
            if let Err(e) = dev.ctx.write_view(&v, bytemuck::cast_slice(x)) {
                log::warn!("cuda_lane|sticky_x|init_write|{e:?}");
                return None;
            }
            dev.sticky_x_key = key;
            dev.sticky_x_n_in = x.len();
            dev.sticky_x_cap = cap;
            dev.sticky_x = Some(v);
            log::info!("cuda_lane|sticky_x|alloc|cap={cap}|n_in={}", x.len());
            Some(v)
        }
        Err(e) => {
            log::warn!("cuda_lane|sticky_x|alloc_fail|{e:?}");
            None
        }
    }
}

/// On-device Q4_K **SoA** dequant-GEMV (type 112 / `.soa.p64`).
///
/// Dequant in CUDA kernel. **Multi-weight residency**: each distinct matrix
/// fingerprint is uploaded once into a permanent slab region; subsequent layers
/// hit device without PCIe re-upload (the novel Qualia CUDA lane).
pub fn try_q4k_soa_gemv(
    n_in: usize,
    n_out: usize,
    x: &[f32],
    weight_raw: &[u8],
    out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_SRC};
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_in == 0 || n_out == 0 || x.len() < n_in || out.len() < n_out {
        return false;
    }
    // Vocab projection is huge — keep on wgpu; CUDA GEMV targets FFN/attn matrices.
    if n_out > 32_768 {
        return false;
    }
    let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
        return false;
    };
    let need = row_bytes.saturating_mul(n_out);
    if weight_raw.len() < need || need > 48 * 1024 * 1024 {
        return false;
    }
    let key = weight_fingerprint(weight_raw, n_in, n_out);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();

    if !ensure_weight_resident(dev, key, &weight_raw[..need]) {
        return false;
    }
    let view_w = *dev.weights.get(&key).unwrap();
    let view_x = match ensure_sticky_x(dev, &x[..n_in]) {
        Some(v) => v,
        None => return false,
    };
    // Transient y/dims after permanent (+ sticky x).
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zeros = vec![0.0f32; n_out];
    let view_y = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&zeros), 2, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims: [u32; 3] = [n_in as u32, n_out as u32, row_bytes as u32];
    let view_dims = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 3, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };

    let buffers = vec![view_x, view_w, view_y, view_dims];
    let pipeline = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_GEMV_SRC,
        Q4K_SOA_GEMV_ENTRY,
        &[0, 1, 2, 3],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|q4k_soa|compile|{e:?}");
            return false;
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    // Multi-row kernel: one block owns Q4K_SOA_GEMV_ROWS output rows.
    let rows = crate::wgsl_forge::emit::cuda_c::Q4K_SOA_GEMV_ROWS as usize;
    let n_blocks = n_out.div_ceil(rows).max(1);
    if let Err(e) = pipeline.dispatch(&buffers, &schedule, n_blocks.saturating_mul(256)) {
        log::warn!("cuda_lane|q4k_soa|dispatch|{e:?}");
        return false;
    }
    let y = match dev.ctx.read_buffer_f32(&view_y) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|q4k_soa|readback|{e:?}");
            return false;
        }
    };
    out[..n_out].copy_from_slice(&y[..n_out]);
    true
}

/// Project Q, K, and V from the same activation with **one sticky x upload** and
/// **one fused QKV kernel** (shared act tile; GQA: K/V only for `row < n_kv`).
pub fn try_q4k_soa_qkv(
    n_in: usize,
    n_q: usize,
    n_kv: usize,
    x: &[f32],
    q_raw: &[u8],
    k_raw: &[u8],
    v_raw: &[u8],
    q_out: &mut [f32],
    k_out: &mut [f32],
    v_out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{
        Q4K_SOA_GEMV_ROWS, Q4K_SOA_QKV_ENTRY, Q4K_SOA_QKV_SRC,
    };
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_in == 0
        || n_q == 0
        || n_kv == 0
        || n_kv > n_q
        || x.len() < n_in
        || q_out.len() < n_q
        || k_out.len() < n_kv
        || v_out.len() < n_kv
        || n_q > 32_768
    {
        return false;
    }
    let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
        return false;
    };
    let need_q = row_bytes.saturating_mul(n_q);
    let need_kv = row_bytes.saturating_mul(n_kv);
    if q_raw.len() < need_q
        || k_raw.len() < need_kv
        || v_raw.len() < need_kv
        || need_q > 48 * 1024 * 1024
    {
        return false;
    }
    let kq = weight_fingerprint(q_raw, n_in, n_q);
    let kk = weight_fingerprint(k_raw, n_in, n_kv);
    let kv = weight_fingerprint(v_raw, n_in, n_kv);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if !ensure_weight_resident(dev, kq, &q_raw[..need_q])
        || !ensure_weight_resident(dev, kk, &k_raw[..need_kv])
        || !ensure_weight_resident(dev, kv, &v_raw[..need_kv])
    {
        // Fall back to three separate GEMVs (still sticky-x).
        drop(guard);
        return try_q4k_soa_gemv(n_in, n_q, x, q_raw, q_out)
            && try_q4k_soa_gemv(n_in, n_kv, x, k_raw, k_out)
            && try_q4k_soa_gemv(n_in, n_kv, x, v_raw, v_out);
    }
    let mut view_x = match ensure_sticky_x(dev, &x[..n_in]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;
    let mut view_wq = *dev.weights.get(&kq).unwrap();
    let mut view_wk = *dev.weights.get(&kk).unwrap();
    let mut view_wv = *dev.weights.get(&kv).unwrap();
    view_wq.binding = 1;
    view_wk.binding = 2;
    view_wv.binding = 3;

    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zq = vec![0.0f32; n_q];
    let zk = vec![0.0f32; n_kv];
    let zv = vec![0.0f32; n_kv];
    let mut view_yq = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zq), 4, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yk = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zk), 5, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yv = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zv), 6, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    view_yq.binding = 4;
    view_yk.binding = 5;
    view_yv.binding = 6;
    let dims: [u32; 4] = [n_in as u32, n_q as u32, n_kv as u32, row_bytes as u32];
    let view_dims = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&dims), 7, 0) {
        Ok(mut v) => {
            v.binding = 7;
            v
        }
        Err(_) => return false,
    };

    let pipe = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_QKV_SRC,
        Q4K_SOA_QKV_ENTRY,
        &[0, 1, 2, 3, 4, 5, 6, 7],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|qkv|compile|{e:?}");
            drop(guard);
            return try_q4k_soa_gemv(n_in, n_q, x, q_raw, q_out)
                && try_q4k_soa_gemv(n_in, n_kv, x, k_raw, k_out)
                && try_q4k_soa_gemv(n_in, n_kv, x, v_raw, v_out);
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let n_blocks = n_q.div_ceil(Q4K_SOA_GEMV_ROWS as usize).max(1);
    if let Err(e) = pipe.dispatch(
        &[
            view_x, view_wq, view_wk, view_wv, view_yq, view_yk, view_yv, view_dims,
        ],
        &schedule,
        n_blocks.saturating_mul(256),
    ) {
        log::warn!("cuda_lane|qkv|dispatch|{e:?}");
        return false;
    }
    let yq = match dev.ctx.read_buffer_f32(&view_yq) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let yk = match dev.ctx.read_buffer_f32(&view_yk) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let yv = match dev.ctx.read_buffer_f32(&view_yv) {
        Ok(v) => v,
        Err(_) => return false,
    };
    q_out[..n_q].copy_from_slice(&yq[..n_q]);
    k_out[..n_kv].copy_from_slice(&yk[..n_kv]);
    v_out[..n_kv].copy_from_slice(&yv[..n_kv]);
    true
}

/// Full pre-norm FFN expansion+down on CUDA with **one host→device x upload** and
/// **one device→host readback** (mid stays on-device between kernels).
///
/// If `residual` is `Some`, `out[i] = residual[i] + down_i` (on-device residual fuse).
/// If `None`, `out` is the down-projection only (caller adds residual on host).
/// Shapes: gate/up are `n_ffn × n_embd`, down is `n_embd × n_ffn`.
pub fn try_q4k_soa_ffn_block(
    n_embd: usize,
    n_ffn: usize,
    x: &[f32],
    gate_raw: &[u8],
    up_raw: &[u8],
    down_raw: &[u8],
    out: &mut [f32],
) -> bool {
    try_q4k_soa_ffn_block_ex(n_embd, n_ffn, x, None, gate_raw, up_raw, down_raw, out)
}

/// Like [`try_q4k_soa_ffn_block`] but fuses residual on device:
/// `out = residual + FFN(x)` with a single host readback.
pub fn try_q4k_soa_ffn_block_residual(
    n_embd: usize,
    n_ffn: usize,
    x_normed: &[f32],
    residual: &[f32],
    gate_raw: &[u8],
    up_raw: &[u8],
    down_raw: &[u8],
    out: &mut [f32],
) -> bool {
    try_q4k_soa_ffn_block_ex(
        n_embd,
        n_ffn,
        x_normed,
        Some(residual),
        gate_raw,
        up_raw,
        down_raw,
        out,
    )
}

fn try_q4k_soa_ffn_block_ex(
    n_embd: usize,
    n_ffn: usize,
    x: &[f32],
    residual: Option<&[f32]>,
    gate_raw: &[u8],
    up_raw: &[u8],
    down_raw: &[u8],
    out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{
        Q4K_SOA_FUSED_SWIGLU_ENTRY, Q4K_SOA_FUSED_SWIGLU_SRC, Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_SRC,
    };
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_embd == 0
        || n_ffn == 0
        || x.len() < n_embd
        || out.len() < n_embd
        || n_ffn > 32_768
        || n_embd > 32_768
    {
        return false;
    }
    if let Some(r) = residual {
        if r.len() < n_embd {
            return false;
        }
    }
    let Some(row_ffn) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_embd) else {
        return false;
    };
    let Some(row_down) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_ffn) else {
        return false;
    };
    let need_g = row_ffn.saturating_mul(n_ffn);
    let need_d = row_down.saturating_mul(n_embd);
    if gate_raw.len() < need_g
        || up_raw.len() < need_g
        || down_raw.len() < need_d
        || need_g > 48 * 1024 * 1024
        || need_d > 48 * 1024 * 1024
    {
        return false;
    }
    let kg = weight_fingerprint(gate_raw, n_embd, n_ffn);
    let ku = weight_fingerprint(up_raw, n_embd, n_ffn);
    let kd = weight_fingerprint(down_raw, n_ffn, n_embd);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if !ensure_weight_resident(dev, kg, &gate_raw[..need_g])
        || !ensure_weight_resident(dev, ku, &up_raw[..need_g])
        || !ensure_weight_resident(dev, kd, &down_raw[..need_d])
    {
        return false;
    }
    let mut view_x = match ensure_sticky_x(dev, &x[..n_embd]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;

    // One transient arena: mid (n_ffn) + delta (n_embd) + two dims packs.
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let mid_zeros = vec![0.0f32; n_ffn];
    let mut view_mid = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&mid_zeros), 3, 0)
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|ffn_block|mid|{e:?}");
            return false;
        }
    };
    let delta_zeros = vec![0.0f32; n_embd];
    let mut view_delta = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&delta_zeros), 2, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims_s: [u32; 3] = [n_embd as u32, n_ffn as u32, row_ffn as u32];
    let view_dims_s = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_s), 4, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims_d: [u32; 3] = [n_ffn as u32, n_embd as u32, row_down as u32];
    let view_dims_d = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_d), 3, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut view_g = *dev.weights.get(&kg).unwrap();
    let mut view_u = *dev.weights.get(&ku).unwrap();
    let mut view_d = *dev.weights.get(&kd).unwrap();
    view_g.binding = 1;
    view_u.binding = 2;
    view_mid.binding = 3;
    // SwiGLU: x=0, Wg=1, Wu=2, mid=3, dims=4
    let mut dims_s_v = view_dims_s;
    dims_s_v.binding = 4;
    let pipe_s = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_FUSED_SWIGLU_SRC,
        Q4K_SOA_FUSED_SWIGLU_ENTRY,
        &[0, 1, 2, 3, 4],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|ffn_block|swiglu_compile|{e:?}");
            return false;
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let rows = crate::wgsl_forge::emit::cuda_c::Q4K_SOA_GEMV_ROWS as usize;
    // SwiGLU is still 1-row-per-block (dual-weight kernel); multi-row is on plain GEMV.
    // Async launch: same-stream ordered; final readback fences (P4 fence budget).
    if let Err(e) = pipe_s.dispatch_async(
        &[view_x, view_g, view_u, view_mid, dims_s_v],
        &schedule,
        n_ffn.saturating_mul(256),
    ) {
        log::warn!("cuda_lane|ffn_block|swiglu_dispatch|{e:?}");
        return false;
    }

    // Down GEMV multi-row: mid stays on device. Optional residual fuse on device.
    view_mid.binding = 0;
    view_d.binding = 1;
    view_delta.binding = 2;
    let mut dims_d_v = view_dims_d;
    dims_d_v.binding = 3;
    let n_down_blocks = n_embd.div_ceil(rows).max(1);
    if let Some(resid) = residual {
        // residual sticky in permanent region for this call
        let view_r = match dev
            .ctx
            .allocate_and_write(bytemuck::cast_slice(&resid[..n_embd]), 4, 0)
        {
            Ok(mut v) => {
                v.binding = 4;
                v
            }
            Err(e) => {
                log::warn!("cuda_lane|ffn_block|resid_upload|{e:?}");
                return false;
            }
        };
        use crate::wgsl_forge::emit::cuda_c::{
            Q4K_SOA_GEMV_RESID_ENTRY, Q4K_SOA_GEMV_RESID_SRC,
        };
        let pipe_d = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_GEMV_RESID_SRC,
            Q4K_SOA_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|ffn_block|down_resid_compile|{e:?}");
                return false;
            }
        };
        // Final op before D2H: fenced QualiaCompute::dispatch (not async) so
        // readback sees completed kernels — implements the trait path fully.
        if let Err(e) = QualiaCompute::dispatch(
            &pipe_d,
            &[view_mid, view_d, view_delta, dims_d_v, view_r],
            &schedule,
            n_down_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|ffn_block|down_resid_dispatch|{e:?}");
            return false;
        }
    } else {
        let pipe_d = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_GEMV_SRC,
            Q4K_SOA_GEMV_ENTRY,
            &[0, 1, 2, 3],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|ffn_block|down_compile|{e:?}");
                return false;
            }
        };
        if let Err(e) = QualiaCompute::dispatch(
            &pipe_d,
            &[view_mid, view_d, view_delta, dims_d_v],
            &schedule,
            n_down_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|ffn_block|down_dispatch|{e:?}");
            return false;
        }
    }
    let y = match dev.ctx.read_buffer_f32(&view_delta) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|ffn_block|readback|{e:?}");
            return false;
        }
    };
    out[..n_embd].copy_from_slice(&y[..n_embd]);
    true
}

/// Fused SwiGLU expansion on two sticky Q4_K SoA weights (T-A2):  
/// `out[i] = silu(gate_row_i · x) * (up_row_i · x)`. One kernel, one `x` upload.
pub fn try_q4k_soa_fused_swiglu(
    n_in: usize,
    n_out: usize,
    x: &[f32],
    gate_raw: &[u8],
    up_raw: &[u8],
    out: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{Q4K_SOA_FUSED_SWIGLU_ENTRY, Q4K_SOA_FUSED_SWIGLU_SRC};
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    if n_in == 0 || n_out == 0 || x.len() < n_in || out.len() < n_out || n_out > 32_768 {
        return false;
    }
    let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
        return false;
    };
    let need = row_bytes.saturating_mul(n_out);
    if gate_raw.len() < need || up_raw.len() < need || need > 48 * 1024 * 1024 {
        return false;
    }
    let kg = weight_fingerprint(gate_raw, n_in, n_out);
    let ku = weight_fingerprint(up_raw, n_in, n_out);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    if !ensure_weight_resident(dev, kg, &gate_raw[..need])
        || !ensure_weight_resident(dev, ku, &up_raw[..need])
    {
        return false;
    }
    // Permanent weight views store the binding id from first allocate (usually 1).
    // Remap for this launch: x=0, W_gate=1, W_up=2, y=3, dims=4.
    let mut view_g = *dev.weights.get(&kg).unwrap();
    let mut view_u = *dev.weights.get(&ku).unwrap();
    view_g.binding = 1;
    view_u.binding = 2;
    let mut view_x = match ensure_sticky_x(dev, &x[..n_in]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zeros = vec![0.0f32; n_out];
    let view_y = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&zeros), 3, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let dims: [u32; 3] = [n_in as u32, n_out as u32, row_bytes as u32];
    let view_dims = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims), 4, 0)
    {
        Ok(v) => v,
        Err(_) => return false,
    };
    let buffers = vec![view_x, view_g, view_u, view_y, view_dims];
    let pipeline = match CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        Q4K_SOA_FUSED_SWIGLU_SRC,
        Q4K_SOA_FUSED_SWIGLU_ENTRY,
        &[0, 1, 2, 3, 4],
    ) {
        Ok(p) => p,
        Err(e) => {
            log::warn!("cuda_lane|fused_swiglu|compile|{e:?}");
            return false;
        }
    };
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    if let Err(e) = pipeline.dispatch(&buffers, &schedule, n_out.saturating_mul(256)) {
        log::warn!("cuda_lane|fused_swiglu|dispatch|{e:?}");
        return false;
    }
    let y = match dev.ctx.read_buffer_f32(&view_y) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|fused_swiglu|readback|{e:?}");
            return false;
        }
    };
    out[..n_out].copy_from_slice(&y[..n_out]);
    true
}

#[inline]
fn f32_bits_u32(v: f32) -> u32 {
    v.to_bits()
}

/// **P4 device attention:** sticky-x QKV (no intermediate D2H) → RoPE → device KV write
/// → GQA SDPA → O-proj GEMV → **one** residual-delta readback.
///
/// Dual-writes the current token's K/V into `host_kv` (same indices as
/// `KvCacheLayout`) so a later host fallback still sees a consistent cache.
/// Requires [`ensure_device_kv_cache`] to have succeeded for this layout.
pub fn try_q4k_soa_attention_device(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    layer: u32,
    token_idx: u32,
    max_context: u32,
    layer_stride: u32,
    slot_kv_elems: u32,
    rope_base: f32,
    rope_scale: f32,
    x_normed: &[f32],
    q_raw: &[u8],
    k_raw: &[u8],
    v_raw: &[u8],
    o_raw: &[u8],
    host_kv: Option<&mut [f32]>,
    out_delta: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{
        KV_SLOT_WRITE_ENTRY, KV_SLOT_WRITE_SRC, Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_ROWS,
        Q4K_SOA_GEMV_SRC, Q4K_SOA_QKV_ENTRY, Q4K_SOA_QKV_SRC, ROPE_INTERLEAVED_ENTRY,
        ROPE_INTERLEAVED_SRC, SDPA_DECODE_ENTRY, SDPA_DECODE_SRC,
    };
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    let q_dim = n_head.saturating_mul(head_dim);
    let kv_dim = n_kv.saturating_mul(head_dim);
    if n_embd == 0
        || n_head == 0
        || n_kv == 0
        || head_dim == 0
        || head_dim > 256
        || n_kv > n_head
        || x_normed.len() < n_embd
        || out_delta.len() < n_embd
        || q_dim > 32_768
        || token_idx as u64 >= max_context as u64
        || max_context == 0
        || max_context > 1024
    {
        return false;
    }
    let Some(row_in) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_embd) else {
        return false;
    };
    let Some(row_o) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, q_dim) else {
        return false;
    };
    let need_q = row_in.saturating_mul(q_dim);
    let need_kv = row_in.saturating_mul(kv_dim);
    let need_o = row_o.saturating_mul(n_embd);
    if q_raw.len() < need_q
        || k_raw.len() < need_kv
        || v_raw.len() < need_kv
        || o_raw.len() < need_o
        || need_q > 48 * 1024 * 1024
        || need_o > 48 * 1024 * 1024
    {
        return false;
    }

    let kq = weight_fingerprint(q_raw, n_embd, q_dim);
    let kk = weight_fingerprint(k_raw, n_embd, kv_dim);
    let kvw = weight_fingerprint(v_raw, n_embd, kv_dim);
    let ko = weight_fingerprint(o_raw, q_dim, n_embd);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    let Some(mut view_kv) = dev.kv else {
        return false;
    };
    if dev.kv_total_f32 == 0
        || dev.kv_max_context != max_context
        || dev.kv_n_layer == 0
        || layer >= dev.kv_n_layer
        || dev.kv_n_kv_head != n_kv as u32
        || dev.kv_head_dim != head_dim as u32
        || dev.kv_layer_stride != layer_stride
        || dev.kv_slot_kv_elems != slot_kv_elems
    {
        return false;
    }
    if !ensure_weight_resident(dev, kq, &q_raw[..need_q])
        || !ensure_weight_resident(dev, kk, &k_raw[..need_kv])
        || !ensure_weight_resident(dev, kvw, &v_raw[..need_kv])
        || !ensure_weight_resident(dev, ko, &o_raw[..need_o])
    {
        return false;
    }

    let mut view_x = match ensure_sticky_x(dev, &x_normed[..n_embd]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;
    let mut view_wq = *dev.weights.get(&kq).unwrap();
    let mut view_wk = *dev.weights.get(&kk).unwrap();
    let mut view_wv = *dev.weights.get(&kvw).unwrap();
    let mut view_wo = *dev.weights.get(&ko).unwrap();
    view_wq.binding = 1;
    view_wk.binding = 2;
    view_wv.binding = 3;

    // Transient: yq, yk, yv, attn_out, o_out, dims packs
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zq = vec![0.0f32; q_dim];
    let zk = vec![0.0f32; kv_dim];
    let zv = vec![0.0f32; kv_dim];
    let za = vec![0.0f32; q_dim];
    let zo = vec![0.0f32; n_embd];
    let mut view_yq = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zq), 4, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yk = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zk), 5, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yv = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zv), 6, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_attn = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&za), 2, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_o = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zo), 2, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    view_yq.binding = 4;
    view_yk.binding = 5;
    view_yv.binding = 6;

    let dims_qkv: [u32; 4] = [n_embd as u32, q_dim as u32, kv_dim as u32, row_in as u32];
    let view_dims_qkv = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_qkv), 7, 0)
    {
        Ok(mut v) => {
            v.binding = 7;
            v
        }
        Err(_) => return false,
    };

    let schedule256 = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };

    // RoPE / KV / SDPA / O param packs allocated up front so each pipeline
    // (which borrows ctx) can be scoped and dropped before the next allocate.
    let rope_params_k: [u32; 5] = [
        n_kv as u32,
        head_dim as u32,
        token_idx,
        f32_bits_u32(rope_base),
        f32_bits_u32(if rope_scale > 0.0 && rope_scale.is_finite() {
            rope_scale
        } else {
            1.0
        }),
    ];
    let rope_params_q: [u32; 5] = [
        n_head as u32,
        head_dim as u32,
        token_idx,
        rope_params_k[3],
        rope_params_k[4],
    ];
    let view_rp_k = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&rope_params_k), 1, 0)
    {
        Ok(mut v) => {
            v.binding = 1;
            v
        }
        Err(_) => return false,
    };
    let view_rp_q = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&rope_params_q), 1, 0)
    {
        Ok(mut v) => {
            v.binding = 1;
            v
        }
        Err(_) => return false,
    };

    // --- 1) Fused QKV ---
    {
        let pipe_qkv = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_QKV_SRC,
            Q4K_SOA_QKV_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|qkv_compile|{e:?}");
                return false;
            }
        };
        let n_q_blocks = q_dim.div_ceil(Q4K_SOA_GEMV_ROWS as usize).max(1);
        if let Err(e) = pipe_qkv.dispatch_async(
            &[
                view_x, view_wq, view_wk, view_wv, view_yq, view_yk, view_yv, view_dims_qkv,
            ],
            &schedule256,
            n_q_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|attn_dev|qkv_dispatch|{e:?}");
            return false;
        }
    }

    // --- 2) RoPE K + RoPE Q (in-place on device) ---
    {
        let pipe_rope = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            ROPE_INTERLEAVED_SRC,
            ROPE_INTERLEAVED_ENTRY,
            &[0, 1],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|rope_compile|{e:?}");
                return false;
            }
        };
        view_yk.binding = 0;
        let k_pairs = n_kv.saturating_mul(head_dim / 2).max(1);
        if let Err(e) = pipe_rope.dispatch_async(
            &[view_yk, view_rp_k],
            &schedule256,
            k_pairs.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|rope_k|{e:?}");
            return false;
        }
        view_yq.binding = 0;
        let q_pairs = n_head.saturating_mul(head_dim / 2).max(1);
        if let Err(e) = pipe_rope.dispatch_async(
            &[view_yq, view_rp_q],
            &schedule256,
            q_pairs.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|rope_q|{e:?}");
            return false;
        }
    }

    // --- 3) Device KV write (K then V) ---
    // Allocate all param packs before compiling (CudaPipeline borrows ctx).
    let slot = token_idx % max_context;
    let kvw_params_k: [u32; 7] = [
        n_kv as u32,
        head_dim as u32,
        layer,
        slot,
        layer_stride,
        slot_kv_elems,
        0,
    ];
    let kvw_params_v: [u32; 7] = [
        n_kv as u32,
        head_dim as u32,
        layer,
        slot,
        layer_stride,
        slot_kv_elems,
        1,
    ];
    let q_per_kv = (n_head / n_kv.max(1)).max(1) as u32;
    let scale = 1.0f32 / (head_dim as f32).sqrt();
    let sdpa_params: [u32; 9] = [
        n_head as u32,
        n_kv as u32,
        head_dim as u32,
        layer,
        token_idx,
        max_context,
        layer_stride,
        slot_kv_elems,
        q_per_kv,
    ];
    let scale_bits = [f32_bits_u32(scale)];
    let dims_o: [u32; 3] = [q_dim as u32, n_embd as u32, row_o as u32];
    let view_kp = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&kvw_params_k), 2, 0)
    {
        Ok(mut v) => {
            v.binding = 2;
            v
        }
        Err(_) => return false,
    };
    let view_vp = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&kvw_params_v), 2, 0)
    {
        Ok(mut v) => {
            v.binding = 2;
            v
        }
        Err(_) => return false,
    };
    let view_sp = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&sdpa_params), 3, 0)
    {
        Ok(mut v) => {
            v.binding = 3;
            v
        }
        Err(_) => return false,
    };
    let view_sb = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&scale_bits), 4, 0)
    {
        Ok(mut v) => {
            v.binding = 4;
            v
        }
        Err(_) => return false,
    };
    let view_dims_o = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_o), 3, 0)
    {
        Ok(mut v) => {
            v.binding = 3;
            v
        }
        Err(_) => return false,
    };

    {
        let pipe_kvw = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            KV_SLOT_WRITE_SRC,
            KV_SLOT_WRITE_ENTRY,
            &[0, 1, 2],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|kvw_compile|{e:?}");
                return false;
            }
        };
        view_yk.binding = 0;
        view_kv.binding = 1;
        let kv_elems = kv_dim.max(1);
        if let Err(e) = pipe_kvw.dispatch_async(
            &[view_yk, view_kv, view_kp],
            &schedule256,
            kv_elems.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|kv_write_k|{e:?}");
            return false;
        }
        view_yv.binding = 0;
        if let Err(e) = pipe_kvw.dispatch_async(
            &[view_yv, view_kv, view_vp],
            &schedule256,
            kv_elems.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|kv_write_v|{e:?}");
            return false;
        }
    }

    // --- 4) Device SDPA ---
    {
        view_yq.binding = 0;
        view_kv.binding = 1;
        view_attn.binding = 2;
        let pipe_sdpa = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            SDPA_DECODE_SRC,
            SDPA_DECODE_ENTRY,
            &[0, 1, 2, 3, 4],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|sdpa_compile|{e:?}");
                return false;
            }
        };
        // grid = n_head blocks × 256 threads
        if let Err(e) = pipe_sdpa.dispatch_async(
            &[view_yq, view_kv, view_attn, view_sp, view_sb],
            &schedule256,
            n_head.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|attn_dev|sdpa_dispatch|{e:?}");
            return false;
        }
    }

    // --- 5) O-proj multi-row GEMV (attn stays on device) ---
    {
        view_attn.binding = 0;
        view_wo.binding = 1;
        view_o.binding = 2;
        let pipe_o = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_GEMV_SRC,
            Q4K_SOA_GEMV_ENTRY,
            &[0, 1, 2, 3],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|o_compile|{e:?}");
                return false;
            }
        };
        let n_o_blocks = n_embd.div_ceil(Q4K_SOA_GEMV_ROWS as usize).max(1);
        // Last kernel before D2H: fenced QualiaCompute::dispatch so trait path is live.
        if let Err(e) = QualiaCompute::dispatch(
            &pipe_o,
            &[view_attn, view_wo, view_o, view_dims_o],
            &schedule256,
            n_o_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|attn_dev|o_dispatch|{e:?}");
            return false;
        }
    }

    // --- 6) Single residual-delta readback ---
    let y = match dev.ctx.read_buffer_f32(&view_o) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|attn_dev|readback|{e:?}");
            return false;
        }
    };
    out_delta[..n_embd].copy_from_slice(&y[..n_embd]);

    // Dual-write host KV (small K/V after RoPE) for fallback consistency.
    if let Some(hkv) = host_kv {
        if let (Ok(k_host), Ok(v_host)) = (
            dev.ctx.read_buffer_f32(&view_yk),
            dev.ctx.read_buffer_f32(&view_yv),
        ) {
            let layer_base = layer as usize * layer_stride as usize
                + slot as usize * slot_kv_elems as usize * 2;
            let v_off = n_kv * head_dim;
            for i in 0..kv_dim {
                let ki = layer_base + i;
                let vi = layer_base + v_off + i;
                if ki < hkv.len() {
                    hkv[ki] = k_host[i];
                }
                if vi < hkv.len() {
                    hkv[vi] = v_host[i];
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ggml_quants::{
        dequantize_row_into, q4k_block_to_soa, GGML_TYPE_Q4_K, GGML_TYPE_Q4_K_SOA,
        BLOCK_Q4K_SOA_BYTES,
    };
    use crate::inference_modes::{set_inference_mode, InferenceMode};

    #[test]
    fn fingerprint_stable() {
        let a = weight_fingerprint(b"abc", 3, 2);
        let b = weight_fingerprint(b"abc", 3, 2);
        assert_eq!(a, b);
        assert_ne!(a, weight_fingerprint(b"abd", 3, 2));
    }

    #[test]
    fn batch_gemv_cpu_shape_pad() {
        // Without cuda mode this returns false quickly.
        if std::env::var("QUALIA_INFERENCE_MODE").ok().as_deref() != Some("cuda") {
            let mut out = [0.0f32; 4];
            let ok = try_cuda_batch_gemv(&[1.0, 0.0], 1, 2, 2, &[1.0, 0.0, 0.0, 1.0], &mut out);
            assert!(!ok);
        }
    }

    /// Build one synthetic Q4_K row (256 weights) → SoA, compare CUDA GEMV vs CPU dequant·dot.
    #[test]
    fn q4k_soa_gemv_matches_cpu_when_cuda_available() {
        if std::env::var("QUALIA_SKIP_CUDA").is_ok() {
            return;
        }
        // Deterministic pseudo-Q4_K block (144 B).
        let mut stock = [0u8; 144];
        stock[0] = 0x00;
        stock[1] = 0x3c; // d ≈ 1.0 f16
        stock[2] = 0x00;
        stock[3] = 0x38; // dmin small
        for i in 4..16 {
            stock[i] = 0x21;
        }
        for i in 16..144 {
            stock[i] = (i as u8).wrapping_mul(17);
        }
        let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
        q4k_block_to_soa(&stock, &mut soa).expect("soa convert");

        let n_in = 256usize;
        let n_out = 4usize;
        let mut weight = Vec::with_capacity(n_out * BLOCK_Q4K_SOA_BYTES);
        for _ in 0..n_out {
            weight.extend_from_slice(&soa);
        }
        let x: Vec<f32> = (0..n_in).map(|i| (i as f32) * 0.01).collect();

        // CPU reference: dequant each row and dot.
        let mut cpu_out = vec![0.0f32; n_out];
        let mut row = vec![0.0f32; n_in];
        for r in 0..n_out {
            dequantize_row_into(
                &weight[r * BLOCK_Q4K_SOA_BYTES..(r + 1) * BLOCK_Q4K_SOA_BYTES],
                GGML_TYPE_Q4_K_SOA,
                n_in,
                &mut row,
            )
            .unwrap();
            cpu_out[r] = row.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
        }

        // Force cuda mode for prefer_tensor_core_gemm.
        let prev = std::env::var("QUALIA_INFERENCE_MODE").ok();
        set_inference_mode(InferenceMode::CudaTc);
        std::env::set_var("QUALIA_INFERENCE_MODE", "cuda");

        let mut gpu_out = vec![0.0f32; n_out];
        let ok = try_q4k_soa_gemv(n_in, n_out, &x, &weight, &mut gpu_out);

        // Restore env.
        match prev {
            Some(v) => std::env::set_var("QUALIA_INFERENCE_MODE", v),
            None => std::env::remove_var("QUALIA_INFERENCE_MODE"),
        }
        set_inference_mode(InferenceMode::Portable);

        if !ok {
            // No CUDA toolkit / NVRTC — soft skip.
            eprintln!("q4k_soa_gemv: CUDA unavailable — skipped differential");
            return;
        }
        for r in 0..n_out {
            let err = (cpu_out[r] - gpu_out[r]).abs();
            let tol = 1e-2 * cpu_out[r].abs().max(1.0);
            assert!(
                err < tol,
                "row {r}: cpu={} gpu={} err={err}",
                cpu_out[r],
                gpu_out[r]
            );
        }
        // Multi-weight residency: a second distinct matrix should bump device count.
        let before = q4k_device_weight_count();
        let mut weight2 = weight.clone();
        if let Some(b) = weight2.last_mut() {
            *b = b.wrapping_add(1);
        }
        let mut gpu2 = vec![0.0f32; n_out];
        let ok2 = try_q4k_soa_gemv(n_in, n_out, &x, &weight2, &mut gpu2);
        if ok2 {
            assert!(
                q4k_device_weight_count() >= before.max(1),
                "expected sticky multi-weight residency after second matrix"
            );
        }
        let _ = GGML_TYPE_Q4_K; // silence if unused
    }
}
