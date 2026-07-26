//! Multi-weight device slab management — permanent Q4 SoA weight residency,
//! sticky activation buffers, and permanent device KV cache arena.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::wgsl_forge::execute::memory::BufferView;
use crate::wgsl_forge::execute::{CapturedCudaGraph, CudaComputeContext};

use super::weight_cache::weight_fingerprint;

/// Sticky activation buffers for the CUDA mega-pass, allocated once in the permanent
/// slab region and overwritten in-place each decode step. Eliminates per-call
/// transient allocation overhead (~15 buffers per decode).
///
/// Layout (all f32, allocated contiguously after KV in the permanent region):
/// hidden_a[n_embd] | hidden_b[n_embd] |
/// yq[q_dim] | yk[kv_dim] | yv[kv_dim] | attn_out[q_dim] |
/// o_delta[n_embd] | ffn_mid[n_ffn] | ffn_out[n_embd] |
/// param packs (small u32 arrays) |
/// logits[max_vocab] | token[1]

/// Multi-weight device residency: Q4 SoA matrices live permanently in one slab;
/// activations use a **fixed sticky slot** (overwrite in place — no permanent growth).
/// Permanent **device KV** (P4) lives after weights when reserved — same layout as
/// host `KvCacheLayout` f32 mode so SDPA/KV write kernels match engine indices.
///
/// Transient y/dims are rewound to `permanent_end` each call. Slab is 2.5 GiB so a
/// full Llama-3.2-3B Q4 weight set (~1.8 GiB) + f32 KV (~0.22 GiB) can stay resident.
pub(crate) struct MultiWeightDevice {
    pub ctx: CudaComputeContext,
    /// Write cursor after permanent weights (+ sticky slot + optional KV). Transient starts here.
    pub permanent_end: u64,
    pub weights: HashMap<u64, crate::wgsl_forge::execute::memory::BufferView>,
    /// Fixed sticky activation buffer (capacity in floats); content overwritten via write_view.
    pub sticky_x_key: u64,
    pub sticky_x_n_in: usize,
    pub sticky_x_cap: usize,
    pub sticky_x: Option<crate::wgsl_forge::execute::memory::BufferView>,
    /// Permanent f32 KV arena (device SDPA). None until [`ensure_device_kv_cache`].
    pub kv: Option<crate::wgsl_forge::execute::memory::BufferView>,
    /// Prepared `[layer][logical_page] -> physical_page` indirection.
    pub kv_block_table: Option<crate::wgsl_forge::execute::memory::BufferView>,
    pub kv_block_size: u32,
    pub kv_blocks_per_layer: u32,
    pub kv_total_f32: usize,
    pub kv_max_context: u32,
    pub kv_n_layer: u32,
    pub kv_n_kv_head: u32,
    pub kv_head_dim: u32,
    pub kv_slot_kv_elems: u32,
    pub kv_layer_stride: u32,
    /// Sticky mega-pass activation arena (allocated once, overwritten in-place).
    pub mega_pass_arena: Option<MegaPassArena>,
    /// Captured full-model decode graph for the current prepared-plan shape.
    pub decode_graph: Option<CapturedCudaGraph>,
    pub decode_graph_key: u64,
    /// Exact number of device kernel nodes recorded in `decode_graph`.
    pub decode_graph_node_count: u64,
    /// Exact dynamic H2D traffic required before each graph launch.
    pub decode_graph_h2d_bytes_per_token: u64,
    /// Fingerprint of static parameter packs currently resident in the arena.
    pub mega_params_key: u64,
}

/// CUDA slab for multi-weight SoA + device KV (bytes). 2.5 GiB covers 3B Q4 + KV headroom.
const CUDA_SOA_SLAB_BYTES: u64 = (5 * 1024 * 1024 * 1024) / 2;

pub(crate) fn multi_weight_device() -> &'static Mutex<Option<MultiWeightDevice>> {
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

pub fn q4k_weight_resident(key: u64) -> bool {
    multi_weight_device()
        .lock()
        .ok()
        .and_then(|guard| {
            guard
                .as_ref()
                .map(|device| device.weights.contains_key(&key))
        })
        .unwrap_or(false)
}

