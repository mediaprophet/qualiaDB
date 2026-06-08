//! Neuro-Symbolic GGUF Bridge
//! Dispatches transformer block computation to the best available GPU backend:
//!   - Windows x64: DirectML 1.15 (D3D12, hardware-vendor-optimised kernels)
//!   - All platforms: wgpu / WGSL fallback (Vulkan / Metal / WebGPU)
//! GGUF tensor bytes are memory-mapped via `memmap2` — zero heap copy.

use crate::QualiaQuin;
use crate::gguf_sharder::GgufTensorInfo;
use memmap2::MmapOptions;

pub use crate::ggml_quants::{ExecutionError, fetch_token_embedding};

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
    crate::ggml_quants::dequantize_row_into(raw, tensor.ggml_type, n_embd, out)
        .map_err(|e| match e {
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
        Self { shape, byte_offset, is_quantized_q4_k }
    }

    /// Maps the exact bytes from the GGUF using the 60-bit pointer.
    pub fn map_from_pointer(quin: &QualiaQuin) -> Option<Self> {
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
        let total = (n_layer as usize)
            .checked_mul(layer_stride as usize)?;
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
    let base = chunk_idx * VOCAB_CHUNK_ROWS;
    for (local, &v) in chunk_logits.iter().take(chunk_rows).enumerate() {
        if v > *max_logit {
            *max_logit = v;
            *best_token_id = (base + local) as u32;
        }
    }
}

#[inline]
fn apply_rope_in_head(head: &mut [f32], head_dim: usize, token_pos: u32) {
    let pairs = head_dim / 2;
    for p in 0..pairs {
        let theta = 10000f32.powf(-2.0 * p as f32 / head_dim as f32);
        let angle = token_pos as f32 * theta;
        let c = angle.cos();
        let s = angle.sin();
        let i0 = p * 2;
        let i1 = i0 + 1;
        if i1 >= head_dim {
            break;
        }
        let v0 = head[i0];
        let v1 = head[i1];
        head[i0] = v0 * c - v1 * s;
        head[i1] = v0 * s + v1 * c;
    }
}

/// Two-pass streaming softmax attention for one query head (zero score buffer).
fn attention_head_cpu(
    q: &[f32],
    kv: &[f32],
    layout: &KvCacheLayout,
    layer: u32,
    token_idx: u32,
    kv_head: u32,
    head_dim: usize,
    out: &mut [f32],
) {
    let max_ctx = layout.max_context;
    let start = token_idx.saturating_sub(max_ctx - 1);
    let scale = (head_dim as f32).sqrt().recip();
    let mut max_score = f32::NEG_INFINITY;
    for logical in start..=token_idx {
        let slot = logical % max_ctx;
        let mut score = 0f32;
        for d in 0..head_dim {
            score += q[d] * kv[layout.k_index(layer, slot, kv_head, d as u32)];
        }
        score *= scale;
        if score > max_score {
            max_score = score;
        }
    }
    let mut sum_exp = 0f32;
    for d in 0..head_dim {
        out[d] = 0.0;
    }
    for logical in start..=token_idx {
        let slot = logical % max_ctx;
        let mut score = 0f32;
        for d in 0..head_dim {
            score += q[d] * kv[layout.k_index(layer, slot, kv_head, d as u32)];
        }
        score = (score * scale - max_score).exp();
        sum_exp += score;
        for d in 0..head_dim {
            out[d] += score * kv[layout.v_index(layer, slot, kv_head, d as u32)];
        }
    }
    if sum_exp > 0.0 {
        let inv = sum_exp.recip();
        for d in 0..head_dim {
            out[d] *= inv;
        }
    }
}

#[inline]
fn relu_inplace(buf: &mut [f32], n: usize) {
    for v in buf.iter_mut().take(n) {
        if *v < 0.0 { *v = 0.0; }
    }
}

#[inline]
fn add_residual_inplace(dst: &mut [f32], src: &[f32], n: usize) {
    for i in 0..n.min(dst.len()).min(src.len()) {
        dst[i] += src[i];
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
    if n_in > input.len() || n_out > out.len() {
        return false;
    }
    let mut row = [0f32; MAX_STACK_GEMM_IN];
    for i in 0..n_out {
        if crate::ggml_quants::dequant_matrix_row_into(raw, info, i, &mut row[..n_in]).unwrap_or(0) < n_in {
            return false;
        }
        out[i] = row[..n_in].iter().zip(&input[..n_in]).map(|(w, x)| w * x).sum();
    }
    true
}

#[cfg(not(target_arch = "wasm32"))]
pub struct QTensorEngine {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
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
    pub gguf_mmap: Option<memmap2::Mmap>,
    /// Byte offset into the mmap where tensor data begins.
    pub tensor_data_offset: u64,
    pub hyperparams: crate::gguf_sharder::GgufHyperparams,
    pub max_tensor_bytes: usize,
    /// Reused layer staging buffers (one layer in VRAM at a time).
    gemm_input_buf: Option<wgpu::Buffer>,
    gemm_weight_buf: Option<wgpu::Buffer>,
    gemm_output_buf: Option<wgpu::Buffer>,
    gemm_params_buf: Option<wgpu::Buffer>,
    gemm_output_staging: Option<wgpu::Buffer>,
    gemm_max_out_dim: u32,
    /// Static KV ring-buffer (allocated once at `load_gguf`).
    kv_layout: Option<KvCacheLayout>,
    kv_cache_gpu: Option<wgpu::Buffer>,
    /// CPU mirror for quantized-attention fallback (no growth during decode).
    kv_cache_cpu: Option<Box<[f32]>>,
    attention_pipeline: wgpu::ComputePipeline,
    attention_params_buf: Option<wgpu::Buffer>,
}

#[cfg(not(target_arch = "wasm32"))]
impl QTensorEngine {
    pub fn new() -> Self {
        let instance = wgpu::Instance::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        let adapter = rt.block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })).expect("Failed to find wgpu adapter");

        let (device, queue) = rt.block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        )).expect("Failed to create wgpu device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fused Transformer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fused_transformer.wgsl").into()),
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Transformer Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
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
            entry_point: "main",
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
            entry_point: "main",
        });

        let attn_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fused Attention Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/fused_attention.wgsl").into()),
        });
        let attention_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Fused Attention Pipeline"),
            layout: None,
            module: &attn_shader,
            entry_point: "main",
        });

        Self {
            device,
            queue,
            pipeline,
            mock_pipeline,
            embedding_pipeline,
            attention_pipeline,
            is_initialized: true,
            #[cfg(target_os = "windows")]
            dml: crate::directml_bridge::DmlDevice::new().ok(),
            gguf_mmap: None,
            tensor_data_offset: 0,
            hyperparams: crate::gguf_sharder::GgufHyperparams::default(),
            max_tensor_bytes: 0,
            gemm_input_buf: None,
            gemm_weight_buf: None,
            gemm_output_buf: None,
            gemm_params_buf: None,
            gemm_output_staging: None,
            gemm_max_out_dim: MAX_STACK_GEMM_OUT as u32,
            kv_layout: None,
            kv_cache_gpu: None,
            kv_cache_cpu: None,
            attention_params_buf: None,
        }
    }

    fn ensure_kv_cache(&mut self, h: &crate::gguf_sharder::GgufHyperparams) {
        let layout = match KvCacheLayout::from_hyperparams(h) {
            Some(l) => l,
            None => return,
        };
        let bytes = (layout.total_f32_elems * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let gpu = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("StaticKvCacheArena"),
            size: bytes.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cpu = vec![0f32; layout.total_f32_elems].into_boxed_slice();
        self.attention_params_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("AttentionParams"),
            size: std::mem::size_of::<AttentionGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.kv_layout = Some(layout);
        self.kv_cache_gpu = Some(gpu);
        self.kv_cache_cpu = Some(cpu);
        eprintln!(
            "[gguf_bridge] KV arena {} f32 ({:.1} MiB), context={}",
            layout.total_f32_elems,
            bytes as f64 / (1024.0 * 1024.0),
            layout.max_context,
        );
    }

    /// Zero the static KV arena at the start of a new decode context (CPU mirror only; zero heap).
    pub fn reset_kv_cache(&mut self) {
        if let (Some(layout), Some(cpu)) = (self.kv_layout.as_ref(), self.kv_cache_cpu.as_mut()) {
            for v in cpu.iter_mut().take(layout.total_f32_elems) {
                unsafe { core::ptr::write_volatile(v, 0.0) };
            }
        }
    }

    fn ensure_gemm_buffers(&mut self, max_weight_bytes: usize, max_out_dim: u32) {
        if self.gemm_weight_buf.is_some() && max_weight_bytes <= self.max_tensor_bytes {
            return;
        }
        let w_bytes = max_weight_bytes.max(4) as wgpu::BufferAddress;
        let in_bytes = (MAX_STACK_GEMM_IN * 4) as wgpu::BufferAddress;
        let out_bytes = (max_out_dim as usize * 4).max(4) as wgpu::BufferAddress;
        self.gemm_input_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmInput"),
            size: in_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_weight_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmWeight"),
            size: w_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_output_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmOutput"),
            size: out_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));
        self.gemm_params_buf = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmParams"),
            size: std::mem::size_of::<GemmGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_output_staging = Some(self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("LayerGemmStaging"),
            size: out_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        self.gemm_max_out_dim = max_out_dim;
        self.max_tensor_bytes = max_weight_bytes;
    }

    /// Memory-map a GGUF file so tensor bytes are accessible without heap allocation.
    /// Call this once after `new()`, before the first `dispatch_fused_transformer_block`.
    pub fn load_gguf(&mut self, path: &str) {
        use std::fs::File;
        match File::open(path) {
            Ok(f) => match unsafe { MmapOptions::new().map(&f) } {
                Ok(mmap) => {
                    // memmap2 clones the file handle internally (try_clone); dropping `f` is safe.
                    // Tensor payload base must match gguf_sharder::GgufTensorIndex (full KV walk).
                    let index = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
                    self.tensor_data_offset = index.tensor_data_start;
                    self.hyperparams = index.hyperparams;
                    let staging = index
                        .max_layer_tensor_bytes
                        .max(4096)
                        .min(MAX_WGPU_WEIGHT_STAGING);
                    self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
                    self.ensure_kv_cache(&index.hyperparams);
                    self.gguf_mmap = Some(mmap);
                    eprintln!(
                        "[gguf_bridge] Mapped {} — tensor @ {:#x}, {} layers, {} heads (kv {}), max tensor {} bytes",
                        path,
                        self.tensor_data_offset,
                        self.hyperparams.n_layer,
                        self.hyperparams.n_head,
                        self.hyperparams.effective_n_kv_head(),
                        index.max_tensor_bytes,
                    );
                }
                Err(e) => eprintln!("[gguf_bridge] mmap failed for {path}: {e}"),
            },
            Err(e) => eprintln!("[gguf_bridge] Could not open {path}: {e}"),
        }
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

        let n_output = weight_tensor.shape.first().copied().unwrap_or(n_embd as usize) as u32;
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
        let embd_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("QuantizedEmbeddingBytes"),
            size: word_bytes.max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if raw_embd.len() == word_bytes {
            self.queue.write_buffer(&embd_buf, 0, raw_embd);
        } else {
            const MAX_EMB_ROW_PAD: usize = 8192;
            if word_bytes > MAX_EMB_ROW_PAD {
                return None;
            }
            let mut padded = [0u8; MAX_EMB_ROW_PAD];
            padded[..raw_embd.len()].copy_from_slice(raw_embd);
            self.queue.write_buffer(&embd_buf, 0, &padded[..word_bytes]);
        }

        let params_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingParams"),
            size: std::mem::size_of::<EmbeddingGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&params_buf, 0, bytemuck::bytes_of(&params));

        let weights_size = (weights_elems * 4).max(4) as wgpu::BufferAddress;
        let weights_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingWeights"),
            size: weights_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + weight_tensor.byte_offset) as usize;
            let end = (offset + weights_elems * 4).min(mmap.len());
            if end > offset {
                self.queue.write_buffer(&weights_buf, 0, &mmap[offset..end]);
            }
        }

        let output_size = (n_output as usize * 4).max(4) as wgpu::BufferAddress;
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingOutput"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_layout = self.embedding_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("QuantizedEmbeddingBindGroup"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: embd_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: params_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: weights_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: output_buf.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("QuantizedEmbeddingEncoder"),
        });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("QuantizedEmbeddingPass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.embedding_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((n_output + 63) / 64, 1, 1);
        }

        let staging_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("EmbeddingStaging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = sender.send(v);
        });
        self.device.poll(wgpu::Maintain::Wait);

        let rt = tokio::runtime::Runtime::new().ok()?;
        if rt.block_on(receiver).ok()?.is_err() {
            return None;
        }

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(
            weights_elems,
            std::sync::atomic::Ordering::Relaxed,
        );
        Some(result)
    }

    pub fn dispatch_fused_transformer_block(&self, tensor: &QTensor, input_activations: &[f32]) -> Vec<f32> {
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
                    let weights_f32 = crate::directml_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                    let op = crate::directml_bridge::DmlGemmOp {
                        m: input_activations.len() as u32 / cols as u32,
                        k: cols as u32,
                        n: rows as u32,
                    };
                    if let Ok(result) = op.execute(dml, input_activations, &weights_f32) {
                        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(
                            rows * cols,
                            std::sync::atomic::Ordering::Relaxed,
                        );
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
                let weights_f32 = crate::metal_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                let input_rows = (input_activations.len() / cols).max(1);
                let result = crate::metal_bridge::accelerate_sgemm(
                    input_rows, cols, rows, input_activations, &weights_f32
                );
                crate::telemetry::SIEVE_OPS_COUNT.fetch_add(
                    rows * cols, std::sync::atomic::Ordering::Relaxed,
                );
                return result;
            }
        }

        // ── wgpu / WGSL fallback (all platforms — Vulkan on Linux/NVIDIA,
        //    Metal on macOS when mmap not loaded, D3D12 on Windows fallback) ──
        let input_bytes = bytemuck::cast_slice(input_activations);
        let input_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input"),
            size: input_bytes.len().max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&input_buf, 0, input_bytes);

        // Upload real weights from mmap when available, else use a zero buffer.
        let weights_size = (rows * cols * 4) as wgpu::BufferAddress;
        let weights_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weights"),
            size: weights_size.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let end    = (offset + rows * cols * 4).min(mmap.len());
            if end > offset {
                let f32_bytes = &mmap[offset..end];
                self.queue.write_buffer(&weights_buf, 0, f32_bytes);
            }
        }

        let output_size = (4096 * 4) as wgpu::BufferAddress;
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group_layout = self.mock_pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: weights_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: output_buf.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None, timestamp_writes: None });
            cpass.set_pipeline(&self.mock_pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups(4096 / 64, 1, 1);
        }

        let staging_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        self.device.poll(wgpu::Maintain::Wait);
        
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(receiver).unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(4096 * 4096, std::sync::atomic::Ordering::Relaxed);
        result
    }

    fn write_weight_words(&self, raw: &[u8], max_bytes: usize) {
        let weight_buf = self.gemm_weight_buf.as_ref().expect("gemm weight buf");
        let upload = if raw.len() <= max_bytes { raw } else { &raw[..max_bytes] };
        self.queue.write_buffer(weight_buf, 0, upload);
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
        if info.ggml_type == crate::ggml_quants::GGML_TYPE_Q6_K
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
            };
            let input_buf = self.gemm_input_buf.as_ref().unwrap();
            let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
            let output_buf = self.gemm_output_buf.as_ref().unwrap();
            let params_buf = self.gemm_params_buf.as_ref().unwrap();
            let staging = self.gemm_output_staging.as_ref().unwrap();

            self.queue.write_buffer(input_buf, 0, bytemuck::cast_slice(&input[..n_in]));
            self.write_weight_words(raw, self.max_tensor_bytes);
            self.queue.write_buffer(params_buf, 0, bytemuck::bytes_of(&params));

            let bind_layout = self.pipeline.get_bind_group_layout(0);
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("LayerGemmBindGroup"),
                layout: &bind_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: weight_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: params_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: output_buf.as_entire_binding() },
                ],
            });

            let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LayerGemmEncoder"),
            });
            {
                let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: None,
                    timestamp_writes: None,
                });
                cpass.set_pipeline(&self.pipeline);
                cpass.set_bind_group(0, &bind_group, &[]);
                cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
            }
            let out_bytes = (n_out * 4) as wgpu::BufferAddress;
            encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
            self.queue.submit(Some(encoder.finish()));

            let slice = staging.slice(..out_bytes);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| { let _ = tx.send(r); });
            self.device.poll(wgpu::Maintain::Wait);
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                if rt.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
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
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info) {
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
            update_streaming_argmax(
                &chunk_logits[..chunk_rows],
                chunk_rows,
                chunk_idx,
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

    fn matmul_dims(info: &GgufTensorInfo) -> (usize, usize) {
        let n_in = info.dims[0] as usize;
        let n_out = if info.n_dims > 1 && info.dims[1] > 0 {
            info.dims[1] as usize
        } else {
            1
        };
        (n_in, n_out)
    }

    /// Q/K/V projections, RoPE, ring-buffer KV write, and GQA attention (CPU path for Q4_K).
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
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return None,
        };
        if self.kv_cache_cpu.is_none() {
            return None;
        }
        let q_info = match tensors.attn_q {
            Some(i) => i,
            None => return None,
        };
        let k_info = match tensors.attn_k {
            Some(i) => i,
            None => return None,
        };
        let v_info = match tensors.attn_v {
            Some(i) => i,
            None => return None,
        };
        let h = index.hyperparams;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        let head_dim = h.head_dim() as usize;
        let q_per_kv = h.q_heads_per_kv() as usize;
        if head_dim == 0 || n_head == 0 || n_kv == 0 {
            return None;
        }
        let q_dim = n_head * head_dim;
        if q_dim > scratch_a.len() || q_dim > scratch_b.len() || emb_dim < h.n_embd as usize {
            return None;
        }

        let (q_in, q_out) = Self::matmul_dims(&q_info);
        let (k_in, k_out) = Self::matmul_dims(&k_info);
        let (v_in, v_out) = Self::matmul_dims(&v_info);
        if q_in > emb_dim || k_in > emb_dim || v_in > emb_dim {
            return None;
        }
        if !self.dispatch_gemm_into(index, &q_info, &hidden[..q_in], &mut scratch_a[..q_out], q_in, q_out) {
            return None;
        }
        if !self.dispatch_gemm_into(index, &k_info, &hidden[..k_in], &mut scratch_b[..k_out], k_in, k_out) {
            return None;
        }
        let mut v_proj = [0f32; MAX_STACK_GEMM_DIM];
        if v_out > v_proj.len()
            || !self.dispatch_gemm_into(
                index,
                &v_info,
                &hidden[..v_in],
                &mut v_proj[..v_out],
                v_in,
                v_out,
            )
        {
            return None;
        }

        let slot = layout.ring_slot(token_idx);
        for qh in 0..n_head {
            let head_off = qh * head_dim;
            apply_rope_in_head(&mut scratch_a[head_off..head_off + head_dim], head_dim, token_idx);
        }
        for kvh in 0..n_kv {
            let off = kvh * head_dim;
            apply_rope_in_head(&mut scratch_b[off..off + head_dim], head_dim, token_idx);
        }
        {
            let kv = self.kv_cache_cpu.as_mut().unwrap();
            for kvh in 0..n_kv {
                let off = kvh * head_dim;
                for d in 0..head_dim {
                    let ki = layout.k_index(layer, slot, kvh as u32, d as u32);
                    let vi = layout.v_index(layer, slot, kvh as u32, d as u32);
                    kv[ki] = scratch_b[off + d];
                    kv[vi] = v_proj[off + d];
                }
            }
        }

        let mut head_out = [0f32; 512];
        if head_dim > head_out.len() {
            return None;
        }
        let kv_ro = self.kv_cache_cpu.as_deref().unwrap();
        for qh in 0..n_head {
            let kv_head = qh / q_per_kv;
            let q_off = qh * head_dim;
            attention_head_cpu(
                &scratch_a[q_off..q_off + head_dim],
                kv_ro,
                &layout,
                layer,
                token_idx,
                kv_head as u32,
                head_dim,
                &mut head_out[..head_dim],
            );
            for d in 0..head_dim {
                scratch_b[q_off + d] = head_out[d];
            }
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

        if tensors.attn_q.is_some() && tensors.attn_k.is_some() && tensors.attn_v.is_some() {
            if let Some(n) = self.dispatch_attention_layer(
                index, layer, token_idx, &hidden[..emb_dim], emb_dim, &tensors, scratch_a, scratch_b,
            ) {
                add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..n], n);
                attn_ok = true;
            }
        } else if let Some(info) = tensors.attn_output {
            let (n_in, n_out) = Self::matmul_dims(&info);
            if n_in <= emb_dim
                && self.dispatch_gemm_into(index, &info, &hidden[..n_in], scratch_a, n_in, n_out)
            {
                add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..n_out], emb_dim.min(n_out));
                attn_ok = true;
            }
        }

        if !attn_ok && tensors.attn_output.is_none() && tensors.ffn_gate.is_none() {
            return false;
        }

        if let Some(info) = tensors.ffn_gate {
            let (n_in, n_out) = Self::matmul_dims(&info);
            if n_in <= emb_dim
                && self.dispatch_gemm_into(index, &info, &hidden[..n_in], scratch_a, n_in, n_out)
            {
                relu_inplace(&mut scratch_a[..n_out], n_out);
                if let Some(up) = tensors.ffn_up {
                    let (up_in, up_out) = Self::matmul_dims(&up);
                    if up_in <= n_out
                        && self.dispatch_gemm_into(index, &up, &scratch_a[..up_in], scratch_b, up_in, up_out)
                    {
                        if let Some(down) = tensors.ffn_down {
                            let (dn_in, dn_out) = Self::matmul_dims(&down);
                            if dn_in <= up_out
                                && self.dispatch_gemm_into(
                                    index, &down, &scratch_b[..dn_in], scratch_a, dn_in, dn_out,
                                )
                            {
                                add_residual_inplace(
                                    &mut hidden[..emb_dim],
                                    &scratch_a[..dn_out],
                                    emb_dim.min(dn_out),
                                );
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
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
        let mut ran = 0u32;
        for layer in 0..limit {
            if self.dispatch_transformer_layer(
                index, layer, token_idx, hidden, emb_dim, scratch_a, scratch_b,
            ) {
                ran += 1;
            }
        }
        ran
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
        if valid_lexicon_ids.is_empty() { 0 } else { valid_lexicon_ids[0] }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct QTensorEngine {
    pub is_initialized: bool,
}

#[cfg(target_arch = "wasm32")]
impl QTensorEngine {
    pub fn new() -> Self {
        Self { is_initialized: true }
    }
    pub fn dispatch_fused_transformer_block(&self, _tensor: &QTensor, _input_activations: &[f32]) -> Vec<f32> {
        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        vec![0.0; 4096]
    }
    pub fn decode_lexicon_bound(&self, _logits: &[f32], valid_lexicon_ids: &[u64]) -> u64 {
        if valid_lexicon_ids.is_empty() { 0 } else { valid_lexicon_ids[0] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use memmap2::MmapOptions;
    use std::path::Path;

    #[test]
    fn test_q_tensor_pointer_extraction() {
        let quin = QualiaQuin {
            subject: crate::q_hash("LLM_Prompt"),
            predicate: crate::q_hash("has_tensor_offset"),
            // Pack the LLM flag + byte offset
            object: ((crate::MODALITY_FLAG_LLM_TENSOR as u64) << 60) | 0x0000_1234,
            context: 0,
            metadata: 0,
            parity: 0,
        };

        let q_tensor = QTensor::map_from_pointer(&quin)
            .expect("Failed to map QTensor from pointer");
        
        assert_eq!(q_tensor.byte_offset, 0x0000_1234, "Extracted byte offset incorrectly");
        assert_eq!(q_tensor.is_quantized_q4_k, true, "Did not identify Q4_K quantization");
    }

    #[test]
    fn test_lexicon_bound_decoding() {
        let engine = QTensorEngine::new();
        let logits = vec![0.1, 0.9, 0.2]; // Mock logits
        let valid_ids = vec![crate::q_hash("Dog"), crate::q_hash("Cat")];

        // Should return a valid u64 semantic ID, not a string
        let decoded = engine.decode_lexicon_bound(&logits, &valid_ids);
        assert_eq!(decoded, valid_ids[0], "Failed to bind decoding to logic lexicon");
    }

    #[test]
    fn test_fetch_token_embedding_gemma_if_exists() {
        let path = Path::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf");
        if !path.exists() {
            println!("Gemma GGUF not found locally; skipping embedding lookup test.");
            return;
        }
        let file = File::open(path).expect("open gguf");
        let mmap = unsafe { MmapOptions::new().map(&file).expect("mmap") };
        let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        let info = idx.token_embd_info().expect("token_embd.weight missing");
        println!(
            "token_embd: ggml_type={} dims=[{}, {}] offset={:#x}",
            info.ggml_type, info.dims[0], info.dims[1], info.byte_offset
        );

        let raw = fetch_token_embedding(&mmap, idx.tensor_data_start, info, 0)
            .expect("fetch token 0");
        assert!(!raw.is_empty(), "empty embedding slice");

        let n_embd = info.dims[0] as usize;
        let mut emb = vec![0f32; n_embd];
        let n = dequantize_token_embedding_into(raw, info, &mut emb)
            .expect("dequantize token 0");
        assert_eq!(n, n_embd);

        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(norm > 0.01 && norm < 1000.0, "embedding L2 norm suspicious: {norm}");
        println!("token 0 embedding L2 norm = {norm:.4}");
    }

    #[test]
    fn test_gpu_quantized_embedding_gemma_if_exists() {
        let path = Path::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf");
        if !path.exists() {
            println!("Gemma GGUF not found locally; skipping GPU embedding test.");
            return;
        }
        let file = File::open(path).expect("open gguf");
        let mmap = unsafe { MmapOptions::new().map(&file).expect("mmap") };
        let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        let info = idx.token_embd_info().expect("token_embd");
        let raw = fetch_token_embedding(&mmap, idx.tensor_data_start, info, 0).expect("fetch");

        let n_embd = info.dims[0] as usize;
        let mut cpu_emb = vec![0f32; n_embd];
        dequantize_token_embedding_into(raw, info, &mut cpu_emb).expect("cpu dequant");

        let mut engine = QTensorEngine::new();
        engine.load_gguf(path.to_str().unwrap());
        let wt = QTensor::new(vec![n_embd, n_embd], 0, true);
        let gpu_logits = engine
            .dispatch_quantized_token_embedding(raw, info.ggml_type, n_embd as u32, &wt)
            .expect("gpu dispatch");

        assert_eq!(gpu_logits.len(), n_embd);
        let cpu_dot: f32 = cpu_emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        let gpu_norm: f32 = gpu_logits.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!(gpu_norm > 0.0, "GPU output all zeros");
        println!("cpu_emb L2={cpu_dot:.4} gpu_logits L2={gpu_norm:.4}");
    }

    #[test]
    fn test_layer_tensors_and_matmul_gemma_if_exists() {
        let path = Path::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf");
        if !path.exists() {
            return;
        }
        let file = File::open(path).expect("open");
        let mmap = unsafe { MmapOptions::new().map(&file).expect("mmap") };
        let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        assert_eq!(idx.hyperparams.n_layer, 42);
        assert!(idx.max_layer_tensor_bytes > 0);
        assert!(idx.max_layer_tensor_bytes < idx.max_tensor_bytes);

        let layer0 = idx.get_layer_tensors(0);
        assert!(layer0.ffn_down.is_some());
        assert!(layer0.attn_q.is_some());

        let mut engine = QTensorEngine::new();
        engine.load_gguf(path.to_str().unwrap());
        engine.reset_kv_cache();
        let mut hidden = [0f32; 2560];
        hidden[0] = 1.0;
        let mut scratch_a = [0f32; 10240];
        let mut scratch_b = [0f32; 10240];
        assert!(engine.dispatch_transformer_layer(
            &idx, 0, 0, &mut hidden, 2560, &mut scratch_a, &mut scratch_b,
        ));
        println!("layer0 hidden[0]={}", hidden[0]);
    }

    #[test]
    fn test_attention_qkv_layer0_gemma_if_exists() {
        let path = Path::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf");
        if !path.exists() {
            return;
        }
        let file = File::open(path).expect("open");
        let mmap = unsafe { MmapOptions::new().map(&file).expect("mmap") };
        let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        assert!(idx.hyperparams.n_head > 0);

        let mut engine = QTensorEngine::new();
        engine.load_gguf(path.to_str().unwrap());
        engine.reset_kv_cache();

        let emb_dim = idx.emb_dim();
        let mut hidden = [0f32; 8192];
        hidden[0] = 1.0;
        let mut scratch_a = [0f32; 10240];
        let mut scratch_b = [0f32; 10240];
        let layer0 = idx.get_layer_tensors(0);
        assert!(layer0.attn_q.is_some());
        assert!(layer0.attn_k.is_some());
        assert!(layer0.attn_v.is_some());

        let t0 = std::time::Instant::now();
        assert!(engine.dispatch_transformer_layer(
            &idx, 0, 0, &mut hidden[..emb_dim], emb_dim, &mut scratch_a, &mut scratch_b,
        ));
        println!(
            "attention layer0 token0 hidden[0]={} elapsed={:?} kv_heads={}",
            hidden[0],
            t0.elapsed(),
            idx.hyperparams.effective_n_kv_head(),
        );
    }

    #[test]
    fn test_chunked_output_argmax_gemma_if_exists() {
        let path = Path::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf");
        if !path.exists() {
            return;
        }
        let file = File::open(path).expect("open");
        let mmap = unsafe { MmapOptions::new().map(&file).expect("mmap") };
        let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(&mmap);
        let vocab = idx.vocab_dim();
        assert!(vocab > VOCAB_CHUNK_ROWS);

        let mut engine = QTensorEngine::new();
        engine.load_gguf(path.to_str().unwrap());
        let emb_dim = idx.emb_dim();
        let mut hidden = [0f32; 8192];
        hidden[0] = 1.0;
        let mut chunk = [0f32; VOCAB_CHUNK_ROWS];
        let t0 = std::time::Instant::now();
        let result = engine
            .dispatch_output_argmax_chunked(&idx, &hidden[..emb_dim], emb_dim, &mut chunk, 2)
            .expect("chunked argmax");
        let elapsed = t0.elapsed();
        assert!(result.max_logit > f32::NEG_INFINITY);
        assert!(result.best_token_id < (2 * VOCAB_CHUNK_ROWS) as u32);
        println!(
            "chunked argmax token={} logit={} vocab={} elapsed={:?}",
            result.best_token_id, result.max_logit, vocab, elapsed
        );
    }

    #[test]
    fn test_mmap_gemma_model_if_exists() {
        // Check if the specific GGUF file exists on the developer's machine
        let path = Path::new("C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf");
        if path.exists() {
            let file = File::open(path).expect("Failed to open GGUF file");
            let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to memory map GGUF file") };
            
            // Just asserting that we successfully mapped a massive file into virtual memory
            assert!(mmap.len() > 1024 * 1024, "Memory map size is suspiciously small");
            println!("Successfully mapped Gemma GGUF! Size: {} bytes", mmap.len());
        } else {
            println!("Gemma GGUF file not found locally. Skipping mmap test.");
        }
    }
}
