//! Current-format Hann-windowed complex STFT + full retained f32 PCM producer.
//! No fake synthesis, zero padding, magnitude reduction, or hidden allocation.
use super::provider::{transform_into, Plan, ProviderError, ProviderLimits};
use crate::audio::qac::signal::{
    writer::{PcmSamples, Provenance, ProvenanceRole, SignalRelation},
    Error, Limits, Scalar, Timing,
};
use crate::q42::{
    field_envelope::{self as env, BlockInput, EntryInput, EnvelopeInput, FieldInput, FieldValue as V, MetadataInput},
    field_semantics as sem,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StftQuantity {
    DigitalSample,
    Pressure,
}
impl StftQuantity {
    fn pcm_names(self) -> (&'static str, &'static str) {
        match self {
            Self::DigitalSample => ("digital_sample", "1"),
            Self::Pressure => ("pressure", "Pa"),
        }
    }
    fn stft_names(self) -> (&'static str, &'static str) {
        match self {
            Self::DigitalSample => ("complex_digital_spectrum", "1"),
            Self::Pressure => ("complex_pressure_spectrum", "Pa"),
        }
    }
}

/// Caller-owned staging buffers for STFT calculation and container serialization.
/// None of these buffers is a published result when encoding returns an error.
pub struct StftWriteBuffers<'a> {
    pub mono: &'a mut [f32],
    pub fft: &'a mut [f32],
    pub window: &'a mut [f32],
    pub spectra: &'a mut [u8],
    pub metadata: &'a mut [u8],
    pub payload: &'a mut [u8],
    pub file: &'a mut [u8],
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StftWriteOptions {
    pub envelope: env::Limits,
    pub shared: sem::Limits,
    pub signal: Limits,
    pub provider: ProviderLimits,
}

#[derive(Clone, Copy, Debug)]
pub struct StftDocument<'a> {
    pub samples: PcmSamples<'a>,
    pub timing: Timing,
    pub stream_id: u32,
    pub channels: u32,
    pub channel_order: &'a str,
    pub fft_size: u32,
    pub hop: u32,
    pub pcm_purpose: &'a str,
    pub stft_purpose: &'a str,
    pub quantity: StftQuantity,
    pub relation: SignalRelation,
    pub pcm_provenance: Provenance<'a>,
    pub stft_provenance: Provenance<'a>,
    pub original: Option<&'a [u8]>,
}

fn map_provider_err(err: ProviderError) -> Error {
    match err {
        ProviderError::Shape | ProviderError::Frame | ProviderError::ShortInput => Error::Shape,
        ProviderError::Capacity => Error::Envelope(env::Error::Extent),
        ProviderError::Limit => Error::Limit,
        ProviderError::Overflow => Error::Overflow,
        ProviderError::Nonfinite => Error::Nonfinite,
        ProviderError::Provider(etalon_signal::SignalError::NonFiniteInput) => Error::Nonfinite,
        ProviderError::Provider(_) => Error::Shape,
    }
}

fn relation_name(relation: SignalRelation) -> &'static str {
    match relation {
        SignalRelation::DigitalUnrelatedToPressure => "digital-unrelated-to-pressure",
        SignalRelation::RawCounts => "raw-counts",
        SignalRelation::PhysicalBaseUnits => "physical-base-units",
    }
}

fn f(key: u16, value: V<'_>) -> FieldInput<'_> {
    FieldInput { key, flags: 1, value }
}
fn b<'a>(id: u32, schema_type: u16, fields: &'a [FieldInput<'a>]) -> BlockInput<'a> {
    BlockInput { id, schema_type, revision: 1, flags: 1, fields }
}
fn push<'a>(out: &mut [FieldInput<'a>], n: &mut usize, key: u16, value: V<'a>) {
    out[*n] = f(key, value);
    *n += 1;
}

