//! Model-inference runtime (honest module name: `crate::inference_runtime`).
//!
//! This is the **runtime** that runs model inference — not an "AI engine". It does two ordinary
//! systems jobs: (1) **reads the GGUF weight file format** (tensors memory-mapped via `memmap2`,
//! zero heap copy) and (2) **dispatches the tensor program to a GPU backend** (DirectML 1.15 on
//! Windows x64; wgpu/WGSL — Vulkan/Metal/WebGPU — elsewhere).
//!
//! The *mathematics* of inference is not here and is not proprietary: it lives in `crate::solvers`
//! as named STEM — GEMM (`linear_algebra::gemm`), activations/softmax/normalization
//! (`activation`), attention (`attention`), RoPE (`rope`), the SwiGLU FFN (`feed_forward`) — and
//! each kernel in this crate is proven equal to that library definition (the `*_stem_parity_tests`
//! and `substrate_parity_tests`). What remains here is plumbing: GPU command encoding, the KV
//! cache, weight loading, the autoregressive loop, and the GGUF/dequant codec.

// pub(crate) so the concern submodules (gemm / ffn / attention / output / embedding / mc8_wasm …)
// inherit these via `use super::*` — keeps the impl split a pure structural move.
pub(crate) use crate::gguf_sharder::GgufTensorInfo;
pub(crate) use crate::NQuin;
use log;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use memmap2::MmapOptions;
pub(crate) use std::sync::Arc;

pub use crate::ggml_quants::{fetch_token_embedding, ExecutionError};

/// Dequantize a mmap embedding row into caller-supplied `out` (no heap allocation).
pub fn dequantize_token_embedding_into(
    raw: &[u8],
    tensor: &GgufTensorInfo,
    out: &mut [f32],
) -> Result<usize, ExecutionError> {
    let n_embd = tensor.dims[0] as usize;
    if out.len() < n_embd {
        return Err(ExecutionError::MmapBounds);
    }
    crate::ggml_quants::dequantize_row_into(raw, tensor.ggml_type, n_embd, out).map_err(|e| match e
    {
        crate::ggml_quants::GgmlDequantError::UnsupportedType => ExecutionError::UnsupportedType,
        crate::ggml_quants::GgmlDequantError::BufferTooSmall
        | crate::ggml_quants::GgmlDequantError::TruncatedInput => ExecutionError::MmapBounds,
    })
}

/// Represents a Q4_K Quantized or standard float Tensor mapped from a monolithic GGUF file.
#[derive(Debug, Clone)]
pub struct QTensor {
    pub shape: Vec<usize>,
    pub byte_offset: u64,
    pub is_quantized_q4_k: bool,
}

impl QTensor {
    pub fn new(shape: Vec<usize>, byte_offset: u64, is_quantized_q4_k: bool) -> Self {
        Self {
            shape,
            byte_offset,
            is_quantized_q4_k,
        }
    }

    /// Maps the exact bytes from the GGUF using the 60-bit pointer.
    pub fn map_from_pointer(quin: &NQuin) -> Option<Self> {
        use crate::QuinPointerExt;

        let flag = quin.extract_modality_flag();
        if flag != crate::MODALITY_FLAG_LLM_TENSOR {
            return None; // Not an LLM tensor
        }

        let offset = quin.extract_byte_offset();

        // Mock parsing the GGUF header at the offset to find shape and quantization
        // For demonstration, we assume a Q4_K tensor representation.
        Some(Self::new(vec![4096, 4096], offset, true))
    }
}

// ── gguf_bridge library submodules (extracted from the former 9k-line monolith) ──
// GPU uniform-buffer param structs (EmbeddingGpuParams / GemmGpuParams / AttentionGpuParams /
// ElemGpuParams + ELEM_OP_* codes) and the quant-support gates now live in dedicated files.
mod gpu_params;
mod quant_support;
pub(crate) use gpu_params::*;
pub(crate) use quant_support::*;

/// KV attention bitmask words uploaded to `fused_attention.wgsl` binding 5.
pub const KV_ATTENTION_MASK_WORDS: usize = crate::compute_universe::KV_ATTENTION_MASK_WORDS;

// ElemGpuParams + ELEM_OP_* codes moved to `gpu_params` (see submodule declarations above).

