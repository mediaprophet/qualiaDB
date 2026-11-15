use super::helpers::{
    assert_both_families, base_single, base_two, encode_envelope, expect_open_err, get_u64,
    put_u16, put_u32, put_u64, raw_meta_envelope, reseal, sample_metadata,
};
use crate::q42::field_envelope::{Error, Family, Limits, StructuralView, HEADER_BYTES};

fn raw_field(key: u16, tag: u8, flags: u8, count: u32, val: &[u8], reserved: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&key.to_le_bytes());
    out.push(tag);
    out.push(flags);
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&(val.len() as u32).to_le_bytes());
    out.extend_from_slice(&reserved.to_le_bytes());
    out.extend_from_slice(val);
    while out.len() % 8 != 0 {
        out.push(0);
    }
    out
}

fn raw_block(id: u32, schema: u16, rev: u16, flags: u32, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&schema.to_le_bytes());
    out.extend_from_slice(&rev.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(body);
    while out.len() % 8 != 0 {
        out.push(0);
    }
    out
}

#[test]
fn rejects_metadata_tlv_and_padding_and_utf8_violations() {
    assert_both_families(|family| {
        expect_open_err(&raw_meta_envelope(family, &[1, 0, 0, 0, 1, 0, 1, 0]), family, Error::Metadata);

        let blk_id0 = raw_block(0, 65000, 1, 0, &[]);
        expect_open_err(&raw_meta_envelope(family, &blk_id0), family, Error::Metadata);

        let mut blk_dup = raw_block(5, 65000, 1, 0, &[]);
        blk_dup.extend_from_slice(&raw_block(5, 65000, 1, 0, &[]));
        expect_open_err(&raw_meta_envelope(family, &blk_dup), family, Error::Metadata);

        let mut blk_desc = raw_block(5, 65000, 1, 0, &[]);
        blk_desc.extend_from_slice(&raw_block(4, 65000, 1, 0, &[]));
        expect_open_err(&raw_meta_envelope(family, &blk_desc), family, Error::Metadata);

        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 2, &[])), family, Error::Metadata);
        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &raw_field(0, 1, 0, 1, &[1, 0, 0, 0], 0))), family, Error::Metadata);

        let mut fld_dup = raw_field(2, 1, 0, 1, &[1, 0, 0, 0], 0);
        fld_dup.extend_from_slice(&raw_field(2, 1, 0, 1, &[2, 0, 0, 0], 0));
        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &fld_dup)), family, Error::Metadata);

        let mut fld_desc = raw_field(3, 1, 0, 1, &[1, 0, 0, 0], 0);
        fld_desc.extend_from_slice(&raw_field(2, 1, 0, 1, &[2, 0, 0, 0], 0));
        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &fld_desc)), family, Error::Metadata);

        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &raw_field(1, 1, 2, 1, &[1, 0, 0, 0], 0))), family, Error::Metadata);
        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &raw_field(1, 1, 0, 1, &[1, 0, 0, 0], 1))), family, Error::Metadata);
        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &raw_field(1, 1, 0, 2, &[1, 0, 0, 0], 0))), family, Error::Numeric);

        let mut bad_pad = raw_field(1, 6, 0, 3, b"abc", 0);
        *bad_pad.last_mut().unwrap() = 0x7f;
        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &bad_pad)), family, Error::Padding);

        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &raw_field(1, 6, 0, 2, &[0xff, 0xfe], 0))), family, Error::Metadata);
        expect_open_err(&raw_meta_envelope(family, &raw_block(1, 65000, 1, 0, &raw_field(1, 6, 0, 7, b"bad\0str", 0))), family, Error::Metadata);
    });
}

#[test]
fn rejects_non_finite_f32_and_f64_including_signaling_nan() {
    let f32_bad = [
        0x7fc0_0000u32, // qNaN
        0x7f80_0001u32, // sNaN
        0x7f80_0000u32, // +Inf
        0xff80_0000u32, // -Inf
    ];
    let f64_bad = [
        0x7ff8_0000_0000_0000u64, // qNaN
        0x7ff0_0000_0000_0001u64, // sNaN
        0x7ff0_0000_0000_0000u64, // +Inf
        0xfff0_0000_0000_0000u64, // -Inf
    ];
    assert_both_families(|family| {
        for bits in f32_bad {
            let blk = raw_block(1, 65000, 1, 0, &raw_field(1, 4, 0, 1, &bits.to_le_bytes(), 0));
            expect_open_err(&raw_meta_envelope(family, &blk), family, Error::Numeric);
        }
        for bits in f64_bad {
            let blk = raw_block(1, 65000, 1, 0, &raw_field(1, 5, 0, 1, &bits.to_le_bytes(), 0));
            expect_open_err(&raw_meta_envelope(family, &blk), family, Error::Numeric);
        }
    });
}

