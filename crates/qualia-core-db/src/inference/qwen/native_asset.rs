//! Native Qwen4Exp asset preparation.
//!
//! The v4 P64 format intentionally uses 32-bit relative offsets, so a single
//! P64 can never represent the 90+ GiB Qwen3.8 Flash Next checkpoint.  The
//! correct native artefact today is a compact `.hmc` contract which attests to
//! the external GGUF and describes the PLE table's direct NVMe byte span.
//! Future sub-4-GiB P64 trunk shards, `.q42` policy volumes, and `.10d`
//! schedules are added as intact HMC entries without ever copying the PLE.

use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::graph_contract::{validate_qwen4exp_graph, Qwen4ExpGraphReport, Qwen4ExpLayerContract};
use crate::bundle::{BundleReader, BundleWriter};
use crate::ggml_quants::tensor_byte_len;
use crate::gguf_sharder::{GgufTensorIndex, GgufTensorInfo, Qwen4ExpPleConfig, ARCH_QWEN4EXP};

pub const QWEN4EXP_NATIVE_FORMAT: &str = "qualia.qwen4exp.external-ple.v1";
const HMC_DESCRIPTOR_KEY: &str = "qwen4exp/placement.cbor";
const HMC_REQUIREMENTS_KEY: &str = "qwen4exp/runtime-requirements.txt";

/// A verified external source.  The HMC intentionally stores the identity and
/// relative file name, not a host-specific absolute path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalGgufAsset {
    pub file_name: String,
    /// Explicit local source for this activation contract. The HMC can live on
    /// the fast artifact volume while the immutable GGUF remains elsewhere.
    /// Empty preserves the legacy "beside the HMC" convention.
    #[serde(default)]
    pub source_path: String,
    pub byte_len: u64,
    /// Empty means metadata-attested preparation; a full 32-byte SHA-256 is
    /// deliberately an explicit audit operation, not an activation delay.
    #[serde(default)]
    pub sha256: Vec<u8>,
}

/// Directly addressable PLE tensor span inside the external GGUF payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Qwen4ExpPleSpan {
    pub tensor_data_start: u64,
    pub tensor_byte_offset: u64,
    pub absolute_offset: u64,
    pub storage_bytes: u64,
    pub ggml_type: u32,
    pub row_width: u64,
    pub rows: u64,
    pub row_bytes: u64,
}

/// The extracted PLE payload.  It is the only large native artifact: a raw,
/// row-addressable 26.8 GiB table on the chosen NVMe volume, never a second
/// GGUF or a RAM/VRAM-resident buffer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Qwen4ExpPleStorage {
    pub file_path: String,
    pub byte_len: u64,
}

/// Cold activation data placed in the `.hmc` descriptor entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Qwen4ExpNativeDescriptor {
    pub format: String,
    pub architecture: String,
    pub source: ExternalGgufAsset,
    pub ple: Qwen4ExpPleSpan,
    pub ple_storage: Qwen4ExpPleStorage,
    /// Exact PLE n-gram hash and per-head row layout from the source GGUF.
    #[serde(default)]
    pub ple_config: Qwen4ExpPleConfig,
    /// All model bytes other than the PLE.  They remain GGUF until the graph
    /// executor has emitted independently valid, sub-4-GiB P64 trunk shards.
    pub trunk_gguf_bytes: u64,
    pub n_layer: u32,
    pub n_embd: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub full_attention_interval: u32,
    pub gated_deltanet_layers: u32,
    pub qsa_layers: u32,
    pub ple_layers: u32,
    pub required_graph_features: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Qwen4ExpNativeAssetError {
    InputMissing,
    OutputExists,
    Open,
    Map,
    NotQwen4Exp,
    MissingPle,
    InvalidPle {
        dims: [u64; 4],
        n_dims: u32,
        ggml_type: u32,
    },
    SpanOverflow,
    SourceTruncated,
    PackageOpen,
    PackageInvalid,
    SourceNameMismatch,
    SourceLengthMismatch,
    SourceHashUnavailable,
    PleOutputExists,
    PleOutputLengthMismatch {
        expected: u64,
        actual: u64,
    },
    PleOutputPath,
    PleCopy,
    SourceHashMismatch,
    GraphContract,
    Serialize,
    Bundle,
    Write,
}

