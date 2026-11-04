//! Numeric directory hashing: explicit fields and float bits, never Debug text.
use super::*;

pub(super) fn engine_metadata(engine: &QTensorEngine) -> Result<[u8; 32], String> {
    let index = engine
        .tensor_index_cache
        .as_ref()
        .ok_or("missing prepared tensor index")?;
    let mut hash = Sha256::new();
    hash.update(b"qualia:native-numeric-directory:v1\0");
    let hp = engine.hyperparams;
    for value in [
        hp.n_layer,
        hp.n_embd,
        hp.n_head,
        hp.n_kv_head,
        hp.rope_freq_base.to_bits(),
        hp.rope_scale.to_bits(),
        hp.head_dim,
        hp.head_dim_swa,
        hp.sliding_window,
        hp.shared_kv_layers,
        hp.logit_softcap.to_bits(),
        hp.ssm_conv_kernel,
        hp.ssm_state_size,
        hp.ssm_group_count,
        hp.ssm_time_step_rank,
        hp.ssm_inner_size,
        hp.full_attention_interval,
        hp.architecture,
        hp.arch_flags,
    ] {
        hash.update(value.to_le_bytes());
    }
    // The engine and directory may not disagree about parsed configuration.
    if index.hyperparams != hp {
        return Err("inconsistent model hyperparameters".into());
    }
    hash.update(engine.tensor_data_offset.to_le_bytes());
    hash.update(index.tensor_data_start.to_le_bytes());
    hash.update((index.entries.len() as u64).to_le_bytes());
    for (name, info) in &index.entries {
        hash.update(name.to_le_bytes());
        tensor(&mut hash, Some(*info));
    }
    for info in [
        index.token_embd,
        index.output_weight,
        index.output_norm,
        index.ple_ngram_embedding,
    ] {
        tensor(&mut hash, info);
    }
    if let Some(config) = index.ple_config {
        hash.update([1]);
        for value in [
            config.ngram_size,
            config.heads_per_ngram,
            config.eos_token_id,
            config.embedding_row_width,
            config.layer_count,
            config.multiplier_count,
            config.head_count,
        ] {
            hash.update(value.to_le_bytes());
        }
        for value in config.layers {
            hash.update(value.to_le_bytes());
        }
        for value in config.layer_multipliers {
            hash.update(value.to_le_bytes());
        }
        for value in config.head_offsets {
            hash.update(value.to_le_bytes());
        }
        for value in config.head_vocab_sizes {
            hash.update(value.to_le_bytes());
        }
    } else {
        hash.update([0]);
    }
    Ok(hash.finalize().into())
}

fn tensor(hash: &mut Sha256, info: Option<GgufTensorInfo>) {
    if let Some(info) = info {
        hash.update([1]);
        for dimension in info.dims {
            hash.update(dimension.to_le_bytes());
        }
        hash.update(info.n_dims.to_le_bytes());
        hash.update(info.ggml_type.to_le_bytes());
        hash.update(info.byte_offset.to_le_bytes());
    } else {
        hash.update([0]);
    }
}
