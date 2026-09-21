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
    let mut block_selected = [0usize; CACHE];
    let mut block_scores = [0.0f32; CACHE];
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
        block_selected: &mut block_selected,
        block_scores: &mut block_scores,
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

/// Real multi-token Qwen4Exp generation: prompt → token embedding → PLE →
/// all 48 trained layers (HC → GDN|QSA → HC → MoE) → final HC mixer →
/// streamed `output.weight` argmax → native tokenizer decode.  Every step
/// streams rows from the C: PLE payload and E: trunk; no model residency.
#[allow(clippy::too_many_arguments)]
pub fn run_decode_qwen4exp(
    package: &Path,
    prompt: Option<&str>,
    token_ids: &[u32],
    max_tokens: usize,
    context: usize,
    chat: bool,
    trunk_mmap: bool,
) -> Result<(), String> {
    use qualia_core_db::inference::qwen::{
        decode_tokens, Qwen4ExpDecodeReceipt, Qwen4ExpDecodeScratch, Qwen4ExpSession,
        Qwen4ExpTrunkResidency,
    };

    let residency = if trunk_mmap {
        Qwen4ExpTrunkResidency::Mapped
    } else {
        Qwen4ExpTrunkResidency::Streamed
    };
    let mut runtime =
        qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate_with_residency(
            package, residency,
        )
        .map_err(|error| error.to_string())?;

    // The tokenizer lives in the bounded GGUF header prefix (KV section
    // ends before the tensor index); read exactly that many bytes.
    let source = runtime
        .descriptor
        .source_beside(package)
        .map_err(|error| error.to_string())?;
    let header_len = runtime.index.tensor_data_start as usize;
    let mut header = vec![0u8; header_len];
    {
        use std::io::Read;
        let mut file = std::fs::File::open(&source)
            .map_err(|error| format!("open source GGUF for tokenizer: {error}"))?;
        file.read_exact(&mut header)
            .map_err(|error| format!("read GGUF header for tokenizer: {error}"))?;
    }
    let tokenizer = qualia_core_db::gguf_sharder::GgufTokenizer::from_gguf(&header);

    let prompt_tokens: Vec<u32> = if !token_ids.is_empty() {
        token_ids.to_vec()
    } else if let Some(text) = prompt {
        if chat {
            tokenizer.encode_chat_prompt(text)
        } else {
            tokenizer.encode_prompt(text)
        }
    } else {
        return Err("decode-qwen4exp needs --prompt or --token-ids".to_string());
    };
    if prompt_tokens.is_empty() {
        return Err("prompt produced no tokens".to_string());
    }
    if prompt_tokens.len() + max_tokens > context {
        return Err(format!(
            "prompt ({} tokens) + max_tokens ({max_tokens}) exceeds QSA cache capacity {context}",
            prompt_tokens.len()
        ));
    }

    let hidden = runtime.index.emb_dim();
    let mut session = Qwen4ExpSession::new(&runtime.index, context)
        .map_err(|error| error.to_string())?;
    let mut scratch = Qwen4ExpDecodeScratch::new(hidden);
    let stops: Vec<u32> = tokenizer.stop_tokens().to_vec();
    let mut receipt = Qwen4ExpDecodeReceipt::default();
    let mut trace = Vec::new();
    decode_tokens(
        &mut runtime,
        &mut session,
        &mut scratch,
        &prompt_tokens,
        max_tokens,
        &|id| stops.contains(&id),
        &mut receipt,
        &mut trace,
    )
    .map_err(|error| format!("Qwen4Exp decode: {error}"))?;

    let generated = receipt.generated_tokens.len();
    let text = tokenizer.decode(&receipt.generated_tokens);
    let secs = receipt.elapsed.as_secs_f64();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp native multi-token decode (real execution)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ prompt tokens: {}", receipt.prompt_tokens);
    println!("├─ generated tokens: {generated}  ids={:?}", receipt.generated_tokens);
    println!("├─ decoded text: {text}");
    println!("├─ elapsed: {secs:.3}s  ({:.3} tokens/s generated)",
        generated as f64 / secs.max(1e-9));
    println!(
        "├─ trunk {}: {} reads, {} rows, {:.2} MiB",
        if trunk_mmap { "access (mmap)" } else { "I/O" },
        receipt.trunk_reads,
        receipt.trunk_rows,
        receipt.trunk_bytes as f64 / (1024.0 * 1024.0)
    );
    println!(
        "├─ PLE I/O: {} reads, {} rows, {:.2} MiB",
        receipt.ple_reads,
        receipt.ple_rows,
        receipt.ple_bytes as f64 / (1024.0 * 1024.0)
    );
    println!(
        "└─ resident state: {:.2} MiB persistent + {:.2} MiB scratch",
        receipt.state_bytes as f64 / (1024.0 * 1024.0),
        receipt.scratch_bytes as f64 / (1024.0 * 1024.0)
    );
    Ok(())
}

