use super::*;
use crate::p64::{integrity::crc32c, P64View};

pub const MODEL_METADATA_EXTENSION_BYTES: usize = 736;
const LOCATOR_MAGIC: &[u8; 8] = b"ETLMDTA\0";
const RECORD_MAGIC: &[u8; 8] = b"ETLIMD1\0";

impl ModelMetadata {
    pub fn needs_extension(&self) -> bool {
        let hp = self.hyperparams;
        self.moe_routing != Default::default()
            || hp.ssm_conv_kernel != 0
            || hp.ssm_state_size != 0
            || hp.ssm_group_count != 0
            || hp.ssm_time_step_rank != 0
            || hp.ssm_inner_size != 0
            || hp.full_attention_interval != 0
            || self.ple_config.is_some()
    }

    pub(super) fn write_legacy_extension_into(
        &self,
        out: &mut [u8],
    ) -> Result<usize, PrepareModelError> {
        let out = out
            .get_mut(..MODEL_METADATA_EXTENSION_BYTES)
            .ok_or(PrepareModelError::BudgetExceeded)?;
        out.fill(0);
        out[..8].copy_from_slice(RECORD_MAGIC);
        out[8..10].copy_from_slice(&1u16.to_le_bytes());
        out[12..16].copy_from_slice(&(MODEL_METADATA_EXTENSION_BYTES as u32).to_le_bytes());
        let mut cursor = 16;
        put(out, &mut cursor, &self.hyperparams.n_layer.to_le_bytes());
        put(out, &mut cursor, &self.hyperparams.n_embd.to_le_bytes());
        put(out, &mut cursor, &self.hyperparams.n_head.to_le_bytes());
        put(out, &mut cursor, &self.hyperparams.n_kv_head.to_le_bytes());
        put(
            out,
            &mut cursor,
            &self.hyperparams.rope_freq_base.to_le_bytes(),
        );
        put(out, &mut cursor, &self.hyperparams.rope_scale.to_le_bytes());
        put(out, &mut cursor, &self.hyperparams.head_dim.to_le_bytes());
        put(
            out,
            &mut cursor,
            &self.hyperparams.head_dim_swa.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.sliding_window.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.shared_kv_layers.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.logit_softcap.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.ssm_conv_kernel.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.ssm_state_size.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.ssm_group_count.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.ssm_time_step_rank.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.ssm_inner_size.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.full_attention_interval.to_le_bytes(),
        );
        put(
            out,
            &mut cursor,
            &self.hyperparams.architecture.to_le_bytes(),
        );
        put(out, &mut cursor, &self.hyperparams.arch_flags.to_le_bytes());
        put(
            out,
            &mut cursor,
            &u32::from(self.ple_config.is_some()).to_le_bytes(),
        );
        let ple = self.ple_config.unwrap_or_default();
        put(out, &mut cursor, &ple.ngram_size.to_le_bytes());
        put(out, &mut cursor, &ple.heads_per_ngram.to_le_bytes());
        put(out, &mut cursor, &ple.eos_token_id.to_le_bytes());
        put(out, &mut cursor, &ple.embedding_row_width.to_le_bytes());
        put(out, &mut cursor, &ple.layer_count.to_le_bytes());
        for value in ple.layers {
            put(out, &mut cursor, &value.to_le_bytes());
        }
        put(out, &mut cursor, &ple.multiplier_count.to_le_bytes());
        for value in ple.layer_multipliers {
            put(out, &mut cursor, &value.to_le_bytes());
        }
        put(out, &mut cursor, &ple.head_count.to_le_bytes());
        for value in ple.head_offsets {
            put(out, &mut cursor, &value.to_le_bytes());
        }
        for value in ple.head_vocab_sizes {
            put(out, &mut cursor, &value.to_le_bytes());
        }
        let crc = crc32c(&out[..cursor]);
        put(out, &mut cursor, &crc.to_le_bytes());
        Ok(cursor)
    }

