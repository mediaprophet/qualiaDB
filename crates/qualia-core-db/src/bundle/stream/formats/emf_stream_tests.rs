use super::super::{AuthenticatedRange, FormatError};
use super::{emf_present_id, EmfAdmitError, EmfProfile, EmfStreamView};
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{verify_leaf_span, SpanLimits, VerifiedLeafSpan, LEAF_BYTES},
    selection::{select_entry, SelectedPlainEntry},
};
use crate::container_10d::crc32c::crc32c;
use crate::net::qdnf::types::StrongDigest;
use crate::q42::emf_container::{EmfCellRecord64, EmfContainerWriter};
use crate::q42::field_envelope::{self, Family, Limits};
const OPAQUE: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/emf_candidate-opaque.emf");
const ALIGNED: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/emf_candidate-aligned4096.emf");
const TWO: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/emf_candidate-two-entries.emf");
const TRUNCATED: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/malformed/emf_candidate-truncated.emf");
const UNKNOWN_CRITICAL: &[u8] = include_bytes!(
    "../../../q42/field_envelope/fixtures/malformed/emf_candidate-unknown-critical-schema.emf"
);
const PAYLOAD_DIGEST: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/malformed/emf_candidate-payload.emf");
fn with_qbdl_span(
    payload: &[u8],
    run: impl FnOnce(SelectedPlainEntry<'_>, &VerifiedLeafSpan<'_>),
) {
    let fixture = Fixture::new_with_payload(payload);
    let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
    let closure = fixture.bound(&pins, pin, publisher);
    let header_view = admit_header(
        &fixture.source()[..64],
        fixture.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut entry_scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &header_view,
        &fixture.source()[header_view.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut entry_scratch,
    )
    .unwrap();
    let mut entry_rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let selected = select_entry(
        &closure,
        fixture.body(),
        &fixture.source()[..64],
        &index,
        fixture.entry_page(),
        0,
        &mut entry_rows,
        &mut framing,
    )
    .unwrap();
    let mut sibling_scratch = [StrongDigest::ZERO; 32];
    let span = verify_leaf_span(
        &selected.tree_spec().unwrap(),
        selected.root(),
        0,
        payload,
        &[&[0]],
        &mut sibling_scratch,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: 16_384,
        },
    )
    .unwrap();
    run(selected, &span);
}
fn complete<'a>(
    selected: SelectedPlainEntry<'a>,
    span: &'a VerifiedLeafSpan<'_>,
    payload: &[u8],
) -> AuthenticatedRange<'a> {
    AuthenticatedRange::new(selected, span, 0, payload.len()).unwrap()
}
fn admit_err(range: AuthenticatedRange<'_>, limits: Limits) -> EmfAdmitError {
    match EmfStreamView::admit(range, limits) {
        Err(e) => e,
        Ok(_) => panic!("admitted"),
    }
}
fn admit_bytes(payload: &[u8]) -> EmfAdmitError {
    let mut err = None;
    with_qbdl_span(payload, |selected, span| {
        err = Some(admit_err(
            complete(selected, span, payload),
            Limits::default(),
        ));
    });
    match err {
        Some(e) => e,
        None => panic!("no error"),
    }
}
fn retouch_header_crc(bytes: &mut [u8]) {
    let mut block = [0u8; 128];
    block.copy_from_slice(&bytes[..128]);
    block[124..].fill(0);
    bytes[124..128].copy_from_slice(&crc32c(&block).to_le_bytes());
}
#[test]
fn v4_candidate_admits_envelope_profile() {
    with_qbdl_span(OPAQUE, |selected, span| {
        let view = EmfStreamView::admit(
            complete(selected, span, OPAQUE),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(view.profile(), EmfProfile::EnvelopeV4);
        assert!(view.ready_for_plan());
        match view.structure() {
            Some(structure) => assert_eq!(structure.family(), Family::EmfV4Candidate),
            None => panic!("v4 structure"),
        }
        assert_eq!(view.range().offset(), 0);
        assert_eq!(view.range().bytes().len(), OPAQUE.len());
        assert_eq!(emf_present_id(7, &view), Some(7));
        assert!(view.declared_interval(0).is_none());
    });
}
#[test]
fn two_emf_entries_are_distinct_regions() {
    let mut opaque_offset = None;
    let mut opaque_chunk = None;
    with_qbdl_span(OPAQUE, |selected, span| {
        let view = EmfStreamView::admit(
            complete(selected, span, OPAQUE),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(view.profile(), EmfProfile::EnvelopeV4);
        opaque_offset = Some(view.structure().unwrap().entry(1).unwrap().payload_offset());
        opaque_chunk = Some(view.select_region(0).unwrap().to_vec());
    });
    let mut aligned_offset = None;
    let mut aligned_chunk = None;
    with_qbdl_span(ALIGNED, |selected, span| {
        let view = EmfStreamView::admit(
            complete(selected, span, ALIGNED),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(view.profile(), EmfProfile::EnvelopeV4);
        aligned_offset = Some(view.structure().unwrap().entry(1).unwrap().payload_offset());
        aligned_chunk = Some(view.select_region(0).unwrap().to_vec());
    });
    assert_eq!(opaque_chunk.unwrap(), aligned_chunk.unwrap());
    assert_ne!(opaque_offset.unwrap(), aligned_offset.unwrap());
    with_qbdl_span(TWO, |selected, span| {
        let view =
            EmfStreamView::admit(complete(selected, span, TWO), Limits::default()).unwrap();
        let structure = match view.structure() {
            Some(structure) => {
                assert_eq!(structure.entry_count(), 2);
                structure
            }
            None => panic!("v4 structure"),
        };
        let a = view.select_region(0).unwrap();
        let b = view.select_region(1).unwrap();
        assert_eq!(a, b);
        assert_ne!(a.as_ptr(), b.as_ptr());
        assert_ne!(
            structure.entry(1).unwrap().payload_offset(),
            structure.entry(2).unwrap().payload_offset()
        );
    });
}
#[test]
fn version_byte_mismatch_is_not_treated_as_v4() {
    let mut as_v2 = OPAQUE.to_vec();
    as_v2[8] = 2;
    as_v2[9] = 0;
    match admit_bytes(&as_v2) {
        EmfAdmitError::Unsupported
        | EmfAdmitError::Extent
        | EmfAdmitError::Format(FormatError::Integrity | FormatError::Geometry) => {}
        other => panic!("v2 version byte treated as v4 {other:?}"),
    }
    let mut as_v3 = OPAQUE.to_vec();
    as_v3[8] = 3;
    as_v3[9] = 0;
    match admit_bytes(&as_v3) {
        EmfAdmitError::Unsupported
        | EmfAdmitError::Extent
        | EmfAdmitError::Format(FormatError::Integrity | FormatError::Geometry) => {}
        other => panic!("v3 version byte treated as v4 {other:?}"),
    }
    let mut unknown = OPAQUE.to_vec();
    unknown[8] = 99;
    unknown[9] = 0;
    assert_eq!(admit_bytes(&unknown), EmfAdmitError::Version);
    let mut legacy = OPAQUE.to_vec();
    legacy[8] = 1;
    legacy[9] = 0;
    assert_eq!(admit_bytes(&legacy), EmfAdmitError::Unsupported);
}
#[test]
fn truncated_or_partial_range_refuses() {
    with_qbdl_span(OPAQUE, |selected, span| {
        let short = AuthenticatedRange::new(selected, span, 0, OPAQUE.len() / 2).unwrap();
        assert_eq!(admit_err(short, Limits::default()), EmfAdmitError::Extent);
        let shifted = AuthenticatedRange::new(selected, span, 8, OPAQUE.len() - 8).unwrap();
        assert_eq!(
            admit_err(shifted, Limits::default()),
            EmfAdmitError::Extent
        );
    });
    assert_eq!(admit_bytes(&OPAQUE[..100]), EmfAdmitError::Extent);
    assert!(matches!(
        admit_bytes(TRUNCATED),
        EmfAdmitError::Envelope(field_envelope::Error::Header) | EmfAdmitError::Extent
    ));
}
#[test]
fn last_digest_or_verify_chunk_refuses_integrity() {
    let mut changed = OPAQUE.to_vec();
    changed[87] ^= 0xff;
    retouch_header_crc(&mut changed);
    assert_eq!(admit_bytes(&changed), EmfAdmitError::IndexDigest);
    with_qbdl_span(PAYLOAD_DIGEST, |selected, span| {
        let view = EmfStreamView::admit(
            complete(selected, span, PAYLOAD_DIGEST),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(view.profile(), EmfProfile::EnvelopeV4);
        match view.select_region(0) {
            Err(EmfAdmitError::PayloadDigest) => {}
            Err(other) => panic!("expected PayloadDigest, got {other:?}"),
            Ok(_) => panic!("selected corrupt chunk"),
        }
    });
}
#[test]
fn select_region_returns_authenticated_chunk() {
    with_qbdl_span(OPAQUE, |selected, span| {
        let view = EmfStreamView::admit(
            complete(selected, span, OPAQUE),
            Limits::default(),
        )
        .unwrap();
        let chunk = view.select_region(0).unwrap();
        match view.structure() {
            Some(structure) => match structure.verify_chunk(1) {
                Ok(expected) => assert_eq!(chunk, expected),
                Err(_) => panic!("verify_chunk"),
            },
            None => panic!("v4 structure"),
        }
        match view.select_region(99) {
            Err(EmfAdmitError::Envelope(field_envelope::Error::EntryIndex)) => {}
            Err(other) => panic!("expected EntryIndex, got {other:?}"),
            Ok(_) => panic!("selected missing chunk"),
        }
    });
}
#[test]
fn opaque_archival_is_not_ready_for_plan() {
    with_qbdl_span(UNKNOWN_CRITICAL, |selected, span| {
        let view = EmfStreamView::admit(
            complete(selected, span, UNKNOWN_CRITICAL),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(view.profile(), EmfProfile::OpaqueArchival);
        assert!(!view.ready_for_plan());
        match view.structure() {
            Some(structure) => assert_eq!(structure.family(), Family::EmfV4Candidate),
            None => panic!("v4 structure"),
        }
        assert_eq!(emf_present_id(7, &view), None);
        let _preserved = view.select_region(0).unwrap();
    });
}
#[test]
fn non_emf_bytes_fail_family_without_filename_sniff() {
    let mut junk = vec![0u8; 256];
    junk[..8].copy_from_slice(b"qac\0\0\0\0\0");
    assert_eq!(admit_bytes(&junk), EmfAdmitError::Family);
    junk[..8].copy_from_slice(&[0xff; 8]);
    assert_eq!(admit_bytes(&junk), EmfAdmitError::Family);
}
fn v2_brick() -> Vec<u8> {
    let mut writer = EmfContainerWriter::new([0.; 3], 1., [2, 1, 1], 0.);
    match writer.add_brick_chunk(0, &[EmfCellRecord64::default()], 0, 1, 0) {
        Ok(()) => {}
        Err(_) => panic!("v2 chunk 0"),
    }
    match writer.add_brick_chunk(1, &[EmfCellRecord64::default(); 2], 0, 1, 0) {
        Ok(()) => {}
        Err(_) => panic!("v2 chunk 1"),
    }
    match writer.to_bytes() {
        Ok(bytes) => bytes,
        Err(_) => panic!("v2 brick"),
    }
}
#[test]
fn v2_brick_admits_directory_profile() {
    let brick = v2_brick();
    assert!(
        (brick.len() as u64) <= LEAF_BYTES,
        "small v2 brick must fit one leaf"
    );
    with_qbdl_span(&brick, |selected, span| {
        let view = match EmfStreamView::admit(complete(selected, span, &brick), Limits::default()) {
            Ok(v) => v,
            Err(_) => panic!("v2 brick admit"),
        };
        assert_eq!(view.profile(), EmfProfile::DirectoryV2);
        assert!(view.ready_for_plan());
        assert!(view.structure().is_none());
        assert_eq!(view.range().offset(), 0);
        assert_eq!(view.range().bytes().len(), brick.len());
        assert_eq!(emf_present_id(7, &view), Some(7));
        assert!(view.declared_interval(0).is_none());
    });
}
#[test]
fn truncated_v2_directory_is_extent_or_geometry() {
    let brick = v2_brick();
    let truncated = &brick[..80];
    match admit_bytes(truncated) {
        EmfAdmitError::Extent | EmfAdmitError::Format(FormatError::Geometry) => {}
        other => panic!("truncated directory {other:?}"),
    }
    let mid = &brick[..100];
    match admit_bytes(mid) {
        EmfAdmitError::Extent | EmfAdmitError::Format(FormatError::Geometry) => {}
        other => panic!("mid directory {other:?}"),
    }
}
#[test]
fn select_region_on_admitted_v2_chunk() {
    let brick = v2_brick();
    with_qbdl_span(&brick, |selected, span| {
        let view = match EmfStreamView::admit(complete(selected, span, &brick), Limits::default()) {
            Ok(v) => v,
            Err(_) => panic!("v2 brick admit"),
        };
        assert_eq!(view.profile(), EmfProfile::DirectoryV2);
        let chunk = match view.select_region(0) {
            Ok(c) => c,
            Err(_) => panic!("v2 chunk"),
        };
        assert_eq!(chunk.len(), 64);
        assert_eq!(chunk, &brick[192..256]);
        match view.select_region(1) {
            Ok(second) => {
                assert_eq!(second.len(), 128);
                assert_eq!(second, &brick[320..448]);
            }
            Err(_) => panic!("v2 chunk 1"),
        }
    });
}
