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
                wlog(&format!(
                    "[prefill_layer] FAILED missing attn_k layer={layer}"
                ));
                return false;
            }
        };
        let v_info = match tensors.attn_v.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!(
                    "[prefill_layer] FAILED missing attn_v layer={layer}"
                ));
                return false;
            }
        };
        if tensors.attn_q.is_none() {
            wlog(&format!(
                "[prefill_layer] FAILED missing attn_q layer={layer}"
            ));
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
                wlog(&format!(
                    "[prefill_layer] q_ffn FAILED layer={layer} t={t} abs={abs}"
                ));
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
                wlog(&format!(
                    "[prefill_layer] FAILED missing attn_k layer={layer}"
                ));
                return false;
            }
        };
        let v_info = match tensors.attn_v.as_ref() {
            Some(i) => i,
            None => {
                wlog(&format!(
                    "[prefill_layer] FAILED missing attn_v layer={layer}"
                ));
                return false;
            }
        };
        if tensors.attn_q.is_none() {
            wlog(&format!(
                "[prefill_layer] FAILED missing attn_q layer={layer}"
            ));
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
            let n =
                dequant_norm_row_into(mmap, index.tensor_data_start, norm_info, &mut norm_w_attn);
            if n >= n_embd {
                for t in 0..n_tokens as usize {
                    let off = t * n_embd;
                    norm_scratch[off..off + n_embd]
                        .copy_from_slice(&batch_hidden[off..off + n_embd]);
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
                wlog(&format!(
                    "[prefill_layer] q_ffn FAILED layer={layer} t={t} abs={abs}"
                ));
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
        // W3: resident single-fence-per-chunk arena (toggle-gated, default OFF). Populates the KV
        // cache for the whole chunk in ONE submit; any ineligibility falls back to the legacy loop.
        #[cfg(not(target_arch = "wasm32"))]
        if crate::llm_bench::resident_prefill_enabled() {
            if self
                .dispatch_prefill_chunk_resident(
                    index,
                    &batch_hidden[..],
                    emb_dim,
                    n_tokens,
                    batch_start_token_idx,
                    max_layers,
                )
                .is_some()
            {
                crate::llm_bench::record_resident_prefill_hit();
                return true;
            }
            crate::llm_bench::record_resident_prefill_fallback();
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
        // Native-only: `llm_bench` is `#[cfg(not(wasm32))]`; the wasm-full LLM bundle skips profiling.
        #[cfg(not(target_arch = "wasm32"))]
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

        #[cfg(not(target_arch = "wasm32"))]
        crate::llm_bench::add_decode_attn_ns(t_attn.elapsed().as_nanos() as u64);

        if !attn_ok && tensors.attn_output.is_none() && tensors.ffn_gate.is_none() {
            return false;
        }

        // #48 diagnostic: localize the residual explosion — attention output magnitude per layer.
        if layer < 3 && std::env::var("QUALIA_LLM_DEBUG_DECODE").is_ok() {
            let max_attn = scratch_a[..emb_dim]
                .iter()
                .fold(0f32, |m, &v| m.max(v.abs()));
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

        #[cfg(not(target_arch = "wasm32"))]
        let t_ffn = std::time::Instant::now();
        let ffn_ok = self
            .dispatch_ffn_block_pre_norm(index, hidden, emb_dim, &tensors, scratch_a, scratch_b);
        #[cfg(not(target_arch = "wasm32"))]
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
                    .dispatch_gemm_into_async(index, &info, &hidden[..n_in], scratch_a, n_in, n_out)
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
            index, hidden, emb_dim, &tensors, scratch_a, scratch_b,
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
        let q_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, q_info) {
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
            let o_raw = match crate::ggml_quants::fetch_tensor_bytes(
                mmap,
                index.tensor_data_start,
                out_info,
            ) {
                Ok(s) => s,
                Err(_) => return false,
            };
            if work_aliases_hidden {
                pipeline
                    .encoder
                    .copy_buffer_to_buffer(token_hidden, 0, aux_buf, 0, emb_bytes);
                self.mc8_flush(pipeline);
            }
            if o_in > q_dim
                || !self.encode_gemm_bufs(pipeline, out_info, o_raw, o_in, o_out, ffn_buf, work_buf)
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
        if gate_in > n_embd
            || up_in != gate_in
            || up_out != n_ffn
            || dn_in != n_ffn
            || dn_out < n_embd
        {
            return false;
        }
        let gate_raw = match crate::ggml_quants::fetch_tensor_bytes(
            mmap,
            index.tensor_data_start,
            gate_info,
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let up_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, up_info) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let down_raw = match crate::ggml_quants::fetch_tensor_bytes(
            mmap,
            index.tensor_data_start,
            down_info,
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let base_save = match self.prefill_scratch_buf.as_ref() {
            Some(b) => b,
            None => return false,
        };
        pipeline
            .encoder
            .copy_buffer_to_buffer(token_hidden, 0, base_save, 0, emb_bytes);
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
            pipeline
                .encoder
                .copy_buffer_to_buffer(token_hidden, 0, aux_buf, 0, emb_bytes);
        }
        self.mc8_flush(pipeline);
        if !self.encode_gemm_bufs(
            pipeline, gate_info, gate_raw, gate_in, n_ffn, aux_buf, work_buf,
        ) {
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
        if !self.encode_gemm_bufs(
            pipeline, down_info, down_raw, dn_in, dn_out, aux_buf, work_buf,
        ) {
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
        let k_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info) {
                Ok(s) => s,
                Err(_) => return false,
            };
        let v_raw =
            match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info) {
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
        self.gpu_queue()
            .write_buffer(batch_buf, 0, bytemuck::cast_slice(&hidden[..n_embd]));
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
        let mut layer_uniform_cursors = Mc8ChunkUniformCursors {
            attn: 0,
            elem: 0,
            gemm: 0,
        };
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
            let k_raw =
                match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, k_info)
                {
                    Ok(s) => s,
                    Err(_) => break,
                };
            let v_raw =
                match crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, v_info)
                {
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
                    let (norm_b, norm_b_off) = match self.mc8_norm_source(
                        mmap,
                        index.tensor_data_start,
                        norm,
                        n_embd,
                        layer,
                        false,
                    ) {
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
                &mut enc,
                k_info,
                k_raw,
                n_embd,
                kv_dim,
                attn_src,
                0,
                geom.batch_in_bytes,
                k_proj,
                0,
                kv_proj_bytes,
                n_tokens,
                n_embd as u32,
                kv_dim as u32,
                uniforms.off_k_gemm,
                layer,
                Mc8WeightRole::AttnK,
            ) {
                break;
            }
            if !self.encode_gemm_bufs_offset(
                &mut enc,
                v_info,
                v_raw,
                n_embd,
                kv_dim,
                attn_src,
                0,
                geom.batch_in_bytes,
                v_proj,
                0,
                kv_proj_bytes,
                n_tokens,
                n_embd as u32,
                kv_dim as u32,
                uniforms.off_v_gemm,
                layer,
                Mc8WeightRole::AttnV,
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
            let hidden_ok =
                index.dequantize_token_embedding_into(mmap.as_ref(), cur, &mut emb_buf[..emb_dim]);
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
}
