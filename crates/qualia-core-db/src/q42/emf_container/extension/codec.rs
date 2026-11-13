//! Exact-byte extension integrity, checked extents and borrowed PCM access.
use super::super::header::EmfError;
use super::model::*;
use crate::container_10d::crc32c::crc32c;
use crate::q42::chunk_integrity::{blake3_trailer, BLAKE3_TRAILER_BYTES};
use std::collections::BTreeSet;

pub const EXTENSION_FOOTER_BYTES: usize = 64;
pub const EXTENSION_FOOTER_MAGIC: [u8; 8] = *b"Q42EMFX3";

pub(super) fn align64(value: usize) -> Result<usize, EmfError> {
    value
        .checked_add(63)
        .map(|v| v & !63)
        .ok_or(EmfError::ArithmeticOverflow)
}
pub(super) fn add(a: usize, b: usize) -> Result<usize, EmfError> {
    a.checked_add(b).ok_or(EmfError::ArithmeticOverflow)
}
pub(super) fn count(value: u64) -> Result<usize, EmfError> {
    usize::try_from(value).map_err(|_| EmfError::ArithmeticOverflow)
}
pub(super) fn u64_at(bytes: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap_or([0; 8]))
}
pub(super) fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap_or([0; 4]))
}

/// Metadata is bounded owned cold state; sample payloads remain borrowed original bytes.
#[derive(Clone, Debug)]
pub struct EmfExtensionView<'a> {
    pub(super) metadata: EmfExtensionMetadata,
    pub(super) pcm: &'a [u8],
    pub(crate) field_end: usize,
}
impl<'a> EmfExtensionView<'a> {
    pub fn metadata(&self) -> &EmfExtensionMetadata {
        &self.metadata
    }
    pub fn audio_payload(&self, track: usize) -> Option<&'a [u8]> {
        let descriptor = self.metadata.audio_tracks.get(track)?;
        let start = usize::try_from(descriptor.pcm_offset).ok()?;
        let length = usize::try_from(descriptor.pcm_bytes).ok()?;
        self.pcm.get(start..start.checked_add(length)?)
    }
    pub fn audio_sample(&self, track: usize, frame: u64, channel: u16) -> Option<f32> {
        let descriptor = self.metadata.audio_tracks.get(track)?;
        if frame >= descriptor.frames || channel >= descriptor.spec.channels {
            return None;
        }
        let sample = frame
            .checked_mul(u64::from(descriptor.spec.channels))?
            .checked_add(u64::from(channel))?;
        let offset = usize::try_from(sample).ok()?.checked_mul(4)?;
        let bytes = self
            .audio_payload(track)?
            .get(offset..offset.checked_add(4)?)?;
        Some(f32::from_le_bytes(bytes.try_into().ok()?))
    }
}

