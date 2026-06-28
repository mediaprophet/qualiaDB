//! WASM MC8 engine — encode concern (split from mc8_wasm.rs; verbatim, no behaviour change).
use super::super::*;

impl QTensorEngine {
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        pipeline
            .encoder
            .copy_buffer_to_buffer(scratch, scratch_off, dst, dst_off, bytes);
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
        self.gpu_queue().write_buffer(
            self.attention_params_buf.as_ref().unwrap(),
            0,
            bytemuck::bytes_of(&params),
        );
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
                        resource: self
                            .attention_params_buf
                            .as_ref()
                            .unwrap()
                            .as_entire_binding(),
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
                        resource: self
                            .attention_mask_buf
                            .as_ref()
                            .unwrap()
                            .as_entire_binding(),
                    },
                ],
            });
        let (wg_x, wg_y) = if proj_kind == 0 && num_tokens_in_batch > 1 {
            (h.n_head, num_tokens_in_batch)
        } else {
            (n_workgroups.max(1), 1)
        };
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
        cpass.set_pipeline(&self.attention_pipeline);
        cpass.set_bind_group(0, &bind, &[]);
        cpass.dispatch_workgroups(wg_x, wg_y, 1);
        true
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        self.encode_elem(pipeline, ELEM_OP_ADD_RESIDUAL, dim, 1, base, delta, scratch);
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
                        resource: self
                            .attention_mask_buf
                            .as_ref()
                            .unwrap()
                            .as_entire_binding(),
                    },
                ],
            });
        let (wg_x, wg_y) = if proj_kind == 0 && num_tokens_in_batch > 1 {
            (h.n_head, num_tokens_in_batch)
        } else {
            (n_workgroups.max(1), 1)
        };
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
        cpass.set_pipeline(&self.attention_pipeline);
        cpass.set_bind_group(0, &bind, &[attn_dyn_offset]);
        cpass.dispatch_workgroups(wg_x, wg_y, 1);
        true
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
        let bind = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
                        resource: self
                            .attention_mask_buf
                            .as_ref()
                            .unwrap()
                            .as_entire_binding(),
                    },
                ],
            });
        let mut cpass = pipeline
            .encoder
            .begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });
        cpass.set_pipeline(&self.attention_pipeline);
        cpass.set_bind_group(0, &bind, &[attn_dyn_offset]);
        cpass.dispatch_workgroups(h.n_head, n_tokens, 1);
        true
    }
}
