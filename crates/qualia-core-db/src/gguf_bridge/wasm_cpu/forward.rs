use crate::ggml_quants::fetch_tensor_bytes;
use crate::gguf_sharder::GgufTensorInfo;

use super::{CpuWasmEngine, CpuWasmError, CpuWasmStep};
use crate::gguf_bridge::{
    dequant_norm_row_into, rms_norm_inplace, rope_inplace, stack_gemm_quant, RMS_NORM_EPS,
};

#[inline]
fn dims(info: &GgufTensorInfo) -> (usize, usize) {
    (
        info.dims[0] as usize,
        if info.n_dims > 1 && info.dims[1] > 0 {
            info.dims[1] as usize
        } else {
            1
        },
    )
}

fn gemv(
    model: &[u8],
    tensor_data_start: u64,
    layer: u32,
    role: &'static str,
    info: &GgufTensorInfo,
    input: &[f32],
    output: &mut [f32],
) -> Result<usize, CpuWasmError> {
    let (n_in, n_out) = dims(info);
    if n_in > input.len() || n_out > output.len() {
        return Err(CpuWasmError::KernelFailed { layer, role });
    }
    let raw = fetch_tensor_bytes(model, tensor_data_start, info)
        .map_err(|_| CpuWasmError::InvalidModel("tensor payload outside model bytes".into()))?;
    if !stack_gemm_quant(raw, info, input, output, n_in, n_out) {
        return Err(CpuWasmError::KernelFailed { layer, role });
    }
    Ok(n_out)
}

impl CpuWasmEngine {
    #[inline]
    fn kv_index(&self, value: bool, layer: usize, pos: usize, head: usize, dim: usize) -> usize {
        let plane = if value { self.kv_plane_elems } else { 0 };
        plane + ((((layer * self.max_context + pos) * self.n_kv_head + head) * self.head_dim) + dim)
    }

    fn norm_hidden(&mut self, info: Option<&GgufTensorInfo>) -> Result<(), CpuWasmError> {
        self.normed[..self.n_embd].copy_from_slice(&self.hidden[..self.n_embd]);
        let Some(info) = info else {
            return Ok(());
        };
        if dequant_norm_row_into(
            &self.model,
            self.index.tensor_data_start,
            info,
            &mut self.norm_weight,
        ) < self.n_embd
        {
            return Err(CpuWasmError::InvalidModel(
                "norm tensor decode failed".into(),
            ));
        }
        rms_norm_inplace(
            &mut self.normed[..self.n_embd],
            &self.norm_weight[..self.n_embd],
            RMS_NORM_EPS,
        );
        Ok(())
    }

    fn forward_layer(&mut self, layer: usize, position: usize) -> Result<(), CpuWasmError> {
        let layer_u32 = layer as u32;
        let tensors = self.index.get_layer_tensors(layer_u32);
        let q_info = tensors.attn_q.ok_or(CpuWasmError::MissingTensor {
            layer: layer_u32,
            role: "attn_q.weight",
        })?;
        let k_info = tensors.attn_k.ok_or(CpuWasmError::MissingTensor {
            layer: layer_u32,
            role: "attn_k.weight",
        })?;
        let v_info = tensors.attn_v.ok_or(CpuWasmError::MissingTensor {
            layer: layer_u32,
            role: "attn_v.weight",
        })?;
        let o_info = tensors.attn_output.ok_or(CpuWasmError::MissingTensor {
            layer: layer_u32,
            role: "attn_output.weight",
        })?;

        self.norm_hidden(tensors.attn_norm.as_ref())?;
        let data_start = self.index.tensor_data_start;
        let qn = gemv(
            &self.model,
            data_start,
            layer_u32,
            "attn_q.weight",
            &q_info,
            &self.normed,
            &mut self.q,
        )?;
        let kn = gemv(
            &self.model,
            data_start,
            layer_u32,
            "attn_k.weight",
            &k_info,
            &self.normed,
            &mut self.k,
        )?;
        let vn = gemv(
            &self.model,
            data_start,
            layer_u32,
            "attn_v.weight",
            &v_info,
            &self.normed,
            &mut self.v,
        )?;
        if qn != self.n_head * self.head_dim || kn != self.n_kv_head * self.head_dim || vn != kn {
            return Err(CpuWasmError::KernelFailed {
                layer: layer_u32,
                role: "attention dimensions",
            });
        }
        let h = self.index.hyperparams;
        rope_inplace(
            &mut self.q[..qn],
            self.n_head,
            self.head_dim,
            position as u32,
            h.effective_rope_freq_base(),
            h.effective_rope_scale(),
        );
        rope_inplace(
            &mut self.k[..kn],
            self.n_kv_head,
            self.head_dim,
            position as u32,
            h.effective_rope_freq_base(),
            h.effective_rope_scale(),
        );
        for head in 0..self.n_kv_head {
            for dim in 0..self.head_dim {
                let src = head * self.head_dim + dim;
                let ki = self.kv_index(false, layer, position, head, dim);
                let vi = self.kv_index(true, layer, position, head, dim);
                self.kv[ki] = self.k[src];
                self.kv[vi] = self.v[src];
            }
        }

        self.attention[..self.n_embd].fill(0.0);
        let q_per_kv = self.n_head / self.n_kv_head;
        let scale = 1.0 / (self.head_dim as f32).sqrt();
        for q_head in 0..self.n_head {
            let kv_head = q_head / q_per_kv;
            let q_off = q_head * self.head_dim;
            let mut max_score = f32::NEG_INFINITY;
            for past in 0..=position {
                let mut dot = 0.0f32;
                for dim in 0..self.head_dim {
                    dot += self.q[q_off + dim]
                        * self.kv[self.kv_index(false, layer, past, kv_head, dim)];
                }
                let score = dot * scale;
                self.scores[past] = score;
                max_score = max_score.max(score);
            }
            let mut sum = 0.0f32;
            for past in 0..=position {
                let value = (self.scores[past] - max_score).exp();
                self.scores[past] = value;
                sum += value;
            }
            if !sum.is_finite() || sum <= 0.0 {
                return Err(CpuWasmError::KernelFailed {
                    layer: layer_u32,
                    role: "attention softmax",
                });
            }
            for past in 0..=position {
                let probability = self.scores[past] / sum;
                for dim in 0..self.head_dim {
                    self.attention[q_off + dim] +=
                        probability * self.kv[self.kv_index(true, layer, past, kv_head, dim)];
                }
            }
        }

        let on = gemv(
            &self.model,
            data_start,
            layer_u32,
            "attn_output.weight",
            &o_info,
            &self.attention,
            &mut self.projection,
        )?;
        if on < self.n_embd {
            return Err(CpuWasmError::KernelFailed {
                layer: layer_u32,
                role: "attn_output dimensions",
            });
        }
        for i in 0..self.n_embd {
            self.hidden[i] += self.projection[i];
        }

        self.norm_hidden(tensors.ffn_norm.as_ref())?;
        let gate_info = tensors.ffn_gate.ok_or(CpuWasmError::MissingTensor {
            layer: layer_u32,
            role: "ffn_gate.weight",
        })?;
        let up_info = tensors.ffn_up.ok_or(CpuWasmError::MissingTensor {
            layer: layer_u32,
            role: "ffn_up.weight",
        })?;
        let down_info = tensors.ffn_down.ok_or(CpuWasmError::MissingTensor {
            layer: layer_u32,
            role: "ffn_down.weight",
        })?;
        let gate_n = gemv(
            &self.model,
            data_start,
            layer_u32,
            "ffn_gate.weight",
            &gate_info,
            &self.normed,
            &mut self.gate,
        )?;
        let up_n = gemv(
            &self.model,
            data_start,
            layer_u32,
            "ffn_up.weight",
            &up_info,
            &self.normed,
            &mut self.up,
        )?;
        if gate_n != self.n_ffn || up_n != gate_n {
            return Err(CpuWasmError::KernelFailed {
                layer: layer_u32,
                role: "FFN dimensions",
            });
        }
        for i in 0..self.n_ffn {
            let g = self.gate[i];
            self.gate[i] = (g / (1.0 + (-g).exp())) * self.up[i];
        }
        let down_n = gemv(
            &self.model,
            data_start,
            layer_u32,
            "ffn_down.weight",
            &down_info,
            &self.gate,
            &mut self.projection,
        )?;
        if down_n < self.n_embd {
            return Err(CpuWasmError::KernelFailed {
                layer: layer_u32,
                role: "ffn_down dimensions",
            });
        }
        for i in 0..self.n_embd {
            self.hidden[i] += self.projection[i];
        }
        Ok(())
    }

