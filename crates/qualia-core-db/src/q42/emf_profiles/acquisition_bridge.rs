//! Mixed-domain EMF v4 and raw acquisition storage composition & admission bridge.
//!
//! Composes `ProfileRegistry` (modes `0x0100..=0x0107`, companion schemas `100..=107`, `132..=137`)
//! with `AcquisitionRegistry` (mode `0x0104` opaque-original chunks with Schemas 6/7), closing R03 Gap #2.

use super::super::field_envelope::{Block, Entry, Family, StructuralView};
use super::super::field_semantics::{self as semantics, Context, DomainRegistry, FieldRule, Scratch};
use super::registry::ProfileRegistry;
use super::{field, required, scalar32, scalar64, shared_error, text, u32_at, Error, Limits, ProfileChunk};
use crate::sensing::acquisition::storage::AcquisitionRegistry;

/// Mixed-domain registry composing EMF scientific profiles with raw acquisition storage.
#[derive(Clone, Copy, Debug)]
pub struct EmfAcquisitionRegistry {
    pub profiles: ProfileRegistry,
    pub acquisition: AcquisitionRegistry,
}

impl Default for EmfAcquisitionRegistry {
    fn default() -> Self {
        Self {
            profiles: ProfileRegistry,
            acquisition: AcquisitionRegistry {
                limits: crate::sensing::acquisition::storage::Limits::default(),
            },
        }
    }
}

impl EmfAcquisitionRegistry {
    pub const fn new(limits: crate::sensing::acquisition::storage::Limits) -> Self {
        Self {
            profiles: ProfileRegistry,
            acquisition: AcquisitionRegistry { limits },
        }
    }
}

impl DomainRegistry for EmfAcquisitionRegistry {
    fn rules(&self, schema: u16, revision: u16) -> Option<&'static [FieldRule]> {
        if schema == 6 || schema == 7 { self.acquisition.rules(schema, revision) } else { self.profiles.rules(schema, revision) }
    }
    fn descriptor_target_schema(&self, key: u16) -> Option<u16> { self.profiles.descriptor_target_schema(key) }
    fn validate_block(&self, block: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), semantics::Error> {
        if block.schema_type() == 6 || block.schema_type() == 7 { self.acquisition.validate_block(block, ctx) } else { self.profiles.validate_block(block, ctx) }
    }
    fn validate_units(&self, units: &str, ctx: &Context<'_, '_>) -> Result<(), semantics::Error> { self.profiles.validate_units(units, ctx) }
    fn validate_grid_frame(&self, frame: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), semantics::Error> { self.profiles.validate_grid_frame(frame, ctx) }
    fn validate_spatial_entry(&self, entry: Entry<'_>, frame: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), semantics::Error> { self.profiles.validate_spatial_entry(entry, frame, ctx) }
    fn validate_frame_profile(&self, frame: Block<'_>, ctx: &Context<'_, '_>) -> Result<(), semantics::Error> { self.profiles.validate_frame_profile(frame, ctx) }
}

/// A verified raw acquisition original entry (Mode `0x0104`, Schemas 6/7).
#[derive(Clone, Copy, Debug)]
pub struct AcquisitionOriginalEntry<'a> {
    pub entry_index: u32,
    pub entry: Entry<'a>,
    pub descriptor_id: u32,
    pub view_block: Block<'a>,
    pub lineage_block: Option<Block<'a>>,
    pub payload: &'a [u8],
}

