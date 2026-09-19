//! Integration tests for companion operator package (W2: EOS-023).

use tempfile::tempdir;
use qualia_inference_kernel::operators::{
    AccumKind, OperatorDescriptor, OperatorKind, ScaleLayout, Q4K_SUPERBLOCK_BYTES,
    Q4K_SUPERBLOCK_ELEMS,
};
use crate::q42::p64_weight::{P64_FLAG_Q4K_SOA, P64_MAGIC, P64_VERSION};
use super::manifest::{FidelityContract, OperatorRecord, PackageError};
use super::q4k_repack::{reconstruct_source_q4k, repack_q4k_tensor};
use super::segment::{SegmentError, SegmentKind};
use super::{load_package_from_file, OperatorPackage, PackageBuilder};

fn make_synthetic_q4k_tensor(n_blocks: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity(n_blocks * Q4K_SUPERBLOCK_BYTES);
    for b in 0..n_blocks {
        let mut blk = [0u8; Q4K_SUPERBLOCK_BYTES];
        blk[0] = 0x00; // d = 1.0 (f16)
        blk[1] = 0x3c;
        blk[2] = 0x00; // dmin = 0.1 (f16)
        blk[3] = 0x2e;
        for i in 4..16 {
            blk[i] = ((b + i) & 0x3f) as u8;
        }
        for i in 16..144 {
            blk[i] = (b as u8).wrapping_add(i as u8);
        }
        raw.extend_from_slice(&blk);
    }
    raw
}

#[test]
fn companion_package_round_trip_and_source_preservation() {
    let n_blocks = 4;
    let n_elems = n_blocks * Q4K_SUPERBLOCK_ELEMS as usize;
    let raw_source = make_synthetic_q4k_tensor(n_blocks);

    let repacked = repack_q4k_tensor(&raw_source, n_elems).expect("repack must succeed");

    let mut builder = PackageBuilder::new(
        repacked.source_digest,
        repacked.representation_digest,
        FidelityContract::SourceBytePreserving,
    );

    builder
        .add_segment_payload(1, SegmentKind::SourcePayload, repacked.source_bytes.clone())
        .expect("add source segment");
    builder
        .add_segment_payload(2, SegmentKind::ScaleMinPlane, repacked.scale_plane.clone())
        .expect("add scale segment");
    builder
        .add_segment_payload(3, SegmentKind::BitPlaneTiles, repacked.bitplane_tiles.clone())
        .expect("add bitplane segment");

    let desc = OperatorDescriptor {
        kind: OperatorKind::Q4KBitPlane,
        in_features: 256,
        out_features: n_blocks as u32,
        batch_hint: 1,
        tile_elems: Q4K_SUPERBLOCK_ELEMS,
        scale_layout: ScaleLayout::GgmlQ4K,
        accum: AccumKind::F32,
        max_workspace_bytes: 0,
        representation_digest: repacked.representation_digest,
    };

    builder
        .add_operator(OperatorRecord {
            tensor_role: 1,
            name: "test.q4k_tensor".to_string(),
            descriptor: desc,
            source_segment_id: 1,
            primary_segment_id: 3,
            scale_segment_id: 2,
        })
        .expect("add operator");

    let pkg_bytes = builder.build().expect("package build must succeed");
    let pkg = OperatorPackage::from_bytes(pkg_bytes).expect("package load must succeed");

    assert_eq!(pkg.manifest().magic, *b"QOP1");
    assert_eq!(pkg.manifest().version, 1);
    assert_eq!(pkg.manifest().source_digest, repacked.source_digest);
    assert_eq!(pkg.manifest().representation_digest, repacked.representation_digest);
    assert_eq!(pkg.manifest().fidelity_contract, FidelityContract::SourceBytePreserving);

    // Verify Segment 1 (retained source) matches bit-for-bit
    let src_view = pkg.get_segment_view(1).expect("source view");
    assert_eq!(src_view.as_slice(), &raw_source);

    // Verify Segment 2 (scales) + Segment 3 (bitplane) reconstruct exact source bytes
    let scale_view = pkg.get_segment_view(2).expect("scale view");
    let bit_view = pkg.get_segment_view(3).expect("bitplane view");

    let mut reconstructed = vec![0u8; raw_source.len()];
    reconstruct_source_q4k(
        scale_view.as_slice(),
        bit_view.as_slice(),
        n_blocks,
        &mut reconstructed,
    )
    .expect("reconstruct source");

    assert_eq!(reconstructed, raw_source, "source-byte preservation contract violated");
}

