//! Retained private historical compiler.
use super::*;
#[allow(dead_code)]
fn compile_gguf_to_p64_legacy(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(input);
    if idx.tensor_data_start == 0 && idx.hyperparams.n_layer == 0 {
        return Err("GGUF parse failed or yielded no tensor metadata".to_string());
    }
    let page_log2 = if page_log2 == 0 {
        12 // 4096 default
    } else {
        page_log2
    };
    if page_log2 < 8 || page_log2 > 30 {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let n_layer = idx.hyperparams.n_layer;
    let tds = idx.tensor_data_start as usize;

    let mut planned: Vec<(u16, u16, crate::gguf_sharder::GgufTensorInfo)> = Vec::new();
    let mut push = |role_id: u16, layer: u16, t: Option<crate::gguf_sharder::GgufTensorInfo>| {
        if let Some(info) = t {
            planned.push((role_id, layer, info));
        }
    };
    for layer in 0..n_layer {
        let t = idx.get_layer_tensors(layer);
        let l = layer as u16;
        push(P64_ROLE_ATTN_NORM, l, t.attn_norm);
        push(P64_ROLE_ATTN_Q, l, t.attn_q);
        push(P64_ROLE_ATTN_K, l, t.attn_k);
        push(P64_ROLE_ATTN_V, l, t.attn_v);
        push(P64_ROLE_ATTN_OUTPUT, l, t.attn_output);
        push(P64_ROLE_FFN_NORM, l, t.ffn_norm);
        push(P64_ROLE_FFN_GATE, l, t.ffn_gate);
        push(P64_ROLE_FFN_UP, l, t.ffn_up);
        push(P64_ROLE_FFN_DOWN, l, t.ffn_down);
    }
    push(
        P64_ROLE_TOKEN_EMBD,
        P64_LAYER_GLOBAL,
        idx.token_embd_info().copied(),
    );
    push(
        P64_ROLE_OUTPUT_NORM,
        P64_LAYER_GLOBAL,
        idx.output_norm_info().copied(),
    );
    push(
        P64_ROLE_OUTPUT,
        P64_LAYER_GLOBAL,
        idx.output_weight_info().copied(),
    );

    // Filter missing/invalid tensors
    planned.retain(|(_, _, info)| info.dims[0] > 0);

    let names = planned
        .iter()
        .map(|(role, layer, info)| {
            if let Some(suffix) = p64_role_suffix(*role) {
                if *layer == P64_LAYER_GLOBAL {
                    String::from_utf8_lossy(suffix).to_string()
                } else {
                    format!("blk.{}.{}", layer, String::from_utf8_lossy(suffix))
                }
            } else {
                format!("tensor_{}", info.byte_offset)
            }
        })
        .collect::<Vec<_>>();
    let tok = crate::gguf_sharder::GgufTokenizer::from_gguf(input);
    let count = idx
        .hyperparams
        .n_layer
        .checked_add(1)
        .ok_or("p64: manifold count overflow")? as usize;
    let mut manifolds = Vec::with_capacity(
        count
            .checked_mul(40)
            .ok_or("p64: manifold table overflow")?,
    );
    for layer in 0..count {
        let coordinate = if layer == idx.hyperparams.n_layer as usize {
            crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(0, 1)
        } else {
            crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
                layer as u32,
                idx.hyperparams.n_layer,
            )
        };
        for v in coordinate.as_f32_array() {
            manifolds.extend_from_slice(&v.to_le_bytes());
        }
    }
    let hparams = idx.hyperparams.into();
    etalon_model::p64::convert::compile_legacy(
        input,
        page_log2,
        etalon_model::p64::convert::LegacyInput {
            tensor_data_start: tds,
            hparams: &hparams,
            tensors: &planned,
            names: &names,
            vocab: &tok.vocab,
            manifold_bytes: &manifolds,
        },
        Default::default(),
    )
}
