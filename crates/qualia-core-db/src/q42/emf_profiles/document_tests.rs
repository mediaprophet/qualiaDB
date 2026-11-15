use super::*;
use super::super::semantics::NodeState;
#[test]
fn reader_retains_shared_admission_and_budgets_repeated_payload_work() {
    let payload = field_payload(0);
    let bytes = encode(&metadata(0, false), &[row(0, &payload, 1, false)]);
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut m = [NodeState::default(); 32];
    let mut e = [NodeState::default(); 4];
    let mut scratch = Scratch {
        metadata: &mut m,
        entries: &mut e,
    };
    let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 768).unwrap();
    assert_eq!(reader.entry_count(), 1);
    let initial = reader.remaining_shared_work();
    let chunk = reader.chunk(1).unwrap();
    assert_eq!(chunk.verified_stored_bytes(), payload);
    assert!(reader.remaining_shared_work() < initial);
    let after_first = reader.remaining_shared_work();
    reader.chunk(1).unwrap();
    assert!(reader.remaining_shared_work() < after_first);
    assert_eq!(reader.remaining_payload_work(), 0);
    assert!(matches!(reader.chunk(1), Err(Error::Limit)));
    // Verified borrowed chunks stay usable after the reader's budget is exhausted.
    assert!(chunk.field(0).unwrap().is_some());
}
#[test]
fn reader_checks_budget_before_corruption_and_never_accepts_wrong_registry() {
    let payload = field_payload(0);
    let mut bytes = encode(&metadata(0, false), &[row(0, &payload, 1, false)]);
    let offset = StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default())
        .unwrap()
        .entry(1)
        .unwrap()
        .payload_offset() as usize;
    bytes[offset] ^= 1;
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    let mut m = [NodeState::default(); 32];
    let mut e = [NodeState::default(); 4];
    let mut scratch = Scratch {
        metadata: &mut m,
        entries: &mut e,
    };
    let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 383).unwrap();
    assert!(matches!(reader.chunk(1), Err(Error::Limit)));
    let mut reader = ProfileReader::open(&view, &mut scratch, Limits::default(), 384).unwrap();
    assert!(matches!(
        reader.chunk(1),
        Err(Error::Storage(
            super::super::super::field_envelope::Error::PayloadDigest
        ))
    ));
    let mut unknown = metadata(0, false);
    replace(&mut unknown, 4, 6, s(6, "unqualified_model"));
    let bytes = encode(&unknown, &[row(0, &payload, 1, false)]);
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    assert!(matches!(
        ProfileReader::open(&view, &mut scratch, Limits::default(), 384),
        Err(Error::UnsupportedProfile)
    ));
}
