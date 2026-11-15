//! Current-format original PCM producer. No defaults, normalization or invented
//! calibration, pressure relationship, coordinate frame or measured provenance.
use super::{Error, Limits, Scalar, Timing};
use crate::q42::{field_envelope as env, field_semantics as sem};
mod metadata;
mod samples;
pub use samples::PcmSamples;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PcmQuantity {
    DigitalSample,
    RawSampleCount,
    Pressure,
}
impl PcmQuantity {
    fn names(self) -> (&'static str, &'static str) {
        match self {
            Self::DigitalSample => ("digital_sample", "1"),
            Self::RawSampleCount => ("raw_sample_count", "1"),
            Self::Pressure => ("pressure", "Pa"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignalRelation {
    DigitalUnrelatedToPressure,
    RawCounts,
    PhysicalBaseUnits,
}
impl SignalRelation {
    fn name(self) -> &'static str {
        match self {
            Self::DigitalUnrelatedToPressure => "digital-unrelated-to-pressure",
            Self::RawCounts => "raw-counts",
            Self::PhysicalBaseUnits => "physical-base-units",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ProvenanceRole {
    Measured = 0,
    Model = 1,
    Derived = 2,
    Command = 3,
    OpaquePreserved = 4,
}
/// Supplied assertions only; recording role/purpose grants no execution authority.
#[derive(Clone, Copy, Debug)]
pub struct Provenance<'a> {
    pub role: ProvenanceRole,
    pub algorithm: Option<&'a str>,
    pub algorithm_version: Option<&'a str>,
    pub instrument_uri: Option<&'a str>,
    pub lineage: Option<&'a str>,
}
#[derive(Clone, Copy, Debug)]
pub struct PcmDocument<'a> {
    pub samples: PcmSamples<'a>,
    pub timing: Timing,
    pub stream_id: u32,
    pub channels: u32,
    pub channel_order: &'a str,
    pub purpose: &'a str,
    pub quantity: PcmQuantity,
    pub relation: SignalRelation,
    pub units_per_stored_sample: f64,
    pub stored_offset: f64,
    pub provenance: Provenance<'a>,
    pub original: Option<&'a [u8]>,
}
/// Disposable caller staging. No buffer here is a published result on error.
pub struct WriteBuffers<'a> {
    pub metadata: &'a mut [u8],
    pub payload: &'a mut [u8],
    pub file: &'a mut [u8],
}
#[derive(Clone, Copy, Debug, Default)]
pub struct WriteOptions {
    pub envelope: env::Limits,
    pub shared: sem::Limits,
    pub signal: Limits,
}
impl PcmDocument<'_> {
    fn preflight(&self, options: WriteOptions) -> Result<(usize, u64, u32), Error> {
        let n = self.samples.encoded_len()?;
        let scalar = self.samples.scalar();
        let stride = self
            .channels
            .checked_mul(scalar.bytes())
            .ok_or(Error::Overflow)?;
        if stride == 0 || n == 0 || n % stride as usize != 0 || self.stream_id == 0 {
            return Err(Error::Shape);
        }
        let records = n as u64 / u64::from(stride);
        let original = self.original.map_or(0, |b| b.len() as u64);
        if self.original.is_some_and(|b| b.is_empty()) {
            return Err(Error::Shape);
        }
        let bytes = (n as u64).checked_add(original).ok_or(Error::Overflow)?;
        let samples = (n as u64 / u64::from(scalar.bytes()))
            .checked_add(original)
            .ok_or(Error::Overflow)?;
        if options.signal.work < bytes.checked_mul(2).ok_or(Error::Overflow)? {
            return Err(Error::Limit);
        }
        if bytes > options.signal.payload_bytes
            || samples > options.signal.samples
            || bytes > options.envelope.aggregate_decoded_bytes
            || n as u64 > options.envelope.chunk_bytes
            || original > options.envelope.chunk_bytes
            || options.signal.entries < if self.original.is_some() { 2 } else { 1 }
        {
            return Err(Error::Limit);
        }
        if !self.units_per_stored_sample.is_finite()
            || self.units_per_stored_sample <= 0.0
            || !self.stored_offset.is_finite()
        {
            return Err(Error::Profile);
        }
        match (self.quantity, self.relation) {
            (PcmQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure) => {}
            (PcmQuantity::RawSampleCount, SignalRelation::RawCounts) => {
                if !matches!(scalar, Scalar::I16 | Scalar::I24 | Scalar::I32)
                    || self.units_per_stored_sample != 1.0
                    || self.stored_offset != 0.0
                {
                    return Err(Error::Unit);
                }
            }
            (PcmQuantity::Pressure, SignalRelation::PhysicalBaseUnits) => {
                if self.units_per_stored_sample != 1.0 || self.stored_offset != 0.0 {
                    return Err(Error::Unit);
                }
            }
            _ => return Err(Error::Unit),
        }
        self.timing.time_at(records - 1)?;
        Ok((n, records, stride))
    }
    /// Metadata, payload and file staging are caller-owned. On every failure the
    /// entire final output is unchanged; source slices are never modified. Original
    /// bytes become a bound opaque source entry, not a replacement for PCM samples.
    pub fn encode_into(
        &self,
        buffers: WriteBuffers<'_>,
        scratch: &mut sem::Scratch<'_>,
        output: &mut [u8],
        options: WriteOptions,
    ) -> Result<usize, Error> {
        let (n, records, stride) = self.preflight(options)?;
        let payload_n = self.samples.encode_into(buffers.payload)?;
        debug_assert_eq!(n, payload_n);
        let meta_n = metadata::encode(self, records, buffers.metadata, options.envelope)?;
        let empty = env::EntryInput {
            mode: 0x104,
            revision: 1,
            flags: 0,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: 1,
            stride: 1,
            codec: 0,
            descriptor_id: 8,
            payload: &[],
        };
        let mut entries = [empty; 2];
        let mut count = 0;
        if let Some(original) = self.original {
            entries[0] = env::EntryInput {
                record_count: original.len() as u64,
                payload: original,
                ..empty
            };
            count = 1;
        }
        entries[count] = env::EntryInput {
            mode: 0x100,
            revision: 1,
            flags: 2,
            stream_id: self.stream_id,
            frame_id: 0,
            start_tick: self.timing.start_tick,
            time_count: records,
            spatial_key: 0,
            record_count: records,
            stride,
            codec: 0,
            descriptor_id: 5,
            payload: &buffers.payload[..n],
        };
        count += 1;
        let input = env::EnvelopeInput {
            family: env::Family::QacV1Candidate,
            flags: 0,
            metadata: &buffers.metadata[..meta_n],
            entries: &entries[..count],
        };
        super::encode(
            input,
            options.envelope,
            options.shared,
            options.signal,
            scratch,
            buffers.file,
            output,
        )
    }
}
#[cfg(test)]
mod tests;
