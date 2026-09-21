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

/// Cumulative physical I/O accounting for one reader.  `reads` counts file
/// read calls (chunked), `rows` counts model rows consumed, `bytes` the raw
/// GGUF bytes pulled from storage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrunkIoStats {
    pub rows: u64,
    pub reads: u64,
    pub bytes: u64,
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
/// bound each individual read/dequantization operation.  A fixed 4 MiB read
/// chunk amortizes syscalls across consecutive rows without holding model
/// weights; it carries raw bytes only.
///
/// In mapped mode (`open_mapped`) the file is exposed through a read-only
/// `memmap2` view so row access slices the OS page cache instead of issuing
/// syscalls.  `stats.reads` then counts real syscalls (zero), while rows and
/// bytes still record the logical data volume touched.
pub struct TrunkNvmeReader {
    file: File,
    map: Option<memmap2::Mmap>,
    tensor_data_start: u64,
    chunk: Vec<u8>,
    stats: TrunkIoStats,
}

const TRUNK_CHUNK_BYTES: usize = 4 * 1024 * 1024;

/// f32 dot product accumulated through eight independent lanes so LLVM can
/// vectorize the multiply-accumulate; a strict left-to-right fold cannot be
/// reassociated under IEEE rules and stays scalar.  Lane order changes the
/// result's last-ulp rounding only.
#[inline]
fn dot_f32(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let a = &a[..n];
    let b = &b[..n];
    let mut acc = [0.0f32; 8];
    let mut index = 0;
    while index + 8 <= n {
        for lane in 0..8 {
            acc[lane] += a[index + lane] * b[index + lane];
        }
        index += 8;
    }
    let mut sum = (acc[0] + acc[1]) + (acc[2] + acc[3]) + (acc[4] + acc[5]) + (acc[6] + acc[7]);
    while index < n {
        sum += a[index] * b[index];
        index += 1;
    }
    sum
}

impl TrunkNvmeReader {
    pub fn open(source: &Path, tensor_data_start: u64) -> Result<Self, TrunkNvmeError> {
        let file = File::open(source).map_err(|_| TrunkNvmeError::Io)?;
        Ok(Self {
            file,
            map: None,
            tensor_data_start,
            chunk: vec![0u8; TRUNK_CHUNK_BYTES],
            stats: TrunkIoStats::default(),
        })
    }

    /// Open the same source through a read-only OS mapping.  Page residency
    /// is the OS's decision; this reader still owns no decoded weights.
    pub fn open_mapped(source: &Path, tensor_data_start: u64) -> Result<Self, TrunkNvmeError> {
        let file = File::open(source).map_err(|_| TrunkNvmeError::Io)?;
        // Safety: read-only mapping of a file the decode contract treats as
        // immutable for the session, identical to gguf_bridge's memmap2 use.
        let map = unsafe { memmap2::Mmap::map(&file) }.map_err(|_| TrunkNvmeError::Io)?;
        Ok(Self {
            file,
            map: Some(map),
            tensor_data_start,
            chunk: Vec::new(),
            stats: TrunkIoStats::default(),
        })
    }

    /// Whether row access is served from the page-cache mapping.
    pub fn is_mapped(&self) -> bool {
        self.map.is_some()
    }

    /// Cumulative physical I/O performed by this reader.
    pub fn io_stats(&self) -> TrunkIoStats {
        self.stats
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
        let mapped = match self.map.as_ref() {
            Some(map) => {
                let start = usize::try_from(offset)
                    .map_err(|_| TrunkNvmeError::OffsetOverflow)?;
                let end = start
                    .checked_add(row_bytes)
                    .ok_or(TrunkNvmeError::OffsetOverflow)?;
                Some(map.get(start..end).ok_or(TrunkNvmeError::Io)?)
            }
            None => None,
        };
        let raw: &[u8] = match mapped {
            Some(bytes) => bytes,
            None => {
                self.file
                    .seek(SeekFrom::Start(offset))
                    .map_err(|_| TrunkNvmeError::Io)?;
                self.file
                    .read_exact(&mut raw_scratch[..row_bytes])
                    .map_err(|_| TrunkNvmeError::Io)?;
                self.stats.reads += 1;
                &raw_scratch[..row_bytes]
            }
        };
        self.stats.rows += 1;
        self.stats.bytes += row_bytes as u64;
        crate::ggml_quants::dequantize_row_into(raw, info.ggml_type, width, out)
            .map_err(|_| TrunkNvmeError::Dequantization)
    }