impl<'a> AcquisitionOriginalEntry<'a> {
    pub fn entry_index(&self) -> u32 { self.entry_index }
    pub fn payload(&self) -> &'a [u8] { self.payload }
    pub fn view_block(&self) -> Block<'a> { self.view_block }
    pub fn lineage_block(&self) -> Option<Block<'a>> { self.lineage_block }
    pub fn view_kind(&self) -> Result<u32, Error> { scalar32(self.view_block, 1) }
    pub fn payload_offset(&self) -> Result<u64, Error> { scalar64(self.view_block, 4) }
    pub fn payload_bytes(&self) -> Result<u64, Error> { scalar64(self.view_block, 5) }
    pub fn subrange(&self) -> Result<&'a [u8], Error> {
        let offset = usize::try_from(self.payload_offset()?).map_err(|_| Error::Overflow)?;
        let bytes = usize::try_from(self.payload_bytes()?).map_err(|_| Error::Overflow)?;
        let end = offset.checked_add(bytes).ok_or(Error::Overflow)?;
        if end > self.payload.len() { return Err(Error::Shape); }
        Ok(&self.payload[offset..end])
    }
}

/// Admitted entry from a mixed-domain container: either typed EMF or raw acquisition original.
#[derive(Clone, Copy, Debug)]
pub enum AdmittedMixedEntry<'a> {
    Emf(ProfileChunk<'a>),
    Acquisition(AcquisitionOriginalEntry<'a>),
}

impl<'a> AdmittedMixedEntry<'a> {
    pub fn entry_index(&self) -> u32 {
        match self { Self::Emf(c) => c.entry_index(), Self::Acquisition(a) => a.entry_index }
    }
    pub fn as_emf(&self) -> Option<&ProfileChunk<'a>> {
        match self { Self::Emf(c) => Some(c), Self::Acquisition(_) => None }
    }
    pub fn as_acquisition(&self) -> Option<&AcquisitionOriginalEntry<'a>> {
        match self { Self::Emf(_) => None, Self::Acquisition(a) => Some(a) }
    }
}

/// Mixed-domain container reader admitting both EMF profiles and acquisition originals.
pub struct EmfMixedReader<'v, 'a> {
    view: &'v StructuralView<'a>,
    admission: semantics::SharedAdmission<'v, 'a>,
    registry: EmfAcquisitionRegistry,
    limits: Limits,
    payload_work: u64,
}

impl<'v, 'a> EmfMixedReader<'v, 'a> {
    pub fn view(&self) -> &'v StructuralView<'a> { self.view }
    pub fn entry_count(&self) -> u32 { self.view.entry_count() }
    pub fn remaining_payload_work(&self) -> u64 { self.payload_work }
    pub fn remaining_shared_work(&self) -> u64 { self.admission.context.remaining_work() }
    pub fn context(&self) -> &Context<'v, 'a> { &self.admission.context }
    pub fn registry(&self) -> &EmfAcquisitionRegistry { &self.registry }
    pub fn chunk(&mut self, entry_index: u32) -> Result<AdmittedMixedEntry<'a>, Error> {
        let entry = self.view.entry(entry_index).map_err(Error::Storage)?;
        let cost = entry.stored_bytes().checked_mul(3).ok_or(Error::Overflow)?;
        self.payload_work = self.payload_work.checked_sub(cost).ok_or(Error::Limit)?;
        admit_emf_or_acquisition_entry(self.view, &self.admission.context, entry_index, self.limits)
    }
}

/// Open an `.emf` v4 container using the default mixed-domain registry.
pub fn open_emf_mixed_container<'v, 'a>(
    view: &'v StructuralView<'a>,
    scratch: &mut Scratch<'_>,
    limits: Limits,
    total_payload_work: u64,
) -> Result<EmfMixedReader<'v, 'a>, Error> {
    open_emf_mixed_container_with_registry(view, scratch, EmfAcquisitionRegistry::default(), limits, total_payload_work)
}

/// Open an `.emf` v4 container using an explicit mixed-domain registry.
pub fn open_emf_mixed_container_with_registry<'v, 'a>(
    view: &'v StructuralView<'a>,
    scratch: &mut Scratch<'_>,
    registry: EmfAcquisitionRegistry,
    limits: Limits,
    total_payload_work: u64,
) -> Result<EmfMixedReader<'v, 'a>, Error> {
    if view.family() != Family::EmfV4Candidate { return Err(Error::UnsupportedProfile); }
    let admission = semantics::admit_shared(view, &registry, scratch, limits.shared).map_err(shared_error)?;
    Ok(EmfMixedReader { view, admission, registry, limits, payload_work: total_payload_work })
}

