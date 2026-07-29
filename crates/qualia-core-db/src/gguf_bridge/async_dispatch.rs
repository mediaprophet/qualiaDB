//! Decode-tail dispatch: final logits projection + async GEMM / async attention paths.
//! Split from gguf_bridge/forward.rs (structural; no behaviour change).
use super::*;

impl QTensorEngine {
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
    pub(crate) async fn dispatch_gemm_raw_into_async(
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
            let bind_group = self
                .gpu_device()
                .create_bind_group(&wgpu::BindGroupDescriptor {
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

            let mut encoder =
                self.device()
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
                let data = slice
                    .get_mapped_range()
                    .expect("wgpu buffer map_range failed");
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
    pub(crate) async fn dispatch_gemm_resident_chunk_async(
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

        let use_mmv = weight_ggml_type == crate::ggml_quants::GGML_TYPE_Q8_0 && (n_in % 32 == 0);
        let active_pipeline = if use_mmv {
            &self.mmv_q8_0_pipeline
        } else {
            &self.pipeline
        };
        let bind_layout = active_pipeline.get_bind_group_layout(0);
        let bind_group = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
            cpass.set_pipeline(active_pipeline);
            cpass.set_bind_group(0, &bind_group, &[0]);
            if use_mmv {
                cpass.dispatch_workgroups((n_out as u32 + 3) / 4, 1, 1);
            } else {
                cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
            }
        }
        let out_bytes = (n_out * 4) as wgpu::BufferAddress;
        encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm);

        let slice = staging.slice(..out_bytes);
        if await_wgpu_map(slice).await {
            let data = slice
                .get_mapped_range()
                .expect("wgpu buffer map_range failed");
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
        self.dispatch_gemm_raw_into_async(info, raw, input, out, n_in, n_out)
            .await
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn dispatch_attention_layer_async(
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
    pub(crate) async fn dispatch_attention_pass_async(
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
            wlog(&format!(
                "[attn_pass_async] GUARD unsupported quant kind={proj_kind}"
            ));
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
        let bind_group = self
            .gpu_device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
            let data = slice
                .get_mapped_range()
                .expect("wgpu buffer map_range failed");
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
    #[cfg(all(target_arch = "wasm32", feature = "wasm-llm-diagnostics"))]
    pub(crate) async fn dispatch_attention_q_ffn_token_async(
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
            index, hidden, emb_dim, tensors, scratch_a, scratch_b,
        )
        .await
    }
}
