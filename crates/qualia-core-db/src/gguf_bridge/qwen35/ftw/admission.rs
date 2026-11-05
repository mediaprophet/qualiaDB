//! Cold admission with an explicit exporter contract and a reviewed artifact receipt.
use super::{
    attention,
    weights::{self, Nvfp4Projection},
    LayerPlan, Plan, Profile,
};
use crate::inference::moe::ftw_loader::FtwModelPackage;
use etalon_model::inference::{
    qwen_attention::{QkNorm, Qwen35AttentionGeometry},
    qwen_roles::{Qwen35Geometry, Qwen35Packing},
    TensorInfo,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::{path::Path, sync::Arc};

/// Reviewed FreeToken mixed-FP8 exporter: global Q,K,V,Z / B,A blocks;
/// per-head Q,gate; ordinary RMSNorm +1 folded, gated RMSNorm unchanged.
/// Source: FreeToken qwen3_5_moe/{weight,gdn,attention}.py. See receipt below.
#[derive(Clone, Copy, Debug)]
pub enum ExportContract {
    FreeTokenQwen35MixedFp8V1,
}

// This receipt selects semantics, not a model path. Unknown exports require an
// explicit reviewed contract; matching architecture names never grant admission.
// Reviewed converter SHA256:
// weight.py 90f6bafdcf91c52cfbd19de517ea71f7c2642f30672a56e595dd6f49c499cd0a
// gdn.py 3fe0d06a3ba3aeb65b53bcfbadfddf20ddcd15664820214a91f3c6d10cabc630
// attention.py d2de35ac0acb47e126d9e793611502bcf12ba920fda95b0918879f1b500804db
const RECEIPT: [(&str, &str); 3] = [
    (
        "freetoken_weight.json",
        "79bb256dc54229377986d1863f91cb123b43d7d015cc7418ddbf580aa8c9249d",
    ),
    (
        "config.json",
        "58aefa1c9eff7989f431d748f2ddec39446cb1fd2a69acc46e285c6a37b0cecc",
    ),
    (
        "hf_quant_config.json",
        "75fe7cc8d5836b58734e05ee67423a4ce91d602aaad45c8173a1b7597cd57663",
    ),
];
const CONFIG_LIMIT: usize = 2 * 1024 * 1024;

fn bounded_read(path: &Path, budget: usize) -> Result<Vec<u8>, String> {
    let len = usize::try_from(std::fs::metadata(path).map_err(|e| e.to_string())?.len())
        .map_err(|_| "Qwen metadata size overflow")?;
    if len > CONFIG_LIMIT || len > budget {
        return Err("Qwen metadata budget exhausted".into());
    }
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = vec![0; len];
    file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
    if file.read(&mut [0]).map_err(|e| e.to_string())? != 0 {
        return Err("Qwen metadata changed while reading".into());
    }
    Ok(bytes)
}

pub fn admit_known(
    pkg: Arc<FtwModelPackage>,
    dir: &Path,
    max_context: usize,
    byte_budget: usize,
) -> Result<Arc<Plan>, String> {
    let mut verified_config = None;
    for (name, expected) in RECEIPT {
        let bytes = bounded_read(&dir.join(name), byte_budget)?;
        let digest = Sha256::digest(&bytes);
        let mut encoded = [0u8; 64];
        for (index, &byte) in digest.iter().enumerate() {
            encoded[index * 2] = b"0123456789abcdef"[(byte >> 4) as usize];
            encoded[index * 2 + 1] = b"0123456789abcdef"[(byte & 15) as usize];
        }
        if encoded != expected.as_bytes() {
            return Err(format!("Qwen exporter receipt mismatch: {name}"));
        }
        if name == "freetoken_weight.json" {
            let manifest: crate::inference::moe::ftw_loader::FtwManifest =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if manifest.format != pkg.manifest.format
                || manifest.version != pkg.manifest.version
                || manifest.quant_format != pkg.manifest.quant_format
                || manifest.expert_bank_num_layers != pkg.manifest.expert_bank_num_layers
                || manifest.total_bytes != pkg.manifest.total_bytes
                || manifest.tensors.len() != pkg.manifest.tensors.len()
                || manifest.shards.len() != pkg.manifest.shards.len()
                || manifest
                    .tensors
                    .iter()
                    .zip(&pkg.manifest.tensors)
                    .any(|(a, b)| {
                        a.name != b.name
                            || a.kind != b.kind
                            || a.dtype != b.dtype
                            || a.shape != b.shape
                            || a.nbytes != b.nbytes
                            || a.global_off != b.global_off
                    })
                || manifest
                    .shards
                    .iter()
                    .zip(&pkg.manifest.shards)
                    .any(|(a, b)| {
                        a.file != b.file || a.global_off != b.global_off || a.nbytes != b.nbytes
                    })
            {
                return Err("Qwen receipt does not describe retained package".into());
            }
        }
        if name == "config.json" {
            verified_config = Some(bytes);
        }
    }
    admit_config(
        pkg,
        &verified_config.ok_or("Qwen receipt missing config")?,
        max_context,
        byte_budget,
        ExportContract::FreeTokenQwen35MixedFp8V1,
    )
}

fn number(v: &Value, key: &str) -> Result<usize, String> {
    usize::try_from(
        v[key]
            .as_u64()
            .ok_or_else(|| format!("Qwen missing integer {key}"))?,
    )
    .map_err(|_| format!("Qwen integer overflow {key}"))
}
fn tensor(pkg: &FtwModelPackage, name: &str, shape: &[u64]) -> Result<TensorInfo, String> {
    let entry = weights::entry(pkg, name).map_err(|e| format!("Qwen {name}: {e:?}"))?;
    if entry.shape != shape || !matches!(entry.dtype.as_str(), "bfloat16" | "float16" | "float32") {
        return Err(format!("Qwen scalar tensor shape/encoding {name}"));
    }
    let count = shape
        .iter()
        .try_fold(1u64, |n, &d| n.checked_mul(d))
        .ok_or("Qwen tensor size overflow")?;
    let bytes = count
        .checked_mul(if entry.dtype == "float32" { 4 } else { 2 })
        .ok_or("Qwen tensor byte overflow")?;
    if bytes != entry.nbytes
        || pkg
            .fetch_by_global_offset(
                entry.global_off,
                usize::try_from(bytes).map_err(|_| "Qwen tensor byte overflow")?,
            )
            .is_none()
    {
        return Err(format!("Qwen tensor bounds {name}"));
    }
    let mut dims = [0; 4];
    for (slot, &size) in dims.iter_mut().zip(shape.iter().rev()) {
        *slot = size;
    }
    Ok(TensorInfo {
        dims,
        n_dims: shape.len() as u32,
        byte_offset: entry.global_off,
        ggml_type: match entry.dtype.as_str() {
            "bfloat16" => crate::ggml_quants::GGML_TYPE_BF16,
            "float16" => crate::ggml_quants::GGML_TYPE_F16,
            _ => crate::ggml_quants::GGML_TYPE_F32,
        },
    })
}

pub fn admit(
    pkg: Arc<FtwModelPackage>,
    dir: &Path,
    max_context: usize,
    byte_budget: usize,
    _contract: ExportContract,
) -> Result<Arc<Plan>, String> {
    let bytes = bounded_read(&dir.join("config.json"), byte_budget)?;
    admit_config(pkg, &bytes, max_context, byte_budget, _contract)
}

fn admit_config(
    pkg: Arc<FtwModelPackage>,
    bytes: &[u8],
    max_context: usize,
    byte_budget: usize,
    _contract: ExportContract,
) -> Result<Arc<Plan>, String> {
    let config: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if config["model_type"] != "qwen3_5_moe"
        || config["architectures"] != serde_json::json!(["Qwen3_5MoeForConditionalGeneration"])
    {
        return Err("Qwen architecture contract mismatch".into());
    }
    let c = &config["text_config"];
    // The reviewed FreeToken MoE implementation unconditionally renormalizes
    // selected softmax weights. An explicitly conflicting config is refused.
    if c["model_type"] != "qwen3_5_moe_text"
        || c["attention_bias"] != false
        || c["attn_output_gate"] != true
        || c["hidden_act"] != "silu"
        || (!c["norm_topk_prob"].is_null() && c["norm_topk_prob"] != true)
    {
        return Err("Qwen text execution contract mismatch".into());
    }
    let h = pkg.tensor_index.hyperparams;
    let hidden = number(c, "hidden_size")?;
    let layers = number(c, "num_hidden_layers")?;
    let vocab = number(c, "vocab_size")?;
    let experts = number(c, "num_experts")?;
    let topk = number(c, "num_experts_per_tok")?;
    let intermediate = number(c, "moe_intermediate_size")?;
    if hidden != h.n_embd as usize
        || layers != h.n_layer as usize
        || layers > 1024
        || experts != 256
        || topk == 0
        || topk > 8
        || hidden > 16384
        || intermediate == 0
        || intermediate > 8192
        || intermediate % 16 != 0
        || intermediate != number(c, "shared_expert_intermediate_size")?
        || max_context == 0
        || max_context > number(c, "max_position_embeddings")?
    {
        return Err("Qwen geometry mismatch".into());
    }
    let epsilon = c["rms_norm_eps"].as_f64().ok_or("Qwen missing epsilon")? as f32;
    if !epsilon.is_finite() || epsilon <= 0.0 {
        return Err("Qwen invalid epsilon".into());
    }
    let rope = &c["rope_parameters"];
    if rope["rope_type"] != "default" || rope["mrope_interleaved"] != true {
        return Err("Qwen unsupported rotary contract".into());
    }
    let head_dim = number(c, "head_dim")?;
    if head_dim != h.effective_head_dim() as usize || head_dim > 1024 {
        return Err("Qwen head dimension config/index mismatch".into());
    }
    let partial = rope["partial_rotary_factor"]
        .as_f64()
        .ok_or("Qwen missing rotary fraction")?;
    let theta = rope["rope_theta"]
        .as_f64()
        .ok_or("Qwen missing rotary base")?;
    let rotary = (head_dim as f64) * partial;
    if !theta.is_finite()
        || theta <= 0.0
        || !rotary.is_finite()
        || rotary.fract() != 0.0
        || rotary <= 0.0
        || rotary > head_dim as f64
        || rotary as usize % 2 != 0
    {
        return Err("Qwen invalid rotary geometry".into());
    }
    let attention_geometry = Qwen35AttentionGeometry {
        hidden,
        query_heads: number(c, "num_attention_heads")?,
        kv_heads: number(c, "num_key_value_heads")?,
        head_dim,
        rotary_dim: rotary as usize,
    };
    attention_geometry
        .validate()
        .map_err(|e| format!("Qwen attention geometry {e:?}"))?;
    let linear_geometry = Qwen35Geometry::from_hyperparams(h.into())
        .map_err(|e| format!("Qwen linear geometry {e:?}"))?;
    if linear_geometry.key_heads != number(c, "linear_num_key_heads")?
        || linear_geometry.value_heads != number(c, "linear_num_value_heads")?
        || linear_geometry.key_dim != number(c, "linear_key_head_dim")?
        || linear_geometry.value_dim != number(c, "linear_value_head_dim")?
        || linear_geometry.kernel != number(c, "linear_conv_kernel_dim")?
    {
        return Err("Qwen recurrent config/index mismatch".into());
    }
    let metadata = layers
        .checked_mul(std::mem::size_of::<LayerPlan>())
        .and_then(|n| n.checked_add(std::mem::size_of::<Plan>()))
        .and_then(|n| n.checked_add((rotary as usize / 2) * 4))
        .and_then(|n| n.checked_add(pkg.prepared_metadata_bytes()?))
        .ok_or("Qwen plan budget overflow")?;
    if metadata > byte_budget {
        return Err("Qwen plan budget exhausted".into());
    }
    let profile = Profile {
        hidden,
        layers,
        vocab,
        intermediate,
        experts,
        topk,
        max_context,
        epsilon,
        linear_geometry,
        attention_geometry,
        rotary_inv_freq: (0..rotary as usize / 2)
            .map(|i| (1.0 / theta.powf((2 * i) as f64 / rotary)) as f32)
            .collect(),
        packing: Qwen35Packing::FusedGlobal,
        ordinary_norm: QkNorm::FoldedMultiplier,
    };
    let kinds = c["layer_types"]
        .as_array()
        .ok_or("Qwen missing explicit layer schedule")?;
    if kinds.len() != layers {
        return Err("Qwen layer schedule mismatch".into());
    }
    let mut plans = Vec::with_capacity(layers);
    for (layer, kind) in kinds.iter().enumerate() {
        let is_linear = match kind.as_str() {
            Some("linear_attention") => true,
            Some("full_attention") => false,
            _ => return Err("Qwen unsupported layer kind".into()),
        };
        let base = format!("model.layers.{layer}");
        let attention = attention::resolve(&pkg, &profile, layer, is_linear)?;
        let plan = LayerPlan {
            attention,
            input_norm: tensor(
                &pkg,
                &format!("{base}.input_layernorm.weight"),
                &[hidden as u64],
            )?,
            post_norm: tensor(
                &pkg,
                &format!("{base}.post_attention_layernorm.weight"),
                &[hidden as u64],
            )?,
            router: tensor(
                &pkg,
                &format!("{base}.mlp.gate.weight"),
                &[experts as u64, hidden as u64],
            )?,
            shared_gate: tensor(
                &pkg,
                &format!("{base}.mlp.shared_expert_gate.weight"),
                &[1, hidden as u64],
            )?,
            shared_up: Nvfp4Projection::admit(
                &pkg,
                &format!("{base}.mlp.shared_expert.gate_up_proj"),
                2 * intermediate,
                hidden,
            )
            .map_err(|e| format!("Qwen shared gate/up {e:?}"))?,
            shared_down: Nvfp4Projection::admit(
                &pkg,
                &format!("{base}.mlp.shared_expert.down_proj"),
                hidden,
                intermediate,
            )
            .map_err(|e| format!("Qwen shared down {e:?}"))?,
        };
        for (name, dtype, shape, expected) in [
            (
                "gate_up_packed",
                "uint8",
                vec![
                    experts as u64,
                    (2 * intermediate) as u64,
                    (hidden / 2) as u64,
                ],
                experts * intermediate * hidden,
            ),
            (
                "gate_up_scale",
                "float8_e4m3fn",
                vec![
                    experts as u64,
                    (2 * intermediate) as u64,
                    (hidden / 16) as u64,
                ],
                experts * 2 * intermediate * hidden / 16,
            ),
            (
                "gate_up_global",
                "float16",
                vec![experts as u64, (2 * intermediate) as u64],
                experts * 2 * intermediate * 2,
            ),
            (
                "down_packed",
                "uint8",
                vec![experts as u64, hidden as u64, (intermediate / 2) as u64],
                experts * hidden * intermediate / 2,
            ),
            (
                "down_scale",
                "float8_e4m3fn",
                vec![experts as u64, hidden as u64, (intermediate / 16) as u64],
                experts * hidden * intermediate / 16,
            ),
            (
                "down_global",
                "float16",
                vec![experts as u64, hidden as u64],
                experts * hidden * 2,
            ),
        ] {
            let name = format!("{name}#L{layer:05}");
            let bank =
                weights::entry(&pkg, &name).map_err(|e| format!("Qwen bank {name}: {e:?}"))?;
            if bank.dtype != dtype || bank.shape != shape || bank.nbytes != expected as u64 {
                return Err(format!("Qwen bank encoding/shape {name}"));
            }
        }
        for expert in 0..experts {
            let data = pkg
                .get_expert_data(layer as u16, expert as u16)
                .ok_or("Qwen missing expert bank")?;
            if data.gate_up_packed.len() != 2 * intermediate * hidden / 2
                || data.gate_up_scale.len() != 2 * intermediate * hidden / 16
                || data.gate_up_global.len() != 2 * intermediate * 2
                || data.down_packed.len() != hidden * intermediate / 2
                || data.down_scale.len() != hidden * intermediate / 16
                || data.down_global.len() != hidden * 2
            {
                return Err("Qwen expert bank shape mismatch".into());
            }
        }
        plans.push(plan);
    }
    let output = Nvfp4Projection::admit(&pkg, "lm_head", vocab, hidden)
        .map_err(|e| format!("Qwen output {e:?}"))?;
    let final_norm = tensor(&pkg, "model.norm.weight", &[hidden as u64])?;
    let embedding = tensor(
        &pkg,
        "model.embed_tokens.weight",
        &[vocab as u64, hidden as u64],
    )?;
    Ok(Arc::new(Plan {
        pkg,
        profile,
        layers: plans,
        output,
        final_norm,
        embedding,
        reserved_bytes: metadata,
    }))
}
