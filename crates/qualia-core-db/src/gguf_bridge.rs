//! Neuro-Symbolic GGUF Bridge
//! Dispatches transformer block computation to the best available GPU backend:
//!   - Windows x64: DirectML 1.15 (D3D12, hardware-vendor-optimised kernels)
//!   - All platforms: wgpu / WGSL fallback (Vulkan / Metal / WebGPU)
//! GGUF tensor bytes are memory-mapped via `memmap2` — zero heap copy.

use crate::gguf_sharder::GgufTensorInfo;
use crate::NQuin;
use log;
#[cfg(not(target_arch = "wasm32"))]
use memmap2::MmapOptions;
use std::sync::Arc;

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

/// Uniform block passed to `quantized_embedding.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct EmbeddingGpuParams {
    n_embd: u32,
    ggml_type: u32,
    n_output: u32,
    raw_byte_len: u32,
}

/// Uniform block passed to `fused_transformer.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct GemmGpuParams {
    n_in: u32,
    n_out: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32,
    n_batch: u32,
    in_row_stride: u32,
    out_row_stride: u32,
}

/// Uniform block passed to `fused_attention.wgsl`.
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct AttentionGpuParams {
    n_embd: u32,
    n_head: u32,
    n_kv_head: u32,
    head_dim: u32,
    q_heads_per_kv: u32,
    token_idx: u32,
    max_context: u32,
    layer_idx: u32,
    layer_stride: u32,
    slot_kv_elems: u32,
    weight_ggml_type: u32,
    weight_row_elems: u32,
    weight_byte_len: u32,
    proj_kind: u32,
    rope_theta_base: f32,
    rope_scale: f32,
    num_tokens_in_batch: u32,
    batch_start_token_idx: u32,
    mask_active: u32,
    mask_word_count: u32,
    out_stride_elems: u32,
    /// Phase 5.5: row stride (floats/token) of the PRE-COMPUTED Q/K/V projection bound at binding 0.
    /// Non-zero → the shader reads the projection directly (parallel GEMM did the matmul) instead of
    /// doing the per-element `gemm_row` matmul itself. 0 → legacy in-shader projection.
    proj_row_stride: u32,
    /// WGSL uniform struct size must be a multiple of 16 bytes.
    _pad: [u32; 2],
}

/// KV attention bitmask words uploaded to `fused_attention.wgsl` binding 5.
pub const KV_ATTENTION_MASK_WORDS: usize = crate::compute_universe::KV_ATTENTION_MASK_WORDS;

/// Uniform block for `wasm_elementwise.wgsl` (MC8 GPU norm / SwiGLU / residual).
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct ElemGpuParams {
    n: u32,
    batch: u32,
    op: u32,
    eps: f32,
    a_row_stride: u32,
    b_row_stride: u32,
    out_row_stride: u32,
    a_slot: u32,
    b_slot: u32,
    out_slot: u32,
    _pad: u32,
}

const ELEM_OP_RMS_NORM: u32 = 0;
const ELEM_OP_SILU_MUL: u32 = 1;
const ELEM_OP_ADD_RESIDUAL: u32 = 2;

/// MC8 Part 3s: WebGPU dynamic uniform offsets must be multiples of 256 bytes.
#[cfg(target_arch = "wasm32")]
const MC8_UNIFORM_ALIGN: usize = 256;
#[cfg(target_arch = "wasm32")]
const MC8_MAX_GEMM_UNIFORM_SLOTS: usize = 8;
#[cfg(target_arch = "wasm32")]
const MC8_MAX_ELEM_UNIFORM_SLOTS: usize = 8;
#[cfg(target_arch = "wasm32")]
const MC8_MAX_ATTN_UNIFORM_SLOTS: usize = 8;
#[cfg(target_arch = "wasm32")]
const MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS: usize = 8;
/// MC8 Part 3v / Phase 5.4: layers encoded into one submit batch. Sizes the per-layer uniform
/// buffers (slots_per_layer × this) and is the decode forward's chunk size — 64 → the whole
/// ≤64-layer forward is a single submit. Per-chunk flush + reset handles deeper models.
#[cfg(target_arch = "wasm32")]
const MC8_LAYERS_PER_ENCODER: u32 = 64;
/// Uniform slots reserved per layer within a chunk (must cover K/V/Q + tail).
#[cfg(target_arch = "wasm32")]
const MC8_ATTN_SLOTS_PER_LAYER: usize = 4;
#[cfg(target_arch = "wasm32")]
const MC8_ELEM_SLOTS_PER_LAYER: usize = 6;
#[cfg(target_arch = "wasm32")]
const MC8_GEMM_SLOTS_PER_LAYER: usize = 8; // o, gate, up, down + Phase 5.5 Q/K/V projection
#[cfg(target_arch = "wasm32")]
const MC8_MAX_ATTN_UNIFORM_CHUNK_SLOTS: usize =
    MC8_ATTN_SLOTS_PER_LAYER * MC8_LAYERS_PER_ENCODER as usize;
#[cfg(target_arch = "wasm32")]
const MC8_MAX_ELEM_UNIFORM_CHUNK_SLOTS: usize =
    MC8_ELEM_SLOTS_PER_LAYER * MC8_LAYERS_PER_ENCODER as usize;
#[cfg(target_arch = "wasm32")]
const MC8_MAX_GEMM_UNIFORM_CHUNK_SLOTS: usize =
    MC8_GEMM_SLOTS_PER_LAYER * MC8_LAYERS_PER_ENCODER as usize;

/// Part 3v: absolute uniform slot cursors within one encoder chunk.
#[cfg(target_arch = "wasm32")]
struct Mc8ChunkUniformCursors {
    attn: usize,
    elem: usize,
    gemm: usize,
}

