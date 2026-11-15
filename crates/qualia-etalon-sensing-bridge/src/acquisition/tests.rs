use super::*;
use crate::{BridgeAuthority, BridgeError};
use core::cell::Cell;
use etalon_sense_core::{
    buffer::SampleLayout,
    evidence::{ArtifactDigest, DigestAlgorithm, EvidenceOrigin},
    observation::{
        HostPolicyDecision, MeasurementUncertainty, ObservationEnvelope, ObservationKind,
        ObservationTimeAnchor, QuantityUnitIds,
    },
    quality::{CalibrationRecord, CalibrationState, QualityRecord, SourceDescriptor, SourceKind},
    quantity::SampleEncoding,
    time::ClockStamp,
};
use qualia_core_db::{
    container_10d as c,
    sensing::{
        acquisition as a, record::ResolvedFieldPayloadBinding, PhysicalFieldSemantics, SemanticId,
    },
};
fn id(n: u64) -> SemanticId {
    SemanticId::new(n).unwrap()
}
pub(super) fn binding(data: &[u8]) -> ResolvedFieldPayloadBinding {
    let section = c::SectionInput {
        section_type: c::SectionType::FieldSidecar,
        alignment_tier: c::AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: data,
    };
    let typed = c::TypedFieldRecord {
        field_id: 9,
        kind: c::FIELD_KIND_SCALAR,
        flags: 0,
        unit_id: 11,
        payload_section_id: c::SectionType::FieldSidecar as u32,
        payload_offset: 0,
        payload_length: data.len() as u64,
    };
    let identity = c::ManifoldIdentityV2 {
        entity_id: c::StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    let mut manifest = [0; c::MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let mut sections = [section; 2];
    let mut bytes = [0; 4096];
    let len = c::encode_container_with_manifold_identity(
        &c::Container10dHeader::proposed(),
        &[section],
        &identity,
        &[typed],
        &mut manifest,
        &mut sections,
        &mut bytes,
    )
    .unwrap();
    c::seal_whole_file_crc32c(&mut bytes[..len]);
    let mut scratch = [c::TypedFieldRecord::default(); 1];
    let mut fields = [c::TypedFieldRecord::default(); 1];
    let resolver =
        c::typed_payloads::TypedPayloadResolver::new(&bytes[..len], &[], &mut scratch, &mut fields)
            .unwrap();
    let field = c::field_section::FieldDescriptor {
        name_hash: 22,
        unit_hash: 33,
        type_kind: c::field_section::FieldTypeKind::Spectral as u8,
        support: c::field_section::FieldSupportKind::Stream as u8,
        representation: c::field_section::FieldRepresentationKind::Sampled as u8,
        reserved: 0,
        reserved2: 0,
        sample_offset: 0,
    };
    let physical = PhysicalFieldSemantics::from_existing(
        &field,
        &typed,
        qualia_core_db::sensing::MeasurementModality::Electromagnetic,
        id(13),
    )
    .unwrap();
    ResolvedFieldPayloadBinding::new(physical, &resolver, [1; 16]).unwrap()
}
fn observation(
    anchor: ObservationTimeAnchor,
    digest: Option<ArtifactDigest>,
) -> ObservationEnvelope {
    ObservationEnvelope::new(
        ObservationKind::RecordedFrame,
        SourceDescriptor::new(41, SourceKind::Recorded).unwrap(),
        Some(8),
        Some(2),
        Some(9),
        Some(ClockStamp::new(2, 3, 100, 7).unwrap()),
        QuantityUnitIds::new(Some(301), Some(302)).unwrap(),
        MeasurementUncertainty::Unknown,
        QualityRecord::Unknown,
        CalibrationRecord::new(CalibrationState::Missing).unwrap(),
        EvidenceOrigin::Unknown,
        digest,
        HostPolicyDecision::NotEvaluated,
    )
    .unwrap()
    .with_time_anchor(anchor)
    .unwrap()
}
pub(super) fn mapping() -> AcquisitionMapping {
    AcquisitionMapping {
        axes: [
            a::Axis::Sample,
            a::Axis::Channel,
            a::Axis::Sensor,
            a::Axis::Lag,
        ],
        byte_order: a::ByteOrder::Little,
        components: a::ComponentOrder::Scalar,
        context: a::Context {
            quantity: a::Quantity {
                kind: id(13),
                unit: id(11),
            },
            stage: a::SampleStage::RawCodes,
            sensor: id(17),
            geometry: None,
            calibration: None,
            uncertainty: None,
        },
        clock: None,
        signal_axes: a::SignalAxes {
            carrier: a::Carrier::Unknown,
            baseband_hz: None,
            pulse_rate: None,
            complex_convention: None,
            phase_origin: None,
        },
    }
}
fn mapped_clock() -> a::Clock {
    a::Clock::new(
        id(2),
        3,
        a::Timing {
            scale: 0,
            epoch_seconds: 0,
            epoch_nanoseconds: 0,
            ticks: a::Rate::new(1_000_000_000, 1).unwrap(),
            samples: a::Rate::new(48_000, 1).unwrap(),
            start_tick: 100,
        },
    )
    .unwrap()
}
pub(super) struct Authority {
    pub base: bool,
    pub extra: bool,
    pub clock: bool,
    pub extra_calls: Cell<usize>,
    expected_source: ([u8; 32], usize),
}
impl BridgeAuthority for Authority {
    fn confirm_mapping(&self, o: &ObservationEnvelope, s: u64, p: &PhysicalFieldSemantics) -> bool {
        o.source().source_id() == 41 && s == 17 && p.physical_quantity() == id(13)
    }
    fn approve(&self, _: &ObservationEnvelope, _: u64, _: &ResolvedFieldPayloadBinding) -> bool {
        self.base
    }
    fn confirm_clock_namespace(&self, _: &ObservationEnvelope) -> bool {
        self.clock
    }
}
impl AcquisitionAuthority for Authority {
    fn confirm_source_index_binding(
        &self,
        binding: &ResolvedFieldPayloadBinding,
        original: a::Original<'_>,
        offset: usize,
        length: usize,
    ) -> bool {
        let selected = binding.raw_payload();
        selected.container_id() == [1; 16]
            && selected.section_id() == c::SectionType::FieldSidecar as u32
            && selected.offset() == 0
            && selected.length() == 16
            && length == 16
            && original.digest() == self.expected_source.0
            && offset == self.expected_source.1
    }
    fn confirm_acquisition(
        &self,
        _: u64,
        _: u64,
        _: &ResolvedFieldPayloadBinding,
        _: &AcquisitionInput<'_, '_>,
    ) -> bool {
        self.extra_calls.set(self.extra_calls.get() + 1);
        self.extra
    }
}
fn authority() -> Authority {
    Authority {
        base: true,
        extra: true,
        clock: true,
        extra_calls: Cell::new(0),
        expected_source: (a::Original::admit(&[0u8; 16], 16).unwrap().digest(), 0),
    }
}
fn authority_for(bytes: &[u8], offset: usize) -> Authority {
    Authority {
        expected_source: (
            a::Original::admit(bytes, bytes.len()).unwrap().digest(),
            offset,
        ),
        ..authority()
    }
}
fn layout() -> SampleLayout {
    SampleLayout::new(1, [8, 0, 0, 0], [2, 0, 0, 0], SampleEncoding::U16).unwrap()
}
fn input<'a, 'l>(bytes: &'a [u8], layout: &'l SampleLayout) -> AcquisitionInput<'a, 'l> {
    AcquisitionInput {
        observation: observation(ObservationTimeAnchor::FirstSample, None),
        layout,
        original_bytes: bytes,
        payload_offset: 0,
        max_original_bytes: bytes.len(),
        mapping: mapping(),
    }
}
#[test]
fn preserves_actual_provider_envelope_layout_and_original_separately() {
    let bytes = [0xffu8; 16];
    let l = layout();
    let src = input(&bytes, &l);
    let original = src.observation;
    let view = admit_observation(&authority_for(&bytes, 0), 7, 17, binding(&bytes), src).unwrap();
    assert_eq!(*view.observation(), original);
    assert_eq!(view.provider_layout(), &l);
    assert_eq!(view.tensor().region().original().bytes(), bytes);
    assert_eq!(view.tensor().numeric([7, 0, 0, 0]).unwrap(), [65535.0, 0.0]);
    assert_eq!(
        view.observation().policy(),
        HostPolicyDecision::NotEvaluated
    );
    assert_eq!(
        view.observation().source().trust_state(),
        etalon_sense_core::quality::TrustState::Unverified
    );
    assert_eq!(view.tensor().clock(), None);
}
#[test]
fn existing_denial_cannot_be_overridden_and_precedes_source_work() {
    let bytes = [0u8; 16];
    let l = layout();
    let mut auth = authority();
    auth.base = false;
    let mut src = input(&bytes, &l);
    src.payload_offset = usize::MAX;
    src.max_original_bytes = 0;
    assert!(matches!(
        admit_observation(&auth, 7, 17, binding(&bytes), src),
        Err(Error::Bridge(BridgeError::NotAuthorized))
    ));
    assert_eq!(auth.extra_calls.get(), 0);
    auth.base = true;
    auth.extra = false;
    let mut src = input(&bytes, &l);
    src.payload_offset = usize::MAX;
    src.max_original_bytes = 0;
    assert!(matches!(
        admit_observation(&auth, 7, 17, binding(&bytes), src),
        Err(Error::NotAuthorized)
    ));
}
#[test]
fn scope_and_original_ranges_fail_closed() {
    let bytes = [0u8; 16];
    let l = layout();
    let mut src = input(&bytes, &l);
    src.mapping.context.sensor = id(18);
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::ScopeMismatch)
    ));
    let mut src = input(&bytes, &l);
    src.mapping.context.quantity.unit = id(99);
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::ScopeMismatch)
    ));
    let mut src = input(&bytes, &l);
    src.payload_offset = 1;
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::Acquisition(a::Error::Bounds))
    ));
    let mut src = input(&bytes, &l);
    src.payload_offset = usize::MAX;
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::Acquisition(a::Error::Overflow))
    ));
    let mut src = input(&bytes, &l);
    src.max_original_bytes = 15;
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::Acquisition(a::Error::Budget))
    ));
}
#[test]
fn exact_first_sample_clock_requires_existing_gate_and_domain_match() {
    let bytes = [0u8; 16];
    let l = layout();
    let mut src = input(&bytes, &l);
    src.mapping.clock = Some(mapped_clock());
    let view = admit_observation(&authority(), 7, 17, binding(&bytes), src).unwrap();
    assert_eq!(
        view.tensor().clock().unwrap().time_at(0).unwrap(),
        a::ExactTime {
            numerator: 1,
            denominator: 10_000_000
        }
    );
    let mut auth = authority();
    auth.clock = false;
    let mut src = input(&bytes, &l);
    src.mapping.clock = Some(mapped_clock());
    assert!(matches!(
        admit_observation(&auth, 7, 17, binding(&bytes), src),
        Err(Error::Bridge(BridgeError::UnmappedClock))
    ));
    let mut src = input(&bytes, &l);
    src.mapping.clock = Some(a::Clock::new(id(3), 3, mapped_clock().timing()).unwrap());
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::Acquisition(a::Error::Clock))
    ));
    let mut src = input(&bytes, &l);
    let mut t = mapped_clock().timing();
    t.start_tick = 101;
    src.mapping.clock = Some(a::Clock::new(id(2), 3, t).unwrap());
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::Acquisition(a::Error::Clock))
    ));
    let mut src = input(&bytes, &l);
    src.observation = observation(ObservationTimeAnchor::ReadCompletion, None);
    src.mapping.clock = Some(mapped_clock());
    assert!(matches!(
        admit_observation(&authority(), 7, 17, binding(&bytes), src),
        Err(Error::Acquisition(a::Error::Clock))
    ));
    let mut src = input(&bytes, &l);
    src.observation = observation(ObservationTimeAnchor::ReadCompletion, None);
    assert!(admit_observation(&authority(), 7, 17, binding(&bytes), src)
        .unwrap()
        .tensor()
        .clock()
        .is_none());
}
#[test]
fn payload_digest_matches_selected_bytes_not_unrelated_original_metadata() {
    let mut bytes = [0u8; 19];
    bytes[..3].copy_from_slice(b"hdr");
    bytes[3..].fill(5);
    let l = layout();
    let digest = a::Original::admit(&bytes[3..], 16).unwrap().digest();
    let mut src = input(&bytes, &l);
    src.payload_offset = 3;
    src.observation = observation(
        ObservationTimeAnchor::FirstSample,
        Some(ArtifactDigest::new(DigestAlgorithm::Blake3, digest)),
    );
    let view =
        admit_observation(&authority_for(&bytes, 3), 7, 17, binding(&bytes[3..]), src).unwrap();
    assert_eq!(view.tensor().region().original().bytes(), bytes);
    assert_ne!(view.tensor().region().original().digest(), digest);
    let mut src = input(&bytes, &l);
    src.payload_offset = 3;
    src.observation = observation(
        ObservationTimeAnchor::FirstSample,
        Some(ArtifactDigest::new(DigestAlgorithm::Blake3, [7; 32])),
    );
    assert!(matches!(
        admit_observation(&authority_for(&bytes, 3), 7, 17, binding(&bytes[3..]), src),
        Err(Error::PayloadDigest)
    ));
    let mut src = input(&bytes, &l);
    src.payload_offset = 3;
    src.observation = observation(
        ObservationTimeAnchor::FirstSample,
        Some(ArtifactDigest::new(DigestAlgorithm::Sha256, [7; 32])),
    );
    assert!(matches!(
        admit_observation(&authority_for(&bytes, 3), 7, 17, binding(&bytes[3..]), src),
        Err(Error::UnsupportedDigest)
    ));
}

