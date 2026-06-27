//! Native resident-activation decode forward (0.0.21).
//!
//! The legacy native decode (`dispatch_transformer_layer`) keeps the residual stream `hidden` in
//! CPU memory and reads it back from the GPU **twice per layer** — once after attention, once after
//! the FFN — because the RMSNorm pre-norm is computed on the CPU and therefore needs `hidden`
//! resident in host memory between the two halves. Those two synchronous `map_async` + `poll_wait`
//! round-trips per layer (×32 layers) are the decode bottleneck, not the kernel math.
//!
//! This module keeps `hidden` resident in a single VRAM buffer (`resident_hidden_buf`) across the
//! whole stack. RMSNorm and the residual add run on the GPU (`wasm_elementwise.wgsl`), and the
//! attention / FFN compute is fed straight from `gemm_input_buf` (where the GPU RMSNorm wrote the
//! normed input) — so **no per-layer readback happens at all**. The residual stream is read back
//! exactly once, after the final layer, for the final norm + `lm_head`.
//!
//! Submits are still per-op (one command submission per GPU pass) — that is cheap and pipelined; the
//! cost we remove is the *readback* stall, not the submit. Folding the per-layer passes into a
//! single submission (the wasm super-arena's dynamic-offset uniform trick) is a later optimization
//! and is the one allowed follow-up: the default path remains the verified source of truth, this
//! opt-in path duplicates its encode logic until coherence + tok/s are proven, after which the two
//! are unified.
//!
//! Gated behind `QUALIA_LLM_RESIDENT_DECODE` (default OFF) — the legacy CPU-hidden path is untouched.

use crate::gguf_bridge::*;

