//! Native K/V preprojection fast path.
//!
//! The legacy decode path asks `fused_attention.wgsl` to do K and V matmul inside
//! the attention shader. That shader is optimized around online softmax, not
//! projection throughput. This path runs K/V projection through the cooperative
//! GEMV pipeline, then reuses the attention shader only for RoPE and KV-cache
//! writes. K and V share one command submission and do not read back to the CPU.

use crate::gguf_bridge::*;

const PARAM_SLOT: wgpu::BufferAddress = 256;

impl QTensorEngine {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn dispatch_attention_kv_preproject_fused(
        &self,
        hidden: &[f32],
        n_embd: usize,
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
        if !crate::llm_bench::resident_weights_enabled()
            || !crate::llm_bench::coop_gemv_enabled()
            || kv_dim == 0
            || kv_dim > self.gemm_max_out_dim as usize
            || n_embd > hidden.len()
            || n_embd > self.gemm_max_input_floats
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
        if k_in > n_embd || v_in != k_in || k_out != kv_dim || v_out != kv_dim {
            return false;
        }

        let k_weight = match self.resident_weight_buffer(k_raw.as_ptr() as u64, k_raw) {
            Some(b) => b,
            None => return false,
        };
        let v_weight = match self.resident_weight_buffer(v_raw.as_ptr() as u64, v_raw) {
            Some(b) => b,
            None => return false,
        };

        let input_buf = match self.gemm_input_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let proj_buf = match self.gemm_output_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let attn_out_scratch = match self.gemm_aux_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let kv_buf = match self.kv_cache_gpu.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let mask_buf = match self.attention_mask_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let gemm_params_buf = match self.attention_kv_gemm_params.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let attn_params_buf = match self.attention_kv_params.as_ref() {
            Some(b) => b,
            None => return false,
        };

        let k_gemm = GemmGpuParams {
            n_in: k_in as u32,
            n_out: kv_dim as u32,
            weight_ggml_type: k_info.ggml_type,
            weight_row_elems: k_info.dims[0] as u32,
            weight_byte_len: k_raw.len() as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let v_gemm = GemmGpuParams {
            n_in: v_in as u32,
            n_out: kv_dim as u32,
            weight_ggml_type: v_info.ggml_type,
            weight_row_elems: v_info.dims[0] as u32,
            weight_byte_len: v_raw.len() as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let mut k_attn = Self::attention_gpu_params(
            h,
            layout,
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
        k_attn.proj_row_stride = kv_dim as u32;
        let mut v_attn = Self::attention_gpu_params(
            h,
            layout,
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
        v_attn.proj_row_stride = kv_dim as u32;

        self.gpu_queue()
            .write_buffer(input_buf, 0, bytemuck::cast_slice(&hidden[..k_in]));
        self.gpu_queue()
            .write_buffer(gemm_params_buf, 0, bytemuck::bytes_of(&k_gemm));
        self.gpu_queue()
            .write_buffer(gemm_params_buf, PARAM_SLOT, bytemuck::bytes_of(&v_gemm));
        self.gpu_queue()
            .write_buffer(attn_params_buf, 0, bytemuck::bytes_of(&k_attn));
        self.gpu_queue()
            .write_buffer(attn_params_buf, PARAM_SLOT, bytemuck::bytes_of(&v_attn));

        let layer_f32s = layout.layer_stride as usize;
        let layer_bytes = (layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let layer_offset =
            (layer as usize * layer_f32s * std::mem::size_of::<f32>()) as wgpu::BufferAddress;
        let kv_binding = wgpu::BufferBinding {
            buffer: kv_buf,
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };

        let gemm_layout = self.native_gemm_bind_layout(true).clone();
        let attn_layout = self.attention_bind_layout.clone();
        let gp_sz = std::num::NonZeroU64::new(std::mem::size_of::<GemmGpuParams>() as u64);
        let ap_sz = std::num::NonZeroU64::new(std::mem::size_of::<AttentionGpuParams>() as u64);
        let gemm_params_at = |slot: wgpu::BufferAddress| {
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: gemm_params_buf,
                offset: slot,
                size: gp_sz,
            })
        };
        let attn_params_at = |slot: wgpu::BufferAddress| {
            wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: attn_params_buf,
                offset: slot,
                size: ap_sz,
            })
        };

        let device = self.gpu_device();
        let k_gemm_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AttentionKPreprojectGemmBG"),
            layout: &gemm_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: k_weight.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: gemm_params_at(0) },
                wgpu::BindGroupEntry { binding: 3, resource: proj_buf.as_entire_binding() },
            ],
        });
        let v_gemm_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AttentionVPreprojectGemmBG"),
            layout: &gemm_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: v_weight.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: gemm_params_at(PARAM_SLOT) },
                wgpu::BindGroupEntry { binding: 3, resource: proj_buf.as_entire_binding() },
            ],
        });
        let k_attn_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AttentionKPreprojectWriteBG"),
            layout: &attn_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: proj_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: k_weight.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: attn_params_at(0) },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(kv_binding),
                },
                wgpu::BindGroupEntry { binding: 4, resource: attn_out_scratch.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: mask_buf.as_entire_binding() },
            ],
        });
        let kv_binding = wgpu::BufferBinding {
            buffer: kv_buf,
            offset: layer_offset,
            size: std::num::NonZeroU64::new(layer_bytes.max(4)),
        };
        let v_attn_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("AttentionVPreprojectWriteBG"),
            layout: &attn_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: proj_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: v_weight.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: attn_params_at(PARAM_SLOT) },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(kv_binding),
                },
                wgpu::BindGroupEntry { binding: 4, resource: attn_out_scratch.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 5, resource: mask_buf.as_entire_binding() },
            ],
        });

        let mut encoder = self.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("AttentionKvPreprojectEncoder"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AttentionKPreprojectGemm"),
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_begin(),
            });
            pass.set_pipeline(&self.coop_gemv_pipeline);
            pass.set_bind_group(0, &k_gemm_bg, &[]);
            pass.dispatch_workgroups(kv_dim as u32, 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AttentionKPreprojectWrite"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.attention_pipeline);
            pass.set_bind_group(0, &k_attn_bg, &[]);
            pass.dispatch_workgroups(h.effective_n_kv_head().max(1), 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AttentionVPreprojectGemm"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.coop_gemv_pipeline);
            pass.set_bind_group(0, &v_gemm_bg, &[]);
            pass.dispatch_workgroups(kv_dim as u32, 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("AttentionVPreprojectWrite"),
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_end(),
            });
            pass.set_pipeline(&self.attention_pipeline);
            pass.set_bind_group(0, &v_attn_bg, &[]);
            pass.dispatch_workgroups(h.effective_n_kv_head().max(1), 1, 1);
        }
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Attention);
        true
    }
}
