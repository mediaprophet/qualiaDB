//! Neuro-Symbolic GGUF Bridge
//! Dispatches transformer block computation to the best available GPU backend:
//!   - Windows x64: DirectML 1.15 (D3D12, hardware-vendor-optimised kernels)
//!   - All platforms: wgpu / WGSL fallback (Vulkan / Metal / WebGPU)
//! GGUF tensor bytes are memory-mapped via `memmap2` — zero heap copy.

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
pub(crate) const MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS: usize = 8;
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
            encoder: engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
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

/// Static ring-buffer KV layout: `[layer][slot][K | V]` in f32.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvCacheLayout {
    pub max_context: u32,
    pub n_layer: u32,
    pub n_kv_head: u32,
    pub head_dim: u32,
    pub slot_kv_elems: u32,
    pub layer_stride: u32,
    pub total_f32_elems: usize,
}

impl KvCacheLayout {
    pub fn from_hyperparams(h: &crate::gguf_sharder::GgufHyperparams) -> Option<Self> {
        let n_layer = h.n_layer;
        let n_kv_head = h.effective_n_kv_head();
        let head_dim = h.head_dim();
        if n_layer == 0 || n_kv_head == 0 || head_dim == 0 {
            return None;
        }
        let slot_kv_elems = n_kv_head * head_dim;
        let layer_stride = MAX_CONTEXT_WINDOW * slot_kv_elems * 2;
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
/// Vocabulary projection rows per chunked logits sweep (L2-friendly).
pub const VOCAB_CHUNK_ROWS: usize = 8192;

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

// Concern submodules — each holds an `impl QTensorEngine` block for one hot-path area. Methods are
// pub(crate) so they call across modules freely; types/imports arrive via each file's `use super::*`.
mod attention;
mod embedding;
mod ffn;
mod gemm;
mod output;
mod forward;
mod async_dispatch;
mod prefill_async;

/// MC8 pt3e: max abs error over the first `n` elements.
#[cfg(target_arch = "wasm32")]
fn probe_max_abs_diff(a: &[f32], b: &[f32], n: usize) -> f32 {
    let n = n.min(a.len()).min(b.len());
    let mut m = 0.0f32;
    for i in 0..n {
        m = m.max((a[i] - b[i]).abs());
    }
    m
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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
#[cfg(target_arch = "wasm32")]
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
    let o_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, out_info).ok()?;
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
    let gate_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info).ok()?;
    let up_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info).ok()?;
    let down_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info).ok()?;
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
    if !stack_gemm_quant(gate_raw, gate_info, normed, &mut gate[..n_ffn], gate_in, n_ffn) {
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
    if !stack_gemm_quant(down_raw, down_info, &swiglu[..dn_in], &mut down[..n_embd], dn_in, n_embd) {
        return None;
    }
    Some(n_ffn)
}