/// MC8 Part 3t: disjoint weight staging — eliminates `write_buffer` races within one layer submit.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mc8WeightRole {
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
    fn idx(self) -> usize {
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
struct Mc8WeightArenaBufs {
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
struct Mc8PrefillLayerUniforms {
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
struct Mc8PrefillLayerGeom {
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
struct Mc8UniformArena {
    bytes: [u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
    slots: usize,
}

/// Part 3u: larger elem arena for full-layer super-staging.
#[cfg(target_arch = "wasm32")]
struct Mc8ElemUniformArena {
    bytes: [u8; MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS * MC8_UNIFORM_ALIGN],
    slots: usize,
}

#[cfg(target_arch = "wasm32")]
struct Mc8AttnUniformArena {
    bytes: [u8; MC8_MAX_ATTN_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
    slots: usize,
}

#[cfg(target_arch = "wasm32")]
impl Mc8ElemUniformArena {
    fn push<T: bytemuck::Pod>(&mut self, value: &T) -> u32 {
        debug_assert!(std::mem::size_of::<T>() <= MC8_UNIFORM_ALIGN);
        debug_assert!(self.slots < MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS);
        let byte_off = self.slots * MC8_UNIFORM_ALIGN;
        self.slots += 1;
        self.bytes[byte_off..byte_off + std::mem::size_of::<T>()]
            .copy_from_slice(bytemuck::bytes_of(value));
        byte_off as u32
    }

    fn upload(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer) {
        if self.slots == 0 {
            return;
        }
        queue.write_buffer(buf, 0, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }

    fn upload_at(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer, base_slot: usize) {
        if self.slots == 0 {
            return;
        }
        let byte_off = (base_slot * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress;
        queue.write_buffer(buf, byte_off, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }
}

#[cfg(target_arch = "wasm32")]
impl Mc8AttnUniformArena {
    fn push<T: bytemuck::Pod>(&mut self, value: &T) -> u32 {
        debug_assert!(std::mem::size_of::<T>() <= MC8_UNIFORM_ALIGN);
        debug_assert!(self.slots < MC8_MAX_ATTN_UNIFORM_SLOTS);
        let byte_off = self.slots * MC8_UNIFORM_ALIGN;
        self.slots += 1;
        self.bytes[byte_off..byte_off + std::mem::size_of::<T>()]
            .copy_from_slice(bytemuck::bytes_of(value));
        byte_off as u32
    }

    fn upload(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer) {
        if self.slots == 0 {
            return;
        }
        queue.write_buffer(buf, 0, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }

    fn upload_at(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer, base_slot: usize) {
        if self.slots == 0 {
            return;
        }
        let byte_off = (base_slot * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress;
        queue.write_buffer(buf, byte_off, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }
}

#[cfg(target_arch = "wasm32")]
impl Mc8UniformArena {
    fn push<T: bytemuck::Pod>(&mut self, value: &T) -> u32 {
        debug_assert!(std::mem::size_of::<T>() <= MC8_UNIFORM_ALIGN);
        debug_assert!(self.slots < MC8_MAX_GEMM_UNIFORM_SLOTS);
        let byte_off = self.slots * MC8_UNIFORM_ALIGN;
        self.slots += 1;
        let dst = &mut self.bytes[byte_off..byte_off + std::mem::size_of::<T>()];
        dst.copy_from_slice(bytemuck::bytes_of(value));
        byte_off as u32
    }

    fn upload(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer) {
        if self.slots == 0 {
            return;
        }
        queue.write_buffer(buf, 0, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }

    fn upload_at(&self, queue: &wgpu::Queue, buf: &wgpu::Buffer, base_slot: usize) {
        if self.slots == 0 {
            return;
        }
        let byte_off = (base_slot * MC8_UNIFORM_ALIGN) as wgpu::BufferAddress;
        queue.write_buffer(buf, byte_off, &self.bytes[..self.slots * MC8_UNIFORM_ALIGN]);
    }
}

#[cfg(target_arch = "wasm32")]
impl Mc8ChunkUniformCursors {
    fn reset(&mut self) {
        self.attn = 0;
        self.elem = 0;
        self.gemm = 0;
    }

    fn attn_base_byte(&self) -> u32 {
        (self.attn * MC8_UNIFORM_ALIGN) as u32
    }

    fn elem_base_byte(&self) -> u32 {
        (self.elem * MC8_UNIFORM_ALIGN) as u32
    }

    fn gemm_base_byte(&self) -> u32 {
        (self.gemm * MC8_UNIFORM_ALIGN) as u32
    }
}

/// MC8: accumulates compute passes; submit + map_async only at pipeline boundary.
#[cfg(target_arch = "wasm32")]
struct WasmGpuPipeline {
    encoder: wgpu::CommandEncoder,
}

#[inline]
fn ggml_gpu_quant_supported(ggml_type: u32) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        // WASM CPU fallback dequantizes all stack_gemm-supported types.
        matches!(
            ggml_type,
            crate::ggml_quants::GGML_TYPE_F32
                | crate::ggml_quants::GGML_TYPE_F16
                | crate::ggml_quants::GGML_TYPE_Q4_0
                | crate::ggml_quants::GGML_TYPE_Q5_0
                | crate::ggml_quants::GGML_TYPE_Q8_0
                | crate::ggml_quants::GGML_TYPE_Q4_K
                | crate::ggml_quants::GGML_TYPE_Q6_K
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K
            || ggml_type == crate::ggml_quants::GGML_TYPE_Q6_K
    }
}

/// Weight types implemented in `fused_attention.wgsl` / `fused_transformer.wgsl` dequant.
/// F16 added for the all-F16 Llama-3.2 family: without it `dispatch_attention_layer` returned
/// `None` and (because attn_q/k/v are present, skipping the `attn_output`-only fallback) attention
/// was silently dropped from every block — coherent-looking run, garbage logits. Mirrors the #49
/// Q8_0 widening. `fused_attention.wgsl::dequant_f16_weight` is the matching shader-side path.
#[inline]
fn ggml_gpu_attention_shader_supported(ggml_type: u32) -> bool {
    matches!(
        ggml_type,
        crate::ggml_quants::GGML_TYPE_F16
            | crate::ggml_quants::GGML_TYPE_Q4_0
            | crate::ggml_quants::GGML_TYPE_Q5_0
            | crate::ggml_quants::GGML_TYPE_Q8_0
            | crate::ggml_quants::GGML_TYPE_Q4_K
            | crate::ggml_quants::GGML_TYPE_Q6_K
    )
}

/// Weight types the native GEMM shader (`fused_transformer.wgsl`) actually dequantizes — WIDER than
/// `ggml_gpu_quant_supported` (Q4_K/Q6_K only). The GEMM `dequant_weight` also implements
/// Q4_0/Q5_0/Q8_0 (identical code to the attention shader), but the narrow predicate was silently
/// routing Q8_0 FFN + output-projection GEMMs to the CPU `stack_gemm_quant` fallback. Widening this
/// is the GEMM-side analogue of the #49 attention-support fix. Verified end-to-end for Q8_0
/// (SmolLM2-360M-q8_0); Q4_0/Q5_0 share the proven dequant but have no resident test model yet.
#[inline]
fn ggml_gpu_gemm_supported(ggml_type: u32) -> bool {
    matches!(
        ggml_type,
        crate::ggml_quants::GGML_TYPE_F16
            | crate::ggml_quants::GGML_TYPE_Q4_0
            | crate::ggml_quants::GGML_TYPE_Q5_0
            | crate::ggml_quants::GGML_TYPE_Q8_0
            | crate::ggml_quants::GGML_TYPE_Q4_K
            | crate::ggml_quants::GGML_TYPE_Q6_K
    )
}

/// Await `map_async` without `poll(Wait)` — yields to the browser event loop (MC6).
#[cfg(target_arch = "wasm32")]
async fn await_wgpu_map(slice: wgpu::BufferSlice<'_>) -> bool {
    let (tx, rx) = futures_channel::oneshot::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    matches!(rx.await, Ok(Ok(())))
}

#[cfg(target_arch = "wasm32")]
impl WasmGpuPipeline {
    fn begin(engine: &QTensorEngine) -> Self {
        Self {
            encoder: engine.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MC8FusedEncoder"),
            }),
        }
    }

    fn finish(self) -> wgpu::CommandBuffer {
        self.encoder.finish()
    }
}

#[cfg(target_arch = "wasm32")]
impl QTensorEngine {
    fn mc8_flush(&self, pipeline: &mut WasmGpuPipeline) {
        let finished = std::mem::replace(pipeline, WasmGpuPipeline::begin(self));
        self.gpu_queue().submit(Some(finished.finish()));
    }

    fn mc8_weight_role_buf(&self, role: Mc8WeightRole) -> &wgpu::Buffer {
        let arena = self.mc8_weight_arena.as_ref().expect("mc8 weight arena");
        match role {
            Mc8WeightRole::AttnK => &arena.qkv_k,
            Mc8WeightRole::AttnV => &arena.qkv_v,
            Mc8WeightRole::AttnQ => &arena.qkv_q,
            Mc8WeightRole::OProj => &arena.o_proj,
            Mc8WeightRole::Gate => &arena.gate,
            Mc8WeightRole::Up => &arena.up,
            Mc8WeightRole::Down => &arena.down,
        }
    }

    fn write_weight_role(&self, role: Mc8WeightRole, raw: &[u8], max_bytes: usize) {
        let weight_buf = self.mc8_weight_role_buf(role);
        let upload = if raw.len() <= max_bytes {
            raw
        } else {
            &raw[..max_bytes]
        };
        self.gpu_queue().write_buffer(weight_buf, 0, upload);
    }

    /// MC8 Part 3x: weight binding for `role` at `layer`. When weights are resident, binds the
    /// per-layer sub-range `[layer*stride, layer*stride + stride)`; otherwise the whole (single-
    /// layer) role buffer that the caller just `write_weight_role`'d.
    fn mc8_weight_binding(&self, role: Mc8WeightRole, layer: u32) -> wgpu::BindingResource<'_> {
        let buf = self.mc8_weight_role_buf(role);
        if self.mc8_weights_resident {
            let stride = self.mc8_weight_role_stride[role.idx()];
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: buf,
                offset: layer as u64 * stride,
                size: std::num::NonZeroU64::new(stride.max(4)),
            })
        } else {
            buf.as_entire_binding()
        }
    }

    /// MC8 Part 3x: upload every layer's K/V/Q/O/gate/up/down weights to the GPU **once**, into
    /// 7 role buffers each sized `stride * n_layer` (256-byte-aligned per-role stride). After this
    /// the hot-path encoders bind per-layer sub-ranges instead of re-`write_buffer`ing ~208 MB of
    /// model weights every forward pass. Returns false (leaving `mc8_weights_resident=false`, so
    /// callers fall back to per-forward upload) if any role tensor is missing.
    #[cfg(target_arch = "wasm32")]
    fn mc8_upload_all_resident_weights(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
    ) -> bool {
        if self.mc8_weights_resident {
            return true;
        }
        // Clone the Arc so the mmap borrow does not block mutating `self` below.
        let mmap_arc = match self.gguf_mmap.clone() {
            Some(a) => a,
            None => return false,
        };
        let mmap: &[u8] = &mmap_arc;
        let tds = index.tensor_data_start;
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || n_layer as usize > 1024 {
            return false;
        }
        const ROLES: [Mc8WeightRole; 7] = [
            Mc8WeightRole::AttnK,
            Mc8WeightRole::AttnV,
            Mc8WeightRole::AttnQ,
            Mc8WeightRole::OProj,
            Mc8WeightRole::Gate,
            Mc8WeightRole::Up,
            Mc8WeightRole::Down,
        ];
        fn role_info<'a>(
            role: Mc8WeightRole,
            t: &'a crate::gguf_sharder::LayerTensors,
        ) -> Option<&'a GgufTensorInfo> {
            match role {
                Mc8WeightRole::AttnK => t.attn_k.as_ref(),
                Mc8WeightRole::AttnV => t.attn_v.as_ref(),
                Mc8WeightRole::AttnQ => t.attn_q.as_ref(),
                Mc8WeightRole::OProj => t.attn_output.as_ref(),
                Mc8WeightRole::Gate => t.ffn_gate.as_ref(),
                Mc8WeightRole::Up => t.ffn_up.as_ref(),
                Mc8WeightRole::Down => t.ffn_down.as_ref(),
            }
        }
        // Pass 1: max byte length per role across all layers → 256-aligned stride.
        let mut max_len = [0usize; 7];
        for layer in 0..n_layer {
            let t = index.get_layer_tensors(layer);
            for role in ROLES {
                let info = match role_info(role, &t) {
                    Some(i) => i,
                    None => {
                        wlog(&format!(
                            "[MC8] resident weights: role {:?} missing at layer {layer} — per-forward fallback",
                            role
                        ));
                        return false;
                    }
                };
                let len = match crate::ggml_quants::fetch_tensor_bytes(mmap, tds, info) {
                    Ok(s) => s.len(),
                    Err(_) => return false,
                };
                let i = role.idx();
                if len > max_len[i] {
                    max_len[i] = len;
                }
            }
        }
        let mut stride = [0u64; 7];
        let mut total_bytes = 0u64;
        for i in 0..7 {
            let s = (((max_len[i] + 255) & !255).max(256)) as u64;
            stride[i] = s;
            total_bytes += s * n_layer as u64;
        }
        // Allocate the 7 resident role buffers (replaces the single-layer arena).
        let usage = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let arena = {
            let dev = self.gpu_device();
            let mk = |i: usize, label: &str| {
                dev.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(label),
                    size: stride[i] * n_layer as u64,
                    usage,
                    mapped_at_creation: false,
                })
            };
            Mc8WeightArenaBufs {
                qkv_k: mk(0, "MC8ResidentAttnK"),
                qkv_v: mk(1, "MC8ResidentAttnV"),
                qkv_q: mk(2, "MC8ResidentAttnQ"),
                o_proj: mk(3, "MC8ResidentOProj"),
                gate: mk(4, "MC8ResidentGate"),
                up: mk(5, "MC8ResidentUp"),
                down: mk(6, "MC8ResidentDown"),
            }
        };
        self.mc8_weight_arena = Some(arena);
        self.mc8_weight_role_stride = stride;
        // Pass 2: upload every layer's weights into its resident slot.
        {
            let queue = self.gpu_queue();
            for layer in 0..n_layer {
                let t = index.get_layer_tensors(layer);
                for role in ROLES {
                    let info = match role_info(role, &t) {
                        Some(i) => i,
                        None => return false,
                    };
                    let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, tds, info) {
                        Ok(s) => s,
                        Err(_) => return false,
                    };
                    let off = layer as u64 * stride[role.idx()];
                    queue.write_buffer(self.mc8_weight_role_buf(role), off, raw);
                }
            }
        }
        self.mc8_weights_resident = true;
        wlog(&format!(
            "[MC8] resident weights uploaded once: {:.1} MB across {} layers (Part 3x)",
            total_bytes as f64 / (1024.0 * 1024.0),
            n_layer
        ));
        true
    }

    /// Phase 5.3: upload the output/logits projection (tied `token_embd`, ~50 MB Q8_0) to a
    /// dedicated resident `STORAGE` buffer **once**, so the per-token argmax binds per-chunk
    /// sub-ranges instead of `write_buffer`-ing the whole matrix every token (the decode
    /// throughput killer — Phase 5 root cause). Idempotent. Returns false (→ per-token upload
    /// fallback) if the projection is missing or its bytes don't divide evenly into rows.
    fn mc8_upload_resident_logits(&mut self, index: &crate::gguf_sharder::GgufTensorIndex) -> bool {
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
        // VOCAB_CHUNK_ROWS is a multiple of 256, so every chunk's byte offset
        // (chunk_idx * VOCAB_CHUNK_ROWS * row_bytes) is 256-aligned for the storage binding.
        let buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("MC8ResidentLogits"),
            size: total as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&buf, 0, raw);
        self.mc8_logits_resident_buf = Some(buf);
        self.mc8_logits_row_bytes = row_bytes as u32;
        wlog(&format!(
            "[MC8] resident logits projection uploaded once: {:.1} MB ({} rows × {} B)",
            total as f64 / (1024.0 * 1024.0),
            vocab,
            row_bytes
        ));
        true
    }

    /// Phase 5.4: upload every layer's attn_norm + ffn_norm weights to a resident buffer **once**
    /// (slot `2L` = attn_norm, `2L+1` = ffn_norm; per-slot stride 256-aligned for binding), so the
    /// hot-path RMSNorm binds a per-layer sub-range instead of `write_buffer`-ing the shared
    /// single-layer `norm_weight_buf` every layer. Removes the second per-layer write_buffer race
    /// (the first being the super-arena uniforms) that forces the per-layer submit flush.
    fn mc8_upload_resident_norms(&mut self, index: &crate::gguf_sharder::GgufTensorIndex) -> bool {
        if self.mc8_norm_resident_buf.is_some() {
            return true;
        }
        let n_layer = index.hyperparams.n_layer;
        let n_embd = index.hyperparams.n_embd as usize;
        if n_layer == 0 || n_embd == 0 || n_embd > MAX_HIDDEN_DIM {
            return false;
        }
        let mmap_arc = match self.gguf_mmap.clone() {
            Some(a) => a,
            None => return false,
        };
        let mmap: &[u8] = &mmap_arc;
        let tds = index.tensor_data_start;
        let stride_bytes = (((n_embd * 4) + 255) & !255) as u64;
        let slots = n_layer as u64 * 2;
        let buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("MC8ResidentNorms"),
            size: slots * stride_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let queue = self.gpu_queue();
        let mut tmp = [0f32; MAX_HIDDEN_DIM];
        for layer in 0..n_layer {
            let t = index.get_layer_tensors(layer);
            if let Some(info) = t.attn_norm.as_ref() {
                if dequant_norm_row_into(mmap, tds, info, &mut tmp) < n_embd {
                    return false;
                }
                let off = (2 * layer as u64) * stride_bytes;
                queue.write_buffer(&buf, off, bytemuck::cast_slice(&tmp[..n_embd]));
            }
            if let Some(info) = t.ffn_norm.as_ref() {
                if dequant_norm_row_into(mmap, tds, info, &mut tmp) < n_embd {
                    return false;
                }
                let off = (2 * layer as u64 + 1) * stride_bytes;
                queue.write_buffer(&buf, off, bytemuck::cast_slice(&tmp[..n_embd]));
            }
        }
        self.mc8_norm_resident_buf = Some(buf);
        self.mc8_norm_stride = stride_bytes as u32;
        wlog(&format!(
            "[MC8] resident norm weights uploaded once: {} slots × {} B",
            slots, stride_bytes
        ));
        true
    }

    /// RMSNorm weight source: the resident norm arena sub-range (no per-layer upload → race-free)
    /// when available, else the per-layer `norm_weight_buf` upload fallback. `is_ffn` picks the
    /// ffn_norm slot (`2L+1`) vs attn_norm (`2L`).
    fn mc8_norm_source(
        &self,
        mmap: &[u8],
        tensor_data_start: u64,
        info: &GgufTensorInfo,
        n_embd: usize,
        layer: u32,
        is_ffn: bool,
    ) -> Option<(&wgpu::Buffer, wgpu::BufferAddress)> {
        if let Some(buf) = self.mc8_norm_resident_buf.as_ref() {
            let slot = 2u64 * layer as u64 + if is_ffn { 1 } else { 0 };
            Some((buf, slot * self.mc8_norm_stride as u64))
        } else {
            if !self.upload_norm_weights(mmap, tensor_data_start, info, n_embd) {
                return None;
            }
            Some((self.norm_weight_buf.as_ref().unwrap(), 0))
        }
    }

    /// Phase 4: boot from a `.q42` weight container. Validates integrity (CRC via `from_q42`), builds
    /// a synthetic `GgufTensorIndex` from the manifest, points the byte source at the `.q42` bytes
    /// (`tensor_data_start = 0`, absolute blob offsets), reserves the GEMM/KV arenas, and uploads the
    /// resident weights via the **standard** path — so the entire GGUF hot path runs unchanged
    /// (format-agnostic). NOTE: the `.q42` weight container does not yet carry the tokenizer, so a
    /// q42-only boot maps weights/params but cannot tokenize until a tokenizer section lands.
    fn adopt_resident_q42(&mut self, data: Arc<[u8]>) -> Result<(), String> {
        let q = crate::q42_weight::Q42TensorIndex::from_q42(&data)?;
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
        // Byte source for fetch_tensor_bytes (tensor_data_start=0 + absolute blob offsets).
        self.gguf_mmap = Some(data.clone());
        self.q42_resident = Some(data);
        // Resident weight upload reuses the standard path through the synthetic index.
        if !self.mc8_upload_all_resident_weights(&index) {
            wlog("[Q42] eager resident upload skipped — will retry lazily");
        }
        if !self.mc8_upload_resident_logits(&index) {
            wlog("[Q42] resident logits projection skipped — per-token upload fallback");
        }
        if !self.mc8_upload_resident_norms(&index) {
            wlog("[Q42] resident norm weights skipped — per-layer upload fallback");
        }
        wlog(&format!(
            "[Q42] boot OK: {} tensors, {} layers (synthetic GGUF index; weights resident)",
            q.entries.len(),
            hp.n_layer
        ));
        Ok(())
    }

    fn mc8_dynamic_uniform_binding(buf: &wgpu::Buffer) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: buf,
            offset: 0,
            size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
        })
    }

    fn mc8_gemm_params(
        info: &GgufTensorInfo,
        raw_len: usize,
        n_in: usize,
        n_out: usize,
        n_batch: u32,
        in_row_stride: u32,
        out_row_stride: u32,
    ) -> GemmGpuParams {
        GemmGpuParams {
            n_in: n_in as u32,
            n_out: n_out as u32,
            weight_ggml_type: info.ggml_type,
            weight_row_elems: info.dims[0] as u32,
            weight_byte_len: raw_len as u32,
            n_batch: n_batch.max(1),
            in_row_stride,
            out_row_stride,
        }
    }

    fn mc8_upload_attn_param(&self, params: &AttentionGpuParams) -> u32 {
        let mut arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let off = arena.push(params);
        arena.upload(
            self.gpu_queue(),
            self.attention_params_buf.as_ref().expect("attn params buf"),
        );
        off
    }

    fn mc8_elem_params(
        op: u32,
        n: u32,
        batch: u32,
        a_row_stride: u32,
        b_row_stride: u32,
        out_row_stride: u32,
        a_slot: u32,
        b_slot: u32,
        out_slot: u32,
    ) -> ElemGpuParams {
        ElemGpuParams {
            n,
            batch: batch.max(1),
            op,
            eps: RMS_NORM_EPS,
            a_row_stride,
            b_row_stride,
            out_row_stride,
            a_slot,
            b_slot,
            out_slot,
            _pad: 0,
        }
    }

    fn mc8_buf_slice(
        buf: &wgpu::Buffer,
        byte_off: wgpu::BufferAddress,
        byte_len: wgpu::BufferAddress,
    ) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: buf,
            offset: byte_off,
            size: std::num::NonZeroU64::new(byte_len.max(4)),
        })
    }

    fn mc8_prefill_row_off(t: u32, row_stride_floats: usize) -> wgpu::BufferAddress {
        (t as usize * row_stride_floats * 4) as wgpu::BufferAddress
    }

    fn mc8_emb_off(t: u32, n_embd: usize) -> wgpu::BufferAddress {
        (t as usize * n_embd * 4) as wgpu::BufferAddress
    }

    fn encode_elem_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        op: u32,
        n: u32,
        batch: u32,
        a: &wgpu::Buffer,
        a_off: wgpu::BufferAddress,
        a_bytes: wgpu::BufferAddress,
        b: &wgpu::Buffer,
        b_off: wgpu::BufferAddress,
        b_bytes: wgpu::BufferAddress,
        out: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        a_row_stride: u32,
        b_row_stride: u32,
        out_row_stride: u32,
        a_slot: u32,
        b_slot: u32,
        out_slot: u32,
        elem_dyn_offset: u32,
    ) {
        let b_dispatch = batch.max(1);
        let (pipe, wg_x, wg_y) = match op {
            ELEM_OP_RMS_NORM => (&self.elem_rms_norm_pipeline, 1u32, b_dispatch),
            ELEM_OP_SILU_MUL => (&self.elem_silu_mul_pipeline, (n + 63) / 64, b_dispatch),
            ELEM_OP_ADD_RESIDUAL => (&self.elem_add_residual_pipeline, (n + 63) / 64, b_dispatch),
            _ => return,
        };
        let params_buf = self.elem_params_buf.as_ref().unwrap();
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ElemBindOff"),
            layout: &self.mc8_elem_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: Self::mc8_buf_slice(a, a_off, a_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: Self::mc8_buf_slice(b, b_off, b_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: Self::mc8_buf_slice(out, out_off, out_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
            ],
        });
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(pipe);
        cpass.set_bind_group(0, &bind, &[elem_dyn_offset]);
        cpass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    fn encode_gemm_bufs_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        info: &GgufTensorInfo,
        raw: &[u8],
        n_in: usize,
        n_out: usize,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_batch: u32,
        in_row_stride: u32,
        out_row_stride: u32,
        gemm_dyn_offset: u32,
        layer: u32,
        weight_role: Mc8WeightRole,
    ) -> bool {
        if !ggml_gpu_attention_shader_supported(info.ggml_type)
            || n_in > self.gemm_max_input_floats as usize
            || n_out > self.gemm_max_out_dim as usize
            || raw.len() > self.max_tensor_bytes
        {
            return false;
        }
        let batch = n_batch.max(1);
        if !self.mc8_weights_resident {
            self.write_weight_role(weight_role, raw, self.max_tensor_bytes);
        }
        let params_buf = self.gemm_params_buf.as_ref().unwrap();
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MC8GemmBindOff"),
            layout: &self.mc8_gemm_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: Self::mc8_buf_slice(input, in_off, in_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.mc8_weight_binding(weight_role, layer),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: Self::mc8_buf_slice(output, out_off, out_bytes),
                },
            ],
        });
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bind, &[gemm_dyn_offset]);
        cpass.dispatch_workgroups((n_out as u32 + 63) / 64, batch, 1);
        true
    }

    /// Phase 5 dispatch fusion: `silu(gate·x) * (up·x)` in a single compute pass.
    /// Binds both resident gate+up weight sub-ranges for `layer` and reuses the gate
    /// GEMM's staged `GemmParams` (`gemm_dyn_offset`) — valid because the caller has
    /// verified gate and up share ggml_type + dims. Replaces the gate GEMM + up GEMM +
    /// SiLU×mul elementwise (3 dispatches → 1) and skips two n_ffn VRAM round-trips.
    /// Requires resident weights (`mc8_weights_resident`); caller falls back otherwise.
    #[cfg(target_arch = "wasm32")]
    fn encode_fused_ffn_expansion(
        &self,
        pipeline: &mut WasmGpuPipeline,
        layer: u32,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_out: u32,
        n_batch: u32,
        gemm_dyn_offset: u32,
    ) -> bool {
        let params_buf = self.gemm_params_buf.as_ref().unwrap();
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MC8FfnFusedBind"),
            layout: &self.mc8_ffn_fused_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: Self::mc8_buf_slice(input, in_off, in_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.mc8_weight_binding(Mc8WeightRole::Gate, layer),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.mc8_weight_binding(Mc8WeightRole::Up, layer),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: Self::mc8_buf_slice(output, out_off, out_bytes),
                },
            ],
        });
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.mc8_ffn_fused_pipeline);
        cpass.set_bind_group(0, &bind, &[gemm_dyn_offset]);
        cpass.dispatch_workgroups((n_out + 63) / 64, n_batch.max(1), 1);
        true
    }

    fn encode_residual_add_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        base: &wgpu::Buffer,
        base_off: wgpu::BufferAddress,
        delta: &wgpu::Buffer,
        delta_off: wgpu::BufferAddress,
        dst: &wgpu::Buffer,
        dst_off: wgpu::BufferAddress,
        scratch: &wgpu::Buffer,
        scratch_off: wgpu::BufferAddress,
        dim: u32,
    ) {
        let bytes = (dim as usize * 4) as wgpu::BufferAddress;
        self.encode_elem_offset(
            pipeline,
            ELEM_OP_ADD_RESIDUAL,
            dim,
            1,
            base,
            base_off,
            bytes,
            delta,
            delta_off,
            bytes,
            scratch,
            scratch_off,
            bytes,
            0,
            0,
            0,
            0,
            0,
            0,
            0, // elem_dyn_offset — caller must pre-upload arena
        );
        pipeline.encoder.copy_buffer_to_buffer(scratch, scratch_off, dst, dst_off, bytes);
    }

    fn encode_attention_pass_gpu_offset(
        &self,
        pipeline: &mut WasmGpuPipeline,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
    ) -> bool {
        if !ggml_gpu_attention_shader_supported(info.ggml_type)
            || self.kv_cache_gpu.is_none()
            || self.attention_params_buf.is_none()
        {
            return false;
        }
        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(layout, token_idx, proj_kind);
        let params = Self::attention_gpu_params(
            h,
            layout,
            layer,
            token_idx,
            info,
            raw_weights.len(),
            proj_kind,
            num_tokens_in_batch.max(1),
            batch_start_token_idx,
            mask_active,
            mask_word_count,
            0,
        );
        self.write_weight_words(raw_weights, self.max_tensor_bytes);
        self.gpu_queue()
            .write_buffer(self.attention_params_buf.as_ref().unwrap(), 0, bytemuck::bytes_of(&params));
        self.gpu_queue().write_buffer(
            self.attention_mask_buf.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&mask_words),
        );
        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset =
            (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding {
            buffer: self.kv_cache_gpu.as_ref().unwrap(),
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };
        let bind_layout = self.attention_pipeline.get_bind_group_layout(0);
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MC8AttnBindOff"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: Self::mc8_buf_slice(input, in_off, in_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.gemm_weight_buf.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.attention_params_buf.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(kv_binding),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: Self::mc8_buf_slice(output, out_off, out_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.attention_mask_buf.as_ref().unwrap().as_entire_binding(),
                },
            ],
        });
        let (wg_x, wg_y) = if proj_kind == 0 && num_tokens_in_batch > 1 {
            (h.n_head, num_tokens_in_batch)
        } else {
            (n_workgroups.max(1), 1)
        };
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.attention_pipeline);
        cpass.set_bind_group(0, &bind, &[]);
        cpass.dispatch_workgroups(wg_x, wg_y, 1);
        true
    }

    fn mc8_buffers_ready(&self) -> bool {
        self.gemm_input_buf.is_some()
            && self.gemm_output_buf.is_some()
            && self.gemm_aux_buf.is_some()
            && self.gemm_ffn_buf.is_some()
            && self.elem_params_buf.is_some()
            && self.norm_weight_buf.is_some()
            && self.prefill_scratch_buf.is_some()
            && self.prefill_work_buf_a.is_some()
            && self.prefill_work_buf_b.is_some()
            && self.mc8_weight_arena.is_some()
    }

    fn encode_elem(
        &self,
        pipeline: &mut WasmGpuPipeline,
        op: u32,
        n: u32,
        batch: u32,
        a: &wgpu::Buffer,
        b: &wgpu::Buffer,
        out: &wgpu::Buffer,
    ) {
        let b_dispatch = batch.max(1);
        let params = ElemGpuParams {
            n,
            batch: b_dispatch,
            op,
            eps: RMS_NORM_EPS,
            a_row_stride: 0,
            b_row_stride: 0,
            out_row_stride: 0,
            a_slot: 0,
            b_slot: 0,
            out_slot: 0,
            _pad: 0,
        };
        let mut arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let elem_dyn_offset = arena.push(&params);
        let params_buf = self.elem_params_buf.as_ref().unwrap();
        arena.upload(self.gpu_queue(), params_buf);
        let (pipe, wg_x, wg_y) = match op {
            ELEM_OP_RMS_NORM => (&self.elem_rms_norm_pipeline, 1u32, b_dispatch),
            ELEM_OP_SILU_MUL => (&self.elem_silu_mul_pipeline, (n + 63) / 64, b_dispatch),
            ELEM_OP_ADD_RESIDUAL => (&self.elem_add_residual_pipeline, (n + 63) / 64, b_dispatch),
            _ => return,
        };
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ElemBind"),
            layout: &self.mc8_elem_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: a.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: out.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
            ],
        });
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(pipe);
        cpass.set_bind_group(0, &bind, &[elem_dyn_offset]);
        cpass.dispatch_workgroups(wg_x, wg_y, 1);
    }

    /// Residual add with disjoint storage bindings (WebGPU forbids aliasing read + read_write).
    /// `scratch` must not alias `base` or `delta` (MC8 pt3b: never reuse `gemm_ffn_buf` here —
    /// it holds SwiGLU up / Q proj and aliases `base_save` fallback).
    fn encode_residual_add_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        base: &wgpu::Buffer,
        delta: &wgpu::Buffer,
        dst: &wgpu::Buffer,
        scratch: &wgpu::Buffer,
        dim: u32,
    ) {
        self.encode_elem(
            pipeline,
            ELEM_OP_ADD_RESIDUAL,
            dim,
            1,
            base,
            delta,
            scratch,
        );
        let bytes = (dim as usize * 4) as wgpu::BufferAddress;
        pipeline
            .encoder
            .copy_buffer_to_buffer(scratch, 0, dst, 0, bytes);
    }

    fn encode_gemm_bufs(
        &self,
        pipeline: &mut WasmGpuPipeline,
        info: &GgufTensorInfo,
        raw: &[u8],
        n_in: usize,
        n_out: usize,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
    ) -> bool {
        if !ggml_gpu_attention_shader_supported(info.ggml_type)
            || n_in > self.gemm_max_input_floats as usize
            || n_out > self.gemm_max_out_dim as usize
            || raw.len() > self.max_tensor_bytes
        {
            return false;
        }
        let params = Self::mc8_gemm_params(info, raw.len(), n_in, n_out, 1, 0, 0);
        let mut arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let gemm_dyn_offset = arena.push(&params);
        let params_buf = self.gemm_params_buf.as_ref().unwrap();
        arena.upload(self.gpu_queue(), params_buf);
        self.write_weight_words(raw, self.max_tensor_bytes);
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MC8GemmBind"),
            layout: &self.mc8_gemm_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.gemm_weight_buf.as_ref().unwrap().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.pipeline);
        cpass.set_bind_group(0, &bind, &[gemm_dyn_offset]);
        cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
        true
    }

    fn encode_attention_pass_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        input: &wgpu::Buffer,
        output: &wgpu::Buffer,
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
        attn_dyn_offset: u32,
        weight_role: Mc8WeightRole,
    ) -> bool {
        if !ggml_gpu_attention_shader_supported(info.ggml_type)
            || self.kv_cache_gpu.is_none()
            || self.attention_params_buf.is_none()
        {
            return false;
        }
        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(layout, token_idx, proj_kind);
        if !self.mc8_weights_resident {
            self.write_weight_role(weight_role, raw_weights, self.max_tensor_bytes);
        }
        if mask_active != 0 {
            self.gpu_queue().write_buffer(
                self.attention_mask_buf.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&mask_words),
            );
        }
        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset =
            (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding {
            buffer: self.kv_cache_gpu.as_ref().unwrap(),
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };
        let params_buf = self.attention_params_buf.as_ref().unwrap();
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MC8AttnBind"),
            layout: &self.mc8_attn_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.mc8_weight_binding(weight_role, layer),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(kv_binding),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.attention_mask_buf.as_ref().unwrap().as_entire_binding(),
                },
            ],
        });
        let (wg_x, wg_y) = if proj_kind == 0 && num_tokens_in_batch > 1 {
            (h.n_head, num_tokens_in_batch)
        } else {
            (n_workgroups.max(1), 1)
        };
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.attention_pipeline);
        cpass.set_bind_group(0, &bind, &[attn_dyn_offset]);
        cpass.dispatch_workgroups(wg_x, wg_y, 1);
        true
    }

    #[cfg(target_arch = "wasm32")]
    fn mc8_prefill_row_stride(n_embd: usize, n_ffn_est: usize, gemm_max_out_dim: u32) -> usize {
        (n_embd + 2 * n_ffn_est + n_embd)
            .max(gemm_max_out_dim as usize * 2)
            .max(n_embd)
    }

    /// MC8 Part 3o/3t: batched Q-SDPA — caller pre-uploads `attn_dyn_offset` in shared attn arena.
    #[cfg(target_arch = "wasm32")]
    fn encode_attention_batched_q_prefill(
        &self,
        pipeline: &mut WasmGpuPipeline,
        input: &wgpu::Buffer,
        in_off: wgpu::BufferAddress,
        in_bytes: wgpu::BufferAddress,
        output: &wgpu::Buffer,
        out_off: wgpu::BufferAddress,
        out_bytes: wgpu::BufferAddress,
        n_embd: usize,
        n_tokens: u32,
        layout: &KvCacheLayout,
        layer: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        raw_weights: &[u8],
        attn_dyn_offset: u32,
    ) -> bool {
        if self.kv_cache_gpu.is_none()
            || self.attention_params_buf.is_none()
            || self.attention_mask_buf.is_none()
            || n_tokens == 0
        {
            return false;
        }
        if !self.mc8_weights_resident {
            self.write_weight_role(Mc8WeightRole::AttnQ, raw_weights, self.max_tensor_bytes);
        }
        let params_buf = self.attention_params_buf.as_ref().unwrap();
        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset =
            (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding {
            buffer: self.kv_cache_gpu.as_ref().unwrap(),
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MC8AttnBatchedQ"),
            layout: &self.mc8_attn_bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: Self::mc8_buf_slice(input, in_off, in_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.mc8_weight_binding(Mc8WeightRole::AttnQ, layer),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(kv_binding),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: Self::mc8_buf_slice(output, out_off, out_bytes),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.attention_mask_buf.as_ref().unwrap().as_entire_binding(),
                },
            ],
        });
        let mut cpass = pipeline.encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.attention_pipeline);
        cpass.set_bind_group(0, &bind, &[attn_dyn_offset]);
        cpass.dispatch_workgroups(h.n_head, n_tokens, 1);
        true
    }

    fn upload_norm_weights(&self, mmap: &[u8], tensor_data_start: u64, info: &GgufTensorInfo, n: usize) -> bool {
        let mut norm_w = [0f32; MAX_HIDDEN_DIM];
        if dequant_norm_row_into(mmap, tensor_data_start, info, &mut norm_w) < n {
            return false;
        }
        self.gpu_queue().write_buffer(
            self.norm_weight_buf.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&norm_w[..n]),
        );
        true
    }

    async fn pipeline_read_hidden(&self, emb_dim: usize, hidden: &mut [f32]) -> bool {
        let staging = self.gemm_output_staging.as_ref().unwrap();
        let hidden_buf = self.gemm_input_buf.as_ref().unwrap();
        let out_bytes = (emb_dim * 4) as wgpu::BufferAddress;
        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MC8Readback"),
        });
        encoder.copy_buffer_to_buffer(hidden_buf, 0, staging, 0, out_bytes);
        self.gpu_queue().submit(Some(encoder.finish()));
        let slice = staging.slice(..out_bytes);
        if !await_wgpu_map(slice).await {
            let _ = staging.unmap();
            return false;
        }
        let data = slice.get_mapped_range();
        let floats: &[f32] = bytemuck::cast_slice(&data);
        hidden[..emb_dim].copy_from_slice(&floats[..emb_dim]);
        drop(data);
        staging.unmap();
        true
    }

    async fn pipeline_read_batch(&self, batch_elems: usize, out: &mut [f32]) -> bool {
        if batch_elems > out.len() || batch_elems > self.gemm_max_input_floats {
            return false;
        }
        let staging = self.gemm_output_staging.as_ref().unwrap();
        let batch_buf = self.gemm_output_buf.as_ref().unwrap();
        let out_bytes = (batch_elems * 4) as wgpu::BufferAddress;
        if out_bytes > staging.size() {
            return false;
        }
        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MC8BatchReadback"),
        });
        encoder.copy_buffer_to_buffer(batch_buf, 0, staging, 0, out_bytes);
        self.gpu_queue().submit(Some(encoder.finish()));
        let slice = staging.slice(..out_bytes);
        if !await_wgpu_map(slice).await {
            let _ = staging.unmap();
            return false;
        }
        let data = slice.get_mapped_range();
        let floats: &[f32] = bytemuck::cast_slice(&data);
        out[..batch_elems].copy_from_slice(&floats[..batch_elems]);
        drop(data);
        staging.unmap();
        true
    }

    async fn pipeline_read_gpu_bytes_at(
        &self,
        src: &wgpu::Buffer,
        byte_offset: wgpu::BufferAddress,
        out: &mut [u8],
    ) -> bool {
        if out.is_empty() {
            return false;
        }
        let staging = self.gemm_output_staging.as_ref().unwrap();
        let nbytes = out.len() as wgpu::BufferAddress;
        if nbytes > staging.size() {
            return false;
        }
        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MC8ProbeReadback"),
        });
        encoder.copy_buffer_to_buffer(src, byte_offset, staging, 0, nbytes);
        self.gpu_queue().submit(Some(encoder.finish()));
        let slice = staging.slice(..nbytes);
        if !await_wgpu_map(slice).await {
            let _ = staging.unmap();
            return false;
        }
        let data = slice.get_mapped_range();
        out.copy_from_slice(&data);
        drop(data);
        staging.unmap();
        true
    }

    async fn pipeline_read_kv_head(
        &self,
        layout: &KvCacheLayout,
        layer: u32,
        slot: u32,
        kv_h: u32,
        head_dim: usize,
        k_not_v: bool,
        out: &mut [f32],
    ) -> bool {
        if head_dim == 0 || head_dim > out.len() {
            return false;
        }
        let kv = match self.kv_cache_gpu.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let idx = if k_not_v {
            layout.k_index(layer, slot, kv_h, 0)
        } else {
            layout.v_index(layer, slot, kv_h, 0)
        };
        let byte_off = (idx * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let mut bytes = [0u8; 512];
        let nbytes = head_dim * std::mem::size_of::<f32>();
        if nbytes > bytes.len() {
            return false;
        }
        if !self
            .pipeline_read_gpu_bytes_at(kv, byte_off, &mut bytes[..nbytes])
            .await
        {
            return false;
        }
        let floats: &[f32] = bytemuck::cast_slice(&bytes[..nbytes]);
        out[..head_dim].copy_from_slice(floats);
        true
    }
}

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
const RMS_NORM_EPS: f32 = 1e-5;
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