impl core::fmt::Display for Qwen4ExpNativeAssetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use Qwen4ExpNativeAssetError::*;
        match self {
            InputMissing => write!(f, "Qwen4Exp source GGUF was not found"),
            OutputExists => write!(f, "refusing to overwrite an existing native HMC"),
            Open => write!(f, "could not open Qwen4Exp source GGUF"),
            Map => write!(f, "could not map Qwen4Exp source GGUF"),
            NotQwen4Exp => write!(f, "GGUF is not a Qwen4Exp checkpoint with a PLE table"),
            MissingPle => write!(f, "Qwen4Exp GGUF is missing its PLE n-gram tensor"),
            InvalidPle {
                dims,
                n_dims,
                ggml_type,
            } => write!(
                f,
                "Qwen4Exp PLE tensor is not a supported row-addressable table (dims={dims:?}, n_dims={n_dims}, ggml_type={ggml_type})"
            ),
            SpanOverflow => write!(f, "Qwen4Exp PLE byte span overflowed"),
            SourceTruncated => write!(f, "Qwen4Exp PLE span extends beyond source GGUF"),
            PackageOpen => write!(f, "could not open Qwen4Exp native HMC package"),
            PackageInvalid => write!(f, "Qwen4Exp native HMC package is invalid"),
            SourceNameMismatch => write!(f, "Qwen4Exp source name does not match HMC contract"),
            SourceLengthMismatch => write!(f, "Qwen4Exp source length does not match HMC contract"),
            SourceHashUnavailable => write!(f, "Qwen4Exp HMC has no completed SHA-256 audit"),
            PleOutputExists => write!(f, "refusing to overwrite an existing extracted PLE file"),
            PleOutputLengthMismatch { expected, actual } => write!(
                f,
                "existing extracted PLE has {actual} bytes; source contract requires {expected}"
            ),
            PleOutputPath => write!(f, "could not create the extracted PLE output path"),
            PleCopy => write!(f, "could not extract the Qwen4Exp PLE payload"),
            SourceHashMismatch => write!(f, "Qwen4Exp source SHA-256 does not match HMC contract"),
            GraphContract => write!(
                f,
                "Qwen4Exp GGUF does not satisfy its required execution graph"
            ),
            Serialize => write!(f, "could not encode Qwen4Exp HMC descriptor"),
            Bundle => write!(f, "could not build Qwen4Exp HMC package"),
            Write => write!(f, "could not write Qwen4Exp HMC package"),
        }
    }
}

impl std::error::Error for Qwen4ExpNativeAssetError {}

impl Qwen4ExpNativeDescriptor {
    /// Fast activation identity check. Full source hashing is deliberately
    /// separate because it reads 90+ GiB; callers use `verify_source_sha256`
    /// for installation/audit and this check before ordinary activation.
    pub fn verify_source_metadata(&self, source: &Path) -> Result<(), Qwen4ExpNativeAssetError> {
        let name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or(Qwen4ExpNativeAssetError::SourceNameMismatch)?;
        if name != self.source.file_name {
            return Err(Qwen4ExpNativeAssetError::SourceNameMismatch);
        }
        let length = std::fs::metadata(source)
            .map_err(|_| Qwen4ExpNativeAssetError::InputMissing)?
            .len();
        if length != self.source.byte_len {
            return Err(Qwen4ExpNativeAssetError::SourceLengthMismatch);
        }
        Ok(())
    }

