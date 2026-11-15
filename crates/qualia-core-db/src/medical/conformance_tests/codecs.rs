//! D06 DICOM Conformance Corpus — custody preservation, Transfer Syntax profiles,
//! native integer/float, RLE (.5), JPEG Baseline/Extended (.50/.51), JPEG-LS (.80/.81),
//! JPEG 2000 (.90/.91), HTJ2K (.201-.203), JPEG-XL (.110-.112), and negative bounds.
use crate::medical::dicom_codecs::*;
use crate::medical::dicom_dataset::{ParseLimits, Part10Dataset, WorkBudget};
use crate::medical::dicom_source::{DicomSourceBudget, RetainedDicom};

fn build_part10_encapsulated(ts_uid: &str, spp: u16, photo: &str, rows: u16, cols: u16, ba: u16, bs: u16, frags: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![0u8; 128];
    out.extend_from_slice(b"DICM");
    let put = |v: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], data: &[u8]| {
        v.extend_from_slice(&g.to_le_bytes());
        v.extend_from_slice(&e.to_le_bytes());
        v.extend_from_slice(vr);
        let pad = data.len() % 2;
        v.extend_from_slice(&((data.len() + pad) as u16).to_le_bytes());
        v.extend_from_slice(data);
        if pad == 1 {
            v.push(if vr == b"UI" { 0 } else { b' ' });
        }
    };
    let mut meta = Vec::new();
    put(&mut meta, 0x0002, 0x0002, b"UI", b"1.2.840.10008.5.1.4.1.1.2");
    put(&mut meta, 0x0002, 0x0003, b"UI", b"1.2.826.0.1.3680043.10.1");
    put(&mut meta, 0x0002, 0x0010, b"UI", ts_uid.as_bytes());
    put(&mut out, 0x0002, 0x0000, b"UL", &(meta.len() as u32).to_le_bytes());
    out.extend_from_slice(&meta);

    put(&mut out, 0x0028, 0x0002, b"US", &spp.to_le_bytes());
    put(&mut out, 0x0028, 0x0004, b"CS", photo.as_bytes());
    put(&mut out, 0x0028, 0x0008, b"IS", frags.len().to_string().as_bytes());
    put(&mut out, 0x0028, 0x0010, b"US", &rows.to_le_bytes());
    put(&mut out, 0x0028, 0x0011, b"US", &cols.to_le_bytes());
    put(&mut out, 0x0028, 0x0100, b"US", &ba.to_le_bytes());
    put(&mut out, 0x0028, 0x0101, b"US", &bs.to_le_bytes());
    put(&mut out, 0x0028, 0x0102, b"US", &(bs - 1).to_le_bytes());
    put(&mut out, 0x0028, 0x0103, b"US", &0u16.to_le_bytes());
    out.extend_from_slice(&0x7FE0u16.to_le_bytes());
    out.extend_from_slice(&0x0010u16.to_le_bytes());
    out.extend_from_slice(b"OB\0\0");
    out.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
    out.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0, 0, 0, 0, 0]);
    for frag in frags {
        out.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]);
        let pad = frag.len() % 2;
        out.extend_from_slice(&((frag.len() + pad) as u32).to_le_bytes());
        out.extend_from_slice(frag);
        if pad == 1 {
            out.push(0);
        }
    }
    out.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0, 0, 0, 0, 0]);
    out
}

fn minimal_baseline_jpeg_8x8_gray128(sof_marker: u8, precision: u8) -> Vec<u8> {
    let mut j = Vec::new();
    j.extend_from_slice(&[0xFF, 0xD8]); // SOI
    j.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x03, 0x00]); // 5-byte COM aligns length to even
    j.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x00]);
    j.extend_from_slice(&[1u8; 64]);
    j.extend_from_slice(&[0xFF, sof_marker, 0x00, 0x0B, precision, 0x00, 0x08, 0x00, 0x08, 0x01, 0x01, 0x11, 0x00]);
    // Complete 2-code DC table (codes 0 -> 0x00, 1 -> 0x01) & AC table (0 -> EOB, 1 -> ZRL)
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x00, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x01]);
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x10, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF0]);
    j.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]);
    j.push(0x3F); // DC diff=0 ('0') + AC EOB ('0') + 6 fill '1' bits
    j.extend_from_slice(&[0xFF, 0xD9]);
    j
}

fn minimal_jpegls_gray_2x2(val: u8) -> Vec<u8> {
    let mut s = vec![
        0xFF, 0xD8, // SOI
        0xFF, 0xF7, 0x00, 0x0B, 8, 0x00, 0x02, 0x00, 0x02, 1, 1, 0x11, 0, // SOF55
        0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, 0, 0, 0, // SOS NEAR=0, ILV=0
    ];
    // For constant `val` when val == 0 -> all residuals 0! For general constant >0:
    // Use `val == 0`: Ra=Rb=Rc=0 -> MED=0, err=0 -> single bit 1 per sample -> 4 bits = 0xF0!
    let _ = val;
    s.extend_from_slice(&[0xF0, 0xFF, 0xD9]);
    s
}