    fn forward_hidden(&mut self, token_id: u32, position: usize) -> Result<(), CpuWasmError> {
        if position >= self.max_context {
            return Err(CpuWasmError::ContextExceeded {
                position,
                max_context: self.max_context,
            });
        }
        if self.index.dequantize_token_embedding_into(
            &self.model,
            token_id,
            &mut self.hidden[..self.n_embd],
        ) != self.n_embd
        {
            return Err(CpuWasmError::InvalidModel(format!(
                "embedding lookup failed for token {token_id}"
            )));
        }
        for layer in 0..self.n_layer {
            self.forward_layer(layer, position)?;
        }
        Ok(())
    }

    /// Populate the KV cache for one prompt token without wasting a vocabulary
    /// projection. This is the CPU-WASM prefill primitive.
    pub fn ingest_token(&mut self, token_id: u32, position: usize) -> Result<(), CpuWasmError> {
        self.forward_hidden(token_id, position)
    }

    /// Execute one complete autoregressive transformer step without allocation.
    pub fn run_token(
        &mut self,
        token_id: u32,
        position: usize,
    ) -> Result<CpuWasmStep, CpuWasmError> {
        self.forward_hidden(token_id, position)?;
        if let Some(info) = self.index.output_norm_info() {
            if dequant_norm_row_into(
                &self.model,
                self.index.tensor_data_start,
                info,
                &mut self.norm_weight,
            ) < self.n_embd
            {
                return Err(CpuWasmError::InvalidModel(
                    "output norm decode failed".into(),
                ));
            }
            rms_norm_inplace(
                &mut self.hidden[..self.n_embd],
                &self.norm_weight[..self.n_embd],
                RMS_NORM_EPS,
            );
        }
        let output = *self
            .index
            .logits_projection_info()
            .ok_or_else(|| CpuWasmError::InvalidModel("missing output projection".into()))?;
        let vocab = gemv(
            &self.model,
            self.index.tensor_data_start,
            u32::MAX,
            "output.weight",
            &output,
            &self.hidden,
            &mut self.logits,
        )?;
        let mut best_token_id = 0u32;
        let mut max_logit = f32::NEG_INFINITY;
        for (token, &score) in self.logits[..vocab].iter().enumerate() {
            if score.is_finite()
                && (score > max_logit || (score == max_logit && token < best_token_id as usize))
            {
                max_logit = score;
                best_token_id = token as u32;
            }
        }
        if max_logit == f32::NEG_INFINITY {
            return Err(CpuWasmError::KernelFailed {
                layer: u32::MAX,
                role: "output argmax",
            });
        }
        Ok(CpuWasmStep {
            token_id: best_token_id,
            max_logit,
        })
    }
}