/// Read one KV head from the CPU mirror arena.
#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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
#[cfg(target_arch = "wasm32")]
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
    let q_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info).ok()?;
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
                .pipeline_read_kv_head(layout, 0, past_slot, kv_h as u32, head_dim, true, &mut k_slot)
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
                    .pipeline_read_kv_head(layout, 0, past_slot, kv_h as u32, head_dim, false, &mut v_slot)
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
fn rope_inplace(
    vec: &mut [f32],
    n_heads: usize,
    head_dim: usize,
    pos: u32,
    base: f32,
    scale: f32,
) {
    let half = head_dim / 2;
    if half == 0 {
        return;
    }
    let scale = if scale > 0.0 && scale.is_finite() { scale } else { 1.0 };
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

/// Zero-heap CPU GEMM: `out[i] = dot(weight_row(i), input)` with per-row dequant.
fn stack_gemm_quant(
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
            input.len(), out.len()
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
    /// Legacy f32×f32 mock block for offset-0 `QTensor` fallback (no mmap).
    mock_pipeline: wgpu::ComputePipeline,
    /// GPU-side Q6_K embedding dequant + matmul (zero CPU dequant).
    pub embedding_pipeline: wgpu::ComputePipeline,
    pub is_initialized: bool,
    /// DirectML device — Some on Windows when DirectML 1.15 is linked.
    #[cfg(target_os = "windows")]
    pub dml: Option<crate::directml_bridge::DmlDevice>,
    /// Memory-mapped GGUF file (set after `load_gguf`).
    #[cfg(not(target_arch = "wasm32"))]
    pub gguf_mmap: Option<Arc<memmap2::Mmap>>,
    #[cfg(target_arch = "wasm32")]
    pub gguf_mmap: Option<Arc<[u8]>>,
    /// Phase 4: resident `.q42` container bytes (set when booted from a `.q42`, not a GGUF).
    #[cfg(target_arch = "wasm32")]
    pub q42_resident: Option<Arc<[u8]>>,

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
    output_topk_pipeline: Option<wgpu::ComputePipeline>,
    topk_cand_val_buf: Option<wgpu::Buffer>,
    topk_cand_idx_buf: Option<wgpu::Buffer>,
    topk_cand_staging: Option<wgpu::Buffer>,
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
    attention_params_buf: Option<wgpu::Buffer>,
    attention_mask_buf: Option<wgpu::Buffer>,
    /// MC8 elementwise GPU ops (RMSNorm / SiLU×mul / residual).
    elem_rms_norm_pipeline: wgpu::ComputePipeline,
    elem_silu_mul_pipeline: wgpu::ComputePipeline,
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
    /// Phase 5.3: the output/logits projection (tied `token_embd`, ~50 MB) uploaded to VRAM
    /// once at init so the per-token argmax binds resident sub-ranges instead of re-uploading
    /// the whole matrix every token (the decode throughput killer). A1a step-2 ports this to the
    /// native top-k decode path, so these two fields are available on both targets.
    mc8_logits_resident_buf: Option<wgpu::Buffer>,
    mc8_logits_row_bytes: u32,
    /// A1b (STELLAR §A): resident 2-bit ternary-FFN GEMM dispatcher, built once at `.q42` boot from
    /// the container's base-3 FFN blobs (rebaked to 2-bit, uploaded once). `None` until a ternary
    /// `.q42` is adopted; the FFN dispatch branch (`dispatch_ternary_ffn`) uses it when present +
    /// the toggle is on, else the CPU oracle. Native-only; the wasm ternary path is a later step.
    #[cfg(not(target_arch = "wasm32"))]
    ternary_ffn: Option<crate::ternary_gpu::TernaryFfnResident>,
    /// Phase 5.4: all layers' attn_norm + ffn_norm weights resident (slot 2L = attn, 2L+1 = ffn),
    /// so RMSNorm binds a per-layer sub-range instead of re-`write_buffer`ing a shared single-layer
    /// `norm_weight_buf` every layer (the second per-layer write_buffer race blocking single-submit).
    #[cfg(target_arch = "wasm32")]
    mc8_norm_resident_buf: Option<wgpu::Buffer>,
    #[cfg(target_arch = "wasm32")]
    mc8_norm_stride: u32,
}

impl QTensorEngine {
    #[inline]
    pub(crate) fn gpu_device(&self) -> &wgpu::Device {
        #[cfg(target_arch = "wasm32")]
        {
            return &self.device;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            &crate::gpu_context::shared_gpu().device
        }
    }

    #[inline]
    pub(crate) fn gpu_queue(&self) -> &wgpu::Queue {
        #[cfg(target_arch = "wasm32")]
        {
            return &self.queue;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            &crate::gpu_context::shared_gpu().queue
        }
    }

    /// Shared process-wide wgpu device (LLM + render coexistence).
    #[inline]
    pub fn device(&self) -> &wgpu::Device {
        self.gpu_device()
    }

    /// Shared process-wide wgpu queue.
    #[inline]
    pub fn queue(&self) -> &wgpu::Queue {
        self.gpu_queue()
    }

    pub async fn try_new() -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        log::info!("LLM_LOAD|engine-init|0.10|Initializing native GGUF runtime (shared GpuContext)");
        #[cfg(target_arch = "wasm32")]
        log::info!("LLM_LOAD|engine-init|0.10|Initializing WASM GGUF runtime");

        #[cfg(not(target_arch = "wasm32"))]
        let shared = crate::gpu_context::shared_gpu();
        #[cfg(not(target_arch = "wasm32"))]
        let device = &shared.device;
        #[cfg(not(target_arch = "wasm32"))]
        let queue = &shared.queue;
        #[cfg(not(target_arch = "wasm32"))]
        log::info!("LLM_LOAD|gpu-device|0.35|Reusing process-wide wgpu device");

        #[cfg(target_arch = "wasm32")]
        let (wasm_device, wasm_queue) = {
            let instance = wgpu::Instance::default();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .map_err(|e| format!("Failed to find wgpu adapter: {e}"))?;
            adapter
                .request_device(&wgpu::DeviceDescriptor::default())
                .await
                .map_err(|e| e.to_string())?
        };
        #[cfg(target_arch = "wasm32")]
        let device = &wasm_device;
        #[cfg(target_arch = "wasm32")]
        let queue = &wasm_queue;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fused Transformer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/fused_transformer.wgsl").into()),
        });

        #[cfg(target_arch = "wasm32")]
        let mc8_gemm_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MC8GemmBGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        #[cfg(target_arch = "wasm32")]
        let mc8_gemm_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MC8GemmPL"),
            bind_group_layouts: &[Some(&mc8_gemm_bind_layout)],
            immediate_size: 0,
        });

        #[cfg(target_arch = "wasm32")]
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Transformer Pipeline"),
            layout: Some(&mc8_gemm_pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        #[cfg(not(target_arch = "wasm32"))]
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Transformer Pipeline"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let mock_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Mock Fused Contraction Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/fused_tensor_contraction.wgsl").into(),
            ),
        });
        let mock_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mock Fused Contraction Pipeline"),
            layout: None,
            module: &mock_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let emb_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Quantized Embedding Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../shaders/quantized_embedding.wgsl").into(),
            ),
        });
        let embedding_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Quantized Embedding Pipeline"),
            layout: None,
            module: &emb_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let attn_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fused Attention Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/fused_attention.wgsl").into()),
        });
        #[cfg(target_arch = "wasm32")]
        let mc8_attn_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MC8AttnBGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        #[cfg(target_arch = "wasm32")]
        let mc8_attn_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MC8AttnPL"),
            bind_group_layouts: &[Some(&mc8_attn_bind_layout)],
            immediate_size: 0,
        });
        #[cfg(target_arch = "wasm32")]
        let attention_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Attention Pipeline"),
            layout: Some(&mc8_attn_pipeline_layout),
            module: &attn_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        #[cfg(not(target_arch = "wasm32"))]
        let attention_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Attention Pipeline"),
            layout: None,
            module: &attn_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let elem_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Wasm Elementwise Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/wasm_elementwise.wgsl").into()),
        });
        #[cfg(target_arch = "wasm32")]
        let mc8_elem_bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MC8ElemBGL"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                    },
                    count: None,
                },
            ],
        });
        #[cfg(target_arch = "wasm32")]
        let mc8_elem_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MC8ElemPL"),
            bind_group_layouts: &[Some(&mc8_elem_bind_layout)],
            immediate_size: 0,
        });
        #[cfg(target_arch = "wasm32")]
        let elem_rms_norm_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemRmsNorm"),
                layout: Some(&mc8_elem_pipeline_layout),
                module: &elem_shader,
                entry_point: Some("rms_norm_batch"),
                compilation_options: Default::default(),
                cache: None,
            });
        #[cfg(target_arch = "wasm32")]
        let elem_silu_mul_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemSiluMul"),
                layout: Some(&mc8_elem_pipeline_layout),
                module: &elem_shader,
                entry_point: Some("silu_mul_main"),
                compilation_options: Default::default(),
                cache: None,
            });
        #[cfg(target_arch = "wasm32")]
        let elem_add_residual_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemAddResidual"),
                layout: Some(&mc8_elem_pipeline_layout),
                module: &elem_shader,
                entry_point: Some("add_residual_main"),
                compilation_options: Default::default(),
                cache: None,
            });
        #[cfg(not(target_arch = "wasm32"))]
        let elem_rms_norm_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemRmsNorm"),
                layout: None,
                module: &elem_shader,
                entry_point: Some("rms_norm_batch"),
                compilation_options: Default::default(),
                cache: None,
            });
        #[cfg(not(target_arch = "wasm32"))]
        let elem_silu_mul_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemSiluMul"),
                layout: None,
                module: &elem_shader,
                entry_point: Some("silu_mul_main"),
                compilation_options: Default::default(),
                cache: None,
            });
        #[cfg(not(target_arch = "wasm32"))]
        let elem_add_residual_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("ElemAddResidual"),
                layout: None,
                module: &elem_shader,
                entry_point: Some("add_residual_main"),
                compilation_options: Default::default(),
                cache: None,
            });

        // Phase 5 — Fused FFN expansion pipeline (gate · SiLU · up in one dispatch).
        // The dequant math is authored once in `dequant_template.wgsl` and instantiated
        // per weight role here (Rust-side modular WGSL composition), so the proven GEMM
        // path in `fused_transformer.wgsl` is untouched.
        #[cfg(target_arch = "wasm32")]
        let mc8_ffn_fused_bind_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MC8FfnFusedBGL"),
                entries: &[
                    // 0: ffn_input (normalized hidden, storage read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 1: gate_words (quantized gate weight, storage read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 2: up_words (quantized up weight, storage read)
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // 3: params (GemmParams, dynamic uniform offset — gate's staged params)
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
                        },
                        count: None,
                    },
                    // 4: ffn_output (silu(gate)·up intermediate, storage read_write)
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        #[cfg(target_arch = "wasm32")]
        let mc8_ffn_fused_pipeline = {
            // Modular WGSL: shared scaffold + per-role dequant instances composed at runtime.
            let tpl = include_str!("../shaders/dequant_template.wgsl");
            let gate_fns = tpl.replace("$W", "gate_words").replace("$S", "_gate");
            let up_fns = tpl.replace("$W", "up_words").replace("$S", "_up");
            let base = include_str!("../shaders/fused_ffn.wgsl");
            // Inject the per-role dequant math at the marker (between shared helpers and
            // the entry point) so declarations precede their uses.
            let src = base.replace(
                "// @@DEQUANT_FUNCTIONS@@",
                &format!("{gate_fns}\n{up_fns}"),
            );
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("FusedFFNExpansion"),
                source: wgpu::ShaderSource::Wgsl(src.into()),
            });
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MC8FfnFusedPL"),
                bind_group_layouts: &[Some(&mc8_ffn_fused_bind_layout)],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("FusedFFNExpansionPipeline"),
                layout: Some(&layout),
                module: &module,
                entry_point: Some("fused_ffn_expansion"),
                compilation_options: Default::default(),
                cache: None,
            })
        };

        #[cfg(target_os = "windows")]
        let dml_status = match crate::directml_bridge::DmlDevice::new() {
            Ok(device) => {
                log::info!(
                    "DirectML device initialization: Ok({})",
                    device.adapter_desc
                );
                log::info!(
                    "LLM_LOAD|gpu-backend|0.45|DirectML ready on {}",
                    device.adapter_desc
                );
                log::info!(
                    "LLM_LOAD|gpu-route|0.48|Streaming weights through DirectML with {:.1} GiB VRAM free",
                    bytes_to_gib(
                        device
                            .local_budget_bytes
                            .saturating_sub(device.local_usage_bytes)
                    )
                );
                Some(device)
            }
            Err(err) => {
                log::warn!("DirectML device initialization failed: {:?}", err);
                log::info!("LLM_LOAD|gpu-backend|0.45|DirectML unavailable; using wgpu fallback");
                None
            }
        };

        #[cfg(not(target_os = "windows"))]
        {
            log::info!("LLM_LOAD|gpu-backend|0.45|Using wgpu fallback backend for native compute");
        }

        Ok(Self {
            #[cfg(target_arch = "wasm32")]
            device: wasm_device,
            #[cfg(target_arch = "wasm32")]
            queue: wasm_queue,
            pipeline,
            mock_pipeline,
            embedding_pipeline,
            attention_pipeline,
            is_initialized: true,
            #[cfg(target_os = "windows")]
            dml: dml_status,
            gguf_mmap: None,
            #[cfg(target_arch = "wasm32")]
            q42_resident: None,
            tensor_data_offset: 0,
            hyperparams: crate::gguf_sharder::GgufHyperparams::default(),
            max_tensor_bytes: 0,
            gemm_input_buf: None,
            gemm_weight_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_weight_arena: None,
            #[cfg(target_arch = "wasm32")]
            mc8_weights_resident: false,
            #[cfg(target_arch = "wasm32")]
            mc8_weight_role_stride: [0u64; 7],
            #[cfg(target_arch = "wasm32")]
            gemm_weight_buf_b: None,
            gemm_output_buf: None,
            gemm_params_buf: None,
            gemm_output_staging: None,
            output_topk_pipeline: None,
            topk_cand_val_buf: None,
            topk_cand_idx_buf: None,
            topk_cand_staging: None,
            topk_params_buf: None,
            gemm_aux_buf: None,
            gemm_ffn_buf: None,
            #[cfg(target_arch = "wasm32")]
            prefill_scratch_buf: None,
            #[cfg(target_arch = "wasm32")]
            prefill_work_buf_a: None,
            #[cfg(target_arch = "wasm32")]
            prefill_work_buf_b: None,
            #[cfg(target_arch = "wasm32")]
            mc8_q_proj_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_k_proj_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_v_proj_buf: None,
            gemm_max_out_dim: MAX_STACK_GEMM_OUT as u32,
            gemm_max_input_floats: 0,
            kv_layout: None,
            kv_cache_gpu: None,
            kv_cache_cpu: None,
            attention_params_buf: None,
            attention_mask_buf: None,
            elem_rms_norm_pipeline,
            elem_silu_mul_pipeline,
            elem_add_residual_pipeline,
            elem_params_buf: None,
            norm_weight_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_gemm_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_elem_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_attn_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_ffn_fused_bind_layout,
            #[cfg(target_arch = "wasm32")]
            mc8_ffn_fused_pipeline,
            mc8_logits_resident_buf: None,
            mc8_logits_row_bytes: 0,
            #[cfg(not(target_arch = "wasm32"))]
            ternary_ffn: None,
            #[cfg(target_arch = "wasm32")]
            mc8_norm_resident_buf: None,
            #[cfg(target_arch = "wasm32")]
            mc8_norm_stride: 0,
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new() -> Self {
        let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| { let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap())); rt.handle().clone() });
        tokio::task::block_in_place(|| {
            handle.block_on(Self::try_new())
                .expect("Failed to initialize native GGUF engine")
        })
    }

    pub(crate) fn ensure_kv_cache(&mut self, h: &crate::gguf_sharder::GgufHyperparams) {
        let layout = match KvCacheLayout::from_hyperparams(h) {
            Some(l) => l,
            None => {
                #[cfg(target_arch = "wasm32")]
                wlog("[kv_cache] FAILED from_hyperparams (zero dims or exceeds KV_CACHE_MAX_BYTES)");
                return;
            }
        };
        let bytes = (layout.total_f32_elems * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        // Native: honour U0 VRAM ledger pins. WASM: always allocate CPU mirror + wgpu storage
        // (ledger models host adapter VRAM; browser WebGPU has separate limits).
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ledger = crate::gpu_context::global_vram_ledger();
            let orch = crate::gpu_context::universe_orchestrator();
            if !ledger.can_allocate_in_universe(
                &orch,
                crate::gpu_context::ComputeUniverse::LlmInference,
                bytes,
            ) {
                log::warn!(
                    "LLM_LOAD|kv-cache|denied|U0 budget {:.1} MiB used, need {:.1} MiB (mode {:?})",
                    ledger.universe_used_bytes(crate::gpu_context::ComputeUniverse::LlmInference) as f64
                        / (1024.0 * 1024.0),
                    bytes as f64 / (1024.0 * 1024.0),
                    orch.active_mode,
                );
                return;
            }
        }
        let gpu = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("StaticKvCacheArena"),
            size: bytes.max(4),
            // COPY_SRC: MC8 pt3e L0 probe reads K/V slots via pipeline_read_kv_head.
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let cpu = vec![0f32; layout.total_f32_elems].into_boxed_slice();
        let attn_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_ATTN_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<AttentionGpuParams>() as wgpu::BufferAddress
            }
        };
        self.attention_params_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("AttentionParams"),
            size: attn_params_bytes.max(4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        let mask_bytes = (MAX_ATTN_MASK_UPLOAD_WORDS * std::mem::size_of::<u32>()).max(4);
        self.attention_mask_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("AttentionKvMaskBatch"),
            size: mask_bytes as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.kv_layout = Some(layout);
        self.kv_cache_gpu = Some(gpu);
        self.kv_cache_cpu = Some(cpu);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ledger = crate::gpu_context::global_vram_ledger();
            ledger.record_kv_cache(bytes);
        }
        log::info!(
            "LLM_LOAD|kv-cache|0.86|Reserved {:.1} MiB KV cache (GPU + CPU mirror, context {})",
            bytes as f64 / (1024.0 * 1024.0),
            layout.max_context,
        );
        #[cfg(not(target_arch = "wasm32"))]
        eprintln!(
            "[gguf_bridge] KV arena {} f32 ({:.1} MiB), context={}",
            layout.total_f32_elems,
            bytes as f64 / (1024.0 * 1024.0),
            layout.max_context,
        );
    }

    /// Zero the static KV arena at the start of a new decode context (zero heap in decode).
    pub fn reset_kv_cache(&mut self) {
        let Some(layout) = self.kv_layout.as_ref() else {
            return;
        };
        let n = layout.total_f32_elems;
        if let Some(cpu) = self.kv_cache_cpu.as_mut() {
            for v in cpu.iter_mut().take(n) {
                unsafe { core::ptr::write_volatile(v, 0.0) };
            }
        }
        if let (Some(cpu), Some(gpu)) = (self.kv_cache_cpu.as_ref(), self.kv_cache_gpu.as_ref()) {
            self.gpu_queue()
                .write_buffer(gpu, 0, bytemuck::cast_slice(&cpu[..n]));
        }
    }

    pub(crate) fn ensure_gemm_buffers(&mut self, max_weight_bytes: usize, max_out_dim: u32) {
        // A1a: build the persistent GPU top-k pipeline + candidate buffers once (additive; the
        // existing argmax path is unaffected whether or not this succeeds).
        #[cfg(not(target_arch = "wasm32"))]
        if self.output_topk_pipeline.is_none() {
            self.init_output_topk();
        }
        let need_input = MAX_STACK_GEMM_IN.max(MAX_PREFILL_BATCH_FLOATS);
        let prefill_bufs_ready = {
            #[cfg(target_arch = "wasm32")]
            {
                self.prefill_scratch_buf.is_some()
                    && self.prefill_work_buf_a.is_some()
                    && self.prefill_work_buf_b.is_some()
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                true
            }
        };
        #[cfg(target_arch = "wasm32")]
        let weight_arena_ready = self.mc8_weight_arena.is_some();
        #[cfg(not(target_arch = "wasm32"))]
        let weight_arena_ready = true;
        if self.gemm_weight_buf.is_some()
            && max_weight_bytes <= self.max_tensor_bytes
            && self.gemm_max_input_floats >= need_input
            && prefill_bufs_ready
            && weight_arena_ready
        {
            return;
        }
        let w_bytes = max_weight_bytes.max(4) as wgpu::BufferAddress;
        let in_bytes = (need_input * 4) as wgpu::BufferAddress;
        let out_bytes = (max_out_dim as usize * 4).max(4) as wgpu::BufferAddress;
        self.gemm_input_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmInput"),
            size: in_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.gemm_weight_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmWeight"),
            size: w_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        #[cfg(target_arch = "wasm32")]
        {
            let weight_usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
            let mk = |label: &str| {
                self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: w_bytes,
                    usage: weight_usage,
                    mapped_at_creation: false,
                })
            };
            let qkv_k = mk("MC8WeightAttnK");
            let qkv_v = mk("MC8WeightAttnV");
            let qkv_q = mk("MC8WeightAttnQ");
            let o_proj = mk("MC8WeightOProj");
            let gate = mk("MC8WeightGate");
            let up = mk("MC8WeightUp");
            let down = mk("MC8WeightDown");
            let weight_b = mk("LayerGemmWeightB");
            self.mc8_weight_arena = Some(Mc8WeightArenaBufs {
                qkv_k,
                qkv_v,
                qkv_q,
                o_proj,
                gate,
                up,
                down,
            });
            self.gemm_weight_buf_b = Some(weight_b);
        }
        self.gemm_output_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmOutput"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        let gemm_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_GEMM_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<GemmGpuParams>() as wgpu::BufferAddress
            }
        };
        self.gemm_params_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmParams"),
            size: gemm_params_bytes.max(4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        #[cfg(target_arch = "wasm32")]
        let staging_bytes = out_bytes.max((65536 * 4) as wgpu::BufferAddress);
        #[cfg(not(target_arch = "wasm32"))]
        let staging_bytes = out_bytes;
        self.gemm_output_staging = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmStaging"),
            size: staging_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_aux_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmAux"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.gemm_ffn_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmFfnUp"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        #[cfg(target_arch = "wasm32")]
        {
            self.prefill_scratch_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("PrefillBatchScratch"),
                size: in_bytes,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
            // Per-token row: norm + gate + up + save (see encode_prefill_q_ffn_tail_fused).
            let work_row_floats = (MAX_HIDDEN_DIM + 2 * max_out_dim as usize + MAX_HIDDEN_DIM).max(4);
            let work_bytes =
                (PREFILL_CHUNK_SIZE * work_row_floats * 4).max(4) as wgpu::BufferAddress;
            let work_usage = wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC;
            self.prefill_work_buf_a = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("PrefillBatchWorkA"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
            self.prefill_work_buf_b = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("PrefillBatchWorkB"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
            // Phase 5.5: Q/K/V projection scratch (parallel-GEMM output). work_bytes ≥ q_dim×tokens.
            self.mc8_q_proj_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("MC8QProj"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
            self.mc8_k_proj_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("MC8KProj"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
            self.mc8_v_proj_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("MC8VProj"),
                size: work_bytes,
                usage: work_usage,
                mapped_at_creation: false,
            }));
        }
        let elem_params_bytes = {
            #[cfg(target_arch = "wasm32")]
            {
                (MC8_MAX_ELEM_UNIFORM_CHUNK_SLOTS * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                std::mem::size_of::<ElemGpuParams>() as wgpu::BufferAddress
            }
        };
        self.elem_params_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ElemParams"),
            size: elem_params_bytes.max(4),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        let norm_bytes = (MAX_HIDDEN_DIM * 4) as wgpu::BufferAddress;
        self.norm_weight_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("NormWeights"),
            size: norm_bytes.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_max_out_dim = max_out_dim;
        self.gemm_max_input_floats = need_input;
        self.max_tensor_bytes = max_weight_bytes;
    }

    pub fn kv_cache_bytes(&self) -> u64 {
        self.kv_layout
            .as_ref()
            .map(|layout| (layout.total_f32_elems * std::mem::size_of::<f32>()) as u64)
            .unwrap_or(0)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_gguf_checked(&mut self, path: &str) -> Result<GgufLoadReport, String> {
        use std::fs::File;

        log::info!("LLM_LOAD|gguf-open|0.52|Opening GGUF file {}", path);
        let file = File::open(path).map_err(|e| {
            log::error!("GGUF mmap open failed for {}: {}", path, e);
            log::error!("LLM_LOAD|failed|1.00|Could not open GGUF: {}", e);
            e.to_string()
        })?;
        log::info!("LLM_LOAD|mmap-start|0.64|Memory-mapping GGUF into virtual memory");
        let mmap = unsafe { MmapOptions::new().map(&file) }.map_err(|e| {
            log::error!("GGUF mmap failed for {}: {}", path, e);
            log::error!("LLM_LOAD|failed|1.00|Memory map failed: {}", e);
            e.to_string()
        })?;
        let file_size = mmap.len();
        log::info!(
            "LLM_LOAD|ram-map|0.70|Mapped {:.2} GiB GGUF into system memory",
            bytes_to_gib(file_size as u64)
        );
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            let msg = "GGUF header parse failed or yielded no tensor metadata".to_string();
            log::error!("LLM_LOAD|failed|1.00|{}", msg);
            return Err(msg);
        }

        self.tensor_data_offset = index.tensor_data_start;
        self.hyperparams = index.hyperparams;
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&index.hyperparams);
        self.gguf_mmap = Some(Arc::new(mmap));

        let kv_cache_bytes = self.kv_cache_bytes();
        log::info!(
            "LLM_LOAD|gguf-index|0.78|Parsed {} layers, {} attention heads",
            self.hyperparams.n_layer,
            self.hyperparams.n_head
        );
        log::info!(
            "LLM_LOAD|gguf-ready|0.92|GGUF indexed and cache arena reserved ({} MiB)",
            kv_cache_bytes / (1024 * 1024)
        );

        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }

    /// Memory-map a GGUF file so tensor bytes are accessible without heap allocation.
    /// Call this once after `new()`, before the first `dispatch_fused_transformer_block`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_gguf(&mut self, path: &str) {
        if let Err(e) = self.load_gguf_checked(path) {
            eprintln!("[gguf_bridge] Could not load {path}: {e}");
        }
    }

    /// A1b: build the resident 2-bit ternary-FFN dispatcher from a `.q42` container's base-3 FFN
    /// blobs (rebaked to 2-bit + uploaded once). Returns false if there are no ternary FFN tensors
    /// or the GPU build fails — the FFN then runs the CPU oracle (`dispatch_ternary_ffn` fallback).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn build_ternary_ffn_resident(&mut self, q: &crate::q42_weight::Q42TensorIndex) -> bool {
        let mmap_arc = match self.gguf_mmap.clone() {
            Some(a) => a,
            None => return false,
        };
        let data: &[u8] = &mmap_arc;
        let mut tensors: Vec<(u64, usize, usize, &[u8])> = Vec::new();
        for e in &q.entries {
            if e.ggml_type != crate::ternary::GGML_TYPE_TERNARY_158 {
                continue;
            }
            let (n_in, n_out) = (e.dim0 as usize, e.dim1 as usize);
            let (off, len) = (e.blob_offset as usize, e.byte_len as usize);
            if n_in == 0 || n_out == 0 || off + len > data.len() {
                continue;
            }
            // key = the .q42 blob offset == the synthetic index's GgufTensorInfo::byte_offset.
            tensors.push((e.blob_offset, n_in, n_out, &data[off..off + len]));
        }
        if tensors.is_empty() {
            return false;
        }
        match crate::ternary_gpu::TernaryFfnResident::build(
            self.gpu_device(),
            self.gpu_queue(),
            &tensors,
        ) {
            Some(r) => {
                log::info!(
                    "LLM_LOAD|ternary-ffn|0.71|resident 2-bit FFN: {} tensors, {:.1} MB",
                    r.len(),
                    r.resident_bytes() as f64 / (1024.0 * 1024.0)
                );
                self.ternary_ffn = Some(r);
                true
            }
            None => false,
        }
    }

    /// A1b: boot from an already-mapped `.q42` weight container (native). Mirrors the GGUF
    /// `adopt_resident_mmap` but for the `Q42W` format: validates + builds a synthetic GGUF index
    /// from the manifest, points the byte source at the `.q42` bytes (`tensor_data_start = 0`,
    /// absolute blob offsets), reserves the GEMM/KV arenas, makes the (verbatim) output projection
    /// resident, and builds the resident 2-bit ternary-FFN dispatcher from the FFN blobs. The
    /// attention/norm/embed tensors stay at source precision and run the standard GGUF hot path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_resident_q42_mmap(
        &mut self,
        mmap: Arc<memmap2::Mmap>,
    ) -> Result<GgufLoadReport, String> {
        let file_size = mmap.len();
        if file_size == 0 {
            return Err("Empty Q42 mmap".to_string());
        }
        let q = crate::q42_weight::Q42TensorIndex::from_q42(&mmap[..])?;
        let index = q.to_gguf_index();
        let hp = index.hyperparams;
        if hp.n_layer == 0 || hp.n_embd == 0 {
            return Err("Q42: missing hyperparameters in header".to_string());
        }
        self.hyperparams = hp;
        self.tensor_data_offset = 0; // q42 blob offsets are absolute
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&hp);
        if self.kv_layout.is_none() || self.kv_cache_cpu.is_none() {
            return Err("Q42: KV cache allocation failed".to_string());
        }
        self.gguf_mmap = Some(mmap);
        if !self.mc8_upload_resident_logits(&index) {
            log::info!("LLM_LOAD|q42-logits|0.70|skipped — per-token upload fallback");
        }
        if !self.build_ternary_ffn_resident(&q) {
            log::info!(
                "LLM_LOAD|ternary-ffn|0.71|no resident set (no ternary FFN or build failed) — CPU oracle path"
            );
        }
        let kv_cache_bytes = self.kv_cache_bytes();
        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: 0,
            n_layer: hp.n_layer,
            n_head: hp.n_head,
            n_kv_head: hp.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }

    /// A1b: number of resident ternary FFN tensors (0 unless a ternary `.q42` was adopted). Lets a
    /// test confirm the GPU resident path is actually populated (not a silent CPU-only fallback).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn ternary_ffn_resident_len(&self) -> usize {
        self.ternary_ffn.as_ref().map_or(0, |r| r.len())
    }

    /// Attach an already-mapped resident GGUF (shared with orchestrator slot).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn adopt_resident_mmap(&mut self, mmap: Arc<memmap2::Mmap>) -> Result<GgufLoadReport, String> {
        let file_size = mmap.len();
        if file_size == 0 {
            return Err("Empty GGUF mmap".to_string());
        }
        log::info!(
            "LLM_LOAD|resident-mmap|0.68|Reusing resident GGUF mapping ({:.2} GiB)",
            bytes_to_gib(file_size as u64)
        );
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(mmap.as_ref());
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            return Err("GGUF header parse failed or yielded no tensor metadata".to_string());
        }
        self.tensor_data_offset = index.tensor_data_start;
        self.hyperparams = index.hyperparams;
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&index.hyperparams);
        self.gguf_mmap = Some(mmap);
        // A1a step-2: make the output/logits projection resident (upload once) so the per-token
        // top-k decode binds per-chunk 256-aligned sub-ranges instead of re-uploading the whole
        // ~47 MB matrix every token (the documented decode throughput killer). Fail-soft: a false
        // return leaves `mc8_logits_resident_buf=None` and the decode keeps its per-token upload.
        if !self.mc8_upload_resident_logits(&index) {
            log::info!("LLM_LOAD|resident-logits|0.70|skipped — per-token upload fallback");
        }
        let kv_cache_bytes = self.kv_cache_bytes();
        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }

    /// A1a step-2 (native port of Phase 5.3): upload the output/logits projection (tied
    /// `token_embd`) to a resident `STORAGE` buffer **once**, so the per-token top-k decode binds
    /// per-chunk 256-aligned sub-ranges instead of re-uploading the whole ~47 MB matrix every
    /// token (the decode throughput killer). Idempotent. Returns false (→ per-token upload
    /// fallback) if the projection is missing or its bytes don't divide evenly into rows.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn mc8_upload_resident_logits(&mut self, index: &crate::gguf_sharder::GgufTensorIndex) -> bool {
        if self.mc8_logits_resident_buf.is_some() {
            return true;
        }
        let info = match index.logits_projection_info() {
            Some(i) => i,
            None => return false,
        };
        let (_, vocab) = Self::matmul_dims(info);
        if vocab == 0 {
            return false;
        }
        // Clone the Arc so the mmap borrow does not block mutating `self` below.
        let mmap_arc = match self.gguf_mmap.clone() {
            Some(a) => a,
            None => return false,
        };
        let mmap: &[u8] = &mmap_arc;
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let total = raw.len();
        if total == 0 || total % vocab != 0 {
            return false;
        }
        let row_bytes = total / vocab;
        // VOCAB_CHUNK_ROWS (8192) is a multiple of 256, so every chunk's byte offset
        // (chunk_idx * VOCAB_CHUNK_ROWS * row_bytes) is 256-aligned for the storage binding.
        let buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ResidentLogits"),
            size: total as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&buf, 0, raw);
        self.mc8_logits_resident_buf = Some(buf);
        self.mc8_logits_row_bytes = row_bytes as u32;
        log::info!(
            "LLM_LOAD|resident-logits|0.70|output projection resident once: {:.1} MB ({} rows x {} B)",
            total as f64 / (1024.0 * 1024.0),
            vocab,
            row_bytes
        );
        true
    }

    /// Decode-profiler: blocking GPU fence wait + round-trip counter. Every native sync point routes
    /// through this (via the `self.gpu_device().poll(Maintain::Wait)` → `self.poll_wait()` rewrite),
    /// so the bench can count submit→wait round-trips per token and separate synchronization stall
    /// from real kernel time. Behaviourally identical to a bare blocking poll.
    #[inline]
    pub(crate) fn poll_wait(&self) {
        let _ = self.gpu_device().poll(wgpu::PollType::wait_indefinitely());
        GPU_WAIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Decode-profiler: wall-clock for `n` EMPTY `submit → poll(Maintain::Wait)` round-trips (no
    /// compute dispatched). Isolates the fixed CPU↔GPU fence latency: if a token's forward time ≈
    /// (its round-trip count × this per-round-trip cost), the bottleneck is synchronization, not
    /// math; if forward ≫ that, the kernels themselves are slow. Does NOT touch `GPU_WAIT_COUNT`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn bench_empty_submit_roundtrip(&self, n: u32) -> u64 {
        let t = std::time::Instant::now();
        for _ in 0..n {
            let enc = self
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("EmptyRT"),
                });
            self.gpu_queue().submit(Some(enc.finish()));
            let _ = self.gpu_device().poll(wgpu::PollType::wait_indefinitely());
        }
        t.elapsed().as_nanos() as u64
    }
    #[cfg(target_arch = "wasm32")]
    pub fn adopt_resident_mmap(&mut self, mmap: Arc<[u8]>) -> Result<GgufLoadReport, String> {
        let file_size = mmap.len();
        if file_size == 0 {
            return Err("Empty GGUF mmap".to_string());
        }
        log::info!(
            "LLM_LOAD|resident-mmap|0.68|Reusing resident GGUF mapping ({:.2} GiB)",
            bytes_to_gib(file_size as u64)
        );
        let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(mmap.as_ref());
        if index.tensor_data_start == 0
            && index.max_tensor_bytes == 0
            && index.hyperparams.n_layer == 0
        {
            return Err("GGUF header parse failed or yielded no tensor metadata".to_string());
        }
        self.tensor_data_offset = index.tensor_data_start;
        self.hyperparams = index.hyperparams;
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&index.hyperparams);
        if self.kv_layout.is_none() || self.kv_cache_cpu.is_none() {
            return Err("KV cache allocation failed (layout or CPU mirror missing)".to_string());
        }
        self.gguf_mmap = Some(mmap);
        // Part 3y: stage all layer weights to the GPU now (init time, before the TTFT clock),
        // so the 219 MB upload is not charged to the first token's latency.
        if !self.mc8_upload_all_resident_weights(&index) {
            wlog("[MC8] eager resident weight upload skipped at init — will retry lazily");
        }
        // Phase 5.3: also make the output/logits projection resident (eliminates the ~50 MB
        // per-token re-upload in the decode argmax).
        if !self.mc8_upload_resident_logits(&index) {
            wlog("[MC8] resident logits projection skipped at init — per-token upload fallback");
        }
        // Phase 5.4: norm weights resident (removes the per-layer norm write_buffer race).
        if !self.mc8_upload_resident_norms(&index) {
            wlog("[MC8] resident norm weights skipped at init — per-layer upload fallback");
        }
        let kv_cache_bytes = self.kv_cache_bytes();
        Ok(GgufLoadReport {
            mapped_bytes: file_size as u64,
            tensor_data_offset: self.tensor_data_offset,
            n_layer: self.hyperparams.n_layer,
            n_head: self.hyperparams.n_head,
            n_kv_head: self.hyperparams.effective_n_kv_head(),
            max_tensor_bytes: index.max_tensor_bytes,
            kv_cache_bytes,
            directml_enabled: {
                #[cfg(target_os = "windows")]
                {
                    self.dml.is_some()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    false
                }
            },
        })
    }

}