/// Admit a single entry as either a typed EMF profile chunk or an acquisition original entry.
pub fn admit_emf_or_acquisition_entry<'v, 'a>(
    view: &'v StructuralView<'a>,
    context: &Context<'v, 'a>,
    entry_index: u32,
    limits: Limits,
) -> Result<AdmittedMixedEntry<'a>, Error> {
    if view.family() != Family::EmfV4Candidate { return Err(Error::UnsupportedProfile); }
    let entry = view.entry(entry_index).map_err(Error::Storage)?;
    if entry.record_count() > limits.records { return Err(Error::Limit); }
    let desc = context.require_schema(entry.descriptor_id(), 3).map_err(Error::Shared)?;
    let is_acq = entry.mode() == 0x0104
        && field(desc, 7).and_then(|f| text(f).ok()) == Some("original_bytes")
        && field(desc, 12).and_then(|f| text(f).ok()) == Some("urn:qualia:acquisition:original:1");

    if is_acq {
        if entry.revision() != 1 || entry.flags() != 0 || entry.stride() != 1 || entry.frame_id() != 0 {
            return Err(Error::UnsupportedProfile);
        }
        let att = field(desc, 18).ok_or(Error::Descriptor)?;
        let (mut view_block, mut lineage_block) = (None, None);
        for raw in att.value().chunks_exact(4) {
            let b = context.block(u32_at(raw, 0)).map_err(Error::Shared)?;
            match b.schema_type() { 6 => view_block = Some(b), 7 => lineage_block = Some(b), _ => {} }
        }
        let view_block = view_block.ok_or(Error::Descriptor)?;
        let payload = view.verify_chunk(entry_index).map_err(Error::Storage)?;
        Ok(AdmittedMixedEntry::Acquisition(AcquisitionOriginalEntry {
            entry_index, entry, descriptor_id: desc.id(), view_block, lineage_block, payload,
        }))
    } else {
        super::chunk::admit_with_context(view, context, entry_index, limits).map(AdmittedMixedEntry::Emf)
    }
}

/// Verify that `derived_entry_index` declares `source_entry_index` in its provenance chain with matching BLAKE3 digest.
pub fn verify_source_entry_lineage<'v, 'a>(
    view: &'v StructuralView<'a>,
    context: &Context<'v, 'a>,
    derived_entry_index: u32,
    source_entry_index: u32,
) -> Result<(), Error> {
    let derived = view.entry(derived_entry_index).map_err(Error::Storage)?;
    let source = view.entry(source_entry_index).map_err(Error::Storage)?;
    let desc = context.require_schema(derived.descriptor_id(), 3).map_err(Error::Shared)?;
    let prov = context.require_schema(scalar32(desc, 3)?, 4).map_err(Error::Shared)?;
    let sources = field(desc, 17).ok_or(Error::Descriptor)?.value();
    let digests = field(prov, 2).ok_or(Error::Descriptor)?.value();
    if digests.len() != (sources.len() / 4).checked_mul(32).ok_or(Error::Overflow)? {
        return Err(Error::Descriptor);
    }
    for (src, dig) in sources.chunks_exact(4).zip(digests.chunks_exact(32)) {
        if u32_at(src, 0) == source_entry_index {
            return if dig == source.digest() { Ok(()) } else { Err(Error::Validity) };
        }
    }
    Err(Error::Index)
}

