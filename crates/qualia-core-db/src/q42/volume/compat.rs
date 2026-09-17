//! Migration / compatibility matrix and the lossless Monarch-class lexicon gate.
//!
//! These tests are the format-contract suite for the v3 header extensions
//! (field ranges, compact postings, volume roots). They must stay green when
//! the on-disk layout changes.

use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};

use super::super::{
    migrate_v2_to_v3, write_unified_volume, write_volume_root, write_volume_root_with_lex,
    Q42Volume, Q42VolumeManifest, StreamingQ42VolumeWriter, HEADER_SIZE, Q42_MAGIC,
};
use crate::q42_lex::Q42LexMmap;
use crate::NQuin;
use tempfile::NamedTempFile;

fn quin(subject: u64, predicate: u64, object: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context: 0,
        metadata: 0,
        parity: subject ^ predicate ^ object,
    }
}

#[test]
fn v2_header_migrates_and_remains_readable() {
    let file = NamedTempFile::new().unwrap();
    let mut lex = HashMap::new();
    lex.insert(1, "s".into());
    lex.insert(2, "p".into());
    lex.insert(3, "o".into());
    write_unified_volume(file.path(), &lex, &[(3, 3)], &[vec![quin(1, 2, 3)]]).unwrap();

    let mut raw = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(file.path())
        .unwrap();
    let mut header = [0u8; HEADER_SIZE];
    std::io::Read::read_exact(&mut raw, &mut header).unwrap();
    header[4..6].copy_from_slice(&2u16.to_le_bytes());
    raw.seek(SeekFrom::Start(0)).unwrap();
    raw.write_all(&header).unwrap();
    drop(raw);

    migrate_v2_to_v3(file.path()).unwrap();
    let volume = Q42Volume::open(file.path()).unwrap();
    assert_eq!(volume.block_count(), 1);
    assert_eq!(&std::fs::read(file.path()).unwrap()[0..4], &Q42_MAGIC);
}

#[test]
fn v3_without_postings_still_opens() {
    // Synthesize a pre-PIDX v3 file: write a good volume, then clear the
    // postings flag so older shipped catalogs remain a supported open path.
    let file = NamedTempFile::new().unwrap();
    let mut lex = HashMap::new();
    lex.insert(1, "s".into());
    lex.insert(2, "p".into());
    lex.insert(3, "o".into());
    write_unified_volume(file.path(), &lex, &[(3, 3)], &[vec![quin(1, 2, 3)]]).unwrap();
    let mut raw = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(file.path())
        .unwrap();
    let mut header = [0u8; HEADER_SIZE];
    std::io::Read::read_exact(&mut raw, &mut header).unwrap();
    let flags = u16::from_le_bytes([header[6], header[7]]) & !super::super::FLAG_FIELD_POSTINGS;
    header[6..8].copy_from_slice(&flags.to_le_bytes());
    raw.seek(SeekFrom::Start(0)).unwrap();
    raw.write_all(&header).unwrap();
    drop(raw);

    let volume = Q42Volume::open(file.path()).expect("v3 without PIDX flag must stay readable");
    assert!(volume.block_count() >= 1);
    assert_eq!(volume.header().flags & super::super::FLAG_FIELD_POSTINGS, 0);

    let reprocessed =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/data/dublincore/dct.q42");
    let dct = Q42Volume::open(&reprocessed).expect("reprocessed dct.q42 must stay readable");
    assert!(dct.block_count() >= 1);
    assert_ne!(dct.header().flags & super::super::FLAG_FIELD_POSTINGS, 0);
}