#[inline]
fn bytes_to_gib(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0 * 1024.0)
}

#[inline]
fn scrub_f32_volatile(buf: &mut [f32], n: usize) {
    for v in buf.iter_mut().take(n) {
        // Prevent logits residue from surviving across decode frames.
        unsafe { core::ptr::write_volatile(v, 0.0) };
    }
}

#[inline]
fn update_streaming_argmax(
    chunk_logits: &[f32],
    chunk_rows: usize,
    chunk_idx: usize,
    best_token_id: &mut u32,
    max_logit: &mut f32,
) {
    update_streaming_argmax_sieved(
        chunk_logits,
        chunk_rows,
        chunk_idx,
        None,
        best_token_id,
        max_logit,
    );
}

/// Chunked argmax with optional FSM sieve mask (disallowed tokens treated as `-∞`).
#[inline]
fn update_streaming_argmax_sieved(
    chunk_logits: &[f32],
    chunk_rows: usize,
    chunk_idx: usize,
    sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    best_token_id: &mut u32,
    max_logit: &mut f32,
) {
    let base = chunk_idx * VOCAB_CHUNK_ROWS;
    for (local, &v) in chunk_logits.iter().take(chunk_rows).enumerate() {
        let abs_id = (base + local) as u32;
        let score = if sieve_mask.map(|m| m.allows(abs_id)).unwrap_or(true) {
            v
        } else {
            f32::NEG_INFINITY
        };
        if score > *max_logit {
            *max_logit = score;
            *best_token_id = abs_id;
        }
    }
}

#[inline]
fn relu_inplace(buf: &mut [f32], n: usize) {
    for v in buf.iter_mut().take(n) {
        if *v < 0.0 {
            *v = 0.0;
        }
    }
}

/// SiLU (Swish): x * sigmoid(x) = x / (1 + e^{-x}). Llama/SmolLM2 SwiGLU gate activation.
#[inline]
fn silu_inplace(x: &mut [f32], n: usize) {
    for v in x.iter_mut().take(n) {
        *v = *v / (1.0 + (-*v).exp());
    }
}

#[inline]
fn add_residual_inplace(dst: &mut [f32], src: &[f32], n: usize) {
    for i in 0..n.min(dst.len()).min(src.len()) {
        dst[i] += src[i];
    }
}

#[inline]
fn rms_norm_inplace(x: &mut [f32], weight: &[f32], eps: f32) {
    let n = x.len().min(weight.len());
    if n == 0 {
        return;
    }
    let mut ss = 0.0f32;
    for i in 0..n {
        ss += x[i] * x[i];
    }
    ss /= n as f32;
    let inv_rms = 1.0 / (ss + eps).sqrt();
    for i in 0..n {
        x[i] = x[i] * inv_rms * weight[i];
    }
}

/// Dequantize a 1-D norm weight row (`attn_norm` / `ffn_norm` / `output_norm`) into `out`.
fn dequant_norm_row_into(
    mmap: &[u8],
    tensor_data_start: u64,
    info: &GgufTensorInfo,
    out: &mut [f32],
) -> usize {
    let n = info.dims[0] as usize;
    if n == 0 || n > out.len() {
        return 0;
    }
    let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, tensor_data_start, info) {
        Ok(s) => s,
        Err(_) => return 0,
    };
    crate::ggml_quants::dequantize_row_into(raw, info.ggml_type, n, &mut out[..n]).unwrap_or(0)
}

