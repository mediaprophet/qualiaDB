//! Attention: GQA params, KV mask, attention pass, CPU SDPA ref, layer + q/ffn dispatch
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

#[cfg(not(target_arch = "wasm32"))]
mod fused_tail;
#[cfg(not(target_arch = "wasm32"))]
mod preproject;

impl QTensorEngine {
    pub(crate) fn matmul_dims(info: &GgufTensorInfo) -> (usize, usize) {
        let n_in = info.dims[0] as usize;
        let n_out = if info.n_dims > 1 && info.dims[1] > 0 {
            info.dims[1] as usize
        } else {
            1
        };
        (n_in, n_out)
    }

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
            _pad: 0,
        }
    }

    #[inline]
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
        if !ggml_gpu_attention_shader_supported(info.ggml_type) {
            wlog(&format!(
                "[attn_pass] GUARD unsupported quant kind={proj_kind}"
            ));
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

            // #49: honor norm_weight on the GPU path. Prefill passes RAW hidden + attn_norm here (the
            // CPU-reference convention); without per-token RMSNorm the prefill K/V are projected from the
            // un-normed residual and the KV cache explodes. Decode passes norm_weight=None → single upload.
            let norm_ok =
                n_embd <= MAX_HIDDEN_DIM && norm_weight.map_or(false, |w| w.len() >= n_embd);
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
                    self.gpu_queue().write_buffer(
                        input_buf,
                        0,
                        bytemuck::cast_slice(&hidden[..hidden_elems]),
                    );
                }
            }
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
            let weight_binding: &wgpu::Buffer = match resident.as_ref() {
                Some(r) => r,
                None => {
                    self.write_weight_words(raw_weights, self.max_tensor_bytes);
                    weight_buf
                }
            };
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

            let bind_layout = self.attention_bind_layout.clone();
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
                            resource: weight_binding.as_entire_binding(),
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

            let mut encoder =
                self.device()
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
        let head_dim = h.head_dim() as usize;
        let n_head = h.n_head as usize;
        let n_kv = h.effective_n_kv_head() as usize;
        if head_dim == 0 || n_head == 0 || n_kv == 0 {
            return false;
        }
        let (n_in, out_dim) = Self::matmul_dims(info);
        if out_dim == 0 || out_dim > MAX_STACK_GEMM_OUT || head_dim > out_dim {
            wlog(&format!(
                "[cpu_attn] bad dims out_dim={out_dim} head_dim={head_dim}"
            ));
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
                wlog(&format!(
                    "[cpu_attn] hidden OOB t={t} need={}",
                    tok_start + n_embd
                ));
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
            if !stack_gemm_quant(
                raw_weights,
                info,
                gemm_in,
                &mut proj[..out_dim],
                n_in,
                out_dim,
            ) {
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
                                let k_idx = layout.k_index(layer, past_slot, kv_h as u32, d as u32);
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
                                let v_idx = layout.v_index(layer, past_slot, kv_h as u32, d as u32);
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

        #[cfg(not(target_arch = "wasm32"))]
        if crate::llm_bench::attention_preproject_enabled()
            && self.dispatch_attention_kv_preproject_fused(
                hidden_input,
                n_embd,
                &layout,
                layer,
                token_idx,
                &h,
                k_info,
                k_raw,
                v_info,
                v_raw,
                n_kv * head_dim,
            )
        {
            if crate::llm_bench::attention_o_fuse_enabled() {
                if let Some(out_info) = tensors.attn_output {
                    let (o_in, o_out) = Self::matmul_dims(&out_info);
                    if o_in <= q_dim && o_out <= scratch_a.len() {
                        if let Ok(o_raw) = crate::ggml_quants::fetch_tensor_bytes(
                            mmap,
                            index.tensor_data_start,
                            &out_info,
                        ) {
                            if self.dispatch_attention_q_o_fused(
                                hidden_input,
                                n_embd,
                                &layout,
                                layer,
                                token_idx,
                                &h,
                                q_info,
                                q_raw,
                                &out_info,
                                o_raw,
                                &mut scratch_a[..o_out],
                                o_in,
                                o_out,
                            ) {
                                return Some(o_out.min(emb_dim));
                            }
                        }
                    }
                }
            }
            if self.dispatch_attention_pass(
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
                } else {
                    let n = q_dim.min(emb_dim);
                    scratch_a[..n].copy_from_slice(&scratch_b[..n]);
                    return Some(n);
                }
            }
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

        #[cfg(not(target_arch = "wasm32"))]
        if crate::llm_bench::attention_o_fuse_enabled() {
            if let Some(out_info) = tensors.attn_output {
                let (o_in, o_out) = Self::matmul_dims(&out_info);
                if o_in <= q_dim && o_out <= scratch_a.len() {
                    if let Ok(o_raw) = crate::ggml_quants::fetch_tensor_bytes(
                        mmap,
                        index.tensor_data_start,
                        &out_info,
                    ) {
                        if self.dispatch_attention_q_o_fused(
                            hidden_input,
                            n_embd,
                            &layout,
                            layer,
                            token_idx,
                            &h,
                            q_info,
                            q_raw,
                            &out_info,
                            o_raw,
                            &mut scratch_a[..o_out],
                            o_in,
                            o_out,
                        ) {
                            return Some(o_out.min(emb_dim));
                        }
                    }
                }
            }
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
}