/// Verify derivation input binding when Schema 7 is attached to an entry.
pub fn verify_derivation_schema7_lineage<'v, 'a>(
    context: &Context<'v, 'a>,
    lineage_block: Block<'a>,
    derived_entry_index: u32,
    source_entry_index: u32,
) -> Result<(), Error> {
    if lineage_block.schema_type() != 7 || lineage_block.revision() != 1 { return Err(Error::Descriptor); }
    if scalar32(lineage_block, 1)? != derived_entry_index { return Err(Error::Descriptor); }
    let inputs = required(lineage_block, 7)?;
    let digests = required(lineage_block, 8)?;
    let n = inputs.element_count() as usize;
    if digests.value().len() != n.checked_mul(32).ok_or(Error::Overflow)? { return Err(Error::Descriptor); }
    for (src, dig) in inputs.value().chunks_exact(4).zip(digests.value().chunks_exact(32)) {
        if u32_at(src, 0) == source_entry_index {
            let source_entry = context.entry(source_entry_index).map_err(Error::Shared)?;
            return if dig == source_entry.digest() { Ok(()) } else { Err(Error::Validity) };
        }
    }
    Err(Error::Index)
}

/// Encode a calibrated RF I/Q acquisition packet (`0x0105`) into a self-admitted `.emf` v4 envelope.
#[allow(clippy::too_many_arguments)]
pub fn acquire_iq_to_emf<'out>(
    clock: super::FieldClock, frame: super::FieldFrame, provenance: super::FieldProvenance<'_>,
    header: &super::IqHeader, samples: &[(f64, f64)], acquisition: super::AcquisitionSpec<'_>,
    calibration: super::CalibrationSpec<'_>, payload_buf: &mut [u8], meta_buf: &mut [u8],
    staging: &mut [u8], output: &'out mut [u8], scratch: &mut Scratch<'_>, limits: super::FieldWriteLimits,
) -> Result<&'out mut [u8], Error> {
    let n = super::encode_iq_packet(header, samples, true, payload_buf)?;
    let spec = super::ExtendedModeSpec::Iq { acquisition, calibration, samples_per_packet: header.sample_count };
    super::write_extended_envelope(
        clock, frame, provenance, 0x0105, Some(i64::from(header.sample_count)),
        header.first_sample_tick, 1, None, 1, &payload_buf[..n], &spec, meta_buf, staging, output, scratch, limits,
    )
}

