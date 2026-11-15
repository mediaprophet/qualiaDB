//! Canonical v3 metadata validation over its exact contiguous tail and directory.
//! Validates interpretation closure; unrelated brick contents are not inspected.
use super::super::header::EmfError;
use super::codec::{
    add, align64, checked_audio_timing, count, u32_at, u64_at, EmfExtensionView,
    EXTENSION_FOOTER_BYTES, EXTENSION_FOOTER_MAGIC,
};
use super::model::*;
use crate::container_10d::crc32c::crc32c;
use crate::q42::chunk_integrity::{verify_blake3_trailer, BLAKE3_TRAILER_BYTES};
use std::collections::BTreeSet;
pub(crate) fn parse_extension_tail(
    bytes: &[u8],
    field_end: usize,
    total: usize,
    limits: EmfExtensionLimits,
) -> Result<EmfExtensionView<'_>, EmfError> {
    if total > limits.max_file_bytes {
        return Err(EmfError::ExtensionLimits);
    }
    if add(field_end, bytes.len())? != total
        || bytes.len() < BLAKE3_TRAILER_BYTES + EXTENSION_FOOTER_BYTES
    {
        return Err(EmfError::ExtensionMetadata);
    }
    let footer = &bytes[bytes.len() - 64..];
    if footer[..8] != EXTENSION_FOOTER_MAGIC
        || u32_at(footer, 44) != u32::from(EMF_EXTENSION_SCHEMA_VERSION)
        || footer[48..60].iter().any(|b| *b != 0)
    {
        return Err(EmfError::ExtensionMetadata);
    }
    let computed = crc32c(&footer[..60]);
    let stored = u32_at(footer, 60);
    if stored != computed {
        return Err(EmfError::ChecksumMismatch { stored, computed });
    }
    let declared_start = count(u64_at(footer, 8))?;
    if declared_start != field_end {
        return Err(EmfError::ExtensionMetadata);
    }
    let start = 0usize;
    let json_len = count(u64_at(footer, 16))?;
    let pcm_len = count(u64_at(footer, 24))?;
    let payload_len = count(u64_at(footer, 32))?;
    let track_count = u32_at(footer, 40) as usize;
    if field_end < 64
        || field_end % 64 != 0
        || json_len == 0
        || json_len > limits.max_metadata_bytes
        || pcm_len > limits.max_audio_bytes
        || track_count > limits.max_tracks
        || track_count > 16
        || payload_len % 64 != 0
    {
        return Err(EmfError::ExtensionLimits);
    }
    let json_aligned = align64(json_len)?;
    if add(json_aligned, pcm_len)? != payload_len
        || add(add(start, payload_len)?, 128)? != bytes.len()
    {
        return Err(EmfError::ExtensionMetadata);
    }
    let end = add(start, payload_len)?;
    // Authenticate exact metadata/padding/audio before parsing JSON or allocating metadata.
    if !verify_blake3_trailer(&bytes[start..end], &bytes[end..end + 64]) {
        return Err(EmfError::ExtensionHashMismatch);
    }
    if bytes[start + json_len..start + json_aligned]
        .iter()
        .any(|b| *b != 0)
    {
        return Err(EmfError::ExtensionMetadata);
    }
    let metadata: EmfExtensionMetadata = serde_json::from_slice(&bytes[start..start + json_len])
        .map_err(|_| EmfError::ExtensionMetadata)?;
    if metadata.schema_version != EMF_EXTENSION_SCHEMA_VERSION
        || metadata.audio_tracks.len() != track_count
    {
        return Err(EmfError::ExtensionMetadata);
    }
    metadata.field.validate()?;
    let pcm = &bytes[start + json_aligned..end];
    let mut next = 0usize;
    let mut ids = BTreeSet::new();
    for (index, descriptor) in metadata.audio_tracks.iter().enumerate() {
        descriptor.spec.validate(index)?;
        let length = count(descriptor.pcm_bytes)?;
        let offset = count(descriptor.pcm_offset)?;
        let expected = count(descriptor.frames)?
            .checked_mul(descriptor.spec.channels as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or(EmfError::ArithmeticOverflow)?;
        if descriptor.frames == 0
            || length != expected
            || offset != next
            || !ids.insert(descriptor.spec.id.as_str())
        {
            return Err(EmfError::AudioTrack { track_index: index });
        }
        let last = add(offset, length)?;
        next = align64(last)?;
        let payload = pcm
            .get(offset..last)
            .ok_or(EmfError::AudioTrack { track_index: index })?;
        if pcm
            .get(last..next)
            .ok_or(EmfError::AudioTrack { track_index: index })?
            .iter()
            .any(|b| *b != 0)
        {
            return Err(EmfError::AudioTrack { track_index: index });
        }
        if *blake3::hash(payload).as_bytes() != descriptor.blake3 {
            return Err(EmfError::AudioHashMismatch { track_index: index });
        }
        checked_audio_timing(
            &descriptor.spec,
            descriptor.frames,
            &metadata.field.timebase,
        )?;
        for bytes in payload.chunks_exact(4) {
            let sample = f32::from_le_bytes(
                bytes
                    .try_into()
                    .map_err(|_| EmfError::AudioTrack { track_index: index })?,
            );
            if !sample.is_finite()
                || (matches!(descriptor.spec.units, EmfSignalUnits::NormalizedAmplitude)
                    && sample.abs() > 1.0)
            {
                return Err(EmfError::AudioTrack { track_index: index });
            }
        }
    }
    if next != pcm.len() {
        return Err(EmfError::ExtensionMetadata);
    }
    Ok(EmfExtensionView {
        metadata,
        pcm,
        field_end,
    })
}

