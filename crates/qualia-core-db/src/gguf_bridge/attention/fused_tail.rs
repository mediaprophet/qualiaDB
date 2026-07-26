//! Native fused attention tail.
//!
//! This is the Q-attention -> O-projection fast path: Q-SDPA writes into a GPU
//! buffer, `o_proj` consumes that buffer in the same command encoder, and only the
//! projected residual is read back. Keeping it isolated makes the queue/readback
//! troubleshooting surface much smaller than the general attention module.

use crate::gguf_bridge::*;

impl QTensorEngine {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn dispatch_attention_q_o_fused(
        &self,
        hidden: &[f32],
        n_embd: usize,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        q_info: &GgufTensorInfo,
        q_raw: &[u8],
        o_info: &GgufTensorInfo,
        o_raw: &[u8],
        out: &mut [f32],
        o_in: usize,
        o_out: usize,
    ) -> bool {
        if !crate::llm_bench::resident_weights_enabled()
            || !ggml_gpu_attention_shader_supported(q_info.ggml_type)
            || !ggml_gpu_gemm_supported(o_info.ggml_type)
            || hidden.len() < n_embd
            || o_in > self.gemm_max_input_floats
            || o_out > out.len()
            || o_out > self.gemm_max_out_dim as usize
            || q_raw.len() > self.max_tensor_bytes
            || o_raw.len() > self.max_tensor_bytes
        {
            return false;
        }

        let input_buf = match self.gemm_input_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let attn_out_buf = match self.gemm_output_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let o_out_buf = match self.gemm_aux_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let staging = match self.gemm_output_staging.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let attn_params_buf = match self.attention_params_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let mask_buf = match self.attention_mask_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let gemm_params_buf = match self.gemm_params_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let kv_buf = match self.kv_cache_gpu.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let q_weight = match self.resident_weight_buffer(q_raw.as_ptr() as u64, q_raw) {
            Some(b) => b,
            None => return false,
        };
        let o_weight = match self.resident_weight_buffer(o_raw.as_ptr() as u64, o_raw) {
            Some(b) => b,
            None => return false,
        };

        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(layout, token_idx, 0);
        let q_params = Self::attention_gpu_params(
            h,
            layout,
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
        let o_params = GemmGpuParams {
            n_in: o_in as u32,
            n_out: o_out as u32,
            weight_ggml_type: o_info.ggml_type,
            weight_row_elems: o_info.dims[0] as u32,
            weight_byte_len: o_raw.len() as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };

        self.gpu_queue()
            .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..n_embd]));
        self.gpu_queue()
            .write_buffer(attn_params_buf, 0, bytemuck::bytes_of(&q_params));
        self.gpu_queue()
            .write_buffer(mask_buf, 0, bytemuck::cast_slice(&mask_words));
        self.gpu_queue()
            .write_buffer(gemm_params_buf, 0, bytemuck::bytes_of(&o_params));

        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset =
            (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding {
            buffer: kv_buf,
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };

        let attn_layout = self.attention_bind_layout.clone();
        let attn_bg = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FusedTailAttentionBG"),
                layout: &attn_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: q_weight.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: attn_params_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Buffer(kv_binding),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: attn_out_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: mask_buf.as_entire_binding(),
                    },
                ],
            });

        let use_coop = crate::llm_bench::coop_gemv_enabled();
        let gemm_pipeline: &wgpu::ComputePipeline = if use_coop {
            &self.coop_gemv_pipeline
        } else {
            &self.pipeline
        };
        let gemm_layout = self.native_gemm_bind_layout(use_coop).clone();
        let gemm_bg = if use_coop {
            self.gpu_device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("FusedTailOProjBG"),
                    layout: &gemm_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: input_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: o_weight.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: gemm_params_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: o_out_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: input_buf.as_entire_binding(),
                        },
                    ],
                })
        } else {
            self.gpu_device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("FusedTailOProjBG"),
                    layout: &gemm_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: input_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: o_weight.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: gemm_params_buf.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: o_out_buf.as_entire_binding(),
                        },
                    ],
                })
        };

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FusedAttentionTailEncoder"),
            });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FusedTailQAttention"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.attention_pipeline);
            cpass.set_bind_group(0, &attn_bg, &[]);
            cpass.dispatch_workgroups(h.n_head.max(1), 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            attn_out_buf,
            0,
            input_buf,
            0,
            (o_in * 4) as wgpu::BufferAddress,
        );
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FusedTailOProj"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(gemm_pipeline);
            cpass.set_bind_group(0, &gemm_bg, &[]);
            if use_coop {
                cpass.dispatch_workgroups(o_out as u32, 1, 1);
            } else {
                cpass.dispatch_workgroups((o_out as u32 + 63) / 64, 1, 1);
            }
        }
        let out_bytes = (o_out * 4) as wgpu::BufferAddress;
        encoder.copy_buffer_to_buffer(o_out_buf, 0, staging, 0, out_bytes);
        self.gpu_queue().submit(Some(encoder.finish()));

        let slice = staging.slice(..out_bytes);
        let (tx, rx) = futures_channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        self.poll_wait();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
                let data = slice
                    .get_mapped_range()
                    .expect("wgpu buffer map_range failed");
                let floats: &[f32] = bytemuck::cast_slice(&data);
                out[..o_out].copy_from_slice(&floats[..o_out]);
                drop(data);
                staging.unmap();
                return true;
            }
        }
        let _ = staging.unmap();
        false
    }
}
