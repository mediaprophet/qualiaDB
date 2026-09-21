//! Direct, bounded NVMe reads for Qwen4Exp PLE n-gram rows.
//!
//! Qwen3.8 Flash Next's PLE is a hash-gathered embedding table, not a
//! matrix-multiplication weight.  Mapping or uploading the entire table makes
//! the model impossible to host on the A2000 tier.  This reader retains only a
//! file handle and reads/dequantizes the selected rows into caller storage.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use super::native_asset::Qwen4ExpNativeDescriptor;
use crate::gguf_sharder::{GgufTensorIndex, GgufTensorInfo};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PleNvmeError {
    MissingTensor,
    NotRowAddressable,
    RowOutOfRange,
    RawScratchTooSmall,
    OutputTooSmall,
    OffsetOverflow,
    Io,
    Dequantization,
    DescriptorMismatch,
}

/// Deterministic accounting for one selected-row PLE gather.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PleGatherReceipt {
    pub rows: u32,
    pub storage_bytes: u64,
    pub expanded_f32_bytes: u64,
}

/// Cumulative physical I/O performed by a PLE reader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PleIoStats {
    pub rows: u64,
    pub reads: u64,
    pub bytes: u64,
}

/// Read-only direct-row source for one embedded PLE tensor.
///
/// It is intentionally not `Clone`: each active engine owns a single file
/// cursor, and callers provide all scratch needed by a gather.  The OS may
/// cache individual pages, but the runtime never maps, pins, or copies the
/// whole table into RAM or VRAM.
pub struct PleNvmeReader {
    file: File,
    tensor_data_start: u64,
    info: GgufTensorInfo,
    row_bytes: usize,
    rows: u64,
    row_width: usize,
    stats: PleIoStats,
}

impl PleNvmeReader {
    /// Open the extracted PLE payload from the `.hmc` contract.  This file is
    /// deliberately a raw C:-resident table: its byte zero is the PLE tensor's
    /// first byte, so decode never seeks through or maps the E:-resident GGUF.
    pub fn open_from_native_descriptor(
        descriptor: &Qwen4ExpNativeDescriptor,
    ) -> Result<Self, PleNvmeError> {
        let path = std::path::Path::new(&descriptor.ple_storage.file_path);
        let length = std::fs::metadata(path)
            .map_err(|_| PleNvmeError::DescriptorMismatch)?
            .len();
        if length != descriptor.ple_storage.byte_len || length != descriptor.ple.storage_bytes {
            return Err(PleNvmeError::DescriptorMismatch);
        }
        let info = GgufTensorInfo {
            dims: [descriptor.ple.row_width, descriptor.ple.rows, 1, 1],
            n_dims: 2,
            ggml_type: descriptor.ple.ggml_type,
            byte_offset: 0,
        };
        let reader = Self::open(path, 0, info)?;
        if reader.row_bytes as u64 != descriptor.ple.row_bytes {
            return Err(PleNvmeError::DescriptorMismatch);
        }
        Ok(reader)
    }

    /// Open the PLE source selected by a parsed Qwen4Exp GGUF index.
    pub fn open_from_index(
        path: &std::path::Path,
        index: &GgufTensorIndex,
    ) -> Result<Self, PleNvmeError> {
        let info = *index
            .ple_ngram_embedding_info()
            .ok_or(PleNvmeError::MissingTensor)?;
        Self::open(path, index.tensor_data_start, info)
    }

    pub fn open(
        path: &std::path::Path,
        tensor_data_start: u64,
        info: GgufTensorInfo,
    ) -> Result<Self, PleNvmeError> {
        if info.n_dims != 2 || info.dims[0] == 0 || info.dims[1] == 0 {
            return Err(PleNvmeError::NotRowAddressable);
        }
        let row_width = info.dims[0] as usize;
        let row_bytes = crate::ggml_quants::ggml_row_bytes(info.ggml_type, row_width)
            .ok_or(PleNvmeError::NotRowAddressable)?;
        let file = File::open(path).map_err(|_| PleNvmeError::Io)?;
        Ok(Self {
            file,
            tensor_data_start,
            info,
            row_bytes,
            rows: info.dims[1],
            row_width,
            stats: PleIoStats::default(),
        })
    }

    /// Cumulative physical I/O performed by this reader.
    pub fn io_stats(&self) -> PleIoStats {
        self.stats
    }

    pub fn row_width(&self) -> usize {
        self.row_width
    }

    pub fn row_count(&self) -> u64 {
        self.rows
    }

    pub fn raw_row_bytes(&self) -> usize {
        self.row_bytes
    }

