//! Mixed-domain QAC v1 (`Family::QacV1Candidate`) admission bridge composing
//! `audio::qac::signal` with `sensing::acquisition::storage` Schemas 6 and 7.
use crate::audio::qac::metadata::{self as meta, SignalRegistry};
use crate::audio::qac::signal::{samples::finite, Error, Limits, Rate, Scalar, SignalView, Timing};
use crate::q42::{
    field_envelope::{self as env, Block, Entry, Family, StructuralView},
    field_semantics::{self as sem, Context, DomainRegistry, FieldRule},
};
use crate::sensing::acquisition::{self as a, storage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeError {
    Envelope(env::Error), Semantic(sem::Error), Signal(Error), Acquisition(storage::Error),
    AcquisitionCore(a::Error), UnsupportedFamily, Index, Limit, Profile, MismatchedSourceDigest,
    MissingSourceEntry, InvalidSourceMode, Reference, Overflow,
}

impl From<env::Error> for BridgeError { fn from(e: env::Error) -> Self { Self::Envelope(e) } }
impl From<sem::Error> for BridgeError { fn from(e: sem::Error) -> Self { Self::Semantic(e) } }
impl From<Error> for BridgeError { fn from(e: Error) -> Self { Self::Signal(e) } }
impl From<storage::Error> for BridgeError { fn from(e: storage::Error) -> Self { Self::Acquisition(e) } }
impl From<a::Error> for BridgeError { fn from(e: a::Error) -> Self { Self::AcquisitionCore(e) } }

#[derive(Clone, Copy, Debug, Default)]
pub struct QacAcquisitionRegistry {
    pub acquisition_limits: storage::Limits,
}

impl DomainRegistry for QacAcquisitionRegistry {
    fn rules(&self, s: u16, v: u16) -> Option<&'static [FieldRule]> {
        if v != 1 { return None; }
        match s {
            storage::ACQUISITION_SCHEMA | storage::DERIVATION_SCHEMA => {
                storage::AcquisitionRegistry { limits: self.acquisition_limits }.rules(s, v)
            }
            _ => SignalRegistry.rules(s, v),
        }
    }
    fn validate_block(&self, b: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), sem::Error> {
        match b.schema_type() {
            storage::ACQUISITION_SCHEMA | storage::DERIVATION_SCHEMA => {
                storage::AcquisitionRegistry { limits: self.acquisition_limits }.validate_block(b, ctx)
            }
            _ => SignalRegistry.validate_block(b, ctx),
        }
    }
    fn descriptor_target_schema(&self, k: u16) -> Option<u16> { SignalRegistry.descriptor_target_schema(k) }
    fn validate_units(&self, u: &str, ctx: &Context<'_, '_>) -> Result<(), sem::Error> { SignalRegistry.validate_units(u, ctx) }
}

#[derive(Clone, Copy, Debug)]
pub struct AdmittedAcquisitionEntry<'a> {
    pub entry_index: u32, pub original: a::Original<'a>, pub region: a::Region<'a>,
    pub kind: u32, pub descriptor_id: u32, pub view_block_id: u32, pub lineage_block_id: Option<u32>,
}

impl<'a> AdmittedAcquisitionEntry<'a> {
    pub fn bytes(&self) -> &'a [u8] { self.region.bytes() }
    pub fn original_bytes(&self) -> &'a [u8] { self.original.bytes() }
    pub fn digest(&self) -> [u8; 32] { self.original.digest() }
}

#[derive(Clone, Copy, Debug)]
pub struct QacSignalWithAcquisitionSource<'a> {
    pub signal: SignalView<'a>, pub acquisition_source: AdmittedAcquisitionEntry<'a>,
}

#[derive(Clone, Copy, Debug)]
pub enum QacMixedEntry<'a> {
    Signal(SignalView<'a>), Acquisition(AdmittedAcquisitionEntry<'a>),
}

