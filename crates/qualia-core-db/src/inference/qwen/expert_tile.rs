//! Bounded C:-resident HMC tiles for router-selected Qwen4Exp experts.
//!
//! A tile contains exactly the gate, up, and down planes for one `(layer,
//! expert)` pair.  Promotion is cold-path, explicit, size-admitted, and never
//! copies a full GGUF or whole layer to the NVMe cache.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use serde::{Deserialize, Serialize};

use crate::bundle::{BundleReader, BundleWriter};
use crate::gguf_sharder::GgufTensorInfo;

pub const QWEN_EXPERT_TILE_FORMAT: &str = "qualia.qwen4exp.expert-tile.v1";
pub const QWEN_EXPERT_TILE_SUFFIX: &str = ".qwen-expert.hmc";
const COPY_CHUNK_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QwenExpertPlane {
    pub dims: [u64; 4],
    pub n_dims: u32,
    pub ggml_type: u32,
    pub source_byte_offset: u64,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QwenExpertTileDescriptor {
    pub format: String,
    pub layer: u16,
    pub expert: u16,
    pub source_file_name: String,
    pub source_byte_len: u64,
    pub gate: QwenExpertPlane,
    pub up: QwenExpertPlane,
    pub down: QwenExpertPlane,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QwenExpertTileAdmission {
    pub cache_used_bytes: u64,
    pub requested_bytes: u64,
    pub cache_limit_bytes: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QwenExpertTileError {
    InvalidTensor,
    ExpertOutOfRange,
    OffsetOverflow,
    SourceIo,
    OutputExists,
    OutputPath,
    CacheBudgetExceeded,
    Bundle,
    Serialize,
    Write,
    InvalidPackage,
    BufferTooSmall,
}

impl core::fmt::Display for QwenExpertTileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTensor => write!(
                f,
                "expert tensor is not a supported [input, output, expert] GGML plane"
            ),
            Self::ExpertOutOfRange => {
                write!(f, "expert id is outside the tensor's expert dimension")
            }
            Self::OffsetOverflow => write!(f, "expert tile byte range overflowed"),
            Self::SourceIo => write!(f, "could not read source GGUF expert plane"),
            Self::OutputExists => write!(f, "refusing to overwrite an existing Qwen expert tile"),
            Self::OutputPath => write!(f, "Qwen expert tile output path is invalid"),
            Self::CacheBudgetExceeded => write!(
                f,
                "Qwen expert tile would exceed the configured cache budget"
            ),
            Self::Bundle => write!(f, "could not build Qwen expert HMC tile"),
            Self::Serialize => write!(f, "could not serialize Qwen expert tile descriptor"),
            Self::Write => write!(f, "could not write Qwen expert tile"),
            Self::InvalidPackage => write!(f, "Qwen expert tile HMC is invalid"),
            Self::BufferTooSmall => write!(f, "Qwen expert tile caller buffer is too small"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QwenExpertTilePlaneKind {
    Gate,
    Up,
    Down,
}

/// A validated C:-resident expert tile. Its mapping is bounded by one explicit
/// promotion artifact, never the original multi-gigabyte GGUF.
pub struct QwenExpertTileReader {
    mapped: Mmap,
    pub descriptor: QwenExpertTileDescriptor,
    gate_offset: usize,
    up_offset: usize,
    down_offset: usize,
}

impl std::error::Error for QwenExpertTileError {}

fn plane_for_expert(
    tensor: &GgufTensorInfo,
    expert: u16,
) -> Result<QwenExpertPlane, QwenExpertTileError> {
    if tensor.n_dims != 3 || tensor.dims[0] == 0 || tensor.dims[1] == 0 || tensor.dims[2] == 0 {
        return Err(QwenExpertTileError::InvalidTensor);
    }
    if expert as u64 >= tensor.dims[2] {
        return Err(QwenExpertTileError::ExpertOutOfRange);
    }
    let row_bytes = crate::ggml_quants::ggml_row_bytes(tensor.ggml_type, tensor.dims[0] as usize)
        .ok_or(QwenExpertTileError::InvalidTensor)? as u64;
    let bytes = row_bytes
        .checked_mul(tensor.dims[1])
        .ok_or(QwenExpertTileError::OffsetOverflow)?;
    let plane_offset = bytes
        .checked_mul(expert as u64)
        .ok_or(QwenExpertTileError::OffsetOverflow)?;
    let source_byte_offset = tensor
        .byte_offset
        .checked_add(plane_offset)
        .ok_or(QwenExpertTileError::OffsetOverflow)?;
    Ok(QwenExpertPlane {
        dims: [tensor.dims[0], tensor.dims[1], 1, 1],
        n_dims: 2,
        ggml_type: tensor.ggml_type,
        source_byte_offset,
        bytes,
    })
}

fn read_plane(
    source: &mut File,
    tensor_data_start: u64,
    plane: QwenExpertPlane,
) -> Result<Vec<u8>, QwenExpertTileError> {
    let start = tensor_data_start
        .checked_add(plane.source_byte_offset)
        .ok_or(QwenExpertTileError::OffsetOverflow)?;
    let length = usize::try_from(plane.bytes).map_err(|_| QwenExpertTileError::OffsetOverflow)?;
    let mut bytes = vec![0u8; length];
    source
        .seek(SeekFrom::Start(start))
        .map_err(|_| QwenExpertTileError::SourceIo)?;
    // Keep the read bounded even when a future checkpoint uses larger planes.
    let mut copied = 0usize;
    while copied < bytes.len() {
        let end = (copied + COPY_CHUNK_BYTES).min(bytes.len());
        source
            .read_exact(&mut bytes[copied..end])
            .map_err(|_| QwenExpertTileError::SourceIo)?;
        copied = end;
    }
    Ok(bytes)
}

/// Compute the requested tile's budget admission without creating any files.
/// Existing cache files are only counted when their name carries the dedicated
/// tile suffix; unrelated C: artifacts are never scanned or deleted.
pub fn admit_expert_tile(
    cache_dir: &Path,
    requested_bytes: u64,
    cache_limit_bytes: u64,
) -> Result<QwenExpertTileAdmission, QwenExpertTileError> {
    let mut used = 0u64;
    if cache_dir.exists() {
        for entry in std::fs::read_dir(cache_dir).map_err(|_| QwenExpertTileError::OutputPath)? {
            let entry = entry.map_err(|_| QwenExpertTileError::OutputPath)?;
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.ends_with(QWEN_EXPERT_TILE_SUFFIX))
            {
                used = used.saturating_add(
                    entry
                        .metadata()
                        .map_err(|_| QwenExpertTileError::OutputPath)?
                        .len(),
                );
            }
        }
    }
    let admission = QwenExpertTileAdmission {
        cache_used_bytes: used,
        requested_bytes,
        cache_limit_bytes,
    };
    if requested_bytes > cache_limit_bytes
        || used
            .checked_add(requested_bytes)
            .ok_or(QwenExpertTileError::OffsetOverflow)?
            > cache_limit_bytes
    {
        return Err(QwenExpertTileError::CacheBudgetExceeded);
    }
    Ok(admission)
}

/// Extract one selected expert's quantized source planes into an HMC tile.
/// The output path must be new.  The caller supplies the C: cache directory
/// and budget; this function never evicts or creates an unbounded cache.
#[allow(clippy::too_many_arguments)]
pub fn promote_expert_tile(
    source_path: &Path,
    source_byte_len: u64,
    tensor_data_start: u64,
    layer: u16,
    expert: u16,
    gate: &GgufTensorInfo,
    up: &GgufTensorInfo,
    down: &GgufTensorInfo,
    cache_dir: &Path,
    cache_limit_bytes: u64,
) -> Result<(PathBuf, QwenExpertTileAdmission), QwenExpertTileError> {
    let gate_plane = plane_for_expert(gate, expert)?;
    let up_plane = plane_for_expert(up, expert)?;
    let down_plane = plane_for_expert(down, expert)?;
    let requested = gate_plane
        .bytes
        .checked_add(up_plane.bytes)
        .and_then(|value| value.checked_add(down_plane.bytes))
        .ok_or(QwenExpertTileError::OffsetOverflow)?;
    let admission = admit_expert_tile(cache_dir, requested, cache_limit_bytes)?;
    std::fs::create_dir_all(cache_dir).map_err(|_| QwenExpertTileError::OutputPath)?;
    let output = cache_dir.join(format!(
        "layer-{layer:02}-expert-{expert:03}{QWEN_EXPERT_TILE_SUFFIX}"
    ));
    if output.exists() {
        return Err(QwenExpertTileError::OutputExists);
    }
    let source_name = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(QwenExpertTileError::OutputPath)?;
    let descriptor = QwenExpertTileDescriptor {
        format: QWEN_EXPERT_TILE_FORMAT.to_string(),
        layer,
        expert,
        source_file_name: source_name.to_string(),
        source_byte_len,
        gate: gate_plane,
        up: up_plane,
        down: down_plane,
    };
    let mut descriptor_bytes = Vec::new();
    ciborium::into_writer(&descriptor, &mut descriptor_bytes)
        .map_err(|_| QwenExpertTileError::Serialize)?;
    let mut source = File::open(source_path).map_err(|_| QwenExpertTileError::SourceIo)?;
    let gate_bytes = read_plane(&mut source, tensor_data_start, gate_plane)?;
    let up_bytes = read_plane(&mut source, tensor_data_start, up_plane)?;
    let down_bytes = read_plane(&mut source, tensor_data_start, down_plane)?;
    let mut bundle = BundleWriter::new();
    bundle
        .add_file("descriptor", "cbor", descriptor_bytes, None)
        .map_err(|_| QwenExpertTileError::Bundle)?;
    bundle
        .add_file("gate", "ggml-plane", gate_bytes, None)
        .map_err(|_| QwenExpertTileError::Bundle)?;
    bundle
        .add_file("up", "ggml-plane", up_bytes, None)
        .map_err(|_| QwenExpertTileError::Bundle)?;
    bundle
        .add_file("down", "ggml-plane", down_bytes, None)
        .map_err(|_| QwenExpertTileError::Bundle)?;
    let bytes = bundle.build().map_err(|_| QwenExpertTileError::Bundle)?;
    let mut temporary =
        tempfile::NamedTempFile::new_in(cache_dir).map_err(|_| QwenExpertTileError::OutputPath)?;
    if temporary.write_all(&bytes).is_err() || temporary.flush().is_err() {
        return Err(QwenExpertTileError::Write);
    }
    temporary
        .persist_noclobber(&output)
        .map_err(|_| QwenExpertTileError::OutputExists)?;
    Ok((output, admission))
}

/// Read and validate a compact expert-tile descriptor.  Tensor payloads remain
/// in the HMC until a caller explicitly opens that small tile for execution.
pub fn load_expert_tile_descriptor(
    path: &Path,
) -> Result<QwenExpertTileDescriptor, QwenExpertTileError> {
    let bytes = std::fs::read(path).map_err(|_| QwenExpertTileError::InvalidPackage)?;
    let bundle = BundleReader::parse(&bytes).map_err(|_| QwenExpertTileError::InvalidPackage)?;
    let descriptor_bytes = bundle
        .get("descriptor")
        .ok_or(QwenExpertTileError::InvalidPackage)?;
    let descriptor: QwenExpertTileDescriptor =
        ciborium::from_reader(descriptor_bytes).map_err(|_| QwenExpertTileError::InvalidPackage)?;
    if descriptor.format != QWEN_EXPERT_TILE_FORMAT
        || descriptor.gate.n_dims != 2
        || descriptor.up.n_dims != 2
        || descriptor.down.n_dims != 2
        || descriptor.gate.bytes == 0
        || descriptor.up.bytes == 0
        || descriptor.down.bytes == 0
    {
        return Err(QwenExpertTileError::InvalidPackage);
    }
    Ok(descriptor)
}

impl QwenExpertTileReader {
    pub fn open(path: &Path) -> Result<Self, QwenExpertTileError> {
        let file = File::open(path).map_err(|_| QwenExpertTileError::InvalidPackage)?;
        let mapped = unsafe { memmap2::MmapOptions::new().map(&file) }
            .map_err(|_| QwenExpertTileError::InvalidPackage)?;
        let bundle =
            BundleReader::parse(&mapped).map_err(|_| QwenExpertTileError::InvalidPackage)?;
        let descriptor_bytes = bundle
            .get("descriptor")
            .ok_or(QwenExpertTileError::InvalidPackage)?;
        let descriptor: QwenExpertTileDescriptor = ciborium::from_reader(descriptor_bytes)
            .map_err(|_| QwenExpertTileError::InvalidPackage)?;
        if descriptor.format != QWEN_EXPERT_TILE_FORMAT {
            return Err(QwenExpertTileError::InvalidPackage);
        }
        let entry_offset = |key: &str, expected: u64| {
            let entry = bundle
                .entry(key)
                .ok_or(QwenExpertTileError::InvalidPackage)?;
            if entry.length != expected {
                return Err(QwenExpertTileError::InvalidPackage);
            }
            usize::try_from(entry.offset).map_err(|_| QwenExpertTileError::InvalidPackage)
        };
        let gate_offset = entry_offset("gate", descriptor.gate.bytes)?;
        let up_offset = entry_offset("up", descriptor.up.bytes)?;
        let down_offset = entry_offset("down", descriptor.down.bytes)?;
        drop(bundle);
        Ok(Self {
            mapped,
            descriptor,
            gate_offset,
            up_offset,
            down_offset,
        })
    }

    fn plane(&self, kind: QwenExpertTilePlaneKind) -> (QwenExpertPlane, usize) {
        match kind {
            QwenExpertTilePlaneKind::Gate => (self.descriptor.gate, self.gate_offset),
            QwenExpertTilePlaneKind::Up => (self.descriptor.up, self.up_offset),
            QwenExpertTilePlaneKind::Down => (self.descriptor.down, self.down_offset),
        }
    }

    /// Compute a cached expert plane directly from its C: HMC segment. Caller
    /// storage bounds every dequantized row and output vector.
    pub fn gemv_into(
        &self,
        kind: QwenExpertTilePlaneKind,
        input: &[f32],
        out: &mut [f32],
        row_scratch: &mut [f32],
    ) -> Result<usize, QwenExpertTileError> {
        let (plane, offset) = self.plane(kind);
        let width = plane.dims[0] as usize;
        let rows = plane.dims[1] as usize;
        let row_bytes = crate::ggml_quants::ggml_row_bytes(plane.ggml_type, width)
            .ok_or(QwenExpertTileError::InvalidPackage)?;
        if input.len() < width || out.len() < rows || row_scratch.len() < width {
            return Err(QwenExpertTileError::BufferTooSmall);
        }
        for row in 0..rows {
            let start = offset
                .checked_add(
                    row.checked_mul(row_bytes)
                        .ok_or(QwenExpertTileError::OffsetOverflow)?,
                )
                .ok_or(QwenExpertTileError::OffsetOverflow)?;
            let end = start
                .checked_add(row_bytes)
                .ok_or(QwenExpertTileError::OffsetOverflow)?;
            if end > self.mapped.len() {
                return Err(QwenExpertTileError::InvalidPackage);
            }
            crate::ggml_quants::dequantize_row_into(
                &self.mapped[start..end],
                plane.ggml_type,
                width,
                &mut row_scratch[..width],
            )
            .map_err(|_| QwenExpertTileError::InvalidPackage)?;
            let mut sum = 0.0f32;
            for index in 0..width {
                sum += row_scratch[index] * input[index];
            }
            out[row] = sum;
        }
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expert_plane_is_layer_local_and_byte_exact() {
        let tensor = GgufTensorInfo {
            dims: [32, 4, 3, 0],
            n_dims: 3,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 128,
        };
        let second = plane_for_expert(&tensor, 1).unwrap();
        assert_eq!(second.bytes, 512);
        assert_eq!(second.source_byte_offset, 640);
        assert_eq!(second.dims, [32, 4, 1, 1]);
    }

    #[test]
    fn cache_admission_never_overcommits() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(
            admit_expert_tile(directory.path(), 9, 8).unwrap_err(),
            QwenExpertTileError::CacheBudgetExceeded
        );
    }

    #[test]
    fn promoted_tile_replays_a_quantized_plane_from_its_hmc_segment() {
        let source_dir = tempfile::tempdir().unwrap();
        let source_path = source_dir.path().join("source.gguf");
        let mut bytes = Vec::new();
        for value in [
            1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
        ] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        std::fs::write(&source_path, &bytes).unwrap();
        let plane = |offset| GgufTensorInfo {
            dims: [2, 2, 1, 0],
            n_dims: 3,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: offset,
        };
        let cache_dir = source_dir.path().join("cache");
        let (path, _) = promote_expert_tile(
            &source_path,
            bytes.len() as u64,
            0,
            0,
            0,
            &plane(0),
            &plane(16),
            &plane(32),
            &cache_dir,
            1024,
        )
        .unwrap();
        let reader = QwenExpertTileReader::open(&path).unwrap();
        let mut out = [0.0f32; 2];
        let mut row = [0.0f32; 2];
        assert_eq!(
            reader
                .gemv_into(
                    QwenExpertTilePlaneKind::Gate,
                    &[1.0, 1.0],
                    &mut out,
                    &mut row
                )
                .unwrap(),
            2
        );
        assert_eq!(out, [3.0, 7.0]);
    }
}
