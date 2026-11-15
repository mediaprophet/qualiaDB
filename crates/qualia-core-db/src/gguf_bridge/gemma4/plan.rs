use super::metadata::Profile;
use crate::gguf_sharder::{GgufTensorIndex, LayerTensors};
use etalon_model::inference::TensorInfo;
use std::collections::BTreeSet;

#[derive(Clone)]
pub(super) struct Layer {
    pub tensors: LayerTensors,
    pub source: usize,
    pub offset: usize,
    pub slots: usize,
    pub dim: usize,
    pub kv_heads: usize,
    pub sliding: bool,
}
pub(super) struct Plan {
    pub profile: Profile,
    pub index: GgufTensorIndex,
    pub layers: Vec<Layer>,
    pub context: usize,
    pub kv_floats: usize,
    pub ple_projection: Option<TensorInfo>,
    pub ple_norm: Option<TensorInfo>,
    pub rope: TensorInfo,
}
impl Plan {
    pub fn new(
        mut index: GgufTensorIndex,
        p: Profile,
        bytes: &[u8],
        context: usize,
    ) -> Result<Self, String> {
        if context == 0
            || context > p.trained_context
            || context > u32::MAX as usize
            || index.entries.len() > 4096
            || index.hyperparams.architecture != crate::gguf_sharder::ARCH_GEMMA4
            || index.ple_config.is_some()
        {
            return Err("Gemma context/directory bound".into());
        }
        let mut seen = BTreeSet::new();
        let mut take = |name: &str,
                        dims: &[usize],
                        scalar: bool,
                        optional: bool|
         -> Result<Option<TensorInfo>, String> {
            let hash = etalon_model::gguf::fnv1a64(name.as_bytes());
            let mut rows = index.entries.iter().filter(|(h, _)| *h == hash);
            let Some((_, info)) = rows.next() else {
                return if optional {
                    Ok(None)
                } else {
                    Err(format!("missing Gemma {name}"))
                };
            };
            if rows.next().is_some()
                || !seen.insert(hash)
                || info.n_dims as usize != dims.len()
                || dims
                    .iter()
                    .enumerate()
                    .any(|(i, n)| info.dims[i] != *n as u64)
                || (scalar && !matches!(info.ggml_type, 0 | 30))
                || (!scalar && !matches!(info.ggml_type, 0 | 1 | 7 | 8 | 12 | 13 | 14 | 30 | 39))
            {
                return Err(format!("unsupported Gemma tensor {name}"));
            }
            let n = etalon_model::ggml::tensor_byte_len(info)
                .ok_or("canonical Gemma tensor codec unavailable")?;
            let start = index
                .tensor_data_start
                .checked_add(info.byte_offset)
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("Gemma tensor offset")?;
            let raw = bytes
                .get(start..start.checked_add(n).ok_or("Gemma tensor size")?)
                .ok_or("truncated Gemma tensor")?;
            if scalar {
                let count = dims.iter().product::<usize>();
                let mut decoded = [0.; 4096];
                if count > decoded.len()
                    || etalon_model::ggml::dequantize_row_into(
                        raw,
                        info.ggml_type,
                        count,
                        &mut decoded[..count],
                    ) != Ok(count)
                    || decoded[..count].iter().any(|v| !v.is_finite())
                {
                    return Err("Gemma scalar weights".into());
                }
            }
            Ok(Some(*info))
        };
        let token = index.token_embd.ok_or("Gemma token embedding")?;
        if token.n_dims != 2 || token.dims[1] == 0 || token.dims[1] > 262144 {
            return Err("Gemma vocabulary bound".into());
        }
        let vocab = token.dims[1] as usize;
        if take("token_embd.weight", &[p.hidden, vocab], false, false)? != index.token_embd
            || take("output.weight", &[p.hidden, vocab], false, true)? != index.output_weight
            || take("output_norm.weight", &[p.hidden], true, false)? != index.output_norm
        {
            return Err("Gemma cached global tensor mismatch".into());
        }
        let rope = take("rope_freqs.weight", &[p.head_dim / 2], true, false)?.unwrap();
        let (ple_projection, ple_norm) = if p.ple > 0 {
            if take(
                "per_layer_token_embd.weight",
                &[p.ple * p.layers, vocab],
                false,
                false,
            )? != index.ple_ngram_embedding
            {
                return Err("Gemma cached PLE tensor mismatch".into());
            }
            (
                take(
                    "per_layer_model_proj.weight",
                    &[p.hidden, p.ple * p.layers],
                    false,
                    false,
                )?,
                take("per_layer_proj_norm.weight", &[p.ple], true, false)?,
            )
        } else {
            if index.ple_ngram_embedding.is_some() {
                return Err("unexpected Gemma PLE table".into());
            }
            (None, None)
        };
        let mut layers: Vec<Layer> = Vec::with_capacity(p.layers);
        let mut total = 0usize;
        let mut last = [None, None];
        for l in 0..p.layers {
            let dim = p.dim(l);
            let kv = p.kv_heads[l];
            let own = l < p.layers - p.shared;
            let class = usize::from(p.sliding[l]);
            let mut get = |suffix: &str, dims: &[usize], scalar: bool, optional: bool| {
                take(&format!("blk.{l}.{suffix}"), dims, scalar, optional)
            };
            let mut t = LayerTensors {
                layer_idx: l as u32,
                ..Default::default()
            };
            t.attn_norm = get("attn_norm.weight", &[p.hidden], true, false)?;
            t.attn_q = get("attn_q.weight", &[p.hidden, p.heads * dim], false, false)?;
            t.attn_k = get("attn_k.weight", &[p.hidden, kv * dim], false, !own)?;
            t.attn_v = get(
                "attn_v.weight",
                &[p.hidden, kv * dim],
                false,
                !own || !p.sliding[l],
            )?;
            t.attn_q_norm = get("attn_q_norm.weight", &[dim], true, false)?;
            t.attn_k_norm = get("attn_k_norm.weight", &[dim], true, !own)?;
            t.attn_output = get(
                "attn_output.weight",
                &[p.heads * dim, p.hidden],
                false,
                false,
            )?;
            get("post_attention_norm.weight", &[p.hidden], true, false)?;
            t.ffn_norm = get("ffn_norm.weight", &[p.hidden], true, false)?;
            t.ffn_gate = get("ffn_gate.weight", &[p.hidden, p.ff[l]], false, false)?;
            t.ffn_up = get("ffn_up.weight", &[p.hidden, p.ff[l]], false, false)?;
            t.ffn_down = get("ffn_down.weight", &[p.ff[l], p.hidden], false, false)?;
            get("post_ffw_norm.weight", &[p.hidden], true, false)?;
            get("layer_output_scale.weight", &[1], true, true)?;
            if p.experts > 0 {
                t.moe_router = get("ffn_gate_inp.weight", &[p.hidden, p.experts], false, false)?;
                t.moe_shared_gate_input = get("ffn_gate_inp.scale", &[p.hidden], true, false)?;
                t.moe_gate_up_exps = get(
                    "ffn_gate_up_exps.weight",
                    &[p.hidden, 2 * p.expert_ff, p.experts],
                    false,
                    false,
                )?;
                t.moe_down_exps = get(
                    "ffn_down_exps.weight",
                    &[p.expert_ff, p.hidden, p.experts],
                    false,
                    false,
                )?;
                get("ffn_down_exps.scale", &[p.experts], true, false)?;
                for name in [
                    "pre_ffw_norm_2.weight",
                    "post_ffw_norm_1.weight",
                    "post_ffw_norm_2.weight",
                ] {
                    get(name, &[p.hidden], true, false)?;
                }
            }
            if p.ple > 0 {
                get("inp_gate.weight", &[p.hidden, p.ple], false, false)?;
                get("proj.weight", &[p.ple, p.hidden], false, false)?;
                get("post_norm.weight", &[p.hidden], true, false)?;
            }
            let (source, offset, slots) = if own {
                let slots = if p.sliding[l] {
                    context.min(p.window)
                } else {
                    context
                };
                let offset = total;
                total = total
                    .checked_add(
                        slots
                            .checked_mul(2 * kv * dim)
                            .ok_or("Gemma KV arithmetic")?,
                    )
                    .ok_or("Gemma KV arithmetic")?;
                last[class] = Some(l);
                (l, offset, slots)
            } else {
                let source = last[class].ok_or("missing Gemma shared-KV producer")?;
                let producer = &layers[source];
                if producer.dim != dim || producer.kv_heads != kv {
                    return Err("Gemma shared-KV geometry mismatch".into());
                }
                (source, producer.offset, producer.slots)
            };
            layers.push(Layer {
                tensors: t,
                source,
                offset,
                slots,
                dim,
                kv_heads: kv,
                sliding: p.sliding[l],
            });
        }
        if seen.len() != index.entries.len() {
            return Err("unknown Gemma graph tensor".into());
        }
        if total
            .checked_mul(4)
            .is_none_or(|n| n > crate::gguf_bridge::KV_CACHE_MAX_BYTES)
        {
            return Err("Gemma KV byte budget".into());
        }
        if p.experts > 0 {
            let r = index
                .moe_routing
                .admit(p.experts)
                .map_err(|e| format!("Gemma router: {e:?}"))?;
            if r.expert_count() != p.experts || r.experts_per_token() != p.topk {
                return Err("Gemma expert metadata mismatch".into());
            }
        }
        let h = &mut index.hyperparams;
        h.n_layer = p.layers as u32;
        h.n_embd = p.hidden as u32;
        h.n_head = p.heads as u32;
        h.n_kv_head = p.kv_heads[0] as u32;
        h.head_dim = p.head_dim as u32;
        h.head_dim_swa = p.swa_dim as u32;
        h.rope_dim = p.rope_dim as u32;
        h.rope_dim_swa = p.swa_rope_dim as u32;
        h.rope_freq_base = p.rope_base;
        h.rope_freq_base_swa = p.swa_base;
        h.rms_norm_eps = p.epsilon;
        h.ple_embd_per_layer = p.ple as u32;
        h.sliding_window = p.window as u32;
        h.shared_kv_layers = p.shared as u32;
        h.logit_softcap = p.softcap;
        Ok(Self {
            profile: p,
            index,
            layers,
            context,
            kv_floats: total,
            ple_projection,
            ple_norm,
            rope,
        })
    }
}