/// Encode bioelectric current-dipole records (`0x0107`) into a self-admitted `.emf` v4 envelope.
#[allow(clippy::too_many_arguments)]
pub fn acquire_biofield_to_emf<'out>(
    clock: super::FieldClock, frame: super::FieldFrame, provenance: super::FieldProvenance<'_>,
    records: &[super::BiofieldRecord], temporal_step_ticks: Option<i64>, start_tick: i64,
    bio: super::BioContextSpec<'_>, evidence: super::ModelEvidenceSpec<'_>, calibration: super::CalibrationSpec<'_>,
    payload_buf: &mut [u8], meta_buf: &mut [u8], staging: &mut [u8], output: &'out mut [u8],
    scratch: &mut Scratch<'_>, limits: super::FieldWriteLimits,
) -> Result<&'out mut [u8], Error> {
    let n = super::encode_biofield_records(records, None, payload_buf)?;
    let spec = super::ExtendedModeSpec::Biofield { bio, evidence, calibration };
    let count = records.len() as u64;
    super::write_extended_envelope(
        clock, frame, provenance, 0x0107, temporal_step_ticks, start_tick,
        if temporal_step_ticks.is_some() { count } else { 0 }, None, count, &payload_buf[..n],
        &spec, meta_buf, staging, output, scratch, limits,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::{FIELD_MODEL_URI, FIELD_PROFILE};
    use crate::q42::field_envelope::{EntryInput, EnvelopeInput, Limits as StoreLimits};

    #[derive(Clone)]
    struct TestBlock { id: u32, schema: u16, fields: Vec<Vec<u8>> }

    fn f(key: u16, tag: u8, count: u32, bytes: &[u8]) -> Vec<u8> {
        let mut v = Vec::with_capacity(16 + bytes.len() + 8);
        v.extend_from_slice(&key.to_le_bytes());
        v.push(tag);
        v.push(1);
        v.extend_from_slice(&count.to_le_bytes());
        v.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes());
        v.extend_from_slice(bytes);
        while v.len() % 8 != 0 { v.push(0); }
        v
    }
    fn n32(key: u16, n: u32) -> Vec<u8> { f(key, 1, 1, &n.to_le_bytes()) }
    fn n64(key: u16, n: u64) -> Vec<u8> { f(key, 2, 1, &n.to_le_bytes()) }
    fn s(key: u16, text: &str) -> Vec<u8> { f(key, 6, text.len() as u32, text.as_bytes()) }
    fn numbers(key: u16, vals: &[f64]) -> Vec<u8> {
        f(key, 5, vals.len() as u32, &vals.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>())
    }

    fn encode_test_container(blocks: &[TestBlock], entries: &[EntryInput<'_>]) -> Vec<u8> {
        let mut sorted = blocks.to_vec();
        sorted.sort_by_key(|b| b.id);
        let mut bytes = Vec::new();
        for m in &sorted {
            let mut fields = m.fields.clone();
            fields.sort_by_key(|v| u16::from_le_bytes(v[..2].try_into().unwrap()));
            let body = fields.concat();
            bytes.extend_from_slice(&m.id.to_le_bytes());
            bytes.extend_from_slice(&m.schema.to_le_bytes());
            bytes.extend_from_slice(&1u16.to_le_bytes());
            bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&1u32.to_le_bytes());
            bytes.extend_from_slice(&body);
        }
        let input = EnvelopeInput { family: Family::EmfV4Candidate, flags: 0, metadata: &bytes, entries };
        let mut out = vec![0; input.encoded_len(StoreLimits::default()).unwrap()];
        input.encode_into(&mut out, StoreLimits::default()).unwrap();
        out
    }

    fn base_mixed_fixture() -> (Vec<TestBlock>, Vec<u8>, Vec<u8>) {
        let acq_payload = b"TEST-ACQUISITION-RAW-SENSOR-BYTES-PACKET-DATA-E01-BRIDGE-42-ABCD";
        let acq_digest = *blake3::hash(acq_payload).as_bytes();
        let mut emf_payload = vec![0u8; 128];
        emf_payload[0..8].copy_from_slice(&1.0f64.to_le_bytes());
        emf_payload[32..40].copy_from_slice(&(1.0f64 / 299_792_458.0).to_le_bytes());

        let blocks = vec![
            TestBlock { id: 1, schema: 1, fields: vec![n32(1, 0), f(2, 3, 1, &0i64.to_le_bytes()), n32(3, 0), n64(4, 1000), n64(5, 1)] },
            TestBlock { id: 2, schema: 2, fields: vec![n32(1, 0), numbers(3, &[0.0; 3]), numbers(4, &[0.0, 0.0, 0.0, 1.0]), s(8, "right-handed-X-forward-Y-left-Z-up")] },
            TestBlock { id: 11, schema: 4, fields: vec![n32(1, 4)] },
            TestBlock { id: 20, schema: 6, fields: vec![
                n32(1, 1), f(2, 9, 1, &1u32.to_le_bytes()), f(3, 7, 32, &acq_digest),
                n64(4, 0), n64(5, 64), n32(8, 0), n32(10, 7), n32(11, 0), n32(12, 0), n32(13, 1),
                f(14, 1, 1, &64u32.to_le_bytes()), f(15, 2, 1, &1u64.to_le_bytes()), f(16, 1, 1, &0u32.to_le_bytes()),
                f(17, 2, 1, &0u64.to_le_bytes()), n32(32, 0),
            ] },
            TestBlock { id: 10, schema: 3, fields: vec![
                f(3, 8, 1, &11u32.to_le_bytes()), s(7, "original_bytes"), s(8, "1"), n32(9, 1), s(10, "byte"),
                n32(11, 7), s(12, "urn:qualia:acquisition:original:1"), f(18, 8, 1, &20u32.to_le_bytes()),
            ] },
            TestBlock { id: 31, schema: 4, fields: vec![n32(1, 2), f(2, 7, 32, &acq_digest)] },
            TestBlock { id: 4, schema: 103, fields: vec![
                n32(1, 0x0100), n32(2, 1), f(3, 8, 1, &1u32.to_le_bytes()), f(4, 8, 1, &2u32.to_le_bytes()),
                f(5, 8, 1, &31u32.to_le_bytes()), s(6, FIELD_PROFILE), s(7, "linear"),
                f(8, 2, 3, &[1u64, 1, 1].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()),
                f(9, 1, 3, &[1u32, 1, 1].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()),
                n32(10, 0), f(11, 3, 1, &0i64.to_le_bytes()),
            ] },
            TestBlock { id: 30, schema: 3, fields: vec![
                f(2, 8, 1, &2u32.to_le_bytes()), f(3, 8, 1, &31u32.to_le_bytes()), s(7, "electromagnetic_field_snapshot"),
                s(8, "mixed:mode-registry"), s(12, FIELD_MODEL_URI), f(17, 9, 1, &1u32.to_le_bytes()), f(18, 8, 1, &4u32.to_le_bytes()),
            ] },
        ];
        (blocks, acq_payload.to_vec(), emf_payload)
    }

    fn make_entries<'a>(acq: &'a [u8], emf: &'a [u8]) -> [EntryInput<'a>; 2] {
        [
            EntryInput { mode: 0x0104, revision: 1, flags: 0, stream_id: 1, frame_id: 0, start_tick: 0, time_count: 0, spatial_key: 0, record_count: acq.len() as u64, stride: 1, codec: 0, descriptor_id: 10, payload: acq },
            EntryInput { mode: 0x0100, revision: 1, flags: 0, stream_id: 2, frame_id: 2, start_tick: 0, time_count: 0, spatial_key: 0, record_count: 1, stride: 128, codec: 0, descriptor_id: 30, payload: emf },
        ]
    }

    #[test]
    fn test_mixed_domain_positive_admission() {
        let (blocks, acq, emf) = base_mixed_fixture();
        let entries = make_entries(&acq, &emf);
        let bytes = encode_test_container(&blocks, &entries);
        let view = StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut m_nodes = [semantics::NodeState::default(); 64];
        let mut e_nodes = [semantics::NodeState::default(); 16];
        let mut scratch = Scratch { metadata: &mut m_nodes, entries: &mut e_nodes };
        let mut reader = open_emf_mixed_container(&view, &mut scratch, Limits::default(), 10_000_000).unwrap();
        assert_eq!(reader.entry_count(), 2);

        let chunk1 = reader.chunk(1).unwrap();
        let acq_entry = chunk1.as_acquisition().expect("expected acquisition original");
        assert_eq!(acq_entry.entry_index(), 1);
        assert_eq!(acq_entry.payload(), acq.as_slice());
        assert_eq!(acq_entry.view_kind().unwrap(), 1);
        assert_eq!(acq_entry.subrange().unwrap(), acq.as_slice());

        let chunk2 = reader.chunk(2).unwrap();
        let emf_chunk = chunk2.as_emf().expect("expected emf chunk");
        assert_eq!(emf_chunk.entry_index(), 2);
        assert_eq!(emf_chunk.canonical_mode(), 0x0100);
        assert!(emf_chunk.field(0).unwrap().is_some());

        verify_source_entry_lineage(&view, reader.context(), 2, 1).unwrap();
    }

    #[test]
    fn test_mixed_domain_malformed_schema6_rejected() {
        let (mut blocks, acq, emf) = base_mixed_fixture();
        let entries = make_entries(&acq, &emf);
        for b in &mut blocks {
            if b.id == 20 {
                for field in &mut b.fields {
                    if u16::from_le_bytes(field[..2].try_into().unwrap()) == 5 { *field = n64(5, 9999); }
                }
            }
        }
        let bytes = encode_test_container(&blocks, &entries);
        let view = StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut m = [semantics::NodeState::default(); 64];
        let mut e = [semantics::NodeState::default(); 16];
        let mut scratch = Scratch { metadata: &mut m, entries: &mut e };
        assert!(open_emf_mixed_container(&view, &mut scratch, Limits::default(), 10_000_000).is_err());
    }

    #[test]
    fn test_mixed_domain_malformed_schema7_rejected() {
        let (mut blocks, acq, emf) = base_mixed_fixture();
        blocks.push(TestBlock {
            id: 25, schema: 7,
            fields: vec![
                f(1, 9, 1, &1u32.to_le_bytes()), f(2, 7, 32, &blake3::hash(&acq).as_bytes()[..]),
                f(3, 7, 32, &[0u8; 32]), n64(4, 1), f(5, 7, 32, &[0u8; 32]), n64(6, 2),
                f(7, 9, 1, &1u32.to_le_bytes()), f(8, 7, 32, &blake3::hash(&acq).as_bytes()[..]),
                f(9, 1, 1, &0u32.to_le_bytes()),
            ],
        });
        for b in &mut blocks {
            if b.id == 10 {
                for field in &mut b.fields {
                    if u16::from_le_bytes(field[..2].try_into().unwrap()) == 18 {
                        *field = f(18, 8, 2, &[20u32, 25].iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>());
                    }
                }
            }
        }
        let entries = make_entries(&acq, &emf);
        let bytes = encode_test_container(&blocks, &entries);
        let view = StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut m = [semantics::NodeState::default(); 64];
        let mut e = [semantics::NodeState::default(); 16];
        let mut scratch = Scratch { metadata: &mut m, entries: &mut e };
        assert!(open_emf_mixed_container(&view, &mut scratch, Limits::default(), 10_000_000).is_err());
    }

    #[test]
    fn test_mixed_domain_non_0104_claiming_schema6_rejected() {
        let (mut blocks, acq, emf) = base_mixed_fixture();
        for b in &mut blocks {
            if b.id == 20 {
                for field in &mut b.fields {
                    if u16::from_le_bytes(field[..2].try_into().unwrap()) == 2 { *field = f(2, 9, 1, &2u32.to_le_bytes()); }
                }
            }
        }
        let entries = make_entries(&acq, &emf);
        let bytes = encode_test_container(&blocks, &entries);
        let view = StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut m = [semantics::NodeState::default(); 64];
        let mut e = [semantics::NodeState::default(); 16];
        let mut scratch = Scratch { metadata: &mut m, entries: &mut e };
        assert!(open_emf_mixed_container(&view, &mut scratch, Limits::default(), 10_000_000).is_err());
    }

    #[test]
    fn test_mixed_domain_mismatched_source_digest_rejected() {
        let (mut blocks, acq, emf) = base_mixed_fixture();
        let bad_digest = [0x55u8; 32];
        for b in &mut blocks {
            if b.id == 31 {
                for field in &mut b.fields {
                    if u16::from_le_bytes(field[..2].try_into().unwrap()) == 2 { *field = f(2, 7, 32, &bad_digest); }
                }
            }
        }
        let entries = make_entries(&acq, &emf);
        let bytes = encode_test_container(&blocks, &entries);
        let view = StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
        let mut m = [semantics::NodeState::default(); 64];
        let mut e = [semantics::NodeState::default(); 16];
        let mut scratch = Scratch { metadata: &mut m, entries: &mut e };
        assert!(open_emf_mixed_container(&view, &mut scratch, Limits::default(), 10_000_000).is_err());
    }
}
