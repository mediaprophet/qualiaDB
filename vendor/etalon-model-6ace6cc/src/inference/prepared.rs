use super::*;
use crate::p64::{IntegrityMode, P64Error, P64HParams, P64View};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelPreparationBudget {
    pub metadata_bytes: usize,
    pub tensor_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModelMetadata {
    pub hyperparams: ModelHyperparams,
    pub ple_config: Option<Qwen4ExpPleConfig>,
    pub moe_routing: moe_routing::MoeRoutingSettings,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrepareModelError {
    Container(P64Error),
    BudgetExceeded,
    AllocationFailed,
    MetadataMismatch,
    MetadataExtension,
    RequiredMetadataMissing,
    TensorPayload(usize),
    UnsupportedStorage,
    UnsupportedMetadata,
}

impl core::fmt::Display for PrepareModelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "model preparation error: {self:?}")
    }
}
impl std::error::Error for PrepareModelError {}

/// Native P64 model: cold owned directory and borrowed authoritative container bytes.
/// Main descriptors use absolute offsets; all raw precision and extension fields remain in view().
#[derive(Clone, Debug)]
pub struct PreparedP64<'a, H = ModelHyperparams> {
    view: P64View<'a>,
    index: TensorIndex<H>,
}

impl<'a, H: From<ModelHyperparams>> PreparedP64<'a, H> {
    pub fn parse(
        bytes: &'a [u8],
        integrity: IntegrityMode,
        budget: ModelPreparationBudget,
        metadata: Option<ModelMetadata>,
    ) -> Result<Self, PrepareModelError> {
        let view = P64View::parse(bytes, integrity).map_err(PrepareModelError::Container)?;
        if view.header().flags & !0x7f != 0 {
            return Err(PrepareModelError::UnsupportedStorage);
        }
        let count = view.tensor_count();
        let required = count
            .checked_mul(core::mem::size_of::<(u64, TensorInfo)>())
            .and_then(|n| n.checked_add(core::mem::size_of::<Self>()))
            .ok_or(PrepareModelError::BudgetExceeded)?;
        if count > budget.tensor_count || required > budget.metadata_bytes {
            return Err(PrepareModelError::BudgetExceeded);
        }
        let stored = ModelHyperparams::from(*view.hparams());
        let stored_metadata = super::extension::container_metadata(&view)?;
        if let (Some(stored), Some(supplied)) = (stored_metadata, metadata) {
            if stored != supplied {
                return Err(PrepareModelError::MetadataMismatch);
            }
        }
        let metadata = stored_metadata.or(metadata);
        if let Some(metadata) = metadata {
            super::moe_metadata::validate(metadata.moe_routing)?;
        }
        let (hyperparams, ple_config) = match metadata {
            Some(metadata) => {
                if !metadata.hyperparams.matches_p64(view.hparams()) {
                    return Err(PrepareModelError::MetadataMismatch);
                }
                (metadata.hyperparams, metadata.ple_config)
            }
            None => (stored, None),
        };
        if matches!(
            hyperparams.architecture,
            ARCH_QWEN4EXP | ARCH_QWEN35_MOE | ARCH_GRANITE_HYBRID
        ) && (hyperparams.ssm_conv_kernel == 0
            || hyperparams.ssm_state_size == 0
            || hyperparams.ssm_group_count == 0
            || hyperparams.ssm_time_step_rank == 0
            || hyperparams.ssm_inner_size == 0)
        {
            return Err(PrepareModelError::RequiredMetadataMissing);
        }
        if hyperparams.architecture == ARCH_QWEN4EXP && !ple_config.is_some_and(|c| c.is_complete())
        {
            return Err(PrepareModelError::RequiredMetadataMissing);
        }
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(count)
            .map_err(|_| PrepareModelError::AllocationFailed)?;
        let mut max_tensor_bytes = 0;
        let mut max_layer_tensor_bytes = 0;
        for i in 0..count {
            let tensor = view.tensor(i).map_err(PrepareModelError::Container)?;
            let entry = tensor.entry();
            let info = TensorInfo {
                dims: entry.dimensions.map(u64::from),
                n_dims: entry.rank,
                ggml_type: u32::from(entry.dtype),
                byte_offset: u64::from(entry.blob_offset),
            };
            if let Some(required) = crate::ggml::tensor_byte_len(&info) {
                if required > tensor.blob().len() {
                    return Err(PrepareModelError::TensorPayload(i));
                }
            }
            let name = tensor.name_bytes();
            let mut canonical = [0u8; 128];
            let n = canonical_component_name(name, &mut canonical);
            let lookup_name = if n == 0 { name } else { &canonical[..n] };
            let name_hash = synthetic_tensor_hash(name)
                .filter(|hash| *hash == entry.source_name_hash)
                .unwrap_or_else(|| tensor_name_hash(lookup_name));
            entries.push((name_hash, info));
            max_tensor_bytes = max_tensor_bytes.max(entry.blob_size as usize);
            if is_layer_matmul_tensor_name(lookup_name) {
                max_layer_tensor_bytes = max_layer_tensor_bytes.max(entry.blob_size as usize);
            }
        }
        let mut index = TensorIndex::from_entries(
            entries,
            hyperparams.into(),
            0,
            max_tensor_bytes,
            max_layer_tensor_bytes,
        );
        index.ple_config = ple_config;
        index.moe_routing = metadata.map(|m| m.moe_routing).unwrap_or_default();
        Ok(Self { view, index })
    }

    pub fn index(&self) -> &TensorIndex<H> {
        &self.index
    }
    pub fn view(&self) -> &P64View<'a> {
        &self.view
    }
    pub fn into_index(self) -> TensorIndex<H> {
        self.index
    }
}

impl From<P64HParams> for ModelHyperparams {
    fn from(value: P64HParams) -> Self {
        Self {
            n_layer: value.n_layer,
            n_embd: value.n_embd,
            n_head: value.n_head,
            n_kv_head: value.n_kv_head,
            rope_freq_base: value.rope_freq_base,
            rope_scale: value.rope_scale,
            head_dim: value.head_dim,
            head_dim_swa: value.head_dim_swa,
            sliding_window: value.sliding_window,
            shared_kv_layers: value.shared_kv_layers,
            logit_softcap: value.logit_softcap,
            architecture: value.architecture,
            arch_flags: value.arch_flags,
            ..Self::default()
        }
    }
}

impl ModelHyperparams {
    fn matches_p64(&self, stored: &P64HParams) -> bool {
        let common = Self::from(*stored);
        self.n_layer == common.n_layer
            && self.n_embd == common.n_embd
            && self.n_head == common.n_head
            && self.effective_n_kv_head() == common.effective_n_kv_head()
            && self.effective_rope_freq_base().to_bits()
                == common.effective_rope_freq_base().to_bits()
            && self.effective_rope_scale().to_bits() == common.effective_rope_scale().to_bits()
            && self.head_dim == common.head_dim
            && self.head_dim_swa == common.head_dim_swa
            && self.sliding_window == common.sliding_window
            && self.shared_kv_layers == common.shared_kv_layers
            && self.logit_softcap.to_bits() == common.logit_softcap.to_bits()
            && self.architecture == common.architecture
            && self.arch_flags == common.arch_flags
    }
}