fn fill_provenance<'a>(
    out: &mut [FieldInput<'a>; 7],
    role: &'a [u32; 1],
    digest: Option<&'a [u8; 32]>,
    prov: &'a Provenance<'a>,
) -> usize {
    let mut n = 0;
    push(out, &mut n, 1, V::U32(role));
    if let Some(d) = digest {
        push(out, &mut n, 2, V::Bytes(d));
    }
    for (k, opt) in [(3, prov.algorithm), (4, prov.algorithm_version), (5, prov.instrument_uri), (7, prov.lineage)] {
        if let Some(s) = opt {
            push(out, &mut n, k, V::Utf8(s));
        }
    }
    n
}

impl StftDocument<'_> {
    pub const fn pcm_entry_index(&self) -> u32 {
        if self.original.is_some() { 2 } else { 1 }
    }
    pub const fn stft_entry_index(&self) -> u32 {
        if self.original.is_some() { 3 } else { 2 }
    }
    fn preflight(&self, options: StftWriteOptions) -> Result<(usize, Plan, u32, u32, crate::audio::qac::signal::Rate), Error> {
        if self.samples.scalar() != Scalar::F32 {
            return Err(Error::Scalar);
        }
        if self.stream_id == 0 || self.channels == 0 {
            return Err(Error::Shape);
        }
        if self.stft_provenance.role != ProvenanceRole::Derived
            || self.stft_provenance.algorithm.is_none()
            || self.stft_provenance.algorithm_version.is_none()
        {
            return Err(Error::Profile);
        }
        match (self.quantity, self.relation) {
            (StftQuantity::DigitalSample, SignalRelation::DigitalUnrelatedToPressure)
            | (StftQuantity::Pressure, SignalRelation::PhysicalBaseUnits) => {}
            _ => return Err(Error::Unit),
        }
        if self.original.is_some_and(|b| b.is_empty()) {
            return Err(Error::Shape);
        }
        let pcm_bytes = self.samples.encoded_len()?;
        let fft_size = usize::try_from(self.fft_size).map_err(|_| Error::Overflow)?;
        let hop = usize::try_from(self.hop).map_err(|_| Error::Overflow)?;
        let plan = Plan::new(pcm_bytes, self.channels, fft_size, hop, options.provider)
            .map_err(map_provider_err)?;
        let pcm_stride = self.channels.checked_mul(4).ok_or(Error::Overflow)?;
        let stft_stride = self
            .channels
            .checked_mul(self.fft_size)
            .and_then(|n| n.checked_mul(8))
            .ok_or(Error::Overflow)?;
        let frame_rate = self.timing.samples.divided_by(self.hop)?;
        let orig_bytes = self.original.map_or(0u64, |b| b.len() as u64);
        let stft_bytes = plan.output_bytes() as u64;
        let total_bytes = (pcm_bytes as u64)
            .checked_add(stft_bytes)
            .and_then(|n| n.checked_add(orig_bytes))
            .ok_or(Error::Overflow)?;
        let pcm_scalars = pcm_bytes as u64 / 4;
        let stft_scalars = stft_bytes / 8;
        let total_samples = pcm_scalars
            .checked_add(stft_scalars)
            .and_then(|n| n.checked_add(orig_bytes))
            .ok_or(Error::Overflow)?;
        let min_entries = if self.original.is_some() { 3 } else { 2 };
        if options.signal.work < total_bytes.checked_mul(2).ok_or(Error::Overflow)?
            || total_bytes > options.signal.payload_bytes
            || total_samples > options.signal.samples
            || total_bytes > options.envelope.aggregate_decoded_bytes
            || pcm_bytes as u64 > options.envelope.chunk_bytes
            || stft_bytes > options.envelope.chunk_bytes
            || orig_bytes > options.envelope.chunk_bytes
            || options.signal.entries < min_entries
        {
            return Err(Error::Limit);
        }
        self.timing.time_at((plan.sample_frames() as u64) - 1)?;
        let stft_timing = Timing { samples: frame_rate, ..self.timing };
        stft_timing.time_at((plan.frames() as u64) - 1)?;
        Ok((pcm_bytes, plan, pcm_stride, stft_stride, frame_rate))
    }

    /// Computes Hann-windowed complex STFT via `etalon_signal`, serializes PCM,
    /// STFT, optional opaque original, and strict schema metadata, then runs
    /// full candidate admission before writing any byte to `output`.
    pub fn encode_into(
        &self,
        buffers: StftWriteBuffers<'_>,
        scratch: &mut sem::Scratch<'_>,
        output: &mut [u8],
        options: StftWriteOptions,
    ) -> Result<usize, Error> {
        let (pcm_bytes, plan, pcm_stride, stft_stride, frame_rate) = self.preflight(options)?;
        let pcm_n = self.samples.encode_into(buffers.payload)?;
        debug_assert_eq!(pcm_n, pcm_bytes);
        let pcm_slice = &buffers.payload[..pcm_n];
        let fft_size = plan.fft_size();
        let stft_plan = transform_into(
            pcm_slice,
            self.channels,
            fft_size,
            plan.hop(),
            buffers.mono,
            buffers.fft,
            buffers.window,
            buffers.spectra,
            options.provider,
        )
        .map_err(map_provider_err)?;
        let stft_n = stft_plan.output_bytes();
        let stft_slice = &buffers.spectra[..stft_n];
        let window_slice = &buffers.window[..fft_size];
        let pcm_digest = *blake3::hash(pcm_slice).as_bytes();
        let orig_digest = self.original.map(|raw| *blake3::hash(raw).as_bytes());
        let meta_n = encode_metadata(
            self,
            &plan,
            frame_rate,
            window_slice,
            &pcm_digest,
            orig_digest.as_ref(),
            buffers.metadata,
            options.envelope,
        )?;
        let empty = EntryInput {
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
            descriptor_id: 13,
            payload: &[],
        };
        let mut entries = [empty; 3];
        let mut count = 0;
        if let Some(original) = self.original {
            entries[count] = EntryInput {
                record_count: original.len() as u64,
                payload: original,
                ..empty
            };
            count += 1;
        }
        entries[count] = EntryInput {
            mode: 0x100,
            revision: 1,
            flags: 2,
            stream_id: self.stream_id,
            frame_id: 0,
            start_tick: self.timing.start_tick,
            time_count: plan.sample_frames() as u64,
            spatial_key: 0,
            record_count: plan.sample_frames() as u64,
            stride: pcm_stride,
            codec: 0,
            descriptor_id: 5,
            payload: pcm_slice,
        };
        count += 1;
        entries[count] = EntryInput {
            mode: 0x101,
            revision: 1,
            flags: 2,
            stream_id: self.stream_id,
            frame_id: 0,
            start_tick: self.timing.start_tick,
            time_count: plan.frames() as u64,
            spatial_key: 0,
            record_count: plan.frames() as u64,
            stride: stft_stride,
            codec: 0,
            descriptor_id: 10,
            payload: stft_slice,
        };
        count += 1;
        let input = EnvelopeInput {
            family: env::Family::QacV1Candidate,
            flags: 0,
            metadata: &buffers.metadata[..meta_n],
            entries: &entries[..count],
        };
        super::super::encode(
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

#[allow(clippy::too_many_arguments)]
fn encode_metadata(
    doc: &StftDocument<'_>,
    plan: &Plan,
    frame_rate: crate::audio::qac::signal::Rate,
    window: &[f32],
    pcm_digest: &[u8; 32],
    orig_digest: Option<&[u8; 32]>,
    out: &mut [u8],
    limits: env::Limits,
) -> Result<usize, Error> {
    let t = doc.timing;
    let scale = [t.scale];
    let epoch = [t.epoch_seconds];
    let ns = [t.epoch_nanoseconds];
    let ticks_n = [t.ticks.numerator];
    let ticks_d = [t.ticks.denominator];
    let clock = [
        f(1, V::U32(&scale)),
        f(2, V::I64(&epoch)),
        f(3, V::U32(&ns)),
        f(4, V::U64(&ticks_n)),
        f(5, V::U64(&ticks_d)),
    ];

    let empty = f(0, V::Bytes(&[]));
    let pcm_role = [doc.pcm_provenance.role as u32];
    let mut pcm_prov = [empty; 7];
    let pcm_pn = fill_provenance(&mut pcm_prov, &pcm_role, orig_digest, &doc.pcm_provenance);
    let stft_role = [doc.stft_provenance.role as u32];
    let mut stft_prov = [empty; 7];
    let stft_pn = fill_provenance(&mut stft_prov, &stft_role, Some(pcm_digest), &doc.stft_provenance);

    let pcm_dims = [plan.sample_frames() as u64, u64::from(doc.channels)];
    let pcm_shape = [
        f(1, V::Utf8("urn:qualia:qac:pcm-preservation:1")),
        f(2, V::Utf8("1")),
        f(14, V::U64(&pcm_dims)),
        f(15, V::Utf8("sample")),
    ];
    let stft_dims = [plan.frames() as u64, u64::from(doc.channels), u64::from(doc.fft_size)];
    let stft_shape = [
        f(1, V::Utf8("urn:qualia:qac:stft-preservation:1")),
        f(2, V::Utf8("1")),
        f(14, V::U64(&stft_dims)),
        f(15, V::Utf8("real,imag")),
    ];

    let gain_one = [1.0f64];
    let offset_zero = [0.0f64];
    let rel_str = relation_name(doc.relation);
    let pcm_encoding = [
        f(1, V::Utf8(doc.channel_order)),
        f(2, V::F64(&gain_one)),
        f(3, V::F64(&offset_zero)),
        f(4, V::Utf8(doc.pcm_purpose)),
        f(5, V::Utf8(rel_str)),
    ];
    let stft_encoding = [
        f(1, V::Utf8(doc.channel_order)),
        f(2, V::F64(&gain_one)),
        f(3, V::F64(&offset_zero)),
        f(4, V::Utf8(doc.stft_purpose)),
        f(5, V::Utf8(rel_str)),
    ];

    let (pcm_q, pcm_u) = doc.quantity.pcm_names();
    let (stft_q, stft_u) = doc.quantity.stft_names();
    let clock_id = [1];
    let pcm_prov_id = [2];
    let pcm_ext = [3, 4];
    let channels = [doc.channels];
    let f32_scalar = [Scalar::F32 as u32];
    let in_rate_n = [t.samples.numerator];
    let in_rate_d = [t.samples.denominator];
    let orig_src = [1];
    let mut pcm_desc = [empty; 12];
    let mut pdn = 0;
    for (k, v) in [
        (1, V::MetadataRefs(&clock_id)),
        (3, V::MetadataRefs(&pcm_prov_id)),
        (7, V::Utf8(pcm_q)),
        (8, V::Utf8(pcm_u)),
        (9, V::U32(&channels)),
        (10, V::Utf8("time-channel")),
        (11, V::U32(&f32_scalar)),
        (12, V::Utf8("urn:qualia:qac:pcm-preservation:1")),
        (13, V::U64(&in_rate_n)),
        (14, V::U64(&in_rate_d)),
    ] {
        push(&mut pcm_desc, &mut pdn, k, v);
    }
    if doc.original.is_some() {
        push(&mut pcm_desc, &mut pdn, 17, V::EntryRefs(&orig_src));
    }
    push(&mut pcm_desc, &mut pdn, 18, V::MetadataRefs(&pcm_ext));

    let fft_u32 = [doc.fft_size];
    let hop_u32 = [doc.hop];
    let zero_u32 = [0u32];
    let zero_i64 = [0i64];
    let transform = [
        f(1, V::Utf8("STFT-complex")),
        f(2, V::U32(&fft_u32)),
        f(3, V::U32(&hop_u32)),
        f(4, V::Utf8("hann-symmetric-authored-f32")),
        f(6, V::Utf8("forward-unscaled-inverse-1/N")),
        f(7, V::U32(&zero_u32)),
        f(9, V::Utf8("forward-negative-exponent")),
        f(10, V::I64(&zero_i64)),
        f(11, V::Utf8("none")),
        f(12, V::Utf8(doc.channel_order)),
        f(13, V::U32(&fft_u32)),
        f(14, V::U64(&in_rate_n)),
        f(15, V::U64(&in_rate_d)),
        f(16, V::F32(window)),
        f(17, V::Utf8("source-entry-first-sample")),
    ];

    let stft_prov_id = [6];
    let stft_tx_id = [9];
    let stft_ext = [7, 8];
    let cf32_scalar = [Scalar::ComplexF32 as u32];
    let frame_rate_n = [frame_rate.numerator];
    let frame_rate_d = [frame_rate.denominator];
    let pcm_src = [doc.pcm_entry_index()];
    let stft_desc = [
        f(1, V::MetadataRefs(&clock_id)),
        f(3, V::MetadataRefs(&stft_prov_id)),
        f(5, V::MetadataRefs(&stft_tx_id)),
        f(7, V::Utf8(stft_q)),
        f(8, V::Utf8(stft_u)),
        f(9, V::U32(&channels)),
        f(10, V::Utf8("frame-channel-bin")),
        f(11, V::U32(&cf32_scalar)),
        f(12, V::Utf8("urn:qualia:qac:stft-preservation:1")),
        f(13, V::U64(&frame_rate_n)),
        f(14, V::U64(&frame_rate_d)),
        f(17, V::EntryRefs(&pcm_src)),
        f(18, V::MetadataRefs(&stft_ext)),
    ];

    let opaque_role = [4u32];
    let orig_len = [doc.original.map_or(0, |s| s.len() as u64)];
    let orig_prov_id = [11];
    let orig_shape_id = [12];
    let one = [1u32];
    let byte_scalar = [Scalar::U8 as u32];
    let orig_prov = [f(1, V::U32(&opaque_role))];
    let orig_shape = [
        f(1, V::Utf8("urn:qualia:qac:opaque-original:1")),
        f(2, V::Utf8("1")),
        f(14, V::U64(&orig_len)),
        f(15, V::Utf8("byte")),
    ];
    let orig_desc = [
        f(3, V::MetadataRefs(&orig_prov_id)),
        f(7, V::Utf8("original_bytes")),
        f(8, V::Utf8("1")),
        f(9, V::U32(&one)),
        f(10, V::Utf8("byte")),
        f(11, V::U32(&byte_scalar)),
        f(12, V::Utf8("urn:qualia:qac:opaque-original:1")),
        f(18, V::MetadataRefs(&orig_shape_id)),
    ];

    let blocks = [
        b(1, 1, &clock),
        b(2, 4, &pcm_prov[..pcm_pn]),
        b(3, 205, &pcm_shape),
        b(4, 210, &pcm_encoding),
        b(5, 3, &pcm_desc[..pdn]),
        b(6, 4, &stft_prov[..stft_pn]),
        b(7, 205, &stft_shape),
        b(8, 210, &stft_encoding),
        b(9, 201, &transform),
        b(10, 3, &stft_desc),
        b(11, 4, &orig_prov),
        b(12, 205, &orig_shape),
        b(13, 3, &orig_desc),
    ];
    let count = if doc.original.is_some() { 13 } else { 10 };
    Ok(MetadataInput {
        blocks: &blocks[..count],
    }
    .encode_into(out, limits)?
    .len())
}
