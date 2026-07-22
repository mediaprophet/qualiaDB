//! Runtime **reader / loader**: `P64TensorIndex` (parse + integrity policy), weight/tokenizer/
//! manifold access, GGUF-equivalent index synthesis for the hot path, GGUF round-trip validation,
//! `IntegrityMode`, and the convert-layout recommender.

use super::*;
use crate::gguf_sharder::{GgufHyperparams, GgufTensorIndex, GgufTensorInfo};

// CRC-32C (Castagnoli, reflected) — delegated to the shared
// `container_10d::crc32c` module (P0.3 consolidation). The algorithm is
// byte-identical to the previous in-line implementation; the p64 round-trip
// tests verify the checksums are unchanged after delegation.
use crate::container_10d::crc32c::crc32c;

/// Runtime reader: parses a P64 container's header + manifest in microseconds. Tensor blobs
/// stay in the caller's byte slice (zero-copy); only the small manifest is materialized. The
/// `role`/`layer`/`blob_offset` fields map directly to the resident WebGPU weight arenas.
#[derive(Clone)]
pub struct P64TensorIndex {
    pub header: P64WeightHeader,
    pub hparams: P64HParams,
    pub entries: Vec<P64TensorEntry>,
}

pub type Q42TensorIndex = P64TensorIndex;

/// How thoroughly [`P64TensorIndex::from_p64`] verifies integrity.
///
/// Full tensor CRCs over a 360M–3B container dominate activate latency (toolkit probe:
/// ~3 s after table CRC optim, previously ~42 s table-less). Convert and audit use
/// [`IntegrityMode::Full`]; hot activate may use [`IntegrityMode::Metadata`] after a
/// trusted convert wrote the container on this machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IntegrityMode {
    /// Metadata CRC + every tensor blob CRC (safest; slow on large models).
    Full,
    /// Metadata CRC only; still bounds-checks tensor ranges (**default activate**).
    #[default]
    Metadata,
    /// Structure checks only — no CRC. Prefer only for tests / trusted mmap.
    Structure,
}

impl IntegrityMode {
    /// `QUALIA_P64_INTEGRITY=full|metadata|structure`.
    ///
    /// **Default is [`Metadata`]** (bounds + metadata CRC only): convert already sealed
    /// per-tensor CRCs, and full re-scan dominated activate (~2.4 s → ~9 ms on SmolLM2).
    /// Use `full` for audit / untrusted download; `structure` for tests only.
    pub fn from_env() -> Self {
        match std::env::var("QUALIA_P64_INTEGRITY")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(|s| s.to_ascii_lowercase())
            .as_deref()
        {
            Some("full") | Some("strict") | Some("all") => Self::Full,
            Some("structure") | Some("struct") | Some("skip") | Some("none") => Self::Structure,
            Some("metadata") | Some("meta") | Some("fast") | None => Self::Metadata,
            Some(_) => Self::Metadata,
        }
    }
}

/// Recommend convert layout from source size (bytes on disk) and a VRAM headroom budget.
///
/// Heuristic:
/// 1. **F16Expand** when Q4→f16 estimate (`source * 4`) fits under `vram * 0.55` and the
///    4 GiB p64 offset cap — best decode path for small/mid models.
/// 2. Else **Q4kSoa** for mid/large Q4 containers (SoA is ~11% larger than Q4_K; always
///    under the f16 budget). Decode GEMV drops scale-unpack barriers.
/// 3. Else **Verbatim**.
pub fn recommend_convert_layout(source_bytes: u64, vram_budget_bytes: u64) -> P64ConvertLayout {
    let est_f16 = source_bytes.saturating_mul(4);
    let room = (vram_budget_bytes as f64 * 0.55) as u64;
    if est_f16 > 0 && est_f16 < room && est_f16 < (4u64 << 30) {
        P64ConvertLayout::F16Expand
    } else if source_bytes > 256 * 1024 * 1024 {
        // >256 MiB: almost certainly Q4-class weights where SoA pays off.
        P64ConvertLayout::Q4kSoa
    } else {
        P64ConvertLayout::Verbatim
    }
}

