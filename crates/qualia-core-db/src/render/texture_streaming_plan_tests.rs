use super::*;
use crate::render::texture_ktx2::{Ktx2Document, KTX2_IDENTIFIER};

const BUDGET: TextureMipBudget = TextureMipBudget {
    decoded_cpu_bytes: 5_000_000,
    gpu_resident_bytes: 5_000_000,
    upload_staging_bytes: 5_000_000,
    per_frame_upload_bytes: 5_000_000,
};

fn mip(level: u32, width: u32, height: u32, bytes: u64) -> TextureMipCandidate {
    TextureMipCandidate {
        level,
        width,
        height,
        is_srgb: false,
        decoded_cpu_bytes: bytes,
        gpu_resident_bytes: bytes,
        upload_staging_bytes: bytes,
        upload_bytes: bytes,
    }
}

fn put32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn ktx2_multilevel_fixture(
    vk_format: u32,
    base_width: u32,
    base_height: u32,
    levels: &[&[u8]],
) -> Vec<u8> {
    let level_count = levels.len();
    let index_bytes = level_count * 24;
    let dfd_offset = 80 + index_bytes;
    let dfd_len = 28usize;
    let metadata_end = dfd_offset + dfd_len;

    let mut current_offset = metadata_end;
    let mut level_ranges = Vec::new();
    for payload in levels {
        let start = current_offset;
        let end = start + payload.len();
        level_ranges.push((start, payload.len()));
        current_offset = end;
    }

    let mut bytes = vec![0u8; current_offset];
    bytes[..12].copy_from_slice(&KTX2_IDENTIFIER);
    put32(&mut bytes, 12, vk_format);
    put32(&mut bytes, 16, 1); // type_size
    put32(&mut bytes, 20, base_width);
    put32(&mut bytes, 24, base_height);
    put32(&mut bytes, 28, 0); // depth
    put32(&mut bytes, 32, 0); // layer_count
    put32(&mut bytes, 36, 1); // face_count
    put32(&mut bytes, 40, level_count as u32);
    put32(&mut bytes, 44, 0); // supercompression_scheme = 0
    put32(&mut bytes, 48, dfd_offset as u32);
    put32(&mut bytes, 52, dfd_len as u32);

    put32(&mut bytes, dfd_offset, 28);
    put32(&mut bytes, dfd_offset + 8, (24 << 16) | 2);

    for (i, &(offset, len)) in level_ranges.iter().enumerate() {
        let entry_offset = 80 + i * 24;
        put64(&mut bytes, entry_offset, offset as u64);
        put64(&mut bytes, entry_offset + 8, len as u64);
        put64(&mut bytes, entry_offset + 16, len as u64);
        bytes[offset..offset + len].copy_from_slice(levels[i]);
    }

    bytes
}

#[test]
fn chooses_smallest_level_that_covers_projected_footprint() {
    let levels = [
        mip(0, 1024, 1024, 4_194_304),
        mip(1, 512, 512, 1_048_576),
        mip(2, 256, 256, 262_144),
        mip(3, 128, 128, 65_536),
    ];
    let selected = select_texture_mip(&levels, 300, 240, BUDGET).unwrap();
    assert_eq!(selected.candidate.level, 1);
    assert!(selected.meets_footprint);
}

#[test]
fn degrades_to_best_affordable_level_when_footprint_cannot_fit() {
    let levels = [
        mip(0, 1024, 1024, 4_194_304),
        mip(1, 512, 512, 1_048_576),
        mip(2, 256, 256, 262_144),
        mip(3, 128, 128, 65_536),
    ];
    let constrained = TextureMipBudget {
        decoded_cpu_bytes: 300_000,
        gpu_resident_bytes: 300_000,
        upload_staging_bytes: 300_000,
        per_frame_upload_bytes: 300_000,
    };
    let selected = select_texture_mip(&levels, 300, 240, constrained).unwrap();
    assert_eq!(selected.candidate.level, 2);
    assert!(!selected.meets_footprint);
}

