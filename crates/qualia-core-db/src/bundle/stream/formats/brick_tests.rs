//! Complete scientific brick admission with a deliberately unfetched entry suffix.
use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{encode_proof, verify_leaf_span, OutboardNode, SpanLimits, TreeBuilder},
    selection::select_entry,
};
use crate::net::qdnf::types::StrongDigest as D;
use crate::q42::emf_container::{header::EmfCellRecord64, writer::EmfContainerWriter};
fn emf() -> Vec<u8> {
    let mut w = EmfContainerWriter::new([0.; 3], 1., [2, 1, 1], 0.);
    let cells = [EmfCellRecord64::default(); 512];
    w.add_brick_chunk(0, &cells, 0, 1, 0).unwrap();
    w.add_brick_chunk(1, &cells, 0, 1, 0).unwrap();
    w.to_bytes().unwrap()
}
fn check(bytes: &[u8], valid: bool) {
    assert_eq!(bytes.len(), 65856);
    let f = Fixture::new_with_payload(bytes);
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let closure = f.bound(&pins, pin, publisher);
    let h = admit_header(
        &f.source()[..64],
        f.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut idx = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &h,
        &f.source()[h.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut idx,
    )
    .unwrap();
    let selected = select_entry(
        &closure,
        f.body(),
        &f.source()[..64],
        &index,
        f.entry_page(),
        0,
        &mut [EntryRecord::EMPTY; 128],
        &mut vec![0; HASH_SCRATCH_BYTES],
    )
    .unwrap();
    let spec = selected.tree_spec().unwrap();
    let mut tree = TreeBuilder::new(spec);
    let mut nodes: Vec<OutboardNode> = Vec::new();
    for leaf in bytes.chunks(16384) {
        tree.append(leaf, |n| {
            nodes.push(n);
            Ok(())
        })
        .unwrap();
    }
    assert_eq!(
        tree.finish(|n| {
            nodes.push(n);
            Ok(())
        })
        .unwrap(),
        selected.root()
    );
    let node = |start, count| {
        nodes
            .iter()
            .find(|n| n.start_leaf == start && n.leaf_count == count)
            .unwrap()
            .digest
    };
    let paths = [
        [node(1, 1), node(2, 2), node(4, 1)],
        [node(0, 1), node(2, 2), node(4, 1)],
        [node(3, 1), node(0, 2), node(4, 1)],
    ];
    let mut encoded = [[0; 145]; 3];
    for (path, out) in paths.iter().zip(&mut encoded) {
        assert_eq!(encode_proof(path, out).unwrap(), 145);
    }
    // The receiving side has only three full leaves, not the last two. The first
    // brick plus its entire BLAKE3 trailer is within this authenticated prefix.
    let received = &bytes[..49152];
    let proofs: [&[u8]; 3] = [&encoded[0], &encoded[1], &encoded[2]];
    let span = verify_leaf_span(
        &spec,
        selected.root(),
        0,
        received,
        &proofs,
        &mut [D::ZERO; 32],
        SpanLimits {
            max_leaves: 3,
            max_encoded_bytes: 49152,
        },
    )
    .unwrap();
    let header = AuthenticatedRange::new(selected, &span, 0, 64).unwrap();
    let directory = AuthenticatedRange::new(selected, &span, 64, 128).unwrap();
    let brick = AuthenticatedRange::new(selected, &span, 192, 32832).unwrap();
    assert!(AuthenticatedRange::new(selected, &span, 33024, 32832).is_err());
    let limits = FormatLimits {
        max_entry_bytes: 65856,
        max_rows: 2,
        max_metadata_bytes: 192,
        max_chunk_bytes: 32832,
    };
    use crate::specialized_libs::computational_geometry::allocation_counter::{
        AllocGuard, AllocSnapshot,
    };
    let before = AllocSnapshot::now();
    let guard = AllocGuard::begin("complete-scientific-brick", true);
    let view = EmfDirectoryView::admit(
        header,
        directory,
        limits,
        &mut [PayloadInterval::default(); 2],
    )
    .unwrap();
    let result = view.selected_chunk(0, brick);
    if valid {
        let brick = result.unwrap();
        assert_eq!(brick.records().len(), 32768);
        assert_eq!(brick.records().as_ptr(), received[192..].as_ptr());
        assert!(brick.cell(511).is_some());
        assert!(brick.cell(512).is_none());
    } else {
        assert!(matches!(result, Err(FormatError::Integrity)));
    }
    let calls = AllocSnapshot::now().alloc_delta(before);
    guard.check().unwrap();
    assert_eq!(calls, 0);
}
#[test]
fn complete_hilbert_brick_needs_no_unfetched_entry_suffix() {
    check(&emf(), true);
}
#[test]
fn complete_brick_last_trailer_byte_is_checked_under_a_fresh_valid_outer_root() {
    for offset in [32991, 33023] {
        let mut bytes = emf();
        bytes[offset] ^= 1;
        check(&bytes, false);
    }
}