impl P64TensorIndex {
    pub fn from_q42(data: &[u8]) -> Result<Self, String> {
        Self::from_p64(data)
    }
    pub fn from_p64(data: &[u8]) -> Result<Self, String> {
        Self::from_p64_with_integrity(data, IntegrityMode::from_env())
    }

    /// Parse a P64 container with an explicit integrity policy.
    pub fn from_p64_with_integrity(data: &[u8], integrity: IntegrityMode) -> Result<Self, String> {
        let header = P64WeightHeader::read_le(data)?;
        if header.magic != P64_MAGIC {
            return Err("p64: invalid magic".to_string());
        }
        if header.version != P64_VERSION {
            return Err(format!("p64: unsupported version {}", header.version));
        }
        if header.flags & P64_FLAG_LITTLE_ENDIAN == 0 {
            return Err("p64: non-little-endian container is unsupported".to_string());
        }
        let page = header.page_size as usize;
        if page < 256 || !page.is_power_of_two() {
            return Err("p64: invalid page size".to_string());
        }
        let hparams_start = header.hparams_offset as usize;
        let hparams_end = hparams_start
            .checked_add(64)
            .ok_or("p64: hyperparameter offset overflow")?;
        if hparams_end > data.len() {
            return Err("p64: hyperparameters out of bounds".to_string());
        }
        let hparams = P64HParams::read_le(&data[hparams_start..hparams_end])?;

        let tensor_count = header.tensor_count as usize;
        let tensor_table_start = header.tensor_table_offset as usize;
        let tensor_table_end = tensor_table_start
            .checked_add(
                tensor_count
                    .checked_mul(P64_TENSOR_ENTRY_BYTES)
                    .ok_or("p64: tensor table overflow")?,
            )
            .ok_or("p64: tensor table overflow")?;
        let string_table_start = header.string_table_offset as usize;
        let manifold_table_start = header.manifold_table_offset as usize;
        let manifold_count = hparams
            .n_layer
            .checked_add(1)
            .ok_or("p64: manifold count overflow")? as usize;
        let manifold_table_end = manifold_table_start
            .checked_add(
                manifold_count
                    .checked_mul(P64_MANIFOLD_ENTRY_BYTES)
                    .ok_or("p64: manifold table overflow")?,
            )
            .ok_or("p64: manifold table overflow")?;
        let tokenizer_start = header.tokenizer_offset as usize;
        let checksum_start = header.checksum_offset as usize;
        let checksum_end = checksum_start
            .checked_add(
                tensor_count
                    .checked_add(1)
                    .and_then(|count| count.checked_mul(4))
                    .ok_or("p64: checksum table overflow")?,
            )
            .ok_or("p64: checksum table overflow")?;
        if tensor_table_start < hparams_end
            || tensor_table_end > string_table_start
            || string_table_start > manifold_table_start
            || manifold_table_end > tokenizer_start
            || tokenizer_start > checksum_start
            || checksum_end > data.len()
        {
            return Err("p64: metadata sections overlap or are out of bounds".to_string());
        }
        if manifold_table_start % 64 != 0 {
            return Err("p64: manifold table is not cache-line aligned".to_string());
        }
        if !matches!(integrity, IntegrityMode::Structure) {
            let stored_metadata_crc =
                u32::from_le_bytes(data[checksum_start..checksum_start + 4].try_into().unwrap());
            if crc32c(&data[..checksum_start]) != stored_metadata_crc {
                return Err("p64: metadata CRC-32C mismatch".to_string());
            }
        }

        let mut entries = Vec::with_capacity(tensor_count);
        let mut cursor = header.tensor_table_offset as usize;
        let blob_floor = align_up(checksum_end, page);
        let mut previous_blob_end = blob_floor;
        let mut previous_manifold_idx = None;
        let layer_packed = header.flags & P64_FLAG_LAYER_PACK != 0;
        let verify_tensor_crc = matches!(integrity, IntegrityMode::Full);
        for tensor_index in 0..tensor_count {
            if cursor + P64_TENSOR_ENTRY_BYTES > data.len() {
                return Err("p64: truncated tensor table".to_string());
            }
            let bytes = &data[cursor..cursor + P64_TENSOR_ENTRY_BYTES];
            let eu32 = |o: usize| u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
            let eu16 = |o: usize| u16::from_le_bytes(bytes[o..o + 2].try_into().unwrap());
            let eu64 = |o: usize| u64::from_le_bytes(bytes[o..o + 8].try_into().unwrap());
            let entry = P64TensorEntry {
                name_offset: eu32(0),
                role_id: eu16(4),
                dtype: eu16(6),
                manifold_idx: eu32(8),
                rank: eu32(12),
                dimensions: [eu32(16), eu32(20), eu32(24), eu32(28)],
                blob_offset: eu32(32),
                blob_size: eu32(36),
                source_offset: eu64(40),
                source_name_hash: eu64(48),
                alt_dtype: eu16(56),
                precision_views_mask: eu16(58),
                alt_blob_offset: eu32(60),
            };
            if !(1..=4).contains(&entry.rank) {
                return Err(format!("p64: tensor {tensor_index} has invalid rank"));
            }
            if entry.manifold_idx as usize >= manifold_count {
                return Err(format!(
                    "p64: tensor {tensor_index} has invalid manifold index"
                ));
            }
            let name_start = string_table_start
                .checked_add(entry.name_offset as usize)
                .ok_or("p64: tensor name offset overflow")?;
            if name_start >= manifold_table_start
                || !data[name_start..manifold_table_start].contains(&0)
            {
                return Err(format!("p64: tensor {tensor_index} has invalid name"));
            }
            let blob_start = entry.blob_offset as usize;
            let blob_end = blob_start
                .checked_add(entry.blob_size as usize)
                .ok_or("p64: tensor blob overflow")?;
            let required_alignment = if layer_packed
                && tensor_index > 0
                && previous_manifold_idx == Some(entry.manifold_idx)
            {
                256
            } else {
                page
            };
            if blob_start % required_alignment != 0
                || blob_start < previous_blob_end
                || blob_end > data.len()
            {
                return Err(format!(
                    "p64: tensor {tensor_index} is unaligned, overlapping, or out of bounds"
                ));
            }
            if verify_tensor_crc {
                let crc_start = checksum_start + 4 + tensor_index * 4;
                let stored_crc =
                    u32::from_le_bytes(data[crc_start..crc_start + 4].try_into().unwrap());
                if crc32c(&data[blob_start..blob_end]) != stored_crc {
                    return Err(format!("p64: tensor {tensor_index} CRC-32C mismatch"));
                }
            }
            previous_blob_end = blob_end;
            previous_manifold_idx = Some(entry.manifold_idx);
            entries.push(entry);
            cursor += P64_TENSOR_ENTRY_BYTES;
        }
        Ok(Self {
            header,
            hparams,
            entries,
        })
    }

