//! Forward-pass orchestration: prefill (batch/chunk, sync + async + MC8-fused), transformer
//! layer/forward (sync + async), encode helpers, topology-draft verify, and async lifecycle.
//! Split from gguf_bridge/mod.rs (structural refactor; no behaviour change).
use super::*;

impl QTensorEngine {
    /// One transformer layer for a batched prefill chunk: batched K/V then per-token Q+FFN.
    pub(crate) fn dispatch_prefill_layer_batch(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        if n_tokens == 0 {
            wlog("[prefill_layer] FAILED n_tokens=0");
            return false;
        }
        let layout = match self.kv_layout {
            Some(l) => l,
            None => {
                wlog("[prefill_layer] FAILED kv_layout is None");
                return false;
            }
        };
        let tensors = index.get_layer_tensors(layer);
        let k_info = match tensors.attn_k.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_k layer={layer}"));
                return false;
            }
        };
        let v_info = match tensors.attn_v.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_v layer={layer}"));
                return false;
            }
        };
        if tensors.attn_q.is_none() {
            wlog(&format!("[prefill_layer] FAILED missing attn_q layer={layer}"));
            return false;
        }
        let h = index.hyperparams;
        let n_kv = h.effective_n_kv_head();
        let n_embd = h.n_embd as usize;
        let batch_elems = n_embd * n_tokens as usize;
        if batch_elems > batch_hidden.len() {
            wlog(&format!(
                "[prefill_layer] FAILED batch_elems OOB elems={batch_elems} hidden={}",
                batch_hidden.len()
            ));
            return false;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => {
                wlog("[prefill_layer] FAILED gguf_mmap is None");
                return false;
            }
        };
        let k_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_k bytes: {e:?}"));
                    return false;
                }
            };
        let v_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_v bytes: {e:?}"));
                    return false;
                }
            };
        let n_kv_wg = n_tokens.saturating_mul(n_kv);
        // attn_norm MUST be applied to the K/V projection input on ALL targets — native previously
        // passed None here → prefill wrote K/V from the RAW residual → the KV cache exploded across
        // layers → the whole forward (and decode reading it) blew up. (#48)
        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let norm_weight_attn: Option<&[f32]> = tensors.attn_norm.as_ref().and_then(|info| {
            let n = dequant_norm_row_into(mmap, index.tensor_data_start, info, &mut norm_w_attn);
            if n >= n_embd {
                Some(&norm_w_attn[..n_embd])
            } else {
                None
            }
        });
        if !self.dispatch_attention_pass(
            &batch_hidden[..batch_elems],
            n_embd,
            n_tokens,
            batch_start_token_idx,
            &layout,
            layer,
            batch_start_token_idx,
            &h,
            k_info,
            k_raw,
            1,
            n_kv_wg,
            norm_weight_attn,
            None,
        ) {
            wlog(&format!("[prefill_layer] K pass FAILED layer={layer}"));
            return false;
        }
        if !self.dispatch_attention_pass(
            &batch_hidden[..batch_elems],
            n_embd,
            n_tokens,
            batch_start_token_idx,
            &layout,
            layer,
            batch_start_token_idx,
            &h,
            v_info,
            v_raw,
            2,
            n_kv_wg,
            norm_weight_attn,
            None,
        ) {
            wlog(&format!("[prefill_layer] V pass FAILED layer={layer}"));
            return false;
        }
        for t in 0..n_tokens {
            let abs = batch_start_token_idx + t;
            let off = t as usize * emb_dim;
            if !self.dispatch_attention_q_ffn_token(
                index,
                layer,
                abs,
                &mut batch_hidden[off..off + emb_dim],
                emb_dim,
                &tensors,
                scratch_a,
                scratch_b,
            ) {
                wlog(&format!("[prefill_layer] q_ffn FAILED layer={layer} t={t} abs={abs}"));
                return false;
            }
        }
        true
    }

    /// Phase 2B: batched prefill layer via async GPU attention (K/V GPU; Q+FFN per token).
    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn dispatch_prefill_layer_batch_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        if n_tokens == 0 {
            wlog("[prefill_layer] FAILED n_tokens=0");
            return false;
        }
        let layout = match self.kv_layout {
            Some(l) => l,
            None => {
                wlog("[prefill_layer] FAILED kv_layout is None");
                return false;
            }
        };
        let tensors = index.get_layer_tensors(layer);
        let k_info = match tensors.attn_k.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_k layer={layer}"));
                return false;
            }
        };
        let v_info = match tensors.attn_v.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!("[prefill_layer] FAILED missing attn_v layer={layer}"));
                return false;
            }
        };
        if tensors.attn_q.is_none() {
            wlog(&format!("[prefill_layer] FAILED missing attn_q layer={layer}"));
            return false;
        }
        let h = index.hyperparams;
        let n_kv = h.effective_n_kv_head();
        let n_embd = h.n_embd as usize;
        let batch_elems = n_embd * n_tokens as usize;
        if batch_elems > batch_hidden.len() {
            wlog(&format!(
                "[prefill_layer] FAILED batch_elems OOB elems={batch_elems} hidden={}",
                batch_hidden.len()
            ));
            return false;
        }
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => {
                wlog("[prefill_layer] FAILED gguf_mmap is None");
                return false;
            }
        };
        let k_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_k bytes: {e:?}"));
                    return false;
                }
            };
        let v_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
                Ok(s) => s,
                Err(e) => {
                    wlog(&format!("[prefill_layer] FAILED fetch attn_v bytes: {e:?}"));
                    return false;
                }
            };
        let mut norm_w_attn = [0f32; MAX_HIDDEN_DIM];
        let mut norm_scratch = [0f32; PREFILL_CHUNK_STACK_FLOATS];
        let attn_input: &mut [f32] = if let Some(norm_info) = tensors.attn_norm.as_ref() {
            let n = dequant_norm_row_into(
                mmap,
                index.tensor_data_start,
                norm_info,
                &mut norm_w_attn,
            );
            if n >= n_embd {
                for t in 0..n_tokens as usize {
                    let off = t * n_embd;
                    norm_scratch[off..off + n_embd].copy_from_slice(&batch_hidden[off..off + n_embd]);
                    rms_norm_inplace(
                        &mut norm_scratch[off..off + n_embd],
                        &norm_w_attn[..n_embd],
                        RMS_NORM_EPS,
                    );
                }
                &mut norm_scratch[..batch_elems]
            } else {
                batch_hidden
            }
        } else {
            batch_hidden
        };
        let n_kv_wg = n_tokens.saturating_mul(n_kv);
        if !self
            .dispatch_attention_pass_async(
                attn_input,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                k_info,
                k_raw,
                1,
                n_kv_wg,
                None,
            )
            .await
        {
            wlog(&format!("[prefill_layer] K pass FAILED layer={layer}"));
            return false;
        }
        if !self
            .dispatch_attention_pass_async(
                attn_input,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                v_info,
                v_raw,
                2,
                n_kv_wg,
                None,
            )
            .await
        {
            wlog(&format!("[prefill_layer] V pass FAILED layer={layer}"));
            return false;
        }
        for t in 0..n_tokens {
            let abs = batch_start_token_idx + t;
            let off = t as usize * emb_dim;
            if !self
                .dispatch_attention_q_ffn_token_async(
                    index,
                    layer,
                    abs,
                    &mut batch_hidden[off..off + emb_dim],
                    emb_dim,
                    &tensors,
                    scratch_a,
                    scratch_b,
                )
                .await
            {
                wlog(&format!("[prefill_layer] q_ffn FAILED layer={layer} t={t} abs={abs}"));
                return false;
            }
        }
        true
    }

    /// Chunked prefill: populate KV arena for `n_tokens` prompt positions starting at `batch_start`.
    pub fn dispatch_prefill_chunk(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        max_layers: u32,
    ) -> bool {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || n_tokens == 0 {
            return false;
        }
        let limit = if max_layers == 0 {
            n_layer
        } else {
            max_layers.min(n_layer)
        };
        for layer in 0..limit {
            if !self.dispatch_prefill_layer_batch(
                index,
                layer,
                batch_hidden,
                emb_dim,
                n_tokens,
                batch_start_token_idx,
                scratch_a,
                scratch_b,
            ) {
                return false;
            }
        }
        true
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
        // Decode-profiler: intra-layer split — attention (this block) vs FFN (below).
        let t_attn = std::time::Instant::now();

        if tensors.attn_q.is_some() && tensors.attn_k.is_some() && tensors.attn_v.is_some() {
            if let Some(n) = self.dispatch_attention_layer(
                index,
                layer,
                token_idx,
                &hidden[..emb_dim],
                emb_dim,
                &tensors,
                scratch_a,
                scratch_b,
            ) {
                add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..n], n);
                attn_ok = true;
            }
        } else if let Some(info) = tensors.attn_output {
            let (n_in, n_out) = Self::matmul_dims(&info);
            if n_in <= emb_dim
                && self.dispatch_gemm_into(index, &info, &hidden[..n_in], scratch_a, n_in, n_out)
            {
                add_residual_inplace(
                    &mut hidden[..emb_dim],
                    &scratch_a[..n_out],
                    emb_dim.min(n_out),
                );
                attn_ok = true;
            }
        }

        crate::llm_bench::add_decode_attn_ns(t_attn.elapsed().as_nanos() as u64);

        if !attn_ok && tensors.attn_output.is_none() && tensors.ffn_gate.is_none() {
            return false;
        }

        // #48 diagnostic: localize the residual explosion — attention output magnitude per layer.
        if layer < 3 && std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
            let max_attn = scratch_a[..emb_dim].iter().fold(0f32, |m, &v| m.max(v.abs()));
            let max_hid = hidden[..emb_dim].iter().fold(0f32, |m, &v| m.max(v.abs()));
            eprintln!(
                "[layer-dbg] L{} attn_ok={} attn_norm={} ffn_norm={} max|attn_out|={:.4} max|hidden_postattn|={:.4}",
                layer,
                attn_ok,
                tensors.attn_norm.is_some(),
                tensors.ffn_norm.is_some(),
                max_attn,
                max_hid,
            );
        }

        let t_ffn = std::time::Instant::now();
        let ffn_ok = self.dispatch_ffn_block_pre_norm(
            index,
            hidden,
            emb_dim,
            &tensors,
            scratch_a,
            scratch_b,
        );
        crate::llm_bench::add_decode_ffn_ns(t_ffn.elapsed().as_nanos() as u64);
        ffn_ok
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
        // #48 diagnostic: localize where the hidden state turns non-finite (gated; runs once).
        use std::sync::atomic::{AtomicBool, Ordering as DbgOrdering};
        static FWD_DBG_DONE: AtomicBool = AtomicBool::new(false);
        let dbg = std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok()
            && !FWD_DBG_DONE.swap(true, DbgOrdering::Relaxed);
        if dbg {
            let nf = hidden[..emb_dim].iter().filter(|v| !v.is_finite()).count();
            eprintln!(
                "[fwd-dbg] post-embed nonfinite={}/{} sample={:?}",
                nf,
                emb_dim,
                &hidden[..emb_dim.min(4)]
            );
        }
        let mut ran = 0u32;
        for layer in 0..limit {
            if self.dispatch_transformer_layer(
                index, layer, token_idx, hidden, emb_dim, scratch_a, scratch_b,
            ) {
                ran += 1;
            }
            if dbg {
                let nf = hidden[..emb_dim].iter().filter(|v| !v.is_finite()).count();
                eprintln!(
                    "[fwd-dbg] after layer {} nonfinite={}/{} ran={} sample={:?}",
                    layer,
                    nf,
                    emb_dim,
                    ran,
                    &hidden[..emb_dim.min(4)]
                );
                if nf > 0 {
                    break;
                }
            }
        }
        ran
    }

    /// Phase 2B: async single-layer forward (GPU `map_async`; CPU path unchanged in sync API).
    #[cfg(target_arch = "wasm32")]
    pub async fn dispatch_transformer_layer_async(
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
            if let Some(n) = self
                .dispatch_attention_layer_async(
                    index,
                    layer,
                    token_idx,
                    &hidden[..emb_dim],
                    emb_dim,
                    &tensors,
                    scratch_a,
                    scratch_b,
                )
                .await
            {
                add_residual_inplace(&mut hidden[..emb_dim], &scratch_a[..n], n);
                attn_ok = true;
            }
        } else if let Some(info) = tensors.attn_output {
            let (n_in, n_out) = Self::matmul_dims(&info);
            if n_in <= emb_dim
                && self
                    .dispatch_gemm_into_async(
                        index,
                        &info,
                        &hidden[..n_in],
                        scratch_a,
                        n_in,
                        n_out,
                    )
                    .await
            {
                add_residual_inplace(
                    &mut hidden[..emb_dim],
                    &scratch_a[..n_out],
                    emb_dim.min(n_out),
                );
                attn_ok = true;
            }
        }

        if !attn_ok && tensors.attn_output.is_none() && tensors.ffn_gate.is_none() {
            return false;
        }

        self.dispatch_ffn_block_pre_norm_async(
            index,
            hidden,
            emb_dim,
            &tensors,
            scratch_a,
            scratch_b,
        )
        .await
    }

    /// MC8: Q + o_proj + FFN tail (K/V already written for this token).
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn encode_attn_ffn_tail_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        emb_dim: usize,
        tensors: &crate::gguf_sharder::LayerTensors,
        token_hidden: &wgpu::Buffer,
        attn_input: Option<&wgpu::Buffer>,
        work_aliases_hidden: bool,
    ) -> bool {
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let work_buf = self.gemm_output_buf.as_ref().unwrap();
        let aux_buf = self.gemm_aux_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        let q_info = match tensors.attn_q.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let q_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let q_in_buf = if let Some(pre) = attn_input {
            pre
        } else if let Some(norm) = tensors.attn_norm.as_ref() {
            if !self.upload_norm_weights(mmap, index.tensor_data_start, norm, n_embd) {
                return false;
            }
            self.encode_elem(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                1,
                token_hidden,
                norm_buf,
                aux_buf,
            );
            aux_buf
        } else {
            token_hidden
        };
        let ffn_buf = self.gemm_ffn_buf.as_ref().unwrap();
        let emb_bytes = (emb_dim * 4) as wgpu::BufferAddress;
        let q_dim = (h.n_head * h.head_dim()) as usize;
        let (mask_words, mask_active, mask_word_count) =
            Self::attention_kv_mask_for_dispatch(&layout, token_idx, 0);
        let q_params = Self::attention_gpu_params(
            &h,
            &layout,
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
        let q_off = self.mc8_upload_attn_param(&q_params);
        if mask_active != 0 {
            self.gpu_queue().write_buffer(
                self.attention_mask_buf.as_ref().unwrap(),
                0,
                bytemuck::cast_slice(&mask_words),
            );
        }
        if !self.encode_attention_pass_gpu(
            pipeline,
            q_in_buf,
            ffn_buf,
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
            h.n_head,
            q_off,
            Mc8WeightRole::AttnQ,
        ) {
            return false;
        }
        self.mc8_flush(pipeline);
        if let Some(out_info) = tensors.attn_output.as_ref() {
            let (o_in, o_out) = Self::matmul_dims(out_info);
            let o_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, out_info)
            {
                Ok(s) => s,
                Err(_) => return false,
            };
            if work_aliases_hidden {
                pipeline.encoder.copy_buffer_to_buffer(token_hidden, 0, aux_buf, 0, emb_bytes);
                self.mc8_flush(pipeline);
            }
            if o_in > q_dim
                || !self.encode_gemm_bufs(
                    pipeline,
                    out_info,
                    o_raw,
                    o_in,
                    o_out,
                    ffn_buf,
                    work_buf,
                )
            {
                return false;
            }
            self.mc8_flush(pipeline);
            let attn_residual_base: &wgpu::Buffer = if work_aliases_hidden {
                aux_buf
            } else {
                token_hidden
            };
            // Never use `prefill_scratch_buf` here — it holds batched attn RMSNorm rows.
            self.encode_residual_add_gpu(
                pipeline,
                attn_residual_base,
                work_buf,
                token_hidden,
                ffn_buf,
                emb_dim as u32,
            );
        } else {
            self.encode_residual_add_gpu(
                pipeline,
                token_hidden,
                ffn_buf,
                token_hidden,
                aux_buf,
                emb_dim as u32,
            );
        }
        self.mc8_flush(pipeline);
        let gate_info = match tensors.ffn_gate.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let up_info = match tensors.ffn_up.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let down_info = match tensors.ffn_down.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let (gate_in, n_ffn) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > n_embd || up_in != gate_in || up_out != n_ffn || dn_in != n_ffn || dn_out < n_embd {
            return false;
        }
        let gate_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info)
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        let up_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let down_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let base_save = match self.prefill_scratch_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        pipeline.encoder.copy_buffer_to_buffer(token_hidden, 0, base_save, 0, emb_bytes);
        self.mc8_flush(pipeline);
        if let Some(norm) = tensors.ffn_norm.as_ref() {
            if !self.upload_norm_weights(mmap, index.tensor_data_start, norm, n_embd) {
                return false;
            }
            self.encode_elem(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                1,
                token_hidden,
                norm_buf,
                aux_buf,
            );
        } else {
            pipeline.encoder.copy_buffer_to_buffer(token_hidden, 0, aux_buf, 0, emb_bytes);
        }
        self.mc8_flush(pipeline);
        if !self.encode_gemm_bufs(pipeline, gate_info, gate_raw, gate_in, n_ffn, aux_buf, work_buf) {
            return false;
        }
        self.mc8_flush(pipeline);
        if !self.encode_gemm_bufs(pipeline, up_info, up_raw, up_in, n_ffn, aux_buf, ffn_buf) {
            return false;
        }
        self.mc8_flush(pipeline);
        self.encode_elem(
            pipeline,
            ELEM_OP_SILU_MUL,
            n_ffn as u32,
            1,
            work_buf,
            ffn_buf,
            aux_buf,
        );
        self.mc8_flush(pipeline);
        if !self.encode_gemm_bufs(pipeline, down_info, down_raw, dn_in, dn_out, aux_buf, work_buf) {
            return false;
        }
        self.mc8_flush(pipeline);
        // FFN residual: down output is in work_buf; pre-FFN skip is in base_save.
        // Use aux_buf as scratch (SiLU output consumed; down GEMM flushed above).
        self.encode_residual_add_gpu(
            pipeline,
            base_save,
            work_buf,
            token_hidden,
            aux_buf,
            emb_dim as u32,
        );
        self.mc8_flush(pipeline);
        true
    }

    /// MC8: encode one decode layer entirely on GPU (no map_async).
    /// Superseded by the Part 3w super-arena decode forward (kept for reference/fallback).
    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)]
    pub(crate) fn encode_transformer_layer_gpu(
        &self,
        pipeline: &mut WasmGpuPipeline,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        token_idx: u32,
        emb_dim: usize,
    ) -> bool {
        let tensors = index.get_layer_tensors(layer);
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        if emb_dim < n_embd {
            return false;
        }
        let hidden_buf = self.gemm_input_buf.as_ref().unwrap();
        let work_buf = self.gemm_output_buf.as_ref().unwrap();
        let aux_buf = self.gemm_aux_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();

        let (k_info, v_info) = match (tensors.attn_k.as_ref(), tensors.attn_v.as_ref()) {
            (Some(k), Some(v)) => (k, v),
            _ => return false,
        };
        if tensors.attn_q.is_none() {
            return false;
        }
        let k_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let v_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
            Ok(s) => s,
            Err(_) => return false,
        };

        let attn_input = if let Some(norm) = tensors.attn_norm.as_ref() {
            if !self.upload_norm_weights(mmap, index.tensor_data_start, norm, n_embd) {
                return false;
            }
            self.encode_elem(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                1,
                hidden_buf,
                norm_buf,
                aux_buf,
            );
            self.mc8_flush(pipeline);
            aux_buf
        } else {
            hidden_buf
        };

        let n_kv = h.effective_n_kv_head();
        let mut attn_arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let k_params = Self::attention_gpu_params(
            &h,
            &layout,
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
        let v_params = Self::attention_gpu_params(
            &h,
            &layout,
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
        let k_off = attn_arena.push(&k_params);
        let v_off = attn_arena.push(&v_params);
        attn_arena.upload(
            self.gpu_queue(),
            self.attention_params_buf.as_ref().unwrap(),
        );
        if !self.encode_attention_pass_gpu(
            pipeline,
            attn_input,
            work_buf,
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
            n_kv,
            k_off,
            Mc8WeightRole::AttnK,
        ) {
            return false;
        }
        if !self.encode_attention_pass_gpu(
            pipeline,
            attn_input,
            work_buf,
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
            n_kv,
            v_off,
            Mc8WeightRole::AttnV,
        ) {
            return false;
        }
        self.mc8_flush(pipeline);
        self.encode_attn_ffn_tail_gpu(
            pipeline,
            index,
            layer,
            token_idx,
            emb_dim,
            &tensors,
            hidden_buf,
            Some(attn_input),
            false,
        )
    }

    /// MC8 Part 3w: decode forward via the prefill super-arena (n_tokens=1).
    /// Reuses `mc8_stage_prefill_layer_super_arena` + `encode_prefill_q_ffn_tail_fused`
    /// (dynamic-offset uniforms + 7 disjoint weight buffers) → **2 submits/layer**
    /// (KV-visibility flush + layer-end), down from the legacy 13 flushes/layer.
    /// A single decode token at absolute position `token_idx` is a 1-row prefill chunk:
    /// dense causal Q (`mask_active=0`, `logical <= abs_pos`) is correct for decode.
    #[cfg(target_arch = "wasm32")]
    pub async fn dispatch_transformer_forward_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &mut [f32],
        emb_dim: usize,
        _scratch_a: &mut [f32],
        _scratch_b: &mut [f32],
        token_idx: u32,
        max_layers: u32,
    ) -> u32 {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || !self.mc8_buffers_ready() {
            return 0;
        }
        if self.prefill_work_buf_a.is_none() || self.prefill_work_buf_b.is_none() {
            wlog("[MC8] decode forward: prefill work buffers missing — cannot run super-arena");
            return 0;
        }
        // Part 3x: upload all layer weights to GPU once (idempotent; falls back if it fails).
        if !self.mc8_weights_resident {
            let _ = self.mc8_upload_all_resident_weights(index);
        }
        let limit = if max_layers == 0 {
            n_layer
        } else {
            max_layers.min(n_layer)
        };
        let n_embd = index.hyperparams.n_embd as usize;
        if emb_dim < n_embd || n_embd > hidden.len() || n_embd > self.gemm_max_input_floats {
            return 0;
        }
        let prefill_scratch = match self.prefill_scratch_buf.as_ref() {
            Some(b) => b,
            None => return 0,
        };
        let batch_buf = self.gemm_input_buf.as_ref().unwrap();
        let token_buf = self.gemm_output_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        self.gpu_queue().write_buffer(
            batch_buf,
            0,
            bytemuck::cast_slice(&hidden[..n_embd]),
        );
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return 0,
        };
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return 0,
        };
        let n_tokens = 1u32;
        let mut ran = 0u32;
        // Phase 5.4 single-submit: one encoder + monotonic uniform cursors across the whole forward;
        // flush only at MC8_LAYERS_PER_ENCODER chunk boundaries → 1 submit for ≤64-layer models (vs
        // the old 2/layer). Per-layer write_buffer races are gone (resident norms + accumulating
        // cursors), so KV/work-buffer visibility relies on WebGPU intra-encoder barriers.
        let mut layer_uniform_cursors = Mc8ChunkUniformCursors { attn: 0, elem: 0, gemm: 0 };
        let mut enc = WasmGpuPipeline::begin(self);
        for layer in 0..limit {
            if layer > 0 && (layer % MC8_LAYERS_PER_ENCODER) == 0 {
                self.mc8_flush(&mut enc);
                layer_uniform_cursors.reset();
            }
            let tensors = index.get_layer_tensors(layer);
            let k_info = match tensors.attn_k.as_ref() {
                Some(i) => i,
                None => break,
            };
            let v_info = match tensors.attn_v.as_ref() {
                Some(i) => i,
                None => break,
            };
            let k_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
                Ok(s) => s,
                Err(_) => break,
            };
            let v_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
                Ok(s) => s,
                Err(_) => break,
            };
            let h = index.hyperparams;
            let n_kv = h.effective_n_kv_head();
            let used_attn_norm = tensors.attn_norm.is_some();
            let (uniforms, geom) = match self.mc8_stage_prefill_layer_super_arena(
                index,
                layer,
                &tensors,
                token_idx,
                n_tokens,
                emb_dim,
                used_attn_norm,
                k_info,
                &k_raw,
                v_info,
                &v_raw,
                &mut layer_uniform_cursors,
            ) {
                Some(v) => v,
                None => break,
            };
            let attn_src = if used_attn_norm {
                if let (Some(norm), Some(off)) =
                    (tensors.attn_norm.as_ref(), uniforms.attn_norm_elem_off)
                {
                    let (norm_b, norm_b_off) = match self
                        .mc8_norm_source(mmap, index.tensor_data_start, norm, n_embd, layer, false)
                    {
                        Some(v) => v,
                        None => break,
                    };
                    self.encode_elem_offset(
                        &mut enc,
                        ELEM_OP_RMS_NORM,
                        n_embd as u32,
                        n_tokens,
                        batch_buf,
                        0,
                        geom.batch_in_bytes,
                        norm_b,
                        norm_b_off,
                        geom.n_embd_bytes,
                        prefill_scratch,
                        0,
                        geom.batch_in_bytes,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        off,
                    );
                }
                prefill_scratch
            } else {
                batch_buf
            };
            let n_kv_wg = n_tokens.saturating_mul(n_kv);
            // Phase 5.5: K/V projection on the parallel GEMM → proj buffers; the (now lightweight)
            // attention shader reads the pre-computed projection instead of matmul-ing on 1 thread.
            let kv_dim = (n_kv * h.head_dim()) as usize;
            let kv_proj_bytes = (kv_dim * n_tokens as usize * 4) as wgpu::BufferAddress;
            let k_proj = self.mc8_k_proj_buf.as_ref().unwrap();
            let v_proj = self.mc8_v_proj_buf.as_ref().unwrap();
            if !self.encode_gemm_bufs_offset(
                &mut enc, k_info, k_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                k_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_k_gemm, layer, Mc8WeightRole::AttnK,
            ) {
                break;
            }
            if !self.encode_gemm_bufs_offset(
                &mut enc, v_info, v_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                v_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_v_gemm, layer, Mc8WeightRole::AttnV,
            ) {
                break;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                k_proj,
                token_buf,
                n_embd,
                n_tokens,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                k_info,
                k_raw,
                1,
                n_kv_wg,
                uniforms.k_off,
                Mc8WeightRole::AttnK,
            ) {
                break;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                v_proj,
                token_buf,
                n_embd,
                n_tokens,
                token_idx,
                &layout,
                layer,
                token_idx,
                &h,
                v_info,
                v_raw,
                2,
                n_kv_wg,
                uniforms.v_off,
                Mc8WeightRole::AttnV,
            ) {
                break;
            }
            // Phase 5.4: NO per-layer flush. KV-cache + work-buffer visibility now relies on WebGPU's
            // automatic intra-encoder barriers between compute passes (the per-layer write_buffer
            // races that previously forced a flush are gone: resident norms + accumulating uniforms).
            let work_a = self.prefill_work_buf_a.as_ref().unwrap();
            let work_b = self.prefill_work_buf_b.as_ref().unwrap();
            if !self.encode_prefill_q_ffn_tail_fused(
                &mut enc,
                index,
                layer,
                &tensors,
                batch_buf,
                attn_src,
                work_a,
                work_b,
                n_tokens,
                token_idx,
                emb_dim,
                used_attn_norm,
                &uniforms,
                &geom,
            ) {
                break;
            }
            ran += 1;
        }
        // Phase 5.4: ONE submit for the whole forward (or per chunk if n_layer > MC8_LAYERS_PER_ENCODER).
        self.mc8_flush(&mut enc);
        if ran > 0 && !self.pipeline_read_hidden(emb_dim, hidden).await {
            return 0;
        }
        ran
    }

    /// Topological speculative verify — accept longest draft prefix (B3.1d).
    pub fn verify_topology_draft_batch(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        ctx: &mut Vec<u32>,
        draft: &crate::compute_universe::TopologyDraftBatch,
        emb_dim: usize,
        emb_buf: &mut [f32],
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        max_layers: u32,
        max_vocab_chunks: u32,
    ) -> u32 {
        let mmap = match self.gguf_mmap.clone() {
            Some(m) => m,
            None => return 0,
        };
        let gamma = draft.draft_len as usize;
        if gamma == 0 || ctx.is_empty() {
            return 0;
        }
        let mut accepted = 0u32;
        for i in 0..gamma {
            let cur = *ctx.last().unwrap();
            let token_idx = ctx.len().saturating_sub(1) as u32;
            let hidden_ok = index.dequantize_token_embedding_into(
                mmap.as_ref(),
                cur,
                &mut emb_buf[..emb_dim],
            );
            if hidden_ok == 0 {
                break;
            }
            let _ = self.dispatch_transformer_forward(
                index,
                &mut emb_buf[..emb_dim],
                emb_dim,
                scratch_a,
                scratch_b,
                token_idx,
                max_layers,
            );
            let pred = if let Some(argmax) = self.dispatch_output_argmax_chunked(
                index,
                &emb_buf[..emb_dim],
                emb_dim,
                scratch_a,
                max_vocab_chunks,
                None,
            ) {
                if argmax.max_logit > f32::NEG_INFINITY {
                    argmax.best_token_id
                } else {
                    break;
                }
            } else {
                break;
            };
            if pred != draft.draft_ids[i] {
                break;
            }
            ctx.push(pred);
            accepted += 1;
        }
        accepted
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
            let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
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

            let mut encoder = self
                .device()
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
                let data = slice.get_mapped_range();
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

        let bind_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
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
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[0]);
            cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
        }
        let out_bytes = (n_out * 4) as wgpu::BufferAddress;
        encoder.copy_buffer_to_buffer(output_buf, 0, staging, 0, out_bytes);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::Gemm);

        let slice = staging.slice(..out_bytes);
        if await_wgpu_map(slice).await {
            let data = slice.get_mapped_range();
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
        self.dispatch_gemm_raw_into_async(info, raw, input, out, n_in, n_out).await
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
            wlog(&format!("[attn_pass_async] GUARD unsupported quant kind={proj_kind}"));
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
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
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
            let data = slice.get_mapped_range();
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
#[cfg(target_arch = "wasm32")]
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
            index,
            hidden,
            emb_dim,
            tensors,
            scratch_a,
            scratch_b,
        )
        .await
    }
