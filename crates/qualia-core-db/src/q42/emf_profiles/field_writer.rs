//! Caller-owned staging followed by one atomic final publication copy.
use super::super::field_envelope::{EntryInput, EnvelopeInput, Family, Limits as StorageLimits};
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct FieldClock {
    pub scale: u32,
    pub epoch_seconds: i64,
    pub epoch_nanoseconds: u32,
    pub ticks_per_second_num: u64,
    pub ticks_per_second_den: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct FieldFrame {
    pub origin_m: [f64; 3],
    pub rotation_xyzw: [f64; 4],
    pub grid: Option<GridSpec>,
}
#[derive(Clone, Copy, Debug)]
pub struct FieldProvenance<'a> {
    pub role: u32,
    pub algorithm: Option<&'a str>,
    pub algorithm_version: Option<&'a str>,
    pub lineage: Option<&'a str>,
}
/// Exact current128-byte records, borrowed immutably. encode_field_samples packs
/// original f64 observations separately when a producer starts from native data.
#[derive(Clone, Copy, Debug)]
pub struct FieldChunkInput<'a> {
    pub stream_id: u32,
    pub start_tick: i64,
    pub time_count: u64,
    pub spatial_key: Option<u64>,
    pub payload: &'a [u8],
}
#[derive(Clone, Copy, Debug)]
pub struct FieldDocument<'a> {
    pub clock: FieldClock,
    pub frame: FieldFrame,
    pub provenance: FieldProvenance<'a>,
    pub temporal_step_ticks: Option<i64>,
    pub chunks: &'a [FieldChunkInput<'a>],
    pub align_payloads_to_4096: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct FieldWriteLimits {
    pub storage: StorageLimits,
    pub profiles: Limits,
    pub total_payload_work: u64,
}
impl Default for FieldWriteLimits {
    fn default() -> Self {
        Self {
            storage: StorageLimits::default(),
            profiles: Limits::default(),
            total_payload_work: 512 << 20,
        }
    }
}
/// Planning/storage fields borrow caller metadata/entry arenas and immutable source
/// payloads. No reference to mutable payload staging escapes this prepare step.
pub struct PreparedField<'m, 'e, 's> {
    metadata: &'m [u8],
    entries: &'e [EntryInput<'s>],
    flags: u32,
    limits: FieldWriteLimits,
}
/// Initialize caller-owned entry slots; raw encoding rejects these until prepare fills them.
pub fn field_entry_slot<'a>() -> EntryInput<'a> {
    EntryInput {
        mode: 0,
        revision: 1,
        flags: 0,
        stream_id: 0,
        frame_id: 2,
        start_tick: 0,
        time_count: 0,
        spatial_key: 0,
        record_count: 0,
        stride: 128,
        codec: 0,
        descriptor_id: 5,
        payload: &[],
    }
}
pub fn field_metadata_len(
    document: &FieldDocument<'_>,
    limits: FieldWriteLimits,
) -> Result<usize, Error> {
    super::field_metadata::with_metadata(document, |input| {
        input.encoded_len(limits.storage).map_err(Error::Storage)
    })
}
pub fn prepare_field_document<'m, 'e, 's>(
    document: &FieldDocument<'s>,
    metadata: &'m mut [u8],
    entry_slots: &'e mut [EntryInput<'s>],
    limits: FieldWriteLimits,
) -> Result<PreparedField<'m, 'e, 's>, Error> {
    if document.chunks.is_empty() {
        return Err(Error::Shape);
    }
    if document.chunks.len() > entry_slots.len() {
        return Err(Error::Limit);
    }
    if document.chunks.len() as u64 > u64::from(limits.storage.entries) {
        return Err(Error::Limit);
    }
    let mut payload_work = 0u64;
    for chunk in document.chunks {
        if chunk.payload.is_empty()
            || chunk.payload.len() % 128 != 0
            || chunk.spatial_key.is_some() != document.frame.grid.is_some()
        {
            return Err(Error::Shape);
        }
        if chunk.payload.len() as u64 / 128 > limits.profiles.records {
            return Err(Error::Limit);
        }
        payload_work = payload_work
            .checked_add(
                (chunk.payload.len() as u64)
                    .checked_mul(2)
                    .ok_or(Error::Overflow)?,
            )
            .ok_or(Error::Overflow)?;
    }
    if payload_work > limits.total_payload_work {
        return Err(Error::Limit);
    }
    let metadata = super::field_metadata::with_metadata(document, |input| {
        input
            .encode_into(metadata, limits.storage)
            .map_err(Error::Storage)
    })?;
    let entries = &mut entry_slots[..document.chunks.len()];
    for (slot, chunk) in entries.iter_mut().zip(document.chunks) {
        *slot = EntryInput {
            mode: 0,
            revision: 1,
            flags: if document.temporal_step_ticks.is_some() {
                2
            } else {
                0
            } | if chunk.spatial_key.is_some() { 4 } else { 0 },
            stream_id: chunk.stream_id,
            frame_id: 2,
            start_tick: chunk.start_tick,
            time_count: chunk.time_count,
            spatial_key: chunk.spatial_key.unwrap_or(0),
            record_count: chunk.payload.len() as u64 / 128,
            stride: 128,
            codec: 0,
            descriptor_id: 5,
            payload: chunk.payload,
        };
    }
    let prepared = PreparedField {
        metadata,
        entries,
        flags: u32::from(document.align_payloads_to_4096),
        limits,
    };
    prepared.encoded_len()?;
    Ok(prepared)
}
impl PreparedField<'_, '_, '_> {
    fn envelope(&self) -> EnvelopeInput<'_> {
        EnvelopeInput {
            family: Family::EmfV4Candidate,
            flags: self.flags,
            metadata: self.metadata,
            entries: self.entries,
        }
    }
    pub fn encoded_len(&self) -> Result<usize, Error> {
        self.envelope()
            .encoded_len(self.limits.storage)
            .map_err(Error::Storage)
    }
    /// Staging may change on error; the ENTIRE final output remains unchanged.
    /// All source chunks are immutable; every selected chunk is verified/admitted
    /// before the one final copy. Any extra output tail is retained on success.
    pub fn encode_into<'out>(
        &self,
        staging: &mut [u8],
        output: &'out mut [u8],
        scratch: &mut Scratch<'_>,
    ) -> Result<&'out mut [u8], Error> {
        let len = self.encoded_len()?;
        if output.len() < len {
            return Err(Error::Storage(super::super::field_envelope::Error::Extent));
        }
        let staged = self
            .envelope()
            .encode_into(staging, self.limits.storage)
            .map_err(Error::Storage)?;
        let view = StructuralView::open(staged, Family::EmfV4Candidate, self.limits.storage)
            .map_err(Error::Storage)?;
        let admitted = semantics::admit_shared(
            &view,
            &ProfileRegistry,
            scratch,
            self.limits.profiles.shared,
        )
        .map_err(shared_error)?;
        for index in 1..=view.entry_count() {
            super::chunk::admit_with_context(
                &view,
                &admitted.context,
                index,
                self.limits.profiles,
            )?;
        }
        output[..len].copy_from_slice(staged);
        Ok(&mut output[..len])
    }
}
/// Native observations; no guessed carrier, constitutive law, flux or confidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldSample {
    pub electric_v_per_m: [f64; 3],
    pub magnetic_tesla: [f64; 3],
    pub position_m: [f64; 3],
    pub tick: i64,
    pub carrier_hz: Option<f64>,
    pub stored_poynting_w_per_m2: Option<[f64; 3]>,
    pub confidence: Option<f32>,
}
pub fn encode_field_samples<'out>(
    samples: &[FieldSample],
    output: &'out mut [u8],
    limits: Limits,
) -> Result<&'out mut [u8], Error> {
    if samples.is_empty() {
        return Err(Error::Shape);
    }
    if samples.len() as u64 > limits.records {
        return Err(Error::Limit);
    }
    let bytes = samples.len().checked_mul(128).ok_or(Error::Overflow)?;
    if (bytes as u64).checked_mul(2).ok_or(Error::Overflow)? > limits.payload_work {
        return Err(Error::Limit);
    }
    if output.len() < bytes {
        return Err(Error::Storage(super::super::field_envelope::Error::Extent));
    }
    for sample in samples {
        let mut record = [0; 128];
        pack(sample, &mut record);
        super::records::field_record(&record, true, sample.tick)?;
    }
    for (sample, record) in samples.iter().zip(output[..bytes].chunks_exact_mut(128)) {
        pack(sample, record);
    }
    Ok(&mut output[..bytes])
}
fn pack(sample: &FieldSample, record: &mut [u8]) {
    record.fill(0);
    for (start, values) in [
        (0, &sample.electric_v_per_m),
        (24, &sample.magnetic_tesla),
        (48, &sample.position_m),
    ] {
        for (axis, value) in values.iter().enumerate() {
            record[start + axis * 8..start + axis * 8 + 8].copy_from_slice(&value.to_le_bytes());
        }
    }
    record[72..80].copy_from_slice(&sample.tick.to_le_bytes());
    let mut flags = 0u32;
    if let Some(carrier) = sample.carrier_hz {
        flags |= 1;
        record[80..88].copy_from_slice(&carrier.to_le_bytes());
    }
    if let Some(flux) = sample.stored_poynting_w_per_m2 {
        flags |= 4;
        for (axis, value) in flux.iter().enumerate() {
            record[96 + axis * 8..104 + axis * 8].copy_from_slice(&value.to_le_bytes());
        }
    }
    if let Some(confidence) = sample.confidence {
        flags |= 8;
        record[124..128].copy_from_slice(&confidence.to_le_bytes());
    }
    record[120..124].copy_from_slice(&flags.to_le_bytes());
}