/// Pre-norm: copy `hidden` into `h_norm`, apply RMSNorm with `norm_info` weights; return slice to use.
fn prepare_pre_norm_input<'a>(
    hidden: &'a [f32],
    emb_dim: usize,
    norm_info: Option<&GgufTensorInfo>,
    mmap: Option<&[u8]>,
    tensor_data_start: u64,
    h_norm: &'a mut [f32],
    norm_w: &mut [f32],
) -> &'a [f32] {
    let n_embd = emb_dim.min(hidden.len()).min(h_norm.len());
    if let (Some(mmap), Some(info)) = (mmap, norm_info) {
        if dequant_norm_row_into(mmap, tensor_data_start, info, norm_w) >= n_embd {
            h_norm[..n_embd].copy_from_slice(&hidden[..n_embd]);
            rms_norm_inplace(&mut h_norm[..n_embd], &norm_w[..n_embd], RMS_NORM_EPS);
            return &h_norm[..n_embd];
        }
    }
    &hidden[..n_embd]
}

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
    fn gpu_device(&self) -> &wgpu::Device {
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
    fn gpu_queue(&self) -> &wgpu::Queue {
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fused_transformer.wgsl").into()),
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
                include_str!("shaders/fused_tensor_contraction.wgsl").into(),
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
                include_str!("shaders/quantized_embedding.wgsl").into(),
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fused_attention.wgsl").into()),
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
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/wasm_elementwise.wgsl").into()),
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
            let tpl = include_str!("shaders/dequant_template.wgsl");
            let gate_fns = tpl.replace("$W", "gate_words").replace("$S", "_gate");
            let up_fns = tpl.replace("$W", "up_words").replace("$S", "_up");
            let base = include_str!("shaders/fused_ffn.wgsl");
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

    fn ensure_kv_cache(&mut self, h: &crate::gguf_sharder::GgufHyperparams) {
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

    fn ensure_gemm_buffers(&mut self, max_weight_bytes: usize, max_out_dim: u32) {
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
    fn build_ternary_ffn_resident(&mut self, q: &crate::q42_weight::Q42TensorIndex) -> bool {
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
    fn mc8_upload_resident_logits(&mut self, index: &crate::gguf_sharder::GgufTensorIndex) -> bool {
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
    fn poll_wait(&self) {
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

    /// Upload raw quantized embedding bytes to the GPU and matmul without CPU dequant.
    /// Returns `None` when the GGML type has no WGSL kernel (caller uses CPU fallback).
    pub fn dispatch_quantized_token_embedding(
        &self,
        raw_embd: &[u8],
        ggml_type: u32,
        n_embd: u32,
        weight_tensor: &QTensor,
    ) -> Option<Vec<f32>> {
        if ggml_type != crate::ggml_quants::GGML_TYPE_Q6_K || raw_embd.is_empty() || n_embd == 0 {
            return None;
        }

        let n_output = weight_tensor
            .shape
            .first()
            .copied()
            .unwrap_or(n_embd as usize) as u32;
        let n_embd_u = n_embd;
        let weights_elems = (n_output as usize).saturating_mul(n_embd as usize);

        let params = EmbeddingGpuParams {
            n_embd: n_embd_u,
            ggml_type,
            n_output,
            raw_byte_len: raw_embd.len() as u32,
        };

        // WGSL storage uses u32 words; pad mmap slice to 4-byte alignment.
        let word_bytes = raw_embd.len().div_ceil(4) * 4;
        let embd_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("QuantizedEmbeddingBytes"),
            size: word_bytes.max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if raw_embd.len() == word_bytes {
            self.gpu_queue().write_buffer(&embd_buf, 0, raw_embd);
        } else {
            const MAX_EMB_ROW_PAD: usize = 8192;
            if word_bytes > MAX_EMB_ROW_PAD {
                return None;
            }
            let mut padded = [0u8; MAX_EMB_ROW_PAD];
            padded[..raw_embd.len()].copy_from_slice(raw_embd);
            self.gpu_queue().write_buffer(&embd_buf, 0, &padded[..word_bytes]);
        }

        let params_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingParams"),
            size: std::mem::size_of::<EmbeddingGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue()
            .write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

        let weights_size = (weights_elems * 4).max(4) as wgpu::BufferAddress;
        let weights_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingWeights"),
            size: weights_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + weight_tensor.byte_offset) as usize;
            let end = (offset + weights_elems * 4).min(mmap.len());
            if end > offset {
                self.gpu_queue().write_buffer(&weights_buf, 0, &mmap[offset..end]);
            }
        }

        let output_size = (n_output as usize * 4).max(4) as wgpu::BufferAddress;
        let output_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingOutput"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_layout = self.embedding_pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("QuantizedEmbeddingBindGroup"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: embd_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: weights_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("QuantizedEmbeddingEncoder"),
            });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("QuantizedEmbeddingPass"),
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.embedding_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((n_output + 63) / 64, 1, 1);
        }

        let staging_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingStaging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Embedding);

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = sender.send(v);
        });
        self.poll_wait();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| { let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap())); rt.handle().clone() });
            if handle.block_on(receiver).ok()?.is_err() {
                return None;
            }
        }
        #[cfg(target_arch = "wasm32")]
        { return None; }

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT
            .fetch_add(weights_elems, std::sync::atomic::Ordering::Relaxed);
        Some(result)
    }

    pub fn dispatch_fused_transformer_block(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
        let rows = tensor.shape.get(0).copied().unwrap_or(4096);
        let cols = tensor.shape.get(1).copied().unwrap_or(4096);

        // ── DirectML path (Windows) ───────────────────────────────────────────
        #[cfg(target_os = "windows")]
        if let Some(dml) = &self.dml {
            if let Some(mmap) = &self.gguf_mmap {
                let offset = self.tensor_data_offset + tensor.byte_offset;
                let q4_bytes_needed = (rows * cols / crate::directml_bridge::Q4_K_BLOCK_SIZE)
                    * crate::directml_bridge::Q4_K_BLOCK_BYTES;
                if (offset as usize + q4_bytes_needed) <= mmap.len() {
                    let q4_slice = &mmap[offset as usize..offset as usize + q4_bytes_needed];
                    let weights_f32 =
                        crate::directml_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                    let op = crate::directml_bridge::DmlGemmOp {
                        m: input_activations.len() as u32 / cols as u32,
                        k: cols as u32,
                        n: rows as u32,
                    };
                    if let Ok(result) = op.execute(dml, input_activations, &weights_f32) {
                        crate::telemetry::SIEVE_OPS_COUNT
                            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                        return result;
                    }
                }
            }
        }

        // ── Accelerate BLAS path (macOS / Apple Silicon AMX) ─────────────────────
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let q4_bytes_needed = (rows * cols / crate::metal_bridge::Q4_K_BLOCK_SIZE)
                * crate::metal_bridge::Q4_K_BLOCK_BYTES;
            if offset + q4_bytes_needed <= mmap.len() {
                let q4_slice = &mmap[offset..offset + q4_bytes_needed];
                let weights_f32 =
                    crate::metal_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                let input_rows = (input_activations.len() / cols).max(1);
                let result = crate::metal_bridge::accelerate_sgemm(
                    input_rows,
                    cols,
                    rows,
                    input_activations,
                    &weights_f32,
                );
                crate::telemetry::SIEVE_OPS_COUNT
                    .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                return result;
            }
        }

        // ── wgpu / WGSL fallback (all platforms — Vulkan on Linux/NVIDIA,
        //    Metal on macOS when mmap not loaded, D3D12 on Windows fallback) ──
        let input_bytes = bytemuck::cast_slice(input_activations);
        let input_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input"),
            size: input_bytes.len().max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&input_buf, 0, input_bytes);

        // Upload real weights from mmap when available, else use a zero buffer.
        let weights_size = (rows * cols * 4) as wgpu::BufferAddress;
        let weights_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weights"),
            size: weights_size.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let end = (offset + rows * cols * 4).min(mmap.len());
            if end > offset {
                let f32_bytes = &mmap[offset..end];
                self.gpu_queue().write_buffer(&weights_buf, 0, f32_bytes);
            }
        }

        let output_size = (rows * 4).max(4) as wgpu::BufferAddress;
        let output_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Upload GemmGpuParams for fused_transformer.wgsl (binding 2).
        let gemm_params = GemmGpuParams {
            n_in: cols as u32,
            n_out: rows as u32,
            weight_ggml_type: if tensor.is_quantized_q4_k { 12 } else { 14 },
            weight_row_elems: cols as u32,
            weight_byte_len: (rows * cols * 4) as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let params_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TransformerParams"),
            size: std::mem::size_of::<GemmGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&params_buf, 0, bytemuck::bytes_of(&gemm_params));

        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weights_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((rows as u32 + 63) / 64, 1, 1);
        }

        let staging_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::FusedBlock);

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        self.poll_wait();

        #[cfg(not(target_arch = "wasm32"))]
        {
            let handle = tokio::runtime::Handle::try_current().unwrap_or_else(|_| { let rt = Box::leak(Box::new(tokio::runtime::Runtime::new().unwrap())); rt.handle().clone() });
            handle.block_on(receiver).unwrap().unwrap();
        }

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT
            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
        result
    }

    fn write_weight_words(&self, raw: &[u8], max_bytes: usize) {
        let weight_buf = self.gemm_weight_buf.as_ref().expect("gemm weight buf");
        let upload = if raw.len() <= max_bytes {
            raw
        } else {
            &raw[..max_bytes]
        };
        self.gpu_queue().write_buffer(weight_buf, 0, upload);
    }

    /// Quantized GEMM from a pre-sliced weight byte range (chunk-local row indices).
    fn dispatch_gemm_raw_into(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            return false;
        }

        let weight_bytes = raw.len();
        // GEMM shader supports a wider quant set than the legacy `ggml_gpu_quant_supported` (Q4_K/Q6_K)
        // — notably Q8_0, which was silently falling back to the CPU `stack_gemm_quant` below (the FFN
        // bottleneck for Q8_0 models). The guards (size/buffer caps) still fail closed → CPU fallback.
        if ggml_gpu_gemm_supported(info.ggml_type)
            && n_in <= MAX_STACK_GEMM_IN
            && n_out <= self.gemm_max_out_dim as usize
            && weight_bytes <= self.max_tensor_bytes
            && self.gemm_input_buf.is_some()
        {
            let params = GemmGpuParams {
                n_in: n_in as u32,
                n_out: n_out as u32,
                weight_ggml_type: info.ggml_type,
                weight_row_elems: info.dims[0] as u32,
                weight_byte_len: raw.len() as u32,
                n_batch: 1,
                in_row_stride: 0,
                out_row_stride: 0,
            };
            let input_buf = self.gemm_input_buf.as_ref().unwrap();
            let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
            let output_buf = self.gemm_output_buf.as_ref().unwrap();
            let params_buf = self.gemm_params_buf.as_ref().unwrap();
            let staging = self.gemm_output_staging.as_ref().unwrap();

            self.gpu_queue()
                .write_buffer(input_buf, 0, bytemuck::cast_slice(&input[..n_in]));
            self.write_weight_words(raw, self.max_tensor_bytes);
            self.gpu_queue()
                .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));

            let bind_layout = self.pipeline.get_bind_group_layout(0);
            let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("LayerGemmBindGroup"),
                layout: &bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weight_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buf.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = self
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("LayerGemmEncoder"),
                });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
                });
                cpass.set_pipeline(&self.pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
            }
            let out_bytes = (n_out * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
            crate::llm_gpu_profiler::resolve(&mut encoder);
            self.gpu_queue().submit(Some(encoder.finish()));
            crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm);

            let slice = staging.slice(..out_bytes);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            self.poll_wait();
            #[cfg(not(target_arch = "wasm32"))]
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                if handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
                    let data = slice.get_mapped_range();
                    let floats: &[f32] = bytemuck::cast_slice(&data);
                    out[..n_out].copy_from_slice(&floats[..n_out]);
                    drop(data);
                    staging.unmap();
                    return true;
                }
            }
            let _ = staging.unmap();
        }

        stack_gemm_quant(raw, info, input, out, n_in, n_out)
    }

    /// A1b: dispatch one **ternary** FFN GEMM (`GGML_TYPE_TERNARY_158`). The resident 2-bit GPU
    /// kernel when the toggle is on AND the tensor is resident; otherwise the CPU base-3 oracle on
    /// the blob fetched from the mmap. Fail-closed (returns false, never garbage) on any mismatch.
    fn dispatch_ternary_ffn(
        &self,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            return false;
        }
        // GPU resident path (toggle on): keyed by the `.q42` blob offset (== info.byte_offset).
        #[cfg(not(target_arch = "wasm32"))]
        if crate::llm_bench::ternary_ffn_enabled() {
            if let Some(res) = self.ternary_ffn.as_ref() {
                if res.gemv(
                    self.gpu_device(),
                    self.gpu_queue(),
                    info.byte_offset,
                    input,
                    out,
                    n_in,
                    n_out,
                ) {
                    return true;
                }
            }
        }
        // CPU oracle fallback (toggle off, or GPU unavailable): the SAME ternary weights via the
        // base-3 CPU GEMM — correct, slower; this is the toggle's OFF baseline.
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, self.tensor_data_offset, info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        if raw.len() < 4 {
            return false;
        }
        let scale = f32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
        crate::ternary::ternary_gemm_cpu(
            &input[..n_in],
            &raw[4..],
            scale,
            n_in,
            n_out,
            1,
            0,
            0,
            &mut out[..n_out],
        );
        true
    }

    /// Quantized GEMM into caller `out` using reused GPU buffers (Q6_K) or CPU dequant fallback.
    pub fn dispatch_gemm_into(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            wlog(&format!("[gemm_into] GUARD n_in={n_in} n_out={n_out} input={} out={}", input.len(), out.len()));
            return false;
        }
        // A1b: ternary FFN tensors are not row-block quantized — route them to the dedicated ternary
        // dispatch (resident 2-bit GPU kernel / CPU oracle) before the standard fetch+GEMM path.
        if info.ggml_type == crate::ternary::GGML_TYPE_TERNARY_158 {
            return self.dispatch_ternary_ffn(info, input, out, n_in, n_out);
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info)
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        self.dispatch_gemm_raw_into(info, raw, input, out, n_in, n_out)
    }

    /// Chunked vocabulary projection with streaming argmax (zero heap, stack chunk buffer only).
    /// `max_chunks`: `0` sweeps the full vocabulary; otherwise caps chunk iterations (tests).
    pub fn dispatch_output_argmax_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        let info = index.logits_projection_info()?;
        let (n_in, vocab_size) = Self::matmul_dims(info);
        if n_in == 0 || vocab_size == 0 || n_in > emb_dim || n_in > hidden.len() {
            return None;
        }
        if chunk_logits.len() < VOCAB_CHUNK_ROWS {
            return None;
        }
        let mmap = self.gguf_mmap.as_deref()?;
        let full_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        let n_chunks = if max_chunks == 0 {
            full_chunks
        } else {
            (max_chunks as usize).min(full_chunks)
        };
        let mut best_token_id = 0u32;
        let mut max_logit = f32::NEG_INFINITY;

        for chunk_idx in 0..n_chunks {
            let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
            let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
            let raw = crate::ggml_quants::fetch_tensor_row_range_bytes(
                mmap,
                index.tensor_data_start,
                info,
                row_start,
                chunk_rows,
            )
            .ok()?;
            if !self.dispatch_gemm_raw_into(
                info,
                raw,
                &hidden[..n_in],
                &mut chunk_logits[..chunk_rows],
                n_in,
                chunk_rows,
            ) {
                return None;
            }
            #[cfg(target_arch = "wasm32")]
            if chunk_idx == 0 {
            }
            update_streaming_argmax_sieved(
                &chunk_logits[..chunk_rows],
                chunk_rows,
                chunk_idx,
                sieve_mask,
                &mut best_token_id,
                &mut max_logit,
            );
            scrub_f32_volatile(&mut chunk_logits[..chunk_rows], chunk_rows);
        }

        if max_logit == f32::NEG_INFINITY {
            return None;
        }
        Some(StreamingArgmaxResult {
            best_token_id,
            max_logit,
        })
    }

    /// A1a: create the persistent GPU top-k pipeline + small candidate/staging buffers (once).
    #[cfg(not(target_arch = "wasm32"))]
    fn init_output_topk(&mut self) {
        let shader = self
            .gpu_device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("output_topk"),
                source: wgpu::ShaderSource::Wgsl(crate::topk::TOPK_REDUCTION_WGSL.into()),
            });
        let pipeline = self
            .gpu_device()
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("output_topk_pipeline"),
                layout: None,
                module: &shader,
                entry_point: Some("topk_block"),
                compilation_options: Default::default(),
                cache: None,
            });
        // Worst case: a full vocab chunk's blocks × max K candidates.
        let max_blocks = (VOCAB_CHUNK_ROWS / crate::topk::TOPK_BLOCK_SIZE).max(1);
        let cand_bytes = ((max_blocks * crate::topk::TOPK_MAX_K).max(1) * 4) as wgpu::BufferAddress;
        self.topk_cand_val_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkCandVal"),
            size: cand_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.topk_cand_idx_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkCandIdx"),
            size: cand_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.topk_cand_staging = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkCandStaging"),
            size: cand_bytes * 2, // packed: [val .. | idx ..]
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.topk_params_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TopkParams"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.output_topk_pipeline = Some(pipeline);
    }

    /// A1a: GPU top-k over the output projection — the logits stay on-GPU (`gemm_output_buf`), the
    /// top-k kernel reduces them per chunk, and only K `(id, logit)` candidates are read back (vs the
    /// 196 KB/token full-logit readback + CPU argmax in `dispatch_output_argmax_chunked`). Returns the
    /// merged global top-K, or `None` to signal the caller to fall back to the argmax path. v1: no
    /// sieve coupling (caller routes here only when no sieve mask is active).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dispatch_output_topk_chunked(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        k: usize,
    ) -> Option<Vec<crate::topk::TopKItem>> {
        let info = index.logits_projection_info()?;
        let (n_in, vocab_size) = Self::matmul_dims(info);
        if n_in == 0 || vocab_size == 0 || n_in > emb_dim || n_in > hidden.len() {
            return None;
        }
        let pipeline = self.output_topk_pipeline.as_ref()?;
        let input_buf = self.gemm_input_buf.as_ref()?;
        let weight_buf = self.gemm_weight_buf.as_ref()?;
        let output_buf = self.gemm_output_buf.as_ref()?;
        let params_buf = self.gemm_params_buf.as_ref()?;
        let topk_params_buf = self.topk_params_buf.as_ref()?;
        let cand_val = self.topk_cand_val_buf.as_ref()?;
        let cand_idx = self.topk_cand_idx_buf.as_ref()?;
        let staging = self.topk_cand_staging.as_ref()?;
        let mmap = self.gguf_mmap.as_deref()?;

        let k = k.clamp(1, crate::topk::TOPK_MAX_K);
        let block_size = crate::topk::TOPK_BLOCK_SIZE;
        let full_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);

        let mut all_val: Vec<f32> = Vec::new();
        let mut all_idx: Vec<u32> = Vec::new();

        // A1a step-2: when the output projection is resident (uploaded once at init), bind the
        // per-chunk sub-range — zero per-token upload. Each chunk's byte offset is 256-aligned
        // because VOCAB_CHUNK_ROWS (8192) is a multiple of 256. The bound bytes, quant, shader and
        // params are identical to the per-chunk-upload fallback, so logits are byte-for-byte equal.
        let resident_logits = self.mc8_logits_resident_buf.as_ref();
        let resident_row_bytes = self.mc8_logits_row_bytes as u64;

        for chunk_idx in 0..full_chunks {
            let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
            let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);

            let (weight_resource, weight_byte_len) = if let Some(buf) = resident_logits {
                // Only the GPU-quant fast path is supported; otherwise signal fallback to argmax.
                if !(ggml_gpu_quant_supported(info.ggml_type)
                    && n_in <= MAX_STACK_GEMM_IN
                    && chunk_rows <= self.gemm_max_out_dim as usize)
                {
                    return None;
                }
                let byte_len = chunk_rows as u64 * resident_row_bytes;
                let res = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: buf,
                    offset: row_start as u64 * resident_row_bytes,
                    size: std::num::NonZeroU64::new(byte_len),
                });
                (res, byte_len as u32)
            } else {
                let raw = crate::ggml_quants::fetch_tensor_row_range_bytes(
                    mmap,
                    index.tensor_data_start,
                    info,
                    row_start,
                    chunk_rows,
                )
                .ok()?;
                if !(ggml_gpu_quant_supported(info.ggml_type)
                    && n_in <= MAX_STACK_GEMM_IN
                    && chunk_rows <= self.gemm_max_out_dim as usize
                    && raw.len() <= self.max_tensor_bytes)
                {
                    return None;
                }
                let byte_len = raw.len() as u32;
                self.write_weight_words(raw, self.max_tensor_bytes);
                (weight_buf.as_entire_binding(), byte_len)
            };

            let gparams = GemmGpuParams {
                n_in: n_in as u32,
                n_out: chunk_rows as u32,
                weight_ggml_type: info.ggml_type,
                weight_row_elems: info.dims[0] as u32,
                weight_byte_len,
                n_batch: 1,
                in_row_stride: 0,
                out_row_stride: 0,
            };
            self.gpu_queue()
                .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..n_in]));
            self.gpu_queue()
                .write_buffer(params_buf, 0, bytemuck::bytes_of(&gparams));
            let tparams = crate::topk::topk_params_bytes(chunk_rows as u32, k as u32, block_size as u32);
            self.gpu_queue().write_buffer(topk_params_buf, 0, &tparams);

            let num_blocks = chunk_rows.div_ceil(block_size);
            let cand_count = num_blocks * k;

            let gemm_bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("TopkGemmBind"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: weight_resource },
                    wgpu::BindGroupEntry { binding: 2, resource: params_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: output_buf.as_entire_binding() },
                ],
            });
            let topk_bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("TopkBind"),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: output_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: topk_params_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: cand_val.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: cand_idx.as_entire_binding() },
                ],
            });

            let mut encoder = self
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("TopkEncoder") });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("TopkGemmPass"),
                    timestamp_writes: crate::llm_gpu_profiler::pass_writes_begin(),
                });
                cpass.set_pipeline(&self.pipeline);
                cpass.set_bind_group(0, &gemm_bind, &[]);
                cpass.dispatch_workgroups((chunk_rows as u32 + 63) / 64, 1, 1);
            }
            {
                let mut tpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("TopkReducePass"),
                    timestamp_writes: crate::llm_gpu_profiler::pass_writes_end(),
                });
                tpass.set_pipeline(pipeline);
                tpass.set_bind_group(0, &topk_bind, &[]);
                tpass.dispatch_workgroups(num_blocks as u32, 1, 1);
            }
            let cand_bytes = (cand_count * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(cand_val, 0, staging, 0, cand_bytes);
            encoder.copy_buffer_to_buffer(cand_idx, 0, staging, cand_bytes, cand_bytes);
            crate::llm_gpu_profiler::resolve(&mut encoder);
            self.gpu_queue().submit(Some(encoder.finish()));
            crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::OutputTopk);

            let map_bytes = cand_bytes * 2;
            let slice = staging.slice(..map_bytes);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            self.poll_wait();
            let mapped_ok = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false)
            } else {
                false
            };
            if !mapped_ok {
                let _ = staging.unmap();
                return None;
            }
            {
                let data = slice.get_mapped_range();
                let vals: &[f32] = bytemuck::cast_slice(&data[..cand_count * 4]);
                let idxs: &[u32] = bytemuck::cast_slice(&data[cand_count * 4..cand_count * 8]);
                for i in 0..cand_count {
                    let v = vals[i];
                    if v > f32::NEG_INFINITY {
                        all_val.push(v);
                        all_idx.push(row_start as u32 + idxs[i]);
                    }
                }
            }
            staging.unmap();
        }

        let top = crate::topk::merge_block_candidates(&all_val, &all_idx, k, None);
        if top.is_empty() {
            None
        } else {
            Some(top)
        }
    }

    /// Final `output_norm` RMSNorm in-place before vocabulary projection (Pre-Norm LLM tail).
    /// REQUIRED on all targets — native previously skipped it → logits from an un-normed hidden.
    pub fn apply_output_norm_inplace(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
    ) -> bool {
        let info = match index.output_norm_info() {
            Some(i) => i,
            None => return true,
        };
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let n_embd = index.hyperparams.n_embd as usize;
        let n = emb_dim.min(n_embd).min(hidden.len());
        let mut norm_w = [0f32; MAX_HIDDEN_DIM];
        if dequant_norm_row_into(mmap, index.tensor_data_start, info, &mut norm_w) < n {
            return false;
        }
        rms_norm_inplace(&mut hidden[..n], &norm_w[..n], RMS_NORM_EPS);
        true
    }

    /// Pre-norm FFN: RMSNorm(hidden) → SwiGLU (wasm) or ReLU-gated (native) → residual add.
    fn dispatch_ffn_block_pre_norm(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        // SwiGLU FFN with ffn_norm pre-norm — REQUIRED on all targets (native previously ran a
        // norm-less ReLU chain on the raw residual → exponential blow-up to inf).
        {
            let mut norm_w_ffn = [0f32; MAX_HIDDEN_DIM];
            let mut h_norm_ffn = [0f32; MAX_HIDDEN_DIM];
            let ffn_input = prepare_pre_norm_input(
                &hidden[..emb_dim],
                emb_dim,
                tensors.ffn_norm.as_ref(),
                self.gguf_mmap.as_deref().map(|m| &m[..]),
                index.tensor_data_start,
                &mut h_norm_ffn,
                &mut norm_w_ffn,
            );
            let gate_info = match tensors.ffn_gate.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let up_info = match tensors.ffn_up.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let down_info = match tensors.ffn_down.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
            let (up_in, up_out) = Self::matmul_dims(up_info);
            let (dn_in, dn_out) = Self::matmul_dims(down_info);
            if gate_in > emb_dim
                || up_in != gate_in
                || up_out != n_ffn
                || dn_in != n_ffn
                || n_ffn > MAX_STACK_GEMM_DIM
                || dn_out > scratch_a.len()
            {
                return false;
            }
            // AWQ step 1 (calibration only; no-op in production): record per-input-channel salience
            // at the post-ffn_norm FFN input — the activation that scales the gate/up projections.
            crate::llm_awq::record_ffn_input(&ffn_input[..gate_in]);
            let mut gate_buf = [0f32; MAX_STACK_GEMM_DIM];
            let mut up_buf = [0f32; MAX_STACK_GEMM_DIM];
            if !self.dispatch_gemm_into(
                index,
                gate_info,
                &ffn_input[..gate_in],
                &mut gate_buf[..n_ffn],
                gate_in,
                n_ffn,
            ) {
                return false;
            }
            if !self.dispatch_gemm_into(
                index,
                up_info,
                &ffn_input[..up_in],
                &mut up_buf[..n_ffn],
                up_in,
                n_ffn,
            ) {
                return false;
            }
            silu_inplace(&mut gate_buf[..n_ffn], n_ffn);
            for i in 0..n_ffn {
                gate_buf[i] *= up_buf[i];
            }
            if !self.dispatch_gemm_into(
                index,
                down_info,
                &gate_buf[..dn_in],
                scratch_a,
                dn_in,
                dn_out,
            ) {
                return false;
            }
            add_residual_inplace(
                &mut hidden[..emb_dim],
                &scratch_a[..dn_out],
                emb_dim.min(dn_out),
            );
            return true;
        }
    }

    /// Phase 2B: SwiGLU FFN block with async GEMM readback (`map_async` + `await`).
    #[cfg(target_arch = "wasm32")]
    async fn dispatch_ffn_block_pre_norm_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let mut norm_w_ffn = [0f32; MAX_HIDDEN_DIM];
        let mut h_norm_ffn = [0f32; MAX_HIDDEN_DIM];
        let ffn_input = prepare_pre_norm_input(
            &hidden[..emb_dim],
            emb_dim,
            tensors.ffn_norm.as_ref(),
            self.gguf_mmap.as_deref(),
            index.tensor_data_start,
            &mut h_norm_ffn,
            &mut norm_w_ffn,
        );
        let gate_info = match tensors.ffn_gate.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let up_info = match tensors.ffn_up.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let down_info = match tensors.ffn_down.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > emb_dim
            || up_in != gate_in
            || up_out != n_ffn
            || dn_in != n_ffn
            || n_ffn > MAX_STACK_GEMM_DIM
            || dn_out > scratch_a.len()
        {
            return false;
        }
        let mut gate_buf = [0f32; MAX_STACK_GEMM_DIM];
        let mut up_buf = [0f32; MAX_STACK_GEMM_DIM];
        if !self
            .dispatch_gemm_into_async(
                index,
                gate_info,
                &ffn_input[..gate_in],
                &mut gate_buf[..n_ffn],
                gate_in,
                n_ffn,
            )
            .await
        {
            return false;
        }
        if !self
            .dispatch_gemm_into_async(
                index,
                up_info,
                &ffn_input[..up_in],
                &mut up_buf[..n_ffn],
                up_in,
                n_ffn,
            )
            .await
        {
            return false;
        }
        silu_inplace(&mut gate_buf[..n_ffn], n_ffn);
        for i in 0..n_ffn {
            gate_buf[i] *= up_buf[i];
        }
        if !self
            .dispatch_gemm_into_async(
                index,
                down_info,
                &gate_buf[..dn_in],
                scratch_a,
                dn_in,
                dn_out,
            )
            .await
        {
            return false;
        }
        add_residual_inplace(
            &mut hidden[..emb_dim],
            &scratch_a[..dn_out],
            emb_dim.min(dn_out),
        );
        true
    }

    fn matmul_dims(info: &GgufTensorInfo) -> (usize, usize) {
        let n_in = info.dims[0] as usize;
        let n_out = if info.n_dims > 1 && info.dims[1] > 0 {
            info.dims[1] as usize
        } else {
            1
        };
        (n_in, n_out)
    }

    fn attention_gpu_params(
        h: &crate::gguf_sharder::GgufHyperparams,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        info: &GgufTensorInfo,
        raw_len: usize,
        proj_kind: u32,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        mask_active: u32,
        mask_word_count: u32,
        out_stride_elems: u32,
    ) -> AttentionGpuParams {
        AttentionGpuParams {
            n_embd: h.n_embd,
            n_head: h.n_head,
            n_kv_head: h.effective_n_kv_head(),
            head_dim: h.head_dim(),
            q_heads_per_kv: h.q_heads_per_kv(),
            token_idx,
            max_context: layout.max_context,
            layer_idx: layer,
            layer_stride: layout.layer_stride,
            slot_kv_elems: layout.slot_kv_elems,
            weight_ggml_type: info.ggml_type,
            weight_row_elems: info.dims[0] as u32,
            weight_byte_len: raw_len as u32,
            proj_kind,
            rope_theta_base: h.effective_rope_freq_base(),
            rope_scale: h.effective_rope_scale(),
            num_tokens_in_batch,
            batch_start_token_idx,
            mask_active,
            mask_word_count,
            out_stride_elems,
            proj_row_stride: 0, // default = legacy in-shader projection; mc8_stage overrides for B
            _pad: [0; 2],
        }
    }

    #[inline]
    fn attention_kv_mask_for_dispatch(
        layout: &KvCacheLayout,
        token_idx: u32,
        proj_kind: u32,
    ) -> ([u32; KV_ATTENTION_MASK_WORDS], u32, u32) {
        if proj_kind != 0 {
            return ([0u32; KV_ATTENTION_MASK_WORDS], 0, 0);
        }
        let (words, active) =
            crate::compute_universe::attention_kv_mask_u32(token_idx, layout.max_context);
        (words, active, KV_ATTENTION_MASK_WORDS as u32)
    }

    /// Single fused-attention dispatch: K write, V write, or Q+online-softmax.
    fn dispatch_attention_pass(
        &self,
        hidden: &[f32],
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
        norm_weight: Option<&[f32]>,
        mut readback_out: Option<&mut [f32]>,
    ) -> bool {
        // #48 correctness path: route native attention through the CPU reference (the wasm-proven
        // SDPA) when enabled — bypasses the GPU attention shader whose output is currently unbounded.
        #[cfg(not(target_arch = "wasm32"))]
        if crate::llm_bench::cpu_attention_enabled() {
            return self.cpu_attention_pass(
                hidden,
                n_embd,
                num_tokens_in_batch,
                batch_start_token_idx,
                layout,
                layer,
                h,
                info,
                raw_weights,
                proj_kind,
                norm_weight,
                readback_out,
            );
        }
        if !ggml_gpu_attention_shader_supported(info.ggml_type) {
            wlog(&format!("[attn_pass] GUARD unsupported quant kind={proj_kind}"));
            return false;
        }
        let batch = num_tokens_in_batch.max(1) as usize;
        let hidden_elems = n_embd.checked_mul(batch).unwrap_or(0);
        if hidden_elems > hidden.len()
            || hidden_elems > self.gemm_max_input_floats
            || raw_weights.len() > self.max_tensor_bytes
            || self.gemm_input_buf.is_none()
            || self.kv_cache_gpu.is_none()
            || self.attention_params_buf.is_none()
            || self.attention_mask_buf.is_none()
        {
            wlog(&format!(
                "[attn_pass] GUARD buffers kind={proj_kind} hidden_elems={hidden_elems} hidden={} gemm_in={} raw_w={} max_w={} gemm_in_buf={} kv_gpu={} params={} mask={}",
                hidden.len(),
                self.gemm_max_input_floats,
                raw_weights.len(),
                self.max_tensor_bytes,
                self.gemm_input_buf.is_some(),
                self.kv_cache_gpu.is_some(),
                self.attention_params_buf.is_some(),
                self.attention_mask_buf.is_some(),
            ));
            if std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
                eprintln!(
                    "[attn_pass] GUARD kind={proj_kind} gemm_in_buf={} kv_gpu={} params={} mask={} hidden_elems={} gemm_max_in={} raw_w={} max_w={}",
                    self.gemm_input_buf.is_some(),
                    self.kv_cache_gpu.is_some(),
                    self.attention_params_buf.is_some(),
                    self.attention_mask_buf.is_some(),
                    hidden_elems,
                    self.gemm_max_input_floats,
                    raw_weights.len(),
                    self.max_tensor_bytes,
                );
            }
            return false;
        }

        // WASM: the browser cannot read GPU results synchronously, so run the CPU
        // attention kernel (Phase 2A) instead of the dead GPU dispatch + map_async path.
        #[cfg(target_arch = "wasm32")]
        return self.cpu_attention_pass(
            hidden,
            n_embd,
            num_tokens_in_batch,
            batch_start_token_idx,
            layout,
            layer,
            h,
            info,
            raw_weights,
            proj_kind,
            norm_weight,
            readback_out,
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(layout, token_idx, proj_kind);
        let params = Self::attention_gpu_params(
            h,
            layout,
            layer,
            token_idx,
            info,
            raw_weights.len(),
            proj_kind,
            num_tokens_in_batch.max(1),
            batch_start_token_idx,
            mask_active,
            mask_word_count,
            0,
        );
        let input_buf = self.gemm_input_buf.as_ref().unwrap();
        let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
        let output_buf = self.gemm_output_buf.as_ref().unwrap();
        let params_buf = self.attention_params_buf.as_ref().unwrap();
        let mask_buf = self.attention_mask_buf.as_ref().unwrap();
        let kv_buf = self.kv_cache_gpu.as_ref().unwrap();
        let staging = self.gemm_output_staging.as_ref().unwrap();

        // #49: honor norm_weight on the GPU path. Prefill passes RAW hidden + attn_norm here (the
        // CPU-reference convention); without per-token RMSNorm the prefill K/V are projected from the
        // un-normed residual and the KV cache explodes. Decode passes norm_weight=None → single upload.
        let norm_ok = n_embd <= MAX_HIDDEN_DIM && norm_weight.map_or(false, |w| w.len() >= n_embd);
        match norm_weight {
            Some(w) if norm_ok => {
                let mut norm_tok = [0f32; MAX_HIDDEN_DIM];
                for t in 0..batch {
                    let s = t * n_embd;
                    norm_tok[..n_embd].copy_from_slice(&hidden[s..s + n_embd]);
                    rms_norm_inplace(&mut norm_tok[..n_embd], &w[..n_embd], RMS_NORM_EPS);
                    self.gpu_queue().write_buffer(
                        input_buf,
                        (s * 4) as wgpu::BufferAddress,
                        bytemuck::cast_slice(&norm_tok[..n_embd]),
                    );
                }
            }
            _ => {
                self.gpu_queue()
                    .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..hidden_elems]));
            }
        }
        self.write_weight_words(raw_weights, self.max_tensor_bytes);
        self.gpu_queue()
            .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));
        self.gpu_queue()
            .write_buffer(mask_buf, 0, bytemuck::cast_slice(&mask_words));

        // Bind one layer slice of the KV arena (full arena exceeds 128 MiB wgpu binding cap).
        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset =
            (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding {
            buffer: kv_buf,
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };
        let (wg_x, wg_y) = if proj_kind == 0 && num_tokens_in_batch > 1 {
            (h.n_head, num_tokens_in_batch)
        } else {
            (n_workgroups.max(1), 1)
        };

        let bind_layout = self.attention_pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FusedAttentionBindGroup"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(kv_binding),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: mask_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FusedAttentionEncoder"),
            });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FusedAttentionPass"),
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.attention_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        let readback_elems = readback_out.as_ref().map(|o| o.len()).unwrap_or(0);
        if readback_elems > 0 {
            let out_bytes = (readback_elems * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
        }
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Attention);

        if readback_elems == 0 {
            return true;
        }

        let out_bytes = (readback_elems * 4) as wgpu::BufferAddress;
        let slice = staging.slice(..out_bytes);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.poll_wait();
        #[cfg(not(target_arch = "wasm32"))]
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
                let data = slice.get_mapped_range();
                let floats: &[f32] = bytemuck::cast_slice(&data);
                if let Some(out) = readback_out {
                    out[..readback_elems].copy_from_slice(&floats[..readback_elems]);
                }
                drop(data);
                staging.unmap();
                return true;
            }
        }
        let _ = staging.unmap();
        false
        }
    }

    /// WASM CPU attention fallback (Phase 2A). Projects one Q/K/V tensor for a batch of
    /// tokens, applies RoPE (Q/K), writes K/V into `kv_cache_cpu`, and runs SDPA for Q.
    /// `proj_kind`: 0=Q, 1=K, 2=V. Available on native too as the #48 correctness reference.
    fn cpu_attention_pass(
        &self,
        hidden: &[f32],
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        norm_weight: Option<&[f32]>,
        mut readback_out: Option<&mut [f32]>,
    ) -> bool {
        let head_dim = h.head_dim() as usize;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        if head_dim == 0 || n_head == 0 || n_kv == 0 {
            return false;
        }
        let (n_in, out_dim) = Self::matmul_dims(info);
        if out_dim == 0 || out_dim > MAX_STACK_GEMM_OUT || head_dim > out_dim {
            wlog(&format!("[cpu_attn] bad dims out_dim={out_dim} head_dim={head_dim}"));
            return false;
        }
        let proj_heads = out_dim / head_dim;
        let q_dim = n_head * head_dim;
        let q_heads_per_kv = h.q_heads_per_kv() as usize;
        let scale = 1.0f32 / (head_dim as f32).sqrt();
        let base_freq = h.effective_rope_freq_base();
        let rope_scale = h.effective_rope_scale();

        // SAFETY: single-threaded wasm; `kv_cache_cpu` is disjoint from `hidden`,
        // `raw_weights`, the local `proj` scratch, and `readback_out`.
        let (kv_ptr, kv_len) = match self.kv_cache_cpu.as_ref() {
            Some(b) => (b.as_ptr() as *mut f32, b.len()),
            None => return false,
        };

        let mut proj = [0f32; MAX_STACK_GEMM_OUT];
        let mut norm_tok = [0f32; MAX_HIDDEN_DIM];
        for t in 0..num_tokens_in_batch as usize {
            let pos = batch_start_token_idx + t as u32;
            let slot = layout.ring_slot(pos);
            let tok_start = t * n_embd;
            if tok_start + n_embd > hidden.len() {
                wlog(&format!("[cpu_attn] hidden OOB t={t} need={}", tok_start + n_embd));
                return false;
            }
            let htok = &hidden[tok_start..tok_start + n_embd];
            let gemm_in: &[f32] = if let Some(w) = norm_weight {
                if w.len() < n_embd {
                    return false;
                }
                norm_tok[..n_embd].copy_from_slice(htok);
                rms_norm_inplace(&mut norm_tok[..n_embd], &w[..n_embd], RMS_NORM_EPS);
                &norm_tok[..n_embd]
            } else {
                htok
            };
            if !stack_gemm_quant(raw_weights, info, gemm_in, &mut proj[..out_dim], n_in, out_dim) {
                wlog(&format!("[cpu_attn] proj failed kind={proj_kind} n_in={n_in} out_dim={out_dim} hidden={n_embd}"));
                return false;
            }

            match proj_kind {
                1 => {
                    rope_inplace(
                        &mut proj[..out_dim],
                        proj_heads,
                        head_dim,
                        pos,
                        base_freq,
                        rope_scale,
                    );
                    let kv = unsafe { core::slice::from_raw_parts_mut(kv_ptr, kv_len) };
                    for kvh in 0..n_kv {
                        for d in 0..head_dim {
                            let idx = layout.k_index(layer, slot, kvh as u32, d as u32);
                            if idx >= kv.len() {
                                wlog(&format!("[cpu_attn] K idx OOB idx={idx} len={}", kv.len()));
                                return false;
                            }
                            kv[idx] = proj[kvh * head_dim + d];
                        }
                    }
                }
                2 => {
                    let kv = unsafe { core::slice::from_raw_parts_mut(kv_ptr, kv_len) };
                    for kvh in 0..n_kv {
                        for d in 0..head_dim {
                            let idx = layout.v_index(layer, slot, kvh as u32, d as u32);
                            if idx >= kv.len() {
                                wlog(&format!("[cpu_attn] V idx OOB idx={idx} len={}", kv.len()));
                                return false;
                            }
                            kv[idx] = proj[kvh * head_dim + d];
                        }
                    }
                }
                0 => {
                    let mut att_scores = [0f32; MAX_CONTEXT_WINDOW as usize];
                    rope_inplace(
                        &mut proj[..out_dim],
                        proj_heads,
                        head_dim,
                        pos,
                        base_freq,
                        rope_scale,
                    );
                    let out_buf = match readback_out.as_mut() {
                        Some(out) => {
                            let out_off = t * q_dim;
                            if out_off + q_dim > out.len() {
                                wlog(&format!(
                                    "[cpu_attn] Q out OOB off={out_off} q_dim={q_dim} len={}",
                                    out.len()
                                ));
                                return false;
                            }
                            &mut out[out_off..out_off + q_dim]
                        }
                        None => return false,
                    };
                    out_buf.fill(0.0);
                    let pos_usize = pos as usize;
                    if pos_usize >= MAX_CONTEXT_WINDOW as usize {
                        wlog(&format!("[cpu_attn] pos OOB pos={pos}"));
                        return false;
                    }
                    let kv = unsafe { core::slice::from_raw_parts(kv_ptr, kv_len) };
                    for q_h in 0..n_head {
                        let kv_h = q_h / q_heads_per_kv;
                        let q_head_slice = &proj[q_h * head_dim..(q_h + 1) * head_dim];
                        let out_head_slice = &mut out_buf[q_h * head_dim..(q_h + 1) * head_dim];
                        let mut max_score = f32::NEG_INFINITY;
                        for past_pos in 0..=pos {
                            let past_slot = layout.ring_slot(past_pos);
                            let mut dot = 0.0f32;
                            for d in 0..head_dim {
                                let k_idx =
                                    layout.k_index(layer, past_slot, kv_h as u32, d as u32);
                                if k_idx >= kv.len() {
                                    wlog(&format!(
                                        "[cpu_attn] SDPA K idx OOB idx={k_idx} len={}",
                                        kv.len()
                                    ));
                                    return false;
                                }
                                dot += q_head_slice[d] * kv[k_idx];
                            }
                            let score = dot * scale;
                            att_scores[past_pos as usize] = score;
                            if score > max_score {
                                max_score = score;
                            }
                        }
                        let mut sum_exp = 0.0f32;
                        for past_pos in 0..=pos {
                            let exp_val = (att_scores[past_pos as usize] - max_score).exp();
                            att_scores[past_pos as usize] = exp_val;
                            sum_exp += exp_val;
                        }
                        if sum_exp == 0.0 {
                            wlog(&format!(
                                "[MC3] softmax sum_exp=0 layer={layer} pos={pos} q_h={q_h} max_score={max_score}"
                            ));
                            return false;
                        }
                        for past_pos in 0..=pos {
                            let prob = att_scores[past_pos as usize] / sum_exp;
                            let past_slot = layout.ring_slot(past_pos);
                            for d in 0..head_dim {
                                let v_idx =
                                    layout.v_index(layer, past_slot, kv_h as u32, d as u32);
                                if v_idx >= kv.len() {
                                    wlog(&format!(
                                        "[cpu_attn] SDPA V idx OOB idx={v_idx} len={}",
                                        kv.len()
                                    ));
                                    return false;
                                }
                                out_head_slice[d] += kv[v_idx] * prob;
                            }
                        }
                    }
                }
                _ => return false,
            }
        }
        true
    }

    /// GPU-fused Q/K/V projections, RoPE, ring-buffer KV write, and GQA online-softmax.
    fn dispatch_attention_layer(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &[f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> Option<usize> {
        let layout = self.kv_layout?;
        let q_info = tensors.attn_q.as_ref()?;
        let k_info = tensors.attn_k.as_ref()?;
        let v_info = tensors.attn_v.as_ref()?;
        let h = index.hyperparams;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        let head_dim = h.head_dim() as usize;
        {
            use std::sync::atomic::{AtomicBool, Ordering as AO};
            static A_DBG: AtomicBool = AtomicBool::new(false);
            if layer == 0
                && std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok()
                && !A_DBG.swap(true, AO::Relaxed)
            {
                eprintln!(
                    "[attn-dbg] n_head={} n_kv={} head_dim={} q_dim={} qty={} kty={} vty={} ashader_q={}",
                    n_head, n_kv, head_dim, n_head * head_dim,
                    q_info.ggml_type, k_info.ggml_type, v_info.ggml_type,
                    ggml_gpu_attention_shader_supported(q_info.ggml_type),
                );
            }
        }
        if head_dim == 0 || n_head == 0 || n_kv == 0 {
            return None;
        }
        let q_dim = n_head * head_dim;
        if q_dim > scratch_a.len() || q_dim > scratch_b.len() || emb_dim < h.n_embd as usize {
            return None;
        }
        // Use the attention-shader support set (fused_attention.wgsl handles Q4_0/Q5_0/Q8_0/Q4_K/Q6_K)
        // — NOT the narrower GEMM `ggml_gpu_quant_supported`, which wrongly rejected Q8_0 → no attention.
        if !ggml_gpu_attention_shader_supported(q_info.ggml_type)
            || !ggml_gpu_attention_shader_supported(k_info.ggml_type)
            || !ggml_gpu_attention_shader_supported(v_info.ggml_type)
        {
            return None;
        }

        let mmap = self.gguf_mmap.as_deref()?;
        let k_raw =
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, &k_info).ok()?;
        let v_raw =
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, &v_info).ok()?;
        let q_raw =
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, &q_info).ok()?;
        let n_embd = h.n_embd as usize;

        // Pre-norm (attn_norm) on the residual stream before Q/K/V — REQUIRED on all targets.
        // (Native previously skipped this → the residual stream exploded layer-over-layer to inf.)
        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let mut h_norm_attn = [0f32; MAX_HIDDEN_DIM];
        let hidden_input = prepare_pre_norm_input(
            &hidden[..emb_dim],
            emb_dim,
            tensors.attn_norm.as_ref(),
            Some(&mmap[..]),
            index.tensor_data_start,
            &mut h_norm_attn,
            &mut norm_w_attn,
        );

        if !self.dispatch_attention_pass(
            hidden_input,
            n_embd,
            1,
            token_idx,
            &layout,
            layer,
            token_idx,
            &h,
            k_info,
            k_raw,
            1,
            n_kv as u32,
            None,
            None,
        ) {
            return None;
        }
        if !self.dispatch_attention_pass(
            hidden_input,
            n_embd,
            1,
            token_idx,
            &layout,
            layer,
            token_idx,
            &h,
            v_info,
            v_raw,
            2,
            n_kv as u32,
            None,
            None,
        ) {
            return None;
        }
        if !self.dispatch_attention_pass(
            hidden_input,
            n_embd,
            1,
            token_idx,
            &layout,
            layer,
            token_idx,
            &h,
            q_info,
            q_raw,
            0,
            n_head as u32,
            None,
            Some(&mut scratch_b[..q_dim]),
        ) {
            return None;
        }

        if let Some(out_info) = tensors.attn_output {
            let (o_in, o_out) = Self::matmul_dims(&out_info);
            if o_in <= q_dim
                && self.dispatch_gemm_into(
                    index,
                    &out_info,
                    &scratch_b[..o_in],
                    &mut scratch_a[..o_out],
                    o_in,
                    o_out,
                )
            {
                return Some(o_out.min(emb_dim));
            }
        }
        let n = q_dim.min(emb_dim);
        scratch_a[..n].copy_from_slice(&scratch_b[..n]);
        Some(n)
    }

    /// Q+attn, output projection, and FFN for one token (K/V already in arena).
    fn dispatch_attention_q_ffn_token(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let q_info = match tensors.attn_q.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let h = index.hyperparams;
        let n_head = h.n_head as usize;
        let head_dim = h.head_dim() as usize;
        let q_dim = n_head * head_dim;
        if q_dim > scratch_a.len() || q_dim > scratch_b.len() || emb_dim < h.n_embd as usize {
            return false;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let q_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let n_embd = h.n_embd as usize;
        // Pre-norm (attn_norm) on the residual stream before Q/K/V — REQUIRED on all targets.
        // (Native previously skipped this → the residual stream exploded layer-over-layer to inf.)
        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let mut h_norm_attn = [0f32; MAX_HIDDEN_DIM];
        let hidden_input = prepare_pre_norm_input(
            &hidden[..emb_dim],
            emb_dim,
            tensors.attn_norm.as_ref(),
            Some(&mmap[..]),
            index.tensor_data_start,
            &mut h_norm_attn,
            &mut norm_w_attn,
        );
        if !self.dispatch_attention_pass(
            hidden_input,
            n_embd,
            1,
            token_idx,
            &layout,
            layer,
            token_idx,
            &h,
            q_info,
            q_raw,
            0,
            n_head as u32,
            None,
            Some(&mut scratch_b[..q_dim]),
        ) {
            return false;
        }
        let mut attn_ok = false;
        if let Some(out_info) = tensors.attn_output.as_ref() {
            let (o_in, o_out) = Self::matmul_dims(out_info);
            if o_in <= q_dim
                && self.dispatch_gemm_into(
                    index,
                    out_info,
                    &scratch_b[..o_in],
                    &mut scratch_a[..o_out],
                    o_in,
                    o_out,
                )
            {
                add_residual_inplace(
                    &mut hidden[..emb_dim],
                    &scratch_a[..o_out],
                    emb_dim.min(o_out),
                );
                attn_ok = true;
            }
        } else {
            let n = q_dim.min(emb_dim);
            add_residual_inplace(&mut hidden[..emb_dim], &scratch_b[..n], n);
            attn_ok = true;
        }
        if !attn_ok {
            return false;
        }
        self.dispatch_ffn_block_pre_norm(index, hidden, emb_dim, &tensors, scratch_a, scratch_b)
    }

    /// One transformer layer for a batched prefill chunk: batched K/V then per-token Q+FFN.
    fn dispatch_prefill_layer_batch(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        if n_tokens == 0 {
            wlog("[prefill_layer] FAILED n_tokens=0");
            return false;
        }
        let layout = match self.kv_layout {
            Some(l) => l,
            None => {
                wlog("[prefill_layer] FAILED kv_layout is None");
                return false;
            }
        };
        let tensors = index.get_layer_tensors(layer);
        let k_info = match tensors.attn_k.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_k layer={layer}"));
                return false;
            }
        };
        let v_info = match tensors.attn_v.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_v layer={layer}"));
                return false;
            }
        };
        if tensors.attn_q.is_none() {
            wlog(&format!("[prefill_layer] FAILED missing attn_q layer={layer}"));
            return false;
        }
        let h = index.hyperparams;
        let n_kv = h.effective_n_kv_head();
        let n_embd = h.n_embd as usize;
        let batch_elems = n_embd * n_tokens as usize;
        if batch_elems > batch_hidden.len() {
            wlog(&format!(
                "[prefill_layer] FAILED batch_elems OOB elems={batch_elems} hidden={}",
                batch_hidden.len()
            ));
            return false;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => {
                wlog("[prefill_layer] FAILED gguf_mmap is None");
                return false;
            }
        };
        let k_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_k bytes: {e:?}"));
                    return false;
                }
            };
        let v_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_v bytes: {e:?}"));
                    return false;
                }
            };
        let n_kv_wg = n_tokens.saturating_mul(n_kv);
        // attn_norm MUST be applied to the K/V projection input on ALL targets — native previously
        // passed None here → prefill wrote K/V from the RAW residual → the KV cache exploded across
        // layers → the whole forward (and decode reading it) blew up. (#48)
        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let norm_weight_attn: Option<&[f32]> = tensors.attn_norm.as_ref().and_then(|info| {
            let n = dequant_norm_row_into(mmap, index.tensor_data_start, info, &mut norm_w_attn);
            if n >= n_embd {
                Some(&norm_w_attn[..n_embd])
            } else {
                None
            }
        });
        if !self.dispatch_attention_pass(
            &batch_hidden[..batch_elems],
            n_embd,
            n_tokens,
            batch_start_token_idx,
            &layout,
            layer,
            batch_start_token_idx,
            &h,
            k_info,
            k_raw,
            1,
            n_kv_wg,
            norm_weight_attn,
            None,
        ) {
            wlog(&format!("[prefill_layer] K pass FAILED layer={layer}"));
            return false;
        }
        if !self.dispatch_attention_pass(
            &batch_hidden[..batch_elems],
            n_embd,
            n_tokens,
            batch_start_token_idx,
            &layout,
            layer,
            batch_start_token_idx,
            &h,
            v_info,
            v_raw,
            2,
            n_kv_wg,
            norm_weight_attn,
            None,
        ) {
            wlog(&format!("[prefill_layer] V pass FAILED layer={layer}"));
            return false;
        }
        for t in 0..n_tokens {
            let abs = batch_start_token_idx + t;
            let off = t as usize * emb_dim;
            if !self.dispatch_attention_q_ffn_token(
                index,
                layer,
                abs,
                &mut batch_hidden[off..off + emb_dim],
                emb_dim,
                &tensors,
                scratch_a,
                scratch_b,
            ) {
                wlog(&format!("[prefill_layer] q_ffn FAILED layer={layer} t={t} abs={abs}"));
                return false;
            }
        }
        true
    }

    /// Phase 2B: batched prefill layer via async GPU attention (K/V GPU; Q+FFN per token).
    #[cfg(target_arch = "wasm32")]
    async fn dispatch_prefill_layer_batch_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        if n_tokens == 0 {
            wlog("[prefill_layer] FAILED n_tokens=0");
            return false;
        }
        let layout = match self.kv_layout {
            Some(l) => l,
            None => {
                wlog("[prefill_layer] FAILED kv_layout is None");
                return false;
            }
        };
        let tensors = index.get_layer_tensors(layer);
        let k_info = match tensors.attn_k.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_k layer={layer}"));
                return false;
            }
        };
        let v_info = match tensors.attn_v.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_v layer={layer}"));
                return false;
            }
        };
        if tensors.attn_q.is_none() {
            wlog(&format!("[prefill_layer] FAILED missing attn_q layer={layer}"));
            return false;
        }
        let h = index.hyperparams;
        let n_kv = h.effective_n_kv_head();
        let n_embd = h.n_embd as usize;
        let batch_elems = n_embd * n_tokens as usize;
        if batch_elems > batch_hidden.len() {
            wlog(&format!(
                "[prefill_layer] FAILED batch_elems OOB elems={batch_elems} hidden={}",
                batch_hidden.len()
            ));
            return false;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => {
                wlog("[prefill_layer] FAILED gguf_mmap is None");
                return false;
            }
        };
        let k_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_k bytes: {e:?}"));
                    return false;
                }
            };
        let v_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_v bytes: {e:?}"));
                    return false;
                }
            };
        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let mut norm_scratch = [0f32; PREFILL_CHUNK_STACK_FLOATS];
        let attn_input: &mut [f32] = if let Some(norm_info) = tensors.attn_norm.as_ref() {
            let n = dequant_norm_row_into(
                mmap,
                index.tensor_data_start,
                norm_info,
                &mut norm_w_attn,
            );
            if n >= n_embd {
                for t in 0..n_tokens as usize {
                    let off = t * n_embd;
                    norm_scratch[off..off + n_embd].copy_from_slice(&batch_hidden[off..off + n_embd]);
                    rms_norm_inplace(
                        &mut norm_scratch[off..off + n_embd],
                        &norm_w_attn[..n_embd],
                        RMS_NORM_EPS,
                    );
                }
                &mut norm_scratch[..batch_elems]
            } else {
                batch_hidden
            }
        } else {
            batch_hidden
        };
        let n_kv_wg = n_tokens.saturating_mul(n_kv);
        if !self
            .dispatch_attention_pass_async(
                attn_input,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                k_info,
                k_raw,
                1,
                n_kv_wg,
                None,
            )
            .await
        {
            wlog(&format!("[prefill_layer] K pass FAILED layer={layer}"));
            return false;
        }
        if !self
            .dispatch_attention_pass_async(
                attn_input,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                v_info,
                v_raw,
                2,
                n_kv_wg,
                None,
            )
            .await
        {
            wlog(&format!("[prefill_layer] V pass FAILED layer={layer}"));
            return false;
        }
        for t in 0..n_tokens {
            let abs = batch_start_token_idx + t;
            let off = t as usize * emb_dim;
            if !self
                .dispatch_attention_q_ffn_token_async(
                    index,
                    layer,
                    abs,
                    &mut batch_hidden[off..off + emb_dim],
                    emb_dim,
                    &tensors,
                    scratch_a,
                    scratch_b,
                )
                .await
            {
                wlog(&format!("[prefill_layer] q_ffn FAILED layer={layer} t={t} abs={abs}"));
                return false;
            }
        }
        true
    }

    /// Chunked prefill: populate KV arena for `n_tokens` prompt positions starting at `batch_start`.
    pub fn dispatch_prefill_chunk(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        max_layers: u32,
    ) -> bool {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || n_tokens == 0 {
            return false;
        }
        let limit = if max_layers == 0 {
            n_layer
        } else {
            max_layers.min(n_layer)
        };
        for layer in 0..limit {
            if !self.dispatch_prefill_layer_batch(
                index,
                layer,
                batch_hidden,
                emb_dim,
                n_tokens,
                batch_start_token_idx,
                scratch_a,
                scratch_b,
            ) {
                return false;
            }
        }
        true
    }

    /// One transformer block using real mmap tensor offsets (stack buffers only).
    pub fn dispatch_transformer_layer(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let tensors = index.get_layer_tensors(layer);
        let mut attn_ok = false;
        // Decode-profiler: intra-layer split — attention (this block) vs FFN (below).
        let t_attn = std::time::Instant::now();

        if tensors.attn_q.is_some() && tensors.attn_k.is_some() && tensors.attn_v.is_some() {
            if let Some(n) = self.dispatch_attention_layer(
                index,
                layer,
                token_idx,
                &hidden[..emb_dim],
                emb_dim,
                &tensors,
                scratch_a,
                scratch_b,
            ) {
                add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..n], n);
                attn_ok = true;
            }
        } else if let Some(info) = tensors.attn_output {
            let (n_in, n_out) = Self::matmul_dims(&info);
            if n_in <= emb_dim
                && self.dispatch_gemm_into(index, &info, &hidden[..n_in], scratch_a, n_in, n_out)
            {
                add_residual_inplace(
                    &mut hidden[..emb_dim],
                    &scratch_a[..n_out],
                    emb_dim.min(n_out),
                );
                attn_ok = true;
            }
        }

        crate::llm_bench::add_decode_attn_ns(t_attn.elapsed().as_nanos() as u64);

        if !attn_ok && tensors.attn_output.is_none() && tensors.ffn_gate.is_none() {
            return false;
        }

        // #48 diagnostic: localize the residual explosion — attention output magnitude per layer.
        if layer < 3 && std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
            let max_attn = scratch_a[..emb_dim].iter().fold(0f32, |m, &v| m.max(v.abs()));
            let max_hid = hidden[..emb_dim].iter().fold(0f32, |m, &v| m.max(v.abs()));
            eprintln!(
                "[layer-dbg] L{} attn_ok={} attn_norm={} ffn_norm={} max|attn_out|={:.4} max|hidden_postattn|={:.4}",
                layer,
                attn_ok,
                tensors.attn_norm.is_some(),
                tensors.ffn_norm.is_some(),
                max_attn,
                max_hid,
            );
        }

        let t_ffn = std::time::Instant::now();
        let ffn_ok = self.dispatch_ffn_block_pre_norm(
            index,
            hidden,
            emb_dim,
            &tensors,
            scratch_a,
            scratch_b,
        );
        crate::llm_bench::add_decode_ffn_ns(t_ffn.elapsed().as_nanos() as u64);
        ffn_ok
    }

    /// Sequential layer-by-layer forward (one tensor payload in VRAM at a time).
    /// `max_layers`: `0` runs all blocks; otherwise caps how many layers execute.
    pub fn dispatch_transformer_forward(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        token_idx: u32,
        max_layers: u32,
    ) -> u32 {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 {
            return 0;
        }
        let limit = if max_layers == 0 {
            n_layer
        } else {
            max_layers.min(n_layer)
        };
        // #48 diagnostic: localize where the hidden state turns non-finite (gated; runs once).
        use std::sync::atomic::{AtomicBool, Ordering as DbgOrdering};
        static FWD_DBG_DONE: AtomicBool = AtomicBool::new(false);
        let dbg = std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok()
            && !FWD_DBG_DONE.swap(true, DbgOrdering::Relaxed);
        if dbg {
            let nf = hidden[..emb_dim].iter().filter(|v| !v.is_finite()).count();
            eprintln!(
                "[fwd-dbg] post-embed nonfinite={}/{} sample={:?}",
                nf,
                emb_dim,
                &hidden[..emb_dim.min(4)]
            );
        }
        let mut ran = 0u32;
        for layer in 0..limit {
            if self.dispatch_transformer_layer(
                index, layer, token_idx, hidden, emb_dim, scratch_a, scratch_b,
            ) {
                ran += 1;
            }
            if dbg {
                let nf = hidden[..emb_dim].iter().filter(|v| !v.is_finite()).count();
                eprintln!(
                    "[fwd-dbg] after layer {} nonfinite={}/{} ran={} sample={:?}",
                    layer,
                    nf,
                    emb_dim,
                    ran,
                    &hidden[..emb_dim.min(4)]
                );
                if nf > 0 {
                    break;
                }
            }
        }
        ran
    }

    /// Phase 2B: async single-layer forward (GPU `map_async`; CPU path unchanged in sync API).
    #[cfg(target_arch = "wasm32")]
    pub async fn dispatch_transformer_layer_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let tensors = index.get_layer_tensors(layer);
        let mut attn_ok = false;

        if tensors.attn_q.is_some() && tensors.attn_k.is_some() && tensors.attn_v.is_some() {
            if let Some(n) = self
                .dispatch_attention_layer_async(
                    index,
                    layer,
                    token_idx,
                    &hidden[..emb_dim],
                    emb_dim,
                    &tensors,
                    scratch_a,
                    scratch_b,
                )
                .await
            {
                add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..n], n);
                attn_ok = true;
            }
        } else if let Some(info) = tensors.attn_output {
            let (n_in, n_out) = Self::matmul_dims(&info);
            if n_in <= emb_dim
                && self
                    .dispatch_gemm_into_async(
                        index,
                        &info,
                        &hidden[..n_in],
                        scratch_a,
                        n_in,
                        n_out,
                    )
                    .await
            {
                add_residual_inplace(
                    &mut hidden[..emb_dim],
                    &scratch_a[..n_out],
                    emb_dim.min(n_out),
                );
                attn_ok = true;
            }
        }

        if !attn_ok && tensors.attn_output.is_none() && tensors.ffn_gate.is_none() {
            return false;
        }

        self.dispatch_ffn_block_pre_norm_async(
            index,
            hidden,
            emb_dim,
            &tensors,
            scratch_a,
            scratch_b,
        )
        .await
    }

    /// MC8: Q + o_proj + FFN tail (K/V already written for this token).
    #[cfg(target_arch = "wasm32")]
    fn encode_attn_ffn_tail_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        token_hidden: &wgpu::Buffer,
        attn_input: Option<&wgpu::Buffer>,
        work_aliases_hidden: bool,
    ) -> bool {
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let work_buf = self.gemm_output_buf.as_ref().unwrap();
        let aux_buf = self.gemm_aux_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        let q_info = match tensors.attn_q.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let q_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let q_in_buf = if let Some(pre) = attn_input {
            pre
        } else if let Some(norm) = tensors.attn_norm.as_ref() {
            if !self.upload_norm_weights(mmap, index.tensor_data_start, norm, n_embd) {
                return false;
            }
            self.encode_elem(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                1,
                token_hidden,
                norm_buf,
                aux_buf,
            );
            aux_buf
        } else {
            token_hidden
        };
        let ffn_buf = self.gemm_ffn_buf.as_ref().unwrap();
        let emb_bytes = (emb_dim * 4) as wgpu::BufferAddress;
        let q_dim = (h.n_head * h.head_dim()) as usize;
        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(&layout, token_idx, 0);
        let q_params = Self::attention_gpu_params(
            &h,
            &layout,
            layer,
            token_idx,
            q_info,
            q_raw.len(),
            0,
            1,
            token_idx,
            mask_active,
            mask_word_count,
            0,
        );
        let q_off = self.mc8_upload_attn_param(&q_params);
        if mask_active != 0 {
            self.gpu_queue().write_buffer(
                self.attention_mask_buf.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&mask_words),
            );
        }
        if !self.encode_attention_pass_gpu(
            pipeline,
            q_in_buf,
            ffn_buf,
            n_embd,
            1,
            token_idx,
            &layout,
            layer,
            token_idx,
            &h,
            q_info,
            q_raw,
            0,
            h.n_head,
            q_off,
            Mc8WeightRole::AttnQ,
        ) {
            return false;
        }
        self.mc8_flush(pipeline);
        if let Some(out_info) = tensors.attn_output.as_ref() {
            let (o_in, o_out) = Self::matmul_dims(out_info);
            let o_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, out_info)
            {
                Ok(s) => s,
                Err(_) => return false,
            };
            if work_aliases_hidden {
                pipeline.encoder.copy_buffer_to_buffer(token_hidden, 0, aux_buf, 0, emb_bytes);
                self.mc8_flush(pipeline);
            }
            if o_in > q_dim
                || !self.encode_gemm_bufs(
                    pipeline,
                    out_info,
                    o_raw,
                    o_in,
                    o_out,
                    ffn_buf,
                    work_buf,
                )
            {
                return false;
            }
            self.mc8_flush(pipeline);
            let attn_residual_base: &wgpu::Buffer = if work_aliases_hidden {
                aux_buf
            } else {
                token_hidden
            };
            // Never use `prefill_scratch_buf` here — it holds batched attn RMSNorm rows.
            self.encode_residual_add_gpu(
                pipeline,
                attn_residual_base,
                work_buf,
                token_hidden,
                ffn_buf,
                emb_dim as u32,
            );
        } else {
            self.encode_residual_add_gpu(
                pipeline,
                token_hidden,
                ffn_buf,
                token_hidden,
                aux_buf,
                emb_dim as u32,
            );
        }
        self.mc8_flush(pipeline);
        let gate_info = match tensors.ffn_gate.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let up_info = match tensors.ffn_up.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let down_info = match tensors.ffn_down.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > n_embd || up_in != gate_in || up_out != n_ffn || dn_in != n_ffn || dn_out < n_embd {
            return false;
        }
        let gate_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info)
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        let up_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let down_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let base_save = match self.prefill_scratch_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        pipeline.encoder.copy_buffer_to_buffer(token_hidden, 0, base_save, 0, emb_bytes);
        self.mc8_flush(pipeline);
        if let Some(norm) = tensors.ffn_norm.as_ref() {
            if !self.upload_norm_weights(mmap, index.tensor_data_start, norm, n_embd) {
                return false;
            }
            self.encode_elem(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                1,
                token_hidden,
                norm_buf,
                aux_buf,
            );
        } else {
            pipeline.encoder.copy_buffer_to_buffer(token_hidden, 0, aux_buf, 0, emb_bytes);
        }
        self.mc8_flush(pipeline);
        if !self.encode_gemm_bufs(pipeline, gate_info, gate_raw, gate_in, n_ffn, aux_buf, work_buf) {
            return false;
        }
        self.mc8_flush(pipeline);
        if !self.encode_gemm_bufs(pipeline, up_info, up_raw, up_in, n_ffn, aux_buf, ffn_buf) {
            return false;
        }
        self.mc8_flush(pipeline);
        self.encode_elem(
            pipeline,
            ELEM_OP_SILU_MUL,
            n_ffn as u32,
            1,
            work_buf,
            ffn_buf,
            aux_buf,
        );
        self.mc8_flush(pipeline);
        if !self.encode_gemm_bufs(pipeline, down_info, down_raw, dn_in, dn_out, aux_buf, work_buf) {
            return false;
        }
        self.mc8_flush(pipeline);
        // FFN residual: down output is in work_buf; pre-FFN skip is in base_save.
        // Use aux_buf as scratch (SiLU output consumed; down GEMM flushed above).
        self.encode_residual_add_gpu(
            pipeline,
            base_save,
            work_buf,
            token_hidden,
            aux_buf,
            emb_dim as u32,
        );
        self.mc8_flush(pipeline);
        true
    }

    /// MC8: encode one decode layer entirely on GPU (no map_async).
    /// Superseded by the Part 3w super-arena decode forward (kept for reference/fallback).
    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)]
    fn encode_transformer_layer_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        emb_dim: usize,
    ) -> bool {
        let tensors = index.get_layer_tensors(layer);
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        if emb_dim < n_embd {
            return false;
        }
        let hidden_buf = self.gemm_input_buf.as_ref().unwrap();
        let work_buf = self.gemm_output_buf.as_ref().unwrap();
        let aux_buf = self.gemm_aux_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();

        let (k_info, v_info) = match (tensors.attn_k.as_ref(), tensors.attn_v.as_ref()) {
            (Some(k), Some(v)) => (k, v),
            _ => return false,
        };
        if tensors.attn_q.is_none() {
            return false;
        }
        let k_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let v_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let attn_input = if let Some(norm) = tensors.attn_norm.as_ref() {
            if !self.upload_norm_weights(mmap, index.tensor_data_start, norm, n_embd) {
                return false;
            }
            self.encode_elem(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                1,
                hidden_buf,
                norm_buf,
                aux_buf,
            );
            self.mc8_flush(pipeline);
            aux_buf
        } else {
            hidden_buf
        };

        let n_kv = h.effective_n_kv_head();
        let mut attn_arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let k_params = Self::attention_gpu_params(
            &h,
            &layout,
            layer,
            token_idx,
            k_info,
            k_raw.len(),
            1,
            1,
            token_idx,
            0,
            0,
            0,
        );
        let v_params = Self::attention_gpu_params(
            &h,
            &layout,
            layer,
            token_idx,
            v_info,
            v_raw.len(),
            2,
            1,
            token_idx,
            0,
            0,
            0,
        );
        let k_off = attn_arena.push(&k_params);
        let v_off = attn_arena.push(&v_params);
        attn_arena.upload(
            self.gpu_queue(),
            self.attention_params_buf.as_ref().unwrap(),
        );
        if !self.encode_attention_pass_gpu(
            pipeline,
            attn_input,
            work_buf,
            n_embd,
            1,
            token_idx,
            &layout,
            layer,
            token_idx,
            &h,
            k_info,
            k_raw,
            1,
            n_kv,
            k_off,
            Mc8WeightRole::AttnK,
        ) {
            return false;
        }
        if !self.encode_attention_pass_gpu(
            pipeline,
            attn_input,
            work_buf,
            n_embd,
            1,
            token_idx,
            &layout,
            layer,
            token_idx,
            &h,
            v_info,
            v_raw,
            2,
            n_kv,
            v_off,
            Mc8WeightRole::AttnV,
        ) {
            return false;
        }
        self.mc8_flush(pipeline);
        self.encode_attn_ffn_tail_gpu(
            pipeline,
            index,
            layer,
            token_idx,
            emb_dim,
            &tensors,
            hidden_buf,
            Some(attn_input),
            false,
        )
    }

    /// MC8 Part 3w: decode forward via the prefill super-arena (n_tokens=1).
    /// Reuses `mc8_stage_prefill_layer_super_arena` + `encode_prefill_q_ffn_tail_fused`
    /// (dynamic-offset uniforms + 7 disjoint weight buffers) → **2 submits/layer**
    /// (KV-visibility flush + layer-end), down from the legacy 13 flushes/layer.
    /// A single decode token at absolute position `token_idx` is a 1-row prefill chunk:
    /// dense causal Q (`mask_active=0`, `logical <= abs_pos`) is correct for decode.
    #[cfg(target_arch = "wasm32")]
    pub async fn dispatch_transformer_forward_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        _scratch_a: &mut [f32],
        _scratch_b: &mut [f32],
        token_idx: u32,
        max_layers: u32,
    ) -> u32 {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || !self.mc8_buffers_ready() {
            return 0;
        }
        if self.prefill_work_buf_a.is_none() || self.prefill_work_buf_b.is_none() {
            wlog("[MC8] decode forward: prefill work buffers missing — cannot run super-arena");
            return 0;
        }
        // Part 3x: upload all layer weights to GPU once (idempotent; falls back if it fails).
        if !self.mc8_weights_resident {
            let _ = self.mc8_upload_all_resident_weights(index);
        }
        let limit = if max_layers == 0 {
            n_layer
        } else {
            max_layers.min(n_layer)
        };
        let n_embd = index.hyperparams.n_embd as usize;
        if emb_dim < n_embd || n_embd > hidden.len() || n_embd > self.gemm_max_input_floats {
            return 0;
        }
        let prefill_scratch = match self.prefill_scratch_buf.as_ref() {
            Some(b) => b,
            None => return 0,
        };
        let batch_buf = self.gemm_input_buf.as_ref().unwrap();
        let token_buf = self.gemm_output_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        self.gpu_queue().write_buffer(
            batch_buf,
            0,
            bytemuck::cast_slice(&hidden[..n_embd]),
        );
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return 0,
        };
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return 0,
        };
        let n_tokens = 1u32;
        let mut ran = 0u32;
        // Phase 5.4 single-submit: one encoder + monotonic uniform cursors across the whole forward;
        // flush only at MC8_LAYERS_PER_ENCODER chunk boundaries → 1 submit for ≤64-layer models (vs
        // the old 2/layer). Per-layer write_buffer races are gone (resident norms + accumulating
        // cursors), so KV/work-buffer visibility relies on WebGPU intra-encoder barriers.
        let mut layer_uniform_cursors = Mc8ChunkUniformCursors { attn: 0, elem: 0, gemm: 0 };
        let mut enc = WasmGpuPipeline::begin(self);
        for layer in 0..limit {
            if layer > 0 && (layer % MC8_LAYERS_PER_ENCODER) == 0 {
                self.mc8_flush(&mut enc);
                layer_uniform_cursors.reset();
            }
            let tensors = index.get_layer_tensors(layer);
            let k_info = match tensors.attn_k.as_ref() {
                Some(i) => i,
                None => break,
            };
            let v_info = match tensors.attn_v.as_ref() {
                Some(i) => i,
                None => break,
            };
            let k_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
                Ok(s) => s,
                Err(_) => break,
            };
            let v_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
                Ok(s) => s,
                Err(_) => break,
            };
            let h = index.hyperparams;
            let n_kv = h.effective_n_kv_head();
            let used_attn_norm = tensors.attn_norm.is_some();
            let (uniforms, geom) = match self.mc8_stage_prefill_layer_super_arena(
                index,
                layer,
                &tensors,
                token_idx,
                n_tokens,
                emb_dim,
                used_attn_norm,
                k_info,
                &k_raw,
                v_info,
                &v_raw,
                &mut layer_uniform_cursors,
            ) {
                Some(v) => v,
                None => break,
            };
            let attn_src = if used_attn_norm {
                if let (Some(norm), Some(off)) =
                    (tensors.attn_norm.as_ref(), uniforms.attn_norm_elem_off)
                {
                    let (norm_b, norm_b_off) = match self
                        .mc8_norm_source(mmap, index.tensor_data_start, norm, n_embd, layer, false)
                    {
                        Some(v) => v,
                        None => break,
                    };
                    self.encode_elem_offset(
                        &mut enc,
                        ELEM_OP_RMS_NORM,
                        n_embd as u32,
                        n_tokens,
                        batch_buf,
                        0,
                        geom.batch_in_bytes,
                        norm_b,
                        norm_b_off,
                        geom.n_embd_bytes,
                        prefill_scratch,
                        0,
                        geom.batch_in_bytes,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        off,
                    );
                }
                prefill_scratch
            } else {
                batch_buf
            };
            let n_kv_wg = n_tokens.saturating_mul(n_kv);
            // Phase 5.5: K/V projection on the parallel GEMM → proj buffers; the (now lightweight)
            // attention shader reads the pre-computed projection instead of matmul-ing on 1 thread.
            let kv_dim = (n_kv * h.head_dim()) as usize;
            let kv_proj_bytes = (kv_dim * n_tokens as usize * 4) as wgpu::BufferAddress;
            let k_proj = self.mc8_k_proj_buf.as_ref().unwrap();
            let v_proj = self.mc8_v_proj_buf.as_ref().unwrap();
            if !self.encode_gemm_bufs_offset(
                &mut enc, k_info, k_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                k_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_k_gemm, layer, Mc8WeightRole::AttnK,
            ) {
                break;
            }
            if !self.encode_gemm_bufs_offset(
                &mut enc, v_info, v_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                v_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_v_gemm, layer, Mc8WeightRole::AttnV,
            ) {
                break;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                k_proj,
                token_buf,
                n_embd,
                n_tokens,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                k_info,
                k_raw,
                1,
                n_kv_wg,
                uniforms.k_off,
                Mc8WeightRole::AttnK,
            ) {
                break;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                v_proj,
                token_buf,
                n_embd,
                n_tokens,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                v_info,
                v_raw,
                2,
                n_kv_wg,
                uniforms.v_off,
                Mc8WeightRole::AttnV,
            ) {
                break;
            }
            // Phase 5.4: NO per-layer flush. KV-cache + work-buffer visibility now relies on WebGPU's
            // automatic intra-encoder barriers between compute passes (the per-layer write_buffer
            // races that previously forced a flush are gone: resident norms + accumulating uniforms).
            let work_a = self.prefill_work_buf_a.as_ref().unwrap();
            let work_b = self.prefill_work_buf_b.as_ref().unwrap();
            if !self.encode_prefill_q_ffn_tail_fused(
                &mut enc,
                index,
                layer,
                &tensors,
                batch_buf,
                attn_src,
                work_a,
                work_b,
                n_tokens,
                token_idx,
                emb_dim,
                used_attn_norm,
                &uniforms,
                &geom,
            ) {
                break;
            }
            ran += 1;
        }
        // Phase 5.4: ONE submit for the whole forward (or per chunk if n_layer > MC8_LAYERS_PER_ENCODER).
        self.mc8_flush(&mut enc);
        if ran > 0 && !self.pipeline_read_hidden(emb_dim, hidden).await {
            return 0;
        }
        ran
    }

    /// Topological speculative verify — accept longest draft prefix (B3.1d).
    pub fn verify_topology_draft_batch(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        ctx: &mut Vec<u32>,
        draft: &crate::compute_universe::TopologyDraftBatch,
        emb_dim: usize,
        emb_buf: &mut [f32],
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        max_layers: u32,
        max_vocab_chunks: u32,
    ) -> u32 {
        let mmap = match self.gguf_mmap.clone() {
            Some(m) => m,
            None => return 0,
        };
        let gamma = draft.draft_len as usize;
        if gamma == 0 || ctx.is_empty() {
            return 0;
        }
        let mut accepted = 0u32;
        for i in 0..gamma {
            let cur = *ctx.last().unwrap();
            let token_idx = ctx.len().saturating_sub(1) as u32;
            let hidden_ok = index.dequantize_token_embedding_into(
                mmap.as_ref(),
                cur,
                &mut emb_buf[..emb_dim],
            );
            if hidden_ok == 0 {
                break;
            }
            let _ = self.dispatch_transformer_forward(
                index,
                &mut emb_buf[..emb_dim],
                emb_dim,
                scratch_a,
                scratch_b,
                token_idx,
                max_layers,
            );
            let pred = if let Some(argmax) = self.dispatch_output_argmax_chunked(
                index,
                &emb_buf[..emb_dim],
                emb_dim,
                scratch_a,
                max_vocab_chunks,
                None,
            ) {
                if argmax.max_logit > f32::NEG_INFINITY {
                    argmax.best_token_id
                } else {
                    break;
                }
            } else {
                break;
            };
            if pred != draft.draft_ids[i] {
                break;
            }
            ctx.push(pred);
            accepted += 1;
        }
        accepted
    }

    /// Final logits via chunked projection into `logits_out` (fills min(vocab, buf) rows).
    pub fn dispatch_output_logits_into(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        logits_out: &mut [f32],
    ) -> usize {
        let Some(info) = index.logits_projection_info() else {
            let n = emb_dim.min(logits_out.len());
            logits_out[..n].copy_from_slice(&hidden[..n]);
            return n;
        };
        let (n_in, vocab_size) = Self::matmul_dims(info);
        let fill = vocab_size.min(logits_out.len());
        if n_in > emb_dim || fill == 0 {
            let n = emb_dim.min(logits_out.len());
            logits_out[..n].copy_from_slice(&hidden[..n]);
            return n;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => {
                let n = emb_dim.min(logits_out.len());
                logits_out[..n].copy_from_slice(&hidden[..n]);
                return n;
            }
        };
        let mut written = 0usize;
        let n_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        for chunk_idx in 0..n_chunks {
            if written >= fill {
                break;
            }
            let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
            let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
            let raw = match crate::ggml_quants::fetch_tensor_row_range_bytes(
                mmap,
                index.tensor_data_start,
                info,
                row_start,
                chunk_rows,
            ) {
                Ok(s) => s,
                Err(_) => break,
            };
            let out_rows = chunk_rows.min(fill - written);
            if !self.dispatch_gemm_raw_into(
                info,
                raw,
                &hidden[..n_in],
                &mut logits_out[written..written + out_rows],
                n_in,
                out_rows,
            ) {
                break;
            }
            written += out_rows;
        }
        if written > 0 {
            written
        } else {
            let n = emb_dim.min(logits_out.len());
            logits_out[..n].copy_from_slice(&hidden[..n]);
            n
        }
    }

    pub fn decode_lexicon_bound(&self, _logits: &[f32], valid_lexicon_ids: &[u64]) -> u64 {
        if valid_lexicon_ids.is_empty() {
            0
        } else {
            valid_lexicon_ids[0]
        }
    }