#[cfg(target_arch = "wasm32")]
    pub async fn dispatch_prefill_chunk_async(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
        max_layers: u32,
        l1_hidden_out: Option<&mut [f32]>,
    ) -> bool {
        if self
            .dispatch_prefill_chunk_async_mc8_gpu(
                index,
                batch_hidden,
                emb_dim,
                n_tokens,
                batch_start_token_idx,
                scratch_a,
                scratch_b,
                max_layers,
                l1_hidden_out,
            )
            .await
        {
            return true;
        }
        wlog("[MC8] GPU prefill FAILED — CPU fallback blocked (manifold unification)");
        false
    }

    /// Part 3u: stage K/V/Q + tail uniforms in three single `write_buffer` calls (pre-encoder).
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn mc8_stage_prefill_layer_super_arena(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        tensors: &crate::gguf_sharder::LayerTensors,
        batch_start_token_idx: u32,
        n_tokens: u32,
        emb_dim: usize,
        used_attn_norm: bool,
        k_info: &GgufTensorInfo,
        k_raw: &[u8],
        v_info: &GgufTensorInfo,
        v_raw: &[u8],
        cursors: &mut Mc8ChunkUniformCursors,
    ) -> Option<(Mc8PrefillLayerUniforms, Mc8PrefillLayerGeom)> {
        let mmap = self.gguf_mmap.as_deref()?;
        let layout = self.kv_layout?;
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let q_info = tensors.attn_q.as_ref()?;
        let gate_info = tensors.ffn_gate.as_ref()?;
        let up_info = tensors.ffn_up.as_ref()?;
        let down_info = tensors.ffn_down.as_ref()?;
        let q_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info).ok()?;
        let n_ffn_est = Self::matmul_dims(gate_info).1;
        let row_stride = Self::mc8_prefill_row_stride(n_embd, n_ffn_est, self.gemm_max_out_dim);
        let emb_bytes = (emb_dim * 4) as wgpu::BufferAddress;
        let n_embd_bytes = (n_embd * 4) as wgpu::BufferAddress;
        let q_dim = (h.n_head * h.head_dim()) as usize;
        let q_bytes = (q_dim * 4) as wgpu::BufferAddress;
        let slot_o = q_bytes;
        let slot_scratch_half = ((row_stride / 2) * 4) as wgpu::BufferAddress;
        let (_, n_ffn) = Self::matmul_dims(gate_info);
        let slot_gate = n_embd_bytes;
        let slot_up = slot_gate + (n_ffn * 4) as wgpu::BufferAddress;
        let slot_save = slot_up + (n_ffn * 4) as wgpu::BufferAddress;
        let row_stride_u32 = row_stride as u32;
        let batch_in_bytes = (n_embd * n_tokens as usize * 4) as wgpu::BufferAddress;
        let work_span_bytes =
            (n_tokens as usize * row_stride * 4).max(q_bytes as usize) as wgpu::BufferAddress;
        let geom = Mc8PrefillLayerGeom {
            row_stride,
            row_stride_u32,
            batch_in_bytes,
            work_span_bytes,
            emb_bytes,
            n_embd_bytes,
            slot_o,
            slot_gate,
            slot_up,
            slot_save,
            slot_scratch_half,
            slot_o_f: (slot_o / 4) as u32,
            slot_save_f: (slot_save / 4) as u32,
            slot_gate_f: (slot_gate / 4) as u32,
            slot_up_f: (slot_up / 4) as u32,
            slot_scratch_half_f: (slot_scratch_half / 4) as u32,
        };
        let (gate_in, _) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > n_embd || up_in != gate_in || up_out != n_ffn || dn_in != n_ffn || dn_out < n_embd {
            return None;
        }
        let out_info = tensors.attn_output.as_ref();
        let (o_in, o_out) = out_info.map(Self::matmul_dims).unwrap_or((q_dim, n_embd));
        if out_info.is_some() && o_in > q_dim {
            return None;
        }
        let o_raw = out_info
            .and_then(|i| crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, i).ok());
        let gate_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info).ok()?;
        let up_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info).ok()?;
        let down_raw = crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info).ok()?;

        let attn_base_byte = cursors.attn_base_byte();
        let elem_base_byte = cursors.elem_base_byte();
        let gemm_base_byte = cursors.gemm_base_byte();
        let mut attn_arena = Mc8AttnUniformArena {
            bytes: [0u8; MC8_MAX_ATTN_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        // Phase 5.5: projection decoupling — the attention shader reads PRE-COMPUTED Q/K/V
        // (parallel GEMM) at `proj_row_stride` floats/token instead of doing the matmul itself.
        let kv_dim = (h.effective_n_kv_head() * h.head_dim()) as u32;
        let q_dim_u32 = (h.n_head * h.head_dim()) as u32;
        let mut k_params = Self::attention_gpu_params(
            &h, &layout, layer, batch_start_token_idx, k_info, k_raw.len(), 1, n_tokens,
            batch_start_token_idx, 0, 0, 0,
        );
        k_params.proj_row_stride = kv_dim;
        let k_off = attn_base_byte + attn_arena.push(&k_params);
        let mut v_params = Self::attention_gpu_params(
            &h, &layout, layer, batch_start_token_idx, v_info, v_raw.len(), 2, n_tokens,
            batch_start_token_idx, 0, 0, 0,
        );
        v_params.proj_row_stride = kv_dim;
        let v_off = attn_base_byte + attn_arena.push(&v_params);
        let mut q_params = Self::attention_gpu_params(
            &h, &layout, layer, batch_start_token_idx, q_info, q_raw.len(), 0, n_tokens,
            batch_start_token_idx, 0, KV_ATTENTION_MASK_WORDS as u32, row_stride_u32,
        );
        q_params.proj_row_stride = q_dim_u32;
        let q_off = attn_base_byte + attn_arena.push(&q_params);

        let mut elem_arena = Mc8ElemUniformArena {
            bytes: [0u8; MC8_MAX_ELEM_UNIFORM_LAYER_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let attn_norm_elem_off = if used_attn_norm {
            Some(elem_base_byte + elem_arena.push(&Self::mc8_elem_params(
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                n_tokens,
                0,
                0,
                0,
                0,
                0,
                0,
            )))
        } else {
            None
        };
        let off_attn_res = elem_base_byte
            + elem_arena.push(&Self::mc8_elem_params(
            ELEM_OP_ADD_RESIDUAL,
            emb_dim as u32,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            0,
            geom.slot_save_f,
            if out_info.is_some() { geom.slot_o_f } else { 0 },
            0,
        ));
        let off_ffn_norm = tensors.ffn_norm.as_ref().map(|_| {
            elem_base_byte + elem_arena.push(&Self::mc8_elem_params(
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                n_tokens,
                0,
                0,
                row_stride_u32,
                0,
                0,
                0,
            ))
        });
        let off_silu = elem_base_byte
            + elem_arena.push(&Self::mc8_elem_params(
            ELEM_OP_SILU_MUL,
            n_ffn as u32,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            row_stride_u32,
            geom.slot_gate_f,
            geom.slot_up_f,
            geom.slot_scratch_half_f,
        ));
        let off_ffn_res = elem_base_byte
            + elem_arena.push(&Self::mc8_elem_params(
            ELEM_OP_ADD_RESIDUAL,
            emb_dim as u32,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            0,
            geom.slot_save_f,
            0,
            0,
        ));

        let mut gemm_arena = Mc8UniformArena {
            bytes: [0u8; MC8_MAX_GEMM_UNIFORM_SLOTS * MC8_UNIFORM_ALIGN],
            slots: 0,
        };
        let off_o = out_info.map(|i| {
            gemm_base_byte
                + gemm_arena.push(&Self::mc8_gemm_params(
                    i,
                    o_raw.as_ref().map(|r| r.len()).unwrap_or(0),
                    o_in,
                    o_out,
                    n_tokens,
                    row_stride_u32,
                    row_stride_u32,
                ))
        });
        let off_gate = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                gate_info,
                gate_raw.len(),
                gate_in,
                n_ffn,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
            ));
        let off_up = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                up_info,
                up_raw.len(),
                up_in,
                n_ffn,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
            ));
        let off_down = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                down_info,
                down_raw.len(),
                dn_in,
                dn_out,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
            ));
        // Phase 5.5: Q/K/V projection GEMMs run on the parallel kernel (input contiguous n_embd/token,
        // output contiguous q_dim|kv_dim/token) — output feeds the now-lightweight attention shader.
        let off_q_gemm = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                q_info, q_raw.len(), n_embd, q_dim, n_tokens, n_embd as u32, q_dim_u32,
            ));
        let off_k_gemm = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                k_info, k_raw.len(), n_embd, kv_dim as usize, n_tokens, n_embd as u32, kv_dim,
            ));
        let off_v_gemm = gemm_base_byte
            + gemm_arena.push(&Self::mc8_gemm_params(
                v_info, v_raw.len(), n_embd, kv_dim as usize, n_tokens, n_embd as u32, kv_dim,
            ));

        let attn_buf = self.attention_params_buf.as_ref()?;
        let elem_buf = self.elem_params_buf.as_ref()?;
        let gemm_buf = self.gemm_params_buf.as_ref()?;
        let queue = self.gpu_queue();
        attn_arena.upload_at(queue, attn_buf, cursors.attn);
        cursors.attn += attn_arena.slots;
        elem_arena.upload_at(queue, elem_buf, cursors.elem);
        cursors.elem += elem_arena.slots;
        gemm_arena.upload_at(queue, gemm_buf, cursors.gemm);
        cursors.gemm += gemm_arena.slots;

        Some((
            Mc8PrefillLayerUniforms {
                k_off,
                v_off,
                q_off,
                attn_norm_elem_off,
                off_o,
                off_attn_res,
                off_ffn_norm,
                off_gate,
                off_up,
                off_silu,
                off_down,
                off_ffn_res,
                off_q_gemm,
                off_k_gemm,
                off_v_gemm,
            },
            geom,
        ))
    }

    /// MC8 Part 3m/3u: batched Q+FFN tail — uniforms pre-staged in super-arena.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn encode_prefill_q_ffn_tail_fused(
        &self,
        pipeline: &mut WasmGpuPipeline,
        index: &crate::gguf_sharder::GgufTensorIndex,
        layer: u32,
        tensors: &crate::gguf_sharder::LayerTensors,
        batch_buf: &wgpu::Buffer,
        attn_src: &wgpu::Buffer,
        work_a: &wgpu::Buffer,
        work_b: &wgpu::Buffer,
        n_tokens: u32,
        batch_start_token_idx: u32,
        emb_dim: usize,
        used_attn_norm: bool,
        uniforms: &Mc8PrefillLayerUniforms,
        geom: &Mc8PrefillLayerGeom,
    ) -> bool {
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let h = index.hyperparams;
        let n_embd = h.n_embd as usize;
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        let q_info = match tensors.attn_q.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let q_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let row_stride = geom.row_stride;
        let row_stride_u32 = geom.row_stride_u32;
        let emb_bytes = geom.emb_bytes;
        let n_embd_bytes = geom.n_embd_bytes;
        let slot_o = geom.slot_o;
        let slot_gate = geom.slot_gate;
        let slot_up = geom.slot_up;
        let slot_save = geom.slot_save;
        let slot_scratch_half = geom.slot_scratch_half;
        let work_span_bytes = geom.work_span_bytes;
        let batch_in_bytes = geom.batch_in_bytes;
        let slot_o_f = geom.slot_o_f;
        let slot_save_f = geom.slot_save_f;
        let slot_gate_f = geom.slot_gate_f;
        let slot_up_f = geom.slot_up_f;
        let slot_scratch_half_f = geom.slot_scratch_half_f;
        let off_attn_res = uniforms.off_attn_res;
        let off_ffn_norm = uniforms.off_ffn_norm;
        let off_gate = uniforms.off_gate;
        let off_up = uniforms.off_up;
        let off_silu = uniforms.off_silu;
        let off_down = uniforms.off_down;
        let off_ffn_res = uniforms.off_ffn_res;
        let off_o = uniforms.off_o;

        let gate_info = match tensors.ffn_gate.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let up_info = match tensors.ffn_up.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let down_info = match tensors.ffn_down.as_ref() {
            Some(i) => i,
            None => return false,
        };
        let (_, n_ffn) = Self::matmul_dims(gate_info);
        let q_dim = (h.n_head * h.head_dim()) as usize;
        let (gate_in, _) = Self::matmul_dims(gate_info);
        let (up_in, up_out) = Self::matmul_dims(up_info);
        let (dn_in, dn_out) = Self::matmul_dims(down_info);
        if gate_in > n_embd || up_in != gate_in || up_out != n_ffn || dn_in != n_ffn || dn_out < n_embd {
            return false;
        }
        let out_info = tensors.attn_output.as_ref();
        let (o_in, o_out) = out_info.map(Self::matmul_dims).unwrap_or((q_dim, n_embd));
        if out_info.is_some() && o_in > q_dim {
            return false;
        }
        let o_raw = out_info.and_then(|out_info| {
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, out_info).ok()
        });
        let gate_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let up_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let down_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_info) {
                Ok(s) => s,
                Err(_) => return false,
            };

        // Phase 5.5: Q projection on the parallel GEMM → q_proj; the (now SDPA-only) attention
        // shader reads it. Moves the heavy 64×960 matmul off the @workgroup_size(1) attention kernel.
        let q_in = if used_attn_norm { attn_src } else { batch_buf };
        let q_proj = self.mc8_q_proj_buf.as_ref().unwrap();
        let q_proj_bytes = (q_dim * n_tokens as usize * 4) as wgpu::BufferAddress;
        if !self.encode_gemm_bufs_offset(
            pipeline, q_info, q_raw, n_embd, q_dim, q_in, 0, batch_in_bytes,
            q_proj, 0, q_proj_bytes, n_tokens, n_embd as u32, q_dim as u32,
            uniforms.off_q_gemm, layer, Mc8WeightRole::AttnQ,
        ) {
            return false;
        }
        // Part 3u: Q SDPA — attn params pre-staged at uniforms.q_off (now reads pre-computed Q).
        if !self.encode_attention_batched_q_prefill(
            pipeline,
            q_proj,
            0,
            q_proj_bytes,
            work_a,
            0,
            work_span_bytes,
            n_embd,
            n_tokens,
            &layout,
            layer,
            &h,
            q_raw,
            uniforms.q_off,
        ) {
            return false;
        }

        // Snapshot pristine hidden (MC8 pt3i `work_aliases_hidden`) before o_proj overwrites paths.
        for t in 0..n_tokens {
            let emb_off = Self::mc8_emb_off(t, n_embd);
            let row_off = Self::mc8_prefill_row_off(t, row_stride);
            pipeline.encoder.copy_buffer_to_buffer(
                batch_buf,
                emb_off,
                work_a,
                row_off + slot_save,
                emb_bytes,
            );
        }

        // Part 3s — attention block: o_proj + attn residual (dynamic uniform offsets).
        if let (Some(out_info), Some(o_raw)) = (out_info, o_raw) {
            if !self.encode_gemm_bufs_offset(
                pipeline,
                out_info,
                o_raw,
                o_in,
                o_out,
                work_a,
                0,
                work_span_bytes,
                work_b,
                slot_o,
                work_span_bytes,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
                off_o.unwrap(),
                layer,
                Mc8WeightRole::OProj,
            ) {
                return false;
            }
            self.encode_elem_offset(
                pipeline,
                ELEM_OP_ADD_RESIDUAL,
                emb_dim as u32,
                n_tokens,
                work_a,
                0,
                work_span_bytes,
                work_b,
                0,
                work_span_bytes,
                batch_buf,
                0,
                batch_in_bytes,
                row_stride_u32,
                row_stride_u32,
                0,
                slot_save_f,
                slot_o_f,
                0,
                off_attn_res,
            );
        } else {
            self.encode_elem_offset(
                pipeline,
                ELEM_OP_ADD_RESIDUAL,
                emb_dim as u32,
                n_tokens,
                work_a,
                0,
                work_span_bytes,
                work_a,
                0,
                work_span_bytes,
                batch_buf,
                0,
                batch_in_bytes,
                row_stride_u32,
                row_stride_u32,
                0,
                slot_save_f,
                0,
                0,
                off_attn_res,
            );
        }

        // Refresh post-attn hidden for FFN residual (overwrites pristine snapshot in slot_save).
        for t in 0..n_tokens {
            let emb_off = Self::mc8_emb_off(t, n_embd);
            let row_off = Self::mc8_prefill_row_off(t, row_stride);
            pipeline.encoder.copy_buffer_to_buffer(
                batch_buf,
                emb_off,
                work_a,
                row_off + slot_save,
                emb_bytes,
            );
        }

        // Part 3s — FFN block: norm + gate/up (one submit via dynamic offsets + weight ping-pong).
        if let Some(norm) = tensors.ffn_norm.as_ref() {
            let (norm_b, norm_b_off) = match self
                .mc8_norm_source(mmap, index.tensor_data_start, norm, n_embd, layer, true)
            {
                Some(v) => v,
                None => return false,
            };
            self.encode_elem_offset(
                pipeline,
                ELEM_OP_RMS_NORM,
                n_embd as u32,
                n_tokens,
                batch_buf,
                0,
                batch_in_bytes,
                norm_b,
                norm_b_off,
                n_embd_bytes,
                work_b,
                0,
                work_span_bytes,
                0,
                0,
                row_stride_u32,
                0,
                0,
                0,
                off_ffn_norm.unwrap(),
            );
        } else {
            for t in 0..n_tokens {
                let emb_off = Self::mc8_emb_off(t, n_embd);
                let row_off = Self::mc8_prefill_row_off(t, row_stride);
                pipeline.encoder.copy_buffer_to_buffer(batch_buf, emb_off, work_b, row_off, emb_bytes);
            }
        }
        // Phase 5 — FFN expansion fusion: collapse gate GEMM + up GEMM + SiLU×mul into a
        // single dispatch. The result `silu(gate·x)·(up·x)` lands in work_b@slot_scratch_half
        // — exactly where the separate SiLU path wrote — so the `down` projection below is
        // unchanged. Requires resident weights (so both roles are GPU-resident) and gate/up
        // sharing ggml_type+dims (one staged GemmParams describes both). Otherwise fall back
        // to the proven 3-dispatch path.
        let fuse_ffn_expansion =
            self.mc8_weights_resident && gate_info.ggml_type == up_info.ggml_type;
        if fuse_ffn_expansion {
            // WebGPU forbids binding one buffer as both read-only and writable storage within a
            // single pass (validated at buffer granularity, not by offset). The norm wrote the
            // hidden to work_b@0, and the fused intermediate must land in work_b@slot_scratch_half
            // for `down` below — same buffer, so reading + writing work_b in one pass is illegal.
            // Stage the normalized hidden into work_a@0 (small n_embd copy; `down` overwrites
            // work_a@0 immediately after), read it there, and write the silu·up result to
            // work_b@slot_scratch_half exactly as the separate-SiLU path did — `down` unchanged.
            for t in 0..n_tokens {
                let row_off = Self::mc8_prefill_row_off(t, row_stride);
                pipeline.encoder.copy_buffer_to_buffer(work_b, row_off, work_a, row_off, emb_bytes);
            }
            if !self.encode_fused_ffn_expansion(
                pipeline,
                layer,
                work_a,
                0,
                work_span_bytes,
                work_b,
                slot_scratch_half,
                work_span_bytes,
                n_ffn as u32,
                n_tokens,
                off_gate,
            ) {
                return false;
            }
        } else {
            if !self.encode_gemm_bufs_offset(
                pipeline,
                gate_info,
                gate_raw,
                gate_in,
                n_ffn,
                work_b,
                0,
                work_span_bytes,
                work_a,
                slot_gate,
                work_span_bytes,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
                off_gate,
                layer,
                Mc8WeightRole::Gate,
            ) {
                return false;
            }
            if !self.encode_gemm_bufs_offset(
                pipeline,
                up_info,
                up_raw,
                up_in,
                n_ffn,
                work_b,
                0,
                work_span_bytes,
                work_a,
                slot_up,
                work_span_bytes,
                n_tokens,
                row_stride_u32,
                row_stride_u32,
                off_up,
                layer,
                Mc8WeightRole::Up,
            ) {
                return false;
            }

            self.encode_elem_offset(
                pipeline,
                ELEM_OP_SILU_MUL,
                n_ffn as u32,
                n_tokens,
                work_a,
                0,
                work_span_bytes,
                work_a,
                0,
                work_span_bytes,
                work_b,
                0,
                work_span_bytes,
                row_stride_u32,
                row_stride_u32,
                row_stride_u32,
                slot_gate_f,
                slot_up_f,
                slot_scratch_half_f,
                off_silu,
            );
        }
        if !self.encode_gemm_bufs_offset(
            pipeline,
            down_info,
            down_raw,
            dn_in,
            dn_out,
            work_b,
            slot_scratch_half,
            work_span_bytes,
            work_a,
            0,
            work_span_bytes,
            n_tokens,
            row_stride_u32,
            row_stride_u32,
            off_down,
            layer,
            Mc8WeightRole::Down,
        ) {
            return false;
        }
        self.encode_elem_offset(
            pipeline,
            ELEM_OP_ADD_RESIDUAL,
            emb_dim as u32,
            n_tokens,
            work_a,
            0,
            work_span_bytes,
            work_a,
            0,
            work_span_bytes,
            batch_buf,
            0,
            batch_in_bytes,
            row_stride_u32,
            row_stride_u32,
            0,
            slot_save_f,
            0,
            0,
            off_ffn_res,
        );
        true
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) async fn dispatch_prefill_chunk_async_mc8_gpu(
        &mut self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        batch_hidden: &mut [f32],
        emb_dim: usize,
        n_tokens: u32,
        batch_start_token_idx: u32,
        _scratch_a: &mut [f32],
        _scratch_b: &mut [f32],
        max_layers: u32,
        mut l1_hidden_out: Option<&mut [f32]>,
    ) -> bool {
        let n_layer = index.hyperparams.n_layer;
        if n_layer == 0 || n_tokens == 0 || !self.mc8_buffers_ready() {
            return false;
        }
        // Part 3x: upload all layer weights to GPU once (idempotent; falls back if it fails).
        if !self.mc8_weights_resident {
            let _ = self.mc8_upload_all_resident_weights(index);
        }
        let prefill_scratch = match self.prefill_scratch_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        let limit = if max_layers == 0 {
            n_layer
        } else {
            max_layers.min(n_layer)
        };
        let n_embd = index.hyperparams.n_embd as usize;
        let batch_elems = n_embd * n_tokens as usize;
        if batch_elems > batch_hidden.len() || emb_dim < n_embd {
            return false;
        }
        let batch_buf = self.gemm_input_buf.as_ref().unwrap();
        let token_buf = self.gemm_output_buf.as_ref().unwrap();
        let norm_buf = self.norm_weight_buf.as_ref().unwrap();
        if batch_elems > self.gemm_max_input_floats {
            return false;
        }
        self.gpu_queue().write_buffer(
            batch_buf,
            0,
            bytemuck::cast_slice(&batch_hidden[..batch_elems]),
        );
        let mmap = match self.gguf_mmap.as_deref() {
            Some(m) => m,
            None => return false,
        };
        let layout = match self.kv_layout {
            Some(l) => l,
            None => return false,
        };
        for layer in 0..limit {
            let tensors = index.get_layer_tensors(layer);
            let k_info = match tensors.attn_k.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let v_info = match tensors.attn_v.as_ref() {
                Some(i) => i,
                None => return false,
            };
            let k_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info)
            {
                Ok(s) => s,
                Err(_) => return false,
            };
            let v_raw = match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info)
            {
                Ok(s) => s,
                Err(_) => return false,
            };
            let h = index.hyperparams;
            let n_kv = h.effective_n_kv_head();
            let used_attn_norm = tensors.attn_norm.is_some();
            let mut layer_uniform_cursors = Mc8ChunkUniformCursors {
                attn: 0,
                elem: 0,
                gemm: 0,
            };
            let (uniforms, geom) = match self.mc8_stage_prefill_layer_super_arena(
                index,
                layer,
                &tensors,
                batch_start_token_idx,
                n_tokens,
                emb_dim,
                used_attn_norm,
                k_info,
                &k_raw,
                v_info,
                &v_raw,
                &mut layer_uniform_cursors,
            ) {
                Some(v) => v,
                None => return false,
            };
            let mut enc = WasmGpuPipeline::begin(self);
            let attn_src = if used_attn_norm {
                if let (Some(norm), Some(off)) =
                    (tensors.attn_norm.as_ref(), uniforms.attn_norm_elem_off)
                {
                    let (norm_b, norm_b_off) = match self
                        .mc8_norm_source(mmap, index.tensor_data_start, norm, n_embd, layer, false)
                    {
                        Some(v) => v,
                        None => return false,
                    };
                    self.encode_elem_offset(
                        &mut enc,
                        ELEM_OP_RMS_NORM,
                        n_embd as u32,
                        n_tokens,
                        batch_buf,
                        0,
                        geom.batch_in_bytes,
                        norm_b,
                        norm_b_off,
                        geom.n_embd_bytes,
                        prefill_scratch,
                        0,
                        geom.batch_in_bytes,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        off,
                    );
                }
                prefill_scratch
            } else {
                batch_buf
            };
            let n_kv_wg = n_tokens.saturating_mul(n_kv);
            // Phase 5.5: K/V projection on the parallel GEMM → proj buffers (attention reads them).
            let kv_dim = (n_kv * h.head_dim()) as usize;
            let kv_proj_bytes = (kv_dim * n_tokens as usize * 4) as wgpu::BufferAddress;
            let k_proj = self.mc8_k_proj_buf.as_ref().unwrap();
            let v_proj = self.mc8_v_proj_buf.as_ref().unwrap();
            if !self.encode_gemm_bufs_offset(
                &mut enc, k_info, k_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                k_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_k_gemm, layer, Mc8WeightRole::AttnK,
            ) {
                return false;
            }
            if !self.encode_gemm_bufs_offset(
                &mut enc, v_info, v_raw, n_embd, kv_dim, attn_src, 0, geom.batch_in_bytes,
                v_proj, 0, kv_proj_bytes, n_tokens, n_embd as u32, kv_dim as u32,
                uniforms.off_v_gemm, layer, Mc8WeightRole::AttnV,
            ) {
                return false;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                k_proj,
                token_buf,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                k_info,
                k_raw,
                1,
                n_kv_wg,
                uniforms.k_off,
                Mc8WeightRole::AttnK,
            ) {
                return false;
            }
            if !self.encode_attention_pass_gpu(
                &mut enc,
                v_proj,
                token_buf,
                n_embd,
                n_tokens,
                batch_start_token_idx,
                &layout,
                layer,
                batch_start_token_idx,
                &h,
                v_info,
                v_raw,
                2,
                n_kv_wg,
                uniforms.v_off,
                Mc8WeightRole::AttnV,
            ) {
                return false;
            }
            // Part 3u: KV cache writes must be queue-visible before Q-SDPA reads (backend empirical).
            self.mc8_flush(&mut enc);
            const MC8_FUSED_PREFILL_TAIL: bool = true;
            let use_fused_tail = MC8_FUSED_PREFILL_TAIL
                && self.prefill_work_buf_a.is_some()
                && self.prefill_work_buf_b.is_some();
            if use_fused_tail {
                let work_a = self.prefill_work_buf_a.as_ref().unwrap();
                let work_b = self.prefill_work_buf_b.as_ref().unwrap();
                if !self.encode_prefill_q_ffn_tail_fused(
                    &mut enc,
                    index,
                    layer,
                    &tensors,
                    batch_buf,
                    attn_src,
                    work_a,
                    work_b,
                    n_tokens,
                    batch_start_token_idx,
                    emb_dim,
                    used_attn_norm,
                    &uniforms,
                    &geom,
                ) {
                    return false;
                }
            } else {
                let token_buf = self.gemm_output_buf.as_ref().unwrap();
                let aux_buf = self.gemm_aux_buf.as_ref().unwrap();
                let token_bytes = (n_embd * 4) as wgpu::BufferAddress;
                for t in 0..n_tokens {
                    let abs = batch_start_token_idx + t;
                    let off = Self::mc8_emb_off(t, n_embd);
                    enc.encoder.copy_buffer_to_buffer(batch_buf, off, token_buf, 0, token_bytes);
                    let attn_in = if used_attn_norm {
                        enc.encoder.copy_buffer_to_buffer(
                            prefill_scratch,
                            off,
                            aux_buf,
                            0,
                            token_bytes,
                        );
                        self.mc8_flush(&mut enc);
                        Some(aux_buf)
                    } else {
                        None
                    };
                    if !self.encode_attn_ffn_tail_gpu(
                        &mut enc,
                        index,
                        layer,
                        abs,
                        emb_dim,
                        &tensors,
                        token_buf,
                        attn_in,
                        true,
                    ) {
                        return false;
                    }
                    self.mc8_flush(&mut enc);
                    enc.encoder.copy_buffer_to_buffer(token_buf, 0, batch_buf, off, token_bytes);
                    self.mc8_flush(&mut enc);
                }
            }
            // Part 3u: one submit per layer (KV flush + layer-end flush = 2 submits/layer).
            self.mc8_flush(&mut enc);
            let _ = l1_hidden_out.take();
        }
        wlog(&format!(
            "[MC8] GPU prefill OK layers={limit} n_tokens={n_tokens} start={batch_start_token_idx}"
        ));
        true
    }