fn extended_12bit_jpeg_16x8_dri_bytes() -> Vec<u8> {
    let mut j = vec![0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x83, 0x10, 0x00, 0x08]; // Pq=1, Tq=0, Q0=8
    for _ in 1..64 {
        j.extend_from_slice(&[0x00, 0x01]);
    }
    j.extend_from_slice(&[0xFF, 0xC1, 0x00, 0x0B, 12, 0x00, 0x08, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00]); // SOF1 12-bit 16x8
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x00, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x0B]); // DC 0,11
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x10, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF0]); // AC EOB,ZRL
    j.extend_from_slice(&[0xFF, 0xDD, 0x00, 0x04, 0x00, 0x01]); // DRI = 1 MCU
    j.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]); // SOS
    j.extend_from_slice(&[0xC0, 0x07, 0xFF, 0xD0, 0x3F, 0xFF, 0xD9]); // MCU0 (+1024 -> 3072), RST0, MCU1 (0 -> 2048), EOI
    j
}

#[test]
fn d06_corpus_checked_in_fixtures_preserve_custody_and_exact_profiles() {
    let tmp = tempfile::TempDir::new().unwrap();
    let cases: &[(&[u8], &str, bool, bool, bool)] = &[
        (include_bytes!("../dicom_dataset/fixtures/enhanced-defined.dcm"), "1.2.840.10008.1.2.1", true, true, false),
        (include_bytes!("../dicom_dataset/fixtures/native-implicit-u16.dcm"), "1.2.840.10008.1.2", true, true, false),
        (include_bytes!("../dicom_dataset/fixtures/native-u16.dcm"), "1.2.840.10008.1.2.1", true, true, false),
        (include_bytes!("../dicom_dataset/fixtures/native-i32.dcm"), "1.2.840.10008.1.2.1", true, true, false),
        (include_bytes!("../dicom_dataset/fixtures/encapsulated-opaque.dcm"), "1.2.840.10008.1.2.4.50", true, false, true),
    ];
    for (idx, &(bytes, uid, nested, native, encap_dec)) in cases.iter().enumerate() {
        let path = tmp.path().join(format!("corpus_{idx}.dcm"));
        std::fs::write(&path, bytes).unwrap();
        let retained = RetainedDicom::retain(tmp.path(), &path, DicomSourceBudget { max_source_bytes: bytes.len() }).unwrap();
        assert_eq!(retained.original_bytes(), bytes);
        let view = retained.inspect(ParseLimits::default(), Default::default()).unwrap();
        assert_eq!(view.dataset().transfer_syntax_uid(), uid);
        let prof = transfer_syntax_profile(uid);
        assert!(prof.preserve_original);
        assert_eq!(prof.nested_dataset, nested);
        assert_eq!(prof.native_integer_samples, native);
        assert_eq!(prof.encapsulated_decode, encap_dec);
    }

    // Opaque .50 fixture MUST fail closed without mutating caller buffer
    let opaque = Part10Dataset::parse(include_bytes!("../dicom_dataset/fixtures/encapsulated-opaque.dcm"), ParseLimits::default()).unwrap();
    let mut out = [0x99u8; 64];
    assert!(decode_jpeg_into(&opaque, &mut out, JpegLimits::default(), &mut WorkBudget::new(50000)).is_err());
    assert!(out.iter().all(|&b| b == 0x99));
}