#[cfg(target_arch = "wasm32")]
    async fn dispatch_gemm_raw_into_async(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            return false;
        }

        let weight_bytes = raw.len();
        if ggml_gpu_attention_shader_supported(info.ggml_type)
            && n_in <= MAX_STACK_GEMM_IN
            && n_out <= self.gemm_max_out_dim as usize
            && weight_bytes <= self.max_tensor_bytes
            && self.gemm_input_buf.is_some()
        {
            let params = GemmGpuParams {
                n_in: n_in as u32,
                n_out: n_out as u32,
                weight_ggml_type: info.ggml_type,
                weight_row_elems: info.dims[0] as u32,
                weight_byte_len: raw.len() as u32,
                n_batch: 1,
                in_row_stride: 0,
                out_row_stride: 0,
            };
            let input_buf = self.gemm_input_buf.as_ref().unwrap();
            let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
            let output_buf = self.gemm_output_buf.as_ref().unwrap();
            let params_buf = self.gemm_params_buf.as_ref().unwrap();
            let staging = self.gemm_output_staging.as_ref().unwrap();

            self.gpu_queue()
                .write_buffer(input_buf, 0, bytemuck::cast_slice(&input[..n_in]));
            self.write_weight_words(raw, self.max_tensor_bytes);
            self.gpu_queue()
                .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));

            let bind_layout = self.pipeline.get_bind_group_layout(0);
            let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("LayerGemmBindGroup"),
                layout: &bind_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weight_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buf.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = self
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("LayerGemmEncoder"),
                });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
                });
                cpass.set_pipeline(&self.pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
            }
            let out_bytes = (n_out * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
            crate::llm_gpu_profiler::resolve(&mut encoder);
            self.gpu_queue().submit(Some(encoder.finish()));
            crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm);

            let slice = staging.slice(..out_bytes);
            if await_wgpu_map(slice).await {
                let data = slice.get_mapped_range();
                let floats: &[f32] = bytemuck::cast_slice(&data);
                out[..n_out].copy_from_slice(&floats[..n_out]);
                drop(data);
                staging.unmap();
                return true;
            }
            let _ = staging.unmap();
        }

        stack_gemm_quant(raw, info, input, out, n_in, n_out)
    }

    /// Phase 5.3: quantized GEMM against a **resident** weight sub-range (no per-call weight upload).
    /// Same dispatch+readback as `dispatch_gemm_raw_into_async`, but binds the caller-provided
    /// `weight` BindingResource (a slice of the resident output-projection buffer) instead of
    /// `write_buffer`-ing the chunk. Used by the decode argmax once the logits projection is resident.
    #[cfg(target_arch = "wasm32")]
    async fn dispatch_gemm_resident_chunk_async(
        &self,
        weight_ggml_type: u32,
        weight_row_elems: u32,
        weight: wgpu::BindingResource<'_>,
        weight_byte_len: u32,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            return false;
        }
        if n_in > MAX_STACK_GEMM_IN || n_out > self.gemm_max_out_dim as usize {
            return false;
        }
        let input_buf = match self.gemm_input_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let output_buf = match self.gemm_output_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let params_buf = match self.gemm_params_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let staging = match self.gemm_output_staging.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let params = GemmGpuParams {
            n_in: n_in as u32,
            n_out: n_out as u32,
            weight_ggml_type,
            weight_row_elems,
            weight_byte_len,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        self.gpu_queue()
            .write_buffer(input_buf, 0, bytemuck::cast_slice(&input[..n_in]));
        self.gpu_queue()
            .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));

        let bind_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ResidentLogitsBind"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight,
                },
                // self.pipeline uses MC8GemmBGL, whose binding 2 is a DYNAMIC uniform — bind it
                // as a 256-sized dynamic slice and pass offset 0 below (matches encode_gemm_bufs_offset).
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: Self::mc8_dynamic_uniform_binding(params_buf),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ResidentLogitsEncoder"),
            });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[0]);
            cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
        }
        let out_bytes = (n_out * 4) as wgpu::BufferAddress;
        encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm);

        let slice = staging.slice(..out_bytes);
        if await_wgpu_map(slice).await {
            let data = slice.get_mapped_range();
            let floats: &[f32] = bytemuck::cast_slice(&data);
            out[..n_out].copy_from_slice(&floats[..n_out]);
            drop(data);
            staging.unmap();
            return true;
        }
        let _ = staging.unmap();
        false
    }