    /// Installation/audit validation: confirm the entire external GGUF's
    /// cryptographic identity before first use or after a file transfer.
    pub fn verify_source_sha256(&self, source: &Path) -> Result<(), Qwen4ExpNativeAssetError> {
        self.verify_source_metadata(source)?;
        if self.source.sha256.len() != 32 {
            return Err(Qwen4ExpNativeAssetError::SourceHashUnavailable);
        }
        if sha256_file(source)? != self.source.sha256 {
            return Err(Qwen4ExpNativeAssetError::SourceHashMismatch);
        }
        Ok(())
    }

    /// Resolve the explicitly attested external source, falling back to the
    /// legacy beside-HMC convention for portable, already-audited artifacts.
    pub fn source_beside(
        &self,
        package: &Path,
    ) -> Result<std::path::PathBuf, Qwen4ExpNativeAssetError> {
        if !self.source.source_path.is_empty() {
            return Ok(std::path::PathBuf::from(&self.source.source_path));
        }
        let parent = package
            .parent()
            .ok_or(Qwen4ExpNativeAssetError::PackageOpen)?;
        Ok(parent.join(&self.source.file_name))
    }
}

fn sha256_file(path: &Path) -> Result<Vec<u8>, Qwen4ExpNativeAssetError> {
    let file = File::open(path).map_err(|_| Qwen4ExpNativeAssetError::Open)?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut block = [0u8; 1024 * 1024];
    let mut hasher = Sha256::new();
    loop {
        let n = reader
            .read(&mut block)
            .map_err(|_| Qwen4ExpNativeAssetError::Open)?;
        if n == 0 {
            break;
        }
        hasher.update(&block[..n]);
    }
    Ok(hasher.finalize().to_vec())
}

fn ple_span(
    index: &GgufTensorIndex,
    source_bytes: u64,
) -> Result<Qwen4ExpPleSpan, Qwen4ExpNativeAssetError> {
    let info: GgufTensorInfo = *index
        .ple_ngram_embedding_info()
        .ok_or(Qwen4ExpNativeAssetError::MissingPle)?;
    if info.n_dims != 2 || info.dims[0] == 0 || info.dims[1] == 0 {
        return Err(Qwen4ExpNativeAssetError::InvalidPle {
            dims: info.dims,
            n_dims: info.n_dims,
            ggml_type: info.ggml_type,
        });
    }
    let storage_bytes = tensor_byte_len(&info).ok_or(Qwen4ExpNativeAssetError::InvalidPle {
        dims: info.dims,
        n_dims: info.n_dims,
        ggml_type: info.ggml_type,
    })? as u64;
    let row_width = info.dims[0];
    let rows = info.dims[1];
    let row_bytes = storage_bytes
        .checked_div(rows)
        .filter(|bytes| *bytes != 0)
        .ok_or(Qwen4ExpNativeAssetError::InvalidPle {
            dims: info.dims,
            n_dims: info.n_dims,
            ggml_type: info.ggml_type,
        })?;
    let absolute_offset = index
        .tensor_data_start
        .checked_add(info.byte_offset)
        .ok_or(Qwen4ExpNativeAssetError::SpanOverflow)?;
    let end = absolute_offset
        .checked_add(storage_bytes)
        .ok_or(Qwen4ExpNativeAssetError::SpanOverflow)?;
    if end > source_bytes {
        return Err(Qwen4ExpNativeAssetError::SourceTruncated);
    }
    Ok(Qwen4ExpPleSpan {
        tensor_data_start: index.tensor_data_start,
        tensor_byte_offset: info.byte_offset,
        absolute_offset,
        storage_bytes,
        ggml_type: info.ggml_type,
        row_width,
        rows,
        row_bytes,
    })
}