#[path = "tests/provider.rs"]
mod provider;

#[test]
fn same_length_source_substitution_needs_independent_source_index_relation() {
    let indexed = [0u8; 16];
    let substituted = [0xffu8; 16];
    let l = layout();
    // Broad processing and namespace approval are deliberately true. Trusted
    // custody still binds this container/section/range only to indexed bytes.
    assert!(matches!(
        admit_observation(
            &authority(),
            7,
            17,
            binding(&indexed),
            input(&substituted, &l)
        ),
        Err(Error::SourceIndexBinding)
    ));
    let mut original = [0u8; 18];
    original[..2].copy_from_slice(b"xy");
    let mut src = input(&original, &l);
    src.payload_offset = 2;
    // Whole source matches, but its payload-offset relation is deliberately wrong.
    assert!(matches!(
        admit_observation(&authority_for(&original, 1), 7, 17, binding(&indexed), src),
        Err(Error::SourceIndexBinding)
    ));
    let mut src = input(&original, &l);
    src.payload_offset = 2;
    let view =
        admit_observation(&authority_for(&original, 2), 7, 17, binding(&indexed), src).unwrap();
    assert_eq!(view.tensor().region().offset(), 2);
    assert_eq!(view.indexed().record().raw_payload().offset(), 0);
}