#[cfg(target_arch = "wasm32")]
    pub async fn dispatch_gemm_into_async(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        if n_in > input.len() || n_out > out.len() {
            wlog(&format!(
                "[gemm_into_async] GUARD n_in={n_in} n_out={n_out} input={} out={}",
                input.len(),
                out.len()
            ));
            return false;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info)
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        self.dispatch_gemm_raw_into_async(info, raw, input, out, n_in, n_out).await
    }
#[cfg(target_arch = "wasm32")]
    async fn dispatch_attention_layer_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &[f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> Option<usize> {
        let layout = self.kv_layout?;
        let q_info = tensors.attn_q.as_ref()?;
        let k_info = tensors.attn_k.as_ref()?;
        let v_info = tensors.attn_v.as_ref()?;
        let h = index.hyperparams;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        let head_dim = h.head_dim() as usize;
        if head_dim == 0 || n_head == 0 || n_kv == 0 {
            return None;
        }
        let q_dim = n_head * head_dim;
        if q_dim > scratch_a.len() || q_dim > scratch_b.len() || emb_dim < h.n_embd as usize {
            return None;
        }
        if !ggml_gpu_quant_supported(q_info.ggml_type)
            || !ggml_gpu_quant_supported(k_info.ggml_type)
            || !ggml_gpu_quant_supported(v_info.ggml_type)
        {
            return None;
        }

        let mmap = self.gguf_mmap.as_deref()?;
        let k_raw =
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info).ok()?;
        let v_raw =
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info).ok()?;
        let q_raw =
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info).ok()?;
        let n_embd = h.n_embd as usize;

        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let mut h_norm_attn = [0f32; MAX_HIDDEN_DIM];
        let hidden_input = prepare_pre_norm_input(
            &hidden[..emb_dim],
            emb_dim,
            tensors.attn_norm.as_ref(),
            Some(&mmap[..]),
            index.tensor_data_start,
            &mut h_norm_attn,
            &mut norm_w_attn,
        );

        if !self
            .dispatch_attention_pass_async(
                hidden_input,
                n_embd,
                1,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                k_info,
                k_raw,
                1,
                n_kv as u32,
                None,
            )
            .await
        {
            return None;
        }
        if !self
            .dispatch_attention_pass_async(
                hidden_input,
                n_embd,
                1,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                v_info,
                v_raw,
                2,
                n_kv as u32,
                None,
            )
            .await
        {
            return None;
        }
        if !self
            .dispatch_attention_pass_async(
                hidden_input,
                n_embd,
                1,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                q_info,
                q_raw,
                0,
                n_head as u32,
                Some(&mut scratch_b[..q_dim]),
            )
            .await
        {
            return None;
        }

        if let Some(out_info) = tensors.attn_output {
            let (o_in, o_out) = Self::matmul_dims(&out_info);
            if o_in <= q_dim
                && self
                    .dispatch_gemm_into_async(
                        index,
                        &out_info,
                        &scratch_b[..o_in],
                        &mut scratch_a[..o_out],
                        o_in,
                        o_out,
                    )
                    .await
            {
                return Some(o_out.min(emb_dim));
            }
        }
        let n = q_dim.min(emb_dim);
        scratch_a[..n].copy_from_slice(&scratch_b[..n]);
        Some(n)
    }