#[test]
fn raii_tempfile_package_lifecycle_and_cleanup() {
    let dir = tempdir().expect("create temp dir");
    let file_path = dir.path().join("test_companion.qop");

    let raw_source = make_synthetic_q4k_tensor(2);
    let repacked = repack_q4k_tensor(&raw_source, 512).unwrap();

    let mut builder = PackageBuilder::new(
        repacked.source_digest,
        repacked.representation_digest,
        FidelityContract::SourceBytePreserving,
    );
    builder
        .add_segment_payload(1, SegmentKind::SourcePayload, repacked.source_bytes)
        .unwrap();

    builder.write_to_file(&file_path).expect("write to file");
    assert!(file_path.exists());

    let loaded = load_package_from_file(&file_path).expect("load from file");
    assert_eq!(loaded.manifest().source_digest, repacked.source_digest);

    let path_copy = file_path.clone();
    drop(dir); // RAII cleanup

    assert!(!path_copy.exists(), "temp file must be cleaned up on tempdir drop");
}

#[test]
fn malformed_packages_fail_closed() {
    let raw_source = make_synthetic_q4k_tensor(1);
    let repacked = repack_q4k_tensor(&raw_source, 256).unwrap();

    let mut builder = PackageBuilder::new(
        repacked.source_digest,
        repacked.representation_digest,
        FidelityContract::SourceBytePreserving,
    );
    builder
        .add_segment_payload(1, SegmentKind::SourcePayload, repacked.source_bytes)
        .unwrap();

    let valid_bytes = builder.build().unwrap();

    // 1. Corrupt magic
    let mut bad_magic = valid_bytes.clone();
    bad_magic[0] = b'B';
    bad_magic[1] = b'A';
    bad_magic[2] = b'D';
    bad_magic[3] = b'!';
    assert!(matches!(
        OperatorPackage::from_bytes(bad_magic),
        Err(PackageError::InvalidMagic(_))
    ));

    // 2. Unsupported version
    let mut bad_version = valid_bytes.clone();
    bad_version[4] = 99;
    assert!(matches!(
        OperatorPackage::from_bytes(bad_version),
        Err(PackageError::UnsupportedVersion(99))
    ));

    // 3. Truncated package payload
    let valid_pkg = OperatorPackage::from_bytes(valid_bytes.clone()).unwrap();
    let seg_desc = valid_pkg.manifest().find_segment(1).unwrap();
    let truncated = valid_bytes[..(seg_desc.offset as usize + 20)].to_vec();
    assert!(matches!(
        OperatorPackage::from_bytes(truncated),
        Err(PackageError::TruncatedInput)
    ));

    // 4. Corrupt segment checksum
    let pkg = valid_pkg;
    let seg_desc = pkg.manifest().find_segment(1).unwrap();
    let mut tampered = valid_bytes.clone();
    // mutate a byte in the payload
    tampered[seg_desc.offset as usize] ^= 0xff;
    let tampered_pkg = OperatorPackage::from_bytes(tampered).unwrap();
    assert!(matches!(
        tampered_pkg.get_segment_view(1),
        Err(SegmentError::ChecksumMismatch { .. })
    ));
}

#[test]
fn p64_immovables_are_preserved() {
    // Assert that P64 v4 header format constants remain untouched and frozen
    assert_eq!(P64_MAGIC, *b"p64\0");
    assert_eq!(P64_VERSION, 4);
    assert_eq!(P64_FLAG_Q4K_SOA, 1 << 3);
}