/// Return the stable key of the instantiated full-decode CUDA graph, if one is live.
///
/// This is receipt telemetry only: callers cannot launch or mutate the graph through it.
pub(crate) fn decode_graph_key() -> Option<u64> {
    multi_weight_device()
        .lock()
        .ok()
        .and_then(|guard| {
            guard.as_ref().and_then(|device| {
                device
                    .decode_graph
                    .as_ref()
                    .map(|_| device.decode_graph_key)
            })
        })
        .filter(|key| *key != 0)
}

/// Return the exact kernel-node count recorded in the live decode graph.
///
/// Keeping this beside the graph avoids reconstructing telemetry from stale schedule
/// assumptions when an opt-in fusion candidate adds or removes nodes.
pub(crate) fn decode_graph_node_count() -> Option<u64> {
    multi_weight_device()
        .lock()
        .ok()
        .and_then(|guard| {
            guard.as_ref().and_then(|device| {
                device
                    .decode_graph
                    .as_ref()
                    .map(|_| device.decode_graph_node_count)
            })
        })
        .filter(|count| *count != 0)
}

/// Return exact dynamic H2D bytes associated with the live captured graph.
pub(crate) fn decode_graph_h2d_bytes_per_token() -> Option<u64> {
    multi_weight_device().lock().ok().and_then(|guard| {
        guard.as_ref().and_then(|device| {
            device
                .decode_graph
                .as_ref()
                .map(|_| device.decode_graph_h2d_bytes_per_token)
        })
    })
}

/// Upload an immutable prepared-plan blob to the permanent CUDA slab.
///
/// The caller supplies the stable content key. This is used for small f32 vectors such as
/// RMSNorm weights as well as quantized matrices, so the token path never performs H2D writes.
pub fn preload_resident_blob(key: u64, bytes: &[u8]) -> bool {
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};

    if key == 0 || bytes.is_empty() || bytes.len() > 256 * 1024 * 1024 {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    ensure_weight_resident(guard.as_mut().unwrap(), key, bytes)
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
        if n_in == 0 || n_out == 0 || n_out > 131_072 {
            continue;
        }
        let Some(row_bytes) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_in) else {
            continue;
        };
        let need = row_bytes.saturating_mul(n_out);
        if raw.len() < need || need > 256 * 1024 * 1024 {
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
        kv_block_table: None,
        kv_block_size: 0,
        kv_blocks_per_layer: 0,
        kv_total_f32: 0,
        kv_max_context: 0,
        kv_n_layer: 0,
        kv_n_kv_head: 0,
        kv_head_dim: 0,
        kv_slot_kv_elems: 0,
        kv_layer_stride: 0,
        mega_pass_arena: None,
        decode_graph: None,
        decode_graph_key: 0,
        decode_graph_node_count: 0,
        decode_graph_h2d_bytes_per_token: 0,
        mega_params_key: 0,
    }
}