fn extension<'a>(d: Block<'a>, schema: u16, ctx: &Context<'_, 'a>) -> Result<Block<'a>, BridgeError> {
    let mut found = None;
    for raw in meta::get(d, 18)?.value().chunks_exact(4) {
        let block = ctx.block(u32::from_le_bytes(raw.try_into().unwrap()))?;
        if block.schema_type() == schema {
            if found.is_some() { return Err(BridgeError::Signal(Error::Profile)); }
            found = Some(block);
        }
    }
    found.ok_or(BridgeError::Signal(Error::Profile))
}

fn labels(s: &str, n: u32, ctx: &Context<'_, '_>) -> Result<(), BridgeError> {
    ctx.charge_work((s.len() as u64).checked_mul(u64::from(n) + 1).ok_or(BridgeError::Overflow)?)?;
    let mut count = 0u32;
    for (i, label) in s.split(',').enumerate() {
        if label.is_empty() || label.trim() != label || s.split(',').take(i).any(|old| old == label) {
            return Err(BridgeError::Signal(Error::Profile));
        }
        count = count.checked_add(1).ok_or(BridgeError::Overflow)?;
    }
    if count != n { return Err(BridgeError::Signal(Error::Shape)); }
    Ok(())
}

fn inspect_signal_entry<'a>(
    index: u32, e: Entry<'a>, ctx: &Context<'_, 'a>, view: &StructuralView<'a>,
) -> Result<SignalView<'a>, BridgeError> {
    if e.revision() != 1 || ![0x100, 0x101, 0x104].contains(&e.mode()) {
        return Err(BridgeError::Signal(Error::Unsupported));
    }
    let d = ctx.require_schema(e.descriptor_id(), 3)?;
    for key in [9, 10, 11, 18] {
        if meta::get(d, key)?.flags() & 1 == 0 { return Err(BridgeError::Signal(Error::Profile)); }
    }
    let model = meta::text(d, 12)?;
    let scalar = Scalar::from_code(meta::u32v(d, 11)?)?;
    let channels = meta::u32v(d, 9)?;
    if channels == 0 { return Err(BridgeError::Signal(Error::Shape)); }
    let shape = extension(d, 205, ctx)?;
    if shape.revision() != 1 || meta::text(shape, 1)? != model || meta::text(shape, 2)? != "1" {
        return Err(BridgeError::Signal(Error::Profile));
    }
    let dimensions = meta::get(shape, 14)?.value();
    let n = dimensions.len() / 8;
    let dim = |i: usize| u64::from_le_bytes(dimensions[i * 8..i * 8 + 8].try_into().unwrap());
    let quantity = meta::text(d, 7)?;
    let units = meta::text(d, 8)?;
    let layout = meta::text(d, 10)?;
    let mut timing = None;
    let mut channel_order = "";
    let mut purpose = "";
    let mut relation = "";
    let mut units_per_stored_sample = 1.0;
    let mut stored_offset = 0.0;
    let bins = 1;

    if e.mode() == 0x104 {
        if model != "urn:qualia:qac:opaque-original:1" || quantity != "original_bytes" || units != "1"
            || scalar != Scalar::U8 || channels != 1 || layout != "byte" || n != 1
            || dim(0) != e.record_count() || e.flags() & 2 != 0 || sem::field(d, 5).is_some()
            || sem::field(d, 6).is_some() || meta::text(shape, 15)? != "byte"
        {
            return Err(BridgeError::Signal(Error::Profile));
        }
    } else {
        if e.flags() & 2 == 0 || e.time_count() != e.record_count() {
            return Err(BridgeError::Signal(Error::Clock));
        }
        let clock = ctx.require_schema(meta::u32v(d, 1)?, 1)?;
        timing = Some(Timing {
            scale: meta::u32v(clock, 1)?, epoch_seconds: meta::u64v(clock, 2)? as i64, epoch_nanoseconds: meta::u32v(clock, 3)?,
            ticks: Rate::new(meta::u64v(clock, 4)?, meta::u64v(clock, 5)?)?, samples: Rate::new(meta::u64v(d, 13)?, meta::u64v(d, 14)?)?,
            start_tick: e.start_tick(),
        });
        let encoding = extension(d, 210, ctx)?;
        channel_order = meta::text(encoding, 1)?;
        labels(channel_order, channels, ctx)?;
        purpose = meta::text(encoding, 4)?; relation = meta::text(encoding, 5)?;
        units_per_stored_sample = meta::f64v(encoding, 2)?; stored_offset = meta::f64v(encoding, 3)?;
        match relation {
            "digital-unrelated-to-pressure" => if units != "1" || !["digital_sample", "complex_digital_spectrum", "magnitude_digital_spectrum"].contains(&quantity) { return Err(BridgeError::Signal(Error::Unit)); },
            "raw-counts" => if quantity != "raw_sample_count" || units != "1" || ![Scalar::I16, Scalar::I24, Scalar::I32].contains(&scalar) { return Err(BridgeError::Signal(Error::Unit)); },
            "physical-base-units" => if units != "Pa" || !["pressure", "complex_pressure_spectrum", "magnitude_pressure_spectrum"].contains(&quantity) { return Err(BridgeError::Signal(Error::Unit)); },
            _ => return Err(BridgeError::Signal(Error::Profile)),
        }
        if n < 2 || dim(0) != e.record_count() || dim(1) != u64::from(channels) {
            return Err(BridgeError::Signal(Error::Shape));
        }
        if e.mode() == 0x100 && (n != 2 || model != "urn:qualia:qac:pcm-preservation:1" || layout != "time-channel"
            || matches!(scalar, Scalar::ComplexF32 | Scalar::U8) || sem::field(d, 5).is_some()
            || !["digital_sample", "raw_sample_count", "pressure"].contains(&quantity) || meta::text(shape, 15)? != "sample")
        {
            return Err(BridgeError::Signal(Error::Profile));
        }
        timing.unwrap().time_at(e.record_count().checked_sub(1).ok_or(BridgeError::Signal(Error::Shape))?)?;
    }
    let stride = channels.checked_mul(bins).and_then(|x| x.checked_mul(scalar.bytes())).ok_or(BridgeError::Overflow)?;
    if stride != e.stride() || e.record_count().checked_mul(stride.into()).ok_or(BridgeError::Overflow)? != e.decoded_bytes() {
        return Err(BridgeError::Signal(Error::Shape));
    }
    let payload = view.verify_chunk(index)?;
    finite(scalar, payload)?;
    Ok(SignalView {
        payload, mode: e.mode(), scalar, channels, bins, records: e.record_count(),
        stream_id: e.stream_id(), frame_id: e.frame_id(), timing, transform: None,
        window_coefficients_le: None, channel_order, purpose, relation, quantity,
        units, units_per_stored_sample, stored_offset,
    })
}