    /// Return the exact file byte span for one PLE row without reading it.
    pub fn row_range(&self, row: u64) -> Result<(u64, usize), PleNvmeError> {
        if row >= self.rows {
            return Err(PleNvmeError::RowOutOfRange);
        }
        let relative = self
            .info
            .byte_offset
            .checked_add(
                row.checked_mul(self.row_bytes as u64)
                    .ok_or(PleNvmeError::OffsetOverflow)?,
            )
            .ok_or(PleNvmeError::OffsetOverflow)?;
        let absolute = self
            .tensor_data_start
            .checked_add(relative)
            .ok_or(PleNvmeError::OffsetOverflow)?;
        Ok((absolute, self.row_bytes))
    }

    /// Read and dequantize exactly one selected n-gram row.
    ///
    /// `raw_scratch` is normally a small stack/arena buffer (the qwen4exp PLE
    /// row width is 160).  This avoids both whole-table mapping and per-row
    /// allocation.  Multi-row gathers call this once per elected hash row.
    pub fn read_row_into(
        &mut self,
        row: u64,
        raw_scratch: &mut [u8],
        out: &mut [f32],
    ) -> Result<usize, PleNvmeError> {
        if raw_scratch.len() < self.row_bytes {
            return Err(PleNvmeError::RawScratchTooSmall);
        }
        if out.len() < self.row_width {
            return Err(PleNvmeError::OutputTooSmall);
        }
        let (offset, bytes) = self.row_range(row)?;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|_| PleNvmeError::Io)?;
        self.file
            .read_exact(&mut raw_scratch[..bytes])
            .map_err(|_| PleNvmeError::Io)?;
        self.stats.reads += 1;
        self.stats.rows += 1;
        self.stats.bytes += bytes as u64;
        crate::ggml_quants::dequantize_row_into(
            &raw_scratch[..bytes],
            self.info.ggml_type,
            self.row_width,
            out,
        )
        .map_err(|_| PleNvmeError::Dequantization)
    }

    /// Gather selected PLE rows into a tightly packed caller buffer.
    ///
    /// `row_ids` must already be the deterministic, deduplicated selection for
    /// the micro-batch.  Keeping selection separate from I/O lets the model's
    /// exact n-gram hash routine decide IDs without allocating in this layer.
    pub fn gather_rows_into(
        &mut self,
        row_ids: &[u64],
        raw_scratch: &mut [u8],
        out: &mut [f32],
    ) -> Result<PleGatherReceipt, PleNvmeError> {
        let output_len = row_ids
            .len()
            .checked_mul(self.row_width)
            .ok_or(PleNvmeError::OutputTooSmall)?;
        if out.len() < output_len {
            return Err(PleNvmeError::OutputTooSmall);
        }
        for (index, &row) in row_ids.iter().enumerate() {
            let offset = index * self.row_width;
            self.read_row_into(row, raw_scratch, &mut out[offset..offset + self.row_width])?;
        }
        Ok(PleGatherReceipt {
            rows: row_ids.len().min(u32::MAX as usize) as u32,
            storage_bytes: (row_ids.len() as u64).saturating_mul(self.row_bytes as u64),
            expanded_f32_bytes: (output_len as u64)
                .saturating_mul(core::mem::size_of::<f32>() as u64),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_ranges_are_exact_and_bounds_checked() {
        let info = GgufTensorInfo {
            dims: [32, 3, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 128,
        };
        let row_bytes = crate::ggml_quants::ggml_row_bytes(info.ggml_type, 32).unwrap();
        let relative = info.byte_offset + 2 * row_bytes as u64;
        assert_eq!(relative, 384);
        assert_eq!(info.dims[1], 3);
    }

    #[test]
    fn direct_file_row_matches_mapped_reference_row() {
        let info = GgufTensorInfo {
            dims: [2, 2, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 128,
        };
        let mut hp = crate::gguf_sharder::GgufHyperparams::default();
        hp.architecture = crate::gguf_sharder::ARCH_QWEN4EXP;
        let index =
            GgufTensorIndex::from_components(&[(b"per_layer_token_embd.weight", info)], hp, 0);
        let mut source = vec![0u8; 128];
        for value in [1.0f32, 2.0, 3.0, 4.0] {
            source.extend_from_slice(&value.to_le_bytes());
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ple.gguf");
        std::fs::write(&path, &source).unwrap();
        let mut mapped = [0.0f32; 2];
        assert_eq!(index.dequantize_ple_row_into(&source, 1, &mut mapped), 2);
        let mut reader = PleNvmeReader::open_from_index(&path, &index).unwrap();
        let mut raw = [0u8; 8];
        let mut direct = [0.0f32; 2];
        assert_eq!(reader.read_row_into(1, &mut raw, &mut direct).unwrap(), 2);
        assert_eq!(direct, mapped);
    }
}
