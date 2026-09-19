//! Safetensor model loader — native zero-copy adoption of Safetensors models (including BF16).
//!
//! Maps Hugging Face safetensors format (`model.layers.N...`, `lm_head`, `model.embed_tokens`)
//! and GGUF-named safetensors into canonical `GgufTensorIndex` references without transcoding.
//! Supports `BF16` (GGML 30), `F16` (GGML 1), and `F32` (GGML 0).

use crate::inference::gguf_sharder::{
    GgufHyperparams, GgufTensorIndex, GgufTensorInfo, ARCH_LLAMA, ARCH_QWEN2, DEFAULT_ROPE_FREQ_BASE,
};
use crate::inference::safetensor::{
    parse_safetensor_header, safetensor_dtype_to_ggml,
};
use crate::inference::tensor_roles::name_to_role;
use crate::p64_weight::{
    P64_LAYER_GLOBAL, P64_ROLE_ATTN_K, P64_ROLE_ATTN_NORM, P64_ROLE_ATTN_OUTPUT, P64_ROLE_ATTN_Q,
    P64_ROLE_ATTN_SUBLN, P64_ROLE_ATTN_V, P64_ROLE_FFN_DOWN, P64_ROLE_FFN_GATE, P64_ROLE_FFN_NORM,
    P64_ROLE_FFN_SUBLN, P64_ROLE_FFN_UP, P64_ROLE_OUTPUT, P64_ROLE_OUTPUT_NORM,
    P64_ROLE_TOKEN_EMBD,
};

/// Suffix byte slice corresponding to a P64 role id.
pub(crate) fn role_suffix(role_id: u16) -> Option<&'static [u8]> {
    match role_id {
        P64_ROLE_ATTN_NORM => Some(b"attn_norm.weight"),
        P64_ROLE_ATTN_Q => Some(b"attn_q.weight"),
        P64_ROLE_ATTN_K => Some(b"attn_k.weight"),
        P64_ROLE_ATTN_V => Some(b"attn_v.weight"),
        P64_ROLE_ATTN_OUTPUT => Some(b"attn_output.weight"),
        P64_ROLE_ATTN_SUBLN => Some(b"attn_sub_norm.weight"),
        P64_ROLE_FFN_NORM => Some(b"ffn_norm.weight"),
        P64_ROLE_FFN_GATE => Some(b"ffn_gate.weight"),
        P64_ROLE_FFN_DOWN => Some(b"ffn_down.weight"),
        P64_ROLE_FFN_UP => Some(b"ffn_up.weight"),
        P64_ROLE_FFN_SUBLN => Some(b"ffn_sub_norm.weight"),
        crate::p64_weight::P64_ROLE_MOE_ROUTER => Some(b"ffn_gate_inp.weight"),
        crate::p64_weight::P64_ROLE_MOE_SHARED_GATE => Some(b"ffn_gate_shexp.weight"),
        crate::p64_weight::P64_ROLE_MOE_SHARED_UP => Some(b"ffn_up_shexp.weight"),
        crate::p64_weight::P64_ROLE_MOE_SHARED_DOWN => Some(b"ffn_down_shexp.weight"),
        crate::p64_weight::P64_ROLE_MOE_GATE_EXPS => Some(b"ffn_gate_exps.weight"),
        crate::p64_weight::P64_ROLE_MOE_UP_EXPS => Some(b"ffn_up_exps.weight"),
        crate::p64_weight::P64_ROLE_MOE_DOWN_EXPS => Some(b"ffn_down_exps.weight"),
        _ => None,
    }
}

/// Optional JSON config fields from standard HuggingFace `config.json`.
#[derive(Debug, Clone, Default)]
pub struct ModelJsonConfig {
    pub num_hidden_layers: Option<u32>,
    pub hidden_size: Option<u32>,
    pub num_attention_heads: Option<u32>,
    pub num_key_value_heads: Option<u32>,
    pub head_dim: Option<u32>,
    pub rope_theta: Option<f32>,
    pub model_type: Option<String>,
    pub moe_intermediate_size: Option<u32>,
    pub num_experts: Option<u32>,
    pub num_experts_per_tok: Option<u32>,
}

