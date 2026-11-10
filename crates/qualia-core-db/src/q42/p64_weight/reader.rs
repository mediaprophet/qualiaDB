//! Runtime **reader / loader**: `P64TensorIndex` (parse + integrity policy), weight/tokenizer/
//! manifold access, GGUF-equivalent index synthesis for the hot path, GGUF round-trip validation,
//! `IntegrityMode`, and the convert-layout recommender.

use super::*;
use crate::gguf_sharder::{GgufHyperparams, GgufTensorIndex};
pub use etalon_model::p64::convert::P64RoundTripReport;

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
    /// **Default is `Metadata()`** (bounds + metadata CRC only): convert already sealed
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
    /// Prepare the native inference directory while retaining authoritative P64 bytes.
    /// Metadata extensions are explicit because the fixed v4 header has no recurrent/PLE fields.
    pub fn prepare_native<'a>(
        data: &'a [u8],
        integrity: IntegrityMode,
        budget: etalon_model::inference::ModelPreparationBudget,
        metadata: Option<etalon_model::inference::ModelMetadata>,
    ) -> Result<
        etalon_model::inference::PreparedP64<'a, GgufHyperparams>,
        etalon_model::inference::PrepareModelError,
    > {
        let integrity = match integrity {
            IntegrityMode::Full => etalon_model::p64::IntegrityMode::Full,
            IntegrityMode::Metadata => etalon_model::p64::IntegrityMode::Metadata,
            IntegrityMode::Structure => etalon_model::p64::IntegrityMode::Structure,
        };
        etalon_model::inference::PreparedP64::parse(data, integrity, budget, metadata)
    }

    /// Parses a tensor index from a `.q42` volume blob; `Err` on malformed header/directory.
    pub fn from_q42(data: &[u8]) -> Result<Self, String> {
        Self::from_p64(data)
    }
    /// Parses a tensor index from raw `.p64` bytes.
    pub fn from_p64(data: &[u8]) -> Result<Self, String> {
        Self::from_p64_with_integrity(data, IntegrityMode::from_env())
    }

    /// Parse a P64 container with an explicit integrity policy.
    pub fn from_p64_with_integrity(data: &[u8], integrity: IntegrityMode) -> Result<Self, String> {
        super::etalon_adapter::parse_owned(data, integrity)
    }

    /// Model hyperparameters decoded from the index header.
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
            ssm_conv_kernel: 0,
            ssm_state_size: 0,
            ssm_group_count: 0,
            ssm_time_step_rank: 0,
            ssm_inner_size: 0,
            full_attention_interval: 0,
            architecture: self.hparams.architecture,
            arch_flags: self.hparams.arch_flags,
            ..GgufHyperparams::default()
        }
    }

    /// Zero-copy tensor byte slice for `entry` within `data`.
    #[inline]
    pub fn blob<'a>(&self, data: &'a [u8], entry: &P64TensorEntry) -> &'a [u8] {
        entry.blob_slice(data).unwrap_or_else(|| {
            let start = entry.blob_offset as usize;
            &data[start..start + entry.blob_size as usize]
        })
    }

    /// Zero-copy tokenizer blob within `data`.
    #[inline]
    pub fn tokenizer_bytes<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        self.header.tokenizer_slice(data)
    }

    /// Manifold coordinate for a tensor entry (bridges neutral 64B P64 record to Q42 `ManifoldCoordinate10D`).
    pub fn manifold_coordinate(
        &self,
        data: &[u8],
        index: u32,
    ) -> Result<crate::modalities::manifold::ManifoldCoordinate10D, String> {
        let record = self
            .header
            .manifold_record_slice(data, self.hparams.n_layer, index)?;
        crate::modalities::manifold::ManifoldCoordinate10D::from_p64_bytes(record)
    }

    /// Build a GGUF-equivalent index for the existing inference hot path.
    pub fn to_gguf_index(&self) -> GgufTensorIndex {
        etalon_model::p64::convert::synthesize_gguf_index(
            &self.entries,
            self.hparams.n_layer,
            self.hyperparams(),
        )
    }

    /// Prove that a parsed P64 contains every GGUF tensor with identical shape,
    /// type and bytes. This is a cold-path validation routine and intentionally
    /// performs a complete byte comparison in addition to the stored CRCs.
    pub fn validate_against_gguf(
        &self,
        p64_data: &[u8],
        gguf_data: &[u8],
    ) -> Result<P64RoundTripReport, String> {
        etalon_model::p64::convert::validate_p64_against_gguf(
            &self.entries,
            self.hparams.n_layer,
            p64_data,
            gguf_data,
        )
    }
}
