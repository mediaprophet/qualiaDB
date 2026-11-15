//! Selected in-bounds samples from an authenticated M64 georaster v3 directory.
//! This is not a full-volume receipt and exposes no padded-cell access.
use super::{AuthenticatedRange, FormatError as E, FormatLimits};
use crate::q42::m64_matter::{
    georaster::{
        range::{sample_location, validate_header_directory, validate_sample_record, SampleError},
        M64GeoRasterError, M64GeoRasterMetadata, M64GeoRasterShape, M64RasterSample,
        M64_GEORASTER_HEADER_BYTES,
    },
    M64ChunkDirectoryEntry, M64Header,
};

pub struct M64GeoRasterDirectoryView<'a> {
    header: M64Header,
    metadata: M64GeoRasterMetadata,
    shape: M64GeoRasterShape,
    directory: AuthenticatedRange<'a>,
    max_sample_bytes: usize,
}

impl<'a> M64GeoRasterDirectoryView<'a> {
    /// Admit header, GRM3 metadata, and the complete directory. All must be
    /// authenticated ranges from the same selected entry.
    pub fn admit(
        header: AuthenticatedRange<'a>,
        grm3: AuthenticatedRange<'a>,
        directory: AuthenticatedRange<'a>,
        limits: FormatLimits,
        max_records: u64,
    ) -> Result<Self, E> {
        if !cfg!(target_endian = "little") {
            return Err(E::Unsupported);
        }
        if !header.same_entry(&grm3) || !header.same_entry(&directory) {
            return Err(E::Identity);
        }
        if header.offset() != 0 || header.bytes().len() != 64 {
            return Err(E::Bounds);
        }
        if header.selected().logical_length() > limits.max_entry_bytes {
            return Err(E::Limit);
        }
        if grm3.offset() != 64 || grm3.bytes().len() != 128 {
            return Err(E::Bounds);
        }
        let header_value: M64Header = bytemuck::pod_read_unaligned(header.bytes());
        header_value.validate().map_err(|_| E::Integrity)?;
        if header_value.version != 3 {
            return Err(E::Unsupported);
        }
        let metadata = M64GeoRasterMetadata::decode(grm3.bytes()).map_err(|_| E::Integrity)?;
        let shape = metadata.shape().map_err(|_| E::Geometry)?;
        let rows = shape.chunks.checked_mul(64).ok_or(E::Bounds)?;
        let metadata_bytes = 64usize
            .checked_add(128)
            .and_then(|n| n.checked_add(rows))
            .ok_or(E::Bounds)?;
        if shape.chunks > limits.max_rows
            || shape.records as u64 > max_records
            || metadata_bytes > limits.max_metadata_bytes
        {
            return Err(E::Limit);
        }
        if directory.offset() != M64_GEORASTER_HEADER_BYTES as u64
            || directory.bytes().len() != rows
        {
            return Err(E::Bounds);
        }
        let shape = validate_header_directory(
            &header_value,
            &metadata,
            header.selected().logical_length(),
            directory.bytes(),
        )
        .map_err(map_geometry_error)?;
        if limits.max_chunk_bytes < 64 {
            return Err(E::Limit);
        }
        Ok(Self {
            header: header_value,
            metadata,
            shape,
            directory,
            max_sample_bytes: limits.max_chunk_bytes,
        })
    }

    pub const fn header(&self) -> &M64Header {
        &self.header
    }

    pub const fn metadata(&self) -> &M64GeoRasterMetadata {
        &self.metadata
    }

    /// Verify and return one exact 64-byte in-bounds raster sample.
    /// No padded cell or whole-chunk accessor is provided.
    pub fn selected_sample(
        &self,
        x: usize,
        y: usize,
        sample: AuthenticatedRange<'a>,
    ) -> Result<M64RasterSample, E> {
        if !self.directory.same_entry(&sample) {
            return Err(E::Identity);
        }
        if sample.bytes().len() != 64 || sample.bytes().len() > self.max_sample_bytes {
            return Err(E::Bounds);
        }
        let location = sample_location(&self.shape, x, y).map_err(map_sample_error)?;
        let row_start = location.chunk.checked_mul(64).ok_or(E::Bounds)?;
        let raw = self
            .directory
            .bytes()
            .get(row_start..row_start.checked_add(64).ok_or(E::Bounds)?)
            .ok_or(E::Bounds)?;
        let row: M64ChunkDirectoryEntry = bytemuck::pod_read_unaligned(raw);
        let record_offset = u64::try_from(location.record)
            .map_err(|_| E::Bounds)?
            .checked_mul(64)
            .ok_or(E::Bounds)?;
        let expected = row.byte_offset.checked_add(record_offset).ok_or(E::Bounds)?;
        if sample.offset() != expected {
            return Err(E::Bounds);
        }
        validate_sample_record(&self.shape, &self.metadata, location, sample.bytes())
            .map_err(map_sample_error)
    }
}

fn map_geometry_error(error: M64GeoRasterError) -> E {
    match error {
        M64GeoRasterError::Overflow => E::Bounds,
        _ => E::Geometry,
    }
}

fn map_sample_error(error: SampleError) -> E {
    match error {
        SampleError::Padding => E::Geometry,
        SampleError::Geometry => E::Bounds,
        SampleError::RecordCrc | SampleError::NonRasterFields => E::Integrity,
    }
}