impl ModelJsonConfig {
    /// Parse from a JSON string slice, supporting top-level or nested `text_config`.
    pub fn from_json_str(s: &str) -> Option<Self> {
        let val: serde_json::Value = serde_json::from_str(s).ok()?;
        let obj = val.as_object()?;
        let tc = obj.get("text_config").and_then(|v| v.as_object());
        let get_u32 = |k: &str| {
            obj.get(k)
                .or_else(|| tc.and_then(|t| t.get(k)))
                .and_then(|v| v.as_u64())
                .map(|v| v as u32)
        };
        let get_f32 = |k: &str| {
            obj.get(k)
                .or_else(|| tc.and_then(|t| t.get(k)))
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
        };
        let get_str = |k: &str| {
            obj.get(k)
                .or_else(|| tc.and_then(|t| t.get(k)))
                .and_then(|v| v.as_str())
                .map(|v| v.to_string())
        };

        Some(Self {
            num_hidden_layers: get_u32("num_hidden_layers"),
            hidden_size: get_u32("hidden_size"),
            num_attention_heads: get_u32("num_attention_heads"),
            num_key_value_heads: get_u32("num_key_value_heads"),
            head_dim: get_u32("head_dim"),
            rope_theta: get_f32("rope_theta"),
            model_type: get_str("model_type"),
            moe_intermediate_size: get_u32("moe_intermediate_size"),
            num_experts: get_u32("num_experts"),
            num_experts_per_tok: get_u32("num_experts_per_tok"),
        })
    }
}

