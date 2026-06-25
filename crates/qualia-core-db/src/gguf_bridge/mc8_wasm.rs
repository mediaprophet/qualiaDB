//! WASM-only MC8 GPU engine: resident weight arena, fused-encoder prefill/decode, async
//! readback. Carved verbatim from gguf_bridge/mod.rs (structural; no behaviour change).
//! The cfg(wasm32) `mod mc8_wasm;` in mod.rs gates the whole file, so no per-item cfg needed.
use super::*;
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
    pub(crate) fn mc8_weight_binding(&self, role: Mc8WeightRole, layer: u32) -> wgpu::BindingResource<'_> {
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
    pub(crate) fn mc8_upload_resident_norms(&mut self, index: &crate::gguf_sharder::GgufTensorIndex) -> bool {
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

    /// Phase 4: boot from a `.q42` weight container. Validates integrity (CRC via `from_q42`), builds
    /// a synthetic `GgufTensorIndex` from the manifest, points the byte source at the `.q42` bytes
    /// (`tensor_data_start = 0`, absolute blob offsets), reserves the GEMM/KV arenas, and uploads the
    /// resident weights via the **standard** path — so the entire GGUF hot path runs unchanged
    /// (format-agnostic). NOTE: the `.q42` weight container does not yet carry the tokenizer, so a
    /// q42-only boot maps weights/params but cannot tokenize until a tokenizer section lands.
    pub(crate) fn adopt_resident_q42(&mut self, data: Arc<[u8]>) -> Result<(), String> {
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

    pub(crate) fn mc8_dynamic_uniform_binding(buf: &wgpu::Buffer) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: buf,
            offset: 0,
            size: std::num::NonZeroU64::new(MC8_UNIFORM_ALIGN as u64),
        })
    }

    pub(crate) fn mc8_gemm_params(
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

    pub(crate) fn mc8_upload_attn_param(&self, params: &AttentionGpuParams) -> u32 {
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

    pub(crate) fn mc8_elem_params(
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

    pub(crate) fn mc8_buf_slice(
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

    pub(crate) fn mc8_prefill_row_off(t: u32, row_stride_floats: usize) -> wgpu::BufferAddress {
        (t as usize * row_stride_floats * 4) as wgpu::BufferAddress
    }

    pub(crate) fn mc8_emb_off(t: u32, n_embd: usize) -> wgpu::BufferAddress {
        (t as usize * n_embd * 4) as wgpu::BufferAddress
    }

    pub(crate) fn encode_elem_offset(
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

    pub(crate) fn encode_gemm_bufs_offset(
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
    pub(crate) fn encode_fused_ffn_expansion(
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

    pub(crate) fn encode_residual_add_offset(
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

    pub(crate) fn encode_attention_pass_gpu_offset(
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

    pub(crate) fn encode_elem(
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
    pub(crate) fn encode_residual_add_gpu(
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

    pub(crate) fn encode_gemm_bufs(
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

    pub(crate) fn encode_attention_pass_gpu(
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
    pub(crate) fn mc8_prefill_row_stride(n_embd: usize, n_ffn_est: usize, gemm_max_out_dim: u32) -> usize {
        (n_embd + 2 * n_ffn_est + n_embd)
            .max(gemm_max_out_dim as usize * 2)
            .max(n_embd)
    }

    /// MC8 Part 3o/3t: batched Q-SDPA — caller pre-uploads `attn_dyn_offset` in shared attn arena.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn encode_attention_batched_q_prefill(
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

    pub(crate) fn upload_norm_weights(&self, mmap: &[u8], tensor_data_start: u64, info: &GgufTensorInfo, n: usize) -> bool {
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

    pub(crate) async fn pipeline_read_hidden(&self, emb_dim: usize, hidden: &mut [f32]) -> bool {
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

    pub(crate) async fn pipeline_read_batch(&self, batch_elems: usize, out: &mut [f32]) -> bool {
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

    pub(crate) async fn pipeline_read_gpu_bytes_at(
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

    pub(crate) async fn pipeline_read_kv_head(
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
