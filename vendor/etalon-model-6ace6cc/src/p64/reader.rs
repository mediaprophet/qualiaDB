use super::integrity::{crc32c, IntegrityMode};
use super::layout::*;
use super::views::{P64ManifoldRecord, P64TensorView};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum P64OverflowContext {
    HyperparameterOffset,
    TensorTable,
    ManifoldCount,
    ManifoldTable,
    ChecksumTable,
    BlobAlignment,
    TensorName,
    TensorBlob,
    ManifoldRecord,
    ScheduleTable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum P64Error {
    TruncatedHeader,
    InvalidMagic,
    UnsupportedVersion(u16),
    UnsupportedEndian,
    InvalidPageSize,
    ArithmeticOverflow(P64OverflowContext),
    HyperparametersOutOfBounds,
    MetadataBounds,
    ManifoldAlignment,
    MetadataChecksum,
    InvalidRank(usize),
    InvalidManifold(usize),
    InvalidName(usize),
    InvalidBlob(usize),
    TensorChecksum(usize),
    IndexOutOfBounds,
    ScheduleBounds,
}

impl core::fmt::Display for P64Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "P64 validation error: {self:?}")
    }
}

impl core::error::Error for P64Error {}

/// A validated view owning only decoded fixed-size metadata and a borrowed slice.
#[derive(Clone, Debug)]
pub struct P64View<'a> {
    bytes: &'a [u8],
    header: P64WeightHeader,
    hparams: P64HParams,
    integrity: IntegrityMode,
}

fn end(
    start: usize,
    count: usize,
    stride: usize,
    context: P64OverflowContext,
) -> Result<usize, P64Error> {
    count
        .checked_mul(stride)
        .and_then(|size| start.checked_add(size))
        .ok_or(P64Error::ArithmeticOverflow(context))
}

impl<'a> P64View<'a> {
    /// Does not allocate, including malformed-input and CRC failure paths.
    pub fn parse(bytes: &'a [u8], integrity: IntegrityMode) -> Result<Self, P64Error> {
        if bytes.len() < P64_WEIGHT_HEADER_BYTES {
            return Err(P64Error::TruncatedHeader);
        }
        // Length checks keep compatibility readers' allocating error paths unreachable.
        let header = P64WeightHeader::read_le(bytes).map_err(|_| P64Error::TruncatedHeader)?;
        if header.magic != P64_MAGIC {
            return Err(P64Error::InvalidMagic);
        }
        if header.version != P64_VERSION {
            return Err(P64Error::UnsupportedVersion(header.version));
        }
        if header.flags & P64_FLAG_LITTLE_ENDIAN == 0 {
            return Err(P64Error::UnsupportedEndian);
        }
        if !header.has_valid_page_size() {
            return Err(P64Error::InvalidPageSize);
        }
        let page = header.page_size as usize;
        let hp_start = header.hparams_offset as usize;
        let hp_end = end(hp_start, 1, 64, P64OverflowContext::HyperparameterOffset)?;
        let hp_bytes = bytes
            .get(hp_start..hp_end)
            .ok_or(P64Error::HyperparametersOutOfBounds)?;
        let hparams =
            P64HParams::read_le(hp_bytes).map_err(|_| P64Error::HyperparametersOutOfBounds)?;
        let count = header.tensor_count as usize;
        let tensor_start = header.tensor_table_offset as usize;
        let tensor_end = end(
            tensor_start,
            count,
            P64_TENSOR_ENTRY_BYTES,
            P64OverflowContext::TensorTable,
        )?;
        let strings = header.string_table_offset as usize;
        let manifold = header.manifold_table_offset as usize;
        let manifold_count = hparams
            .n_layer
            .checked_add(1)
            .ok_or(P64Error::ArithmeticOverflow(
                P64OverflowContext::ManifoldCount,
            ))? as usize;
        let manifold_end = end(
            manifold,
            manifold_count,
            P64_MANIFOLD_ENTRY_BYTES,
            P64OverflowContext::ManifoldTable,
        )?;
        let tokenizer = header.tokenizer_offset as usize;
        let checksums = header.checksum_offset as usize;
        let checksum_count = count.checked_add(1).ok_or(P64Error::ArithmeticOverflow(
            P64OverflowContext::ChecksumTable,
        ))?;
        let checksum_end = end(
            checksums,
            checksum_count,
            4,
            P64OverflowContext::ChecksumTable,
        )?;
        if tensor_start < hp_end
            || tensor_end > strings
            || strings > manifold
            || manifold_end > tokenizer
            || tokenizer > checksums
            || checksum_end > bytes.len()
        {
            return Err(P64Error::MetadataBounds);
        }
        if !header.is_manifold_cache_aligned() {
            return Err(P64Error::ManifoldAlignment);
        }
        if integrity != IntegrityMode::Structure
            && crc32c(&bytes[..checksums]) != read_u32(bytes, checksums)
        {
            return Err(P64Error::MetadataChecksum);
        }
        let mut previous_end = checked_align_up(checksum_end, page).ok_or(
            P64Error::ArithmeticOverflow(P64OverflowContext::BlobAlignment),
        )?;
        let mut previous_manifold = None;
        let layer_packed = header.flags & P64_FLAG_LAYER_PACK != 0;
        for index in 0..count {
            let offset = tensor_start + index * P64_TENSOR_ENTRY_BYTES;
            let entry = read_entry(&bytes[offset..offset + P64_TENSOR_ENTRY_BYTES]);
            if !(1..=4).contains(&entry.rank) {
                return Err(P64Error::InvalidRank(index));
            }
            if entry.manifold_idx as usize >= manifold_count {
                return Err(P64Error::InvalidManifold(index));
            }
            let name = strings
                .checked_add(entry.name_offset as usize)
                .ok_or(P64Error::ArithmeticOverflow(P64OverflowContext::TensorName))?;
            if name >= manifold || !bytes[name..manifold].contains(&0) {
                return Err(P64Error::InvalidName(index));
            }
            let blob = entry.blob_offset as usize;
            let blob_end = end(
                blob,
                entry.blob_size as usize,
                1,
                P64OverflowContext::TensorBlob,
            )?;
            let alignment =
                if layer_packed && index > 0 && previous_manifold == Some(entry.manifold_idx) {
                    256
                } else {
                    page
                };
            if !entry.is_blob_aligned(alignment) || blob < previous_end || blob_end > bytes.len() {
                return Err(P64Error::InvalidBlob(index));
            }
            if integrity == IntegrityMode::Full
                && crc32c(&bytes[blob..blob_end]) != read_u32(bytes, checksums + 4 + index * 4)
            {
                return Err(P64Error::TensorChecksum(index));
            }
            previous_end = blob_end;
            previous_manifold = Some(entry.manifold_idx);
        }
        Ok(Self {
            bytes,
            header,
            hparams,
            integrity,
        })
    }