#[test]
fn applies_each_budget_and_defers_when_none_fit() {
    let level = mip(0, 128, 128, 65_536);
    for constrained in [
        TextureMipBudget {
            decoded_cpu_bytes: 65_535,
            ..BUDGET
        },
        TextureMipBudget {
            gpu_resident_bytes: 65_535,
            ..BUDGET
        },
        TextureMipBudget {
            upload_staging_bytes: 65_535,
            ..BUDGET
        },
        TextureMipBudget {
            per_frame_upload_bytes: 65_535,
            ..BUDGET
        },
    ] {
        assert_eq!(select_texture_mip(&[level], 64, 64, constrained), None);
    }
}

#[test]
fn selection_is_independent_of_input_order() {
    let coarse = mip(2, 256, 256, 262_144);
    let adequate = mip(1, 512, 512, 1_048_576);
    let fine = mip(0, 1024, 1024, 4_194_304);
    let forward = [fine, adequate, coarse];
    let reverse = [coarse, adequate, fine];
    assert_eq!(
        select_texture_mip(&forward, 300, 240, BUDGET),
        select_texture_mip(&reverse, 300, 240, BUDGET)
    );
}

#[test]
fn fallback_prefers_balanced_projected_coverage_over_raw_area() {
    let wide_and_thin = mip(0, 4096, 1, 4096);
    let balanced = mip(1, 100, 100, 10_000);
    let selected = select_texture_mip(&[wide_and_thin, balanced], 200, 200, BUDGET).unwrap();
    assert_eq!(selected.candidate.level, 1);
    assert!(!selected.meets_footprint);
}

#[test]
fn duplicate_level_ties_are_stable_across_input_order() {
    let first = mip(2, 256, 256, 262_144);
    let duplicate = TextureMipCandidate {
        decoded_cpu_bytes: 250_000,
        ..first
    };
    assert_eq!(
        select_texture_mip(&[first, duplicate], 128, 128, BUDGET),
        select_texture_mip(&[duplicate, first], 128, 128, BUDGET)
    );
}

#[test]
fn rejects_zero_projected_dimensions_and_ignores_zero_sized_levels() {
    let invalid = mip(0, 0, 128, 1);
    assert_eq!(select_texture_mip(&[invalid], 64, 64, BUDGET), None);
    assert_eq!(select_texture_mip(&[], 0, 64, BUDGET), None);
    assert_eq!(select_texture_mip(&[], 64, 0, BUDGET), None);
}

#[test]
fn mip_extent_clamps_small_and_deep_levels_to_one() {
    assert_eq!(super::mip_extent(1, 1), 1);
    assert_eq!(super::mip_extent(1024, 10), 1);
    assert_eq!(super::mip_extent(1024, 32), 1);
    assert_eq!(super::mip_extent(0, 0), 1);
}

#[test]
fn reservation_is_atomic_and_prevents_frame_overcommit() {
    let first = mip(2, 128, 128, 65_536);
    let second = mip(3, 64, 64, 16_384);
    let mut remaining = TextureMipBudget {
        decoded_cpu_bytes: 80_000,
        gpu_resident_bytes: 80_000,
        upload_staging_bytes: 80_000,
        per_frame_upload_bytes: 70_000,
    };
    assert!(reserve_texture_mip(first, &mut remaining));
    let after_first = remaining;
    assert!(!reserve_texture_mip(second, &mut remaining));
    assert_eq!(remaining, after_first);
    assert_eq!(remaining.per_frame_upload_bytes, 4_464);
}