    /// Read `rows` consecutive flattened rows starting at `first_row` through
    /// the internal chunk buffer: one file read per chunk, one dequantization
    /// per row.  `visit` receives each dequantized row in order.
    fn read_row_run<F>(
        &mut self,
        info: &GgufTensorInfo,
        first_row: usize,
        rows: usize,
        raw_scratch: &mut [u8],
        row_scratch: &mut [f32],
        mut visit: F,
    ) -> Result<(), TrunkNvmeError>
    where
        F: FnMut(usize, &[f32]) -> Result<(), TrunkNvmeError>,
    {
        if info.n_dims == 0 || info.dims[0] == 0 {
            return Err(TrunkNvmeError::InvalidTensor);
        }
        let width = info.dims[0] as usize;
        let row_bytes = crate::ggml_quants::ggml_row_bytes(info.ggml_type, width)
            .ok_or(TrunkNvmeError::InvalidTensor)?;
        if row_scratch.len() < width {
            return Err(TrunkNvmeError::OutputTooSmall);
        }
        let mut total_rows = 1usize;
        for dim in info.dims.iter().take(info.n_dims as usize).skip(1) {
            total_rows = total_rows
                .checked_mul(*dim as usize)
                .ok_or(TrunkNvmeError::OffsetOverflow)?;
        }
        let end = first_row
            .checked_add(rows)
            .ok_or(TrunkNvmeError::OffsetOverflow)?;
        if end > total_rows {
            return Err(TrunkNvmeError::RowOutOfRange);
        }
        if let Some(map) = self.map.as_ref() {
            let start = usize::try_from(
                self.tensor_data_start
                    .checked_add(info.byte_offset)
                    .and_then(|base| {
                        base.checked_add((first_row as u64) * row_bytes as u64)
                    })
                    .ok_or(TrunkNvmeError::OffsetOverflow)?,
            )
            .map_err(|_| TrunkNvmeError::OffsetOverflow)?;
            let span = rows
                .checked_mul(row_bytes)
                .ok_or(TrunkNvmeError::OffsetOverflow)?;
            let bytes = map
                .get(start..start.checked_add(span).ok_or(TrunkNvmeError::OffsetOverflow)?)
                .ok_or(TrunkNvmeError::Io)?;
            self.stats.rows += rows as u64;
            self.stats.bytes += span as u64;
            for index in 0..rows {
                let row_start = index * row_bytes;
                crate::ggml_quants::dequantize_row_into(
                    &bytes[row_start..row_start + row_bytes],
                    info.ggml_type,
                    width,
                    &mut row_scratch[..width],
                )
                .map_err(|_| TrunkNvmeError::Dequantization)?;
                visit(first_row + index, &row_scratch[..width])?;
            }
            return Ok(());
        }
        if row_bytes > self.chunk.len() || row_bytes > raw_scratch.len() {
            // Fall back to one read per row when the fixed chunk cannot hold it.
            for row in first_row..end {
                self.read_row_into(info, row, raw_scratch, row_scratch)?;
                visit(row, &row_scratch[..width])?;
            }
            return Ok(());
        }
        let rows_per_chunk = self.chunk.len() / row_bytes;
        let mut row = first_row;
        while row < end {
            let take = rows_per_chunk.min(end - row);
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
            let span = take
                .checked_mul(row_bytes)
                .ok_or(TrunkNvmeError::OffsetOverflow)?;
            self.file
                .seek(SeekFrom::Start(offset))
                .map_err(|_| TrunkNvmeError::Io)?;
            self.file
                .read_exact(&mut self.chunk[..span])
                .map_err(|_| TrunkNvmeError::Io)?;
            self.stats.reads += 1;
            self.stats.rows += take as u64;
            self.stats.bytes += span as u64;
            for index in 0..take {
                let start = index * row_bytes;
                crate::ggml_quants::dequantize_row_into(
                    &self.chunk[start..start + row_bytes],
                    info.ggml_type,
                    width,
                    &mut row_scratch[..width],
                )
                .map_err(|_| TrunkNvmeError::Dequantization)?;
                visit(row + index, &row_scratch[..width])?;
            }
            row += take;
        }
        Ok(())
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
        let row_bytes = crate::ggml_quants::ggml_row_bytes(info.ggml_type, width)
            .ok_or(TrunkNvmeError::InvalidTensor)?;
        if let Some(map) = self.map.as_ref() {
            // Mapped + parallel: each worker dequantizes rows into its own
            // scratch.  Output slots are disjoint, so results are identical
            // to the sequential path regardless of chunking or thread count.
            let start = usize::try_from(
                self.tensor_data_start
                    .checked_add(info.byte_offset)
                    .and_then(|base| {
                        base.checked_add((first_row as u64) * row_bytes as u64)
                    })
                    .ok_or(TrunkNvmeError::OffsetOverflow)?,
            )
            .map_err(|_| TrunkNvmeError::OffsetOverflow)?;
            let span = rows
                .checked_mul(row_bytes)
                .ok_or(TrunkNvmeError::OffsetOverflow)?;
            let bytes = map
                .get(start..start.checked_add(span).ok_or(TrunkNvmeError::OffsetOverflow)?)
                .ok_or(TrunkNvmeError::Io)?;
            self.stats.rows += rows as u64;
            self.stats.bytes += span as u64;
            use rayon::prelude::*;
            output[..rows]
                .par_chunks_mut(256)
                .enumerate()
                .map_init(
                    || vec![0.0f32; width],
                    |scratch, (chunk, out_chunk)| {
                        for (i, slot) in out_chunk.iter_mut().enumerate() {
                            let row_start = (chunk * 256 + i) * row_bytes;
                            if crate::ggml_quants::dequantize_row_into(
                                &bytes[row_start..row_start + row_bytes],
                                info.ggml_type,
                                width,
                                scratch,
                            )
                            .is_err()
                            {
                                return Err(TrunkNvmeError::Dequantization);
                            }
                            *slot = dot_f32(scratch, input);
                        }
                        Ok(())
                    },
                )
                .collect::<Result<(), TrunkNvmeError>>()?;
            return Ok(());
        }
        self.read_row_run(info, first_row, rows, raw_scratch, row_scratch, |row, row_values| {
            output[row - first_row] = dot_f32(row_values, input);
            Ok(())
        })
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
        let mut winner = [StreamedArgmax {
            token_id: 0,
            logit: f32::NEG_INFINITY,
        }];
        self.topk_rows(info, input, raw_scratch, row_scratch, &mut winner)?;
        Ok(winner[0])
    }