    pub fn bytes(&self) -> &'a [u8] {
        self.bytes
    }
    pub fn header(&self) -> &P64WeightHeader {
        &self.header
    }
    pub fn hparams(&self) -> &P64HParams {
        &self.hparams
    }
    pub fn integrity(&self) -> IntegrityMode {
        self.integrity
    }
    pub fn tensor_count(&self) -> usize {
        self.header.tensor_count as usize
    }

    pub fn tensor(&self, index: usize) -> Result<P64TensorView<'a>, P64Error> {
        if index >= self.tensor_count() {
            return Err(P64Error::IndexOutOfBounds);
        }
        let start = self.header.tensor_table_offset as usize + index * P64_TENSOR_ENTRY_BYTES;
        let entry = read_entry(&self.bytes[start..start + P64_TENSOR_ENTRY_BYTES]);
        let name_start = self.header.string_table_offset as usize + entry.name_offset as usize;
        let name_region = &self.bytes[name_start..self.header.manifold_table_offset as usize];
        let name_len = name_region
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(P64Error::InvalidName(index))?;
        let blob_start = entry.blob_offset as usize;
        Ok(P64TensorView {
            entry,
            name: &name_region[..name_len],
            blob: &self.bytes[blob_start..blob_start + entry.blob_size as usize],
        })
    }

    pub fn tokenizer_bytes(&self) -> &'a [u8] {
        self.header.tokenizer_slice(self.bytes)
    }

    pub fn manifold_record(&self, index: u32) -> Result<P64ManifoldRecord<'a>, P64Error> {
        if index > self.hparams.n_layer {
            return Err(P64Error::IndexOutOfBounds);
        }
        let start = end(
            self.header.manifold_table_offset as usize,
            index as usize,
            64,
            P64OverflowContext::ManifoldRecord,
        )?;
        let record = self
            .bytes
            .get(start..end(start, 1, 64, P64OverflowContext::ManifoldRecord)?)
            .ok_or(P64Error::IndexOutOfBounds)?;
        Ok(P64ManifoldRecord {
            bytes: record.try_into().map_err(|_| P64Error::IndexOutOfBounds)?,
        })
    }

    /// Bounds-checks the optional schedule on access; parse does not certify its contents.
    pub fn schedule_bytes(&self) -> Result<&'a [u8], P64Error> {
        if self.header.flags & P64_FLAG_LAYER_SCHEDULE == 0 {
            return Ok(&[]);
        }
        let start = self.header.role_table_offset as usize;
        let stop = end(
            start,
            self.hparams.n_layer as usize,
            64,
            P64OverflowContext::ScheduleTable,
        )?;
        let floor = end(
            self.header.hparams_offset as usize,
            1,
            64,
            P64OverflowContext::HyperparameterOffset,
        )?;
        if !is_cache_line_aligned(start)
            || start < floor
            || stop > self.header.tensor_table_offset as usize
        {
            return Err(P64Error::ScheduleBounds);
        }
        self.bytes.get(start..stop).ok_or(P64Error::ScheduleBounds)
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

#[inline]
fn read_entry(bytes: &[u8]) -> P64TensorEntry {
    P64TensorEntry::read_le_unchecked(bytes)
}
