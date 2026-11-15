use super::Profile;
use crate::gguf_sharder::{GgufTensorIndex, LayerTensors};
use etalon_model::inference::TensorInfo;

pub(super) fn validate(
    index: &GgufTensorIndex,
    p: Profile,
    bytes: &[u8],
) -> Result<Vec<LayerTensors>, String> {
    if index.entries.len() > 4096 {
        return Err("Qwen tensor count bound".into());
    }
    let mut admitted = std::collections::BTreeSet::new();
    let mut get = |name: &[u8], dims: &[usize], scalar: bool| -> Result<TensorInfo, String> {
        let hash = etalon_model::gguf::fnv1a64(name);
        let mut matches = index.entries.iter().filter(|(key, _)| *key == hash);
        let info = matches
            .next()
            .ok_or_else(|| format!("missing {}", String::from_utf8_lossy(name)))?
            .1;
        if matches.next().is_some()
            || !admitted.insert(hash)
            || info.n_dims as usize != dims.len()
            || dims
                .iter()
                .enumerate()
                .any(|(i, n)| info.dims[i] != *n as u64)
            || (scalar && info.ggml_type != 0)
            || (!scalar && !matches!(info.ggml_type, 0 | 1 | 8 | 12 | 13 | 14))
        {
            return Err(format!(
                "unsupported Qwen tensor {}",
                String::from_utf8_lossy(name)
            ));
        }
        let size = etalon_model::ggml::tensor_byte_len(&info).ok_or("Qwen tensor size")?;
        let start = index
            .tensor_data_start
            .checked_add(info.byte_offset)
            .ok_or("Qwen offset")?;
        let start = usize::try_from(start).map_err(|_| "Qwen offset")?;
        let raw = bytes
            .get(start..start.checked_add(size).ok_or("Qwen size")?)
            .ok_or("Qwen truncated payload")?;
        if scalar
            && raw
                .chunks_exact(4)
                .any(|b| !f32::from_le_bytes(b.try_into().unwrap()).is_finite())
        {
            return Err("nonfinite Qwen scalar weight".into());
        }
        if name.ends_with(b".ssm_a")
            && raw
                .chunks_exact(4)
                .any(|b| f32::from_le_bytes(b.try_into().unwrap()) >= 0.)
        {
            return Err("Qwen GGUF requires negative exported A".into());
        }
        Ok(info)
    };
    let embedding = index.token_embd.ok_or("missing Qwen embedding")?;
    if embedding.n_dims != 2 || embedding.dims[1] == 0 || embedding.dims[1] > 262144 {
        return Err("unsupported Qwen vocabulary".into());
    }
    let vocab = embedding.dims[1] as usize;
    get(b"token_embd.weight", &[p.hidden, vocab], false)?;
    get(b"output_norm.weight", &[p.hidden], true)?;
    if index.output_weight.is_some() {
        get(b"output.weight", &[p.hidden, vocab], false)?;
    }
    let h = index.hyperparams;
    let q = h.n_head as usize * h.head_dim() as usize;
    let kv = h.effective_n_kv_head() as usize * h.head_dim() as usize;
    let mut layers = Vec::with_capacity(p.layers);
    for layer in 0..p.layers {
        let mut tensor = |suffix: &str, dims: &[usize], scalar| {
            get(format!("blk.{layer}.{suffix}").as_bytes(), dims, scalar).map(Some)
        };
        let mut t = LayerTensors {
            layer_idx: layer as u32,
            ..LayerTensors::default()
        };
        t.attn_norm = tensor("attn_norm.weight", &[p.hidden], true)?;
        t.ffn_norm = tensor("post_attention_norm.weight", &[p.hidden], true)?;
        t.ffn_gate = tensor("ffn_gate.weight", &[p.hidden, p.ff], false)?;
        t.ffn_up = tensor("ffn_up.weight", &[p.hidden, p.ff], false)?;
        t.ffn_down = tensor("ffn_down.weight", &[p.ff, p.hidden], false)?;
        if p.linear(layer) {
            t.attn_qkv = tensor("attn_qkv.weight", &[p.hidden, p.channels], false)?;
            t.attn_gate = tensor("attn_gate.weight", &[p.hidden, p.value], false)?;
            t.ssm_alpha = tensor("ssm_alpha.weight", &[p.hidden, p.vh], false)?;
            t.ssm_beta = tensor("ssm_beta.weight", &[p.hidden, p.vh], false)?;
            t.ssm_conv1d = tensor("ssm_conv1d.weight", &[4, p.channels], true)?;
            t.ssm_a = tensor("ssm_a", &[p.vh], true)?;
            t.ssm_dt_bias = tensor("ssm_dt.bias", &[p.vh], true)?;
            t.ssm_norm = tensor("ssm_norm.weight", &[p.vd], true)?;
            t.ssm_out = tensor("ssm_out.weight", &[p.value, p.hidden], false)?;
        } else {
            t.attn_q = tensor("attn_q.weight", &[p.hidden, 2 * q], false)?;
            t.attn_k = tensor("attn_k.weight", &[p.hidden, kv], false)?;
            t.attn_v = tensor("attn_v.weight", &[p.hidden, kv], false)?;
            t.attn_q_norm = tensor("attn_q_norm.weight", &[h.head_dim() as usize], true)?;
            t.attn_k_norm = tensor("attn_k_norm.weight", &[h.head_dim() as usize], true)?;
            t.attn_output = tensor("attn_output.weight", &[q, p.hidden], false)?;
        }
        layers.push(t);
    }
    // Unknown graph tensors cannot silently turn into a different execution graph.
    if admitted.len() != index.entries.len() {
        return Err("unexpected Qwen graph tensor".into());
    }
    Ok(layers)
}