    /// Stream the same projection retaining the `out.len()` highest logits,
    /// sorted descending.  Lower token IDs win exact ties, matching the
    /// ascending row scan.
    pub fn topk_rows(
        &mut self,
        info: &GgufTensorInfo,
        input: &[f32],
        raw_scratch: &mut [u8],
        row_scratch: &mut [f32],
        out: &mut [StreamedArgmax],
    ) -> Result<(), TrunkNvmeError> {
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
        if out.is_empty() {
            return Ok(());
        }
        for slot in out.iter_mut() {
            *slot = StreamedArgmax {
                token_id: 0,
                logit: f32::NEG_INFINITY,
            };
        }
        let row_bytes = crate::ggml_quants::ggml_row_bytes(info.ggml_type, width)
            .ok_or(TrunkNvmeError::InvalidTensor)?;
        if let Some(map) = self.map.as_ref() {
            // Parallel local top-k per chunk, then a stable merge.  Chunk
            // order is ascending row, and equal logits keep insertion order,
            // so the merge preserves the sequential ascending-row tie-break.
            let start = usize::try_from(
                self.tensor_data_start
                    .checked_add(info.byte_offset)
                    .ok_or(TrunkNvmeError::OffsetOverflow)?,
            )
            .map_err(|_| TrunkNvmeError::OffsetOverflow)?;
            let span = rows
                .checked_mul(row_bytes)
                .ok_or(TrunkNvmeError::OffsetOverflow)?;
            let bytes = map
                .get(start..start.checked_add(span).ok_or(TrunkNvmeError::OffsetOverflow)?)
                .ok_or(TrunkNvmeError::Io)?;
            self.stats.rows += rows as u64;
            self.stats.bytes += span as u64;
            let k = out.len();
            const ROWS_PER_CHUNK: usize = 2048;
            let n_chunks = rows.div_ceil(ROWS_PER_CHUNK);
            use rayon::prelude::*;
            let partials: Vec<Vec<StreamedArgmax>> = (0..n_chunks)
                .into_par_iter()
                .map_init(
                    || vec![0.0f32; width],
                    |scratch, chunk| {
                        let mut local =
                            vec![
                                StreamedArgmax {
                                    token_id: 0,
                                    logit: f32::NEG_INFINITY,
                                };
                                k
                            ];
                        let row_end = ((chunk + 1) * ROWS_PER_CHUNK).min(rows);
                        for row in chunk * ROWS_PER_CHUNK..row_end {
                            let row_start = row * row_bytes;
                            if crate::ggml_quants::dequantize_row_into(
                                &bytes[row_start..row_start + row_bytes],
                                info.ggml_type,
                                width,
                                scratch,
                            )
                            .is_err()
                            {
                                return Err(TrunkNvmeError::Dequantization);
                            }
                            let logit = dot_f32(scratch, input);
                            if logit <= local[k - 1].logit {
                                continue;
                            }
                            let mut slot = k;
                            while slot > 0 && logit > local[slot - 1].logit {
                                slot -= 1;
                            }
                            for index in (slot + 1..k).rev() {
                                local[index] = local[index - 1];
                            }
                            local[slot] = StreamedArgmax {
                                token_id: row as u32,
                                logit,
                            };
                        }
                        Ok(local)
                    },
                )
                .collect::<Result<Vec<Vec<StreamedArgmax>>, TrunkNvmeError>>()?;
            let mut merged: Vec<StreamedArgmax> = Vec::with_capacity(n_chunks * k);
            for local in partials {
                merged.extend(local);
            }
            merged.retain(|entry| entry.logit != f32::NEG_INFINITY);
            merged.sort_by(|a, b| {
                b.logit
                    .partial_cmp(&a.logit)
                    .unwrap_or(core::cmp::Ordering::Equal)
            });
            let keep = merged.len().min(k);
            out[..keep].copy_from_slice(&merged[..keep]);
            return Ok(());
        }
        self.read_row_run(info, 0, rows, raw_scratch, row_scratch, |row, row_values| {
            let logit = dot_f32(row_values, input);
            if logit <= out[out.len() - 1].logit {
                return Ok(());
            }
            let mut slot = out.len();
            while slot > 0 && logit > out[slot - 1].logit {
                slot -= 1;
            }
            for index in (slot + 1..out.len()).rev() {
                out[index] = out[index - 1];
            }
            out[slot] = StreamedArgmax {
                token_id: row as u32,
                logit,
            };
            Ok(())
        })?;
        Ok(())
    }

