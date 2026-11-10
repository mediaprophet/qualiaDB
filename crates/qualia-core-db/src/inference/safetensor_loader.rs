//! Safetensor model loader — native zero-copy adoption of Safetensors models (including BF16).
//!
//! Maps Hugging Face safetensors format (`model.layers.N...`, `lm_head`, `model.embed_tokens`)
//! and GGUF-named safetensors into canonical `GgufTensorIndex` references without transcoding.
//! Supports `BF16` (GGML 30), `F16` (GGML 1), and `F32` (GGML 0).

use crate::inference::gguf_sharder::{
    GgufHyperparams, GgufTensorIndex, GgufTensorInfo, ARCH_LLAMA, ARCH_QWEN2,
    DEFAULT_ROPE_FREQ_BASE,
};
use crate::inference::safetensor::{parse_safetensor_header, safetensor_dtype_to_ggml};
use crate::inference::tensor_roles::name_to_role;
use crate::p64_weight::{
    P64_LAYER_GLOBAL, P64_ROLE_ATTN_K, P64_ROLE_ATTN_Q, P64_ROLE_OUTPUT, P64_ROLE_OUTPUT_NORM,
    P64_ROLE_TOKEN_EMBD,
};

pub(crate) use crate::inference::tensor_roles::canonical_suffix as role_suffix;

/// GGML synthetic type tag for packed NVFP4 (`U8` weight bytes paired with `F8_E4M3` `.weight_scale` and `F32` `.weight_scale_2`).
pub const GGML_TYPE_NVFP4_PACKED: u32 = 114;

/// Zero-copy view of one NVFP4-quantized matrix inside a Safetensors mmap (`U8` packed nibbles + `F8_E4M3` block scales + `f32` global scale).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafetensorNvfp4Matrix<'a> {
    pub packed: &'a [u8],
    pub block_scales: &'a [u8],
    pub global_scale: f32,
    pub rows: usize,
    pub cols: usize,
}

impl<'a> SafetensorNvfp4Matrix<'a> {
    /// Execute zero-heap NVFP4 matrix-vector multiplication $y = A \cdot x$ directly from the Safetensors mmap.
    pub fn gemv_zero_heap(&self, input: &[f32], out: &mut [f32]) -> Result<(), crate::inference::moe::Nvfp4Error> {
        crate::inference::moe::nvfp4_gemv_zero_heap(
            input,
            self.packed,
            self.block_scales,
            self.global_scale,
            self.rows,
            self.cols,
            out,
        )
    }
}

