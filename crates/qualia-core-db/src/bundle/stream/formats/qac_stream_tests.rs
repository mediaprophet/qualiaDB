use super::super::qac::{QacAdmitError, QacCompleteView};
use super::super::AuthenticatedRange;
use super::{qac_present_id, QacProfile, QacStreamView};
use crate::audio::qac::{signal, QacStorageOptions};
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{verify_leaf_span, SpanLimits, VerifiedLeafSpan},
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::qdnf::types::StrongDigest;
use crate::q42::field_envelope::Family;
use crate::q42::field_semantics::{Limits as SharedLimits, NodeState, Scratch};
const OPAQUE: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
const PCM: &[u8] = include_bytes!(
    "../../../q42/field_envelope/fixtures/candidate/qac_candidate-pcm-signed-zero.qac"
);
const TWO: &[u8] =
    include_bytes!("../../../q42/field_envelope/fixtures/candidate/qac_candidate-two-entries.qac");
const LEGACY: &[u8] = include_bytes!("../../../audio/qac/fixtures/q4au-v2-contract.qac");
const UNKNOWN_CRITICAL: &[u8] = include_bytes!(
    "../../../q42/field_envelope/fixtures/malformed/qac_candidate-unknown-critical-schema.qac"
);
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
    match QacStreamView::admit(range, options) {
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
fn with_scratch<R>(run: impl FnOnce(&mut Scratch<'_>) -> R) -> R {
    let mut metadata = [NodeState::default(); 64];
    let mut entries = [NodeState::default(); 32];
    run(&mut Scratch {
        metadata: &mut metadata,
        entries: &mut entries,
    })
}
#[test]
fn complete_entry_still_admits_opaque_candidate() {
    with_qbdl_span(OPAQUE, |selected, span| {
        let view = match QacStreamView::admit(
            complete(selected, span, OPAQUE),
            QacStorageOptions::default(),
        ) {
            Ok(v) => v,
            Err(_) => panic!("opaque complete-entry"),
        };
        assert_eq!(view.profile(), QacProfile::CompleteEntry);
        assert!(view.ready_for_plan());
        assert_eq!(qac_present_id(1, &view), Some(1));
        assert_eq!(
            view.complete().storage().structure().family(),
            Family::QacV1Candidate
        );
        assert_eq!(view.complete().range().offset(), 0);
        assert_eq!(view.complete().range().bytes().len(), OPAQUE.len());
        match view.complete().storage().structure().verify_chunk(1) {
            Ok(_) => {}
            Err(_) => panic!("chunk"),
        }
    });
}
#[test]
fn admit_signal_on_pcm_signed_zero_preserves_bytes() {
    with_qbdl_span(PCM, |selected, span| {
        let view = match QacStreamView::admit(
            complete(selected, span, PCM),
            QacStorageOptions::default(),
        ) {
            Ok(v) => v,
            Err(_) => panic!("pcm complete-entry"),
        };
        assert_eq!(view.profile(), QacProfile::CompleteEntry);
        let chunk = match view.complete().storage().structure().verify_chunk(1) {
            Ok(b) => b,
            Err(_) => panic!("chunk"),
        };
        assert!(chunk.len() >= 8);
        assert_eq!(&chunk[..4], &[0x00, 0x00, 0x00, 0x00]);
        assert_eq!(&chunk[4..8], &[0x00, 0x00, 0x00, 0x80]);
        let scientific = with_scratch(|scratch| {
            match view.complete().admit_signal(
                1,
                scratch,
                SharedLimits::default(),
                signal::Limits::default(),
            ) {
                Ok(sig) => {
                    assert_eq!(sig.as_bytes(), chunk);
                    true
                }
                Err(QacAdmitError::Unsupported) => false,
                Err(e) => panic!("unexpected signal error {e:?}"),
            }
        });
        let view = with_scratch(|scratch| {
            match view.with_signal(1, scratch, SharedLimits::default(), signal::Limits::default())
            {
                Ok(v) => v,
                Err(_) => panic!("with_signal"),
            }
        });
        if scientific {
            assert_eq!(view.profile(), QacProfile::SignalScientific);
            assert!(view.ready_for_plan());
            assert_eq!(qac_present_id(1, &view), Some(1));
        } else {
            assert_eq!(view.profile(), QacProfile::OpaqueArchival);
            assert!(!view.ready_for_plan());
            assert_eq!(qac_present_id(1, &view), None);
            match view.complete().storage().structure().verify_chunk(1) {
                Ok(again) => assert_eq!(again, chunk),
                Err(_) => panic!("archival lost chunk"),
            }
        }
    });
}
#[test]
fn two_qac_entries_admit_independently() {
    with_qbdl_span(OPAQUE, |sel_a, span_a| {
        with_qbdl_span(PCM, |sel_b, span_b| {
            let a = match QacStreamView::admit(
                complete(sel_a, span_a, OPAQUE),
                QacStorageOptions::default(),
            ) {
                Ok(v) => v,
                Err(_) => panic!("opaque"),
            };
            let b = match QacStreamView::admit(
                complete(sel_b, span_b, PCM),
                QacStorageOptions::default(),
            ) {
                Ok(v) => v,
                Err(_) => panic!("pcm"),
            };
            assert_eq!(a.profile(), QacProfile::CompleteEntry);
            assert_eq!(b.profile(), QacProfile::CompleteEntry);
            assert_eq!(qac_present_id(1, &a), Some(1));
            assert_eq!(qac_present_id(2, &b), Some(2));
            assert_ne!(
                a.complete().range().selected().candidate_identity(),
                b.complete().range().selected().candidate_identity()
            );
            assert_ne!(
                a.complete().range().bytes().len(),
                b.complete().range().bytes().len()
            );
        });
    });
    with_qbdl_span(TWO, |selected, span| {
        let view = match QacCompleteView::admit(
            complete(selected, span, TWO),
            QacStorageOptions::default(),
        ) {
            Ok(v) => v,
            Err(_) => panic!("two-entries"),
        };
        assert_eq!(view.storage().structure().entry_count(), 2);
        let first = with_scratch(|scratch| {
            view.admit_signal(1, scratch, SharedLimits::default(), signal::Limits::default())
        });
        let second = with_scratch(|scratch| {
            view.admit_signal(2, scratch, SharedLimits::default(), signal::Limits::default())
        });
        match (first, second) {
            (Ok(_), Ok(_)) | (Err(QacAdmitError::Unsupported), Err(QacAdmitError::Unsupported)) => {}
            (a, b) => panic!("inner entries not independent {a:?} {b:?}"),
        }
    });
}
#[test]
fn partial_range_still_extent() {
    with_qbdl_span(OPAQUE, |selected, span| {
        let short = AuthenticatedRange::new(selected, span, 0, OPAQUE.len() / 2).unwrap();
        assert_eq!(
            admit_err(short, QacStorageOptions::default()),
            QacAdmitError::Extent
        );
        let shifted = AuthenticatedRange::new(selected, span, 8, OPAQUE.len() - 8).unwrap();
        assert_eq!(
            admit_err(shifted, QacStorageOptions::default()),
            QacAdmitError::Extent
        );
    });
}
#[test]
fn legacy_ua4q_refuses() {
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
fn unknown_critical_or_unsupported_signal_is_archival_not_audio() {
    with_qbdl_span(UNKNOWN_CRITICAL, |selected, span| {
        let view = match QacStreamView::admit(
            complete(selected, span, UNKNOWN_CRITICAL),
            QacStorageOptions::default(),
        ) {
            Ok(v) => v,
            Err(_) => panic!("unknown-critical complete-entry"),
        };
        assert_eq!(view.profile(), QacProfile::CompleteEntry);
        let view = with_scratch(|scratch| {
            match view.with_signal(1, scratch, SharedLimits::default(), signal::Limits::default()) {
                Ok(v) => v,
                Err(e) => panic!("unexpected {e:?}"),
            }
        });
        assert_eq!(view.profile(), QacProfile::OpaqueArchival);
        assert!(!view.ready_for_plan());
        assert_eq!(qac_present_id(1, &view), None);
        match view.complete().storage().structure().verify_chunk(1) {
            Ok(bytes) => assert!(!bytes.is_empty()),
            Err(_) => panic!("archival dropped payload"),
        }
    });
}
#[test]
fn declared_interval_absent_is_not_epoch_zero() {
    with_qbdl_span(OPAQUE, |selected, span| {
        let view = match QacStreamView::admit(
            complete(selected, span, OPAQUE),
            QacStorageOptions::default(),
        ) {
            Ok(v) => v,
            Err(_) => panic!("opaque"),
        };
        let entry = match view.complete().storage().structure().entry(1) {
            Ok(e) => e,
            Err(_) => panic!("opaque entry"),
        };
        assert_eq!(entry.flags() & 2, 0);
        assert!(view.declared_interval().is_none());
    });
    with_qbdl_span(TWO, |selected, span| {
        let view = match QacStreamView::admit(
            complete(selected, span, TWO),
            QacStorageOptions::default(),
        ) {
            Ok(v) => v,
            Err(_) => panic!("two-entries"),
        };
        let entry = match view.complete().storage().structure().entry(1) {
            Ok(e) => e,
            Err(_) => panic!("two-entries entry"),
        };
        assert_eq!(entry.flags() & 2, 0);
        assert!(view.declared_interval().is_none());
    });
}
#[test]
fn declared_interval_matches_timed_pcm_entry() {
    with_qbdl_span(PCM, |selected, span| {
        let view = match QacStreamView::admit(
            complete(selected, span, PCM),
            QacStorageOptions::default(),
        ) {
            Ok(v) => v,
            Err(_) => panic!("pcm"),
        };
        let entry = match view.complete().storage().structure().entry(1) {
            Ok(e) => e,
            Err(_) => panic!("pcm entry"),
        };
        assert_ne!(entry.flags() & 2, 0);
        let interval = match view.declared_interval() {
            Some(i) => i,
            None => panic!("pcm timed interval"),
        };
        assert_eq!(interval.start_num(), entry.start_tick());
        assert_eq!(interval.start_den(), 1);
        match entry.start_tick().checked_add(entry.time_count() as i64) {
            Some(end) => assert_eq!(interval.end_num(), end),
            None => panic!("pcm end"),
        }
        assert_eq!(interval.end_den(), 1);
        assert_eq!(entry.start_tick(), 0);
        assert_eq!(entry.time_count(), 4);
        let view = with_scratch(|scratch| {
            match view.with_signal(1, scratch, SharedLimits::default(), signal::Limits::default())
            {
                Ok(v) => v,
                Err(_) => panic!("with_signal"),
            }
        });
        match view.declared_interval() {
            Some(again) => {
                assert_eq!(again.start_num(), 0);
                assert_eq!(again.end_num(), 4);
            }
            None => panic!("pcm timed after with_signal"),
        }
    });
}
