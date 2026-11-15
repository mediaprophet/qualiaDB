use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, AdmittedIndex, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, BoundClosure, CLOSURE_PURPOSE},
};
fn fixture(run: impl FnOnce(&Fixture, &BoundClosure<'_>, &AdmittedIndex<'_>)) {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let limits = IndexReadLimits::default();
    let h = admit_header(
        &f.source()[..64],
        f.source().len() as u64,
        ExtentLimit::default(),
        limits,
    )
    .unwrap();
    let mut scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &h,
        &f.source()[h.index_offset() as usize..],
        limits,
        &mut scratch,
    )
    .unwrap();
    run(&f, &bound, &index);
}
#[test]
fn actual_signed_body_page_and_canonical_index_select_exact_plain_entry() {
    fixture(|f, bound, index| {
        let mut rows = [EntryRecord::EMPTY; 128];
        let mut framing = vec![0; HASH_SCRATCH_BYTES];
        let selected = select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            f.entry_page(),
            0,
            &mut rows,
            &mut framing,
        )
        .unwrap();
        assert_eq!(selected.candidate_identity(), f.identity());
        let actual = index.entries().next().unwrap().unwrap();
        assert_eq!(selected.offset(), actual.offset());
        assert_eq!(selected.logical_length(), actual.length());
        assert_eq!(selected.kind(), actual.kind());
        assert_eq!(selected.ordinal(), 0);
        selected.check_closure(bound).unwrap();
        assert_eq!(
            selected.tree_spec().unwrap().logical_length(),
            actual.length()
        );
        assert_eq!(selected.source_commit().length, f.source().len() as u64);
    });
}
#[test]
fn changed_body_page_header_and_ordinal_refuse_without_selection() {
    fixture(|f, bound, index| {
        let mut rows = [EntryRecord::EMPTY; 128];
        let mut framing = vec![0; HASH_SCRATCH_BYTES];
        let mut body = f.body().to_vec();
        let end = body.len() - 1;
        body[end] ^= 1;
        assert!(select_entry(
            bound,
            &body,
            &f.source()[..64],
            index,
            f.entry_page(),
            0,
            &mut rows,
            &mut framing
        )
        .is_err());
        let mut page = f.entry_page().to_vec();
        let end = page.len() - 1;
        page[end] ^= 1;
        assert!(select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            &page,
            0,
            &mut rows,
            &mut framing
        )
        .is_err());
        let mut header = f.source()[..64].to_vec();
        header[12] ^= 1;
        assert!(select_entry(
            bound,
            f.body(),
            &header,
            index,
            f.entry_page(),
            0,
            &mut rows,
            &mut framing
        )
        .is_err());
        assert!(select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            f.entry_page(),
            1,
            &mut rows,
            &mut framing
        )
        .is_err());
        assert!(select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            f.entry_page(),
            u32::MAX,
            &mut rows,
            &mut framing
        )
        .is_err());
    });
}
#[test]
fn bounded_rows_and_framing_and_zero_raw_allocation_selection() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    fixture(|f, bound, index| {
        let mut rows = [EntryRecord::EMPTY; 128];
        let mut framing = vec![0; HASH_SCRATCH_BYTES];
        assert!(select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            f.entry_page(),
            0,
            &mut [],
            &mut framing
        )
        .is_err());
        assert!(select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            f.entry_page(),
            0,
            &mut rows,
            &mut []
        )
        .is_err());
        select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            f.entry_page(),
            0,
            &mut rows,
            &mut framing,
        )
        .unwrap();
        let guard = AllocGuard::begin("selected-plain-entry", true);
        let selected = select_entry(
            bound,
            f.body(),
            &f.source()[..64],
            index,
            f.entry_page(),
            0,
            &mut rows,
            &mut framing,
        )
        .unwrap();
        assert_eq!(selected.ordinal(), 0);
        guard
            .check()
            .expect("shared entry selection must use caller buffers");
    });
}