#[cfg(target_arch = "wasm32")]
    pub async fn dispatch_fused_transformer_block_async(
        &self,
        tensor: &QTensor,
        input_activations: &[f32],
    ) -> Vec<f32> {
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
                    let weights_f32 =
                        crate::directml_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                    let op = crate::directml_bridge::DmlGemmOp {
                        m: input_activations.len() as u32 / cols as u32,
                        k: cols as u32,
                        n: rows as u32,
                    };
                    if let Ok(result) = op.execute(dml, input_activations, &weights_f32) {
                        crate::telemetry::SIEVE_OPS_COUNT
                            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
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
                let weights_f32 =
                    crate::metal_bridge::dequantize_q4_k_tensor(q4_slice, rows * cols);
                let input_rows = (input_activations.len() / cols).max(1);
                let result = crate::metal_bridge::accelerate_sgemm(
                    input_rows,
                    cols,
                    rows,
                    input_activations,
                    &weights_f32,
                );
                crate::telemetry::SIEVE_OPS_COUNT
                    .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
                return result;
            }
        }

        // ── wgpu / WGSL fallback (all platforms — Vulkan on Linux/NVIDIA,
        //    Metal on macOS when mmap not loaded, D3D12 on Windows fallback) ──
        let input_bytes = bytemuck::cast_slice(input_activations);
        let input_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Input"),
            size: input_bytes.len().max(4) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&input_buf, 0, input_bytes);

        // Upload real weights from mmap when available, else use a zero buffer.
        let weights_size = (rows * cols * 4) as wgpu::BufferAddress;
        let weights_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Weights"),
            size: weights_size.max(4),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        if let Some(mmap) = &self.gguf_mmap {
            let offset = (self.tensor_data_offset + tensor.byte_offset) as usize;
            let end = (offset + rows * cols * 4).min(mmap.len());
            if end > offset {
                let f32_bytes = &mmap[offset..end];
                self.gpu_queue().write_buffer(&weights_buf, 0, f32_bytes);
            }
        }

        let output_size = (rows * 4).max(4) as wgpu::BufferAddress;
        let output_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Upload GemmGpuParams for fused_transformer.wgsl (binding 2).
        let gemm_params = GemmGpuParams {
            n_in: cols as u32,
            n_out: rows as u32,
            weight_ggml_type: if tensor.is_quantized_q4_k { 12 } else { 14 },
            weight_row_elems: cols as u32,
            weight_byte_len: (rows * cols * 4) as u32,
            n_batch: 1,
            in_row_stride: 0,
            out_row_stride: 0,
        };
        let params_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("TransformerParams"),
            size: std::mem::size_of::<GemmGpuParams>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.gpu_queue().write_buffer(&params_buf, 0, bytemuck::bytes_of(&gemm_params));

        let bind_group_layout = self.pipeline.get_bind_group_layout(0);
        let bind_group = self.gpu_device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weights_buf.as_entire_binding(),
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

        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: crate::llm_gpu_profiler::pass_writes_both(),
            });
            cpass.set_pipeline(&self.pipeline);
            cpass.set_bind_group(0, &bind_group, &[]);
            cpass.dispatch_workgroups((rows as u32 + 63) / 64, 1, 1);
        }

        let staging_buf = self.gpu_device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: output_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size);
        crate::llm_gpu_profiler::resolve(&mut encoder);
        self.gpu_queue().submit(Some(encoder.finish()));
        crate::llm_gpu_profiler::accumulate(crate::llm_gpu_profiler::Phase::FusedBlock);

        let buffer_slice = staging_buf.slice(..);
        let (sender, receiver) = futures_channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());
        

        receiver.await.unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f32> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        crate::telemetry::SIEVE_OPS_COUNT
            .fetch_add(rows * cols, std::sync::atomic::Ordering::Relaxed);
        result
    }
