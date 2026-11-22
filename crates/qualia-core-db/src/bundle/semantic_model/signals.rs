//! Semantic addresses for native electromagnetic and acoustic record ranges.

use super::{
    invalid, open_semantic_model, packing::hex_digest, valid_iri, BundleReader,
    SemanticModelBuilder, RDF_GRAPH, RDF_TYPE,
};
use crate::{
    audio::qac::profiles::{field_opt, field_str, field_u32, AdmittedEnvelope, ProfileLimits},
    q42::{
        emf_container::{EmfContainerReader, EmfReadLimits},
        emf_profiles::{Limits as EmfProfileLimits, ProfileReader},
        field_envelope::{Block, Entry, Family, Limits, StructuralView},
        field_semantics::{NodeState, Scratch},
    },
    q_hash,
};
use std::io;

const HAS_RANGE: &str = "https://qualiadb.org/ontology/hmc#hasNativeRange";
const RECORD_COUNT: &str = "https://qualiadb.org/ontology/hmc#nativeRecordCount";
const RECORD_STRIDE: &str = "https://qualiadb.org/ontology/hmc#nativeRecordStrideBytes";
const SPATIAL_KEY: &str = "https://qualiadb.org/ontology/hmc#nativeSpatialKey";
const TIME_SLICE: &str = "https://qualiadb.org/ontology/hmc#nativeTimeSlice";
const STREAM: &str = "https://qualiadb.org/ontology/hmc#nativeStream";
const MODE: &str = "https://qualiadb.org/ontology/hmc#nativeMode";
const START_TICK: &str = "https://qualiadb.org/ontology/hmc#nativeStartTick";
const MODEL: &str = "https://qualiadb.org/ontology/hmc#nativeModel";
const QUANTITY: &str = "https://qualiadb.org/ontology/hmc#nativeQuantity";
const UNIT: &str = "http://qudt.org/schema/qudt/unit";
const INSTRUMENT: &str = "https://qualiadb.org/ontology/hmc#nativeInstrument";
const ALGORITHM: &str = "https://qualiadb.org/ontology/hmc#nativeAlgorithm";
const RECEPTOR_MODEL: &str = "https://qualiadb.org/ontology/hmc#receptorModel";
const DERATING_MODEL: &str = "https://qualiadb.org/ontology/hmc#deratingModel";
const THERMAL_MODEL: &str = "https://qualiadb.org/ontology/hmc#thermalModel";
const EXPOSURE_CONTEXT: &str = "https://qualiadb.org/ontology/hmc#exposureContext";
const PCM_TRACK: &str = "https://qualiadb.org/ontology/hmc#nativePcmTrack";
const DERIVED_FROM: &str = "http://www.w3.org/ns/prov#wasDerivedFrom";

#[derive(Clone, Copy)]
enum RangeKind {
    EmfCells,
    EmfPcm,
    EmfProfile,
    Qac,
}

impl RangeKind {
    fn name(self) -> &'static str {
        match self {
            Self::EmfCells => "emf-cells",
            Self::EmfPcm => "emf-pcm",
            Self::EmfProfile => "emf-profile",
            Self::Qac => "qac-records",
        }
    }

    fn class(self) -> &'static str {
        match self {
            Self::EmfCells => "https://qualiadb.org/ontology/hmc#EmfCellRange",
            Self::EmfPcm => "https://qualiadb.org/ontology/hmc#EmfPcmFrameRange",
            Self::EmfProfile => "https://qualiadb.org/ontology/hmc#EmfProfileRecordRange",
            Self::Qac => "https://qualiadb.org/ontology/hmc#QacRecordRange",
        }
    }

    fn iri(self, digest: &str, index: u32) -> String {
        format!("urn:sha256:{digest}:{}:{index}", self.name())
    }
}

/// A verified, fixed-stride record slice borrowed from a sibling HMC entry.
#[derive(Clone, Copy)]
pub struct HmcSignalRange<'a> {
    bytes: &'a [u8],
    count: u64,
    stride: u32,
}

impl<'a> HmcSignalRange<'a> {
    fn new(bytes: &'a [u8], count: u64, stride: u32) -> io::Result<Self> {
        if stride == 0 || count.checked_mul(u64::from(stride)) != Some(bytes.len() as u64) {
            return Err(invalid("native signal range has an invalid extent"));
        }
        Ok(Self { bytes, count, stride })
    }

