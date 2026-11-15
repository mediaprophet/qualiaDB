use super::helpers::{
    assert_both_families, base_single, base_two, encode_envelope, expect_open_err, get_u64,
    put_i64, put_u16, put_u32, put_u64, reseal, sample_metadata,
};
use crate::q42::field_envelope::{
    Error, Family, Limits, StructuralView, DIRECTORY_BYTES, HEADER_BYTES,
};

#[test]
fn rejects_magic_cross_family_q4au_and_versions() {
    assert_both_families(|family| {
        let mut bad_magic = base_single(family);
        bad_magic[..8].copy_from_slice(b"BADMAGIC");
        expect_open_err(&bad_magic, family, Error::Family);

        let other = match family {
            Family::EmfV4Candidate => Family::QacV1Candidate,
            Family::QacV1Candidate => Family::EmfV4Candidate,
        };
        expect_open_err(&base_single(other), family, Error::Family);

        let mut q4au_le = [0u8; 128];
        q4au_le[..4].copy_from_slice(&[0x55, 0x41, 0x34, 0x51]);
        expect_open_err(&q4au_le, family, Error::Family);

        let mut q4au_ascii = [0u8; 128];
        q4au_ascii[..4].copy_from_slice(b"Q4AU");
        expect_open_err(&q4au_ascii, family, Error::Family);

        let mut q42ac_alias = base_single(Family::QacV1Candidate);
        q42ac_alias[..8].copy_from_slice(b"Q42AC064");
        expect_open_err(&q42ac_alias, family, Error::Family);

        for ver in [0u16, 2, 3, 5, 99] {
            if ver != family.version() {
                let mut v = base_single(family);
                put_u16(&mut v, 8, ver);
                reseal(&mut v, false, true);
                expect_open_err(&v, family, Error::Version);
            }
        }
    });
}

#[test]
fn rejects_truncation_trailing_bytes_and_header_invariants() {
    assert_both_families(|family| {
        let base = base_single(family);
        expect_open_err(&[], family, Error::Extent);
        expect_open_err(&base[..127], family, Error::Extent);
        expect_open_err(&base[..base.len() - 1], family, Error::Header);

        let mut matched_trunc = base[..base.len() - 1].to_vec();
        let matched_trunc_len = matched_trunc.len() as u64;
        put_u64(&mut matched_trunc, 16, matched_trunc_len);
        reseal(&mut matched_trunc, false, true);
        expect_open_err(&matched_trunc, family, Error::Extent);

        let mut trailing = base.clone();
        trailing.push(0);
        let trailing_len = trailing.len() as u64;
        put_u64(&mut trailing, 16, trailing_len);
        reseal(&mut trailing, false, true);
        expect_open_err(&trailing, family, Error::Extent);

        for (offset, width, val) in [
            (10usize, 2usize, 136u64),
            (12, 4, 2),
            (16, 8, (base.len() - 1) as u64),
            (24, 8, 136),
            (36, 2, 136),
            (38, 2, 2),
            (40, 8, (HEADER_BYTES + DIRECTORY_BYTES + 8) as u64),
        ] {
            let mut m = base.clone();
            match width {
                2 => put_u16(&mut m, offset, val as u16),
                4 => put_u32(&mut m, offset, val as u32),
                _ => put_u64(&mut m, offset, val),
            }
            reseal(&mut m, false, true);
            expect_open_err(&m, family, Error::Header);
        }
    });
}

#[test]
fn rejects_crc_index_domain_and_payload_digest_tampering() {
    assert_both_families(|family| {
        let base = base_single(family);

        let mut hcrc = base.clone();
        hcrc[124] ^= 0x01;
        expect_open_err(&hcrc, family, Error::Crc);

        let mut hbody = base.clone();
        hbody[12] ^= 0x01;
        expect_open_err(&hbody, family, Error::Crc);

        let mut dcrc = base.clone();
        dcrc[HEADER_BYTES + 124] ^= 0x80;
        expect_open_err(&dcrc, family, Error::Crc);

        let mut idxf = base.clone();
        idxf[56] ^= 0x01;
        reseal(&mut idxf, false, false);
        expect_open_err(&idxf, family, Error::IndexDigest);

        // Missing QUALIA-FIELD-INDEX-v1\0 domain prefix
        let mut no_dom = base.clone();
        let meta_end = get_u64(&no_dom, 40) as usize + get_u64(&no_dom, 48) as usize;
        let mut zero_hdr = [0u8; HEADER_BYTES];
        zero_hdr.copy_from_slice(&no_dom[..HEADER_BYTES]);
        zero_hdr[56..88].fill(0);
        zero_hdr[124..128].fill(0);
        let mut h = blake3::Hasher::new();
        h.update(&zero_hdr);
        h.update(&no_dom[HEADER_BYTES..meta_end]);
        no_dom[56..88].copy_from_slice(h.finalize().as_bytes());
        reseal(&mut no_dom, false, false);
        expect_open_err(&no_dom, family, Error::IndexDigest);

        // Chunk payload tampering: structural open succeeds (lazy digest); verify_chunk rejects
        let mut pflip = base.clone();
        let poff = get_u64(&pflip, HEADER_BYTES + 40) as usize;
        pflip[poff] ^= 0x01;
        let view = StructuralView::open(&pflip, family, Limits::default()).unwrap();
        assert_eq!(view.verify_chunk(1).unwrap_err(), Error::PayloadDigest);
    });
}

