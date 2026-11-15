#![cfg(not(target_arch = "wasm32"))]

use super::q42_ranges::Q42AuthenticatedRangeSource;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    formats::AuthenticatedRange,
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{encode_proof, verify_leaf_span, SpanLimits, TreeBuilder},
    selection::select_entry,
};
use crate::mini_parser::hash_token;
use crate::q42::{
    q42_volume::write_unified_volume,
    q42_volume::{Q42RangeSource, Q42RangeVolume},
};
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
use crate::NQuin;
use std::{collections::HashMap, io::Read};
use tempfile::NamedTempFile;

#[test]
fn actual_q42_range_reader_uses_only_authenticated_entry_bytes() {
    // Keep all borrowed selection/page state in this scope.
    let subject = hash_token("urn:q42:authenticated-subject");
    let predicate = hash_token("urn:q42:authenticated-predicate");
    let object = hash_token("urn:q42:authenticated-object");
    let quin = NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let mut lexicon = HashMap::new();
    lexicon.insert(subject, "urn:q42:authenticated-subject".to_string());
    lexicon.insert(predicate, "urn:q42:authenticated-predicate".to_string());
    lexicon.insert(object, "urn:q42:authenticated-object".to_string());
    let file = NamedTempFile::new().unwrap();
    write_unified_volume(file.path(), &lexicon, &[(object, object)], &[vec![quin]]).unwrap();
    let mut q42 = Vec::new();
    std::fs::File::open(file.path())
        .unwrap()
        .read_to_end(&mut q42)
        .unwrap();

    let fixture = Fixture::new_with_payload(&q42);
    let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
    let closure = fixture.bound(&pins, pin, publisher);
    let header = admit_header(
        &fixture.source()[..64],
        fixture.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut entry_scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &header,
        &fixture.source()[header.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut entry_scratch,
    )
    .unwrap();
    let selected = select_entry(
        &closure,
        fixture.body(),
        &fixture.source()[..64],
        &index,
        fixture.entry_page(),
        0,
        &mut [EntryRecord::EMPTY; 128],
        &mut vec![0; HASH_SCRATCH_BYTES],
    )
    .unwrap();
    assert_eq!(selected.logical_length(), q42.len() as u64);
    assert!(q42.len() <= crate::bundle::stream::proof::LEAF_BYTES as usize);

    let mut tree = TreeBuilder::new(selected.tree_spec().unwrap());
    tree.append(&q42, |_| Ok(())).unwrap();
    assert_eq!(tree.finish(|_| Ok(())).unwrap(), selected.root());
    let mut proof = [0u8; 1];
    encode_proof(&[], &mut proof).unwrap();
    let span = verify_leaf_span(
        &selected.tree_spec().unwrap(),
        selected.root(),
        0,
        &q42,
        &[&proof],
        &mut [],
        SpanLimits {
            max_leaves: 1,
            max_encoded_bytes: q42.len(),
        },
    )
    .unwrap();

    let first = q42.len().min(128);
    let second = q42.len().min(256);
    let a = AuthenticatedRange::new(selected, &span, 0, first).unwrap();
    let b = AuthenticatedRange::new(selected, &span, first as u64, second - first).unwrap();
    let c = AuthenticatedRange::new(selected, &span, second as u64, q42.len() - second).unwrap();
    let ranges = [a, b, c];
    let source = Q42AuthenticatedRangeSource::new(selected, &ranges, q42.len() as u64, 3).unwrap();
    assert_eq!(source.length().unwrap(), q42.len() as u64);
    assert!(Q42AuthenticatedRangeSource::new(selected, &ranges, q42.len() as u64 - 1, 3).is_err());
    assert!(Q42AuthenticatedRangeSource::new(selected, &ranges, q42.len() as u64, 2).is_err());
    let overlap = AuthenticatedRange::new(selected, &span, 100, 8).unwrap();
    assert!(
        Q42AuthenticatedRangeSource::new(selected, &[a, overlap], q42.len() as u64, 2).is_err()
    );

    let volume = Q42RangeVolume::open(source).unwrap();
    assert_eq!(volume.block_count(), 1);
    let mut compressed = [0u8; crate::q42::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; crate::q42::q42_volume::SUPERBLOCK_SIZE];
    assert_eq!(
        volume
            .read_superblock_into(0, &mut compressed, &mut decoded)
            .unwrap(),
        crate::q42::q42_volume::SUPERBLOCK_SIZE
    );
    assert_eq!(u64::from_le_bytes(decoded[16..24].try_into().unwrap()), 1);
    assert_eq!(
        u64::from_le_bytes(decoded[160..168].try_into().unwrap()),
        quin.subject
    );
    let source = volume.into_source();

    // A disjoint authenticated collection can be constructed, but a request
    // crossing its gap fails atomically instead of synthesizing zero bytes.
    let left = AuthenticatedRange::new(selected, &span, 0, 8).unwrap();
    let right_start = q42.len().saturating_sub(8);
    let right =
        AuthenticatedRange::new(selected, &span, right_start as u64, q42.len() - right_start)
            .unwrap();
    let sparse_ranges = [left, right];
    let sparse =
        Q42AuthenticatedRangeSource::new(selected, &sparse_ranges, q42.len() as u64, 2).unwrap();
    let mut untouched = [0xa5; 4];
    assert_eq!(
        sparse.read_exact_at(8, &mut untouched).unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
    assert_eq!(untouched, [0xa5; 4]);

    let mut copied = [0u8; 24];
    assert_zero_alloc("q42_authenticated_range_copy", || {
        source.read_exact_at(0, &mut copied).unwrap();
    });
    assert_eq!(&copied, &q42[..24]);
}
