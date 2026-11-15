use super::samples::finite;
use super::*;
use crate::audio::qac::metadata::{
    self as meta, OriginBasis, SignalRegistry, Transform, TransformKind,
};
use crate::q42::{
    field_envelope::{Block, Entry, Family, StructuralView},
    field_semantics::{self as sem, Context},
};
#[derive(Clone, Copy, Debug)]
pub struct SignalView<'a> {
    pub(super) payload: &'a [u8],
    pub(super) mode: u16,
    pub(super) scalar: Scalar,
    pub(super) channels: u32,
    pub(super) bins: u32,
    pub(super) records: u64,
    pub(super) stream_id: u32,
    pub(super) frame_id: u32,
    pub(super) timing: Option<Timing>,
    pub(super) transform: Option<Transform>,
    pub(super) window_coefficients_le: Option<&'a [u8]>,
    pub(super) channel_order: &'a str,
    pub(super) purpose: &'a str,
    pub(super) relation: &'a str,
    pub(super) quantity: &'a str,
    pub(super) units: &'a str,
    pub(super) units_per_stored_sample: f64,
    pub(super) stored_offset: f64,
}
fn extension<'a>(d: Block<'a>, schema: u16, ctx: &Context<'_, 'a>) -> Result<Block<'a>, Error> {
    let mut found = None;
    for raw in meta::get(d, 18)?.value().chunks_exact(4) {
        let block = ctx.block(u32::from_le_bytes(raw.try_into().unwrap()))?;
        if block.schema_type() == schema {
            if found.is_some() {
                return Err(Error::Profile);
            }
            found = Some(block);
        }
    }
    found.ok_or(Error::Profile)
}
fn labels(s: &str, n: u32, ctx: &Context<'_, '_>) -> Result<(), Error> {
    ctx.charge_work(
        (s.len() as u64)
            .checked_mul(u64::from(n) + 1)
            .ok_or(Error::Overflow)?,
    )?;
    let mut count = 0u32;
    for (i, label) in s.split(',').enumerate() {
        if label.is_empty() || label.trim() != label || s.split(',').take(i).any(|old| old == label)
        {
            return Err(Error::Profile);
        }
        count = count.checked_add(1).ok_or(Error::Overflow)?;
    }
    if count != n {
        return Err(Error::Shape);
    }
    Ok(())
}
fn inspect<'a>(
    index: u32,
    e: Entry<'a>,
    payload: &'a [u8],
    ctx: &Context<'_, 'a>,
) -> Result<SignalView<'a>, Error> {
    if e.revision() != 1 || ![0x100, 0x101, 0x104].contains(&e.mode()) {
        return Err(Error::Unsupported);
    }
    let d = ctx.require_schema(e.descriptor_id(), 3)?;
    for key in [9, 10, 11, 18] {
        if meta::get(d, key)?.flags() & 1 == 0 {
            return Err(Error::Profile);
        }
    }
    let model = meta::text(d, 12)?;
    let scalar = Scalar::from_code(meta::u32v(d, 11)?)?;
    let channels = meta::u32v(d, 9)?;
    if channels == 0 {
        return Err(Error::Shape);
    }
    let shape = extension(d, 205, ctx)?;
    if shape.revision() != 1 || meta::text(shape, 1)? != model || meta::text(shape, 2)? != "1" {
        return Err(Error::Profile);
    }
    let dimensions = meta::get(shape, 14)?.value();
    let n = dimensions.len() / 8;
    let dim = |i: usize| u64::from_le_bytes(dimensions[i * 8..i * 8 + 8].try_into().unwrap());
    let quantity = meta::text(d, 7)?;
    let units = meta::text(d, 8)?;
    let layout = meta::text(d, 10)?;
    let mut result = SignalView {
        payload,
        mode: e.mode(),
        scalar,
        channels,
        bins: 1,
        records: e.record_count(),
        stream_id: e.stream_id(),
        frame_id: e.frame_id(),
        timing: None,
        transform: None,
        window_coefficients_le: None,
        channel_order: "",
        purpose: "",
        relation: "",
        quantity,
        units,
        units_per_stored_sample: 1.0,
        stored_offset: 0.0,
    };
    if e.mode() == 0x104 {
        if model != "urn:qualia:qac:opaque-original:1"
            || quantity != "original_bytes"
            || units != "1"
            || scalar != Scalar::U8
            || channels != 1
            || layout != "byte"
            || n != 1
            || dim(0) != e.record_count()
            || e.flags() & 2 != 0
            || sem::field(d, 5).is_some()
            || sem::field(d, 6).is_some()
            || meta::text(shape, 15)? != "byte"
        {
            return Err(Error::Profile);
        }
    } else {
        if e.flags() & 2 == 0 || e.time_count() != e.record_count() {
            return Err(Error::Clock);
        }
        let clock = ctx.require_schema(meta::u32v(d, 1)?, 1)?;
        result.timing = Some(Timing {
            scale: meta::u32v(clock, 1)?,
            epoch_seconds: meta::u64v(clock, 2)? as i64,
            epoch_nanoseconds: meta::u32v(clock, 3)?,
            ticks: Rate::new(meta::u64v(clock, 4)?, meta::u64v(clock, 5)?)?,
            samples: Rate::new(meta::u64v(d, 13)?, meta::u64v(d, 14)?)?,
            start_tick: e.start_tick(),
        });
        let encoding = extension(d, 210, ctx)?;
        result.channel_order = meta::text(encoding, 1)?;
        labels(result.channel_order, channels, ctx)?;
        result.purpose = meta::text(encoding, 4)?;
        result.relation = meta::text(encoding, 5)?;
        result.units_per_stored_sample = meta::f64v(encoding, 2)?;
        result.stored_offset = meta::f64v(encoding, 3)?;
        match result.relation {
            "digital-unrelated-to-pressure" => {
                if units != "1"
                    || ![
                        "digital_sample",
                        "complex_digital_spectrum",
                        "magnitude_digital_spectrum",
                    ]
                    .contains(&quantity)
                {
                    return Err(Error::Unit);
                }
            }
            "raw-counts" => {
                if quantity != "raw_sample_count"
                    || units != "1"
                    || ![Scalar::I16, Scalar::I24, Scalar::I32].contains(&scalar)
                {
                    return Err(Error::Unit);
                }
            }
            "physical-base-units" => {
                if units != "Pa"
                    || ![
                        "pressure",
                        "complex_pressure_spectrum",
                        "magnitude_pressure_spectrum",
                    ]
                    .contains(&quantity)
                {
                    return Err(Error::Unit);
                }
            }
            _ => return Err(Error::Profile),
        }
        if sem::field(d, 6).is_some() && result.relation != "raw-counts" {
            let lineage = ctx.require_schema(meta::u32v(d, 6)?, 202)?;
            if meta::text(lineage, 2)? != units {
                return Err(Error::Unit);
            }
        }
        if n < 2 || dim(0) != e.record_count() || dim(1) != u64::from(channels) {
            return Err(Error::Shape);
        }
        if e.mode() == 0x100 {
            if n != 2
                || model != "urn:qualia:qac:pcm-preservation:1"
                || layout != "time-channel"
                || matches!(scalar, Scalar::ComplexF32 | Scalar::U8)
                || sem::field(d, 5).is_some()
                || !["digital_sample", "raw_sample_count", "pressure"].contains(&quantity)
            {
                return Err(Error::Profile);
            }
            if meta::text(shape, 15)? != "sample" {
                return Err(Error::Profile);
            }
        } else {
            if n != 3
                || model != "urn:qualia:qac:stft-preservation:1"
                || layout != "frame-channel-bin"
            {
                return Err(Error::Profile);
            }
            let transform_block = ctx.require_schema(meta::u32v(d, 5)?, 201)?;
            let t = Transform::from_block(transform_block)?;
            if dim(2) != u64::from(t.bins) {
                return Err(Error::Shape);
            }
            result.bins = t.bins;
            result.window_coefficients_le = sem::field(transform_block, 16).map(|f| f.value());
            match t.kind {
                TransformKind::Complex => {
                    if scalar != Scalar::ComplexF32
                        || result.stored_offset != 0.0
                        || !["complex_digital_spectrum", "complex_pressure_spectrum"]
                            .contains(&quantity)
                        || meta::text(shape, 15)? != "real,imag"
                    {
                        return Err(Error::Profile);
                    }
                }
                TransformKind::Magnitude => {
                    if scalar != Scalar::F32
                        || !["magnitude_digital_spectrum", "magnitude_pressure_spectrum"]
                            .contains(&quantity)
                        || meta::text(shape, 15)? != "magnitude"
                    {
                        return Err(Error::Profile);
                    }
                }
            }
            let timing = result.timing.unwrap();
            if timing.samples != t.input_rate.divided_by(t.hop)? {
                return Err(Error::Clock);
            }
            match t.origin_basis {
                OriginBasis::EpochSampleGrid => {
                    let left = i128::from(e.start_tick())
                        .checked_mul(timing.ticks.denominator.into())
                        .and_then(|x| x.checked_mul(t.input_rate.numerator.into()))
                        .ok_or(Error::Overflow)?;
                    let right = i128::from(t.origin_samples)
                        .checked_mul(t.input_rate.denominator.into())
                        .and_then(|x| x.checked_mul(timing.ticks.numerator.into()))
                        .ok_or(Error::Overflow)?;
                    if left != right {
                        return Err(Error::Clock);
                    }
                }
                OriginBasis::SourceEntryFirstSample => {
                    if t.origin_samples != 0 {
                        return Err(Error::Clock);
                    }
                    let sources = sem::field(d, 17).ok_or(Error::Profile)?;
                    if sources.flags() & 1 == 0
                        || sources.type_tag() != 9
                        || sources.element_count() != 1
                        || sources.value().len() != 4
                    {
                        return Err(Error::Profile);
                    }
                    let src_idx = u32::from_le_bytes(sources.value().try_into().unwrap());
                    if src_idx == index {
                        return Err(Error::Profile);
                    }
                    let src_entry = ctx.entry(src_idx)?;
                    if src_entry.mode() != 0x100
                        || src_entry.revision() != 1
                        || src_entry.flags() & 2 == 0
                    {
                        return Err(Error::Profile);
                    }
                    if src_entry.start_tick() != e.start_tick()
                        || u64::from(t.fft_size) > src_entry.record_count()
                    {
                        return Err(Error::Clock);
                    }
                    let covered = result
                        .records
                        .checked_sub(1)
                        .and_then(|r| r.checked_mul(u64::from(t.hop)))
                        .and_then(|n| n.checked_add(u64::from(t.fft_size)))
                        .ok_or(Error::Overflow)?;
                    if covered > src_entry.record_count() {
                        return Err(Error::Shape);
                    }
                    let src_d = ctx.require_schema(src_entry.descriptor_id(), 3)?;
                    if meta::u32v(src_d, 1)? != meta::u32v(d, 1)?
                        || meta::u32v(src_d, 9)? != channels
                        || meta::text(src_d, 8)? != units
                        || Rate::new(meta::u64v(src_d, 13)?, meta::u64v(src_d, 14)?)?
                            != t.input_rate
                    {
                        return Err(Error::Clock);
                    }
                    let src_enc = extension(src_d, 210, ctx)?;
                    if meta::text(src_enc, 1)? != result.channel_order
                        || meta::text(src_enc, 5)? != result.relation
                    {
                        return Err(Error::Profile);
                    }
                }
            }
            if meta::text(transform_block, 12)? != result.channel_order {
                return Err(Error::Clock);
            }
            result.transform = Some(t);
        }
        result.time_at(result.records - 1)?;
    }
    let stride = channels
        .checked_mul(result.bins)
        .and_then(|n| n.checked_mul(scalar.bytes()))
        .ok_or(Error::Overflow)?;
    if stride != e.stride()
        || result
            .records
            .checked_mul(stride.into())
            .ok_or(Error::Overflow)?
            != e.decoded_bytes()
    {
        return Err(Error::Shape);
    }
    Ok(result)
}
// Conservative rejection-only preflight, before shared validity/reference work can
// touch payloads. The structural index is already checked; all semantics follow.
fn preflight(view: &StructuralView<'_>, limits: Limits, work: &mut u64) -> Result<(), Error> {
    let mut samples = 0u64;
    let mut bytes = 0u64;
    for index in 1..=view.entry_count() {
        let e = view.entry(index)?;
        if ![0x100, 0x101, 0x104].contains(&e.mode()) {
            return Err(Error::Unsupported);
        }
        let mut descriptor = None;
        for b in view.blocks() {
            *work = work.checked_sub(1).ok_or(Error::Limit)?;
            if b.id() == e.descriptor_id() {
                descriptor = Some(b);
                break;
            }
        }
        let d = descriptor.ok_or(Error::Profile)?;
        if d.schema_type() != 3 || d.revision() != 1 {
            return Err(Error::Profile);
        }
        let field_scan_work = (d.fields().count() as u64)
            .checked_mul(2)
            .ok_or(Error::Overflow)?;
        *work = work.checked_sub(field_scan_work).ok_or(Error::Limit)?;
        let f = meta::get(d, 11)?;
        if f.type_tag() != 1 || f.element_count() != 1 || f.value().len() != 4 {
            return Err(Error::Scalar);
        }
        let width = u64::from(Scalar::from_code(meta::u32v(d, 11)?)?.bytes());
        if e.decoded_bytes() % width != 0 {
            return Err(Error::Shape);
        }
        samples = samples
            .checked_add(e.decoded_bytes() / width)
            .ok_or(Error::Overflow)?;
        bytes = bytes.checked_add(e.stored_bytes()).ok_or(Error::Overflow)?;
        if samples > limits.samples || bytes > limits.payload_bytes {
            return Err(Error::Limit);
        }
    }
    Ok(())
}
fn validate_payload(signal: SignalView<'_>) -> Result<(), Error> {
    finite(signal.scalar, signal.payload)?;
    if signal
        .transform
        .is_some_and(|t| t.kind == TransformKind::Magnitude)
        && signal
            .payload
            .chunks_exact(4)
            .any(|b| f32::from_le_bytes(b.try_into().unwrap()) < 0.0)
    {
        return Err(Error::Profile);
    }
    Ok(())
}
/// Check every supported entry, reference, shape and whole payload; return one
/// immutable signal view. Unknown profiles remain available through archival
/// storage. Scalar/byte budgets are checked before payload hashing or scanning.
pub fn admit<'a>(
    view: &StructuralView<'a>,
    selected: u32,
    scratch: &mut sem::Scratch<'_>,
    shared_limits: sem::Limits,
    limits: Limits,
) -> Result<SignalView<'a>, Error> {
    if view.family() != Family::QacV1Candidate {
        return Err(Error::Unsupported);
    }
    if selected == 0 || selected > view.entry_count() {
        return Err(Error::Index);
    }
    if view.entry_count() > limits.entries {
        return Err(Error::Limit);
    }
    let mut work = limits.work;
    preflight(view, limits, &mut work)?;
    let admission = sem::admit_shared(view, &SignalRegistry, scratch, shared_limits)?;
    let ctx = &admission.context;
    let mut samples = 0u64;
    let mut chosen = None;
    for index in 1..=view.entry_count() {
        let e = ctx.entry(index)?;
        // Validate typed descriptor/shape/stride before processing any numeric bytes.
        let mut signal = inspect(index, e, &[], ctx)?;
        samples = samples
            .checked_add(e.decoded_bytes() / u64::from(signal.scalar.bytes()))
            .ok_or(Error::Overflow)?;
        if samples > limits.samples {
            return Err(Error::Limit);
        }
        work = work
            .checked_sub(e.stored_bytes().checked_mul(2).ok_or(Error::Overflow)?)
            .ok_or(Error::Limit)?;
        ctx.charge_work(e.stored_bytes())?;
        signal.payload = view.verify_chunk(index)?;
        validate_payload(signal)?;
        if index == selected {
            chosen = Some(signal)
        }
    }
    chosen.ok_or(Error::Index)
}