#[test]
fn rejects_directory_extents_codec_and_ordering() {
    assert_both_families(|family| {
        let base = base_single(family);
        let two = base_two(family);

        let dir_cases = [
            (HEADER_BYTES + 4, 4usize, 8u64),
            (HEADER_BYTES + 112, 4, 0),
            (HEADER_BYTES + 72, 4, 0),
            (HEADER_BYTES + 64, 8, 0),
            (HEADER_BYTES + 24, 8, 0),
            (HEADER_BYTES + 32, 8, 7),
            (HEADER_BYTES + 48, 8, 99),
            (HEADER_BYTES + 56, 8, 99),
        ];
        for (off, width, val) in dir_cases {
            let mut m = base.clone();
            match width {
                4 => put_u32(&mut m, off, val as u32),
                _ => put_u64(&mut m, off, val),
            }
            reseal(&mut m, true, true);
            expect_open_err(&m, family, Error::Directory);
        }

        let mut time_clr = base.clone();
        put_u32(&mut time_clr, HEADER_BYTES + 4, 1);
        put_i64(&mut time_clr, HEADER_BYTES + 16, 12);
        put_u64(&mut time_clr, HEADER_BYTES + 24, 0);
        reseal(&mut time_clr, true, true);
        expect_open_err(&time_clr, family, Error::Directory);

        let mut codec = base.clone();
        put_u16(&mut codec, HEADER_BYTES + 76, 1);
        reseal(&mut codec, true, true);
        expect_open_err(&codec, family, Error::Codec);

        // Overlap, gap, and non-aligned offset
        let p1 = get_u64(&two, HEADER_BYTES + 40);
        let p2 = get_u64(&two, HEADER_BYTES + DIRECTORY_BYTES + 40);
        for target_off in [p1, p2 + 64, p2 + 3] {
            let mut m = two.clone();
            put_u64(&mut m, HEADER_BYTES + DIRECTORY_BYTES + 40, target_off);
            reseal(&mut m, true, true);
            expect_open_err(&m, family, Error::Directory);
        }

        // Order violations across all 5 sort keys + signed negative trap (+1 before -2)
        let mut uns_stream = two.clone();
        put_u32(&mut uns_stream, HEADER_BYTES + 8, 2);
        put_u32(&mut uns_stream, HEADER_BYTES + DIRECTORY_BYTES + 8, 1);
        reseal(&mut uns_stream, true, true);
        expect_open_err(&uns_stream, family, Error::Order);

        let mut neg_trap = two.clone();
        put_u32(&mut neg_trap, HEADER_BYTES + DIRECTORY_BYTES + 8, 1);
        put_i64(&mut neg_trap, HEADER_BYTES + 16, 1);
        put_i64(&mut neg_trap, HEADER_BYTES + DIRECTORY_BYTES + 16, -2);
        reseal(&mut neg_trap, true, true);
        expect_open_err(&neg_trap, family, Error::Order);

        let mut dup_key = two.clone();
        put_u32(&mut dup_key, HEADER_BYTES + DIRECTORY_BYTES + 8, 1);
        reseal(&mut dup_key, true, true);
        expect_open_err(&dup_key, family, Error::Order);

        let mut uns_mode = two.clone();
        put_u32(&mut uns_mode, HEADER_BYTES + DIRECTORY_BYTES + 8, 1);
        put_u16(&mut uns_mode, HEADER_BYTES, 0x0105);
        reseal(&mut uns_mode, true, true);
        expect_open_err(&uns_mode, family, Error::Order);

        let mut uns_desc = two.clone();
        put_u32(&mut uns_desc, HEADER_BYTES + DIRECTORY_BYTES + 8, 1);
        put_u32(&mut uns_desc, HEADER_BYTES + 112, 9);
        reseal(&mut uns_desc, true, true);
        expect_open_err(&uns_desc, family, Error::Order);
    });
}

#[test]
fn rejects_nonzero_padding_across_all_envelope_zones() {
    assert_both_families(|family| {
        let base = base_single(family);
        let mo = get_u64(&base, 40) as usize;
        let ml = get_u64(&base, 48) as usize;
        let po = get_u64(&base, HEADER_BYTES + 40) as usize;

        for idx in [90usize, mo + ml, HEADER_BYTES + 78, HEADER_BYTES + 118, po + 8] {
            let mut m = base.clone();
            m[idx] = 0x5A;
            reseal(&mut m, true, true);
            expect_open_err(&m, family, Error::Padding);
        }

        let meta = sample_metadata(None, None);
        let aligned4096 = encode_envelope(family, 1, &meta, &[super::helpers::entry_timed(1, 0, 0x0104, 4, b"P4K")]);
        let mut p4k = aligned4096.clone();
        let po4 = get_u64(&p4k, HEADER_BYTES + 40) as usize;
        p4k[po4 + 3] = 0x11;
        reseal(&mut p4k, true, true);
        expect_open_err(&p4k, family, Error::Padding);
    });
}