    pub(super) fn read_legacy_extension(bytes: &[u8]) -> Result<Self, PrepareModelError> {
        if bytes.len() != MODEL_METADATA_EXTENSION_BYTES
            || bytes.get(..8) != Some(RECORD_MAGIC.as_slice())
            || bytes[8..12] != [1, 0, 0, 0]
            || u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize != bytes.len()
            || crc32c(&bytes[..bytes.len() - 4])
                != u32::from_le_bytes(bytes[bytes.len() - 4..].try_into().unwrap())
        {
            return Err(PrepareModelError::MetadataExtension);
        }
        let mut cursor = 16;
        let hyperparams = ModelHyperparams {
            n_layer: u32::from_le_bytes(take(bytes, &mut cursor)),
            n_embd: u32::from_le_bytes(take(bytes, &mut cursor)),
            n_head: u32::from_le_bytes(take(bytes, &mut cursor)),
            n_kv_head: u32::from_le_bytes(take(bytes, &mut cursor)),
            rope_freq_base: f32::from_le_bytes(take(bytes, &mut cursor)),
            rope_scale: f32::from_le_bytes(take(bytes, &mut cursor)),
            head_dim: u32::from_le_bytes(take(bytes, &mut cursor)),
            head_dim_swa: u32::from_le_bytes(take(bytes, &mut cursor)),
            sliding_window: u32::from_le_bytes(take(bytes, &mut cursor)),
            shared_kv_layers: u32::from_le_bytes(take(bytes, &mut cursor)),
            logit_softcap: f32::from_le_bytes(take(bytes, &mut cursor)),
            ssm_conv_kernel: u32::from_le_bytes(take(bytes, &mut cursor)),
            ssm_state_size: u32::from_le_bytes(take(bytes, &mut cursor)),
            ssm_group_count: u32::from_le_bytes(take(bytes, &mut cursor)),
            ssm_time_step_rank: u32::from_le_bytes(take(bytes, &mut cursor)),
            ssm_inner_size: u32::from_le_bytes(take(bytes, &mut cursor)),
            full_attention_interval: u32::from_le_bytes(take(bytes, &mut cursor)),
            architecture: u32::from_le_bytes(take(bytes, &mut cursor)),
            arch_flags: u32::from_le_bytes(take(bytes, &mut cursor)),
        };
        let present = u32::from_le_bytes(take(bytes, &mut cursor));
        if present > 1 {
            return Err(PrepareModelError::MetadataExtension);
        }
        let ple = Qwen4ExpPleConfig {
            ngram_size: u32::from_le_bytes(take(bytes, &mut cursor)),
            heads_per_ngram: u32::from_le_bytes(take(bytes, &mut cursor)),
            eos_token_id: u32::from_le_bytes(take(bytes, &mut cursor)),
            embedding_row_width: u32::from_le_bytes(take(bytes, &mut cursor)),
            layer_count: u32::from_le_bytes(take(bytes, &mut cursor)),
            layers: core::array::from_fn(|_| u32::from_le_bytes(take(bytes, &mut cursor))),
            multiplier_count: u32::from_le_bytes(take(bytes, &mut cursor)),
            layer_multipliers: core::array::from_fn(|_| {
                i64::from_le_bytes(take(bytes, &mut cursor))
            }),
            head_count: u32::from_le_bytes(take(bytes, &mut cursor)),
            head_offsets: core::array::from_fn(|_| i64::from_le_bytes(take(bytes, &mut cursor))),
            head_vocab_sizes: core::array::from_fn(|_| {
                i64::from_le_bytes(take(bytes, &mut cursor))
            }),
        };
        if present == 0 && ple != Qwen4ExpPleConfig::default() {
            return Err(PrepareModelError::MetadataExtension);
        }
        Ok(Self {
            hyperparams,
            ple_config: (present == 1).then_some(ple),
            moe_routing: Default::default(),
        })
    }
}

fn put(out: &mut [u8], cursor: &mut usize, bytes: &[u8]) {
    out[*cursor..*cursor + bytes.len()].copy_from_slice(bytes);
    *cursor += bytes.len();
}
fn take<const N: usize>(bytes: &[u8], cursor: &mut usize) -> [u8; N] {
    let value = bytes[*cursor..*cursor + N].try_into().unwrap();
    *cursor += N;
    value
}

/// Locate the appended record before planning. The returned locator occupies reserved20.
pub fn metadata_extension_locator(tokenizer: &[u8]) -> Result<[u8; 20], PrepareModelError> {
    let mut reserved = [0; 20];
    if tokenizer.len() < MODEL_METADATA_EXTENSION_BYTES {
        return Ok(reserved);
    }
    if tokenizer.len() >= MODEL_ROUTING_EXTENSION_BYTES {
        let start = tokenizer.len() - MODEL_ROUTING_EXTENSION_BYTES;
        if tokenizer.get(start..start + 8) == Some(b"ETLMOE2\0") {
            ModelMetadata::read_extension(&tokenizer[start..])?;
            reserved[..8].copy_from_slice(LOCATOR_MAGIC);
            reserved[8..12].copy_from_slice(
                &u32::try_from(start)
                    .map_err(|_| PrepareModelError::BudgetExceeded)?
                    .to_le_bytes(),
            );
            reserved[12..16].copy_from_slice(&(MODEL_ROUTING_EXTENSION_BYTES as u32).to_le_bytes());
            reserved[16..18].copy_from_slice(&2u16.to_le_bytes());
            return Ok(reserved);
        }
    }
    let start = tokenizer.len() - MODEL_METADATA_EXTENSION_BYTES;
    if tokenizer.get(start..start + 8) != Some(RECORD_MAGIC.as_slice()) {
        return Ok(reserved);
    }
    ModelMetadata::read_extension(&tokenizer[start..])?;
    reserved[..8].copy_from_slice(LOCATOR_MAGIC);
    reserved[8..12].copy_from_slice(
        &u32::try_from(start)
            .map_err(|_| PrepareModelError::BudgetExceeded)?
            .to_le_bytes(),
    );
    reserved[12..16].copy_from_slice(&(MODEL_METADATA_EXTENSION_BYTES as u32).to_le_bytes());
    reserved[16..18].copy_from_slice(&1u16.to_le_bytes());
    Ok(reserved)
}

pub(super) fn container_metadata(
    view: &P64View<'_>,
) -> Result<Option<ModelMetadata>, PrepareModelError> {
    let reserved = &view.header().reserved;
    if &reserved[..8] != LOCATOR_MAGIC {
        if reserved.iter().any(|&byte| byte != 0) {
            return Err(PrepareModelError::UnsupportedMetadata);
        }
        return Ok(None);
    }
    if !matches!(reserved[16..20], [1, 0, 0, 0] | [2, 0, 0, 0]) {
        return Err(PrepareModelError::MetadataExtension);
    }
    let start = u32::from_le_bytes(reserved[8..12].try_into().unwrap()) as usize;
    let len = u32::from_le_bytes(reserved[12..16].try_into().unwrap()) as usize;
    let end = start
        .checked_add(len)
        .ok_or(PrepareModelError::MetadataExtension)?;
    let record = view
        .tokenizer_bytes()
        .get(start..end)
        .ok_or(PrepareModelError::MetadataExtension)?;
    if (reserved[16] == 1 && len != MODEL_METADATA_EXTENSION_BYTES)
        || (reserved[16] == 2 && len != MODEL_ROUTING_EXTENSION_BYTES)
    {
        return Err(PrepareModelError::MetadataExtension);
    }
    ModelMetadata::read_extension(record).map(Some)
}
