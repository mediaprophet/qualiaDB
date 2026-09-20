//! Bounded row reads from a Qwen4Exp GGUF trunk.
//!
//! PLE rows belong on the fast C: NVMe payload. The remaining immutable
//! weights stay in the source GGUF and are consumed as selected matrix rows;
//! this reader prevents a caller from mistaking a mmap address reservation for
//! resident model memory.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::gguf_sharder::GgufTensorInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrunkNvmeError {
    InvalidTensor,
    RowOutOfRange,
    RawScratchTooSmall,
    OutputTooSmall,
    OffsetOverflow,
    Io,
    Dequantization,
}

/// Exact winner from a streamed vocabulary projection.  The token id is a
/// flattened row number in the output tensor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamedArgmax {
    pub token_id: u32,
    pub logit: f32,
}

impl core::fmt::Display for TrunkNvmeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidTensor => write!(f, "trunk tensor is not row-addressable"),
            Self::RowOutOfRange => write!(f, "trunk tensor row is out of range"),
            Self::RawScratchTooSmall => write!(f, "trunk raw row scratch is too small"),
            Self::OutputTooSmall => write!(f, "trunk row output is too small"),
            Self::OffsetOverflow => write!(f, "trunk row byte offset overflowed"),
            Self::Io => write!(f, "trunk row I/O failed"),
            Self::Dequantization => write!(f, "trunk row dequantization failed"),
        }
    }
}

impl std::error::Error for TrunkNvmeError {}

/// One source-GGUF cursor. It owns no model payload and caller-owned buffers
/// bound each individual read/dequantization operation.
pub struct TrunkNvmeReader {
    file: File,
    tensor_data_start: u64,
}

impl TrunkNvmeReader {
    pub fn open(source: &Path, tensor_data_start: u64) -> Result<Self, TrunkNvmeError> {
        let file = File::open(source).map_err(|_| TrunkNvmeError::Io)?;
        Ok(Self {
            file,
            tensor_data_start,
        })
    }