/// Parse a safetensor mmap and build a canonical `GgufTensorIndex`.
pub fn parse_safetensor_to_index(
    mmap: &[u8],
    config_json: Option<&str>,
) -> Result<GgufTensorIndex, String> {
    let plan = parse_safetensor_header(mmap)?;
    let parsed_cfg = config_json.and_then(ModelJsonConfig::from_json_str);

    let mut named_tensors: Vec<(Vec<u8>, GgufTensorInfo)> = Vec::with_capacity(plan.tensors.len());
    let mut inferred_layers = 0u32;
    let mut inferred_embd = 0u32;
    let mut inferred_q_heads = 0u32;
    let mut inferred_kv_heads = 0u32;

    for tensor in &plan.tensors {
        let ggml_type = safetensor_dtype_to_ggml(&tensor.dtype).ok_or_else(|| {
            format!(
                "safetensor: unsupported dtype '{}' on tensor '{}'",
                tensor.dtype, tensor.name
            )
        })?;

        // Canonicalize name
        let role = name_to_role(&tensor.name);
        let canonical_name: Vec<u8> = if let Some(r) = role {
            if r.layer == P64_LAYER_GLOBAL {
                match r.role {
                    P64_ROLE_TOKEN_EMBD => {
                        if tensor.shape.len() >= 2 {
                            inferred_embd = inferred_embd.max(tensor.shape[1] as u32);
                        }
                        b"token_embd.weight".to_vec()
                    }
                    P64_ROLE_OUTPUT => b"output.weight".to_vec(),
                    P64_ROLE_OUTPUT_NORM => b"output_norm.weight".to_vec(),
                    _ => tensor.name.as_bytes().to_vec(),
                }
            } else {
                inferred_layers = inferred_layers.max(r.layer as u32 + 1);
                if let Some(suffix) = role_suffix(r.role) {
                    let mut buf = [0u8; 96];
                    let len = crate::gguf_sharder::write_blk_tensor_name(
                        r.layer as u32,
                        suffix,
                        &mut buf,
                    );
                    if r.role == P64_ROLE_ATTN_Q && tensor.shape.len() >= 2 {
                        let out_dim = tensor.shape[0] as u32;
                        let in_dim = tensor.shape[1] as u32;
                        inferred_embd = inferred_embd.max(in_dim);
                        let h_dim = 64u32; // standard floor
                        inferred_q_heads = inferred_q_heads.max(out_dim / h_dim);
                    } else if r.role == P64_ROLE_ATTN_K && tensor.shape.len() >= 2 {
                        let out_dim = tensor.shape[0] as u32;
                        let h_dim = 64u32;
                        inferred_kv_heads = inferred_kv_heads.max(out_dim / h_dim);
                    }
                    buf[..len].to_vec()
                } else {
                    tensor.name.as_bytes().to_vec()
                }
            }
        } else {
            tensor.name.as_bytes().to_vec()
        };

        let mut dims = [0u64; 4];
        let n_dims = tensor.shape.len().min(4) as u32;
        for (i, d) in tensor.shape.iter().take(4).enumerate() {
            // Reverse dimensions to match GGUF column-major order convention if necessary
            // For 2D matmuls [out_features, in_features], GGUF expects [in_features, out_features].
            if n_dims == 2 {
                dims[0] = tensor.shape[1] as u64;
                dims[1] = tensor.shape[0] as u64;
                break;
            } else {
                dims[i] = *d as u64;
            }
        }

        named_tensors.push((
            canonical_name,
            GgufTensorInfo {
                dims,
                n_dims,
                ggml_type,
                byte_offset: tensor.begin as u64,
            },
        ));
    }

    // Resolve hyperparameters from config.json or inferred dimensions
    let n_layer = parsed_cfg
        .as_ref()
        .and_then(|c| c.num_hidden_layers)
        .unwrap_or(inferred_layers);
    let n_embd = parsed_cfg
        .as_ref()
        .and_then(|c| c.hidden_size)
        .unwrap_or(inferred_embd);
    let n_head = parsed_cfg
        .as_ref()
        .and_then(|c| c.num_attention_heads)
        .unwrap_or_else(|| inferred_q_heads.max(1));
    let n_kv_head = parsed_cfg
        .as_ref()
        .and_then(|c| c.num_key_value_heads)
        .unwrap_or_else(|| inferred_kv_heads.max(1));
    let head_dim = parsed_cfg
        .as_ref()
        .and_then(|c| c.head_dim)
        .unwrap_or_else(|| if n_head > 0 { n_embd / n_head } else { 64 });
    let rope_freq_base = parsed_cfg
        .as_ref()
        .and_then(|c| c.rope_theta)
        .unwrap_or(DEFAULT_ROPE_FREQ_BASE);

    let architecture = if let Some(cfg) = &parsed_cfg {
        match cfg.model_type.as_deref() {
            Some("qwen2") | Some("qwen") => ARCH_QWEN2,
            _ => ARCH_LLAMA,
        }
    } else {
        ARCH_LLAMA
    };

    let hyperparams = GgufHyperparams {
        n_layer,
        n_embd,
        n_head,
        n_kv_head,
        rope_freq_base,
        rope_scale: 1.0,
        head_dim,
        head_dim_swa: head_dim,
        sliding_window: 0,
        shared_kv_layers: 0,
        logit_softcap: 0.0,
        architecture,
        arch_flags: 0,
    };

    let references: Vec<(&[u8], GgufTensorInfo)> = named_tensors
        .iter()
        .map(|(name, info)| (name.as_slice(), *info))
        .collect();

    Ok(GgufTensorIndex::from_components(
        &references,
        hyperparams,
        plan.data_start as u64,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ggml_quants::GGML_TYPE_BF16;

    #[test]
    fn test_parse_synthetic_bf16_safetensor() {
        // Construct a synthetic Safetensors byte buffer with BF16 tensor
        let json_header = r#"{
            "model.embed_tokens.weight": {
                "dtype": "BF16",
                "shape": [10, 64],
                "data_offsets": [0, 1280]
            },
            "model.layers.0.self_attn.q_proj.weight": {
                "dtype": "BF16",
                "shape": [64, 64],
                "data_offsets": [1280, 9472]
            },
            "lm_head.weight": {
                "dtype": "BF16",
                "shape": [10, 64],
                "data_offsets": [9472, 10752]
            }
        }"#;

        let json_bytes = json_header.as_bytes();
        let hlen = json_bytes.len() as u64;

        let mut buf = Vec::new();
        buf.extend_from_slice(&hlen.to_le_bytes());
        buf.extend_from_slice(json_bytes);
        buf.resize(buf.len() + 10752, 0x3fu8); // dummy payload

        let index = parse_safetensor_to_index(&buf, None).expect("safetensor parse ok");
        assert_eq!(index.hyperparams.n_layer, 1);
        assert_eq!(index.hyperparams.n_embd, 64);
        assert!(index.token_embd.is_some());
        assert_eq!(index.token_embd.unwrap().ggml_type, GGML_TYPE_BF16);
    }
}