/// Extract exactly the PLE byte span from the source GGUF to a dedicated NVMe
/// file.  The caller selects the target; it must not already exist.  This is a
/// bounded streaming copy (1 MiB buffer), not a mapping or materialization of
/// the table in process memory.
fn extract_ple_to_nvme(
    source: &Path,
    ple: Qwen4ExpPleSpan,
    destination: &Path,
) -> Result<(), Qwen4ExpNativeAssetError> {
    if destination.exists() {
        return Err(Qwen4ExpNativeAssetError::PleOutputExists);
    }
    let parent = destination
        .parent()
        .ok_or(Qwen4ExpNativeAssetError::PleOutputPath)?;
    if !parent.is_dir() {
        return Err(Qwen4ExpNativeAssetError::PleOutputPath);
    }
    let mut input = File::open(source).map_err(|_| Qwen4ExpNativeAssetError::Open)?;
    input
        .seek(std::io::SeekFrom::Start(ple.absolute_offset))
        .map_err(|_| Qwen4ExpNativeAssetError::PleCopy)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| Qwen4ExpNativeAssetError::PleOutputExists)?;
    let mut remaining = ple.storage_bytes;
    let mut buffer = [0u8; 1024 * 1024];
    while remaining != 0 {
        let take = (remaining as usize).min(buffer.len());
        input
            .read_exact(&mut buffer[..take])
            .map_err(|_| Qwen4ExpNativeAssetError::PleCopy)?;
        if output.write_all(&buffer[..take]).is_err() {
            let _ = std::fs::remove_file(destination);
            return Err(Qwen4ExpNativeAssetError::PleCopy);
        }
        remaining -= take as u64;
    }
    if output.flush().is_err() {
        let _ = std::fs::remove_file(destination);
        return Err(Qwen4ExpNativeAssetError::PleCopy);
    }
    Ok(())
}