#[test]
fn rejects_overflow_and_oversized_limits_claims() {
    assert_both_families(|family| {
        let base = base_single(family);
        let two = base_two(family);
        let empty = encode_envelope(family, 0, &sample_metadata(None, None), &[]);

        // record_count * stride u64 overflow under default Limits
        let mut mul_of = base.clone();
        put_u64(&mut mul_of, HEADER_BYTES + 64, (1u64 << 63) + 1);
        put_u32(&mut mul_of, HEADER_BYTES + 72, 2);
        reseal(&mut mul_of, true, true);
        expect_open_err(&mul_of, family, Error::Overflow);

        // meta_start + meta_len overflow with elevated metadata_bytes limit
        let mut meta_of = empty.clone();
        put_u64(&mut meta_of, 48, u64::MAX - 64);
        let hdr_crc = super::helpers::crc_zeroed(&meta_of[..HEADER_BYTES]);
        put_u32(&mut meta_of, 124, hdr_crc);
        let mut high_meta_lim = Limits::default();
        high_meta_lim.metadata_bytes = u64::MAX;
        assert_eq!(
            StructuralView::open(&meta_of, family, high_meta_lim).unwrap_err(),
            Error::Overflow
        );

        // Oversized header allocation claims exceeding default Limits
        for &(off, width, val) in &[
            (32usize, 4usize, 4097u64),
            (32, 4, u32::MAX as u64),
            (48, 8, (4 << 20) + 1),
            (48, 8, u64::MAX),
        ] {
            let mut m = empty.clone();
            if width == 4 {
                put_u32(&mut m, off, val as u32);
            } else {
                put_u64(&mut m, off, val);
            }
            let crc = super::helpers::crc_zeroed(&m[..HEADER_BYTES]);
            put_u32(&mut m, 124, crc);
            expect_open_err(&m, family, Error::Limit);
        }

        // Tight per-field Limits bounds
        let mut lim_file = Limits::default();
        lim_file.file_bytes = (base.len() - 1) as u64;
        assert_eq!(StructuralView::open(&base, family, lim_file).unwrap_err(), Error::Limit);

        let mut lim_entries = Limits::default();
        lim_entries.entries = 1;
        assert_eq!(StructuralView::open(&two, family, lim_entries).unwrap_err(), Error::Limit);

        let mut lim_chunk = Limits::default();
        lim_chunk.chunk_bytes = 7;
        assert_eq!(StructuralView::open(&base, family, lim_chunk).unwrap_err(), Error::Limit);

        let mut lim_agg = Limits::default();
        lim_agg.aggregate_decoded_bytes = 15;
        assert_eq!(StructuralView::open(&two, family, lim_agg).unwrap_err(), Error::Limit);

        let mut lim_blocks = Limits::default();
        lim_blocks.blocks = 2;
        assert_eq!(StructuralView::open(&base, family, lim_blocks).unwrap_err(), Error::Limit);

        let mut lim_fields = Limits::default();
        lim_fields.fields_per_block = 3;
        assert_eq!(StructuralView::open(&base, family, lim_fields).unwrap_err(), Error::Limit);

        let mut lim_str = Limits::default();
        lim_str.string_bytes = 4;
        assert_eq!(StructuralView::open(&base, family, lim_str).unwrap_err(), Error::Limit);
    });
}

#[test]
fn t01_r02_anti_substitution_and_gap_pins() {
    let two = base_two(Family::EmfV4Candidate);
    let p1 = get_u64(&two, HEADER_BYTES + 40) as usize;
    let p2 = get_u64(&two, HEADER_BYTES + 128 + 40) as usize;

    // Cross-entry chunk payload swap detected by per-entry verify_chunk
    let mut swapped = two.clone();
    let mut tmp = [0u8; 8];
    tmp.copy_from_slice(&swapped[p1..p1 + 8]);
    let b2: [u8; 8] = swapped[p2..p2 + 8].try_into().unwrap();
    swapped[p1..p1 + 8].copy_from_slice(&b2);
    swapped[p2..p2 + 8].copy_from_slice(&tmp);
    let view = StructuralView::open(&swapped, Family::EmfV4Candidate, Limits::default()).unwrap();
    assert_eq!(view.verify_chunk(1).unwrap_err(), Error::PayloadDigest);
    assert_eq!(view.verify_chunk(2).unwrap_err(), Error::PayloadDigest);

    // Stream ID relabel with recomputed directory CRCs only -> INDEX_DOMAIN catches it
    let mut relabel = two.clone();
    put_u32(&mut relabel, HEADER_BYTES + 8, 9);
    put_u32(&mut relabel, HEADER_BYTES + 128 + 8, 10);
    reseal(&mut relabel, true, false);
    expect_open_err(&relabel, Family::EmfV4Candidate, Error::IndexDigest);

    // GAP-010 resolved: TLV tag 6 UTF-8 strings reject ASCII control characters (0x00..=0x1F, 0x7F).
    let ctrl_blk = raw_block(1, 65000, 1, 0, &raw_field(1, 6, 0, 5, b"a\nb\x7fc", 0));
    expect_open_err(
        &raw_meta_envelope(Family::EmfV4Candidate, &ctrl_blk),
        Family::EmfV4Candidate,
        Error::Metadata,
    );

    // Non-critical unknown schema/tag remains structurally tolerated per field_envelope contract
    // (critical flag = 0); semantic adapters reject unknown schema types.

    let schema_zero_blk = raw_block(1, 0, 0, 0, &raw_field(1, 255, 0, 1, &[0, 0, 0, 0], 0));
    assert!(StructuralView::open(
        &raw_meta_envelope(Family::EmfV4Candidate, &schema_zero_blk),
        Family::EmfV4Candidate,
        Limits::default(),
    )
    .is_ok());
    let _ = put_u16;
}