    /// Report each `target_rows` token's logit and its rank across the whole
    /// projection (rank 0 = argmax).  Diagnostics for comparing against a
    /// reference distribution: the target rows are read directly, then one
    /// streamed pass counts how many vocabulary logits beat each target.
    /// `out[i]` receives `(logit, rank)` for `target_rows[i]`.
    pub fn logits_and_ranks(
        &mut self,
        info: &GgufTensorInfo,
        input: &[f32],
        target_rows: &[u32],
        raw_scratch: &mut [u8],
        row_scratch: &mut [f32],
        out: &mut [(f32, u64)],
    ) -> Result<(), TrunkNvmeError> {
        if info.n_dims != 2 || info.dims[0] == 0 || info.dims[1] == 0 {
            return Err(TrunkNvmeError::InvalidTensor);
        }
        let width = info.dims[0] as usize;
        let rows = info.dims[1] as usize;
        if input.len() < width || row_scratch.len() < width || out.len() < target_rows.len() {
            return Err(TrunkNvmeError::OutputTooSmall);
        }
        for (slot, &target) in target_rows.iter().enumerate() {
            self.read_row_into(info, target as usize, raw_scratch, row_scratch)?;
            let logit = dot_f32(&row_scratch[..width], input);
            out[slot] = (logit, 0);
        }
        self.read_row_run(info, 0, rows, raw_scratch, row_scratch, |_row, row_values| {
            let logit = dot_f32(row_values, input);
            for slot in out.iter_mut() {
                if logit > slot.0 {
                    slot.1 += 1;
                }
            }
            Ok(())
        })
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

    #[test]
    fn mapped_parallel_paths_match_streamed_results() {
        // Enough rows to span several parallel chunks, with a deliberate
        // cross-chunk tie (rows 1 and 4097 both score 9) so the merge must
        // keep the ascending-row winner the sequential scan would pick.
        let rows = 5000usize;
        let info = GgufTensorInfo {
            dims: [2, rows as u64, 1, 1],
            n_dims: 2,
            ggml_type: crate::ggml_quants::GGML_TYPE_F32,
            byte_offset: 0,
        };
        let mut source = Vec::with_capacity(rows * 8);
        for row in 0..rows {
            let (a, b) = if row == 1 || row == 4097 {
                (4.0f32, 5.0f32)
            } else {
                ((row % 7) as f32, (row % 3) as f32)
            };
            source.extend_from_slice(&a.to_le_bytes());
            source.extend_from_slice(&b.to_le_bytes());
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mapped.gguf");
        std::fs::write(&path, source).unwrap();

        let input = [1.0f32, 1.0];
        let mut streamed = TrunkNvmeReader::open(&path, 0).unwrap();
        let mut mapped = TrunkNvmeReader::open_mapped(&path, 0).unwrap();
        assert!(mapped.is_mapped());
        let mut raw = [0u8; 8];
        let mut row = [0f32; 2];

        let mut gemv_streamed = [0f32; 5000];
        let mut gemv_mapped = [0f32; 5000];
        streamed
            .gemv_rows_into(&info, 0, rows, &input, &mut gemv_streamed, &mut raw, &mut row)
            .unwrap();
        mapped
            .gemv_rows_into(&info, 0, rows, &input, &mut gemv_mapped, &mut raw, &mut row)
            .unwrap();
        assert_eq!(gemv_streamed, gemv_mapped);

        let mut top_streamed = [StreamedArgmax {
            token_id: 0,
            logit: 0.0,
        }; 4];
        let mut top_mapped = [StreamedArgmax {
            token_id: 0,
            logit: 0.0,
        }; 4];
        streamed
            .topk_rows(&info, &input, &mut raw, &mut row, &mut top_streamed)
            .unwrap();
        mapped
            .topk_rows(&info, &input, &mut raw, &mut row, &mut top_mapped)
            .unwrap();
        assert_eq!(top_streamed, top_mapped);
        assert_eq!(top_mapped[0].token_id, 1);
        assert_eq!(top_mapped[1].token_id, 4097);
    }
}