/// FNV-1a hash for bind group cache keys.
#[cfg(target_arch = "wasm32")]
#[inline]
pub(crate) fn mc8_bg_hash(parts: &[u64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &p in parts {
        h ^= p;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// MC8 Part 3s: WebGPU dynamic uniform offsets must be multiples of 256 bytes.
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_UNIFORM_ALIGN: usize = 256;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_MAX_GEMM_UNIFORM_SLOTS: usize = 8;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_MAX_ELEM_UNIFORM_SLOTS: usize = 8;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_MAX_ATTN_UNIFORM_SLOTS: usize = 8;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS: usize = MC8_MAX_ELEM_UNIFORM_SLOTS;
/// MC8 Part 3v / Phase 5.4: layers encoded into one submit batch. Sizes the per-layer uniform
/// buffers (slots_per_layer × this) and is the decode forward's chunk size — 64 → the whole
/// ≤64-layer forward is a single submit. Per-chunk flush + reset handles deeper models.
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_LAYERS_PER_ENCODER: u32 = 64;
/// Uniform slots reserved per layer within a chunk (must cover K/V/Q + tail).
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_ATTN_SLOTS_PER_LAYER: usize = 4;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_ELEM_SLOTS_PER_LAYER: usize = 6;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_GEMM_SLOTS_PER_LAYER: usize = 8; // o, gate, up, down + Phase 5.5 Q/K/V projection
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_MAX_ATTN_UNIFORM_CHUNK_SLOTS: usize =
    MC8_ATTN_SLOTS_PER_LAYER * MC8_LAYERS_PER_ENCODER as usize;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_MAX_ELEM_UNIFORM_CHUNK_SLOTS: usize =
    MC8_ELEM_SLOTS_PER_LAYER * MC8_LAYERS_PER_ENCODER as usize;
#[cfg(target_arch = "wasm32")]
pub(crate) const MC8_MAX_GEMM_UNIFORM_CHUNK_SLOTS: usize =
    MC8_GEMM_SLOTS_PER_LAYER * MC8_LAYERS_PER_ENCODER as usize;

/// Part 3v: absolute uniform slot cursors within one encoder chunk.
#[cfg(target_arch = "wasm32")]
pub(crate) struct Mc8ChunkUniformCursors {
    attn: usize,
    elem: usize,
    gemm: usize,
}

/// MC8 Part 3t: disjoint weight staging — eliminates `write_buffer` races within one layer submit.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mc8WeightRole {
    AttnK,
    AttnV,
    AttnQ,
    OProj,
    Gate,
    Up,
    Down,
}

#[cfg(target_arch = "wasm32")]
impl Mc8WeightRole {
    /// Stable index into the per-role resident stride table (`mc8_weight_role_stride`).
    #[inline]
    pub(crate) fn idx(self) -> usize {
        match self {
            Mc8WeightRole::AttnK => 0,
            Mc8WeightRole::AttnV => 1,
            Mc8WeightRole::AttnQ => 2,
            Mc8WeightRole::OProj => 3,
            Mc8WeightRole::Gate => 4,
            Mc8WeightRole::Up => 5,
            Mc8WeightRole::Down => 6,
        }
    }
}

/// One buffer per GEMM role so mid-layer weight uploads never clobber in-flight dispatches.
#[cfg(target_arch = "wasm32")]
pub(crate) struct Mc8WeightArenaBufs {
    qkv_k: wgpu::Buffer,
    qkv_v: wgpu::Buffer,
    qkv_q: wgpu::Buffer,
    o_proj: wgpu::Buffer,
    gate: wgpu::Buffer,
    up: wgpu::Buffer,
    down: wgpu::Buffer,
}

/// Part 3u: dynamic offsets for one full prefill layer (staged before encoder dispatches).
#[cfg(target_arch = "wasm32")]
pub(crate) struct Mc8PrefillLayerUniforms {
    k_off: u32,
    v_off: u32,
    q_off: u32,
    attn_norm_elem_off: Option<u32>,
    off_o: Option<u32>,
    off_attn_res: u32,
    off_ffn_norm: Option<u32>,
    off_gate: u32,
    off_up: u32,
    off_silu: u32,
    off_down: u32,
    off_ffn_res: u32,
    /// Phase 5.5: dynamic offsets for the Q/K/V projection GEMMs (parallel kernel).
    off_q_gemm: u32,
    off_k_gemm: u32,
    off_v_gemm: u32,
}

/// Strided work-buffer geometry shared by layer dispatches.
#[cfg(target_arch = "wasm32")]
pub(crate) struct Mc8PrefillLayerGeom {
    row_stride: usize,
    row_stride_u32: u32,
    batch_in_bytes: wgpu::BufferAddress,
    work_span_bytes: wgpu::BufferAddress,
    emb_bytes: wgpu::BufferAddress,
    n_embd_bytes: wgpu::BufferAddress,
    slot_o: wgpu::BufferAddress,
    slot_gate: wgpu::BufferAddress,
    slot_up: wgpu::BufferAddress,
    slot_save: wgpu::BufferAddress,
    slot_scratch_half: wgpu::BufferAddress,
    slot_o_f: u32,
    slot_save_f: u32,
    slot_gate_f: u32,
    slot_up_f: u32,
    slot_scratch_half_f: u32,
}

/// Stack arena for batched uniform uploads (one `write_buffer` per layer section).
#[cfg(target_arch = "wasm32")]
pub(crate) struct Mc8UniformArena {
    bytes: [u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
    slots: usize,
}

/// Part 3u: larger elem arena for full-layer super-staging.
#[cfg(target_arch = "wasm32")]
pub(crate) struct Mc8ElemUniformArena {
    bytes: [u8; MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS * MC8_UNIFORM_ALIGN],
    slots: usize,
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct Mc8AttnUniformArena {
    bytes: [u8; MC8_MAX_ATTN_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
    slots: usize,
}

#[cfg(target_arch = "wasm32")]
impl Mc8ElemUniformArena {
    pub(crate) fn push<T: bytemuck::Pod>(&mut self, value: &T) -> u32 {
        debug_assert!(std::mem::size_of::<T>() <= MC8_UNIFORM_ALIGN);
        debug_assert!(self.slots < MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS);
        let byte_off = self.slots * MC8_UNIFORM_ALIGN;
        self.slots += 1;
        self.bytes[byte_off..byte_off + std::mem::size_of::<T>()]
            .copy_from_slice(bytemuck::bytes_of(value));
        byte_off as u32
    }

    pub(crate) fn upload(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer) {
        if self.slots == 0 {
            return;
        }
        queue.write_buffer(buf, 0, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }

    pub(crate) fn upload_at(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer, base_slot: usize) {
        if self.slots == 0 {
            return;
        }
        if base_slot == 0 {
            self.upload(queue, buf);
            return;
        }
        let byte_off = (base_slot * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress;
        queue.write_buffer(buf, byte_off, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }
}

#[cfg(target_arch = "wasm32")]
impl Mc8AttnUniformArena {
    pub(crate) fn push<T: bytemuck::Pod>(&mut self, value: &T) -> u32 {
        debug_assert!(std::mem::size_of::<T>() <= MC8_UNIFORM_ALIGN);
        debug_assert!(self.slots < MC8_MAX_ATTN_UNIFORM_SLOTS);
        let byte_off = self.slots * MC8_UNIFORM_ALIGN;
        self.slots += 1;
        self.bytes[byte_off..byte_off + std::mem::size_of::<T>()]
            .copy_from_slice(bytemuck::bytes_of(value));
        byte_off as u32
    }

    pub(crate) fn upload(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer) {
        if self.slots == 0 {
            return;
        }
        queue.write_buffer(buf, 0, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }

    pub(crate) fn upload_at(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer, base_slot: usize) {
        if self.slots == 0 {
            return;
        }
        if base_slot == 0 {
            self.upload(queue, buf);
            return;
        }
        let byte_off = (base_slot * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress;
        queue.write_buffer(buf, byte_off, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }
}

#[cfg(target_arch = "wasm32")]
impl Mc8UniformArena {
    pub(crate) fn push<T: bytemuck::Pod>(&mut self, value: &T) -> u32 {
        debug_assert!(std::mem::size_of::<T>() <= MC8_UNIFORM_ALIGN);
        debug_assert!(self.slots < MC8_MAX_GEMM_UNIFORM_SLOTS);
        let byte_off = self.slots * MC8_UNIFORM_ALIGN;
        self.slots += 1;
        let dst = &mut self.bytes[byte_off..byte_off + std::mem::size_of::<T>()];
        dst.copy_from_slice(bytemuck::bytes_of(value));
        byte_off as u32
    }

    pub(crate) fn upload(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer) {
        if self.slots == 0 {
            return;
        }
        queue.write_buffer(buf, 0, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }

    pub(crate) fn upload_at(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer, base_slot: usize) {
        if self.slots == 0 {
            return;
        }
        let byte_off = (base_slot * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress;
        queue.write_buffer(buf, byte_off, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }
}

#[cfg(target_arch = "wasm32")]
impl Mc8ChunkUniformCursors {
    pub(crate) fn reset(&mut self) {
        self.attn = 0;
        self.elem = 0;
        self.gemm = 0;
    }

    pub(crate) fn attn_base_byte(&self) -> u32 {
        (self.attn * MC8_UNIFORM_ALIGN) as u32
    }

    pub(crate) fn elem_base_byte(&self) -> u32 {
        (self.elem * MC8_UNIFORM_ALIGN) as u32
    }

    pub(crate) fn gemm_base_byte(&self) -> u32 {
        (self.gemm * MC8_UNIFORM_ALIGN) as u32
    }
}

/// MC8: accumulates compute passes; submit + map_async only at pipeline boundary.
#[cfg(target_arch = "wasm32")]
pub(crate) struct WasmGpuPipeline {
    encoder: wgpu::CommandEncoder,
}

// ggml_gpu_quant_supported / ggml_gpu_attention_shader_supported / ggml_gpu_gemm_supported moved to
// the `quant_support` submodule (declared above; re-exported via `pub(crate) use quant_support::*`).

/// Await `map_async` without `poll(Wait)` — yields to the browser event loop (MC6).
#[cfg(target_arch = "wasm32")]
pub(crate) async fn await_wgpu_map(slice: wgpu::BufferSlice<'_>) -> bool {
    let (tx, rx) = futures_channel::oneshot::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    matches!(rx.await, Ok(Ok(())))
}

#[cfg(target_arch = "wasm32")]
impl WasmGpuPipeline {
    pub(crate) fn begin(engine: &QTensorEngine) -> Self {
        Self {
            encoder: engine
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("MC8FusedEncoder"),
                }),
        }
    }

    pub(crate) fn finish(self) -> wgpu::CommandBuffer {
        self.encoder.finish()
    }
}

// WASM-only MC8 GPU engine methods (resident weight arena, fused-encoder prefill/decode, async
// readback) carved into the `mc8_wasm` submodule. cfg-gated so native never compiles it.
#[cfg(target_arch = "wasm32")]
mod mc8_wasm;

/// Hard context ceiling — sized to keep KV arena under the 512MB RAM floor (Gemma 42L).
pub const MAX_CONTEXT_WINDOW: u32 = 1024;
/// Maximum bytes for the static KV arena (load-time allocation only).
pub const KV_CACHE_MAX_BYTES: usize = 448 * 1024 * 1024;

/// Static ring-buffer KV layout: `[layer][slot][K | V]` in f32, OR (W5a int8 mode) packed int8 +
/// per-(slot,kv_head) f32 scale in the same 4-byte-element buffer. `total_f32_elems` counts 4-byte
/// slots either way (u32/f32 share the size), so the allocation math is unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvCacheLayout {
    pub max_context: u32,
    pub n_layer: u32,
    pub n_kv_head: u32,
    pub head_dim: u32,
    pub slot_kv_elems: u32,
    pub layer_stride: u32,
    pub total_f32_elems: usize,
    /// W5a: true ⇒ the arena is int8-quantized (K/V stored as packed i8 lanes + one f32 scale per
    /// (slot, kv_head), K then V). `layer_stride` is then the int8 slot layout, not the f32 one.
    pub int8: bool,
    /// W5b Phase 4b: `> 0` ⇒ the arena stores k-sparse **dictionary codes** (`dict_k` code-words per
    /// K/V vector — each word packs `u16 atom-index | f16 coefficient`), reconstructed in the attention
    /// shader. Mutually exclusive with `int8` (dict mode wins). `0` ⇒ f32/int8 as above.
    pub dict_k: u32,
    /// W5b Phase 4b: dictionary size (atoms) — the atoms live in each layer's arena AFTER the code
    /// region (`[K atoms n_atoms×head_dim][V atoms …]`), so the per-layer binding covers them.
    pub dict_n_atoms: u32,
}

impl KvCacheLayout {
    pub fn from_hyperparams(h: &crate::gguf_sharder::GgufHyperparams) -> Option<Self> {
        // W5b Phase 4b: sparse-dictionary KV takes precedence over int8/f32 — but only when its toggle is
        // on AND a certified dictionary is installed whose head_dim matches this model. Otherwise it
        // transparently falls back (dict_k = 0).
        #[cfg(not(target_arch = "wasm32"))]
        let (dict_k, dict_n_atoms) = if crate::llm_bench::kv_dict_enabled() {
            crate::kv_dict_runtime::installed_meta()
                .filter(|&(_, hd, _)| hd == h.head_dim() as usize)
                .map(|(k, _, na)| (k as u32, na as u32))
                .unwrap_or((0, 0))
        } else {
            (0, 0)
        };
        #[cfg(target_arch = "wasm32")]
        let (dict_k, dict_n_atoms) = (0u32, 0u32);

        // W5a int8 KV is a native decode-path optimization, gated behind its own toggle and only when
        // head_dim packs cleanly into u32 lanes. WASM always uses the f32 layout.
        #[cfg(not(target_arch = "wasm32"))]
        let want_int8 = dict_k == 0
            && crate::llm_bench::kv_int8_enabled()
            && (h.head_dim() % 4 == 0)
            && h.head_dim() > 0;
        #[cfg(target_arch = "wasm32")]
        let want_int8 = false;
        Self::from_hyperparams_mode(h, want_int8, dict_k, dict_n_atoms)
    }

    fn from_hyperparams_mode(
        h: &crate::gguf_sharder::GgufHyperparams,
        int8: bool,
        dict_k: u32,
        dict_n_atoms: u32,
    ) -> Option<Self> {
        let n_layer = h.n_layer;
        let n_kv_head = h.effective_n_kv_head();
        let head_dim = h.head_dim();
        if n_layer == 0 || n_kv_head == 0 || head_dim == 0 {
            return None;
        }
        // dict mode wins over int8.
        let int8 = int8 && dict_k == 0;
        let slot_kv_elems = n_kv_head * head_dim;
        // f32:  per slot = 2·n_kv_head·head_dim 4-byte elems (K then V).
        // int8: per slot = 2·n_kv_head·(1 scale + head_dim/4 packed words), K then V — ~3.8× smaller.
        // dict: per slot = 2·n_kv_head·dict_k code-words (K then V) — each word = u16 index | f16 coeff.
        let layer_stride = if dict_k > 0 {
            // codes ([max_context][2 streams][n_kv_head][dict_k]) + the layer's dictionary atoms
            // ([2 streams][dict_n_atoms][head_dim]) resident in the same slice for the shader.
            MAX_CONTEXT_WINDOW * 2 * n_kv_head * dict_k + 2 * dict_n_atoms * head_dim
        } else if int8 {
            MAX_CONTEXT_WINDOW * 2 * n_kv_head * (1 + head_dim / 4)
        } else {
            MAX_CONTEXT_WINDOW * slot_kv_elems * 2
        };
        let total = (n_layer as usize).checked_mul(layer_stride as usize)?;
        let bytes = total.checked_mul(std::mem::size_of::<f32>())?;
        if bytes > KV_CACHE_MAX_BYTES {
            return None;
        }
        Some(Self {
            max_context: MAX_CONTEXT_WINDOW,
            n_layer,
            n_kv_head,
            head_dim,
            slot_kv_elems,
            layer_stride,
            total_f32_elems: total,
            int8,
            dict_k,
            dict_n_atoms,
        })
    }

    /// Derive the dense f32 device layout used by native CUDA kernels without changing the host
    /// cache representation. Host int8/dictionary compression and the CUDA execution arena are
    /// independent storage decisions.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn dense_device_layout(self) -> Option<Self> {
        let slot_kv_elems = self.n_kv_head.checked_mul(self.head_dim)?;
        let layer_stride = self
            .max_context
            .checked_mul(slot_kv_elems)?
            .checked_mul(2)?;
        let total_f32_elems = (self.n_layer as usize).checked_mul(layer_stride as usize)?;
        total_f32_elems
            .checked_mul(std::mem::size_of::<f32>())
            .filter(|bytes| *bytes <= KV_CACHE_MAX_BYTES)?;
        Some(Self {
            slot_kv_elems,
            layer_stride,
            total_f32_elems,
            int8: false,
            dict_k: 0,
            dict_n_atoms: 0,
            ..self
        })
    }

    #[inline]
    pub fn ring_slot(&self, token_idx: u32) -> u32 {
        token_idx % self.max_context
    }

    #[inline]
    pub fn k_index(&self, layer: u32, slot: u32, kv_head: u32, dim: u32) -> usize {
        let base = layer as usize * self.layer_stride as usize
            + slot as usize * self.slot_kv_elems as usize * 2;
        base + kv_head as usize * self.head_dim as usize + dim as usize
    }

    #[inline]
    pub fn v_index(&self, layer: u32, slot: u32, kv_head: u32, dim: u32) -> usize {
        let k_base = layer as usize * self.layer_stride as usize
            + slot as usize * self.slot_kv_elems as usize * 2;
        let v_off = self.n_kv_head as usize * self.head_dim as usize;
        k_base + v_off + kv_head as usize * self.head_dim as usize + dim as usize
    }

    /// W5b Phase 4b (dict mode): word offset of the `i`-th code word (`0..dict_k`) for the K
    /// (`k_not_v = true`) or V vector of `(layer, slot, kv_head)`. Each code word packs
    /// `u16 atom-index (high 16) | f16 coefficient (low 16)`. Slot layout mirrors f32 (K region then V
    /// region), but each vector is `dict_k` words instead of `head_dim` floats.
    #[inline]
    pub fn code_index(&self, layer: u32, slot: u32, kv_head: u32, k_not_v: bool, i: u32) -> usize {
        let dk = self.dict_k as usize;
        let per_slot = 2 * self.n_kv_head as usize * dk;
        let base = layer as usize * self.layer_stride as usize + slot as usize * per_slot;
        let stream_off = if k_not_v {
            0
        } else {
            self.n_kv_head as usize * dk
        };
        base + stream_off + kv_head as usize * dk + i as usize
    }
}

/// Max GEMM row/column for stack buffers and reusable GPU staging (Gemma 4 FFN = 4×2560).
const MAX_STACK_GEMM_DIM: usize = 10240;
const MAX_STACK_GEMM_OUT: usize = MAX_STACK_GEMM_DIM;
const MAX_STACK_GEMM_IN: usize = MAX_STACK_GEMM_DIM;
/// Stack scratch for pre-norm hidden (SmolLM2 n_embd=960; cap supports Gemma-class models).
const MAX_HIDDEN_DIM: usize = 4096;
/// RMSNorm epsilon when GGUF KV does not expose `rms_norm_eps` (Llama/SmolLM default).
pub(crate) const RMS_NORM_EPS: f32 = 1e-5;
/// Prompt tokens per prefill GPU batch (stack + staging footprint = `emb_dim ×` this).
pub const PREFILL_CHUNK_SIZE: usize = 64;
/// Per-token KV masks uploaded for batched Q-SDPA (`PREFILL_CHUNK_SIZE × mask words`).
const MAX_ATTN_MASK_UPLOAD_WORDS: usize = PREFILL_CHUNK_SIZE * KV_ATTENTION_MASK_WORDS;
/// Max stacked embedding floats in a prefill chunk (`MAX_STACK_GEMM_IN × 64`).
pub const MAX_PREFILL_BATCH_FLOATS: usize = MAX_STACK_GEMM_IN * PREFILL_CHUNK_SIZE;
/// `llm_agent` stack chunk buffer (Gemma 2560 × 64).
pub const PREFILL_CHUNK_STACK_FLOATS: usize = 2560 * PREFILL_CHUNK_SIZE;
/// wgpu default max buffer size on many drivers (256 MiB).
const MAX_WGPU_WEIGHT_STAGING: usize = 64 * 1024 * 1024;
/// Vocabulary projection rows per chunked logits sweep.
/// 10240 is the native GEMM output-buffer ceiling and a 256-row multiple, so
/// resident logits chunk offsets stay storage-binding aligned while the current
/// 49k-vocab model drops from six output chunks/token to five.
pub const VOCAB_CHUNK_ROWS: usize = MAX_STACK_GEMM_OUT;

/// Streaming argmax result across chunked vocabulary projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamingArgmaxResult {
    pub best_token_id: u32,
    pub max_logit: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GgufLoadReport {
    pub mapped_bytes: u64,
    pub tensor_data_offset: u64,
    pub n_layer: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub max_tensor_bytes: usize,
    pub kv_cache_bytes: u64,
    pub directml_enabled: bool,
}

// CPU numeric kernels + pre-norm helpers (bytes_to_gib / scrub_f32_volatile / update_streaming_argmax
// [+sieved] / relu / silu / add_residual / rms_norm / dequant_norm_row_into / prepare_pre_norm_input)
// moved to the `cpu_ops` submodule (declared below; re-exported via `pub(crate) use cpu_ops::*`).
mod cpu_ops;
pub(crate) use cpu_ops::*;
#[cfg(not(target_arch = "wasm32"))]
mod pipeline_cache;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use pipeline_cache::*;

// Concern submodules — each holds an `impl QTensorEngine` block for one hot-path area. Methods are
// pub(crate) so they call across modules freely; types/imports arrive via each file's `use super::*`.
mod async_dispatch;
mod attention;
#[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
mod cuda_decode_plan;
/// Hard cap on the KV context window a decode plan may request. Declared here rather than in the
/// `cuda`-gated plan module because the raw-decode harness and the mega-pass guard validate
/// against it on every target, CUDA or not.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const MAX_CUDA_CONTEXT_WINDOW: u32 = 4096;
mod embedding;
mod ffn;
mod forward;
mod gemm;
mod init;
mod load;
/// Cooperative browser yields + init-status for WASM LLM boot (phones).
#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm_yield;
mod output;
#[cfg(not(target_arch = "wasm32"))]
mod prefill_arena;
mod prefill_async;
#[cfg(not(target_arch = "wasm32"))]
mod resident_decode;
mod verify_arena;

/// MC8 pt3e: max abs error over the first `n` elements.
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
fn probe_max_abs_diff(a: &[f32], b: &[f32], n: usize) -> f32 {
    let n = n.min(a.len()).min(b.len());
    let mut m = 0.0f32;
    for i in 0..n {
        m = m.max((a[i] - b[i]).abs());
    }
    m
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
fn probe_log_diff(phase: &str, cpu: &[f32], gpu: &[f32], n: usize) {
    let n = n.min(8).min(cpu.len()).min(gpu.len());
    if n == 0 {
        return;
    }
    let err = probe_max_abs_diff(cpu, gpu, n);
    wlog(&format!(
        "[MC8 L0 diff] {phase}: cpu[0]={:.6} gpu[0]={:.6} max_abs_err={:.6}",
        cpu[0], gpu[0], err
    ));
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
fn probe_log_mid_diff(phase: &str, cpu: &[f32], gpu: &[f32], n: usize) {
    let n = n.min(8).min(cpu.len()).min(gpu.len());
    if n == 0 {
        return;
    }
    let err = probe_max_abs_diff(cpu, gpu, n);
    wlog(&format!(
        "[MC8 L0 mid] {phase}: cpu[0]={:.6} gpu[0]={:.6} max_abs_err={:.6}",
        cpu[0], gpu[0], err
    ));
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
fn probe_log_ffn_diff(phase: &str, cpu: &[f32], gpu: &[f32], n: usize) {
    let n = n.min(8).min(cpu.len()).min(gpu.len());
    if n == 0 {
        return;
    }
    let err = probe_max_abs_diff(cpu, gpu, n);
    wlog(&format!(
        "[MC8 L0 ffn] {phase}: cpu[0]={:.6} gpu[0]={:.6} max_abs_err={:.6}",
        cpu[0], gpu[0], err
    ));
}

/// MC8 pt3g: CPU SwiGLU stages from post-attn hidden @ L0.
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
async fn mc8_cpu_l0_ffn_stages(
    engine: &QTensorEngine,
    index: &crate::gguf_sharder::GgufTensorIndex,
    mmap: &[u8],
    layout: &KvCacheLayout,
    hidden_cpu: &[f32],
    n_embd: usize,
    token_idx: u32,
    post_attn: &mut [f32],
    ffn_input: &mut [f32],
    gate: &mut [f32],
    up: &mut [f32],
    swiglu: &mut [f32],
    down: &mut [f32],
) -> Option<usize> {
    let tensors = index.get_layer_tensors(0);
    let mut attn_out = [0f32; MAX_HIDDEN_DIM];
    let q_dim = mc8_cpu_l0_attn_out(
        engine,
        index,
        mmap,
        layout,
        hidden_cpu,
        n_embd,
        token_idx,
        &mut attn_out,
    )
    .await?;
    let out_info = tensors.attn_output.as_ref()?;
    let o_raw =
        crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, out_info).ok()?;
    let (o_in, _) = QTensorEngine::matmul_dims(out_info);
    if o_in > q_dim {
        return None;
    }
    let mut o_proj = [0f32; MAX_HIDDEN_DIM];
    if !stack_gemm_quant(
        o_raw,
        out_info,
        &attn_out[..q_dim],
        &mut o_proj[..n_embd],
        o_in,
        n_embd,
    ) {
        return None;
    }
    for i in 0..n_embd {
        post_attn[i] = hidden_cpu[i] + o_proj[i];
    }
    let mut norm_w = [0f32; MAX_HIDDEN_DIM];
    let mut ffn_norm_scratch = [0f32; MAX_HIDDEN_DIM];
    let normed = prepare_pre_norm_input(
        &post_attn[..n_embd],
        n_embd,
        tensors.ffn_norm.as_ref(),
        Some(mmap),
        index.tensor_data_start,
        &mut ffn_norm_scratch,
        &mut norm_w,
    );
    ffn_input[..n_embd].copy_from_slice(normed);
    let gate_info = tensors.ffn_gate.as_ref()?;
    let up_info = tensors.ffn_up.as_ref()?;
    let down_info = tensors.ffn_down.as_ref()?;
    let gate_raw =
        crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info).ok()?;
    let up_raw =
        crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info).ok()?;
    let down_raw =
        crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info).ok()?;
    let (gate_in, n_ffn) = QTensorEngine::matmul_dims(gate_info);
    let (up_in, up_out) = QTensorEngine::matmul_dims(up_info);
    let (dn_in, dn_out) = QTensorEngine::matmul_dims(down_info);
    if gate_in > n_embd
        || up_in != gate_in
        || up_out != n_ffn
        || dn_in != n_ffn
        || n_ffn > gate.len()
        || dn_out < n_embd
    {
        return None;
    }
    if !stack_gemm_quant(
        gate_raw,
        gate_info,
        normed,
        &mut gate[..n_ffn],
        gate_in,
        n_ffn,
    ) {
        return None;
    }
    if !stack_gemm_quant(up_raw, up_info, normed, &mut up[..n_ffn], up_in, n_ffn) {
        return None;
    }
    for i in 0..n_ffn {
        let g = gate[i];
        let silu = g / (1.0 + (-g).exp());
        swiglu[i] = silu * up[i];
    }
    if !stack_gemm_quant(
        down_raw,
        down_info,
        &swiglu[..dn_in],
        &mut down[..n_embd],
        dn_in,
        n_embd,
    ) {
        return None;
    }
    Some(n_ffn)
}

/// Read one KV head from the CPU mirror arena.
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
fn read_kv_cpu_head(
    layout: &KvCacheLayout,
    kv: &[f32],
    layer: u32,
    token_pos: u32,
    kv_h: u32,
    head_dim: usize,
    k_not_v: bool,
    out: &mut [f32],
) -> bool {
    if head_dim == 0 || head_dim > out.len() {
        return false;
    }
    let slot = layout.ring_slot(token_pos);
    for d in 0..head_dim {
        let idx = if k_not_v {
            layout.k_index(layer, slot, kv_h, d as u32)
        } else {
            layout.v_index(layer, slot, kv_h, d as u32)
        };
        if idx >= kv.len() {
            return false;
        }
        out[d] = kv[idx];
    }
    true
}

#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
fn probe_log_prefill_diff(phase: &str, cpu: &[f32], gpu: &[f32], n: usize) {
    let n = n.min(8).min(cpu.len()).min(gpu.len());
    if n == 0 {
        return;
    }
    let err = probe_max_abs_diff(cpu, gpu, n);
    wlog(&format!(
        "[MC8 prefill] {phase}: cpu[0]={:.6} gpu[0]={:.6} max_abs_err={:.6}",
        cpu[0], gpu[0], err
    ));
}

/// MC8 pt3f: CPU SDPA @ L0 → full `q_dim` Attn_Out (async KV readback).
#[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
async fn mc8_cpu_l0_attn_out(
    engine: &QTensorEngine,
    index: &crate::gguf_sharder::GgufTensorIndex,
    mmap: &[u8],
    layout: &KvCacheLayout,
    hidden_cpu: &[f32],
    n_embd: usize,
    token_idx: u32,
    out: &mut [f32],
) -> Option<usize> {
    let h = &index.hyperparams;
    let tensors = index.get_layer_tensors(0);
    let head_dim = h.head_dim() as usize;
    let n_head = h.n_head as usize;
    let q_heads_per_kv = h.q_heads_per_kv() as usize;
    if head_dim == 0 || n_head == 0 || q_heads_per_kv == 0 {
        return None;
    }
    let q_dim = n_head * head_dim;
    if q_dim > out.len() {
        return None;
    }
    let mut norm_w = [0f32; MAX_HIDDEN_DIM];
    let mut norm_cpu = [0f32; MAX_HIDDEN_DIM];
    let mut proj = [0f32; MAX_STACK_GEMM_OUT];
    let normed = prepare_pre_norm_input(
        &hidden_cpu[..n_embd],
        n_embd,
        tensors.attn_norm.as_ref(),
        Some(mmap),
        index.tensor_data_start,
        &mut norm_cpu,
        &mut norm_w,
    );
    let q_info = tensors.attn_q.as_ref()?;
    let q_raw =
        crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info).ok()?;
    let (q_in, q_out) = QTensorEngine::matmul_dims(q_info);
    if q_out != q_dim || !stack_gemm_quant(q_raw, q_info, normed, &mut proj[..q_out], q_in, q_out) {
        return None;
    }
    rope_inplace(
        &mut proj[..q_out],
        n_head,
        head_dim,
        token_idx,
        h.effective_rope_freq_base(),
        h.effective_rope_scale(),
    );
    let scale = 1.0f32 / (head_dim as f32).sqrt();
    let mut k_slot = [0f32; 128];
    let mut v_slot = [0f32; 128];
    if head_dim > k_slot.len() {
        return None;
    }
    for qh in 0..n_head {
        let kv_h = qh / q_heads_per_kv;
        let q_off = qh * head_dim;
        let mut att_scores = [0f32; MAX_CONTEXT_WINDOW as usize];
        let mut max_score = f32::NEG_INFINITY;
        for past_pos in 0..=token_idx {
            let past_slot = layout.ring_slot(past_pos);
            if !engine
                .pipeline_read_kv_head(
                    layout,
                    0,
                    past_slot,
                    kv_h as u32,
                    head_dim,
                    true,
                    &mut k_slot,
                )
                .await
            {
                return None;
            }
            let mut dot = 0.0f32;
            for d in 0..head_dim {
                dot += proj[q_off + d] * k_slot[d];
            }
            let score = dot * scale;
            att_scores[past_pos as usize] = score;
            max_score = max_score.max(score);
        }
        let mut sum_exp = 0.0f32;
        for past_pos in 0..=token_idx {
            let exp_val = (att_scores[past_pos as usize] - max_score).exp();
            att_scores[past_pos as usize] = exp_val;
            sum_exp += exp_val;
        }
        if sum_exp > 0.0 {
            for past_pos in 0..=token_idx {
                let prob = att_scores[past_pos as usize] / sum_exp;
                let past_slot = layout.ring_slot(past_pos);
                if !engine
                    .pipeline_read_kv_head(
                        layout,
                        0,
                        past_slot,
                        kv_h as u32,
                        head_dim,
                        false,
                        &mut v_slot,
                    )
                    .await
                {
                    return None;
                }
                for d in 0..head_dim {
                    out[q_off + d] += v_slot[d] * prob;
                }
            }
        }
    }
    Some(q_dim)
}

// --- PHASE 1 WASM OOB DIAGNOSTIC INSTRUMENTATION (remove once the trap is fixed) ---
#[cfg(target_arch = "wasm32")]
#[inline]
pub(crate) fn wlog(s: &str) {
    web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(s));
}
#[cfg(not(target_arch = "wasm32"))]
#[inline]
pub(crate) fn wlog(_s: &str) {}

/// Decode-profiler: count of GPU `submit → poll(Maintain::Wait)` round-trips. Incremented by
/// `QTensorEngine::poll_wait` (every native blocking sync point routes through it); read/reset by
/// the bench to derive per-token synchronization overhead.
pub static GPU_WAIT_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Total native GPU blocking-wait round-trips since the last reset.
#[inline]
pub fn gpu_wait_count() -> u64 {
    GPU_WAIT_COUNT.load(std::sync::atomic::Ordering::Relaxed)
}

/// Reset the GPU blocking-wait counter before a measured run.
#[inline]
pub fn reset_gpu_wait_count() {
    GPU_WAIT_COUNT.store(0, std::sync::atomic::Ordering::Relaxed);
}

/// In-place NEOX-style RoPE over `n_heads` consecutive `head_dim` blocks of `vec`.
/// Rotates split-half pairs `(i, i + head_dim/2)` — required for Llama/SmolLM2 GGUF weights.
/// (`fused_attention.wgsl` mirrors this NEOX split-half layout since MC8 Part 2.)
fn rope_inplace(vec: &mut [f32], n_heads: usize, head_dim: usize, pos: u32, base: f32, scale: f32) {
    let half = head_dim / 2;
    if half == 0 {
        return;
    }
    let scale = if scale > 0.0 && scale.is_finite() {
        scale
    } else {
        1.0
    };
    let scaled_pos = pos as f32 / scale;
    for head in 0..n_heads {
        let off = head * head_dim;
        if off + head_dim > vec.len() {
            return;
        }
        for i in 0..half {
            // Interleaved ("normal"/llama) rope: pair adjacent dims (2i, 2i+1). GGUF llama-arch
            // (SmolLM2) is is_neox=false — weights are permuted for interleaved, NOT split-half.
            let theta = scaled_pos * base.powf(-2.0 * i as f32 / head_dim as f32);
            let (s, c) = theta.sin_cos();
            let x0 = vec[off + 2 * i];
            let x1 = vec[off + 2 * i + 1];
            vec[off + 2 * i] = x0 * c - x1 * s;
            vec[off + 2 * i + 1] = x0 * s + x1 * c;
        }
    }
}

/// STEM-grounding proof: the LLM's `rope_inplace` is the 2-D rotation defined in
/// `solvers::rope` — RoPE is trigonometry (a rotation per dimension pair), not a proprietary
/// operation. The inline `f32` kernel is checked against the `f64` STEM definition.
#[cfg(test)]
mod rope_stem_parity_tests {
    #[test]
    fn rope_kernel_is_the_stem_rotation() {
        let n_heads = 2usize;
        let head_dim = 8usize;
        let (pos, base, scale) = (7u32, 10000.0f32, 1.0f32);
        let xs: Vec<f32> = (0..n_heads * head_dim)
            .map(|i| (i as f32 - 8.0) * 0.25)
            .collect();

        let mut got = xs.clone();
        super::rope_inplace(&mut got, n_heads, head_dim, pos, base, scale);

        let mut want: Vec<f64> = xs.iter().map(|&v| v as f64).collect();
        crate::solvers::rope::rope_interleaved(
            &mut want,
            n_heads,
            head_dim,
            pos as f64,
            base as f64,
            scale as f64,
        );

        for i in 0..xs.len() {
            assert!(
                (got[i] as f64 - want[i]).abs() < 1e-4,
                "RoPE kernel diverges from solvers::rope at {i}: {} vs {}",
                got[i],
                want[i]
            );
        }
    }
}

/// Zero-heap CPU GEMM: `out[i] = dot(weight_row(i), input)` with per-row dequant.
/// `pub(crate)` so toolkit/parity probes can exercise the same kernel as the hot path.
pub(crate) fn stack_gemm_quant(
    raw: &[u8],
    info: &GgufTensorInfo,
    input: &[f32],
    out: &mut [f32],
    n_in: usize,
    n_out: usize,
) -> bool {
    if n_in > input.len() || n_out > out.len() || n_in > MAX_STACK_GEMM_IN {
        wlog(&format!(
            "[stack_gemm] GUARD tripped n_in={n_in} n_out={n_out} input={} out={} MAX_IN={MAX_STACK_GEMM_IN}",
            input.len(),
            out.len()
        ));
        return false;
    }
    let mut row = [0f32; MAX_STACK_GEMM_IN];
    for i in 0..n_out {
        if crate::ggml_quants::dequant_matrix_row_into(raw, info, i, &mut row[..n_in]).unwrap_or(0)
            < n_in
        {
            return false;
        }
        out[i] = row[..n_in]
            .iter()
            .zip(&input[..n_in])
            .map(|(w, x)| w * x)
            .sum();
    }
    true
}

pub struct QTensorEngine {
    /// Browser WASM keeps a private device; native reuses `gpu_context::shared_gpu()`.
    #[cfg(target_arch = "wasm32")]
    device: wgpu::Device,
    #[cfg(target_arch = "wasm32")]
    queue: wgpu::Queue,
    pub pipeline: wgpu::ComputePipeline,
    /// WASM multi-row Q8_0 GEMV (llama.cpp-style: 64 thr, 4 rows/WG, u32 packed reads).
    #[cfg(target_arch = "wasm32")]
    pub mmv_q8_0_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    native_pipeline_cache: Option<wgpu::PipelineCache>,
    #[cfg(not(target_arch = "wasm32"))]
    pipeline_bind_layout: wgpu::BindGroupLayout,
    /// 0.0.21: cooperative GEMV (one workgroup per output row, shared-memory reduction). Same shader
    /// MODULE as `pipeline`, entry point `coop_gemv` / `coop_gemv_sg`. Selected per-call when
    /// `llm_bench::coop_gemv_enabled()`. Native only (the wasm decode path is the MC8 arena).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) coop_gemv_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    coop_gemv_bind_layout: wgpu::BindGroupLayout,
    /// Multi-row coop GEMV (8 rows/WG) for Q4_K_SOA large n_out — see `coop_gemv_mr`.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) coop_gemv_mr_pipeline: wgpu::ComputePipeline,
    /// GEMV + residual add in one dispatch (O-proj / down-proj in resident mega-pass).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) coop_gemv_residual_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    coop_gemv_residual_bind_layout: wgpu::BindGroupLayout,
    /// Multi-row residual GEMV for Q4_K_SOA.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) coop_gemv_residual_mr_pipeline: wgpu::ComputePipeline,
    /// Warp GEMV (32 thr/row) for Q4_K_SOA.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) coop_gemv_warp_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) coop_gemv_residual_warp_pipeline: wgpu::ComputePipeline,
    /// Legacy f32×f32 mock block for offset-0 `QTensor` fallback (no mmap).
    #[cfg(not(target_arch = "wasm32"))]
    mock_pipeline: wgpu::ComputePipeline,
    /// GPU-side Q6_K embedding dequant + matmul (zero CPU dequant).
    pub embedding_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    embedding_bind_layout: wgpu::BindGroupLayout,
    pub is_initialized: bool,
    /// DirectML device — Some on Windows when DirectML 1.15 is linked.
    #[cfg(target_os = "windows")]
    pub dml: Option<crate::directml_bridge::DmlDevice>,
    /// Memory-mapped GGUF file (set after `load_gguf`).
    #[cfg(not(target_arch = "wasm32"))]
    pub gguf_mmap: Option<Arc<memmap2::Mmap>>,
    #[cfg(target_arch = "wasm32")]
    pub gguf_mmap: Option<Arc<[u8]>>,
    /// WASM: cached tokenizer extracted from gguf_mmap before dropping it.
    #[cfg(target_arch = "wasm32")]
    pub cached_tokenizer: Option<crate::gguf_sharder::GgufTokenizer>,
    /// WASM: cached tensor index extracted from gguf_mmap before dropping it.
    #[cfg(target_arch = "wasm32")]
    pub cached_tensor_index: Option<crate::gguf_sharder::GgufTensorIndex>,
    /// WASM: raw bytes of the token_embd tensor (for embedding lookup after dropping gguf_mmap).
    #[cfg(target_arch = "wasm32")]
    pub cached_token_embd: Option<Arc<[u8]>>,
    /// Resident P64 container bytes.
    #[cfg(target_arch = "wasm32")]
    pub p64_resident: Option<Arc<[u8]>>,
    /// Cached P64 index after `adopt_resident_p64_*` — decode must not re-CRC the container.
    #[cfg(not(target_arch = "wasm32"))]
    pub p64_index: Option<crate::p64_weight::P64TensorIndex>,
    /// Cached synthetic GGUF index built from `p64_index` (or from GGUF parse).
    #[cfg(not(target_arch = "wasm32"))]
    pub tensor_index_cache: Option<crate::gguf_sharder::GgufTensorIndex>,

    /// Byte offset into the mmap where tensor data begins.
    pub tensor_data_offset: u64,
    pub hyperparams: crate::gguf_sharder::GgufHyperparams,
    pub max_tensor_bytes: usize,
    /// Reused layer staging buffers (one layer in VRAM at a time).
    gemm_input_buf: Option<wgpu::Buffer>,
    gemm_weight_buf: Option<wgpu::Buffer>,
    /// MC8 Part 3t: disjoint per-role weight arena (prefill single-submit).
    #[cfg(target_arch = "wasm32")]
    mc8_weight_arena: Option<Mc8WeightArenaBufs>,
    /// MC8 Part 3x: when set, the 7 role buffers hold ALL layers' weights (uploaded once);
    /// hot-path encoders bind a per-layer sub-range instead of re-`write_buffer`ing per forward.
    #[cfg(target_arch = "wasm32")]
    mc8_weights_resident: bool,
    /// Per-role 256-byte-aligned per-layer stride (bytes), indexed by `Mc8WeightRole::idx()`.
    #[cfg(target_arch = "wasm32")]
    mc8_weight_role_stride: [u64; 7],
    /// Legacy decode-path ping-pong (decode tail not on weight arena yet).
    #[cfg(target_arch = "wasm32")]
    gemm_weight_buf_b: Option<wgpu::Buffer>,
    gemm_output_buf: Option<wgpu::Buffer>,
    gemm_params_buf: Option<wgpu::Buffer>,
    gemm_output_staging: Option<wgpu::Buffer>,
    // A1a (STELLAR §A): persistent GPU top-k output-projection pipeline + small candidate buffers.
    // Lets the output logits stay on-GPU (top-k over them, read back only K pairs) instead of the
    // 196 KB/token full-logit readback. Created once in `ensure_gemm_buffers`.
    #[cfg(not(target_arch = "wasm32"))]
    output_topk_pipeline: Option<wgpu::ComputePipeline>,
    #[cfg(not(target_arch = "wasm32"))]
    output_topk_bind_layout: Option<wgpu::BindGroupLayout>,
    #[cfg(not(target_arch = "wasm32"))]
    topk_cand_val_buf: Option<wgpu::Buffer>,
    #[cfg(not(target_arch = "wasm32"))]
    topk_cand_idx_buf: Option<wgpu::Buffer>,
    #[cfg(not(target_arch = "wasm32"))]
    topk_cand_staging: Option<wgpu::Buffer>,
    #[cfg(not(target_arch = "wasm32"))]
    topk_params_buf: Option<wgpu::Buffer>,
    /// MC8 FFN / attention scratch (gate, up, o_proj).
    gemm_aux_buf: Option<wgpu::Buffer>,
    /// MC8 SwiGLU up-projection scratch (cannot alias gemm_output/work — in-place GEMM invalid).
    gemm_ffn_buf: Option<wgpu::Buffer>,
    /// Batched prefill RMS output (same span as `gemm_input_buf`; avoids in-place on batch_buf).
    #[cfg(target_arch = "wasm32")]
    prefill_scratch_buf: Option<wgpu::Buffer>,
    /// Strided prefill ping-pong rows (`PREFILL_CHUNK_SIZE × row_stride` floats each).
    #[cfg(target_arch = "wasm32")]
    prefill_work_buf_a: Option<wgpu::Buffer>,
    #[cfg(target_arch = "wasm32")]
    prefill_work_buf_b: Option<wgpu::Buffer>,
    /// Phase 5.5: Q/K/V projection scratch (parallel-GEMM output → lightweight attention shader).
    #[cfg(target_arch = "wasm32")]
    mc8_q_proj_buf: Option<wgpu::Buffer>,
    #[cfg(target_arch = "wasm32")]
    mc8_k_proj_buf: Option<wgpu::Buffer>,
    #[cfg(target_arch = "wasm32")]
    mc8_v_proj_buf: Option<wgpu::Buffer>,
    gemm_max_out_dim: u32,
    gemm_max_input_floats: usize,
    /// Static KV ring-buffer (allocated once at `load_gguf`).
    kv_layout: Option<KvCacheLayout>,
    kv_cache_gpu: Option<wgpu::Buffer>,
    /// CPU mirror for quantized-attention fallback (no growth during decode).
    kv_cache_cpu: Option<Box<[f32]>>,
    attention_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    attention_bind_layout: wgpu::BindGroupLayout,
    attention_params_buf: Option<wgpu::Buffer>,
    attention_mask_buf: Option<wgpu::Buffer>,
    /// MC8 elementwise GPU ops (RMSNorm / SiLU×mul / residual).
    elem_rms_norm_pipeline: wgpu::ComputePipeline,
    elem_silu_mul_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    elem_silu_mul_bind_layout: wgpu::BindGroupLayout,
    elem_add_residual_pipeline: wgpu::ComputePipeline,
    elem_params_buf: Option<wgpu::Buffer>,
    norm_weight_buf: Option<wgpu::Buffer>,
    /// MC8 Part 3s: dynamic-offset bind group layouts (uniform race elimination).
    #[cfg(target_arch = "wasm32")]
    mc8_gemm_bind_layout: wgpu::BindGroupLayout,
    #[cfg(target_arch = "wasm32")]
    mc8_elem_bind_layout: wgpu::BindGroupLayout,
    #[cfg(target_arch = "wasm32")]
    mc8_attn_bind_layout: wgpu::BindGroupLayout,
    /// Phase 5 dispatch fusion: SwiGLU expansion (gate · SiLU · up) collapsed into one pass.
    #[cfg(target_arch = "wasm32")]
    mc8_ffn_fused_bind_layout: wgpu::BindGroupLayout,
    #[cfg(target_arch = "wasm32")]
    mc8_ffn_fused_pipeline: wgpu::ComputePipeline,
    /// Native T-A1: same fused FFN expansion, static uniform (resident mega-pass).
    #[cfg(not(target_arch = "wasm32"))]
    ffn_fused_bind_layout: wgpu::BindGroupLayout,
    /// Naive 64-thread/row fused expansion (wasm-style; fallback).
    #[cfg(not(target_arch = "wasm32"))]
    ffn_fused_pipeline: wgpu::ComputePipeline,
    /// T-A1b: coop 256-thread/row fused expansion (preferred when coop GEMV is on).
    #[cfg(not(target_arch = "wasm32"))]
    ffn_fused_coop_pipeline: wgpu::ComputePipeline,
    /// Multi-row fused FFN (4 rows/WG) for Q4_K_SOA.
    #[cfg(not(target_arch = "wasm32"))]
    ffn_fused_mr_pipeline: wgpu::ComputePipeline,
    /// Warp fused FFN (32 thr/row) for Q4_K_SOA.
    #[cfg(not(target_arch = "wasm32"))]
    ffn_fused_warp_pipeline: wgpu::ComputePipeline,
    /// Dual K+V GEMV (shared act) for resident mega-pass.
    #[cfg(not(target_arch = "wasm32"))]
    dual_gemv_pipeline: wgpu::ComputePipeline,
    /// Dual multi-row (4 rows/WG) — default for SoA K+V.
    #[cfg(not(target_arch = "wasm32"))]
    dual_gemv_mr_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    dual_gemv_bind_layout: wgpu::BindGroupLayout,
    /// Triple Q+K+V GEMV (shared act, GQA-safe) — one dispatch replaces dual+Q.
    #[cfg(not(target_arch = "wasm32"))]
    triple_gemv_pipeline: wgpu::ComputePipeline,
    #[cfg(not(target_arch = "wasm32"))]
    triple_gemv_bind_layout: wgpu::BindGroupLayout,
    /// Phase 5.3: the output/logits projection (tied `token_embd`, ~50 MB) uploaded to VRAM
    /// once at init so the per-token argmax binds resident sub-ranges instead of re-uploading
    /// the whole matrix every token (the decode throughput killer). A1a step-2 ports this to the
    /// native top-k decode path, so these two fields are available on both targets.
    mc8_logits_resident_buf: Option<wgpu::Buffer>,
    mc8_logits_row_bytes: u32,
    /// A1b (STELLAR §A): resident 2-bit ternary-FFN GEMM dispatcher, built once at P64 boot from
    /// the container's base-3 FFN blobs (rebaked to 2-bit, uploaded once). `None` until a ternary
    /// P64 is adopted; the FFN dispatch branch (`dispatch_ternary_ffn`) uses it when present +
    /// the toggle is on, else the CPU oracle. Native-only; the wasm ternary path is a later step.
    #[cfg(not(target_arch = "wasm32"))]
    ternary_ffn: Option<crate::ternary_gpu::TernaryFfnResident>,
    /// Phase 2 (resident weights): resident VRAM weight buffers, keyed by each weight byte-region's
    /// absolute mmap address (unique per distinct weight — incl. each output-projection vocab chunk,
    /// which all share one tensor `byte_offset` — and stable across tokens). Populated lazily on the
    /// first GEMM that touches a region and reused every token, so a weight is uploaded to VRAM ONCE
    /// instead of re-`write_buffer`ed (up to ~50 MB for a 3B FFN tensor) on every GEMM, every token —
    /// the decode-bandwidth lever for large models. Mmap bytes are immutable, so the cache is always
    /// coherent. Native-only (wasm uses the MC8 arena); active when `resident_weights_enabled()`.
    #[cfg(not(target_arch = "wasm32"))]
    gemm_resident_weights: std::sync::Mutex<std::collections::HashMap<u64, wgpu::Buffer>>,
    /// Phase 3 (FFN fusion): a small uniform buffer holding the gate/up/down GEMM `GemmGpuParams`
    /// at 256-aligned sub-ranges (3 slots), so all three GEMMs of one fused FFN submit can bind
    /// distinct params simultaneously. Lazily created native-only on the first fused FFN.
    #[cfg(not(target_arch = "wasm32"))]
    ffn_fused_params: Option<wgpu::Buffer>,
    /// Native attention preproject fusion: two 256-byte-aligned GEMM uniform slots
    /// (K,V) and two attention uniform slots (K-write,V-write), allowing K/V
    /// projection + KV-cache writes to share one submit without uniform races.
    #[cfg(not(target_arch = "wasm32"))]
    attention_kv_gemm_params: Option<wgpu::Buffer>,
    #[cfg(not(target_arch = "wasm32"))]
    attention_kv_params: Option<wgpu::Buffer>,
    /// Phase 5.4: all layers' attn_norm + ffn_norm weights resident (slot 2L = attn, 2L+1 = ffn),
    /// so RMSNorm binds a per-layer sub-range instead of re-`write_buffer`ing a shared single-layer
    /// `norm_weight_buf` every layer (the second per-layer write_buffer race blocking single-submit).
    #[cfg(target_arch = "wasm32")]
    mc8_norm_resident_buf: Option<wgpu::Buffer>,
    #[cfg(target_arch = "wasm32")]
    mc8_norm_stride: u32,
    /// Native GPU-resident single-fence decode plan (see `resident_decode.rs`).
    #[cfg(not(target_arch = "wasm32"))]
    resident_decode: resident_decode::ResidentDecodeState,
    /// Cold-built host descriptor for the native CUDA all-layer plan.
    #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
    cuda_decode_plan: cuda_decode_plan::CudaDecodePlanState,
    /// W3: native GPU-resident single-fence-per-chunk prefill plan (see `prefill_arena.rs`).
    #[cfg(not(target_arch = "wasm32"))]
    prefill_arena: prefill_arena::PrefillArenaState,
    /// W6a: batched speculative-verify forward plan (per-position argmax; see `verify_arena.rs`).
    #[cfg(not(target_arch = "wasm32"))]
    verify_arena: verify_arena::VerifyArenaState,
    /// Bind group cache: eliminates per-token `create_bind_group` calls by caching
    /// bind groups keyed on (buffer addresses, offsets, weight role, layer). Bind groups
    /// are identical across tokens for the same layer/op since only dynamic uniform
    /// offsets change — those are passed at `set_bind_group` time, not baked into the BG.
    #[cfg(target_arch = "wasm32")]
    mc8_bg_cache: std::sync::Mutex<std::collections::HashMap<u64, wgpu::BindGroup>>,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    pub static WASM_ENGINE_INSTANCE: std::cell::RefCell<Option<QTensorEngine>> = std::cell::RefCell::new(None);
}

#[cfg(target_arch = "wasm32")]
pub async fn initialize_webgpu_engine(model_data: std::sync::Arc<[u8]>) -> Result<(), String> {
    initialize_webgpu_engine_with_options(model_data, false).await
}

/// WASM LLM boot with phone-friendly options.
///
/// `defer_weight_upload`: skip the multi‑hundred‑MB GPU weight materialisation at
/// init (the usual "stuck on Initialising WebGPU" phase on phones). Weights upload
/// lazily on the first forward pass instead — init finishes; first token is slower.
#[cfg(target_arch = "wasm32")]
pub async fn initialize_webgpu_engine_with_options(
    model_data: std::sync::Arc<[u8]>,
    defer_weight_upload: bool,
) -> Result<(), String> {
    use wasm_yield::{clear_init_status, phase, set_init_status};

    // Use `try_new()` (not `new_async()`) so a missing/incompatible WebGPU adapter
    // surfaces as a rejected promise the JS layer can display, rather than an
    // `.expect()` panic that aborts the wasm module and leaves the init promise
    // pending forever (the "stuck on Initialising…" hang).
    phase("Requesting WebGPU adapter + device…").await;
    let mut engine = QTensorEngine::try_new().await?;
    phase("WebGPU device ready — loading model metadata…").await;

    // Dual-format boot gate. P64 owns the canonical lowercase four-byte
    // `p64\0` magic.
    if crate::p64_weight::has_p64_magic(&model_data) {
        engine
            .adopt_resident_p64_async(model_data, defer_weight_upload)
            .await?;
    } else {
        engine
            .adopt_resident_mmap_async(model_data, defer_weight_upload)
            .await?;
    }
    phase("Engine resident — finishing…").await;
    WASM_ENGINE_INSTANCE.with(|g| *g.borrow_mut() = Some(engine));
    set_init_status("ready");
    clear_init_status();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
impl QTensorEngine {
    /// W3 kernel-parity probe (test/diagnostic): run the GPU GEMM (`dispatch_gemm_raw_into`) and the
    /// CPU reference (`stack_gemm_quant`) on the SAME quantized weights + input, writing each into a
    /// caller-provided buffer. Ensures the GEMM buffers exist first, so a fresh engine (no model
    /// loaded) can be probed directly. Returns `true` only if both ran; the caller compares the two
    /// outputs with [`crate::llm_kernel_parity`]. Enable [`crate::llm_gpu_profiler`] around the call
    /// to witness that the GPU path actually executed rather than silently falling back to the CPU.
    pub fn gemm_parity_probe(
        &mut self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        gpu_out: &mut [f32],
        cpu_out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        self.ensure_gemm_buffers(raw.len().max(1), n_out as u32);
        let gpu_ok = self.dispatch_gemm_raw_into(info, raw, input, gpu_out, n_in, n_out);
        let cpu_ok = stack_gemm_quant(raw, info, input, cpu_out, n_in, n_out);
        gpu_ok && cpu_ok
    }
}