#[test]
fn v3_with_postings_opens_and_prunes() {
    let file = NamedTempFile::new().unwrap();
    let mut lex = HashMap::new();
    lex.insert(1, "http://ex/s1".into());
    lex.insert(2, "http://ex/p".into());
    lex.insert(3, "http://ex/o1".into());
    lex.insert(4, "http://ex/s2".into());
    lex.insert(5, "http://ex/o2".into());
    let mut writer = StreamingQ42VolumeWriter::new(&lex).unwrap();
    writer.push_block(0, &[quin(1, 2, 3)]).unwrap();
    writer.push_block(1, &[quin(4, 2, 5)]).unwrap();
    writer.finish(file.path()).unwrap();
    let volume = Q42Volume::open(file.path()).unwrap();
    let flags = volume.header().flags;
    assert_ne!(flags & super::super::FLAG_FIELD_POSTINGS, 0);
    let source = super::LocalFileRangeSource::open(file.path()).unwrap();
    let range = super::Q42RangeVolume::open(source).unwrap();
    let mut compressed = [0u8; crate::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
    let mut out = [NQuin::default(); 8];
    let page = range
        .execute_query_page_into(
            super::Q42RangeQueryPlan::for_pattern(super::Q42RangeQueryPattern {
                subject: Some(1),
                predicate: Some(2),
                object: None,
                context: None,
            }),
            super::Q42RangeQueryCursor::default(),
            &mut compressed,
            &mut decoded,
            &mut out,
        )
        .unwrap();
    assert_eq!(page.returned, 1);
    assert_eq!(out[0].subject, 1);
}

#[test]
fn implicit_one_segment_and_manifest_root_are_compatible() {
    let dir = tempfile::TempDir::new().unwrap();
    let child = dir.path().join("child.q42");
    let root = dir.path().join("root.q42");
    write_unified_volume(
        child.as_path(),
        &HashMap::new(),
        &[(3, 3)],
        &[vec![quin(1, 2, 3)]],
    )
    .unwrap();
    let bare = Q42Volume::open(&child).unwrap();
    assert_eq!(bare.block_count(), 1);

    let manifest = Q42VolumeManifest {
        generation: 1,
        segments: vec![Q42VolumeManifest::segment_from_file(&child, "child.q42".into()).unwrap()],
        lexicon_segments: Vec::new(),
    };
    write_volume_root(&root, &manifest).unwrap();
    let root_volume = Q42Volume::open(&root).unwrap();
    let flags = root_volume.header().flags;
    assert_ne!(flags & super::super::FLAG_VOLUME_ROOT, 0);
}

#[test]
fn lossless_monarch_class_lexicon_gate() {
    // A generated Monarch-class shard: many unique IRIs, complete embedded
    // Q42LEX, every used handle reverses. This is the format gate; the 5.4 GB
    // Monarch file is an operational corpus, not a unit-test fixture.
    let mut lex = HashMap::new();
    let mut block = Vec::new();
    for i in 0..128u64 {
        let subject = 10_000 + i;
        let predicate = 20_000 + (i % 7);
        let object = 30_000 + i;
        lex.insert(
            subject,
            format!("https://monarchinitiative.org/subject/{i}"),
        );
        lex.insert(
            predicate,
            format!("https://w3id.org/biolink/vocab/pred/{i}"),
        );
        lex.insert(object, format!("https://monarchinitiative.org/object/{i}"));
        block.push(quin(subject, predicate, object));
    }
    let file = NamedTempFile::new().unwrap();
    let mut writer = StreamingQ42VolumeWriter::new(&lex).unwrap();
    writer.push_block(0, &block).unwrap();
    writer.finish(file.path()).unwrap();

    let bytes = std::fs::read(file.path()).unwrap();
    let volume = Q42Volume::open(file.path()).unwrap();
    let lex_offset = volume.header().lex_offset;
    let lex_length = volume.header().lex_length;
    assert!(
        lex_length > 32,
        "lexicon must not be the empty 32-byte stub"
    );
    let lex_start = lex_offset as usize;
    let lex_end = lex_start + lex_length as usize;
    let mmap = Q42LexMmap::from_bytes(&bytes[lex_start..lex_end]).unwrap();
    for (hash, iri) in &lex {
        let resolved = mmap
            .lookup_hash(*hash)
            .unwrap_or_else(|| panic!("missing lexicon entry for {hash}"));
        assert_eq!(resolved, iri.as_str());
    }
}

#[test]
fn volume_root_can_carry_the_shared_lexicon() {
    let dir = tempfile::TempDir::new().unwrap();
    let child = dir.path().join("child.q42");
    let root = dir.path().join("root.q42");
    let mut lex = HashMap::new();
    lex.insert(1, "https://ex/s".into());
    lex.insert(2, "https://ex/p".into());
    lex.insert(3, "https://ex/o".into());
    write_unified_volume(&child, &HashMap::new(), &[(3, 3)], &[vec![quin(1, 2, 3)]]).unwrap();
    let manifest = Q42VolumeManifest {
        generation: 2,
        segments: vec![Q42VolumeManifest::segment_from_file(&child, "child.q42".into()).unwrap()],
        lexicon_segments: Vec::new(),
    };
    write_volume_root_with_lex(&root, &lex, &manifest).unwrap();
    let volume = Q42Volume::open(&root).unwrap();
    let lex_length = volume.header().lex_length;
    assert!(lex_length > 32);
}
