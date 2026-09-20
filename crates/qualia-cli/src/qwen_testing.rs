//! Physical, read-only Qwen4Exp forward-component probes.

use std::path::Path;

/// Execute a fully connected trained recurrent/MoE layer. The explicit bounded
/// scratch belongs to the CLI decode owner; core hot operators receive only
/// caller-owned slices.
pub fn run_probe_qwen4exp_gdn_moe_layer(
    package: &Path,
    layer_id: u32,
    token_id: u32,
) -> Result<(), String> {
    const HIDDEN: usize = 2_560;
    const WIDE: usize = HIDDEN * 4;
    const HEADS: usize = 48;
    const HEAD_DIM: usize = 128;
    const INNER: usize = HEADS * HEAD_DIM;
    const QKV: usize = 10_240;
    const RANK: usize = 320;
    const EXPERTS: usize = 512;
    const INTERMEDIATE: usize = 640;
    const RAW: usize = 65_536;
    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let embedding = *runtime
        .index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    if embedding.dims[0] as usize != HIDDEN || token_id as u64 >= embedding.dims[1] {
        return Err("Qwen4Exp token embedding does not match the layer contract".to_string());
    }
    let layer = runtime.index.get_layer_tensors(layer_id);
    if !layer.is_hybrid_ssm_layer() {
        return Err(format!(
            "layer {layer_id} is not a Qwen4Exp GatedDeltaNet layer"
        ));
    }
    let mut embedding_raw = [0u8; RAW];
    let mut embedding_values = [0.0f32; HIDDEN];
    runtime
        .trunk
        .read_row_into(
            &embedding,
            token_id as usize,
            &mut embedding_raw,
            &mut embedding_values,
        )
        .map_err(|error| format!("read token embedding: {error}"))?;
    let mut residual = vec![0.0f32; WIDE];
    for stream in 0..4 {
        residual[stream * HIDDEN..(stream + 1) * HIDDEN].copy_from_slice(&embedding_values);
    }
    let mut attn_raw = [0u8; RAW];
    let mut ffn_raw = [0u8; RAW];
    let mut gdn_raw = [0u8; RAW];
    let mut moe_raw = [0u8; RAW];
    let mut attn_projection = vec![0.0f32; WIDE];
    let mut ffn_projection = vec![0.0f32; WIDE];
    let mut attn_norm = vec![0.0f32; WIDE];
    let mut ffn_norm = vec![0.0f32; WIDE];
    let mut attn_normalized = vec![0.0f32; WIDE];
    let mut ffn_normalized = vec![0.0f32; WIDE];
    let mut attn_low_rank = vec![0.0f32; RANK];
    let mut ffn_low_rank = vec![0.0f32; RANK];
    let mut attn_gates = vec![0.0f32; WIDE];
    let mut ffn_gates = vec![0.0f32; WIDE];
    let mut convolution = vec![0.0f32; QKV * 3];
    let mut delta = vec![0.0f32; HEADS * HEAD_DIM * HEAD_DIM];
    let mut gdn_projection = vec![0.0f32; INNER];
    let mut qkv = vec![0.0f32; QKV];
    let mut gate = vec![0.0f32; INNER];
    let mut alpha = [0.0f32; HEADS];
    let mut beta = [0.0f32; HEADS];
    let mut convolved = vec![0.0f32; QKV];
    let mut head_norm = [0.0f32; HEAD_DIM];
    let mut head_vector = [0.0f32; HEAD_DIM];
    let mut output_inner = vec![0.0f32; INNER];
    let mut head_a = [0.0f32; HEADS];
    let mut dt_bias = [0.0f32; HEADS];
    let mut moe_dequantized = vec![0.0f32; HIDDEN];
    let mut router_logits = vec![0.0f32; EXPERTS];
    let mut top_indices = [0usize; 10];
    let mut top_logits = [0.0f32; 10];
    let mut top_weights = [0.0f32; 10];
    let mut moe_gate = vec![0.0f32; INTERMEDIATE];
    let mut moe_up = vec![0.0f32; INTERMEDIATE];
    let mut activation = vec![0.0f32; INTERMEDIATE];
    let mut expert_out = vec![0.0f32; HIDDEN];
    let mut attn_mixed = [0.0f32; HIDDEN];
    let mut gdn_branch = [0.0f32; HIDDEN];
    let mut ffn_mixed = [0.0f32; HIDDEN];
    let mut moe_branch = [0.0f32; HIDDEN];
    let mut attn_inject = [0.0f32; 4];
    let mut ffn_inject = [0.0f32; 4];
    let mut state = qualia_core_db::inference::qwen::GatedDeltaState {
        convolution: &mut convolution,
        delta: &mut delta,
    };
    let mut buffers = qualia_core_db::inference::qwen::StreamedGdnMoeLayerBuffers {
        attn_hyper: qualia_core_db::inference::qwen::StreamedHyperBuffers {
            raw_row: &mut attn_raw,
            projection_row: &mut attn_projection,
            norm_weight: &mut attn_norm,
            normalized: &mut attn_normalized,
            low_rank: &mut attn_low_rank,
            gates: &mut attn_gates,
        },
        ffn_hyper: qualia_core_db::inference::qwen::StreamedHyperBuffers {
            raw_row: &mut ffn_raw,
            projection_row: &mut ffn_projection,
            norm_weight: &mut ffn_norm,
            normalized: &mut ffn_normalized,
            low_rank: &mut ffn_low_rank,
            gates: &mut ffn_gates,
        },
        gdn: qualia_core_db::inference::qwen::GatedDeltaBuffers {
            raw_row: &mut gdn_raw,
            projection_row: &mut gdn_projection,
            qkv: &mut qkv,
            gate: &mut gate,
            alpha: &mut alpha,
            beta: &mut beta,
            convolved: &mut convolved,
            head_norm: &mut head_norm,
            head_vector: &mut head_vector,
            output_inner: &mut output_inner,
            head_a: &mut head_a,
            dt_bias: &mut dt_bias,
        },
        moe: qualia_core_db::inference::qwen::StreamedMoeBuffers {
            raw_row: &mut moe_raw,
            dequantized_row: &mut moe_dequantized,
            router_logits: &mut router_logits,
            top_indices: &mut top_indices,
            top_logits: &mut top_logits,
            top_weights: &mut top_weights,
            gate: &mut moe_gate,
            up: &mut moe_up,
            activation: &mut activation,
            expert_out: &mut expert_out,
        },
        attn_mixed: &mut attn_mixed,
        token_mixer_branch: &mut gdn_branch,
        ffn_mixed: &mut ffn_mixed,
        moe_branch: &mut moe_branch,
        attn_inject: &mut attn_inject,
        ffn_inject: &mut ffn_inject,
    };
    let start = std::time::Instant::now();
    qualia_core_db::inference::qwen::execute_streamed_gdn_moe_layer(
        &mut runtime.trunk,
        &layer,
        &mut residual,
        &mut state,
        &mut buffers,
    )
    .map_err(|error| format!("execute trained Qwen4Exp layer: {error}"))?;
    let elapsed = start.elapsed();
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in residual.iter().chain(delta.iter().step_by(257)) {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp complete trained recurrent/MoE layer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ layer {layer_id}, token {token_id}: Hyper → GDN → Hyper → top-10 MoE");
    println!("├─ complete layer time: {:.3}s", elapsed.as_secs_f64());
    println!("└─ residual/state fingerprint={fingerprint:016x}");
    Ok(())
}

/// Execute the real QSA projection, four-cell indexer and sparse causal GQA
/// path at position zero.  It is deliberately a component probe: its cache is
/// local to this call and no text/token-rate claim is made from it.
pub fn run_probe_qwen4exp_qsa(package: &Path, layer_id: u32, token_id: u32) -> Result<(), String> {
    const HIDDEN: usize = 2_560;
    const QUERY: usize = 6_144;
    const KV: usize = 512;
    const INDEX_QUERY: usize = 512;
    const INDEX_KEY: usize = 128;
    const CACHE: usize = 16;
    const RAW: usize = 65_536;
    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let embedding = *runtime
        .index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    if embedding.dims[0] as usize != HIDDEN || token_id as u64 >= embedding.dims[1] {
        return Err("Qwen4Exp token embedding does not match the QSA contract".to_string());
    }
    let layer = runtime.index.get_layer_tensors(layer_id);
    if !layer.has_qwen_sparse_attention() {
        return Err(format!(
            "layer {layer_id} is not a complete Qwen4Exp QSA layer"
        ));
    }
    let mut embedding_raw = [0u8; RAW];
    let mut input = [0.0f32; HIDDEN];
    runtime
        .trunk
        .read_row_into(
            &embedding,
            token_id as usize,
            &mut embedding_raw,
            &mut input,
        )
        .map_err(|error| format!("read token embedding: {error}"))?;
    let mut keys = vec![0.0f32; CACHE * KV];
    let mut values = vec![0.0f32; CACHE * KV];
    let mut indexer_keys = vec![0.0f32; CACHE * INDEX_KEY];
    let mut raw = [0u8; RAW];
    let mut projection = vec![0.0f32; QUERY];
    let mut q_full = vec![0.0f32; QUERY * 2];
    let mut key = [0.0f32; KV];
    let mut value = [0.0f32; KV];
    let mut index_query = [0.0f32; INDEX_QUERY];
    let mut index_key = [0.0f32; INDEX_KEY];
    let mut q_norm = [0.0f32; 256];
    let mut k_norm = [0.0f32; 256];
    let mut index_q_norm = [0.0f32; INDEX_KEY];
    let mut index_k_norm = [0.0f32; INDEX_KEY];
    let mut selected = [0usize; CACHE];
    let mut scores = [0.0f32; CACHE];
    let mut attention = [0.0f32; QUERY];
    let mut out = [0.0f32; HIDDEN];
    let mut state = qualia_core_db::inference::qwen::QsaState {
        keys: &mut keys,
        values: &mut values,
        indexer_keys: &mut indexer_keys,
        tokens: 0,
    };
    let mut buffers = qualia_core_db::inference::qwen::StreamedQsaBuffers {
        raw_row: &mut raw,
        projection_row: &mut projection,
        q_full: &mut q_full,
        key: &mut key,
        value: &mut value,
        index_query: &mut index_query,
        index_key: &mut index_key,
        q_norm: &mut q_norm,
        k_norm: &mut k_norm,
        index_q_norm: &mut index_q_norm,
        index_k_norm: &mut index_k_norm,
        selected: &mut selected,
        scores: &mut scores,
        attention: &mut attention,
    };
    let start = std::time::Instant::now();
    qualia_core_db::inference::qwen::execute_streamed_qsa(
        &mut runtime.trunk,
        &layer,
        &input,
        &mut state,
        runtime.index.hyperparams.effective_rope_freq_base(),
        &mut out,
        &mut buffers,
    )
    .map_err(|error| format!("execute trained Qwen4Exp QSA: {error}"))?;
    let elapsed = start.elapsed();
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in out.iter().chain(keys.iter().take(KV)) {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp trained sparse-attention token mixer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ layer {layer_id}, token {token_id}: raw index key → four-cell QSA → GQA");
    println!("├─ QSA component time: {:.3}s", elapsed.as_secs_f64());
    println!("└─ branch/cache fingerprint={fingerprint:016x}");
    Ok(())
}