pub(crate) fn ensure_device(guard: &mut Option<MultiWeightDevice>) -> bool {
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
            && dev.kv_block_table.is_some()
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
    let Some(config) = crate::inference::runtime::kv::paged::PagedKvConfig::new(
        n_layer,
        n_kv_head,
        head_dim,
        max_context,
    ) else {
        return false;
    };
    let Ok(table) = crate::inference::runtime::kv::paged::GpuBlockTablePlan::identity(config)
    else {
        return false;
    };
    let checkpoint = dev.permanent_end;
    dev.ctx.restore_checkpoint(dev.permanent_end);
    match dev.ctx.allocate_and_write(&zeros, 0, 0) {
        Ok(v) => {
            let block_table =
                match dev
                    .ctx
                    .allocate_and_write(bytemuck::cast_slice(table.entries()), 0, 0)
                {
                    Ok(table) => table,
                    Err(error) => {
                        dev.ctx.restore_checkpoint(checkpoint);
                        log::warn!("cuda_lane|kv|block_table_alloc_fail|{error:?}");
                        return false;
                    }
                };
            dev.permanent_end = dev.ctx.write_checkpoint();
            dev.kv = Some(v);
            dev.kv_block_table = Some(block_table);
            dev.kv_block_size = config.block_size;
            dev.kv_blocks_per_layer = config.logical_blocks_per_layer();
            dev.kv_total_f32 = total_f32_elems;
            dev.kv_max_context = max_context;
            dev.kv_n_layer = n_layer;
            dev.kv_n_kv_head = n_kv_head;
            dev.kv_head_dim = head_dim;
            dev.kv_slot_kv_elems = slot_kv_elems;
            dev.kv_layer_stride = layer_stride;
            log::info!(
                "cuda_lane|kv|resident_paged|elems={total_f32_elems}|MiB={}|page_tokens={}|pages_per_layer={}",
                bytes / (1024 * 1024),
                dev.kv_block_size,
                dev.kv_blocks_per_layer,
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

pub(crate) fn ensure_weight_resident(dev: &mut MultiWeightDevice, key: u64, raw: &[u8]) -> bool {
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
            log::debug!(
                "cuda_lane|resident_blob|resident+|key={key:#x}|bytes={}|count={}",
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
pub(crate) fn ensure_sticky_x(
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

/// Sticky activation buffers for the CUDA mega-pass, allocated once in the permanent
/// slab region and overwritten in-place each decode step. Eliminates per-call
/// transient allocation overhead (~15 buffers per decode).
pub(crate) struct MegaPassArena {
    // Double-buffered hidden state (residual ↔ norm swap each layer).
    pub hidden_a: BufferView,
    pub hidden_b: BufferView,
    // QKV outputs.
    pub yq: BufferView,
    pub yk: BufferView,
    pub yv: BufferView,
    // Attention output (post-SDPA).
    pub attn_out: BufferView,
    // Long-context online-softmax partials:
    // [query_head][segment][max, sum, head_dim values].
    pub attn_partials: BufferView,
    // FFN intermediate (SwiGLU output).
    pub ffn_mid: BufferView,
    // Reusable transient-Q8 activation and one f32 scale per 32 values.
    pub q8_activation: BufferView,
    pub q8_activation_scales: BufferView,
    // Norm weight upload buffer.
    // Small param packs (reused per layer via write_view).
    pub p_rms: BufferView,
    pub p_qkv: BufferView,
    pub p_rope: BufferView,
    pub p_rope_k: BufferView,
    pub p_kvw: BufferView,
    pub p_sdpa: BufferView,
    pub p_sdpa_scale: BufferView,
    pub p_gemv_dims: BufferView,
    pub p_ffn_dims: BufferView,
    pub p_down_dims: BufferView,
    // Logits + token buffers for lm_head path.
    pub logits: BufferView,
    pub token: BufferView,
    pub p_argmax: BufferView,
    pub p_lm_dims: BufferView,
    /// One immutable device scalar per transformer layer.
    pub p_layer_ids: Vec<BufferView>,
    /// Dynamic `[absolute_position, ring_slot, token_id]`, updated once per token.
    pub p_step: BufferView,
    // Dims snapshot for validation.
    pub n_embd: usize,
    pub n_head: usize,
    pub head_dim: usize,
    pub q_dim: usize,
    pub kv_dim: usize,
    pub n_ffn: usize,
    pub max_vocab: usize,
    pub n_layer: usize,
}

/// Ensure the mega-pass sticky arena is allocated and dimensions match.
/// Allocates in the permanent region after KV. Returns false if dims changed
/// and the arena cannot be reallocated (would require nuking permanent region).
pub(crate) fn ensure_mega_pass_arena(
    dev: &mut MultiWeightDevice,
    n_embd: usize,
    n_head: usize,
    head_dim: usize,
    q_dim: usize,
    kv_dim: usize,
    n_ffn: usize,
    max_vocab: usize,
    n_layer: usize,
) -> bool {
    if let Some(ref arena) = dev.mega_pass_arena {
        if arena.n_embd == n_embd
            && arena.n_head == n_head
            && arena.head_dim == head_dim
            && arena.q_dim == q_dim
            && arena.kv_dim == kv_dim
            && arena.n_ffn == n_ffn
            && arena.max_vocab == max_vocab
            && arena.n_layer == n_layer
        {
            return true;
        }
        // Dim mismatch: cannot relocate without nuking permanent region — soft-fail.
        log::warn!(
            "cuda_lane|mega_pass_arena|dim_mismatch|refuse_realloc|old embd={} q={} kv={} ffn={} vocab={} | new embd={} q={} kv={} ffn={} vocab={}",
            arena.n_embd, arena.q_dim, arena.kv_dim, arena.n_ffn, arena.max_vocab,
            n_embd, q_dim, kv_dim, n_ffn, max_vocab
        );
        return false;
    }

    // Allocate all buffers contiguously in the permanent region.
    dev.ctx.restore_checkpoint(dev.permanent_end);

    let zeros_embd = vec![0.0f32; n_embd];
    let zeros_q = vec![0.0f32; q_dim];
    let zeros_kv = vec![0.0f32; kv_dim];
    let zeros_attn_partials = vec![
        0.0f32;
        n_head
            .saturating_mul(super::paged_attention::MAX_ATTENTION_SEGMENTS)
            .saturating_mul(head_dim.saturating_add(2))
    ];
    let zeros_ffn = vec![0.0f32; n_ffn];
    let zeros_vocab = vec![0.0f32; max_vocab];
    let max_activation = n_embd.max(q_dim).max(n_ffn);
    let zeros_q8_activation = vec![0u8; max_activation];
    let zeros_q8_scales = vec![0.0f32; max_activation.div_ceil(32)];

    let alloc = |ctx: &mut CudaComputeContext, data: &[u8]| -> Option<BufferView> {
        ctx.allocate_and_write(data, 0, 0).ok()
    };

    macro_rules! try_alloc {
        ($e:expr) => {
            match $e {
                Some(v) => v,
                None => return false,
            }
        };
    }

    let hidden_a = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_embd)));
    let hidden_b = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_embd)));
    let yq = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_q)));
    let yk = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_kv)));
    let yv = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_kv)));
    let attn_out = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_q)));
    let attn_partials = try_alloc!(alloc(
        &mut dev.ctx,
        bytemuck::cast_slice(&zeros_attn_partials)
    ));
    let ffn_mid = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_ffn)));
    let q8_activation = try_alloc!(alloc(&mut dev.ctx, &zeros_q8_activation));
    let q8_activation_scales =
        try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_q8_scales)));

    // Small param packs.
    let p_rms = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 2])));
    let p_qkv = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 5])));
    let p_rope = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 5])));
    let p_rope_k = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 5])));
    let p_kvw = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 5])));
    let p_sdpa = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 9])));
    let p_sdpa_scale = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 1])));
    // Shared by plain GEMV (3 words) and fused RMSNorm+SwiGLU (4 words).
    let p_gemv_dims = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 4])));
    let p_ffn_dims = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 4])));
    let p_down_dims = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 3])));

    // Logits + token.
    let logits = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&zeros_vocab)));
    let token = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 1])));
    let p_argmax = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 1])));
    let p_lm_dims = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 3])));
    let mut p_layer_ids = Vec::with_capacity(n_layer);
    for layer in 0..n_layer {
        p_layer_ids.push(try_alloc!(alloc(
            &mut dev.ctx,
            bytemuck::cast_slice(&[layer as u32]),
        )));
    }
    let p_step = try_alloc!(alloc(&mut dev.ctx, bytemuck::cast_slice(&[0u32; 3]),));

    dev.permanent_end = dev.ctx.write_checkpoint();

    let total_floats = n_embd * 2 + q_dim * 2 + kv_dim * 2 + n_ffn + max_vocab;
    let total_bytes = total_floats * 4
        + zeros_q8_activation.len()
        + zeros_q8_scales.len() * 4
        + 3 * 4
        + 5 * 4
        + 5 * 4
        + 5 * 4
        + 7 * 4
        + 9 * 4
        + 4
        + 4 * 4
        + 4
        + 3 * 4;
    log::info!(
        "cuda_lane|mega_pass_arena|alloc|embd={n_embd}|q={q_dim}|kv={kv_dim}|ffn={n_ffn}|vocab={max_vocab}|~{}KiB",
        total_bytes / 1024
    );

    dev.mega_pass_arena = Some(MegaPassArena {
        hidden_a,
        hidden_b,
        yq,
        yk,
        yv,
        attn_out,
        attn_partials,
        ffn_mid,
        q8_activation,
        q8_activation_scales,
        p_rms,
        p_qkv,
        p_rope,
        p_rope_k,
        p_kvw,
        p_sdpa,
        p_sdpa_scale,
        p_gemv_dims,
        p_ffn_dims,
        p_down_dims,
        logits,
        token,
        p_argmax,
        p_lm_dims,
        p_layer_ids,
        p_step,
        n_embd,
        n_head,
        head_dim,
        q_dim,
        kv_dim,
        n_ffn,
        max_vocab,
        n_layer,
    });
    true
}
