//! Cold immutable Qwen input closure for the authenticated native product route.
use std::sync::{atomic::{AtomicU64, Ordering}, Arc};
pub use etalon_model_io::lease::BundleLimits;
use etalon_model_io::lease::{bundle_digest, ModelLease, ModelPart, ModelPartRole};
use crate::gguf_sharder::{GgufTensorIndex, GgufTensorInfo, GgufTokenizer};
use super::{validate_qwen4exp_graph,
    PleNvmeReader, Qwen4ExpNativeRuntime, Qwen4ExpTrunkResidency, TrunkNvmeReader};

static GENERATION: AtomicU64 = AtomicU64::new(1);

/// Independently verified leases are selected by the host, never derived from
/// descriptor path claims. The tokenizer is parsed from the retained trunk KV.
pub struct QwenRetainedSources {
    pub package: Arc<ModelLease>,
    pub trunk: Arc<ModelLease>,
    pub ple: Arc<ModelLease>,
}

/// This owner exposes no mutable runtime, tokenizer or source replacement API.
/// Dropping it disposes every stateful reader; rejected decode owns its session.
pub struct QwenRetainedRuntime {
    pub(crate) runtime: Qwen4ExpNativeRuntime,
    pub(crate) tokenizer: Arc<GgufTokenizer>,
    _sources: QwenRetainedSources,
    digest: [u8; 32],
    generation: u64,
}
impl QwenRetainedRuntime {
    pub fn activate(
        sources: QwenRetainedSources,
        limits: BundleLimits,
        metadata_bytes: usize,
        scratch_bytes: usize,
        residency: Qwen4ExpTrunkResidency,
    ) -> Result<Self, String> {
        // No tile source is active in this scalar driver. Its source set is
        // closed: external tile/tokenizer overrides cannot enter this owner.
        let parts = [
            ModelPart { role: ModelPartRole::BaseModel, format_version: 1, ordinal: 0, lease: &sources.trunk },
            ModelPart { role: ModelPartRole::Shard, format_version: 1, ordinal: 0, lease: &sources.ple },
            ModelPart { role: ModelPartRole::Configuration, format_version: 1, ordinal: 0, lease: &sources.package },
        ];
        let digest = bundle_digest(&parts, limits).map_err(|e| format!("Qwen bundle: {e:?}"))?;
        if sources.package.bytes().len() > metadata_bytes || metadata_bytes < 24 {
            return Err("Qwen descriptor metadata budget exceeded".into());
        }
        let descriptor = super::metadata::descriptor(sources.package.bytes(), metadata_bytes)?;
        if descriptor.source.byte_len != sources.trunk.identity().byte_len
            || descriptor.ple_storage.byte_len != sources.ple.identity().byte_len
            || (!descriptor.source.sha256.is_empty()
                && descriptor.source.sha256.as_slice() != sources.trunk.identity().content_sha256)
        { return Err("Qwen retained source identity disagrees with descriptor".into()); }
        let header = &sources.trunk.bytes()[..metadata_bytes.min(sources.trunk.bytes().len())];
        super::metadata::gguf(header, metadata_bytes)?;
        let index = GgufTensorIndex::from_gguf(header);
        if index.tensor_data_start == 0 || index.tensor_data_start as usize > header.len() {
            return Err("Qwen GGUF header exceeds selected metadata budget".into());
        }
        let mut contracts = [Default::default(); 64];
        let graph = validate_qwen4exp_graph(&index, &mut contracts).map_err(|e| e.to_string())?;
        let info = index.ple_ngram_embedding_info().ok_or("Qwen PLE tensor missing")?;
        if graph.layers != descriptor.n_layer || index.ple_config != Some(descriptor.ple_config)
            || info.dims[0] != descriptor.ple.row_width || info.dims[1] != descriptor.ple.rows
            || info.ggml_type != descriptor.ple.ggml_type
        { return Err("Qwen retained graph disagrees with descriptor".into()); }
        let ple_info = GgufTensorInfo { byte_offset: 0, ..*info };
        let ple = PleNvmeReader::from_lease(sources.ple.clone(), ple_info).map_err(|e| format!("Qwen PLE: {e:?}"))?;
        if ple.raw_row_bytes() as u64 != descriptor.ple.row_bytes {
            return Err("Qwen PLE row width disagrees with descriptor".into());
        }
        let trunk = TrunkNvmeReader::from_lease(sources.trunk.clone(), index.tensor_data_start,
            residency == Qwen4ExpTrunkResidency::Mapped, scratch_bytes).map_err(|e| e.to_string())?;
        let tokenizer = Arc::new(GgufTokenizer::from_gguf(&header[..index.tensor_data_start as usize]));
        let generation = GENERATION.fetch_update(Ordering::AcqRel, Ordering::Acquire,
            |v| v.checked_add(1)).map_err(|_| "Qwen generation exhausted")?;
        Ok(Self { runtime: Qwen4ExpNativeRuntime { descriptor, index, ple, trunk },
            tokenizer, _sources: sources, digest, generation })
    }
    pub fn model_digest(&self) -> [u8; 32] { self.digest }
    pub fn generation(&self) -> u64 { self.generation }
    pub fn tokenizer_owner(&self) -> Arc<GgufTokenizer> { self.tokenizer.clone() }
    pub fn tokenizer(&self) -> &GgufTokenizer { &self.tokenizer }
}