    pub fn hyperparams(&self) -> GgufHyperparams {
        GgufHyperparams {
            n_layer: self.hparams.n_layer,
            n_embd: self.hparams.n_embd,
            n_head: self.hparams.n_head,
            n_kv_head: self.hparams.n_kv_head,
            rope_freq_base: self.hparams.rope_freq_base,
            rope_scale: self.hparams.rope_scale,
            head_dim: self.hparams.head_dim,
            head_dim_swa: self.hparams.head_dim_swa,
            sliding_window: self.hparams.sliding_window,
            shared_kv_layers: self.hparams.shared_kv_layers,
            logit_softcap: self.hparams.logit_softcap,
            architecture: self.hparams.architecture,
            arch_flags: self.hparams.arch_flags,
        }
    }

    pub fn blob<'a>(&self, data: &'a [u8], entry: &P64TensorEntry) -> &'a [u8] {
        let start = entry.blob_offset as usize;
        &data[start..start + entry.blob_size as usize]
    }

    pub fn tokenizer_bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        let start = self.header.tokenizer_offset as usize;
        let end = self.header.checksum_offset as usize;
        if start <= data.len() && end <= data.len() && start <= end {
            &data[start..end]
        } else {
            &[]
        }
    }

    pub fn manifold_coordinate(
        &self,
        data: &[u8],
        index: u32,
    ) -> Result<crate::modalities::manifold::ManifoldCoordinate10D, String> {
        if index > self.hparams.n_layer {
            return Err("p64: manifold index out of bounds".to_string());
        }
        let start = (self.header.manifold_table_offset as usize)
            .checked_add(index as usize * P64_MANIFOLD_ENTRY_BYTES)
            .ok_or("p64: manifold offset overflow")?;
        let end = start + P64_MANIFOLD_ENTRY_BYTES;
        if end > data.len() {
            return Err("p64: manifold coordinate out of bounds".to_string());
        }
        crate::modalities::manifold::ManifoldCoordinate10D::from_p64_bytes(&data[start..end])
    }

    /// Build a GGUF-equivalent index for the existing inference hot path.
    pub fn to_gguf_index(&self) -> GgufTensorIndex {
        let mut named: Vec<(Vec<u8>, GgufTensorInfo)> = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let name = if entry.manifold_idx == self.hparams.n_layer {
                match entry.role_id {
                    P64_ROLE_TOKEN_EMBD => b"token_embd.weight".to_vec(),
                    P64_ROLE_OUTPUT => b"output.weight".to_vec(),
                    P64_ROLE_OUTPUT_NORM => b"output_norm.weight".to_vec(),
                    _ => continue,
                }
            } else if let Some(suffix) = p64_role_suffix(entry.role_id) {
                let mut buffer = [0u8; 96];
                let length = crate::gguf_sharder::write_blk_tensor_name(
                    entry.manifold_idx,
                    suffix,
                    &mut buffer,
                );
                buffer[..length].to_vec()
            } else {
                continue;
            };
            named.push((
                name,
                GgufTensorInfo {
                    dims: entry.dimensions.map(u64::from),
                    n_dims: entry.rank,
                    ggml_type: entry.dtype as u32,
                    byte_offset: entry.blob_offset as u64,
                },
            ));
        }
        let references: Vec<(&[u8], GgufTensorInfo)> = named
            .iter()
            .map(|(name, info)| (name.as_slice(), *info))
            .collect();
        GgufTensorIndex::from_components(&references, self.hyperparams(), 0)
    }

    /// Prove that a parsed P64 contains every GGUF tensor with identical shape,
    /// type and bytes. This is a cold-path validation routine and intentionally
    /// performs a complete byte comparison in addition to the stored CRCs.
    pub fn validate_against_gguf(
        &self,
        p64_data: &[u8],
        gguf_data: &[u8],
    ) -> Result<P64RoundTripReport, String> {
        let source = GgufTensorIndex::from_gguf(gguf_data);
        if source.entries.len() != self.entries.len() {
            return Err(format!(
                "p64: tensor count differs (GGUF {}, P64 {})",
                source.entries.len(),
                self.entries.len()
            ));
        }
        let source_data_start = source.tensor_data_start as usize;
        let mut tensor_bytes = 0u64;
        for (position, entry) in self.entries.iter().enumerate() {
            let (source_name_hash, source_info) = source
                .entries
                .iter()
                .find(|(_, info)| info.byte_offset == entry.source_offset)
                .ok_or_else(|| format!("p64: tensor {position} source offset is missing"))?;
            if *source_name_hash != entry.source_name_hash
                || source_info.ggml_type != entry.dtype as u32
                || source_info.n_dims != entry.rank
                || source_info.dims != entry.dimensions.map(u64::from)
            {
                return Err(format!("p64: tensor {position} metadata differs from GGUF"));
            }
            let source_len = crate::ggml_quants::tensor_byte_len(source_info)
                .ok_or_else(|| format!("p64: tensor {position} has unsupported source type"))?;
            if source_len != entry.blob_size as usize {
                return Err(format!("p64: tensor {position} byte length differs"));
            }
            let source_start = source_data_start + source_info.byte_offset as usize;
            let source_end = source_start + source_len;
            if source_end > gguf_data.len()
                || self.blob(p64_data, entry) != &gguf_data[source_start..source_end]
            {
                return Err(format!("p64: tensor {position} bytes differ from GGUF"));
            }
            tensor_bytes += source_len as u64;
        }
        Ok(P64RoundTripReport {
            tensor_count: self.entries.len(),
            tensor_bytes,
            manifold_count: self.hparams.n_layer as usize + 1,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct P64RoundTripReport {
    pub tensor_count: usize,
    pub tensor_bytes: u64,
    pub manifold_count: usize,
}