impl QTensorEngine {
    /// Ensure the resident residual-stream buffer exists and is at least `emb_dim` floats.
    #[cfg(not(target_arch = "wasm32"))]
    fn ensure_resident_hidden_buf(&mut self, emb_dim: usize) -> bool {
        let need = (emb_dim * std::mem::size_of::<f32>()).max(4) as wgpu::BufferAddress;
        let ok = self
            .resident_hidden_buf
            .as_ref()
            .map(|b| b.size() >= need)
            .unwrap_or(false);
        if !ok {
            self.resident_hidden_buf = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("ResidentHidden"),
                size: need,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }));
        }
        self.resident_hidden_buf.is_some()
    }

    /// Encode + submit one elementwise op (`wasm_elementwise.wgsl`) over the native auto-derived
    /// bind layout. No readback. `a`/`b` are read-only, `out` is read-write (must not alias `a`/`b`).
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_elem(
        &self,
        pipeline: &wgpu::ComputePipeline,
        op: u32,
        n: u32,
        a: &wgpu::Buffer,
        b: &wgpu::Buffer,
        out: &wgpu::Buffer,
    ) -> bool {
        let params_buf = match self.elem_params_buf.as_ref() {
            Some(p) => p,
            None => return false,
        };
        let params = ElemGpuParams {
            n,
            batch: 1,
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
        self.gpu_queue()
            .write_buffer(params_buf, 0, bytemuck::bytes_of(&params));
        let ep_sz = std::num::NonZeroU64::new(std::mem::size_of::<ElemGpuParams>() as u64);
        let layout = pipeline.get_bind_group_layout(0);
        let bind = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ResidentElemBG"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: a.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: b.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: out.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: params_buf,
                        offset: 0,
                        size: ep_sz,
                    }),
                },
            ],
        });
        let (wg_x, wg_y) = match op {
            ELEM_OP_RMS_NORM => (1u32, 1u32),
            _ => ((n + 63) / 64, 1u32),
        };
        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ResidentElemEnc") });
        {
            let mut cp = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ResidentElem"),
                timestamp_writes: None,
            });
            cp.set_pipeline(pipeline);
            cp.set_bind_group(0, &bind, &[]);
            cp.dispatch_workgroups(wg_x, wg_y, 1);
        }
        self.gpu_queue().submit(Some(encoder.finish()));
        true
    }

    /// GPU RMSNorm: `out = rmsnorm(src) * norm_weight`. `norm_weight` must already be uploaded into
    /// `norm_weight_buf` by the caller (`upload_norm_weights`). `out` must not alias `src`.
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_rms_norm(&self, src: &wgpu::Buffer, out: &wgpu::Buffer, n: usize) -> bool {
        let norm_w = match self.norm_weight_buf.as_ref() {
            Some(b) => b.clone(),
            None => return false,
        };
        let pipe = self.elem_rms_norm_pipeline.clone();
        self.rd_elem(&pipe, ELEM_OP_RMS_NORM, n as u32, src, &norm_w, out)
    }

    /// GPU residual add into the resident stream: `base = base + delta`. The shader forbids aliasing
    /// a read binding and the read-write binding, so we compute into `scratch` then copy back.
    /// `scratch` must not alias `base` or `delta`.
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_residual_inplace(
        &self,
        base: &wgpu::Buffer,
        delta: &wgpu::Buffer,
        scratch: &wgpu::Buffer,
        dim: usize,
    ) -> bool {
        let pipe = self.elem_add_residual_pipeline.clone();
        if !self.rd_elem(&pipe, ELEM_OP_ADD_RESIDUAL, dim as u32, base, delta, scratch) {
            return false;
        }
        let bytes = (dim * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let mut enc = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ResidentResidualCopy") });
        enc.copy_buffer_to_buffer(scratch, 0, base, 0, bytes);
        self.gpu_queue().submit(Some(enc.finish()));
        true
    }

    /// Resident K/V preprojection: same passes as [`Self::dispatch_attention_kv_preproject_fused`]
    /// but the normed input is **already** in `gemm_input_buf` (the GPU RMSNorm put it there) — no
    /// CPU upload — and nothing is read back (K/V are written to the KV cache on the GPU).
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_attention_kv_preproject(
        &self,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        k_info: &GgufTensorInfo,
        k_raw: &[u8],
        v_info: &GgufTensorInfo,
        v_raw: &[u8],
        kv_dim: usize,
    ) -> bool {
        const PARAM_SLOT: wgpu::BufferAddress = 256;
        if !crate::llm_bench::resident_weights_enabled()
            || !crate::llm_bench::coop_gemv_enabled()
            || kv_dim == 0
            || kv_dim > self.gemm_max_out_dim as usize
            || !ggml_gpu_gemm_supported(k_info.ggml_type)
            || !ggml_gpu_gemm_supported(v_info.ggml_type)
            || !ggml_gpu_attention_shader_supported(k_info.ggml_type)
            || !ggml_gpu_attention_shader_supported(v_info.ggml_type)
            || k_raw.len() > self.max_tensor_bytes
            || v_raw.len() > self.max_tensor_bytes
        {
            return false;
        }
        let (k_in, k_out) = Self::matmul_dims(k_info);
        let (v_in, v_out) = Self::matmul_dims(v_info);
        if v_in != k_in || k_out != kv_dim || v_out != kv_dim {
            return false;
        }
        let k_weight = match self.resident_weight_buffer(k_raw.as_ptr() as u64, k_raw) { Some(b) => b, None => return false };
        let v_weight = match self.resident_weight_buffer(v_raw.as_ptr() as u64, v_raw) { Some(b) => b, None => return false };
        let input_buf = match self.gemm_input_buf.as_ref() { Some(b) => b, None => return false };
        let proj_buf = match self.gemm_output_buf.as_ref() { Some(b) => b, None => return false };
        let attn_out_scratch = match self.gemm_aux_buf.as_ref() { Some(b) => b, None => return false };
        let kv_buf = match self.kv_cache_gpu.as_ref() { Some(b) => b, None => return false };
        let mask_buf = match self.attention_mask_buf.as_ref() { Some(b) => b, None => return false };
        let gemm_params_buf = match self.attention_kv_gemm_params.as_ref() { Some(b) => b, None => return false };
        let attn_params_buf = match self.attention_kv_params.as_ref() { Some(b) => b, None => return false };

        let k_gemm = GemmGpuParams { n_in: k_in as u32, n_out: kv_dim as u32, weight_ggml_type: k_info.ggml_type, weight_row_elems: k_info.dims[0] as u32, weight_byte_len: k_raw.len() as u32, n_batch: 1, in_row_stride: 0, out_row_stride: 0 };
        let v_gemm = GemmGpuParams { n_in: v_in as u32, n_out: kv_dim as u32, weight_ggml_type: v_info.ggml_type, weight_row_elems: v_info.dims[0] as u32, weight_byte_len: v_raw.len() as u32, n_batch: 1, in_row_stride: 0, out_row_stride: 0 };
        let mut k_attn = Self::attention_gpu_params(h, layout, layer, token_idx, k_info, k_raw.len(), 1, 1, token_idx, 0, 0, 0);
        k_attn.proj_row_stride = kv_dim as u32;
        let mut v_attn = Self::attention_gpu_params(h, layout, layer, token_idx, v_info, v_raw.len(), 2, 1, token_idx, 0, 0, 0);
        v_attn.proj_row_stride = kv_dim as u32;

        // NOTE: input already resident in `input_buf` — the legacy path's `write_buffer(input_buf, hidden)` is intentionally omitted.
        self.gpu_queue().write_buffer(gemm_params_buf, 0, bytemuck::bytes_of(&k_gemm));
        self.gpu_queue().write_buffer(gemm_params_buf, PARAM_SLOT, bytemuck::bytes_of(&v_gemm));
        self.gpu_queue().write_buffer(attn_params_buf, 0, bytemuck::bytes_of(&k_attn));
        self.gpu_queue().write_buffer(attn_params_buf, PARAM_SLOT, bytemuck::bytes_of(&v_attn));

        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset = (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let mk_kv_binding = || wgpu::BufferBinding { buffer: kv_buf, offset: layer_offset, size: std::num::NonZeroU64::new(layer_bytes.max(4)) };

        let gemm_layout = self.native_gemm_bind_layout(true).clone();
        let attn_layout = self.attention_bind_layout.clone();
        let gp_sz = std::num::NonZeroU64::new(std::mem::size_of::<GemmGpuParams>() as u64);
        let ap_sz = std::num::NonZeroU64::new(std::mem::size_of::<AttentionGpuParams>() as u64);
        let gemm_params_at = |slot: wgpu::BufferAddress| wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: gemm_params_buf, offset: slot, size: gp_sz });
        let attn_params_at = |slot: wgpu::BufferAddress| wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: attn_params_buf, offset: slot, size: ap_sz });
        let device = self.gpu_device();
        let k_gemm_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdKPreprojGemmBG"), layout: &gemm_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: k_weight.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: gemm_params_at(0) },
            wgpu::BindGroupEntry { binding: 3, resource: proj_buf.as_entire_binding() },
        ]});
        let v_gemm_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdVPreprojGemmBG"), layout: &gemm_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: v_weight.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: gemm_params_at(PARAM_SLOT) },
            wgpu::BindGroupEntry { binding: 3, resource: proj_buf.as_entire_binding() },
        ]});
        let k_attn_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdKPreprojWriteBG"), layout: &attn_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: proj_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: k_weight.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: attn_params_at(0) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Buffer(mk_kv_binding()) },
            wgpu::BindGroupEntry { binding: 4, resource: attn_out_scratch.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: mask_buf.as_entire_binding() },
        ]});
        let v_attn_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdVPreprojWriteBG"), layout: &attn_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: proj_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: v_weight.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: attn_params_at(PARAM_SLOT) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Buffer(mk_kv_binding()) },
            wgpu::BindGroupEntry { binding: 4, resource: attn_out_scratch.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: mask_buf.as_entire_binding() },
        ]});

        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("RdKvPreprojectEncoder") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("RdKPreprojGemm"), timestamp_writes: None });
            pass.set_pipeline(&self.coop_gemv_pipeline);
            pass.set_bind_group(0, &k_gemm_bg, &[]);
            pass.dispatch_workgroups(kv_dim as u32, 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("RdKPreprojWrite"), timestamp_writes: None });
            pass.set_pipeline(&self.attention_pipeline);
            pass.set_bind_group(0, &k_attn_bg, &[]);
            pass.dispatch_workgroups(h.effective_n_kv_head().max(1), 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("RdVPreprojGemm"), timestamp_writes: None });
            pass.set_pipeline(&self.coop_gemv_pipeline);
            pass.set_bind_group(0, &v_gemm_bg, &[]);
            pass.dispatch_workgroups(kv_dim as u32, 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("RdVPreprojWrite"), timestamp_writes: None });
            pass.set_pipeline(&self.attention_pipeline);
            pass.set_bind_group(0, &v_attn_bg, &[]);
            pass.dispatch_workgroups(h.effective_n_kv_head().max(1), 1, 1);
        }
        self.gpu_queue().submit(Some(encoder.finish()));
        true
    }

    /// Resident Q-attention + o-projection: same as [`Self::dispatch_attention_q_o_fused`] but the
    /// normed input is already in `gemm_input_buf` (no upload) and the o-projection result is left in
    /// `gemm_aux_buf` on the GPU (no readback) for the GPU residual add.
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_attention_q_o(
        &self,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        q_info: &GgufTensorInfo,
        q_raw: &[u8],
        o_info: &GgufTensorInfo,
        o_raw: &[u8],
        o_in: usize,
        o_out: usize,
    ) -> bool {
        if !crate::llm_bench::resident_weights_enabled()
            || !ggml_gpu_attention_shader_supported(q_info.ggml_type)
            || !ggml_gpu_gemm_supported(o_info.ggml_type)
            || o_in > self.gemm_max_input_floats
            || o_out > self.gemm_max_out_dim as usize
            || q_raw.len() > self.max_tensor_bytes
            || o_raw.len() > self.max_tensor_bytes
        {
            return false;
        }
        let input_buf = match self.gemm_input_buf.as_ref() { Some(b) => b, None => return false };
        let attn_out_buf = match self.gemm_output_buf.as_ref() { Some(b) => b, None => return false };
        let o_out_buf = match self.gemm_aux_buf.as_ref() { Some(b) => b, None => return false };
        let attn_params_buf = match self.attention_params_buf.as_ref() { Some(b) => b, None => return false };
        let mask_buf = match self.attention_mask_buf.as_ref() { Some(b) => b, None => return false };
        let gemm_params_buf = match self.gemm_params_buf.as_ref() { Some(b) => b, None => return false };
        let kv_buf = match self.kv_cache_gpu.as_ref() { Some(b) => b, None => return false };
        let q_weight = match self.resident_weight_buffer(q_raw.as_ptr() as u64, q_raw) { Some(b) => b, None => return false };
        let o_weight = match self.resident_weight_buffer(o_raw.as_ptr() as u64, o_raw) { Some(b) => b, None => return false };

        let (mask_words, mask_active, mask_word_count) = Self::attention_kv_mask_for_dispatch(layout, token_idx, 0);
        let q_params = Self::attention_gpu_params(h, layout, layer, token_idx, q_info, q_raw.len(), 0, 1, token_idx, mask_active, mask_word_count, 0);
        let o_params = GemmGpuParams { n_in: o_in as u32, n_out: o_out as u32, weight_ggml_type: o_info.ggml_type, weight_row_elems: o_info.dims[0] as u32, weight_byte_len: o_raw.len() as u32, n_batch: 1, in_row_stride: 0, out_row_stride: 0 };

        // NOTE: input already resident in `input_buf` — the legacy `write_buffer(input_buf, hidden)` is omitted.
        self.gpu_queue().write_buffer(attn_params_buf, 0, bytemuck::bytes_of(&q_params));
        self.gpu_queue().write_buffer(mask_buf, 0, bytemuck::cast_slice(&mask_words));
        self.gpu_queue().write_buffer(gemm_params_buf, 0, bytemuck::bytes_of(&o_params));

        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset = (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding { buffer: kv_buf, offset: layer_offset, size: std::num::NonZeroU64::new(layer_bytes.max(4)) };

        let attn_layout = self.attention_bind_layout.clone();
        let attn_bg = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdQAttentionBG"), layout: &attn_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: q_weight.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: attn_params_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Buffer(kv_binding) },
            wgpu::BindGroupEntry { binding: 4, resource: attn_out_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: mask_buf.as_entire_binding() },
        ]});

        let use_coop = crate::llm_bench::coop_gemv_enabled();
        let gemm_pipeline: &wgpu::ComputePipeline = if use_coop { &self.coop_gemv_pipeline } else { &self.pipeline };
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let gemm_bg = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdOProjBG"), layout: &gemm_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: o_weight.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: gemm_params_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: o_out_buf.as_entire_binding() },
        ]});

        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("RdAttentionQOEncoder") });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("RdQAttention"), timestamp_writes: None });
            cpass.set_pipeline(&self.attention_pipeline);
            cpass.set_bind_group(0, &attn_bg, &[]);
            cpass.dispatch_workgroups(h.n_head.max(1), 1, 1);
        }
        encoder.copy_buffer_to_buffer(attn_out_buf, 0, input_buf, 0, (o_in * 4) as wgpu::BufferAddress);
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("RdOProj"), timestamp_writes: None });
            cpass.set_pipeline(gemm_pipeline);
            cpass.set_bind_group(0, &gemm_bg, &[]);
            if use_coop {
                cpass.dispatch_workgroups(o_out as u32, 1, 1);
            } else {
                cpass.dispatch_workgroups((o_out as u32 + 63) / 64, 1, 1);
            }
        }
        // o-projection result now lives in `o_out_buf` (gemm_aux_buf) on the GPU — no readback.
        self.gpu_queue().submit(Some(encoder.finish()));
        true
    }

    /// Resident FFN: same passes as [`Self::dispatch_ffn_fused_resident`] but the normed FFN input is
    /// already in `gemm_input_buf` (no CPU pre-norm/upload) and the down-projection result is left in
    /// `gemm_input_buf` on the GPU (no readback) for the GPU residual add.
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_ffn(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
    ) -> bool {
        let mmap_arc = match self.gguf_mmap.clone() { Some(a) => a, None => return false };
        let mmap: &[u8] = &mmap_arc;
        let gate_info = match tensors.ffn_gate.as_ref() { Some(i) => i, None => return false };
        let up_info = match tensors.ffn_up.as_ref() { Some(i) => i, None => return false };
        let down_info = match tensors.ffn_down.as_ref() { Some(i) => i, None => return false };
        let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > emb_dim || up_in != gate_in || up_out != n_ffn || dn_in != n_ffn || n_ffn > MAX_STACK_GEMM_DIM || dn_out < emb_dim {
            return false;
        }
        if !(ggml_gpu_gemm_supported(gate_info.ggml_type) && ggml_gpu_gemm_supported(up_info.ggml_type) && ggml_gpu_gemm_supported(down_info.ggml_type)) {
            return false;
        }
        if n_ffn > self.gemm_max_out_dim as usize || dn_out > self.gemm_max_out_dim as usize || gate_in > self.gemm_max_input_floats as usize || n_ffn > self.gemm_max_input_floats as usize {
            return false;
        }
        let gate_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info) { Ok(s) => s, Err(_) => return false };
        let up_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info) { Ok(s) => s, Err(_) => return false };
        let down_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info) { Ok(s) => s, Err(_) => return false };
        if gate_raw.len() > self.max_tensor_bytes || up_raw.len() > self.max_tensor_bytes || down_raw.len() > self.max_tensor_bytes {
            return false;
        }
        let rg = match self.resident_weight_buffer(gate_raw.as_ptr() as u64, gate_raw) { Some(b) => b, None => return false };
        let ru = match self.resident_weight_buffer(up_raw.as_ptr() as u64, up_raw) { Some(b) => b, None => return false };
        let rd = match self.resident_weight_buffer(down_raw.as_ptr() as u64, down_raw) { Some(b) => b, None => return false };

        const SLOT: wgpu::BufferAddress = 256;
        let p_gate = GemmGpuParams { n_in: gate_in as u32, n_out: n_ffn as u32, weight_ggml_type: gate_info.ggml_type, weight_row_elems: gate_info.dims[0] as u32, weight_byte_len: gate_raw.len() as u32, n_batch: 1, in_row_stride: 0, out_row_stride: 0 };
        let p_up = GemmGpuParams { n_in: up_in as u32, n_out: n_ffn as u32, weight_ggml_type: up_info.ggml_type, weight_row_elems: up_info.dims[0] as u32, weight_byte_len: up_raw.len() as u32, n_batch: 1, in_row_stride: 0, out_row_stride: 0 };
        let p_down = GemmGpuParams { n_in: n_ffn as u32, n_out: dn_out as u32, weight_ggml_type: down_info.ggml_type, weight_row_elems: down_info.dims[0] as u32, weight_byte_len: down_raw.len() as u32, n_batch: 1, in_row_stride: 0, out_row_stride: 0 };
        let p_silu = ElemGpuParams { n: n_ffn as u32, batch: 1, op: ELEM_OP_SILU_MUL, eps: RMS_NORM_EPS, a_row_stride: 0, b_row_stride: 0, out_row_stride: 0, a_slot: 0, b_slot: 0, out_slot: 0, _pad: 0 };

        if self.ffn_fused_params.is_none() {
            self.ffn_fused_params = Some(self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
                label: Some("FfnFusedGemmParams"),
                size: SLOT * 3,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
        }
        let (params_buf, in_buf, g_buf, u_buf, s_buf, elem_params) = match (
            self.ffn_fused_params.as_ref(),
            self.gemm_input_buf.as_ref(),
            self.gemm_aux_buf.as_ref(),
            self.gemm_ffn_buf.as_ref(),
            self.gemm_output_buf.as_ref(),
            self.elem_params_buf.as_ref(),
        ) {
            (Some(p), Some(i), Some(g), Some(u), Some(s), Some(e)) => (p, i, g, u, s, e),
            _ => return false,
        };
        let d_buf = in_buf; // down output reuses the now-dead input buffer (gate/up read it first)
        let device = self.gpu_device();

        self.gpu_queue().write_buffer(params_buf, 0, bytemuck::bytes_of(&p_gate));
        self.gpu_queue().write_buffer(params_buf, SLOT, bytemuck::bytes_of(&p_up));
        self.gpu_queue().write_buffer(params_buf, SLOT * 2, bytemuck::bytes_of(&p_down));
        self.gpu_queue().write_buffer(elem_params, 0, bytemuck::bytes_of(&p_silu));
        // NOTE: FFN input already resident in `in_buf` (GPU RMSNorm) — legacy `write_buffer(in_buf, ffn_input)` omitted.

        let use_coop = crate::llm_bench::coop_gemv_enabled();
        let gemm_pipeline: &wgpu::ComputePipeline = if use_coop { &self.coop_gemv_pipeline } else { &self.pipeline };
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let elem_layout = self.elem_silu_mul_bind_layout.clone();
        let gp_sz = std::num::NonZeroU64::new(std::mem::size_of::<GemmGpuParams>() as u64);
        let ep_sz = std::num::NonZeroU64::new(std::mem::size_of::<ElemGpuParams>() as u64);
        let gemm_params_at = |slot: wgpu::BufferAddress| wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: params_buf, offset: slot, size: gp_sz });

        let gate_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdFfnGateBG"), layout: &gemm_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: in_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: rg.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: gemm_params_at(0) },
            wgpu::BindGroupEntry { binding: 3, resource: g_buf.as_entire_binding() },
        ]});
        let up_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdFfnUpBG"), layout: &gemm_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: in_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: ru.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: gemm_params_at(SLOT) },
            wgpu::BindGroupEntry { binding: 3, resource: u_buf.as_entire_binding() },
        ]});
        let silu_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdFfnSiluBG"), layout: &elem_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: g_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: u_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: s_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: elem_params, offset: 0, size: ep_sz }) },
        ]});
        let down_bg = device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("RdFfnDownBG"), layout: &gemm_layout, entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: s_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: rd.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: gemm_params_at(SLOT * 2) },
            wgpu::BindGroupEntry { binding: 3, resource: d_buf.as_entire_binding() },
        ]});

        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("RdFfnEncoder") });
        let gate_groups = if use_coop { n_ffn as u32 } else { (n_ffn as u32 + 63) / 64 };
        let down_groups = if use_coop { dn_out as u32 } else { (dn_out as u32 + 63) / 64 };
        for (label, pipe, bg, groups) in [
            ("RdFfnGate", gemm_pipeline, &gate_bg, gate_groups),
            ("RdFfnUp", gemm_pipeline, &up_bg, gate_groups),
            ("RdFfnSilu", &self.elem_silu_mul_pipeline, &silu_bg, (n_ffn as u32 + 63) / 64),
            ("RdFfnDown", gemm_pipeline, &down_bg, down_groups),
        ] {
            let mut cp = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(label), timestamp_writes: None });
            cp.set_pipeline(pipe);
            cp.set_bind_group(0, bg, &[]);
            cp.dispatch_workgroups(groups, 1, 1);
        }
        // down-projection result now lives in `d_buf` (gemm_input_buf) on the GPU — no readback.
        self.gpu_queue().submit(Some(encoder.finish()));
        true
    }

    /// Native resident-activation decode forward for a single token at absolute position `token_idx`.
    ///
    /// Keeps the residual stream in `resident_hidden_buf` across the whole stack; RMSNorm + residual
    /// run on the GPU; attention/FFN feed off `gemm_input_buf`. Reads back to `hidden` exactly once,
    /// after the last layer. Returns `Some(layers_run)` on success, or `None` on any ineligibility —
    /// in which case `hidden` (CPU) is **untouched**, so the caller falls back to the legacy path
    /// recomputing from the identical input.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn dispatch_transformer_forward_resident(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        token_idx: u32,
        max_layers: u32,
    ) -> Option<u32> {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || emb_dim == 0 || emb_dim > hidden.len() {
            return None;
        }
        if emb_dim != index.hyperparams.n_embd as usize {
            return None;
        }
        let limit = if max_layers == 0 { n_layer } else { max_layers.min(n_layer) };
        let layout = self.kv_layout?;
        if !self.ensure_resident_hidden_buf(emb_dim) {
            return None;
        }
        let mmap_arc = self.gguf_mmap.clone()?;
        let tds = index.tensor_data_start;
        let h = index.hyperparams;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        let head_dim = h.head_dim() as usize;
        if head_dim == 0 || n_head == 0 || n_kv == 0 {
            return None;
        }
        let q_dim = n_head * head_dim;
        let kv_dim = n_kv * head_dim;

        // Cloneable GPU handles (cheap Arc clones) so per-op calls don't fight the `&mut self` borrow.
        let resident = self.resident_hidden_buf.as_ref()?.clone();
        let in_buf = self.gemm_input_buf.as_ref()?.clone();
        let aux_buf = self.gemm_aux_buf.as_ref()?.clone();
        let scratch = self.gemm_output_buf.as_ref()?.clone();

        // Seed the residual stream from the CPU hidden (the only upload; never read back per layer).
        self.gpu_queue()
            .write_buffer(&resident, 0, bytemuck::cast_slice(&hidden[..emb_dim]));

        for layer in 0..limit {
            let tensors = index.get_layer_tensors(layer);
            let q_info = tensors.attn_q.as_ref()?;
            let k_info = tensors.attn_k.as_ref()?;
            let v_info = tensors.attn_v.as_ref()?;
            let o_info = tensors.attn_output.as_ref()?;

            // ── attention pre-norm (GPU): resident → in_buf ──
            match tensors.attn_norm.as_ref() {
                Some(norm) => {
                    if !self.rd_upload_norm_weights(&mmap_arc, tds, norm, emb_dim) {
                        return None;
                    }
                    if !self.rd_rms_norm(&resident, &in_buf, emb_dim) {
                        return None;
                    }
                }
                None => self.rd_copy(&resident, &in_buf, emb_dim),
            }

            // ── K/V projection (GPU, writes KV cache) ──
            let k_raw = crate::ggml_quants::fetch_tensor_bytes(&mmap_arc, tds, k_info).ok()?;
            let v_raw = crate::ggml_quants::fetch_tensor_bytes(&mmap_arc, tds, v_info).ok()?;
            if !self.rd_attention_kv_preproject(&layout, layer, token_idx, &h, k_info, k_raw, v_info, v_raw, kv_dim) {
                return None;
            }

            // ── Q-attention + o-projection (GPU); o_proj lands in aux_buf ──
            let q_raw = crate::ggml_quants::fetch_tensor_bytes(&mmap_arc, tds, q_info).ok()?;
            let o_raw = crate::ggml_quants::fetch_tensor_bytes(&mmap_arc, tds, o_info).ok()?;
            let (o_in, o_out) = Self::matmul_dims(o_info);
            if o_in > q_dim || o_out > emb_dim {
                return None;
            }
            if !self.rd_attention_q_o(&layout, layer, token_idx, &h, q_info, q_raw, o_info, o_raw, o_in, o_out) {
                return None;
            }

            // ── attention residual (GPU): resident += aux_buf ──
            if !self.rd_residual_inplace(&resident, &aux_buf, &scratch, emb_dim) {
                return None;
            }

            // ── FFN pre-norm (GPU): resident → in_buf ──
            match tensors.ffn_norm.as_ref() {
                Some(norm) => {
                    if !self.rd_upload_norm_weights(&mmap_arc, tds, norm, emb_dim) {
                        return None;
                    }
                    if !self.rd_rms_norm(&resident, &in_buf, emb_dim) {
                        return None;
                    }
                }
                None => self.rd_copy(&resident, &in_buf, emb_dim),
            }

            // ── FFN (GPU); down-projection lands in in_buf ──
            if !self.rd_ffn(index, emb_dim, &tensors) {
                return None;
            }

            // ── FFN residual (GPU): resident += in_buf ──
            if !self.rd_residual_inplace(&resident, &in_buf, &scratch, emb_dim) {
                return None;
            }
        }

        // Single readback of the residual stream → CPU `hidden` (after the last layer only).
        let staging = self.gemm_output_staging.as_ref()?.clone();
        let out_bytes = (emb_dim * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let mut enc = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ResidentReadbackEnc") });
        enc.copy_buffer_to_buffer(&resident, 0, &staging, 0, out_bytes);
        self.gpu_queue().submit(Some(enc.finish()));
        let slice = staging.slice(..out_bytes);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.poll_wait();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
                let data = slice.get_mapped_range();
                let floats: &[f32] = bytemuck::cast_slice(&data);
                hidden[..emb_dim].copy_from_slice(&floats[..emb_dim]);
                drop(data);
                staging.unmap();
                {
                    use std::sync::atomic::{AtomicBool, Ordering};
                    static RD_RAN: AtomicBool = AtomicBool::new(false);
                    if !RD_RAN.swap(true, Ordering::Relaxed) {
                        eprintln!(
                            "[resident-decode] ACTIVE — {} layers resident, 1 readback/token (was 2/layer)",
                            limit
                        );
                    }
                }
                return Some(limit);
            }
        }
        let _ = staging.unmap();
        None
    }

    /// Dequantize the norm row from the mmap into `norm_weight_buf` (native equivalent of the
    /// wasm-only `upload_norm_weights`). Returns false if the tensor is shorter than `n`.
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_upload_norm_weights(
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
        let buf = match self.norm_weight_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        self.gpu_queue()
            .write_buffer(buf, 0, bytemuck::cast_slice(&norm_w[..n]));
        true
    }

    /// GPU buffer→buffer copy of `n` floats (used when a layer has no norm weight).
    #[cfg(not(target_arch = "wasm32"))]
    fn rd_copy(&self, src: &wgpu::Buffer, dst: &wgpu::Buffer, n: usize) {
        let bytes = (n * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let mut enc = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("RdCopy") });
        enc.copy_buffer_to_buffer(src, 0, dst, 0, bytes);
        self.gpu_queue().submit(Some(enc.finish()));
    }
}
