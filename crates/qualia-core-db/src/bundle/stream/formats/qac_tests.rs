use super::qac::{QacAdmitError, QacCompleteView};
use super::AuthenticatedRange;
use crate::audio::qac::QacStorageOptions;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{verify_leaf_span, SpanLimits, VerifiedLeafSpan},
    selection::{select_entry, SelectedPlainEntry},
};
use crate::container_10d::crc32c::crc32c;
use crate::net::qdnf::types::StrongDigest;
use crate::q42::field_envelope::{self, Family};
const CURRENT: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
const LEGACY: &[u8] = include_bytes!("../../../audio/qac/fixtures/q4au-v2-contract.qac");
const TRUNCATED: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/malformed/qac_candidate-truncated.qac");
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
fn admit_err(range: AuthenticatedRange<'_>, options: QacStorageOptions) -> QacAdmitError {
    match QacCompleteView::admit(range, options) {
        Err(e) => e,
        Ok(_) => panic!("admitted"),
    }
}
fn admit_bytes(payload: &[u8]) -> QacAdmitError {
    let mut err = None;
    with_qbdl_span(payload, |selected, span| {
        err = Some(admit_err(
            complete(selected, span, payload),
            QacStorageOptions::default(),
        ));
    });
    err.unwrap()
}
fn retouch_header_crc(bytes: &mut [u8]) {
    let mut block = [0u8; 128];
    block.copy_from_slice(&bytes[..128]);
    block[124..].fill(0);
    bytes[124..128].copy_from_slice(&crc32c(&block).to_le_bytes());
}
#[test]
fn current_complete_entry_admits_qac_v1_candidate() {
    with_qbdl_span(CURRENT, |selected, span| {
        let view = QacCompleteView::admit(
            complete(selected, span, CURRENT),
            QacStorageOptions::default(),
        )
        .unwrap();
        assert_eq!(view.storage().structure().family(), Family::QacV1Candidate);
        assert_eq!(view.range().offset(), 0);
        assert_eq!(view.range().bytes().len(), CURRENT.len());
        view.storage().structure().verify_chunk(1).unwrap();
    });
}
#[test]
fn partial_range_is_extent() {
    with_qbdl_span(CURRENT, |selected, span| {
        let short = AuthenticatedRange::new(selected, span, 0, CURRENT.len() / 2).unwrap();
        assert_eq!(
            admit_err(short, QacStorageOptions::default()),
            QacAdmitError::Extent
        );
        let shifted = AuthenticatedRange::new(selected, span, 8, CURRENT.len() - 8).unwrap();
        assert_eq!(
            admit_err(shifted, QacStorageOptions::default()),
            QacAdmitError::Extent
        );
    });
}
#[test]
fn truncated_slice_refuses() {
    assert_eq!(admit_bytes(&CURRENT[..100]), QacAdmitError::Extent);
    assert!(matches!(
        admit_bytes(TRUNCATED),
        QacAdmitError::Envelope(field_envelope::Error::Header) | QacAdmitError::Extent
    ));
}
#[test]
fn trailing_extra_byte_refuses() {
    let mut extra = CURRENT.to_vec();
    extra.push(0);
    // Live open checks header file_size before remainder; no Trailing arm exists.
    assert!(matches!(
        admit_bytes(&extra),
        QacAdmitError::Envelope(field_envelope::Error::Header) | QacAdmitError::Extent
    ));
}
#[test]
fn mutated_version_byte_is_version() {
    let mut changed = CURRENT.to_vec();
    changed[8] = 99;
    assert_eq!(admit_bytes(&changed), QacAdmitError::Version);
}
#[test]
fn last_digest_byte_corrupted_is_digest_error() {
    let mut changed = CURRENT.to_vec();
    // Index digest occupies header[56..88]; CRC is checked first, so retouch it.
    changed[87] ^= 0xff;
    retouch_header_crc(&mut changed);
    assert_eq!(admit_bytes(&changed), QacAdmitError::IndexDigest);
}
#[test]
fn legacy_ua4q_fixture_is_refused() {
    // 72-byte UA4Q is shorter than the current 128-byte header, so open yields Extent, not Family.
    let err = admit_bytes(LEGACY);
    assert!(
        matches!(
            err,
            QacAdmitError::Family | QacAdmitError::Extent | QacAdmitError::Envelope(_)
        ),
        "{err:?}"
    );
}
#[test]
fn non_qac_bytes_fail_family_without_filename_sniff() {
    let mut junk = vec![0u8; 256];
    junk[..8].copy_from_slice(b"Q42EMF64");
    assert_eq!(admit_bytes(&junk), QacAdmitError::Family);
    junk[..8].copy_from_slice(&[0xff; 8]);
    assert_eq!(admit_bytes(&junk), QacAdmitError::Family);
}
