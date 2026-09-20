//! Activation for a Qwen4Exp package with an NVMe-resident PLE payload.
//!
//! Activation reads a bounded GGUF metadata header from the external trunk on
//! E: and opens two ordinary file cursors: C: for PLE row gathers and E: for
//! selected trunk projections.  It deliberately never creates a whole-model
//! mmap or a swap-backed copy.

use std::path::Path;

use crate::gguf_sharder::{GgufHeaderError, GgufTensorIndex};

use super::{
    load_qwen4exp_native_hmc, validate_qwen4exp_graph, PleNvmeError, PleNvmeReader,
    Qwen4ExpGraphError, Qwen4ExpNativeAssetError, Qwen4ExpNativeDescriptor, TrunkNvmeError,
    TrunkNvmeReader,
};

#[derive(Debug)]
pub enum Qwen4ExpActivationError {
    Package(Qwen4ExpNativeAssetError),
    Header(GgufHeaderError),
    Graph(Qwen4ExpGraphError),
    Ple(PleNvmeError),
    Trunk(TrunkNvmeError),
    ContractChanged,
}

impl core::fmt::Display for Qwen4ExpActivationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Package(error) => error.fmt(f),
            Self::Header(error) => error.fmt(f),
            Self::Graph(error) => error.fmt(f),
            Self::Ple(error) => write!(f, "PLE reader error: {error:?}"),
            Self::Trunk(error) => error.fmt(f),
            Self::ContractChanged => {
                write!(f, "Qwen4Exp GGUF PLE contract differs from the HMC package")
            }
        }
    }
}

impl std::error::Error for Qwen4ExpActivationError {}

impl From<Qwen4ExpNativeAssetError> for Qwen4ExpActivationError {
    fn from(value: Qwen4ExpNativeAssetError) -> Self {
        Self::Package(value)
    }
}

impl From<GgufHeaderError> for Qwen4ExpActivationError {
    fn from(value: GgufHeaderError) -> Self {
        Self::Header(value)
    }
}

impl From<Qwen4ExpGraphError> for Qwen4ExpActivationError {
    fn from(value: Qwen4ExpGraphError) -> Self {
        Self::Graph(value)
    }
}

impl From<PleNvmeError> for Qwen4ExpActivationError {
    fn from(value: PleNvmeError) -> Self {
        Self::Ple(value)
    }
}

impl From<TrunkNvmeError> for Qwen4ExpActivationError {
    fn from(value: TrunkNvmeError) -> Self {
        Self::Trunk(value)
    }
}

/// A live package binding.  The file cursors retain no model data in memory;
/// decoder state and all compute scratch remain caller owned.
pub struct Qwen4ExpNativeRuntime {
    pub descriptor: Qwen4ExpNativeDescriptor,
    pub index: GgufTensorIndex,
    pub ple: PleNvmeReader,
    pub trunk: TrunkNvmeReader,
}

impl Qwen4ExpNativeRuntime {
    /// Fail-closed cold activation of an HMC package.  The only GGUF read is
    /// its metadata prefix (bounded by `MAX_GGUF_HEADER_BYTES`).
    pub fn activate(package: &Path) -> Result<Self, Qwen4ExpActivationError> {
        let descriptor = load_qwen4exp_native_hmc(package)?;
        let source = descriptor.source_beside(package)?;
        descriptor.verify_source_metadata(&source)?;
        let index = GgufTensorIndex::from_gguf_header_file(&source)?;
        let mut contracts = [Default::default(); 64];
        let graph = validate_qwen4exp_graph(&index, &mut contracts)?;
        if graph.layers != descriptor.n_layer
            || index.ple_config != Some(descriptor.ple_config)
            || index.ple_ngram_embedding_info().map(|info| info.dims[0])
                != Some(descriptor.ple.row_width)
            || index.ple_ngram_embedding_info().map(|info| info.dims[1])
                != Some(descriptor.ple.rows)
        {
            return Err(Qwen4ExpActivationError::ContractChanged);
        }
        let ple = PleNvmeReader::open_from_native_descriptor(&descriptor)?;
        let trunk = TrunkNvmeReader::open(&source, index.tensor_data_start)?;
        Ok(Self {
            descriptor,
            index,
            ple,
            trunk,
        })
    }
}