#[test]
fn builds_candidate_and_decodes_multilevel_ktx2() {
    // 2x2 base level (16 bytes), 1x1 mip 1 (4 bytes)
    let level0_bytes = [
        10, 11, 12, 13, 20, 21, 22, 23, 30, 31, 32, 33, 40, 41, 42, 43,
    ];
    let level1_bytes = [99, 98, 97, 96];
    let ktx2_bytes = ktx2_multilevel_fixture(43, 2, 2, &[&level0_bytes, &level1_bytes]);
    let doc = Ktx2Document::parse(&ktx2_bytes).expect("valid KTX2 fixture");

    let backend = TextureMipBackendCost {
        gpu_resident_bytes: 256,
        upload_staging_bytes: 128,
    };

    // Candidate for level 0
    let cand0 = rgba8_ktx2_mip_candidate(&doc, 0, backend).unwrap();
    assert_eq!(cand0.level, 0);
    assert_eq!(cand0.width, 2);
    assert_eq!(cand0.height, 2);
    assert!(cand0.is_srgb);
    assert_eq!(cand0.decoded_cpu_bytes, 16);
    assert_eq!(cand0.gpu_resident_bytes, 256);
    assert_eq!(cand0.upload_staging_bytes, 128);
    assert_eq!(cand0.upload_bytes, 16);

    // Candidate for level 1
    let cand1 = rgba8_ktx2_mip_candidate(&doc, 1, backend).unwrap();
    assert_eq!(cand1.level, 1);
    assert_eq!(cand1.width, 1);
    assert_eq!(cand1.height, 1);
    assert!(cand1.is_srgb);
    assert_eq!(cand1.decoded_cpu_bytes, 4);

    // Level out of range
    assert_eq!(
        rgba8_ktx2_mip_candidate(&doc, 2, backend),
        Err(Ktx2MipCandidateError::InvalidLevel)
    );

    // Decode level 1 directly into caller buffer
    let mut out1 = [0u8; 8];
    let decoded1 = decode_rgba8_ktx2_mip_into(&doc, 1, &mut out1).unwrap();
    assert_eq!(decoded1.level, 1);
    assert_eq!(decoded1.width, 1);
    assert_eq!(decoded1.height, 1);
    assert!(decoded1.is_srgb);
    assert_eq!(decoded1.bytes_written, 4);
    assert_eq!(&out1[..4], &level1_bytes);
    assert_eq!(&out1[4..], &[0, 0, 0, 0]); // Suffix untouched

    // Decode level 0 directly into caller buffer
    let mut out0 = [0u8; 16];
    let decoded0 = decode_rgba8_ktx2_mip_into(&doc, 0, &mut out0).unwrap();
    assert_eq!(decoded0.level, 0);
    assert_eq!(decoded0.bytes_written, 16);
    assert_eq!(&out0, &level0_bytes);

    // Output too small leaves buffer untouched
    let mut small = [0xEEu8; 3];
    let err = decode_rgba8_ktx2_mip_into(&doc, 1, &mut small);
    assert_eq!(
        err,
        Err(Ktx2MipDecodeError::OutputTooSmall {
            required: 4,
            available: 3
        })
    );
    assert_eq!(small, [0xEE; 3]);
}

#[test]
fn rejects_unsupported_ktx2_formats_and_shapes() {
    let payload = [1, 2, 3, 4];
    let backend = TextureMipBackendCost {
        gpu_resident_bytes: 0,
        upload_staging_bytes: 0,
    };

    // Unsupported format (e.g. 97)
    let bytes = ktx2_multilevel_fixture(97, 1, 1, &[&payload]);
    let doc = Ktx2Document::parse(&bytes).unwrap();
    assert_eq!(
        rgba8_ktx2_mip_candidate(&doc, 0, backend),
        Err(Ktx2MipCandidateError::UnsupportedFormat)
    );

    // UNORM format (37) produces is_srgb = false
    let bytes_unorm = ktx2_multilevel_fixture(37, 1, 1, &[&payload]);
    let doc_unorm = Ktx2Document::parse(&bytes_unorm).unwrap();
    let cand = rgba8_ktx2_mip_candidate(&doc_unorm, 0, backend).unwrap();
    assert!(!cand.is_srgb);
}