#[cfg(target_arch = "wasm32")]
    async fn dispatch_attention_pass_async(
        &self,
        hidden: &[f32],
        n_embd: usize,
        num_tokens_in_batch: u32,
        batch_start_token_idx: u32,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        info: &GgufTensorInfo,
        raw_weights: &[u8],
        proj_kind: u32,
        n_workgroups: u32,
        readback_out: Option<&mut [f32]>,
    ) -> bool {
        if !ggml_gpu_quant_supported(info.ggml_type) {
            wlog(&format!("[attn_pass_async] GUARD unsupported quant kind={proj_kind}"));
            return false;
        }
        if !ggml_gpu_attention_shader_supported(info.ggml_type) {
            return self.cpu_attention_pass(
                hidden,
                n_embd,
                num_tokens_in_batch,
                batch_start_token_idx,
                layout,
                layer,
                h,
                info,
                raw_weights,
                proj_kind,
                None,
                readback_out,
            );
        }
        let batch = num_tokens_in_batch.max(1) as usize;
        let hidden_elems = n_embd.checked_mul(batch).unwrap_or(0);
        if hidden_elems > hidden.len()
            || hidden_elems > self.gemm_max_input_floats
            || raw_weights.len() > self.max_tensor_bytes
            || self.gemm_input_buf.is_none()
            || self.kv_cache_gpu.is_none()
            || self.attention_params_buf.is_none()
            || self.attention_mask_buf.is_none()
        {
            wlog(&format!(
                "[attn_pass_async] GUARD buffers kind={proj_kind} hidden_elems={hidden_elems} hidden={} gemm_in={} raw_w={} max_w={}",
                hidden.len(),
                self.gemm_max_input_floats,
                raw_weights.len(),
                self.max_tensor_bytes,
            ));
            return false;
        }

        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(layout, token_idx, proj_kind);
        let params = Self::attention_gpu_params(
            h,
            layout,
            layer,
            token_idx,
            info,
            raw_weights.len(),
            proj_kind,
            num_tokens_in_batch.max(1),
            batch_start_token_idx,
            mask_active,
            mask_word_count,
            0,
        );
        let input_buf = self.gemm_input_buf.as_ref().unwrap();
        let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
        let output_buf = self.gemm_output_buf.as_ref().unwrap();
        let params_buf = self.attention_params_buf.as_ref().unwrap();
        let mask_buf = self.attention_mask_buf.as_ref().unwrap();
        let kv_buf = self.kv_cache_gpu.as_ref().unwrap();
        let staging = self.gemm_output_staging.as_ref().unwrap();

        self.gpu_queue()
            .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..hidden_elems]));
        self.write_weight_words(raw_weights, self.max_tensor_bytes);
        self.gpu_queue()
            .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));
        self.gpu_queue()
            .write_buffer(mask_buf, 0, bytemuck::cast_slice(&mask_words));

        // Bind one layer slice of the KV arena (full arena exceeds 128 MiB wgpu binding cap).
        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset =
            (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding {
            buffer: kv_buf,
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };
        let (wg_x, wg_y) = if proj_kind == 0 && num_tokens_in_batch > 1 {
            (h.n_head, num_tokens_in_batch)
        } else {
            (n_workgroups.max(1), 1)
        };

        let bind_layout = self.attention_pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("FusedAttentionBindGroup"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weight_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(kv_binding),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: mask_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FusedAttentionEncoder"),
            });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FusedAttentionPass"),
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.attention_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(wg_x, wg_y, 1);
        }

        let readback_elems = readback_out.as_ref().map(|o| o.len()).unwrap_or(0);
        if readback_elems > 0 {
            let out_bytes = (readback_elems * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
        }
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Attention);

        if readback_elems == 0 {
            return true;
        }

        let out_bytes = (readback_elems * 4) as wgpu::BufferAddress;
        let slice = staging.slice(..out_bytes);
        if await_wgpu_map(slice).await {
            let data = slice.get_mapped_range();
            let floats: &[f32] = bytemuck::cast_slice(&data);
            if let Some(out) = readback_out {
                out[..readback_elems].copy_from_slice(&floats[..readback_elems]);
            }
            drop(data);
            staging.unmap();
            return true;
        }
        let _ = staging.unmap();
        false
    }
#[cfg(target_arch = "wasm32")]
    async fn dispatch_attention_q_ffn_token_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        hidden: &mut [f32],
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let q_info = match tensors.attn_q.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let h = index.hyperparams;
        let n_head = h.n_head as usize;
        let head_dim = h.head_dim() as usize;
        let q_dim = n_head * head_dim;
        if q_dim > scratch_a.len() || q_dim > scratch_b.len() || emb_dim < h.n_embd as usize {
            return false;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let q_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let n_embd = h.n_embd as usize;
        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let mut h_norm_attn = [0f32; MAX_HIDDEN_DIM];
        let hidden_input = prepare_pre_norm_input(
            &hidden[..emb_dim],
            emb_dim,
            tensors.attn_norm.as_ref(),
            Some(&mmap[..]),
            index.tensor_data_start,
            &mut h_norm_attn,
            &mut norm_w_attn,
        );
        if !self
            .dispatch_attention_pass_async(
                hidden_input,
                n_embd,
                1,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                q_info,
                q_raw,
                0,
                n_head as u32,
                Some(&mut scratch_b[..q_dim]),
            )
            .await
        {
            return false;
        }
        let mut attn_ok = false;
        if let Some(out_info) = tensors.attn_output.as_ref() {
            let (o_in, o_out) = Self::matmul_dims(out_info);
            if o_in <= q_dim
                && self
                    .dispatch_gemm_into_async(
                        index,
                        out_info,
                        &scratch_b[..o_in],
                        &mut scratch_a[..o_out],
                        o_in,
                        o_out,
                    )
                    .await
            {
                add_residual_inplace(
                    &mut hidden[..emb_dim],
                    &scratch_a[..o_out],
                    emb_dim.min(o_out),
                );
                attn_ok = true;
            }
        } else {
            let n = q_dim.min(emb_dim);
            add_residual_inplace(&mut hidden[..emb_dim], &scratch_b[..n], n);
            attn_ok = true;
        }
        if !attn_ok {
            return false;
        }
        self.dispatch_ffn_block_pre_norm_async(
            index,
            hidden,
            emb_dim,
            tensors,
            scratch_a,
            scratch_b,
        )
        .await
    }
