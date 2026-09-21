//! Activation for a Qwen4Exp package with an NVMe-resident PLE payload.
//!
//! Activation reads a bounded GGUF metadata header from the external trunk on
//! E:, opens the C: PLE cursor for row gathers, and opens the trunk either as
//! an ordinary file cursor (`Streamed`, default) or as a read-only mmap
//! (`Mapped`) so the OS page cache keeps trunk weights in RAM.  It never
//! creates a swap-backed decoded copy of the model either way.

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

/// How the trunk weights are reached during decode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Qwen4ExpTrunkResidency {
    /// Seek+read each row span from disk; steady-state memory stays bounded.
    #[default]
    Streamed,
    /// Read-only mmap of the source GGUF; the OS page cache holds the trunk
    /// in RAM and row access is a pointer slice.  This deliberately trades
    /// the bounded-memory guarantee for speed on machines whose RAM fits
    /// the trunk (the intended deployment: trunk in RAM, PLE on NVMe).
    Mapped,
}

/// A live package binding.  In `Streamed` residency the file cursors retain
/// no model data in memory; in `Mapped` residency the trunk is a read-only
/// page-cache view.  Decoder state and all compute scratch remain caller
/// owned either way.
pub struct Qwen4ExpNativeRuntime {
    pub descriptor: Qwen4ExpNativeDescriptor,
    pub index: GgufTensorIndex,
    pub ple: PleNvmeReader,
    pub trunk: TrunkNvmeReader,
}

impl Qwen4ExpNativeRuntime {
    /// Fail-closed cold activation of an HMC package with streamed trunk
    /// reads.  The only GGUF read is its metadata prefix (bounded by
    /// `MAX_GGUF_HEADER_BYTES`).
    pub fn activate(package: &Path) -> Result<Self, Qwen4ExpActivationError> {
        Self::activate_with_residency(package, Qwen4ExpTrunkResidency::Streamed)
    }

    /// Activation with an explicit trunk residency mode.
    pub fn activate_with_residency(
        package: &Path,
        residency: Qwen4ExpTrunkResidency,
    ) -> Result<Self, Qwen4ExpActivationError> {
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
        let trunk = match residency {
            Qwen4ExpTrunkResidency::Streamed => {
                TrunkNvmeReader::open(&source, index.tensor_data_start)?
            }
            Qwen4ExpTrunkResidency::Mapped => {
                TrunkNvmeReader::open_mapped(&source, index.tensor_data_start)?
            }
        };
        Ok(Self {
            descriptor,
            index,
            ple,
            trunk,
        })
    }
}