/// v3 canonical field layout and explicit Hilbert/time interpretation.
pub(crate) fn validate_directory_bytes(
    header: &super::super::header::EmfContainerHeader,
    bytes: &[u8],
    extension: &EmfExtensionView<'_>,
) -> Result<(), EmfError> {
    use super::super::chunk_directory::{EmfChunkDirectoryEntry, CHUNK_DIRECTORY_ENTRY_BYTES};
    use crate::q42::m64_matter::brick3d::{decode_hilbert3d_u64, encode_hilbert3d_u64};
    let directory = count(header.directory_offset)?;
    let chunks = header.chunk_count as usize;
    if chunks.checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES) != Some(bytes.len()) {
        return Err(EmfError::DirectoryBounds);
    }
    let field = &extension.metadata.field;
    field.validate_header(header)?;
    if directory != 64 {
        return Err(EmfError::ExtensionMetadata);
    }
    let mut next = add(
        directory,
        chunks
            .checked_mul(CHUNK_DIRECTORY_ENTRY_BYTES)
            .ok_or(EmfError::ArithmeticOverflow)?,
    )?;
    let mut previous = None;
    let mut occupancy = BTreeSet::new();
    for index in 0..chunks {
        let offset = index * CHUNK_DIRECTORY_ENTRY_BYTES;
        let entry: EmfChunkDirectoryEntry =
            bytemuck::pod_read_unaligned(&bytes[offset..offset + CHUNK_DIRECTORY_ENTRY_BYTES]);
        entry.validate()?;
        if entry.bao_hash_prefix != [0; 16]
            || entry.reserved != [0; 12]
            || usize::try_from(entry.cell_count)
                .map_err(|_| EmfError::ArithmeticOverflow)?
                .checked_mul(64)
                .and_then(|n| n.checked_add(BLAKE3_TRAILER_BYTES))
                != Some(entry.byte_length as usize)
        {
            return Err(EmfError::InvalidIntegrityEncoding { chunk_index: index });
        }
        if count(entry.byte_offset)? != next
            || entry.flags != super::super::header::EMF_CHUNK_FLAG_BLAKE3_FULL
            || entry.cell_count == 0
            || entry.cell_count > 512
            || previous.is_some_and(|key| key > entry.spatial_hilbert_index)
            || !occupancy.insert((entry.spatial_hilbert_index, entry.time_slice_index))
        {
            return Err(EmfError::ExtensionMetadata);
        }
        next = add(next, entry.byte_length as usize)?;
        previous = Some(entry.spatial_hilbert_index);
        let order = u32::from(field.hilbert_order);
        let (x, y, z) = decode_hilbert3d_u64(entry.spatial_hilbert_index, order);
        if x >= header.grid_dims_bricks[0]
            || y >= header.grid_dims_bricks[1]
            || z >= header.grid_dims_bricks[2]
            || encode_hilbert3d_u64(x, y, z, order) != entry.spatial_hilbert_index
        {
            return Err(EmfError::HilbertOrder);
        }
        field.timebase.slice_tick(entry.time_slice_index)?;
    }
    if next != extension.field_end {
        return Err(EmfError::ExtensionMetadata);
    }
    Ok(())
}