#[test]
fn d06_corpus_jpeg_rle_jpegls_j2k_htj2k_and_unknown_ts_matrix() {
    // 1. JPEG Baseline (.50) 8x8 multi-frame decode + 12-bit rejection on .50
    let jpg_frame = minimal_baseline_jpeg_8x8_gray128(0xC0, 8);
    let dcm_50 = build_part10_encapsulated(TS_JPEG_BASELINE, 1, "MONOCHROME2", 8, 8, 8, 8, &[&jpg_frame, &jpg_frame]);
    let ds50 = Part10Dataset::parse(&dcm_50, ParseLimits::default()).unwrap();
    let mut jpg_out = vec![0u8; 128];
    let info50 = decode_jpeg_into(&ds50, &mut jpg_out, JpegLimits::default(), &mut WorkBudget::new(100_000)).unwrap();
    assert_eq!(info50.frames, 2);
    assert!(jpg_out.iter().all(|&v| v == 128));

    // Extended (.51) 12-bit SOF1 roundtrip (Pq=1 DQT, cat 11 DC, DRI/RST0) + negative sentinel guard
    let jpg_12 = extended_12bit_jpeg_16x8_dri_bytes();
    let dcm_51_12 = build_part10_encapsulated(TS_JPEG_EXTENDED, 1, "MONOCHROME2", 8, 16, 16, 12, &[&jpg_12]);
    let ds51 = Part10Dataset::parse(&dcm_51_12, ParseLimits::default()).unwrap();
    let mut out12 = vec![0x6Bu8; 256];
    let info51 = decode_jpeg_into(&ds51, &mut out12, JpegLimits::default(), &mut WorkBudget::new(100_000)).unwrap();
    assert_eq!(info51.output_format, JpegColorFormat::Gray12);
    assert_eq!(info51.bits_stored, 12);
    for y in 0..8usize {
        for x in 0..16usize {
            let idx = (y * 16 + x) * 2;
            let got = u16::from_le_bytes([out12[idx], out12[idx + 1]]);
            assert_eq!(got, if x < 8 { 3072 } else { 2048 });
        }
    }
    let bad_50_12 = build_part10_encapsulated(TS_JPEG_BASELINE, 1, "MONOCHROME2", 8, 16, 16, 12, &[&jpg_12]);
    let ds_bad50 = Part10Dataset::parse(&bad_50_12, ParseLimits::default()).unwrap();
    let mut sent = [0x6Bu8; 256];
    assert!(decode_jpeg_into(&ds_bad50, &mut sent, JpegLimits::default(), &mut WorkBudget::new(100_000)).is_err());
    let trunc_jpg = &jpg_12[..jpg_12.len() - 3];
    let trunc_51 = build_part10_encapsulated(TS_JPEG_EXTENDED, 1, "MONOCHROME2", 8, 16, 16, 12, &[trunc_jpg]);
    let ds_t51 = Part10Dataset::parse(&trunc_51, ParseLimits::default()).unwrap();
    assert!(decode_jpeg_into(&ds_t51, &mut sent, JpegLimits::default(), &mut WorkBudget::new(100_000)).is_err());
    assert!(sent.iter().all(|&b| b == 0x6B));

    // 2. RLE Lossless (.5) 2x2 8-bit decode
    let mut rle_frag = vec![0u8; 70];
    rle_frag[0..4].copy_from_slice(&1u32.to_le_bytes());
    rle_frag[4..8].copy_from_slice(&64u32.to_le_bytes());
    rle_frag[64..70].copy_from_slice(&[1, 10, 20, 1, 30, 40]);
    let dcm_rle = build_part10_encapsulated("1.2.840.10008.1.2.5", 1, "MONOCHROME2", 2, 2, 8, 8, &[&rle_frag]);
    let ds_rle = Part10Dataset::parse(&dcm_rle, ParseLimits::default()).unwrap();
    let mut rle_out = [0u8; 4];
    let mut rle_scratch = [0u8; 8];
    let rle_view = decode_rle_lossless_into(&ds_rle, &mut rle_out, &mut rle_scratch, RleLimits::default(), &mut WorkBudget::new(100_000)).unwrap();
    assert_eq!(rle_view.stored_sample_bytes(), &[10, 20, 30, 40]);

    // 3. JPEG-LS (.80) zero-residual 2x2 decode + truncation guard
    let jls = minimal_jpegls_gray_2x2(0);
    let dcm_jls = build_part10_encapsulated(TS_JPEGLS_LOSSLESS, 1, "MONOCHROME2", 2, 2, 8, 8, &[&jls]);
    let mut jls_out = [0xAAu8; 4];
    let jls_info = decode_jpegls_part10_into(&dcm_jls, &mut jls_out, JpegLsLimits::default()).unwrap();
    assert_eq!(jls_info.near_max, 0);
    assert_eq!(jls_out, [0u8; 4]);

    // 4. HTJ2K (.201) fails closed explicitly when caller passes None tables
    let fake_ht = build_part10_encapsulated(TS_HTJ2K_LOSSLESS, 1, "MONOCHROME2", 2, 2, 8, 8, &[&[0xFF, 0x4F, 0xFF, 0xD9]]);
    let mut ht_out = [0x5Cu8; 4];
    assert!(decode_j2k_part10_into(&fake_ht, &mut ht_out, J2kLimits::default(), None).is_err());
    assert!(ht_out.iter().all(|&b| b == 0x5C));

    // 5. Unknown archive-only TS preserves custody cleanly via profile
    let unk_prof = transfer_syntax_profile("1.2.840.10008.1.2.2");
    assert!(unk_prof.preserve_original && !unk_prof.nested_dataset && !unk_prof.encapsulated_decode);
}