    pub fn record_count(self) -> u64 {
        self.count
    }

    pub fn record_stride(self) -> u32 {
        self.stride
    }

    pub fn record(self, ordinal: u64) -> Option<&'a [u8]> {
        if ordinal >= self.count {
            return None;
        }
        let stride = self.stride as usize;
        let start = usize::try_from(ordinal).ok()?.checked_mul(stride)?;
        self.bytes.get(start..start.checked_add(stride)?)
    }
}

fn legacy_emf(bytes: &[u8]) -> io::Result<EmfContainerReader<'_>> {
    EmfContainerReader::parse_with_limits(bytes, EmfReadLimits::conservative())
        .map_err(|error| invalid(error.to_string()))
}

fn emf_cells<'a>(emf: &EmfContainerReader<'a>, index: usize) -> io::Result<HmcSignalRange<'a>> {
    let row = emf.chunk_entry(index).ok_or_else(|| invalid("missing EMF cell range"))?;
    let bytes = emf.chunk_payload(index).ok_or_else(|| invalid("missing EMF cell bytes"))?;
    HmcSignalRange::new(bytes, u64::from(row.cell_count), 64)
}

fn emf_pcm<'a>(emf: &EmfContainerReader<'a>, index: usize) -> io::Result<HmcSignalRange<'a>> {
    let extension = emf.extension().ok_or_else(|| invalid("missing EMF PCM extension"))?;
    let track = extension.metadata().audio_tracks.get(index)
        .ok_or_else(|| invalid("missing EMF PCM track"))?;
    let bytes = extension.audio_payload(index).ok_or_else(|| invalid("missing EMF PCM bytes"))?;
    HmcSignalRange::new(bytes, track.frames, u32::from(track.spec.channels) * 4)
}

fn envelope_range<'a>(
    view: &StructuralView<'a>, row: Entry<'a>, index: u32,
) -> io::Result<HmcSignalRange<'a>> {
    let bytes = view.verify_chunk(index)
        .map_err(|error| invalid(format!("native signal payload: {error:?}")))?;
    HmcSignalRange::new(bytes, row.record_count(), row.stride())
}

/// Open a known Q42 range IRI against its exact, content-addressed bundle sibling.
/// Admission verifies package and native integrity. Subsequent ordinal reads borrow the
/// original fixed-stride bytes without copying or allocating.
pub fn open_hmc_signal_range<'a>(
    bundle: &BundleReader<'a>, key: &str, range_iri: &str,
    lex_page: &mut [u8], lex_term: &mut [u8],
) -> io::Result<HmcSignalRange<'a>> {
    let entry = bundle.entry(key).ok_or_else(|| invalid("missing HMC signal entry"))?;
    let digest = hex_digest(&entry.sha256);
    let prefix = format!("urn:sha256:{digest}:");
    let (kind, ordinal) = range_iri.strip_prefix(&prefix).and_then(|s| s.split_once(':'))
        .ok_or_else(|| invalid("HMC signal range does not address the selected resource"))?;
    let index: u32 = ordinal.parse().map_err(|_| invalid("invalid HMC signal range ordinal"))?;
    let kind = match (entry.kind.as_str(), kind) {
        ("emf", "emf-cells") => RangeKind::EmfCells,
        ("emf", "emf-pcm") => RangeKind::EmfPcm,
        ("emf", "emf-profile") => RangeKind::EmfProfile,
        ("qac", "qac-records") => RangeKind::Qac,
        _ => return Err(invalid("HMC signal format and range type disagree")),
    };
    if kind.iri(&digest, index) != range_iri {
        return Err(invalid("noncanonical HMC signal range IRI"));
    }
    let semantic = open_semantic_model(bundle)?;
    let root = if bundle.entry(super::SEMANTIC_MODEL_KEY).is_some() {
        super::SEMANTIC_MODEL_KEY
    } else {
        super::MODEL_ALIAS
    };
    if !bundle.verify_entry(root) || !bundle.verify_entry(key) {
        return Err(invalid("HMC semantic model or signal resource digest mismatch"));
    }
    let hash = q_hash(range_iri);
    if hash == 0 || semantic.lookup_lexicon_hash_into(hash, lex_page, lex_term)?
        .and_then(|len| lex_term.get(..len)) != Some(range_iri.as_bytes())
    {
        return Err(invalid("HMC signal range IRI is absent from Q42LEX"));
    }
    let bytes = bundle.get(key).ok_or_else(|| invalid("missing HMC signal bytes"))?;
    match kind {
        RangeKind::EmfCells => emf_cells(&legacy_emf(bytes)?, index as usize),
        RangeKind::EmfPcm => emf_pcm(&legacy_emf(bytes)?, index as usize),
        RangeKind::EmfProfile | RangeKind::Qac => {
            let family = match kind {
                RangeKind::EmfProfile => Family::EmfV4Candidate,
                _ => Family::QacV1Candidate,
            };
            let view = StructuralView::open(bytes, family, Limits::default())
                .map_err(|error| invalid(format!("native signal envelope: {error:?}")))?;
            let row = view.entry(index)
                .map_err(|error| invalid(format!("native signal directory: {error:?}")))?;
            envelope_range(&view, row, index)
        }
    }
}