fn inspect_acquisition_entry<'a>(
    index: u32, e: Entry<'a>, ctx: &Context<'_, 'a>, view: &StructuralView<'a>, acq_limits: storage::Limits,
) -> Result<AdmittedAcquisitionEntry<'a>, BridgeError> {
    if e.mode() != 0x104 || e.revision() != 1 || e.flags() != 0 || e.stride() != 1 || e.frame_id() != 0 {
        return Err(BridgeError::InvalidSourceMode);
    }
    let d = ctx.require_schema(e.descriptor_id(), 3)?;
    if meta::text(d, 7)? != "original_bytes" || meta::text(d, 8)? != "1" || meta::text(d, 10)? != "byte"
        || meta::u32v(d, 9)? != 1 || meta::u32v(d, 11)? != 7
    {
        return Err(BridgeError::Profile);
    }
    let model = meta::text(d, 12)?;
    if model != storage::ORIGINAL_MODEL && model != "urn:qualia:qac:opaque-original:1" {
        return Err(BridgeError::Profile);
    }
    let ext_field = meta::get(d, 18)?;
    let mut view_block_id = None;
    let mut lineage_block_id = None;
    for raw in ext_field.value().chunks_exact(4) {
        let bid = u32::from_le_bytes(raw.try_into().unwrap());
        let b = ctx.block(bid)?;
        match b.schema_type() {
            storage::ACQUISITION_SCHEMA => {
                if view_block_id.is_some() { return Err(BridgeError::Profile); }
                view_block_id = Some(bid);
            }
            storage::DERIVATION_SCHEMA => {
                if lineage_block_id.is_some() { return Err(BridgeError::Profile); }
                lineage_block_id = Some(bid);
            }
            _ => {}
        }
    }
    let b6_id = view_block_id.ok_or(BridgeError::Profile)?;
    let b6 = ctx.block(b6_id)?;
    let kind = meta::u32v(b6, 1)?;
    if meta::u32v(b6, 2)? != index { return Err(BridgeError::Reference); }
    if meta::get(b6, 3)?.value() != e.digest() { return Err(BridgeError::MismatchedSourceDigest); }
    let offset = usize::try_from(meta::u64v(b6, 4)?).map_err(|_| BridgeError::Overflow)?;
    let bytes = usize::try_from(meta::u64v(b6, 5)?).map_err(|_| BridgeError::Overflow)?;
    if offset.checked_add(bytes).ok_or(BridgeError::Overflow)? as u64 > e.decoded_bytes() {
        return Err(BridgeError::Signal(Error::Shape));
    }
    if let Some(b7_id) = lineage_block_id {
        let b7 = ctx.block(b7_id)?;
        if meta::u32v(b7, 1)? != index { return Err(BridgeError::Reference); }
        if meta::get(b7, 2)?.value() != e.digest() { return Err(BridgeError::MismatchedSourceDigest); }
    }
    let payload = view.verify_chunk(index)?;
    let original = a::Original::admit(payload, usize::try_from(acq_limits.source_bytes).map_err(|_| BridgeError::Overflow)?)?;
    let region = original.region(offset, bytes).map_err(BridgeError::AcquisitionCore)?;
    Ok(AdmittedAcquisitionEntry {
        entry_index: index, original, region, kind, descriptor_id: e.descriptor_id(),
        view_block_id: b6_id, lineage_block_id,
    })
}