/// Resolve an NVFP4 matrix triplet (`{prefix}.weight`, `{prefix}.weight_scale`, `{prefix}.weight_scale_2`) from a Safetensors mmap.
pub fn resolve_nvfp4_matrix<'a>(
    mmap: &'a [u8],
    plan: &crate::inference::safetensor::SafeTensorPlan,
    weight_name: &str,
) -> Option<SafetensorNvfp4Matrix<'a>> {
    let base = weight_name.strip_suffix(".weight")?;
    let scale_name = format!("{base}.weight_scale");
    let scale2_name = format!("{base}.weight_scale_2");
    let w = plan.tensors.iter().find(|t| t.name == weight_name && t.dtype == "U8")?;
    let s = plan.tensors.iter().find(|t| t.name == scale_name && t.dtype == "F8_E4M3")?;
    let s2 = plan.tensors.iter().find(|t| t.name == scale2_name && t.dtype == "F32")?;
    if w.shape.len() != 2 || s2.byte_len() < 4 {
        return None;
    }
    let rows = w.shape[0];
    let cols = w.shape[1] * 2;
    let ds = plan.data_start;
    let packed = mmap.get(ds + w.begin..ds + w.end)?;
    let block_scales = mmap.get(ds + s.begin..ds + s.end)?;
    let s2_bytes = mmap.get(ds + s2.begin..ds + s2.begin + 4)?;
    let global_scale = f32::from_le_bytes(s2_bytes.try_into().ok()?);
    Some(SafetensorNvfp4Matrix {
        packed,
        block_scales,
        global_scale,
        rows,
        cols,
    })
}
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
        if tensor.name.ends_with(".input_scale")
            || tensor.name.ends_with(".weight_scale")
            || tensor.name.ends_with(".weight_scale_2")
            || tensor.name.starts_with("model.vision_tower.")
            || tensor.name.starts_with("model.embed_vision.")
        {
            continue;
        }
        let ggml_type = if tensor.dtype == "U8" && tensor.name.ends_with(".weight") {
            GGML_TYPE_NVFP4_PACKED
        } else if let Some(t) = safetensor_dtype_to_ggml(&tensor.dtype) {
            t
        } else {
            return Err(format!(
                "safetensor: unsupported dtype '{}' on tensor '{}'",
                tensor.dtype, tensor.name
            ));
        };

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
                let in_elems = if ggml_type == GGML_TYPE_NVFP4_PACKED {
                    (tensor.shape[1] * 2) as u64
                } else {
                    tensor.shape[1] as u64
                };
                dims[0] = in_elems;
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
        ssm_conv_kernel: 0,
        ssm_state_size: 0,
        ssm_group_count: 0,
        ssm_time_step_rank: 0,
        ssm_inner_size: 0,
        full_attention_interval: 0,
        architecture,
        arch_flags: 0,
        ..GgufHyperparams::default()
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

    #[test]
    fn real_qwen35_4b_nvfp4_and_gemma4_26b_nvfp4_safetensor_gemv() {
        let qwen_path = r"C:\LLM_files\qwen\Qwen3.5-4B-NVFP4\model.safetensors";
        if std::path::Path::new(qwen_path).exists() {
            let cfg_str = std::fs::read_to_string(r"C:\LLM_files\qwen\Qwen3.5-4B-NVFP4\config.json")
                .ok();
            let file = std::fs::File::open(qwen_path).unwrap();
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file).unwrap() };
            let plan = parse_safetensor_header(&mmap[..]).expect("qwen nvfp4 header");
            let index = parse_safetensor_to_index(&mmap[..], cfg_str.as_deref())
                .expect("qwen nvfp4 index");
            assert_eq!(index.hyperparams.n_layer, 32);
            assert_eq!(index.hyperparams.n_embd, 2560);
            let mat = resolve_nvfp4_matrix(
                &mmap[..],
                &plan,
                "model.language_model.layers.0.mlp.gate_proj.weight",
            )
            .expect("resolve qwen gate_proj nvfp4");
            assert_eq!(mat.rows, 9216);
            assert_eq!(mat.cols, 2560);
            let input = [0.125f32; 2560];
            let mut out = vec![0.0f32; 9216];
            mat.gemv_zero_heap(&input, &mut out).expect("nvfp4 gemv");
            assert!(out.iter().all(|v| v.is_finite()));
            assert!(out.iter().any(|&v| v.abs() > 1e-6));
        }

        let gemma_path =
            r"C:\LLM_files\gemma\Gemma-4-26B-A4B-NVFP4\model-00001-of-00002.safetensors";
        if std::path::Path::new(gemma_path).exists() {
            let file = std::fs::File::open(gemma_path).unwrap();
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file).unwrap() };
            let plan = parse_safetensor_header(&mmap[..]).expect("gemma nvfp4 header");
            let mat = resolve_nvfp4_matrix(
                &mmap[..],
                &plan,
                "model.language_model.layers.0.experts.0.gate_proj.weight",
            )
            .expect("resolve gemma expert 0 gate_proj nvfp4");
            assert_eq!(mat.rows, 704);
            assert_eq!(mat.cols, 2816);
            let input = [0.125f32; 2816];
            let mut out = [0.0f32; 704];
            mat.gemv_zero_heap(&input, &mut out).expect("gemma nvfp4 gemv");
            assert!(out.iter().all(|v| v.is_finite()));
            assert!(out.iter().any(|&v| v.abs() > 1e-6));
        }
    }
}