    /// Read and dequantize one flattened tensor row. For an expert tensor,
    /// `row` includes its expert-plane offset; no entire expert tensor is read.
    pub fn read_row_into(
        &mut self,
        info: &GgufTensorInfo,
        row: usize,
        raw_scratch: &mut [u8],
        out: &mut [f32],
    ) -> Result<usize, TrunkNvmeError> {
        if info.n_dims == 0 || info.dims[0] == 0 {
            return Err(TrunkNvmeError::InvalidTensor);
        }
        let width = info.dims[0] as usize;
        let row_bytes = crate::ggml_quants::ggml_row_bytes(info.ggml_type, width)
            .ok_or(TrunkNvmeError::InvalidTensor)?;
        if raw_scratch.len() < row_bytes {
            return Err(TrunkNvmeError::RawScratchTooSmall);
        }
        if out.len() < width {
            return Err(TrunkNvmeError::OutputTooSmall);
        }
        let mut total_rows = 1usize;
        for dim in info.dims.iter().take(info.n_dims as usize).skip(1) {
            total_rows = total_rows
                .checked_mul(*dim as usize)
                .ok_or(TrunkNvmeError::OffsetOverflow)?;
        }
        if row >= total_rows {
            return Err(TrunkNvmeError::RowOutOfRange);
        }
        let relative = info
            .byte_offset
            .checked_add(
                (row as u64)
                    .checked_mul(row_bytes as u64)
                    .ok_or(TrunkNvmeError::OffsetOverflow)?,
            )
            .ok_or(TrunkNvmeError::OffsetOverflow)?;
        let offset = self
            .tensor_data_start
            .checked_add(relative)
            .ok_or(TrunkNvmeError::OffsetOverflow)?;
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|_| TrunkNvmeError::Io)?;
        self.file
            .read_exact(&mut raw_scratch[..row_bytes])
            .map_err(|_| TrunkNvmeError::Io)?;
        crate::ggml_quants::dequantize_row_into(
            &raw_scratch[..row_bytes],
            info.ggml_type,
            width,
            out,
        )
        .map_err(|_| TrunkNvmeError::Dequantization)
    }

    /// Execute a bounded streamed matrix-vector product over consecutive
    /// flattened rows. Each row is read, dequantized, dotted, then discarded.
    pub fn gemv_rows_into(
        &mut self,
        info: &GgufTensorInfo,
        first_row: usize,
        rows: usize,
        input: &[f32],
        output: &mut [f32],
        raw_scratch: &mut [u8],
        row_scratch: &mut [f32],
    ) -> Result<(), TrunkNvmeError> {
        let width = info.dims[0] as usize;
        if input.len() < width || output.len() < rows || row_scratch.len() < width {
            return Err(TrunkNvmeError::OutputTooSmall);
        }
        for row_offset in 0..rows {
            self.read_row_into(
                info,
                first_row
                    .checked_add(row_offset)
                    .ok_or(TrunkNvmeError::OffsetOverflow)?,
                raw_scratch,
                &mut row_scratch[..width],
            )?;
            let mut sum = 0.0f32;
            for index in 0..width {
                sum += row_scratch[index] * input[index];
            }
            output[row_offset] = sum;
        }
        Ok(())
    }

    /// Execute one selected plane of a `[in, out, expert]` GGUF tensor.
    /// This is the required MoE storage contract: the ten routed experts are
    /// processed while the other 502 experts remain untouched on disk.
    pub fn gemv_expert_plane_into(
        &mut self,
        info: &GgufTensorInfo,
        expert: usize,
        input: &[f32],
        output: &mut [f32],
        raw_scratch: &mut [u8],
        row_scratch: &mut [f32],
    ) -> Result<usize, TrunkNvmeError> {
        if info.n_dims != 3 || info.dims[1] == 0 || info.dims[2] == 0 {
            return Err(TrunkNvmeError::InvalidTensor);
        }
        let rows_per_expert = info.dims[1] as usize;
        if expert >= info.dims[2] as usize {
            return Err(TrunkNvmeError::RowOutOfRange);
        }
        let first_row = expert
            .checked_mul(rows_per_expert)
            .ok_or(TrunkNvmeError::OffsetOverflow)?;
        self.gemv_rows_into(
            info,
            first_row,
            rows_per_expert,
            input,
            output,
            raw_scratch,
            row_scratch,
        )?;
        Ok(rows_per_expert)
    }

    /// Evaluate a row-addressable output projection without materialising a
    /// logits vector.  The decoder scans one vocabulary row at a time and
    /// retains only the current winner, so a 248k-token vocabulary does not
    /// require a multi-megabyte hot-path allocation.
    pub fn argmax_rows(
        &mut self,
        info: &GgufTensorInfo,
        input: &[f32],
        raw_scratch: &mut [u8],
        row_scratch: &mut [f32],
    ) -> Result<StreamedArgmax, TrunkNvmeError> {
        if info.n_dims != 2 || info.dims[0] == 0 || info.dims[1] == 0 {
            return Err(TrunkNvmeError::InvalidTensor);
        }
        let width = info.dims[0] as usize;
        let rows = info.dims[1] as usize;
        if input.len() < width || row_scratch.len() < width {
            return Err(TrunkNvmeError::OutputTooSmall);
        }
        if rows > u32::MAX as usize {
            return Err(TrunkNvmeError::RowOutOfRange);
        }
        let mut winner = StreamedArgmax {
            token_id: 0,
            logit: f32::NEG_INFINITY,
        };
        for row in 0..rows {
            self.read_row_into(info, row, raw_scratch, &mut row_scratch[..width])?;
            let mut logit = 0.0f32;
            for index in 0..width {
                logit += row_scratch[index] * input[index];
            }
            // Preserve the lowest token ID on exact ties, matching a stable
            // ascending row scan rather than platform-dependent ordering.
            if logit > winner.logit {
                winner = StreamedArgmax {
                    token_id: row as u32,
                    logit,
                };
            }
        }
        Ok(winner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_the_requested_flattened_row() {
        let info = GgufTensorInfo {
            dims: [2, 2, 2, 1],
            n_dims: 3,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 16,
        };
        let mut source = vec![0u8; 16];
        for value in 0..8u32 {
            source.extend_from_slice(&(value as f32).to_le_bytes());
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("trunk.gguf");
        std::fs::write(&path, source).unwrap();
        let mut reader = TrunkNvmeReader::open(&path, 0).unwrap();
        let mut raw = [0u8; 8];
        let mut output = [0f32; 2];
        assert_eq!(
            reader
                .read_row_into(&info, 3, &mut raw, &mut output)
                .unwrap(),
            2
        );
        assert_eq!(output, [6.0, 7.0]);
    }

    #[test]
    fn streams_only_the_selected_expert_plane() {
        let info = GgufTensorInfo {
            dims: [2, 2, 2, 1],
            n_dims: 3,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 0,
        };
        // expert 0: [[1, 2], [3, 4]], expert 1: [[5, 6], [7, 8]]
        let mut source = Vec::new();
        for value in 1..=8u32 {
            source.extend_from_slice(&(value as f32).to_le_bytes());
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("experts.gguf");
        std::fs::write(&path, source).unwrap();
        let mut reader = TrunkNvmeReader::open(&path, 0).unwrap();
        let mut raw = [0u8; 8];
        let mut row = [0f32; 2];
        let mut output = [0f32; 2];
        assert_eq!(
            reader
                .gemv_expert_plane_into(&info, 1, &[1.0, 1.0], &mut output, &mut raw, &mut row,)
                .unwrap(),
            2
        );
        assert_eq!(output, [11.0, 15.0]);
    }

    #[test]
    fn argmax_streams_vocabulary_without_logits_buffer() {
        let info = GgufTensorInfo {
            dims: [2, 3, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 0,
        };
        // Dot with [1, 1] yields 3, 9, and 9. The earlier token must win.
        let mut source = Vec::new();
        for value in [1.0f32, 2.0, 4.0, 5.0, 7.0, 2.0] {
            source.extend_from_slice(&value.to_le_bytes());
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("logits.gguf");
        std::fs::write(&path, source).unwrap();
        let mut reader = TrunkNvmeReader::open(&path, 0).unwrap();
        let mut raw = [0u8; 8];
        let mut row = [0f32; 2];
        assert_eq!(
            reader
                .argmax_rows(&info, &[1.0, 1.0], &mut raw, &mut row)
                .unwrap(),
            StreamedArgmax {
                token_id: 1,
                logit: 9.0
            }
        );
    }
}