fn verify_source_lineage<'a>(
    sig_entry_idx: u32, sig_desc: Block<'a>, ctx: &Context<'_, 'a>,
    view: &StructuralView<'a>, acq_limits: storage::Limits,
) -> Result<AdmittedAcquisitionEntry<'a>, BridgeError> {
    let sources = sem::field(sig_desc, 17).ok_or(BridgeError::MissingSourceEntry)?;
    if sources.flags() & 1 == 0 || sources.type_tag() != 9 || sources.element_count() != 1 || sources.value().len() != 4 {
        return Err(BridgeError::MissingSourceEntry);
    }
    let src_idx = u32::from_le_bytes(sources.value().try_into().unwrap());
    if src_idx == 0 || src_idx == sig_entry_idx || src_idx > view.entry_count() {
        return Err(BridgeError::Reference);
    }
    let src_e = ctx.entry(src_idx)?;
    let acq_entry = inspect_acquisition_entry(src_idx, src_e, ctx, view, acq_limits)?;
    let prov_id = meta::u32v(sig_desc, 3)?;
    let prov_block = ctx.require_schema(prov_id, 4)?;
    if let Some(f2) = sem::field(prov_block, 2) {
        if f2.value() != acq_entry.original.digest() {
            return Err(BridgeError::MismatchedSourceDigest);
        }
    }
    Ok(acq_entry)
}