impl SemanticModelBuilder {
    /// Import existing native semantic pointers and content-addressed signal ranges.
    pub fn merge_resource_addressability(
        mut self, kind: &str, bytes: &[u8], resource_iri: &str, digest: &[u8; 32],
    ) -> io::Result<Self> {
        match kind {
            "m64" => return self.merge_m64(bytes, resource_iri),
            "gtop" => return self.merge_gtop(bytes, resource_iri),
            "qac" | "emf" => {
                let digest = hex_digest(digest);
                match kind {
                    "qac" => self.merge_qac_ranges(bytes, resource_iri, &digest)?,
                    _ if bytes.get(8..10) == Some(&4u16.to_le_bytes()[..]) => {
                        self.merge_emf_profile_ranges(bytes, resource_iri, &digest)?;
                    }
                    _ => self.merge_emf64_ranges(bytes, resource_iri, &digest)?,
                }
            }
            _ => {}
        }
        Ok(self)
    }

    fn signal_number(
        &mut self, range: &str, predicate: &str, value: impl std::fmt::Display,
    ) -> io::Result<()> {
        let literal = format!("\"{value}\"^^<http://www.w3.org/2001/XMLSchema#integer>");
        self.fact(range, predicate, &literal, RDF_GRAPH)
    }

    fn signal_iri_field(
        &mut self, range: &str, predicate: &str, block: Block<'_>, key: u16,
    ) -> io::Result<()> {
        if let Some(field) = field_opt(block, key) {
            let term = field_str(field).map_err(|error| invalid(error.to_string()))?;
            if valid_iri(term) {
                self.fact(range, predicate, term, RDF_GRAPH)?;
            }
        }
        Ok(())
    }

    fn signal_range_facts(
        &mut self, resource: &str, range: &str, kind: RangeKind, records: HmcSignalRange<'_>,
    ) -> io::Result<()> {
        self.fact(resource, HAS_RANGE, range, RDF_GRAPH)?;
        self.fact(range, RDF_TYPE, kind.class(), RDF_GRAPH)?;
        self.signal_number(range, RECORD_COUNT, records.count)?;
        self.signal_number(range, RECORD_STRIDE, records.stride)
    }

    fn merge_emf64_ranges(&mut self, bytes: &[u8], resource: &str, digest: &str) -> io::Result<()> {
        let emf = legacy_emf(bytes)?;
        for index in 0..emf.chunk_count() {
            let kind = RangeKind::EmfCells;
            let iri = kind.iri(digest, index as u32);
            let row = emf.chunk_entry(index).ok_or_else(|| invalid("missing EMF chunk"))?;
            self.signal_range_facts(resource, &iri, kind, emf_cells(&emf, index)?)?;
            self.signal_number(&iri, SPATIAL_KEY, row.spatial_hilbert_index)?;
            self.signal_number(&iri, TIME_SLICE, row.time_slice_index)?;
        }
        if let Some(extension) = emf.extension() {
            for (index, track) in extension.metadata().audio_tracks.iter().enumerate() {
                let kind = RangeKind::EmfPcm;
                let iri = kind.iri(digest, index as u32);
                self.signal_range_facts(resource, &iri, kind, emf_pcm(&emf, index)?)?;
                self.signal_number(&iri, START_TICK, track.spec.start_tick)?;
                if valid_iri(&track.spec.id) {
                    self.fact(&iri, PCM_TRACK, &track.spec.id, RDF_GRAPH)?;
                }
            }
        }
        Ok(())
    }

