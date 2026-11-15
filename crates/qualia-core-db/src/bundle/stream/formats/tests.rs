use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{verify_leaf_span, SpanLimits},
    selection::select_entry,
};
use crate::net::qdnf::types::StrongDigest;
use crate::q42::emf_container::{header::EmfCellRecord64, writer::EmfContainerWriter};
fn limits() -> FormatLimits {
    FormatLimits {
        max_entry_bytes: 16384,
        max_rows: 2,
        max_metadata_bytes: 192,
        max_chunk_bytes: 192,
    }
}
fn emf() -> Vec<u8> {
    let mut writer = EmfContainerWriter::new([0.; 3], 1., [2, 1, 1], 0.);
    writer
        .add_brick_chunk(0, &[EmfCellRecord64::default()], 0, 1, 0)
        .unwrap();
    writer
        .add_brick_chunk(1, &[EmfCellRecord64::default(); 2], 0, 1, 0)
        .unwrap();
    let mut out = vec![0; writer.total_required_bytes().unwrap()];
    writer.write_to_slice(&mut out).unwrap();
    out
}
fn authenticated(
    payload: &[u8],
    run: impl FnOnce(AuthenticatedRange<'_>, AuthenticatedRange<'_>, AuthenticatedRange<'_>),
) {
    let f = Fixture::new_with_payload(payload);
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let closure = f.bound(&pins, pin, publisher);
    let h = admit_header(
        &f.source()[..64],
        f.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &h,
        &f.source()[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut scratch,
    )
    .unwrap();
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let selected = select_entry(
        &closure,
        f.body(),
        &f.source()[..64],
        &index,
        f.entry_page(),
        0,
        &mut rows,
        &mut framing,
    )
    .unwrap();
    let spec = selected.tree_spec().unwrap();
    let mut siblings = [StrongDigest::ZERO; 32];
    let span = verify_leaf_span(
        &spec,
        selected.root(),
        0,
        payload,
        &[&[0]],
        &mut siblings,
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: 16384,
        },
    )
    .unwrap();
    let header = AuthenticatedRange::new(selected, &span, 0, 64).unwrap();
    let directory = AuthenticatedRange::new(selected, &span, 64, 128).unwrap();
    let chunk = AuthenticatedRange::new(selected, &span, 320, 192).unwrap();
    run(header, directory, chunk);
}
#[test]
fn selected_emf_chunk_uses_actual_outer_root_and_all_inner_digest_bytes() {
    let good = emf();
    authenticated(&good, |h, d, p| {
        use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
        let mut scratch = [PayloadInterval::default(); 2];
        let guard = AllocGuard::begin("emf-selective-admission", true);
        let view = EmfDirectoryView::admit(h, d, limits(), &mut scratch).unwrap();
        let chunk = view.selected_chunk(1, p).unwrap();
        assert_eq!(chunk.records().as_ptr(), good[320..].as_ptr());
        assert_eq!(chunk.records().len(), 128);
        assert!(chunk.cell(1).is_some());
        assert!(chunk.cell(2).is_none());
        guard.check().unwrap();
    });
    // The outer source is freshly signed for each altered file: the inner full
    // seal, not a failing outer root, must catch these corruptions.
    for byte in [448, 463, 464, 479, 480, 511] {
        let mut bad = good.clone();
        bad[byte] ^= 1;
        authenticated(&bad, |h, d, p| {
            let view =
                EmfDirectoryView::admit(h, d, limits(), &mut [PayloadInterval::default(); 2])
                    .unwrap();
            assert!(matches!(
                view.selected_chunk(1, p),
                Err(FormatError::Integrity)
            ));
        });
    }
}
#[test]
fn selective_emf_budgets_and_mismatched_range_placement_refuse() {
    authenticated(&emf(), |h, d, p| {
        assert!(matches!(
            EmfDirectoryView::admit(h, d, limits(), &mut [PayloadInterval::default(); 1]),
            Err(FormatError::Capacity)
        ));
        for limited in [
            FormatLimits {
                max_entry_bytes: 511,
                ..limits()
            },
            FormatLimits {
                max_rows: 1,
                ..limits()
            },
            FormatLimits {
                max_metadata_bytes: 191,
                ..limits()
            },
        ] {
            assert!(matches!(
                EmfDirectoryView::admit(h, d, limited, &mut [PayloadInterval::default(); 2]),
                Err(FormatError::Limit)
            ));
        }
        assert!(matches!(
            EmfDirectoryView::admit(d, h, limits(), &mut [PayloadInterval::default(); 2]),
            Err(FormatError::Bounds)
        ));
        let view =
            EmfDirectoryView::admit(h, d, limits(), &mut [PayloadInterval::default(); 2]).unwrap();
        assert!(matches!(
            view.selected_chunk(0, p),
            Err(FormatError::Bounds)
        ));
        assert!(matches!(
            view.selected_chunk(usize::MAX, p),
            Err(FormatError::Bounds)
        ));
        let view = EmfDirectoryView::admit(
            h,
            d,
            FormatLimits {
                max_chunk_bytes: 191,
                ..limits()
            },
            &mut [PayloadInterval::default(); 2],
        )
        .unwrap();
        assert!(matches!(view.selected_chunk(1, p), Err(FormatError::Limit)));
    });
}
#[test]
fn signed_overlapping_and_legacy_directories_refuse() {
    use crate::q42::emf_container::{
        chunk_directory::EmfChunkDirectoryEntry, header::EmfContainerHeader,
    };
    let mut bad = emf();
    let mut row: EmfChunkDirectoryEntry = bytemuck::pod_read_unaligned(&bad[128..192]);
    row.byte_offset = 192;
    row.update_crc();
    bad[128..192].copy_from_slice(bytemuck::bytes_of(&row));
    authenticated(&bad, |h, d, _| {
        assert!(matches!(
            EmfDirectoryView::admit(h, d, limits(), &mut [PayloadInterval::default(); 2]),
            Err(FormatError::Geometry)
        ));
    });
    let mut bad = emf();
    let mut h: EmfContainerHeader = bytemuck::pod_read_unaligned(&bad[..64]);
    h.version = 1;
    h.update_crc();
    bad[..64].copy_from_slice(bytemuck::bytes_of(&h));
    authenticated(&bad, |h, d, _| {
        assert!(matches!(
            EmfDirectoryView::admit(h, d, limits(), &mut [PayloadInterval::default(); 2]),
            Err(FormatError::Unsupported)
        ));
    });
}