/// Lab instrument: trace one real token step end-to-end.  Every earlier
/// prompt token runs through the full graph first so the traced step sees
/// the same recurrent/cache state generation would; the traced step then
/// reports a per-layer fingerprint/RMS/abs-max table plus the top-k
/// vocabulary logits — this localizes which stage corrupts the forward
/// signal instead of asserting aggregate correctness.
pub fn run_probe_qwen4exp_trace(
    package: &Path,
    prompt: Option<&str>,
    token_ids: &[u32],
    chat: bool,
    context: usize,
    topk: usize,
    rank_tokens: &[u32],
    dump: Option<&Path>,
) -> Result<(), String> {
    use qualia_core_db::inference::qwen::{
        decode_step, Qwen4ExpDecodeScratch, Qwen4ExpSession, StreamedArgmax,
        QWEN4EXP_TRACE_META_LAYER,
    };

    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let source = runtime
        .descriptor
        .source_beside(package)
        .map_err(|error| error.to_string())?;
    let header_len = runtime.index.tensor_data_start as usize;
    let mut header = vec![0u8; header_len];
    {
        use std::io::Read;
        let mut file = std::fs::File::open(&source)
            .map_err(|error| format!("open source GGUF for tokenizer: {error}"))?;
        file.read_exact(&mut header)
            .map_err(|error| format!("read GGUF header for tokenizer: {error}"))?;
    }
    let tokenizer = qualia_core_db::gguf_sharder::GgufTokenizer::from_gguf(&header);

    let prompt_tokens: Vec<u32> = if !token_ids.is_empty() {
        token_ids.to_vec()
    } else if let Some(text) = prompt {
        if chat {
            tokenizer.encode_chat_prompt(text)
        } else {
            tokenizer.encode_prompt(text)
        }
    } else {
        return Err("probe-qwen4exp-trace needs --prompt or --token-ids".to_string());
    };
    if prompt_tokens.is_empty() {
        return Err("prompt produced no tokens".to_string());
    }
    if prompt_tokens.len() > context {
        return Err(format!(
            "prompt ({} tokens) exceeds QSA cache capacity {context}",
            prompt_tokens.len()
        ));
    }

    let hidden = runtime.index.emb_dim();
    let mut session = Qwen4ExpSession::new(&runtime.index, context)
        .map_err(|error| error.to_string())?;
    let mut scratch = Qwen4ExpDecodeScratch::new(hidden);

    let start = std::time::Instant::now();
    let (&traced_token, prefix) = prompt_tokens.split_last().unwrap();
    for &token in prefix {
        decode_step(
            &mut runtime,
            &mut session,
            &mut scratch,
            token,
            &mut Vec::new(),
            false,
        )
        .map_err(|error| format!("prefix decode: {error}"))?;
    }
    let mut trace = Vec::new();
    let winner = decode_step(
        &mut runtime,
        &mut session,
        &mut scratch,
        traced_token,
        &mut trace,
        dump.is_some(),
    )
    .map_err(|error| format!("traced decode: {error}"))?;

    let logits = *runtime
        .index
        .logits_projection_info()
        .ok_or("missing output.weight tensor")?;
    let mut top = vec![
        StreamedArgmax {
            token_id: 0,
            logit: f32::NEG_INFINITY,
        };
        topk.max(1)
    ];
    let mut raw = vec![0u8; 65_536];
    let mut row = vec![0f32; hidden];
    runtime
        .trunk
        .topk_rows(&logits, scratch.final_mixed(), &mut raw, &mut row, &mut top)
        .map_err(|error| format!("top-k projection: {error}"))?;
    let mut ranks = vec![(0.0f32, 0u64); rank_tokens.len()];
    if !rank_tokens.is_empty() {
        runtime
            .trunk
            .logits_and_ranks(
                &logits,
                scratch.final_mixed(),
                rank_tokens,
                &mut raw,
                &mut row,
                &mut ranks,
            )
            .map_err(|error| format!("rank projection: {error}"))?;
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp per-stage trace (one real token step)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ prompt tokens: {}  traced token: {traced_token}", prompt_tokens.len());
    println!("├─ prompt ids: {:?}", prompt_tokens);
    println!("├─ step wall time: {:.3}s", start.elapsed().as_secs_f64());
    println!("├─ layer  stage           fingerprint          rms        abs_max");
    for record in &trace {
        let layer = if record.layer == QWEN4EXP_TRACE_META_LAYER {
            " -- ".to_string()
        } else {
            format!("{:4}", record.layer)
        };
        println!(
            "│  {}  {:<14} {:016x}  {:<10.4} {:<10.4}",
            layer, record.stage, record.fingerprint, record.rms, record.abs_max
        );
    }
    println!("├─ argmax: id={} logit={:.4}", winner.token_id, winner.logit);
    for (rank, entry) in top.iter().enumerate() {
        let text = tokenizer.decode(&[entry.token_id]);
        println!(
            "│  top{rank}: id={} logit={:.4} text={text:?}",
            entry.token_id, entry.logit
        );
    }
    for (index, (&token, &(logit, rank))) in rank_tokens.iter().zip(ranks.iter()).enumerate() {
        let text = tokenizer.decode(&[token]);
        println!("│  probe{index}: id={token} logit={logit:.4} rank={rank} text={text:?}");
    }
    if let Some(path) = dump {
        // Stage dump: u32 layer, u32 stage-name len, name bytes, u32 value
        // count, then little-endian f32 values.  Numpy parses it directly for
        // per-stage comparison against a reference forward pass.
        use std::io::Write;
        let mut file = std::fs::File::create(path)
            .map_err(|error| format!("create dump {}: {error}", path.display()))?;
        for record in &trace {
            file.write_all(&record.layer.to_le_bytes())
                .and_then(|()| file.write_all(&(record.stage.len() as u32).to_le_bytes()))
                .and_then(|()| file.write_all(record.stage.as_bytes()))
                .and_then(|()| file.write_all(&(record.values.len() as u32).to_le_bytes()))
                .and_then(|()| {
                    for value in &record.values {
                        file.write_all(&value.to_le_bytes())?;
                    }
                    Ok(())
                })
                .map_err(|error| format!("write dump {}: {error}", path.display()))?;
        }
        println!("├─ dumped {} stage vectors to {}", trace.len(), path.display());
    }
    println!("└─ done");
    Ok(())
}