pub(crate) struct PreparedExtension {
    pub metadata: EmfExtensionMetadata,
    json: Vec<u8>,
    json_aligned: usize,
    pcm_bytes: usize,
    limits: EmfExtensionLimits,
}
impl PreparedExtension {
    pub fn required_file_bytes(&self, field_end: usize) -> Result<usize, EmfError> {
        let bytes = add(
            add(add(field_end, self.json_aligned)?, self.pcm_bytes)?,
            BLAKE3_TRAILER_BYTES + EXTENSION_FOOTER_BYTES,
        )?;
        if bytes > self.limits.max_file_bytes {
            return Err(EmfError::ExtensionLimits);
        }
        Ok(bytes)
    }
    pub fn write_into(
        &self,
        tracks: &[EmfAudioTrack<'_>],
        out: &mut [u8],
        field_end: usize,
    ) -> Result<usize, EmfError> {
        let required = self.required_file_bytes(field_end)?;
        if field_end % 64 != 0
            || out.len() < required
            || tracks.len() != self.metadata.audio_tracks.len()
        {
            return Err(EmfError::ExtensionMetadata);
        }
        let payload_end = required - BLAKE3_TRAILER_BYTES - EXTENSION_FOOTER_BYTES;
        out[field_end..required].fill(0);
        out[field_end..field_end + self.json.len()].copy_from_slice(&self.json);
        let pcm_start = field_end + self.json_aligned;
        for (index, track) in tracks.iter().enumerate() {
            let descriptor = &self.metadata.audio_tracks[index];
            if track.samples.len().checked_mul(4) != Some(count(descriptor.pcm_bytes)?) {
                return Err(EmfError::AudioTrack { track_index: index });
            }
            let start = add(pcm_start, count(descriptor.pcm_offset)?)?;
            for (sample, bytes) in track
                .samples
                .iter()
                .zip(out[start..start + count(descriptor.pcm_bytes)?].chunks_exact_mut(4))
            {
                bytes.copy_from_slice(&sample.to_le_bytes());
            }
        }
        let trailer = blake3_trailer(&out[field_end..payload_end]);
        out[payload_end..payload_end + BLAKE3_TRAILER_BYTES].copy_from_slice(&trailer);
        let mut footer = [0u8; 64];
        footer[..8].copy_from_slice(&EXTENSION_FOOTER_MAGIC);
        footer[8..16].copy_from_slice(&(field_end as u64).to_le_bytes());
        footer[16..24].copy_from_slice(&(self.json.len() as u64).to_le_bytes());
        footer[24..32].copy_from_slice(&(self.pcm_bytes as u64).to_le_bytes());
        footer[32..40].copy_from_slice(&((payload_end - field_end) as u64).to_le_bytes());
        footer[40..44].copy_from_slice(&(tracks.len() as u32).to_le_bytes());
        footer[44..48].copy_from_slice(&u32::from(EMF_EXTENSION_SCHEMA_VERSION).to_le_bytes());
        let crc = crc32c(&footer[..60]);
        footer[60..].copy_from_slice(&crc.to_le_bytes());
        out[required - 64..required].copy_from_slice(&footer);
        Ok(required)
    }
}

pub(super) fn checked_audio_timing(
    spec: &EmfAudioTrackSpec,
    frames: u64,
    time: &EmfTimebase,
) -> Result<(), EmfError> {
    // Check the common rational endpoint without converting a timestamp to float.
    let rate = i128::from(spec.sample_rate_num);
    let den = i128::from(time.ticks_per_second_den);
    let start = i128::from(spec.start_tick)
        .checked_mul(rate)
        .and_then(|v| v.checked_mul(den))
        .ok_or(EmfError::ArithmeticOverflow)?;
    let duration = i128::from(frames)
        .checked_mul(i128::from(spec.sample_rate_den))
        .and_then(|v| v.checked_mul(i128::from(time.ticks_per_second_num)))
        .ok_or(EmfError::ArithmeticOverflow)?;
    start
        .checked_add(duration)
        .ok_or(EmfError::ArithmeticOverflow)?;
    rate.checked_mul(i128::from(time.ticks_per_second_num))
        .ok_or(EmfError::ArithmeticOverflow)?;
    Ok(())
}

pub(crate) fn prepare_extension(
    field: &EmfFieldInterpretation,
    tracks: &[EmfAudioTrack<'_>],
    limits: EmfExtensionLimits,
) -> Result<PreparedExtension, EmfError> {
    field.validate()?;
    if tracks.len() > limits.max_tracks || tracks.len() > 16 {
        return Err(EmfError::ExtensionLimits);
    }
    let mut descriptors = Vec::with_capacity(tracks.len());
    let mut ids = BTreeSet::new();
    let mut pcm_bytes = 0usize;
    for (index, track) in tracks.iter().enumerate() {
        track.spec.validate(index)?;
        if !ids.insert(track.spec.id.as_str())
            || track.samples.is_empty()
            || track.samples.len() % track.spec.channels as usize != 0
        {
            return Err(EmfError::AudioTrack { track_index: index });
        }
        let length = track
            .samples
            .len()
            .checked_mul(4)
            .ok_or(EmfError::ArithmeticOverflow)?;
        let start = pcm_bytes;
        pcm_bytes = align64(add(start, length)?)?;
        if pcm_bytes > limits.max_audio_bytes {
            return Err(EmfError::ExtensionLimits);
        }
        let frames = (track.samples.len() / track.spec.channels as usize) as u64;
        checked_audio_timing(&track.spec, frames, &field.timebase)?;
        let mut hash = blake3::Hasher::new();
        for sample in track.samples {
            if !sample.is_finite()
                || (matches!(track.spec.units, EmfSignalUnits::NormalizedAmplitude)
                    && sample.abs() > 1.0)
            {
                return Err(EmfError::AudioTrack { track_index: index });
            }
            hash.update(&sample.to_le_bytes());
        }
        descriptors.push(EmfAudioTrackDescriptor {
            codec: EmfAudioCodec::PcmF32Le,
            spec: track.spec.clone(),
            frames,
            pcm_offset: start as u64,
            pcm_bytes: length as u64,
            blake3: *hash.finalize().as_bytes(),
        });
    }
    let metadata = EmfExtensionMetadata {
        schema_version: EMF_EXTENSION_SCHEMA_VERSION,
        field: field.clone(),
        audio_tracks: descriptors,
    };
    // All variable strings/track counts were bounded before this cold serialization.
    let json = serde_json::to_vec(&metadata).map_err(|_| EmfError::ExtensionMetadata)?;
    if json.len() > limits.max_metadata_bytes {
        return Err(EmfError::ExtensionLimits);
    }
    Ok(PreparedExtension {
        metadata,
        json_aligned: align64(json.len())?,
        json,
        pcm_bytes,
        limits,
    })
}

pub(crate) fn parse_extension(
    bytes: &[u8],
    limits: EmfExtensionLimits,
) -> Result<EmfExtensionView<'_>, EmfError> {
    if bytes.len() > limits.max_file_bytes {
        return Err(EmfError::ExtensionLimits);
    }
    if bytes.len() < 64 + BLAKE3_TRAILER_BYTES + EXTENSION_FOOTER_BYTES {
        return Err(EmfError::ExtensionMetadata);
    }
    let footer = &bytes[bytes.len() - EXTENSION_FOOTER_BYTES..];
    let field_end = count(u64_at(footer, 8))?;
    let tail = bytes.get(field_end..).ok_or(EmfError::ExtensionMetadata)?;
    super::range::parse_extension_tail(tail, field_end, bytes.len(), limits)
}

/// Complete-reader wrapper around the same canonical range directory checks.
pub(crate) fn validate_field_directory(
    header: &super::super::header::EmfContainerHeader,
    directory: usize,
    chunks: usize,
    bytes: &[u8],
    extension: &EmfExtensionView<'_>,
) -> Result<(), EmfError> {
    if count(header.directory_offset)? != directory || header.chunk_count as usize != chunks {
        return Err(EmfError::DirectoryBounds);
    }
    let length = chunks
        .checked_mul(super::super::chunk_directory::CHUNK_DIRECTORY_ENTRY_BYTES)
        .ok_or(EmfError::ArithmeticOverflow)?;
    let rows = bytes
        .get(directory..add(directory, length)?)
        .ok_or(EmfError::DirectoryBounds)?;
    super::range::validate_directory_bytes(header, rows, extension)
}