pub fn admit_qac_mixed_entry<'a>(
    view: &StructuralView<'a>, selected: u32, scratch: &mut sem::Scratch<'_>,
    shared_limits: sem::Limits, signal_limits: Limits, acq_limits: storage::Limits,
) -> Result<QacMixedEntry<'a>, BridgeError> {
    if view.family() != Family::QacV1Candidate { return Err(BridgeError::UnsupportedFamily); }
    if selected == 0 || selected > view.entry_count() { return Err(BridgeError::Index); }
    if view.entry_count() > signal_limits.entries || view.entry_count() > acq_limits.entries {
        return Err(BridgeError::Limit);
    }
    let registry = QacAcquisitionRegistry { acquisition_limits: acq_limits };
    let admission = sem::admit_shared(view, &registry, scratch, shared_limits)?;
    let ctx = &admission.context;
    let e = ctx.entry(selected)?;
    match e.mode() {
        0x100 | 0x101 => {
            let sig_view = inspect_signal_entry(selected, e, ctx, view)?;
            let d = ctx.require_schema(e.descriptor_id(), 3)?;
            if sem::field(d, 17).is_some() {
                let _ = verify_source_lineage(selected, d, ctx, view, acq_limits)?;
            }
            Ok(QacMixedEntry::Signal(sig_view))
        }
        0x104 => {
            let d = ctx.require_schema(e.descriptor_id(), 3)?;
            let is_acq = meta::get(d, 18).map_or(false, |ext| {
                ext.value().chunks_exact(4).any(|raw| {
                    ctx.block(u32::from_le_bytes(raw.try_into().unwrap()))
                        .map_or(false, |b| b.schema_type() == storage::ACQUISITION_SCHEMA)
                })
            });
            if is_acq {
                let acq = inspect_acquisition_entry(selected, e, ctx, view, acq_limits)?;
                Ok(QacMixedEntry::Acquisition(acq))
            } else {
                let sig = inspect_signal_entry(selected, e, ctx, view)?;
                Ok(QacMixedEntry::Signal(sig))
            }
        }
        _ => Err(BridgeError::InvalidSourceMode),
    }
}

