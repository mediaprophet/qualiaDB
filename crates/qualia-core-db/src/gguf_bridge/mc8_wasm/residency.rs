//! WASM MC8 engine — residency concern (split from mc8_wasm.rs; verbatim, no behaviour change).
use super::super::*;
use std::sync::Arc;

impl QTensorEngine {
    pub(crate) fn mc8_flush(&self, pipeline: &mut WasmGpuPipeline) {
        let finished = std::mem::replace(pipeline, WasmGpuPipeline::begin(self));
        self.gpu_queue().submit(Some(finished.finish()));
    }

    pub(crate) fn mc8_weight_role_buf(&self, role: Mc8WeightRole) -> &wgpu::Buffer {
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

    pub(crate) fn write_weight_role(&self, role: Mc8WeightRole, raw: &[u8], max_bytes: usize) {
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
    pub(crate) fn mc8_weight_binding(
        &self,
        role: Mc8WeightRole,
        layer: u32,
    ) -> wgpu::BindingResource<'_> {
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
    pub(crate) fn mc8_upload_all_resident_weights(
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
        // Drop the placeholder arena before allocating resident buffers to minimize peak GPU memory.
        self.mc8_weight_arena = None;
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
        self.mc8_bg_cache.lock().ok().map(|mut c| c.clear());
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
    pub(crate) fn mc8_upload_resident_logits(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
    ) -> bool {
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
        let raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, info)
        {
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
    pub(crate) fn mc8_upload_resident_norms(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
    ) -> bool {
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
    pub(crate) fn mc8_norm_source(
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

    /// Boot from a P64 weight container. Validates integrity (CRC via `from_p64`), builds
    /// a synthetic `GgufTensorIndex` from the manifest, points the byte source at the P64 bytes
    /// (`tensor_data_start = 0`, absolute blob offsets), reserves the GEMM/KV arenas, and uploads the
    /// resident weights via the **standard** path — so the entire GGUF hot path runs unchanged
    /// (format-agnostic). Full GGUF conversions carry the tokenizer in the embedded Q42T section.
    pub(crate) fn adopt_resident_p64(&mut self, data: Arc<[u8]>) -> Result<(), String> {
        let q = crate::p64_weight::P64TensorIndex::from_p64(&data)?;
        let index = q.to_gguf_index();
        let hp = index.hyperparams;
        if hp.n_layer == 0 || hp.n_embd == 0 {
            return Err("P64: missing hyperparameters in header".to_string());
        }
        self.hyperparams = hp;
        self.tensor_data_offset = 0; // P64 blob offsets are absolute
        let staging = index
            .max_layer_tensor_bytes
            .max(4096)
            .min(MAX_WGPU_WEIGHT_STAGING);
        self.ensure_gemm_buffers(staging, MAX_STACK_GEMM_OUT as u32);
        self.ensure_kv_cache(&hp);
        if self.kv_layout.is_none() || self.kv_cache_cpu.is_none() {
            return Err("P64: KV cache allocation failed".to_string());
        }
        // Byte source for fetch_tensor_bytes (tensor_data_start=0 + absolute blob offsets).
        self.gguf_mmap = Some(data.clone());
        self.p64_resident = Some(data);
        // Resident weight upload reuses the standard path through the synthetic index.
        if !self.mc8_upload_all_resident_weights(&index) {
            wlog("[P64] eager resident upload skipped — will retry lazily");
        }
        if !self.mc8_upload_resident_logits(&index) {
            wlog("[P64] resident logits projection skipped — per-token upload fallback");
        }
        if !self.mc8_upload_resident_norms(&index) {
            wlog("[P64] resident norm weights skipped — per-layer upload fallback");
        }
        wlog(&format!(
            "[P64] boot OK: {} tensors, {} layers (synthetic GGUF index; weights resident)",
            q.entries.len(),
            hp.n_layer
        ));
        Ok(())
    }

    pub(crate) fn mc8_buffers_ready(&self) -> bool {
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

    pub(crate) fn upload_norm_weights(
        &self,
        mmap: &[u8],
        tensor_data_start: u64,
        info: &GgufTensorInfo,
        n: usize,
    ) -> bool {
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
}