#[cfg(target_arch = "wasm32")]
    pub async fn dispatch_output_argmax_chunked_async(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        // Part 3l: per-chunk async GPU vocab GEMM + streaming CPU argmax (no WGSL argmax shader).
        self.dispatch_output_argmax_chunked_async_mc8_fused(
            index,
            hidden,
            emb_dim,
            chunk_logits,
            max_chunks,
            sieve_mask,
        )
        .await
    }

    #[cfg(target_arch = "wasm32")]
    /// Per-chunk `dispatch_gemm_raw_into_async` + streaming CPU argmax.
    ///
    /// There is no WGSL argmax reduction shader. The Endgame "fused" batched encoder
    /// (`encode_gemm_bufs` loop + single staging readback) produced garbled tokens because
    /// `queue.write_buffer` weight uploads race across chunks in one submit scope (pt3c
    /// analogue). Per-chunk submit/readback matches the proven sync path semantics.
    pub(crate) async fn dispatch_output_argmax_chunked_async_mc8_fused(
        &self,
        index: &crate::gguf_sharder::GgufTensorIndex,
        hidden: &[f32],
        emb_dim: usize,
        chunk_logits: &mut [f32],
        max_chunks: u32,
        sieve_mask: Option<&crate::neuro_symbolic_sieve::SieveStateMask>,
    ) -> Option<StreamingArgmaxResult> {
        let info = index.logits_projection_info()?;
        let (n_in, vocab_size) = Self::matmul_dims(info);
        if n_in == 0 || vocab_size == 0 || n_in > emb_dim || n_in > hidden.len() {
            return None;
        }
        if chunk_logits.len() < VOCAB_CHUNK_ROWS || !self.mc8_buffers_ready() {
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
            // Phase 5.3: bind the resident output-projection sub-range (zero per-token upload)
            // when available; otherwise fall back to per-chunk fetch + write_buffer. The chunk
            // byte offset is 256-aligned because VOCAB_CHUNK_ROWS is a multiple of 256.
            let ok = if let Some(buf) = self.mc8_logits_resident_buf.as_ref() {
                let row_bytes = self.mc8_logits_row_bytes as u64;
                let weight = wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: buf,
                    offset: row_start as u64 * row_bytes,
                    size: std::num::NonZeroU64::new(chunk_rows as u64 * row_bytes),
                });
                self.dispatch_gemm_resident_chunk_async(
                    info.ggml_type,
                    info.dims[0] as u32,
                    weight,
                    (chunk_rows as u64 * row_bytes) as u32,
                    &hidden[..n_in],
                    &mut chunk_logits[..chunk_rows],
                    n_in,
                    chunk_rows,
                )
                .await
            } else {
                let raw = crate::ggml_quants::fetch_tensor_row_range_bytes(
                    mmap,
                    index.tensor_data_start,
                    info,
                    row_start,
                    chunk_rows,
                )
                .ok()?;
                self.dispatch_gemm_raw_into_async(
                    info,
                    raw,
                    &hidden[..n_in],
                    &mut chunk_logits[..chunk_rows],
                    n_in,
                    chunk_rows,
                )
                .await
            };
            if !ok {
                return None;
            }
            update_streaming_argmax_sieved(
                &chunk_logits[..chunk_rows],
                chunk_rows,
                chunk_idx,
                sieve_mask,
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


#[cfg(target_arch = "wasm32")]
    pub async fn new_async() -> Self {
        Self::try_new().await.expect("Failed to initialize native GGUF engine")
    }
}