pub fn admit_qac_signal_with_acquisition_source<'a>(
    view: &StructuralView<'a>, signal_entry_idx: u32, scratch: &mut sem::Scratch<'_>,
    shared_limits: sem::Limits, signal_limits: Limits, acq_limits: storage::Limits,
) -> Result<QacSignalWithAcquisitionSource<'a>, BridgeError> {
    if view.family() != Family::QacV1Candidate { return Err(BridgeError::UnsupportedFamily); }
    if signal_entry_idx == 0 || signal_entry_idx > view.entry_count() { return Err(BridgeError::Index); }
    if view.entry_count() > signal_limits.entries || view.entry_count() > acq_limits.entries {
        return Err(BridgeError::Limit);
    }
    let registry = QacAcquisitionRegistry { acquisition_limits: acq_limits };
    let admission = sem::admit_shared(view, &registry, scratch, shared_limits)?;
    let ctx = &admission.context;
    let e = ctx.entry(signal_entry_idx)?;
    if ![0x100, 0x101].contains(&e.mode()) { return Err(BridgeError::InvalidSourceMode); }
    let sig_view = inspect_signal_entry(signal_entry_idx, e, ctx, view)?;
    let d = ctx.require_schema(e.descriptor_id(), 3)?;
    let acq_source = verify_source_lineage(signal_entry_idx, d, ctx, view, acq_limits)?;
    Ok(QacSignalWithAcquisitionSource { signal: sig_view, acquisition_source: acq_source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::test_support::*;
    use crate::q42::field_envelope::{EntryInput, EnvelopeInput};

    const RAW_BYTES: &[u8] = b"RAW_ACQ_SENSOR_STREAM_01";
    const PCM_BYTES: &[u8] = &[0, 0, 128, 63, 0, 0, 0, 64];

    fn build_fixture(
        family: Family, corrupt_digest: bool, corrupt_schema6: bool, include_schema7: bool, corrupt_schema7: bool,
    ) -> Vec<u8> {
        let raw_hash = *blake3::hash(RAW_BYTES).as_bytes();
        let prov_hash = if corrupt_digest { [0xee; 32] } else { raw_hash };
        let s6_hash = if corrupt_schema6 { [0xff; 32] } else { raw_hash };

        let mut bs = vec![
            block(1, 1, vec![u32f(1, 0), f(2, 3, 0i64.to_le_bytes().to_vec()), u32f(3, 0), u64f(4, 48000), u64f(5, 1)]),
            block(2, 4, vec![u32f(1, 2), f(2, 7, prov_hash.to_vec()), strf(3, "test-pcm-decoder"), strf(7, "raw-acquisition-derive")]),
            block(3, 205, vec![
                strf(1, "urn:qualia:qac:pcm-preservation:1"), strf(2, "1"),
                f(14, 2, [2u64, 1u64].iter().flat_map(|x| x.to_le_bytes()).collect()), strf(15, "sample"),
            ]),
            block(4, 210, vec![
                strf(1, "channel-0"), f(2, 5, 1.0f64.to_le_bytes().to_vec()), f(3, 5, 0.0f64.to_le_bytes().to_vec()),
                strf(4, "audio"), strf(5, "digital-unrelated-to-pressure"),
            ]),
            block(5, 3, vec![
                ref_f(1, 1), ref_f(3, 2), strf(7, "digital_sample"), strf(8, "1"), u32f(9, 1),
                strf(10, "time-channel"), u32f(11, 1), strf(12, "urn:qualia:qac:pcm-preservation:1"),
                u64f(13, 48000), u64f(14, 1), f(17, 9, 1u32.to_le_bytes().to_vec()),
                f(18, 8, [3u32.to_le_bytes(), 4u32.to_le_bytes()].concat()),
            ]),
            block(11, 4, vec![u32f(1, 4)]),
            block(12, 6, vec![
                u32f(1, 1), f(2, 9, 1u32.to_le_bytes().to_vec()), f(3, 7, s6_hash.to_vec()),
                u64f(4, 0), u64f(5, 24), u32f(8, 0), u32f(10, 7), u32f(11, 1), u32f(12, 0), u32f(13, 1),
                f(14, 1, 24u32.to_le_bytes().to_vec()), f(15, 2, 1u64.to_le_bytes().to_vec()),
                f(16, 1, 0u32.to_le_bytes().to_vec()), f(17, 2, 0u64.to_le_bytes().to_vec()),
                u32f(32, 0),
            ]),
        ];

        let mut desc10_ext = vec![12u32.to_le_bytes()];
        if include_schema7 {
            let s7_out = if corrupt_schema7 { 99u32 } else { 1u32 };
            bs.push(block(13, 7, vec![
                f(1, 9, s7_out.to_le_bytes().to_vec()), f(2, 7, raw_hash.to_vec()),
                f(3, 7, [1u8; 32].to_vec()), u64f(4, 10), f(5, 7, [2u8; 32].to_vec()), u64f(6, 20),
                f(7, 9, 1u32.to_le_bytes().to_vec()), f(8, 7, raw_hash.to_vec()), f(9, 1, 0u32.to_le_bytes().to_vec()),
            ]));
            desc10_ext.push(13u32.to_le_bytes());
        }

        bs.push(block(10, 3, vec![
            ref_f(3, 11), strf(7, "original_bytes"), strf(8, "1"), u32f(9, 1), strf(10, "byte"),
            u32f(11, 7), strf(12, storage::ORIGINAL_MODEL), f(18, 8, desc10_ext.concat()),
        ]));

        for b in &mut bs {
            for fld in &mut b.fields {
                let width = match fld.tag { 1 | 4 | 8 | 9 | 11 => 4, 2 | 3 | 5 => 8, 10 => 2, _ => 1 };
                fld.count = (fld.bytes.len() / width) as u32;
            }
        }

        let meta_bytes = metadata(&bs);
        let entries = [
            EntryInput {
                mode: 0x104, revision: 1, flags: 0, stream_id: 0, frame_id: 0, start_tick: 0,
                time_count: 0, spatial_key: 0, record_count: RAW_BYTES.len() as u64, stride: 1,
                codec: 0, descriptor_id: 10, payload: RAW_BYTES,
            },
            EntryInput {
                mode: 0x100, revision: 1, flags: 2, stream_id: 1, frame_id: 0, start_tick: 0,
                time_count: 2, spatial_key: 0, record_count: 2, stride: 4, codec: 0,
                descriptor_id: 5, payload: PCM_BYTES,
            },
        ];

        let env_input = EnvelopeInput { family, flags: 0, metadata: &meta_bytes, entries: &entries };
        let mut buf = vec![0u8; 16384];
        let n = env_input.encode_into(&mut buf, env::Limits::default()).unwrap().len();
        buf.truncate(n);
        buf
    }

    #[test]
    fn mixed_qac_admission_and_lineage_binding_succeeds() {
        let data = build_fixture(Family::QacV1Candidate, false, false, false, false);
        let view = StructuralView::open(&data, Family::QacV1Candidate, env::Limits::default()).unwrap();
        let (mut m, mut e) = ([sem::NodeState::default(); 64], [sem::NodeState::default(); 16]);
        let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
        let result = admit_qac_signal_with_acquisition_source(
            &view, 2, &mut scratch, sem::Limits::default(), Limits::default(), storage::Limits::default(),
        ).unwrap();
        assert_eq!(result.signal.mode(), 0x100);
        assert_eq!(result.signal.records(), 2);
        assert_eq!(result.acquisition_source.entry_index, 1);
        assert_eq!(result.acquisition_source.original_bytes(), RAW_BYTES);
        assert_eq!(result.acquisition_source.digest(), *blake3::hash(RAW_BYTES).as_bytes());
        let mut scratch2 = sem::Scratch { metadata: &mut m, entries: &mut e };
        let entry1 = admit_qac_mixed_entry(
            &view, 1, &mut scratch2, sem::Limits::default(), Limits::default(), storage::Limits::default(),
        ).unwrap();
        let QacMixedEntry::Acquisition(acq) = entry1 else { panic!() };
        assert_eq!(acq.bytes(), RAW_BYTES);
        let mut scratch3 = sem::Scratch { metadata: &mut m, entries: &mut e };
        let entry2 = admit_qac_mixed_entry(
            &view, 2, &mut scratch3, sem::Limits::default(), Limits::default(), storage::Limits::default(),
        ).unwrap();
        let QacMixedEntry::Signal(sig) = entry2 else { panic!() };
        assert_eq!(sig.records(), 2);
    }

    #[test]
    fn reject_wrong_family() {
        let data = build_fixture(Family::EmfV4Candidate, false, false, false, false);
        let view = StructuralView::open(&data, Family::EmfV4Candidate, env::Limits::default()).unwrap();
        let (mut m, mut e) = ([sem::NodeState::default(); 64], [sem::NodeState::default(); 16]);
        let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
        let err = admit_qac_signal_with_acquisition_source(
            &view, 2, &mut scratch, sem::Limits::default(), Limits::default(), storage::Limits::default(),
        ).unwrap_err();
        assert_eq!(err, BridgeError::UnsupportedFamily);
    }

    #[test]
    fn reject_mismatched_source_digest() {
        let data = build_fixture(Family::QacV1Candidate, true, false, false, false);
        let view = StructuralView::open(&data, Family::QacV1Candidate, env::Limits::default()).unwrap();
        let (mut m, mut e) = ([sem::NodeState::default(); 64], [sem::NodeState::default(); 16]);
        let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
        let err = admit_qac_signal_with_acquisition_source(
            &view, 2, &mut scratch, sem::Limits::default(), Limits::default(), storage::Limits::default(),
        ).unwrap_err();
        assert_eq!(err, BridgeError::MismatchedSourceDigest);
    }

    #[test]
    fn reject_malformed_schema6_payload() {
        let data = build_fixture(Family::QacV1Candidate, false, true, false, false);
        let view = StructuralView::open(&data, Family::QacV1Candidate, env::Limits::default()).unwrap();
        let (mut m, mut e) = ([sem::NodeState::default(); 64], [sem::NodeState::default(); 16]);
        let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
        let res = admit_qac_signal_with_acquisition_source(
            &view, 2, &mut scratch, sem::Limits::default(), Limits::default(), storage::Limits::default(),
        );
        assert!(res.is_err());
    }

    #[test]
    fn reject_malformed_schema7_payload() {
        let data = build_fixture(Family::QacV1Candidate, false, false, true, true);
        let view = StructuralView::open(&data, Family::QacV1Candidate, env::Limits::default()).unwrap();
        let (mut m, mut e) = ([sem::NodeState::default(); 64], [sem::NodeState::default(); 16]);
        let mut scratch = sem::Scratch { metadata: &mut m, entries: &mut e };
        let res = admit_qac_signal_with_acquisition_source(
            &view, 2, &mut scratch, sem::Limits::default(), Limits::default(), storage::Limits::default(),
        );
        assert!(res.is_err());
    }
}