#[cfg(target_arch = "wasm32")]
thread_local! {
    pub static WASM_ENGINE_INSTANCE: std::cell::RefCell<Option<QTensorEngine>> = std::cell::RefCell::new(None);
}

#[cfg(target_arch = "wasm32")]
pub async fn initialize_webgpu_engine(gguf_data: std::sync::Arc<[u8]>) -> Result<(), String> {
    // Use `try_new()` (not `new_async()`) so a missing/incompatible WebGPU adapter
    // surfaces as a rejected promise the JS layer can display, rather than an
    // `.expect()` panic that aborts the wasm module and leaves the init promise
    // pending forever (the "stuck on Initialising…" hang).
    let mut engine = QTensorEngine::try_new().await?;
    // Dual-format boot gate: inspect the first 4 magic bytes.
    //   b"Q42W" → Phase 4 AOT container (validate CRC, map blobs straight into the arenas).
    //   else    → legacy GGUF (parse metadata, reserve GEMM + KV arenas).
    if gguf_data.len() >= 4 && gguf_data[0..4] == *b"Q42W" {
        engine.adopt_resident_q42(gguf_data)?;
    } else {
        engine.adopt_resident_mmap(gguf_data)?;
    }
    WASM_ENGINE_INSTANCE.with(|g| *g.borrow_mut() = Some(engine));
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