    fn merge_qac_ranges(&mut self, bytes: &[u8], resource: &str, digest: &str) -> io::Result<()> {
        match AdmittedEnvelope::open_alloc(bytes, ProfileLimits::default()) {
            Ok(envelope) => self.merge_envelope_ranges(envelope.view(), resource, digest, RangeKind::Qac),
            Err(_) => {
                let qac = crate::audio::qac::QacStorageView::open(
                    bytes,
                    crate::audio::qac::QacStorageOptions::default(),
                )
                .map_err(|error| invalid(format!("QAC v1 storage admission: {error:?}")))?;
                self.merge_envelope_ranges(qac.structure(), resource, digest, RangeKind::Qac)
            }
        }
    }

    fn merge_emf_profile_ranges(
        &mut self, bytes: &[u8], resource: &str, digest: &str,
    ) -> io::Result<()> {
        let view = StructuralView::open(bytes, Family::EmfV4Candidate, Limits::default())
            .map_err(|error| invalid(format!("EMF profile envelope: {error:?}")))?;
        let mut metadata = vec![NodeState::default(); view.blocks().count()];
        let mut entries = vec![NodeState::default(); view.entry_count() as usize];
        let mut scratch = Scratch { metadata: &mut metadata, entries: &mut entries };
        ProfileReader::open(&view, &mut scratch, EmfProfileLimits::default(), 0)
            .map_err(|error| invalid(format!("EMF profile metadata: {error:?}")))?;
        self.merge_envelope_ranges(&view, resource, digest, RangeKind::EmfProfile)
    }

    fn merge_envelope_ranges(
        &mut self, view: &StructuralView<'_>, resource: &str, digest: &str, kind: RangeKind,
    ) -> io::Result<()> {
        let metadata: std::collections::BTreeMap<_, _> =
            view.blocks().map(|block| (block.id(), block)).collect();
        let find = |id| metadata.get(&id).copied().ok_or_else(|| invalid("missing signal metadata"));
        for index in 1..=view.entry_count() {
            let iri = kind.iri(digest, index);
            let row = view.entry(index).map_err(|error| invalid(format!("native signal directory: {error:?}")))?;
            self.signal_range_facts(resource, &iri, kind, envelope_range(view, row, index)?)?;
            self.signal_number(&iri, STREAM, row.stream_id())?;
            self.signal_number(&iri, MODE, row.mode())?;
            if row.flags() & 2 != 0 {
                self.signal_number(&iri, START_TICK, row.start_tick())?;
            }
            if row.flags() & 4 != 0 {
                self.signal_number(&iri, SPATIAL_KEY, row.spatial_key())?;
            }
            let descriptor = find(row.descriptor_id())?;
            for (pred, key) in [(QUANTITY, 7), (UNIT, 8), (MODEL, 12)] {
                self.signal_iri_field(&iri, pred, descriptor, key)?;
            }
            let provenance = field_opt(descriptor, 3).ok_or_else(|| invalid("missing signal provenance"))?;
            let provenance = find(field_u32(provenance).map_err(|error| invalid(error.to_string()))?)?;
            self.signal_iri_field(&iri, ALGORITHM, provenance, 3)?;
            self.signal_iri_field(&iri, INSTRUMENT, provenance, 5)?;
            if let Some(sources) = field_opt(descriptor, 17) {
                for source in sources.value().chunks_exact(4) {
                    let id = u32::from_le_bytes(source.try_into().map_err(|_| invalid("invalid signal source"))?);
                    self.fact(&iri, DERIVED_FROM, &kind.iri(digest, id), RDF_GRAPH)?;
                }
            }
            if let Some(extensions) = field_opt(descriptor, 18) {
                for extension in extensions.value().chunks_exact(4) {
                    let id = u32::from_le_bytes(extension.try_into().map_err(|_| invalid("invalid signal extension"))?);
                    let block = find(id)?;
                    let fields: &[(&str, u16)] = match block.schema_type() {
                        202 => &[(QUANTITY, 1), (UNIT, 2)],
                        203 => &[(ALGORITHM, 1)],
                        205 => &[(MODEL, 1)],
                        207 => &[(RECEPTOR_MODEL, 4)],
                        208 => &[(DERATING_MODEL, 10), (EXPOSURE_CONTEXT, 11), (THERMAL_MODEL, 12)],
                        _ => &[],
                    };
                    for &(predicate, key) in fields {
                        self.signal_iri_field(&iri, predicate, block, key)?;
                    }
                }
            }
        }
        Ok(())
    }
}