#[cfg(target_arch = "wasm32")]
    pub async fn dispatch_prefill_chunk_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        max_layers: u32,
        l1_hidden_out: Option<&mut [f32]>,
    ) -> bool {
        if self
            .dispatch_prefill_chunk_async_mc8_gpu(
                index,
                batch_hidden,
                emb_dim,
                n_tokens,
                batch_start_token_idx,
                scratch_a,
                scratch_b,
                max_layers,
                l1_hidden_out,
            )
            .await
        {
            return true;
        }
        wlog("[MC8] GPU prefill FAILED — CPU fallback blocked (manifold unification)");
        false
    }

    /// Part 3u: stage K/V/Q + tail uniforms in three single `write_buffer` calls (pre-encoder).
    #[cfg(target_arch = "wasm32")]
    fn mc8_stage_prefill_layer_super_arena(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        tensors: &crate::gguf_sharder::LayerTensors,
        batch_start_token_idx: u32,
        n_tokens: u32,
        emb_dim: usize,
        used_attn_norm: bool,
        k_info: &GgufTensorInfo,
        k_raw: &[u8],
        v_info: &GgufTensorInfo,
        v_raw: &[u8],
        cursors: &mut Mc8ChunkUniformCursors,
    ) -> Option<(Mc8PrefillLayerUniforms, Mc8PrefillLayerGeom)> {
        let mmap = self.gguf_mmap.as_deref()?;
        let layout = self.kv_layout?;
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let q_info = tensors.attn_q.as_ref()?;
        let gate_info = tensors.ffn_gate.as_ref()?;
        let up_info = tensors.ffn_up.as_ref()?;
        let down_info = tensors.ffn_down.as_ref()?;
        let q_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info).ok()?;
        let n_ffn_est = Self::matmul_dims(gate_info).1;
        let row_stride = Self::mc8_prefill_row_stride(n_embd, n_ffn_est, self.gemm_max_out_dim);
        let emb_bytes = (emb_dim * 4) as wgpu::BufferAddress;
        let n_embd_bytes = (n_embd * 4) as wgpu::BufferAddress;
        let q_dim = (h.n_head * h.head_dim()) as usize;
        let q_bytes = (q_dim * 4) as wgpu::BufferAddress;
        let slot_o = q_bytes;
        let slot_scratch_half = ((row_stride / 2) * 4) as wgpu::BufferAddress;
        let (_, n_ffn) = Self::matmul_dims(gate_info);
        let slot_gate = n_embd_bytes;
        let slot_up = slot_gate + (n_ffn * 4) as wgpu::BufferAddress;
        let slot_save = slot_up + (n_ffn * 4) as wgpu::BufferAddress;
        let row_stride_u32 = row_stride as u32;
        let batch_in_bytes = (n_embd * n_tokens as usize * 4) as wgpu::BufferAddress;
        let work_span_bytes =
            (n_tokens as usize * row_stride * 4).max(q_bytes as usize) as wgpu::BufferAddress;
        let geom = Mc8PrefillLayerGeom {
            row_stride,
            row_stride_u32,
            batch_in_bytes,
            work_span_bytes,
            emb_bytes,
            n_embd_bytes,
            slot_o,
            slot_gate,
            slot_up,
            slot_save,
            slot_scratch_half,
            slot_o_f: (slot_o / 4) as u32,
            slot_save_f: (slot_save / 4) as u32,
            slot_gate_f: (slot_gate / 4) as u32,
            slot_up_f: (slot_up / 4) as u32,
            slot_scratch_half_f: (slot_scratch_half / 4) as u32,
        };
        let (gate_in, _) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > n_embd || up_in != gate_in || up_out != n_ffn || dn_in != n_ffn || dn_out < n_embd {
            return None;
        }
        let out_info = tensors.attn_output.as_ref();
        let (o_in, o_out) = out_info.map(Self::matmul_dims).unwrap_or((q_dim, n_embd));
        if out_info.is_some() && o_in > q_dim {
            return None;
        }
        let o_raw = out_info
            .and_then(|i| crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, i).ok());
        let gate_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info).ok()?;
        let up_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info).ok()?;
        let down_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info).ok()?;

        let attn_base_byte = cursors.attn_base_byte();
        let elem_base_byte = cursors.elem_base_byte();
        let gemm_base_byte = cursors.gemm_base_byte();
        let mut attn_arena = Mc8AttnUniformArena {
            bytes: [0u8; MC8_MAX_ATTN_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        // Phase 5.5: projection decoupling — the attention shader reads PRE-COMPUTED Q/K/V
        // (parallel GEMM) at `proj_row_stride` floats/token instead of doing the matmul itself.
        let kv_dim = (h.effective_n_kv_head() * h.head_dim()) as u32;
        let q_dim_u32 = (h.n_head * h.head_dim()) as u32;
        let mut k_params = Self::attention_gpu_params(
            &h, &layout, layer, batch_start_token_idx, k_info, k_raw.len(), 1, n_tokens,
            batch_start_token_idx, 0, 0, 0,
        );
        k_params.proj_row_stride = kv_dim;
        let k_off = attn_base_byte + attn_arena.push(&k_params);
        let mut v_params = Self::attention_gpu_params(
            &h, &layout, layer, batch_start_token_idx, v_info, v_raw.len(), 2, n_tokens,
            batch_start_token_idx, 0, 0, 0,
        );
        v_params.proj_row_stride = kv_dim;
        let v_off = attn_base_byte + attn_arena.push(&v_params);
        let mut q_params = Self::attention_gpu_params(
            &h, &layout, layer, batch_start_token_idx, q_info, q_raw.len(), 0, n_tokens,
            batch_start_token_idx, 0, KV_ATTENTION_MASK_WORDS as u32, row_stride_u32,
        );
        q_params.proj_row_stride = q_dim_u32;
        let q_off = attn_base_byte + attn_arena.push(&q_params);

        let mut elem_arena = Mc8ElemUniformArena {
            bytes: [0u8; MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let attn_norm_elem_off = if used_attn_norm {
            Some(elem_base_byte + elem_arena.push(&Self::mc8_elem_params(
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                n_tokens,
                0,
                0,
                0,
                0,
                0,
                0,
            )))
        } else {
            None
        };
        let off_attn_res = elem_base_byte
            + elem_arena.push(&Self::mc8_elem_params(
            ELEM_OP_ADD_RESIDUAL,
            emb_dim as u32,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            0,
            geom.slot_save_f,
            if out_info.is_some() { geom.slot_o_f } else { 0 },
            0,
        ));
        let off_ffn_norm = tensors.ffn_norm.as_ref().map(|_| {
            elem_base_byte + elem_arena.push(&Self::mc8_elem_params(
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                n_tokens,
                0,
                0,
                row_stride_u32,
                0,
                0,
                0,
            ))
        });
        let off_silu = elem_base_byte
            + elem_arena.push(&Self::mc8_elem_params(
            ELEM_OP_SILU_MUL,
            n_ffn as u32,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            row_stride_u32,
            geom.slot_gate_f,
            geom.slot_up_f,
            geom.slot_scratch_half_f,
        ));
        let off_ffn_res = elem_base_byte
            + elem_arena.push(&Self::mc8_elem_params(
            ELEM_OP_ADD_RESIDUAL,
            emb_dim as u32,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            0,
            geom.slot_save_f,
            0,
            0,
        ));

        let mut gemm_arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let off_o = out_info.map(|i| {
            gemm_base_byte
                + gemm_arena.push(&Self::mc8_gemm_params(
                    i,
                    o_raw.as_ref().map(|r| r.len()).unwrap_or(0),
                    o_in,
                    o_out,
                    n_tokens,
                    row_stride_u32,
                    row_stride_u32,
                ))
        });
        let off_gate = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                gate_info,
                gate_raw.len(),
                gate_in,
                n_ffn,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
            ));
        let off_up = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                up_info,
                up_raw.len(),
                up_in,
                n_ffn,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
            ));
        let off_down = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                down_info,
                down_raw.len(),
                dn_in,
                dn_out,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
            ));
        // Phase 5.5: Q/K/V projection GEMMs run on the parallel kernel (input contiguous n_embd/token,
        // output contiguous q_dim|kv_dim/token) — output feeds the now-lightweight attention shader.
        let off_q_gemm = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                q_info, q_raw.len(), n_embd, q_dim, n_tokens, n_embd as u32, q_dim_u32,
            ));
        let off_k_gemm = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                k_info, k_raw.len(), n_embd, kv_dim as usize, n_tokens, n_embd as u32, kv_dim,
            ));
        let off_v_gemm = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                v_info, v_raw.len(), n_embd, kv_dim as usize, n_tokens, n_embd as u32, kv_dim,
            ));

        let attn_buf = self.attention_params_buf.as_ref()?;
        let elem_buf = self.elem_params_buf.as_ref()?;
        let gemm_buf = self.gemm_params_buf.as_ref()?;
        let queue = self.gpu_queue();
        attn_arena.upload_at(queue, attn_buf, cursors.attn);
        cursors.attn += attn_arena.slots;
        elem_arena.upload_at(queue, elem_buf, cursors.elem);
        cursors.elem += elem_arena.slots;
        gemm_arena.upload_at(queue, gemm_buf, cursors.gemm);
        cursors.gemm += gemm_arena.slots;

        Some((
            Mc8PrefillLayerUniforms {
                k_off,
                v_off,
                q_off,
                attn_norm_elem_off,
                off_o,
                off_attn_res,
                off_ffn_norm,
                off_gate,
                off_up,
                off_silu,
                off_down,
                off_ffn_res,
                off_q_gemm,
                off_k_gemm,
                off_v_gemm,
            },
            geom,
        ))
    }

    /// MC8 Part 3m/3u: batched Q+FFN tail — uniforms pre-staged in super-arena.
    #[cfg(target_arch = "wasm32")]
    fn encode_prefill_q_ffn_tail_fused(
        &self,
        pipeline: &mut WasmGpuPipeline,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        tensors: &crate::gguf_sharder::LayerTensors,
        batch_buf: &wgpu::Buffer,
        attn_src: &wgpu::Buffer,
        work_a: &wgpu::Buffer,
        work_b: &wgpu::Buffer,
        n_tokens: u32,
        batch_start_token_idx: u32,
        emb_dim: usize,
        used_attn_norm: bool,
        uniforms: &Mc8PrefillLayerUniforms,
        geom: &Mc8PrefillLayerGeom,
    ) -> bool {
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        let q_info = match tensors.attn_q.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let q_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let row_stride = geom.row_stride;
        let row_stride_u32 = geom.row_stride_u32;
        let emb_bytes = geom.emb_bytes;
        let n_embd_bytes = geom.n_embd_bytes;
        let slot_o = geom.slot_o;
        let slot_gate = geom.slot_gate;
        let slot_up = geom.slot_up;
        let slot_save = geom.slot_save;
        let slot_scratch_half = geom.slot_scratch_half;
        let work_span_bytes = geom.work_span_bytes;
        let batch_in_bytes = geom.batch_in_bytes;
        let slot_o_f = geom.slot_o_f;
        let slot_save_f = geom.slot_save_f;
        let slot_gate_f = geom.slot_gate_f;
        let slot_up_f = geom.slot_up_f;
        let slot_scratch_half_f = geom.slot_scratch_half_f;
        let off_attn_res = uniforms.off_attn_res;
        let off_ffn_norm = uniforms.off_ffn_norm;
        let off_gate = uniforms.off_gate;
        let off_up = uniforms.off_up;
        let off_silu = uniforms.off_silu;
        let off_down = uniforms.off_down;
        let off_ffn_res = uniforms.off_ffn_res;
        let off_o = uniforms.off_o;

        let gate_info = match tensors.ffn_gate.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let up_info = match tensors.ffn_up.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let down_info = match tensors.ffn_down.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let (_, n_ffn) = Self::matmul_dims(gate_info);
        let q_dim = (h.n_head * h.head_dim()) as usize;
        let (gate_in, _) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > n_embd || up_in != gate_in || up_out != n_ffn || dn_in != n_ffn || dn_out < n_embd {
            return false;
        }
        let out_info = tensors.attn_output.as_ref();
        let (o_in, o_out) = out_info.map(Self::matmul_dims).unwrap_or((q_dim, n_embd));
        if out_info.is_some() && o_in > q_dim {
            return false;
        }
        let o_raw = out_info.and_then(|out_info| {
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, out_info).ok()
        });
        let gate_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let up_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let down_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info) {
                Ok(s) => s,
                Err(_) => return false,
            };

        // Phase 5.5: Q projection on the parallel GEMM → q_proj; the (now SDPA-only) attention
        // shader reads it. Moves the heavy 64×960 matmul off the @workgroup_size(1) attention kernel.
        let q_in = if used_attn_norm { attn_src } else { batch_buf };
        let q_proj = self.mc8_q_proj_buf.as_ref().unwrap();
        let q_proj_bytes = (q_dim * n_tokens as usize * 4) as wgpu::BufferAddress;
        if !self.encode_gemm_bufs_offset(
            pipeline, q_info, q_raw, n_embd, q_dim, q_in, 0, batch_in_bytes,
            q_proj, 0, q_proj_bytes, n_tokens, n_embd as u32, q_dim as u32,
            uniforms.off_q_gemm, layer, Mc8WeightRole::AttnQ,
        ) {
            return false;
        }
        // Part 3u: Q SDPA — attn params pre-staged at uniforms.q_off (now reads pre-computed Q).
        if !self.encode_attention_batched_q_prefill(
            pipeline,
            q_proj,
            0,
            q_proj_bytes,
            work_a,
            0,
            work_span_bytes,
            n_embd,
            n_tokens,
            &layout,
            layer,
            &h,
            q_raw,
            uniforms.q_off,
        ) {
            return false;
        }

        // Snapshot pristine hidden (MC8 pt3i `work_aliases_hidden`) before o_proj overwrites paths.
        for t in 0..n_tokens {
            let emb_off = Self::mc8_emb_off(t, n_embd);
            let row_off = Self::mc8_prefill_row_off(t, row_stride);
            pipeline.encoder.copy_buffer_to_buffer(
                batch_buf,
                emb_off,
                work_a,
                row_off + slot_save,
                emb_bytes,
            );
        }

        // Part 3s — attention block: o_proj + attn residual (dynamic uniform offsets).
        if let (Some(out_info), Some(o_raw)) = (out_info, o_raw) {
            if !self.encode_gemm_bufs_offset(
                pipeline,
                out_info,
                o_raw,
                o_in,
                o_out,
                work_a,
                0,
                work_span_bytes,
                work_b,
                slot_o,
                work_span_bytes,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
                off_o.unwrap(),
                layer,
                Mc8WeightRole::OProj,
            ) {
                return false;
            }
            self.encode_elem_offset(
                pipeline,
                ELEM_OP_ADD_RESIDUAL,
                emb_dim as u32,
                n_tokens,
                work_a,
                0,
                work_span_bytes,
                work_b,
                0,
                work_span_bytes,
                batch_buf,
                0,
                batch_in_bytes,
                row_stride_u32,
                row_stride_u32,
                0,
                slot_save_f,
                slot_o_f,
                0,
                off_attn_res,
            );
        } else {
            self.encode_elem_offset(
                pipeline,
                ELEM_OP_ADD_RESIDUAL,
                emb_dim as u32,
                n_tokens,
                work_a,
                0,
                work_span_bytes,
                work_a,
                0,
                work_span_bytes,
                batch_buf,
                0,
                batch_in_bytes,
                row_stride_u32,
                row_stride_u32,
                0,
                slot_save_f,
                0,
                0,
                off_attn_res,
            );
        }

        // Refresh post-attn hidden for FFN residual (overwrites pristine snapshot in slot_save).
        for t in 0..n_tokens {
            let emb_off = Self::mc8_emb_off(t, n_embd);
            let row_off = Self::mc8_prefill_row_off(t, row_stride);
            pipeline.encoder.copy_buffer_to_buffer(
                batch_buf,
                emb_off,
                work_a,
                row_off + slot_save,
                emb_bytes,
            );
        }

        // Part 3s — FFN block: norm + gate/up (one submit via dynamic offsets + weight ping-pong).
        if let Some(norm) = tensors.ffn_norm.as_ref() {
            let (norm_b, norm_b_off) = match self
                .mc8_norm_source(mmap, index.tensor_data_start, norm, n_embd, layer, true)
            {
                Some(v) => v,
                None => return false,
            };
            self.encode_elem_offset(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                n_tokens,
                batch_buf,
                0,
                batch_in_bytes,
                norm_b,
                norm_b_off,
                n_embd_bytes,
                work_b,
                0,
                work_span_bytes,
                0,
                0,
                row_stride_u32,
                0,
                0,
                0,
                off_ffn_norm.unwrap(),
            );
        } else {
            for t in 0..n_tokens {
                let emb_off = Self::mc8_emb_off(t, n_embd);
                let row_off = Self::mc8_prefill_row_off(t, row_stride);
                pipeline.encoder.copy_buffer_to_buffer(batch_buf, emb_off, work_b, row_off, emb_bytes);
            }
        }
        // Phase 5 — FFN expansion fusion: collapse gate GEMM + up GEMM + SiLU×mul into a
        // single dispatch. The result `silu(gate·x)·(up·x)` lands in work_b@slot_scratch_half
        // — exactly where the separate SiLU path wrote — so the `down` projection below is
        // unchanged. Requires resident weights (so both roles are GPU-resident) and gate/up
        // sharing ggml_type+dims (one staged GemmParams describes both). Otherwise fall back
        // to the proven 3-dispatch path.
        let fuse_ffn_expansion =
            self.mc8_weights_resident && gate_info.ggml_type == up_info.ggml_type;
        if fuse_ffn_expansion {
            // WebGPU forbids binding one buffer as both read-only and writable storage within a
            // single pass (validated at buffer granularity, not by offset). The norm wrote the
            // hidden to work_b@0, and the fused intermediate must land in work_b@slot_scratch_half
            // for `down` below — same buffer, so reading + writing work_b in one pass is illegal.
            // Stage the normalized hidden into work_a@0 (small n_embd copy; `down` overwrites
            // work_a@0 immediately after), read it there, and write the silu·up result to
            // work_b@slot_scratch_half exactly as the separate-SiLU path did — `down` unchanged.
            for t in 0..n_tokens {
                let row_off = Self::mc8_prefill_row_off(t, row_stride);
                pipeline.encoder.copy_buffer_to_buffer(work_b, row_off, work_a, row_off, emb_bytes);
            }
            if !self.encode_fused_ffn_expansion(
                pipeline,
                layer,
                work_a,
                0,
                work_span_bytes,
                work_b,
                slot_scratch_half,
                work_span_bytes,
                n_ffn as u32,
                n_tokens,
                off_gate,
            ) {
                return false;
            }
        } else {
            if !self.encode_gemm_bufs_offset(
                pipeline,
                gate_info,
                gate_raw,
                gate_in,
                n_ffn,
                work_b,
                0,
                work_span_bytes,
                work_a,
                slot_gate,
                work_span_bytes,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
                off_gate,
                layer,
                Mc8WeightRole::Gate,
            ) {
                return false;
            }
            if !self.encode_gemm_bufs_offset(
                pipeline,
                up_info,
                up_raw,
                up_in,
                n_ffn,
                work_b,
                0,
                work_span_bytes,
                work_a,
                slot_up,
                work_span_bytes,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
                off_up,
                layer,
                Mc8WeightRole::Up,
            ) {
                return false;
            }

            self.encode_elem_offset(
                pipeline,
                ELEM_OP_SILU_MUL,
                n_ffn as u32,
                n_tokens,
                work_a,
                0,
                work_span_bytes,
                work_a,
                0,
                work_span_bytes,
                work_b,
                0,
                work_span_bytes,
                row_stride_u32,
                row_stride_u32,
                row_stride_u32,
                slot_gate_f,
                slot_up_f,
                slot_scratch_half_f,
                off_silu,
            );
        }
        if !self.encode_gemm_bufs_offset(
            pipeline,
            down_info,
            down_raw,
            dn_in,
            dn_out,
            work_b,
            slot_scratch_half,
            work_span_bytes,
            work_a,
            0,
            work_span_bytes,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            off_down,
            layer,
            Mc8WeightRole::Down,
        ) {
            return false;
        }
        self.encode_elem_offset(
            pipeline,
            ELEM_OP_ADD_RESIDUAL,
            emb_dim as u32,
            n_tokens,
            work_a,
            0,
            work_span_bytes,
            work_a,
            0,
            work_span_bytes,
            batch_buf,
            0,
            batch_in_bytes,
            row_stride_u32,
            row_stride_u32,
            0,
            slot_save_f,
            0,
            0,
            off_ffn_res,
        );
        true
    }

    #[cfg(target_arch = "wasm32")]
    async fn dispatch_prefill_chunk_async_mc8_gpu(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        _scratch_a: &mut [f32],
        _scratch_b: &mut [f32],
        max_layers: u32,
        mut l1_hidden_out: Option<&mut [f32]>,
    ) -> bool {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || n_tokens == 0 || !self.mc8_buffers_ready() {
            return false;
        }
        // Part 3x: upload all layer weights to GPU once (idempotent; falls back if it fails).
        if !self.mc8_weights_resident {
            let _ = self.mc8_upload_all_resident_weights(index);
        }
        let prefill_scratch = match self.prefill_scratch_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let limit = if max_layers == 0 {
            n_layer
        } else {
            max_layers.min(n_layer)
        };
        let n_embd = index.hyperparams.n_embd as usize;
        let batch_elems = n_embd * n_tokens as usize;
        if batch_elems > batch_hidden.len() || emb_dim < n_embd {
            return false;
        }
        let batch_buf = self.gemm_input_buf.as_ref().unwrap();
        let token_buf = self.gemm_output_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        if batch_elems > self.gemm_max_input_floats {
            return false;
        }
        self.gpu_queue().write_buffer(
            batch_buf,
            0,
            bytemuck::cast_slice(&batch_hidden[..batch_elems]),
        );
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        for layer in 0..limit {
            let tensors = index.get_layer_tensors(layer);
            let k_info = match tensors.attn_k.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let v_info = match tensors.attn_v.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let k_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info)
            {
                Ok(s) => s,
                Err(_) => return false,
            };
            let v_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info)
            {
                Ok(s) => s,
                Err(_) => return false,
            };
            let h = index.hyperparams;
            let n_kv = h.effective_n_kv_head();
            let used_attn_norm = tensors.attn_norm.is_some();
            let mut layer_uniform_cursors = Mc8ChunkUniformCursors {
                attn: 0,
                elem: 0,
                gemm: 0,
            };
            let (uniforms, geom) = match self.mc8_stage_prefill_layer_super_arena(
                index,
                layer,
                &tensors,
                batch_start_token_idx,
                n_tokens,
                emb_dim,
                used_attn_norm,
                k_info,
                &k_raw,
                v_info,
                &v_raw,
                &mut layer_uniform_cursors,
            ) {
                Some(v) => v,
                None => return false,
            };
            let mut enc = WasmGpuPipeline::begin(self);
            let attn_src = if used_attn_norm {
                if let (Some(norm), Some(off)) =
                    (tensors.attn_norm.as_ref(), uniforms.attn_norm_elem_off)
                {
                    let (norm_b, norm_b_off) = match self
                        .mc8_norm_source(mmap, index.tensor_data_start, norm, n_embd, layer, false)
                    {
                        Some(v) => v,
                        None => return false,
                    };
                    self.encode_elem_offset(
                        &mut enc,
                        ELEM_OP_RMS_NORM,
                        n_embd as u32,
                        n_tokens,
                        batch_buf,
                        0,
                        geom.batch_in_bytes,
                        norm_b,
                        norm_b_off,
                        geom.n_embd_bytes,
                        prefill_scratch,
                        0,
                        geom.batch_in_bytes,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        off,
                    );
                }
                prefill_scratch
            } else {
                batch_buf
            };
            let n_kv_wg = n_tokens.saturating_mul(n_kv);
            // Phase 5.5: K/V projection on the parallel GEMM → proj buffers (attention reads them).
            let kv_dim = (n_kv * h.head_dim()) as usize;
            let kv_proj_bytes = (kv_dim * n_tokens as usize * 4) as wgpu::BufferAddress;
            let k_proj = self.mc8_k_proj_buf.as_ref().unwrap();
            let v_proj = self.mc8_v_proj_buf.as_ref().unwrap();
            if !self.encode_gemm_bufs_offset(
                &mut enc, k_info, k_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                k_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_k_gemm, layer, Mc8WeightRole::AttnK,
            ) {
                return false;
            }
            if !self.encode_gemm_bufs_offset(
                &mut enc, v_info, v_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                v_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_v_gemm, layer, Mc8WeightRole::AttnV,
            ) {
                return false;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                k_proj,
                token_buf,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                k_info,
                k_raw,
                1,
                n_kv_wg,
                uniforms.k_off,
                Mc8WeightRole::AttnK,
            ) {
                return false;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                v_proj,
                token_buf,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                v_info,
                v_raw,
                2,
                n_kv_wg,
                uniforms.v_off,
                Mc8WeightRole::AttnV,
            ) {
                return false;
            }
            // Part 3u: KV cache writes must be queue-visible before Q-SDPA reads (backend empirical).
            self.mc8_flush(&mut enc);
            const MC8_FUSED_PREFILL_TAIL: bool = true;
            let use_fused_tail = MC8_FUSED_PREFILL_TAIL
                && self.prefill_work_buf_a.is_some()
                && self.prefill_work_buf_b.is_some();
            if use_fused_tail {
                let work_a = self.prefill_work_buf_a.as_ref().unwrap();
                let work_b = self.prefill_work_buf_b.as_ref().unwrap();
                if !self.encode_prefill_q_ffn_tail_fused(
                    &mut enc,
                    index,
                    layer,
                    &tensors,
                    batch_buf,
                    attn_src,
                    work_a,
                    work_b,
                    n_tokens,
                    batch_start_token_idx,
                    emb_dim,
                    used_attn_norm,
                    &uniforms,
                    &geom,
                ) {
                    return false;
                }
            } else {
                let token_buf = self.gemm_output_buf.as_ref().unwrap();
                let aux_buf = self.gemm_aux_buf.as_ref().unwrap();
                let token_bytes = (n_embd * 4) as wgpu::BufferAddress;
                for t in 0..n_tokens {
                    let abs = batch_start_token_idx + t;
                    let off = Self::mc8_emb_off(t, n_embd);
                    enc.encoder.copy_buffer_to_buffer(batch_buf, off, token_buf, 0, token_bytes);
                    let attn_in = if used_attn_norm {
                        enc.encoder.copy_buffer_to_buffer(
                            prefill_scratch,
                            off,
                            aux_buf,
                            0,
                            token_bytes,
                        );
                        self.mc8_flush(&mut enc);
                        Some(aux_buf)
                    } else {
                        None
                    };
                    if !self.encode_attn_ffn_tail_gpu(
                        &mut enc,
                        index,
                        layer,
                        abs,
                        emb_dim,
                        &tensors,
                        token_buf,
                        attn_in,
                        true,
                    ) {
                        return false;
                    }
                    self.mc8_flush(&mut enc);
                    enc.encoder.copy_buffer_to_buffer(token_buf, 0, batch_buf, off, token_bytes);
                    self.mc8_flush(&mut enc);
                }
            }
            // Part 3u: one submit per layer (KV flush + layer-end flush = 2 submits/layer).
            self.mc8_flush(&mut enc);
            let _ = l1_hidden_out.take();
        }
        wlog(&format!(
            "[MC8] GPU prefill OK layers={limit} n_tokens={n_tokens} start={batch_start_token_idx}"
        ));
        true
    }
#[cfg(target_arch = "wasm32")]
    pub async fn dispatch_fused_transformer_block_async(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
        let rows = tensor.shape.get(0).copied().unwrap_or(4096);
        let cols = tensor.shape.get(1).copied().unwrap_or(4096);

        // ── DirectML path (Windows) ───────────────────────────────────────────
        #[cfg(target_os = "windows")]
        if let Some(dml) = &self.dml {
            if let Some(mmap) = &self.gguf_mmap {
                let offset = self.tensor_data_offset + tensor.byte_offset;
                let q4_bytes_needed = (rows * cols / crate::directml_bridge::Q4_K_BLOCK_SIZE)
                    * crate::directml_bridge::Q4_K_BLOCK_BYTES;
                if (offset as usize + q4_bytes_needed) <= mmap.len() {
                    let q4_slice = &mmap[offset as usize..offset as usize + q4_bytes_needed];
                    let weights_f32 =
                        crate::directml_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                    let op = crate::directml_bridge::DmlGemmOp {
                        m: input_activations.len() as u32 / cols as u32,
                        k: cols as u32,
                        n: rows as u32,
                    };
                    if let Ok(result) = op.execute(dml, input_activations, &weights_f32) {
                        crate::telemetry::SIEVE_OPS_COUNT
                            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                        return result;
                    }
                }
            }
        }

        // ── Accelerate BLAS path (macOS / Apple Silicon AMX) ─────────────────────
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let q4_bytes_needed = (rows * cols / crate::metal_bridge::Q4_K_BLOCK_SIZE)
                * crate::metal_bridge::Q4_K_BLOCK_BYTES;
            if offset + q4_bytes_needed <= mmap.len() {
                let q4_slice = &mmap[offset..offset + q4_bytes_needed];
                let weights_f32 =
                    crate::metal_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                let input_rows = (input_activations.len() / cols).max(1);
                let result = crate::metal_bridge::accelerate_sgemm(
                    input_rows,
                    cols,
                    rows,
                    input_activations,
                    &weights_f32,
                );
                crate::telemetry::SIEVE_OPS_COUNT
                    .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                return result;
            }
        }

        // ── wgpu / WGSL fallback (all platforms — Vulkan on Linux/NVIDIA,
        //    Metal on macOS when mmap not loaded, D3D12 on Windows fallback) ──
        let input_bytes = bytemuck::cast_slice(input_activations);
        let input_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input"),
            size: input_bytes.len().max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&input_buf, 0, input_bytes);

        // Upload real weights from mmap when available, else use a zero buffer.
        let weights_size = (rows * cols * 4) as wgpu::BufferAddress;
        let weights_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weights"),
            size: weights_size.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let end = (offset + rows * cols * 4).min(mmap.len());
            if end > offset {
                let f32_bytes = &mmap[offset..end];
                self.gpu_queue().write_buffer(&weights_buf, 0, f32_bytes);
            }
        }

        let output_size = (rows * 4).max(4) as wgpu::BufferAddress;
        let output_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Upload GemmGpuParams for fused_transformer.wgsl (binding 2).
        let gemm_params = GemmGpuParams {
            n_in: cols as u32,
            n_out: rows as u32,
            weight_ggml_type: if tensor.is_quantized_q4_k { 12 } else { 14 },
            weight_row_elems: cols as u32,
            weight_byte_len: (rows * cols * 4) as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let params_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TransformerParams"),
            size: std::mem::size_of::<GemmGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&params_buf, 0, bytemuck::bytes_of(&gemm_params));

        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weights_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buf.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((rows as u32 + 63) / 64, 1, 1);
        }

        let staging_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::FusedBlock);

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        

        receiver.await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT
            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
        result
    }
#[cfg(target_arch = "wasm32")]
    pub async fn dispatch_output_argmax_chunked_async(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        // Part 3l: per-chunk async GPU vocab GEMM + streaming CPU argmax (no WGSL argmax shader).
        self.dispatch_output_argmax_chunked_async_mc8_fused(
            index,
            hidden,
            emb_dim,
            chunk_logits,
            max_chunks,
            sieve_mask,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    /// Per-chunk `dispatch_gemm_raw_into_async` + streaming CPU argmax.
    ///
    /// There is no WGSL argmax reduction shader. The Endgame "fused" batched encoder
    /// (`encode_gemm_bufs` loop + single staging readback) produced garbled tokens because
    /// `queue.write_buffer` weight uploads race across chunks in one submit scope (pt3c
    /// analogue). Per-chunk submit/readback matches the proven sync path semantics.
    async fn dispatch_output_argmax_chunked_async_mc8_fused(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        let info = index.logits_projection_info()?;
        let (n_in, vocab_size) = Self::matmul_dims(info);
        if n_in == 0 || vocab_size == 0 || n_in > emb_dim || n_in > hidden.len() {
            return None;
        }
        if chunk_logits.len() < VOCAB_CHUNK_ROWS || !self.mc8_buffers_ready() {
            return None;
        }
        let mmap = self.gguf_mmap.as_deref()?;
        let full_chunks = vocab_size.div_ceil(VOCAB_CHUNK_ROWS);
        let n_chunks = if max_chunks == 0 {
            full_chunks
        } else {
            (max_chunks as usize).min(full_chunks)
        };
        let mut best_token_id = 0u32;
        let mut max_logit = f32::NEG_INFINITY;
        for chunk_idx in 0..n_chunks {
            let row_start = chunk_idx * VOCAB_CHUNK_ROWS;
            let chunk_rows = VOCAB_CHUNK_ROWS.min(vocab_size - row_start);
            // Phase 5.3: bind the resident output-projection sub-range (zero per-token upload)
            // when available; otherwise fall back to per-chunk fetch + write_buffer. The chunk
            // byte offset is 256-aligned because VOCAB_CHUNK_ROWS is a multiple of 256.
            let ok = if let Some(buf) = self.mc8_logits_resident_buf.as_ref() {
                let row_bytes = self.mc8_logits_row_bytes as u64;
                let weight = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: buf,
                    offset: row_start as u64 * row_bytes,
                    size: std::num::NonZeroU64::new(chunk_rows as u64 * row_bytes),
                });
                self.dispatch_gemm_resident_chunk_async(
                    info.ggml_type,
                    info.dims[0] as u32,
                    weight,
                    (chunk_rows as u64 * row_bytes) as u32,
                    &hidden[..n_in],
                    &mut chunk_logits[..chunk_rows],
                    n_in,
                    chunk_rows,
                )
                .await
            } else {
                let raw = crate::ggml_quants::fetch_tensor_row_range_bytes(
                    mmap,
                    index.tensor_data_start,
                    info,
                    row_start,
                    chunk_rows,
                )
                .ok()?;
                self.dispatch_gemm_raw_into_async(
                    info,
                    raw,
                    &hidden[..n_in],
                    &mut chunk_logits[..chunk_rows],
                    n_in,
                    chunk_rows,
                )
                .await
            };
            if !ok {
                return None;
            }
            update_streaming_argmax_sieved(
                &chunk_logits[..chunk_rows],
                chunk_rows,
                chunk_idx,
                sieve_mask,
                &mut best_token_id,
                &mut max_logit,
            );
            scrub_f32_volatile(&mut chunk_logits[..chunk_rows], chunk_rows);
        }
        if max_logit == f32::NEG_INFINITY {
            return None;
        }
        Some(StreamingArgmaxResult {
            best_token_id,
            max_logit,
        })
    }


#[cfg(target_arch = "wasm32")]
    pub async fn new_async() -> Self {
        Self::try_new().await.expect("Failed to initialize native GGUF engine")
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