/// Create a compact, immutable `.hmc` activation package on an artifact volume
/// while its GGUF trunk remains in place. Only the PLE span is extracted to
/// `ple_output`; all other 60+ GiB stay in the source GGUF. An already-present
/// PLE payload of the exact declared length is adopted without copying it
/// again, allowing a compact HMC schema refresh without consuming disk space.
pub fn prepare_qwen4exp_native_hmc(
    source: &Path,
    output: &Path,
    ple_output: &Path,
) -> Result<Qwen4ExpNativeDescriptor, Qwen4ExpNativeAssetError> {
    if !source.is_file() {
        return Err(Qwen4ExpNativeAssetError::InputMissing);
    }
    if output.exists() {
        return Err(Qwen4ExpNativeAssetError::OutputExists);
    }
    let source_bytes = std::fs::metadata(source)
        .map_err(|_| Qwen4ExpNativeAssetError::Open)?
        .len();
    let file = File::open(source).map_err(|_| Qwen4ExpNativeAssetError::Open)?;
    // SAFETY: immutable source; the mapping is held only while metadata is parsed.
    let mmap = unsafe { memmap2::Mmap::map(&file).map_err(|_| Qwen4ExpNativeAssetError::Map)? };
    let index = GgufTensorIndex::from_gguf(&mmap);
    if index.hyperparams.architecture != ARCH_QWEN4EXP {
        return Err(Qwen4ExpNativeAssetError::NotQwen4Exp);
    }
    let ple = ple_span(&index, source_bytes)?;
    let ple_config = index
        .ple_config
        .filter(Qwen4ExpPleConfig::is_complete)
        .ok_or(Qwen4ExpNativeAssetError::MissingPle)?;
    let mut layer_contracts = [Qwen4ExpLayerContract::default(); 128];
    let graph: Qwen4ExpGraphReport = validate_qwen4exp_graph(&index, &mut layer_contracts)
        .map_err(|_| Qwen4ExpNativeAssetError::GraphContract)?;
    let descriptor = Qwen4ExpNativeDescriptor {
        format: QWEN4EXP_NATIVE_FORMAT.to_string(),
        architecture: index.hyperparams.architecture_name().to_string(),
        source: ExternalGgufAsset {
            file_name: source
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(Qwen4ExpNativeAssetError::InputMissing)?
                .to_string(),
            source_path: source
                .canonicalize()
                .map_err(|_| Qwen4ExpNativeAssetError::InputMissing)?
                .display()
                .to_string(),
            byte_len: source_bytes,
            sha256: Vec::new(),
        },
        ple,
        ple_storage: Qwen4ExpPleStorage {
            file_path: ple_output.display().to_string(),
            byte_len: ple.storage_bytes,
        },
        ple_config,
        trunk_gguf_bytes: source_bytes.saturating_sub(ple.storage_bytes),
        n_layer: index.hyperparams.n_layer,
        n_embd: index.hyperparams.n_embd,
        n_head: index.hyperparams.n_head,
        n_kv_head: index.hyperparams.effective_n_kv_head(),
        full_attention_interval: index.hyperparams.full_attention_interval,
        gated_deltanet_layers: graph.gated_deltanet_layers,
        qsa_layers: graph.qsa_layers,
        ple_layers: graph.ple_layers,
        required_graph_features: vec![
            "PLE direct NVMe row gather".to_string(),
            "Hyper-Connection".to_string(),
            "GatedDeltaNet recurrent state".to_string(),
            "Qwen Sparse Attention".to_string(),
            "top-10-of-512 MoE plus shared expert".to_string(),
        ],
    };
    let mut descriptor_cbor = Vec::new();
    ciborium::into_writer(&descriptor, &mut descriptor_cbor)
        .map_err(|_| Qwen4ExpNativeAssetError::Serialize)?;
    let requirements = b"Qwen4Exp native package. The PLE table is external by design: resolve the source_path in placement.cbor and read selected rows at the recorded spans. Preparation attests file name, length, architecture and graph; a full SHA-256 audit is optional and must not delay first activation. The runtime must not require whole-table residency, staging, or conversion. P64 v4 is limited to one <4 GiB payload; add trunk shards only after each shard has a coherent Qwen4Exp graph consumer.\n".to_vec();
    let mut bundle = BundleWriter::new();
    bundle
        .add_file(
            HMC_DESCRIPTOR_KEY,
            "qwen4exp-placement",
            descriptor_cbor,
            None,
        )
        .map_err(|_| Qwen4ExpNativeAssetError::Bundle)?;
    bundle
        .add_file(HMC_REQUIREMENTS_KEY, "text", requirements, None)
        .map_err(|_| Qwen4ExpNativeAssetError::Bundle)?;
    let package = bundle
        .build()
        .map_err(|_| Qwen4ExpNativeAssetError::Bundle)?;
    let created_ple = if ple_output.exists() {
        let length = std::fs::metadata(ple_output)
            .map_err(|_| Qwen4ExpNativeAssetError::PleOutputPath)?
            .len();
        if length != ple.storage_bytes {
            return Err(Qwen4ExpNativeAssetError::PleOutputLengthMismatch {
                expected: ple.storage_bytes,
                actual: length,
            });
        }
        false
    } else {
        extract_ple_to_nvme(source, ple, ple_output)?;
        true
    };
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|_| {
            if created_ple {
                let _ = std::fs::remove_file(ple_output);
            }
            Qwen4ExpNativeAssetError::Write
        })?;
    target
        .write_all(&package)
        .and_then(|_| target.flush())
        .map_err(|_| {
            if created_ple {
                let _ = std::fs::remove_file(ple_output);
            }
            let _ = std::fs::remove_file(output);
            Qwen4ExpNativeAssetError::Write
        })?;
    Ok(descriptor)
}

