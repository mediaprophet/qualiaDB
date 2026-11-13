use super::coverage::{metadata_length, validate};
use super::*;
use crate::bundle::stream::index::EntryScratch;
use crate::bundle::{BundleReadLimits, BundleReader, BundleWriter};
use crate::crypto::network::object_stream::{MAX_METADATA, MAX_OBJECTS};

fn bundle(lengths: &[usize], aligned: bool) -> Vec<u8> {
    let mut writer = BundleWriter::new().with_page_align_4096(aligned);
    for (i, length) in lengths.iter().enumerate() {
        writer
            .add_file(
                format!("private-{i}"),
                "blob",
                vec![(i % 251) as u8; *length],
                None,
            )
            .unwrap();
    }
    writer.build().unwrap()
}
fn inspect(bytes: &[u8], count: usize, f: impl FnOnce(&PreparedSource<'_, '_>)) {
    let mut index = vec![EntryScratch::EMPTY; count];
    let mut entries = vec![EntryRange::EMPTY; count];
    let mut objects = vec![SourceObject::EMPTY; 2 * count + 1];
    let mut order = vec![0; 2 * count + 1];
    let source = PreparedSource::prepare(
        bytes,
        BundleReadLimits::default(),
        &mut index,
        &mut entries,
        &mut objects,
        &mut order,
    )
    .unwrap();
    f(&source);
}
#[test]
fn source_geometry_borrows_original_bytes_and_covers_every_byte_once() {
    let bytes = bundle(&[0, 3, 0, 64, 0, 2], false);
    let before = bytes.clone();
    let original = BundleReader::parse(&bytes).unwrap();
    inspect(&bytes, 6, |source| {
        assert_eq!(source.entry_count(), 6);
        assert_eq!(source.source_bytes(), bytes);
        assert_eq!(source.header_bytes().as_ptr(), bytes.as_ptr());
        let index_start = u64::from_le_bytes(bytes[12..20].try_into().unwrap()) as usize;
        assert_eq!(source.index_bytes(), &bytes[index_start..]);
        assert_eq!(
            source.metadata_length(),
            288 + 108 * source.objects().len() + 64 + source.index_bytes().len()
        );
        // Independent byte occupancy oracle, not another interval-frontier walk.
        let mut covered = vec![0u8; bytes.len()];
        covered[..64].fill(1);
        covered[index_start..].fill(1);
        let mut ordinals = vec![false; 6];
        for (i, row) in source.objects().iter().enumerate() {
            let start = row.offset() as usize;
            let end = start + row.length() as usize;
            assert_eq!(source.object_bytes(i).unwrap(), &bytes[start..end]);
            for value in &mut covered[start..end] {
                *value += 1;
            }
            if let ObjectKind::Entry(ordinal) = row.kind() {
                assert!(!ordinals[ordinal as usize]);
                ordinals[ordinal as usize] = true;
                let entry = &original.entries()[ordinal as usize];
                assert_eq!((row.offset(), row.length()), (entry.offset, entry.length));
            } else {
                assert!(row.length() > 0);
            }
        }
        assert!(covered.iter().all(|n| *n == 1));
        assert!(ordinals.iter().all(|v| *v));
        assert!(source.object_bytes(source.objects().len()).is_err());
    });
    assert_eq!(bytes, before);
}
#[test]
fn no_entries_and_all_empty_entries_keep_only_maximal_positive_gaps() {
    for aligned in [false, true] {
        let bytes = bundle(&[], aligned);
        inspect(&bytes, 0, |s| {
            assert_eq!(s.objects().len(), usize::from(aligned));
            if aligned {
                assert_eq!(
                    (s.objects()[0].offset(), s.objects()[0].length()),
                    (64, 4032)
                );
            }
        });
        let bytes = bundle(&[0; 4], aligned);
        inspect(&bytes, 4, |s| {
            assert_eq!(s.objects().len(), 4 + usize::from(aligned));
            for (ordinal, row) in s.objects()[..4].iter().enumerate() {
                assert!(row.kind() == ObjectKind::Entry(ordinal as u32));
                assert_eq!(row.length(), 0);
            }
            assert_eq!(
                s.objects()
                    .iter()
                    .filter(|r| r.kind() == ObjectKind::Gap)
                    .count(),
                usize::from(aligned)
            );
        });
    }
}
#[test]
fn maximum_original_entry_count_reaches_8193_objects_without_losing_padding() {
    let bytes = bundle(&vec![1; 4096], true);
    inspect(&bytes, 4096, |s| {
        assert_eq!(s.objects().len(), MAX_OBJECTS);
        assert_eq!(s.objects().len().div_ceil(128), 65);
        assert_eq!(
            s.objects()
                .iter()
                .filter(|r| r.kind() == ObjectKind::Gap)
                .count(),
            4097
        );
        for (ordinal, row) in s.objects()[..4096].iter().enumerate() {
            assert!(row.kind() == ObjectKind::Entry(ordinal as u32));
            assert_eq!(
                (row.offset(), row.length()),
                (4096 * (ordinal as u64 + 1), 1)
            );
        }
        assert_eq!(
            (s.objects()[4096].offset(), s.objects()[4096].length()),
            (64, 4032)
        );
        assert_eq!(s.objects()[MAX_OBJECTS - 1].length(), 4095);
        let payload_bytes: u64 = s.objects().iter().map(SourceObject::length).sum();
        assert_eq!(
            payload_bytes + 64 + s.index_bytes().len() as u64,
            bytes.len() as u64
        );
    });
}
#[test]
fn separate_partition_validator_rejects_missing_duplicate_overlap_hole_and_split_gap() {
    let e = [
        EntryRange::new(64, 192),
        EntryRange::new(128, 0),
        EntryRange::new(320, 64),
        EntryRange::new(448, 0),
    ];
    let valid = [
        SourceObject::entry(0, e[0]),
        SourceObject::entry(1, e[1]),
        SourceObject::entry(2, e[2]),
        SourceObject::entry(3, e[3]),
        SourceObject::gap(256, 64),
        SourceObject::gap(384, 128),
    ];
    let mut order = [0; 16];
    validate(512, &e, &valid, &mut order).unwrap();
    let mut reverse = valid;
    reverse.reverse();
    validate(512, &e, &reverse, &mut order).unwrap();
    let mut bad = valid.to_vec();
    bad.remove(1);
    assert!(validate(512, &e, &bad, &mut order).is_err());
    let mut bad = valid.to_vec();
    bad.push(valid[1]);
    assert!(validate(512, &e, &bad, &mut order).is_err());
    for gap in [
        SourceObject::gap(255, 65),
        SourceObject::gap(257, 63),
        SourceObject::gap(256, 0),
        SourceObject::gap(u64::MAX, 1),
    ] {
        let mut bad = valid;
        bad[4] = gap;
        assert!(validate(512, &e, &bad, &mut order).is_err());
    }
    let mut bad = valid.to_vec();
    bad[5] = SourceObject::gap(384, 64);
    bad.push(SourceObject::gap(448, 64));
    // An empty original row at448 cannot make these adjacent gaps maximal.
    assert!(validate(512, &e, &bad, &mut order).is_err());
    let mut bad = valid;
    bad[1] = SourceObject::entry(1, EntryRange::new(129, 0));
    assert!(validate(512, &e, &bad, &mut order).is_err());
}
#[test]
fn metadata_length_checks_exact_eight_mib_and_arithmetic_before_allocation() {
    let index_length = MAX_METADATA - (288 + 64 + 108 * MAX_OBJECTS as u64);
    assert_eq!(
        metadata_length(MAX_OBJECTS, index_length),
        Ok(MAX_METADATA as usize)
    );
    assert_eq!(
        metadata_length(MAX_OBJECTS, index_length + 1),
        Err(ProtectionError::Limit)
    );
    assert_eq!(
        metadata_length(MAX_OBJECTS + 1, 1),
        Err(ProtectionError::Limit)
    );
    assert_eq!(metadata_length(1, u64::MAX), Err(ProtectionError::Overflow));
    assert_eq!(metadata_length(0, 1), Ok(353));
}
#[test]
fn canonical_full_reader_rejects_crc_and_payload_hash_corruption() {
    let bytes = bundle(&[3, 3], false);
    for recompute_crc in [false, true] {
        let mut bad = bytes.clone();
        bad[64] ^= 1;
        if recompute_crc {
            bad[36..40].fill(0);
            let crc = crate::container_10d::crc32c::crc32c(&bad);
            bad[36..40].copy_from_slice(&crc.to_le_bytes());
        }
        let mut index = [EntryScratch::EMPTY; 2];
        let mut entries = [EntryRange::EMPTY; 2];
        let mut objects = [SourceObject::EMPTY; 5];
        let mut order = [0; 5];
        assert!(matches!(
            PreparedSource::prepare(
                &bad,
                BundleReadLimits::default(),
                &mut index,
                &mut entries,
                &mut objects,
                &mut order
            ),
            Err(ProtectionError::Source)
        ));
    }
}
#[test]
fn preparation_refuses_short_scratch_and_caller_limits_without_minting_a_result() {
    let bytes = bundle(&[3, 3], false);
    for which in 0..4 {
        let mut index = vec![EntryScratch::EMPTY; if which == 0 { 1 } else { 2 }];
        let mut entries = vec![EntryRange::EMPTY; if which == 1 { 1 } else { 2 }];
        let mut objects = vec![SourceObject::EMPTY; if which == 2 { 3 } else { 5 }];
        let mut order = vec![0; if which == 3 { 3 } else { 5 }];
        assert!(
            PreparedSource::prepare(
                &bytes,
                BundleReadLimits::default(),
                &mut index,
                &mut entries,
                &mut objects,
                &mut order
            )
            .is_err()
        );
    }
    for limits in [
        BundleReadLimits {
            file_bytes: bytes.len() - 1,
            ..Default::default()
        },
        BundleReadLimits {
            entries: 1,
            ..Default::default()
        },
        BundleReadLimits {
            index_bytes: 1,
            ..Default::default()
        },
    ] {
        let mut index = [EntryScratch::EMPTY; 2];
        let mut entries = [EntryRange::EMPTY; 2];
        let mut objects = [SourceObject::EMPTY; 5];
        let mut order = [0; 5];
        assert!(
            PreparedSource::prepare(
                &bytes,
                limits,
                &mut index,
                &mut entries,
                &mut objects,
                &mut order
            )
            .is_err()
        );
    }
}

#[test]
fn empty_original_inside_an_occupied_entry_does_not_reset_preparation_frontier() {
    let bytes = bundle(&[192, 0, 64], false);
    let mut rows = BundleReader::parse(&bytes).unwrap().entries().to_vec();
    rows[1].offset = 128;
    let index_start = u64::from_le_bytes(bytes[12..20].try_into().unwrap()) as usize;
    let mut index = Vec::new();
    ciborium::into_writer(&rows, &mut index).unwrap();
    let mut changed = bytes[..index_start].to_vec();
    changed.extend_from_slice(&index);
    changed[20..28].copy_from_slice(&(index.len() as u64).to_le_bytes());
    let total = changed.len() as u64;
    changed[28..36].copy_from_slice(&total.to_le_bytes());
    changed[36..40].fill(0);
    let crc = crate::container_10d::crc32c::crc32c(&changed);
    changed[36..40].copy_from_slice(&crc.to_le_bytes());
    inspect(&changed, 3, |s| {
        assert_eq!(s.objects().len(), 3);
        assert_eq!((s.objects()[1].offset(), s.objects()[1].length()), (128, 0));
        assert!(
            s.objects()
                .iter()
                .all(|r| matches!(r.kind(), ObjectKind::Entry(_)))
        );
    });
}
