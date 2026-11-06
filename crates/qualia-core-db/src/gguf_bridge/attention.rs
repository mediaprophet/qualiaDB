//! Attention: GQA params, KV mask, attention pass, CPU SDPA ref, layer + q/ffn dispatch
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
#![allow(dead_code, unused_variables, unused_mut)]
use super::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
mod fused_tail;
#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
mod preproject;

impl QTensorEngine {
    /// `QTensorEngine::matmul_dims`.
    pub(crate) fn matmul_dims(info: &GgufTensorInfo) -> (usize, usize) {
        let n_in = info.dims[0] as usize;
        let n_out = if info.n_dims > 1 && info.dims[1] > 0 {
            info.dims[1] as usize
        } else {
            1
        };
        (n_in, n_out)
    }

    /// Builds WGSL attention params from the KV layout.
    pub(crate) fn attention_gpu_params(
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
            kv_quant: if layout.int8 { 1 } else { 0 },
            // W5b Phase 4b: dict_k (low 16) | n_atoms (high 16); 0 ⇒ f32/int8.
            dict_pack: if layout.dict_k > 0 {
                layout.dict_k | (layout.dict_n_atoms << 16)
            } else {
                0
            },
        }
    }

    #[inline]
    /// `QTensorEngine::attention_kv_mask_for_dispatch`.
    pub(crate) fn attention_kv_mask_for_dispatch(
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
    pub(crate) fn dispatch_attention_pass(
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
        readback_out: Option<&mut [f32]>,
    ) -> bool {
        debug_assert!(token_idx < layout.max_context || layout.max_context == 0);
        debug_assert!(n_workgroups > 0);
        // Heterogeneous dispatch: ternary weights are strictly routed to adder paths (FFN),
        // reserving the attention MAC units for high-precision projections.
        if info.ggml_type == crate::ternary::GGML_TYPE_TERNARY_158 {
            wlog("[attention] FAILED: ternary weights not supported for attention (MAC reserved)");
            return false;
        }

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
        #[cfg(any(target_arch = "wasm32", not(feature = "gpu-runtime")))]
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

        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        if !ggml_gpu_attention_shader_supported(info.ggml_type) {
            wlog(&format!(
                "[attn_pass] GUARD unsupported quant kind={proj_kind}"
            ));
            return false;
        }
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        let batch = num_tokens_in_batch.max(1) as usize;
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        let hidden_elems = n_embd.checked_mul(batch).unwrap_or(0);
        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
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
            if crate::gguf_bridge::execution_env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
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

        #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
        {
            let (mask_words, mask_active, mask_word_count) =
                Self::attention_kv_mask_for_dispatch(layout, token_idx, proj_kind);
            let mut params = Self::attention_gpu_params(
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
            if n_embd != h.n_embd as usize {
                params.proj_row_stride = n_embd as u32;
            }
            let input_buf = self.gemm_input_buf.as_ref().unwrap();
            let weight_buf = self.gemm_weight_buf.as_ref().unwrap();
            let output_buf = self.gemm_output_buf.as_ref().unwrap();
            let params_buf = self.attention_params_buf.as_ref().unwrap();
            let mask_buf = self.attention_mask_buf.as_ref().unwrap();
            let kv_buf = self.kv_cache_gpu.as_ref().unwrap();
            let staging = self.gemm_output_staging.as_ref().unwrap();

            etalon_inference::gpu_attention::upload_input(
                self.gpu_queue(),
                input_buf,
                hidden,
                n_embd,
                batch,
                norm_weight,
            );
            // Phase 2 (attention): bind this projection's resident VRAM buffer (uploaded once, keyed by
            // its mmap address) instead of re-uploading the Q/K/V weight into the shared buffer EVERY
            // token. For a 3B that re-upload is ~30 MB/layer × n_layer ≈ ~0.8 GB/token of PCIe traffic
            // the GEMM path already shed; attention was still paying it. Output is byte-identical (same
            // bytes, same offsets). Falls back to the per-token upload when residency is off.
            let resident = if crate::llm_bench::resident_weights_enabled() {
                self.resident_weight_buffer(raw_weights.as_ptr() as u64, raw_weights)
            } else {
                None
            };
            let weight_binding: &etalon::Buffer = match resident.as_ref() {
                Some(r) => r,
                None => {
                    self.write_weight_words(raw_weights, self.max_tensor_bytes);
                    weight_buf
                }
            };

            let resources = etalon_inference::gpu_attention::PassResources {
                input: input_buf,
                weight: weight_binding,
                output: output_buf,
                params: params_buf,
                mask: mask_buf,
                kv: kv_buf,
                staging,
                bind_layout: &self.attention_bind_layout,
                pipeline: &self.attention_pipeline,
            };
            let readback_elems = readback_out.as_ref().map(|o| o.len()).unwrap_or(0);
            let mut encoder = match etalon_inference::gpu_attention::encode_pass(
                self.gpu_device(),
                self.gpu_queue(),
                &resources,
                &params,
                &mask_words,
                n_workgroups,
                readback_elems,
                crate::llm_gpu_profiler::pass_writes_both(),
            ) {
                Some(encoder) => encoder,
                None => return false,
            };
            crate::llm_gpu_profiler::resolve(&mut encoder);
            self.submit_commands(Some(encoder.finish()));
            crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Attention);

            if readback_elems == 0 {
                return true;
            }

            let out_bytes = (readback_elems * 4) as etalon::BufferAddress;
            let slice = staging.slice(..out_bytes);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(etalon::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            self.poll_wait();
            #[cfg(not(target_arch = "wasm32"))]
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                if handle.block_on(rx).ok().map(|m| m.is_ok()).unwrap_or(false) {
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
            }
            let _ = staging.unmap();
            false
        }
    }

    /// WASM CPU attention fallback (Phase 2A). Projects one Q/K/V tensor for a batch of
    /// tokens, applies RoPE (Q/K), writes K/V into `kv_cache_cpu`, and runs SDPA for Q.
    /// `proj_kind`: 0=Q, 1=K, 2=V. Available on native too as the #48 correctness reference.
    pub(crate) fn cpu_attention_pass(
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
        let cache = match self.kv_cache_cpu.as_ref() {
            Some(cache) => cache,
            None => return false,
        };
        // Legacy engine cache is exclusively used by the admitted decode thread.
        let kv =
            unsafe { core::slice::from_raw_parts_mut(cache.as_ptr() as *mut f32, cache.len()) };
        // W5b Phase 4b: dict-coded KV cache. When the layout stores sparse codes, clone this layer's
        // dictionaries once for the call — the write path encodes K/V to codes, the read path (Q pass)
        // reconstructs them. `dict_k = 0` ⇒ the untouched f32/int8 paths below.
        let dict_mode = layout.dict_k > 0;
        #[cfg(not(target_arch = "wasm32"))]
        let (k_dict, v_dict) = if dict_mode {
            match proj_kind {
                1 => (
                    crate::kv_dict_runtime::clone_layer_dict(layer as usize, true),
                    None,
                ),
                2 => (
                    None,
                    crate::kv_dict_runtime::clone_layer_dict(layer as usize, false),
                ),
                _ => (
                    crate::kv_dict_runtime::clone_layer_dict(layer as usize, true),
                    crate::kv_dict_runtime::clone_layer_dict(layer as usize, false),
                ),
            }
        } else {
            (None, None)
        };
        #[cfg(target_arch = "wasm32")]
        let (k_dict, v_dict): (
            Option<crate::kv_dict::KvDictionary>,
            Option<crate::kv_dict::KvDictionary>,
        ) = (None, None);

        let host = super::cpu_attention_adapter::CpuAttentionAdapter {
            k: k_dict.as_ref(),
            v: v_dict.as_ref(),
        };
        let layout = etalon_inference::ordinary::cpu_attention::CpuKvLayout {
            max_context: layout.max_context,
            layer_stride: layout.layer_stride,
            slot_kv_elems: layout.slot_kv_elems,
            n_kv_head: layout.n_kv_head,
            head_dim: layout.head_dim,
            dict_k: layout.dict_k,
        };
        etalon_inference::ordinary::cpu_attention::cpu_attention_pass(
            &host,
            kv,
            hidden,
            n_embd,
            num_tokens_in_batch,
            batch_start_token_idx,
            &layout,
            layer,
            &(*h).into(),
            info,
            raw_weights,
            proj_kind,
            norm_weight,
            readback_out,
        )
    }

    /// GPU-fused Q/K/V projections, RoPE, ring-buffer KV write, and GQA online-softmax.
    pub(crate) fn dispatch_attention_layer(
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
        if index.hyperparams.architecture == etalon_model::inference::ARCH_QWEN35_MOE {
            return None; // Gated attention executes only through its admitted owner.
        }
        etalon_inference::ordinary::attention::AttentionEngine { host: self }
            .dispatch_attention_layer(
                index, layer, token_idx, hidden, emb_dim, tensors, scratch_a, scratch_b,
            )
    }

    /// Q+attn, output projection, and FFN for one token (K/V already in arena).
    pub(crate) fn dispatch_attention_q_ffn_token(
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
        if index.hyperparams.architecture == etalon_model::inference::ARCH_QWEN35_MOE {
            return false;
        }
        etalon_inference::ordinary::attention::AttentionEngine { host: self }
            .dispatch_attention_q_ffn_token(
                index, layer, token_idx, hidden, emb_dim, tensors, scratch_a, scratch_b,
            )
    }

    /// CUDA residual attention: prefer **P4 device SDPA/KV** (one D2H); else sticky
    /// Q/K/V GEMV → host RoPE/KV/SDPA → CUDA O-proj.
    /// Returns true when `scratch_a[..emb_dim]` holds the attention residual delta.
    #[cfg(all(not(target_arch = "wasm32"), feature = "cuda"))]
    pub(super) fn try_cuda_soa_attention_layer(
        &self,
        hidden_input: &[f32],
        n_embd: usize,
        q_dim: usize,
        kv_dim: usize,
        n_head: usize,
        n_kv: usize,
        head_dim: usize,
        layout: &KvCacheLayout,
        layer: u32,
        token_idx: u32,
        h: &crate::gguf_sharder::GgufHyperparams,
        q_raw: &[u8],
        k_raw: &[u8],
        v_raw: &[u8],
        out_info: Option<&GgufTensorInfo>,
        mmap: &[u8],
        tensor_data_start: u64,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        let layout = etalon_inference::ordinary::cuda_attention::CudaAttentionLayout {
            max_context: layout.max_context,
            n_layer: layout.n_layer,
            n_kv_head: layout.n_kv_head,
            head_dim: layout.head_dim,
            slot_kv_elems: layout.slot_kv_elems,
            layer_stride: layout.layer_stride,
            total_f32_elems: layout.total_f32_elems,
            int8: layout.int8,
            dict_k: layout.dict_k,
        };
        etalon_inference::ordinary::cuda_attention::try_cuda_soa_attention_layer(
            self,
            hidden_input,
            n_embd,
            q_dim,
            kv_dim,
            n_head,
            n_kv,
            head_dim,
            &layout,
            layer,
            token_idx,
            &(*h).into(),
            q_raw,
            k_raw,
            v_raw,
            out_info,
            mmap,
            tensor_data_start,
            scratch_a,
            scratch_b,
        )
    }

    /// Certified Naga-validated WGSL Forge causal attention device dispatch during decode.
    /// Native + `wgsl-forge` only — wasm profiles do not compile the Forge module.
    #[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
    pub fn dispatch_forge_causal_attention(
        &mut self,
        query: &[f32],
        keys: &[f32],
        values: &[f32],
        scale: f32,
        shape: crate::wgsl_forge::AttentionShape,
        out: &mut [f32],
    ) -> Result<(), crate::wgsl_forge::ForgeError> {
        let schedule = crate::wgsl_forge::AttentionSchedule::a2000_decode();
        schedule.validate(shape)?;
        let q_len = (shape.head_count * shape.head_dim) as usize;
        let kv_len = (shape.head_count * shape.context_tokens * shape.head_dim) as usize;
        if query.len() < q_len || keys.len() < kv_len || values.len() < kv_len || out.len() < q_len
        {
            return Err(crate::wgsl_forge::ForgeError::InvalidKernel(
                "dispatch_forge_causal_attention buffers smaller than required shape".into(),
            ));
        }

        let shader = crate::wgsl_forge::synthesize_causal_attention(shape, schedule)?;
        let params = crate::wgsl_forge::AttentionShaderParams {
            context_tokens: shape.context_tokens,
            head_dim: shape.head_dim,
            head_count: shape.head_count,
            _pad0: 0,
            scale,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        if etalon_inference::gpu_attention::forge::dispatch_forge_attention(
            self,
            &shader.source,
            bytemuck::bytes_of(&params),
            q_len,
            kv_len,
            shape.head_count,
            query,
            keys,
            values,
            out,
        ) {
            return Ok(());
        }

        crate::wgsl_forge::causal_attention_oracle(shape, query, keys, values, scale, out)
    }
}