/// Read the compact descriptor from a transparent HMC without loading any
/// external model bytes. The caller resolves and validates the external GGUF
/// with [`Qwen4ExpNativeDescriptor::verify_source_metadata`] or
/// [`Qwen4ExpNativeDescriptor::verify_source_sha256`].
pub fn load_qwen4exp_native_hmc(
    package: &Path,
) -> Result<Qwen4ExpNativeDescriptor, Qwen4ExpNativeAssetError> {
    let bytes = std::fs::read(package).map_err(|_| Qwen4ExpNativeAssetError::PackageOpen)?;
    let reader =
        BundleReader::parse(&bytes).map_err(|_| Qwen4ExpNativeAssetError::PackageInvalid)?;
    let descriptor_bytes = reader
        .get(HMC_DESCRIPTOR_KEY)
        .ok_or(Qwen4ExpNativeAssetError::PackageInvalid)?;
    let descriptor: Qwen4ExpNativeDescriptor = ciborium::from_reader(descriptor_bytes)
        .map_err(|_| Qwen4ExpNativeAssetError::PackageInvalid)?;
    if descriptor.format != QWEN4EXP_NATIVE_FORMAT
        || descriptor.architecture != "qwen4exp"
        || descriptor.ple.storage_bytes == 0
        || descriptor.ple.row_width == 0
        || descriptor.ple.rows == 0
        || descriptor.ple.row_bytes == 0
        || descriptor.ple_storage.file_path.is_empty()
        || descriptor.ple_storage.byte_len != descriptor.ple.storage_bytes
        || !descriptor.ple_config.is_complete()
        || !(descriptor.source.sha256.is_empty() || descriptor.source.sha256.len() == 32)
        || descriptor.gated_deltanet_layers == 0
        || descriptor.qsa_layers == 0
        || descriptor.ple_layers == 0
    {
        return Err(Qwen4ExpNativeAssetError::PackageInvalid);
    }
    let span_end = descriptor
        .ple
        .absolute_offset
        .checked_add(descriptor.ple.storage_bytes)
        .ok_or(Qwen4ExpNativeAssetError::PackageInvalid)?;
    if span_end > descriptor.source.byte_len {
        return Err(Qwen4ExpNativeAssetError::PackageInvalid);
    }
    Ok(descriptor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ple_span_rejects_truncated_external_source() {
        let ple = GgufTensorInfo {
            dims: [32, 16, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 512,
        };
        let mut hp = crate::gguf_sharder::GgufHyperparams::default();
        hp.architecture = ARCH_QWEN4EXP;
        let index =
            GgufTensorIndex::from_components(&[(b"per_layer_token_embd.weight", ple)], hp, 128);
        assert_eq!(
            ple_span(&index, 1024).unwrap_err(),
            Qwen4ExpNativeAssetError::SourceTruncated
        );
    }

    #[test]
    fn descriptor_rejects_wrong_external_identity_before_opening_ple() {
        let descriptor = Qwen4ExpNativeDescriptor {
            format: QWEN4EXP_NATIVE_FORMAT.to_string(),
            architecture: "qwen4exp".to_string(),
            source: ExternalGgufAsset {
                file_name: "expected.gguf".to_string(),
                source_path: String::new(),
                byte_len: 1,
                sha256: vec![0; 32],
            },
            ple: Qwen4ExpPleSpan {
                tensor_data_start: 0,
                tensor_byte_offset: 0,
                absolute_offset: 0,
                storage_bytes: 1,
                ggml_type: 0,
                row_width: 1,
                rows: 1,
                row_bytes: 1,
            },
            ple_storage: Qwen4ExpPleStorage {
                file_path: "C:\\ple.bin".to_string(),
                byte_len: 1,
            },
            ple_config: Qwen4ExpPleConfig::default(),
            trunk_gguf_bytes: 0,
            n_layer: 48,
            n_embd: 2560,
            n_head: 24,
            n_kv_head: 2,
            full_attention_interval: 4,
            gated_deltanet_layers: 36,
            qsa_layers: 12,
            ple_layers: 1,
            required_graph_features: vec![],
        };
        let dir = tempfile::tempdir().unwrap();
        let wrong = dir.path().join("wrong.gguf");
        std::fs::write(&wrong, [0u8]).unwrap();
        assert_eq!(
            descriptor.verify_source_metadata(&wrong).unwrap_err(),
            Qwen4ExpNativeAssetError::SourceNameMismatch
        );
    }
}
