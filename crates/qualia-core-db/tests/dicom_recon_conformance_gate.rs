//! Wave-3 Conformance Gate (`W3-06` / `GATE-DICOM-RECON-CONFORMANCE`).
//! End-to-end integration verification of `qualia_core_db::medical` under `--no-default-features`.

use core::f64::consts::PI;
use qualia_core_db::medical::dicom_codecs::j2k::{annex_c_ht_vlc_tables, encode_ht_cleanup_segment, lookup_vlc_codeword};
use qualia_core_db::medical::dicom_codecs::jpeg_12bit::decode_jpeg_12bit_frame;
use qualia_core_db::medical::dicom_codecs::padding::{IntegerPaddingView, SampleValidity};
use qualia_core_db::medical::dicom_codecs::{
    decode_j2k_codestream_into, decode_j2k_part10_dataset_into, decode_j2k_part10_into, decode_j2k_part10_with_annex_c_into,
    decode_jpeg_into, decode_jpegls_codestream_into, decode_jpegls_part10_into, decode_jxl_codestream_into, decode_jxl_part10_into,
    decode_rle_lossless_into, encode_jbrd_box_from_jpeg, extract_jbrd_payload, reconstruct_jpeg_from_jbrd,
    reconstruct_jpeg_from_jxl_container, transfer_syntax_profile, J2kLimits, JpegColorFormat, JpegLimits, JpegLsError, JpegLsLimits,
    JxlEncoding, JxlError, JxlLimits, NativePixelView, RleLimits, StoredSample, WaveletTransform, TS_HTJ2K, TS_HTJ2K_LOSSLESS,
    TS_HTJ2K_RPCL, TS_J2K, TS_J2K_LOSSLESS, TS_JPEGLS_LOSSLESS, TS_JPEGLS_NEAR_LOSSLESS, TS_JPEG_BASELINE, TS_JPEG_EXTENDED,
    TS_JXL, TS_JXL_JPEG_RECOMPRESSION, TS_JXL_LOSSLESS,
};
use qualia_core_db::medical::dicom_dataset::{ElementKind, ParseLimits, Part10Dataset, TransferSyntax, WorkBudget};
use qualia_core_db::medical::dicom_frames::{FrameError, FrameLimits, FrameSet, ValueUnits};
use qualia_core_db::medical::dicom_geometry::{FrameGeometryInput, GeometryAttribute, GeometryError, PatientImagePlane};
use qualia_core_db::medical::dicom_part10::{is_dicom_part10, pack_volume_metadata, parse_dicom_metadata_bytes, split_dicom_payload, unpack_volume_metadata};
use qualia_core_db::medical::dicom_source::{archive_part10_source, DicomSourceBudget, RetainedDicom};
use qualia_core_db::medical::reconstruction::{
    analytic_hilbert_envelope_1d, apodization_weight, beamform_ultrasound_das_into, calibrate_and_infer_into, combine_coils_sense,
    combine_coils_sos, fft_1d, fft_2d, fft_3d, fftshift_1d, forward_project_pet, ifft_1d, ifftshift_1d, reconstruct_ct_fbp_into,
    reconstruct_mri_kspace_2d, reconstruct_mri_kspace_3d, reconstruct_pet_osem_into, register_patient_to_anatomy, AnatomyRegError,
    AnatomyTransform, CrossFrameRegistrationCertificate, DasConfig, FbpConfig, FbpFilter, InferenceError, MriKspaceConfig,
    NonVisualDomain, OsemConfig, PetSinogram, ProjectionGeometry, SinogramView, TxFocusMode, UltrasoundArray, UncertaintyBudget, WindowKind,
};
use qualia_core_db::solvers::transforms::fourier::{dft, idft, Cplx};

fn put_vr(v: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], data: &[u8]) {
    v.extend_from_slice(&g.to_le_bytes());
    v.extend_from_slice(&e.to_le_bytes());
    v.extend_from_slice(vr);
    let pad = data.len() % 2;
    if matches!(vr, b"OB" | b"OW") {
        v.extend_from_slice(&[0, 0]);
        v.extend_from_slice(&((data.len() + pad) as u32).to_le_bytes());
    } else {
        v.extend_from_slice(&((data.len() + pad) as u16).to_le_bytes());
    }
    v.extend_from_slice(data);
    if pad == 1 { v.push(if vr == b"UI" { 0 } else { b' ' }); }
}

fn build_part10_encapsulated(ts_uid: &str, spp: u16, photo: &str, rows: u16, cols: u16, ba: u16, bs: u16, frags: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![0u8; 128];
    out.extend_from_slice(b"DICM");
    let mut meta = Vec::new();
    put_vr(&mut meta, 0x0002, 0x0002, b"UI", b"1.2.840.10008.5.1.4.1.1.2");
    put_vr(&mut meta, 0x0002, 0x0003, b"UI", b"1.2.826.0.1.3680043.10.99");
    put_vr(&mut meta, 0x0002, 0x0010, b"UI", ts_uid.as_bytes());
    put_vr(&mut out, 0x0002, 0x0000, b"UL", &(meta.len() as u32).to_le_bytes());
    out.extend_from_slice(&meta);
    put_vr(&mut out, 0x0028, 0x0002, b"US", &spp.to_le_bytes());
    put_vr(&mut out, 0x0028, 0x0004, b"CS", photo.as_bytes());
    put_vr(&mut out, 0x0028, 0x0008, b"IS", frags.len().to_string().as_bytes());
    for &(e, val) in &[(0x0010u16, rows), (0x0011, cols), (0x0100, ba), (0x0101, bs), (0x0102, bs - 1), (0x0103, 0)] {
        put_vr(&mut out, 0x0028, e, b"US", &val.to_le_bytes());
    }
    out.extend_from_slice(&[0xE0, 0x7F, 0x10, 0x00, b'O', b'B', 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0xFF, 0x00, 0xE0, 0, 0, 0, 0]);
    for frag in frags {
        out.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]);
        let pad = frag.len() % 2;
        out.extend_from_slice(&((frag.len() + pad) as u32).to_le_bytes());
        out.extend_from_slice(frag);
        if pad == 1 { out.push(0); }
    }
    out.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0, 0, 0, 0, 0]);
    out
}

fn sample_jpeg12() -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x83, 0x10, 0x00, 0x08];
    for _ in 1..64 { v.extend_from_slice(&[0x00, 0x01]); }
    v.extend_from_slice(&[
        0xFF, 0xC1, 0x00, 0x0B, 12, 0x00, 0x08, 0x00, 0x10, 1, 1, 0x11, 0,
        0xFF, 0xC4, 0x00, 0x15, 0x00, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x0B,
        0xFF, 0xC4, 0x00, 0x15, 0x10, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF0,
        0xFF, 0xDD, 0x00, 0x04, 0x00, 0x01,
        0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, 0x00, 0x3F, 0x00, 0xC0, 0x07, 0xFF, 0xD0, 0x3F, 0xFF, 0xD9,
    ]);
    v
}

struct LsbBw { buf: Vec<u8>, bit_pos: usize }
impl LsbBw {
    fn new() -> Self { Self { buf: Vec::new(), bit_pos: 0 } }
    fn bits(&mut self, val: u64, n: usize) {
        for i in 0..n {
            let (bi, sh) = (self.bit_pos >> 3, self.bit_pos & 7);
            if bi >= self.buf.len() { self.buf.push(0); }
            if ((val >> i) & 1) != 0 { self.buf[bi] |= 1u8 << sh; }
            self.bit_pos += 1;
        }
    }
    fn align(&mut self) {
        let r = self.bit_pos & 7;
        if r != 0 { self.bits(0, 8 - r); }
    }
}

#[test]
fn gate_part10_source_dataset_frames_geometry_and_lut_conformance() {
    let tmp = tempfile::TempDir::new().unwrap();
    let enh = include_bytes!("../src/medical/dicom_dataset/fixtures/enhanced-defined.dcm");
    let rw = include_bytes!("../src/medical/dicom_dataset/fixtures/native-u8-rwvm.dcm");
    assert!(is_dicom_part10(enh) && is_dicom_part10(rw));
    assert_eq!(unpack_volume_metadata(pack_volume_metadata(512, 256, 262_144)), (512, 256, 262_144));
    let p_enh = tmp.path().join("enh.dcm");
    std::fs::write(&p_enh, enh).unwrap();
    let retained = RetainedDicom::retain(tmp.path(), &p_enh, DicomSourceBudget { max_source_bytes: enh.len() }).unwrap();
    assert_eq!(retained.original_bytes(), enh);
    let archived = archive_part10_source(&tmp.path().join("arch"), &p_enh, DicomSourceBudget { max_source_bytes: enh.len() }).unwrap();
    assert_eq!(archived.blake3, *blake3::hash(enh).as_bytes());

    let ds = Part10Dataset::parse(enh, ParseLimits::default()).unwrap();
    assert_eq!(ds.transfer_syntax(), TransferSyntax::ExplicitLittleEndian);
    let mut wb = WorkBudget::new(100_000);
    assert_eq!(ds.frame_count(&mut wb).unwrap(), 2);
    let px_id = ds.find(None, 0x7FE0_0010, &mut wb).unwrap().unwrap();
    assert_eq!(ds.element(px_id).unwrap().kind, ElementKind::Value);

    let nv = NativePixelView::from_dataset(&ds, 4096, &mut wb).unwrap();
    assert_eq!((nv.sample(0, 0, 0).unwrap(), nv.sample(1, 0, 1).unwrap()), (StoredSample::Signed(-2048), StoredSample::Signed(2047)));
    let pv = IntegerPaddingView::bind(&ds, &nv, &mut wb).unwrap();
    assert_eq!(pv.sample(0, 0, 0).unwrap().validity, SampleValidity::Valid);

    let fset = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    assert!(fset.enhanced && fset.frames.len() == 2);
    let plane0 = fset.frames[0].plane().unwrap();
    assert_eq!((plane0.pixel_center_lps_mm(1, 1).unwrap(), plane0.normal_lps()), ([0.75, 0.5, 0.0], [0.0, 0.0, 1.0]));
    assert_eq!(fset.frames[0].quantitative_sample(&nv, 0, 0, false).unwrap(), Some(-5096.0));
    assert_eq!(fset.frames[1].quantitative_sample(&nv, 0, 1, false).unwrap(), Some(6151.0));
    assert_eq!(fset.frames[0].quantitative().modality.unwrap().units(), ValueUnits::RescaleType("HU"));

    let mut sf = build_part10_encapsulated("1.2.840.10008.1.2.1", 1, "MONOCHROME2", 2, 2, 8, 8, &[]);
    sf.truncate(sf.len() - 28);
    for &(g, e, vr, d) in &[
        (0x0020u16, 0x0032u16, b"DS", &b"10.0\\20.0\\30.0"[..]), (0x0020, 0x0037, b"DS", &b"1\\0\\0\\0\\1\\0"[..]),
        (0x0028, 0x0030, b"DS", &b"0.5\\0.25"[..]), (0x0028, 0x1050, b"DS", &b"40.0"[..]),
        (0x0028, 0x1051, b"DS", &b"400.0"[..]), (0x7FE0, 0x0010, b"OW", &[10u8, 20, 30, 40][..]),
    ] {
        put_vr(&mut sf, g, e, vr, d);
    }
    let split = split_dicom_payload(&sf).unwrap();
    assert_eq!(split.meta.photometric_interpretation, "MONOCHROME2");
    assert_eq!((split.meta.window_center, split.meta.window_width, split.pixels.length), (Some(40.0), Some(400.0), 4));
    let parsed_sf = parse_dicom_metadata_bytes(&sf).unwrap();
    let plane_sf = PatientImagePlane::from_metadata(&parsed_sf).unwrap();
    assert_eq!((plane_sf.origin_lps_mm(), plane_sf.pixel_center_lps_mm(2, 4).unwrap()), ([10.0, 20.0, 30.0], [10.5, 22.0, 30.0]));
    assert_eq!(PatientImagePlane::from_frame_geometry(FrameGeometryInput { anatomical_orientation_type: Some("QUADRUPED"), ..Default::default() }).unwrap_err(), GeometryError::UnsupportedQuadruped);
    assert_eq!(PatientImagePlane::from_frame_geometry(FrameGeometryInput::default()).unwrap_err(), GeometryError::Missing(GeometryAttribute::ImagePositionPatient));
    let _ = FrameError::NonImage;
}

#[test]
fn gate_codecs_rle_jpeg8_12_and_jpegls_lossless_near_roundtrip() {
    let mut rle_frag = vec![0u8; 76];
    rle_frag[0..4].copy_from_slice(&2u32.to_le_bytes());
    rle_frag[4..8].copy_from_slice(&64u32.to_le_bytes());
    rle_frag[8..12].copy_from_slice(&70u32.to_le_bytes());
    rle_frag[64..70].copy_from_slice(&[1, 0x01, 0x02, 0xFF, 0x03, 0x80]);
    rle_frag[70..76].copy_from_slice(&[1, 0x10, 0x20, 0xFF, 0x30, 0x80]);
    let dcm_rle = build_part10_encapsulated("1.2.840.10008.1.2.5", 1, "MONOCHROME2", 2, 2, 16, 16, &[&rle_frag]);
    let ds_rle = Part10Dataset::parse(&dcm_rle, ParseLimits::default()).unwrap();
    let (mut rle_out, mut rle_scratch) = ([0u8; 8], [0u8; 8]);
    let rle_view = decode_rle_lossless_into(&ds_rle, &mut rle_out, &mut rle_scratch, RleLimits::default(), &mut WorkBudget::new(100_000)).unwrap();
    assert_eq!((rle_view.sample(0, 0, 0).unwrap(), rle_view.sample(0, 0, 1).unwrap()), (StoredSample::Unsigned(0x0110), StoredSample::Unsigned(0x0220)));
    assert_eq!((rle_view.sample(0, 1, 0).unwrap(), rle_view.sample(0, 1, 1).unwrap()), (StoredSample::Unsigned(0x0330), StoredSample::Unsigned(0x0330)));

    let mut jpg8 = vec![0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x43, 0x00];
    jpg8.extend_from_slice(&[1u8; 64]);
    jpg8.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x0B, 8, 0x00, 0x08, 0x00, 0x08, 1, 1, 0x11, 0,
        0xFF, 0xC4, 0x00, 0x15, 0x00, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x01,
        0xFF, 0xC4, 0x00, 0x15, 0x10, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF0,
        0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, 0x00, 0x3F, 0x00, 0x3F, 0xFF, 0xD9,
    ]);
    let dcm_50 = build_part10_encapsulated(TS_JPEG_BASELINE, 1, "MONOCHROME2", 8, 8, 8, 8, &[&jpg8]);
    let ds_50 = Part10Dataset::parse(&dcm_50, ParseLimits::default()).unwrap();
    let mut out8 = [0u8; 64];
    let info50 = decode_jpeg_into(&ds_50, &mut out8, JpegLimits::default(), &mut WorkBudget::new(100_000)).unwrap();
    assert!(info50.output_format == JpegColorFormat::Gray8 && out8.iter().all(|&b| b == 128));

    let jpg12 = sample_jpeg12();
    let dcm_51 = build_part10_encapsulated(TS_JPEG_EXTENDED, 1, "MONOCHROME2", 8, 16, 16, 12, &[&jpg12]);
    let ds_51 = Part10Dataset::parse(&dcm_51, ParseLimits::default()).unwrap();
    let mut out12 = vec![0u8; 256];
    let info51 = decode_jpeg_into(&ds_51, &mut out12, JpegLimits::default(), &mut WorkBudget::new(100_000)).unwrap();
    assert_eq!(info51.output_format, JpegColorFormat::Gray12);
    assert_eq!((u16::from_le_bytes([out12[0], out12[1]]), u16::from_le_bytes([out12[16], out12[17]])), (3072, 2048));

    let jls_lossless = [0xFF, 0xD8, 0xFF, 0xF7, 0x00, 0x0B, 8, 0x00, 0x02, 0x00, 0x02, 1, 1, 0x11, 0, 0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, 0, 0, 0, 0xF0, 0xFF, 0xD9];
    let mut jls_out = [0xFFu8; 4];
    assert_eq!(decode_jpegls_codestream_into(&jls_lossless, 2, 2, 1, 8, 8, true, &mut jls_out).unwrap(), 0);
    assert_eq!(jls_out, [0u8; 4]);
    let jls_near = [0xFF, 0xD8, 0xFF, 0xF7, 0x00, 0x0B, 8, 0x00, 0x02, 0x00, 0x02, 1, 1, 0x11, 0, 0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, 2, 0, 0, 0xF0, 0xFF, 0xD9];
    let dcm_jls_bad = build_part10_encapsulated(TS_JPEGLS_LOSSLESS, 1, "MONOCHROME2", 2, 2, 8, 8, &[&jls_near]);
    assert_eq!(decode_jpegls_part10_into(&dcm_jls_bad, &mut jls_out, JpegLsLimits::default()).unwrap_err(), JpegLsError::LosslessModeViolation);
    let dcm_jls_near = build_part10_encapsulated(TS_JPEGLS_NEAR_LOSSLESS, 1, "MONOCHROME2", 2, 2, 8, 8, &[&jls_near]);
    assert_eq!(decode_jpegls_part10_into(&dcm_jls_near, &mut jls_out, JpegLsLimits::default()).unwrap().near_max, 2);
}

#[test]
fn gate_codecs_j2k_53_97_and_htj2k_annex_c_part10_roundtrip() {
    let tables = annex_c_ht_vlc_tables();
    assert!(lookup_vlc_codeword(tables, true, 0, 15, 1, 0, 0).is_some() && lookup_vlc_codeword(tables, false, 7, 15, 1, 0, 0).is_some());
    let seg_ll1 = encode_ht_cleanup_segment(&[10i32, 10, 10, 10], 2, 2, 0, tables).unwrap();
    let len5 = seg_ll1.len() as u8;
    let mut pkt_r0 = vec![0b1100_0000u8, 0b0101_1000u8 | ((len5 >> 3) & 0x03), (len5 & 0x07) << 5];
    pkt_r0.extend_from_slice(&seg_ll1);
    pkt_r0.push(0x00);

    let build_j2k = |decomp: u8, wv: u8, qcd: &[u8], pkt: &[u8]| -> Vec<u8> {
        let mut cs = vec![0xFF, 0x4F, 0xFF, 0x51, 0x00, 41, 0, 0];
        for &v in &[4u32, 4, 0, 0, 4, 4, 0, 0] { cs.extend_from_slice(&v.to_be_bytes()); }
        cs.extend_from_slice(&[0, 1, 7, 1, 1, 0xFF, 0x50, 0x00, 8, 0x00, 0x02, 0, 0, 0, 0]);
        cs.extend_from_slice(&[0xFF, 0x52, 0x00, 12, 0, 0, 0, 1, 0, decomp, 4, 4, 0x40, wv]);
        cs.extend_from_slice(qcd);
        cs.extend_from_slice(&[0xFF, 0x90, 0x00, 10, 0, 0]);
        cs.extend_from_slice(&((14 + pkt.len()) as u32).to_be_bytes());
        cs.extend_from_slice(&[0, 1, 0xFF, 0x93]);
        cs.extend_from_slice(pkt);
        cs.extend_from_slice(&[0xFF, 0xD9]);
        cs
    };

    let qcd_1lvl = [0xFF, 0x5C, 0x00, 7, 0x20, 8 << 3, 9 << 3, 9 << 3, 10 << 3];
    let (cs_53, cs_97) = (build_j2k(1, 1, &qcd_1lvl, &pkt_r0), build_j2k(1, 0, &qcd_1lvl, &pkt_r0));
    let mut out = [0u8; 16];
    assert_eq!(decode_j2k_codestream_into(&cs_53, 4, 4, 1, 8, 8, true, &mut out).unwrap(), WaveletTransform::Reversible53);
    assert_eq!(out, [138u8; 16]);
    assert!(decode_j2k_codestream_into(&cs_97, 4, 4, 1, 8, 8, true, &mut out).is_err());
    assert_eq!(decode_j2k_codestream_into(&cs_97, 4, 4, 1, 8, 8, false, &mut out).unwrap(), WaveletTransform::Irreversible97);
    assert_eq!(out, [138u8; 16]);

    let orig = [-12i32, 0, 7, -1, 45, -3, 0, 19, 0, 0, -2, 8, 6, -15, 1, 0];
    let seg = encode_ht_cleanup_segment(&orig, 4, 4, 0, tables).unwrap();
    let l5 = seg.len() as u8;
    let mut pkt = vec![0b1100_0000u8, 0b0101_1000u8 | ((l5 >> 3) & 0x03), (l5 & 0x07) << 5];
    pkt.extend_from_slice(&seg);
    let cs_ht = build_j2k(0, 1, &[0xFF, 0x5C, 0x00, 4, 0x20, 8 << 3], &pkt);
    let expected: Vec<u8> = orig.iter().map(|&v| (v + 128) as u8).collect();
    for ts_uid in [TS_J2K_LOSSLESS, TS_J2K, TS_HTJ2K_LOSSLESS, TS_HTJ2K_RPCL, TS_HTJ2K] {
        let dcm = build_part10_encapsulated(ts_uid, 1, "MONOCHROME2", 4, 4, 8, 8, &[&cs_ht]);
        let ds = Part10Dataset::parse(&dcm, ParseLimits::default()).unwrap();
        let (mut p_out, mut ds_out, mut tbl_out) = ([0u8; 16], [0u8; 16], [0u8; 16]);
        let info = decode_j2k_part10_with_annex_c_into(&dcm, &mut p_out, J2kLimits::default()).unwrap();
        assert!(info.is_htj2k && info.transfer_syntax_uid == ts_uid && p_out.as_slice() == expected.as_slice());
        decode_j2k_part10_dataset_into(&ds, &mut ds_out, J2kLimits::default(), &mut WorkBudget::new(100_000)).unwrap();
        decode_j2k_part10_into(&dcm, &mut tbl_out, J2kLimits::default(), Some(tables)).unwrap();
        assert!(ds_out.as_slice() == expected.as_slice() && tbl_out.as_slice() == expected.as_slice());
    }
    assert!(!transfer_syntax_profile(TS_HTJ2K_LOSSLESS).encapsulated_decode);
}

#[test]
fn gate_codecs_jxl_modular_multi_bucket_ans_prefix_and_jbrd_roundtrip() {
    let px: [i64; 16] = [0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 2, 2, 1, 1, 2, 2];
    let mut residuals = Vec::with_capacity(16);
    for y in 0..4usize {
        for x in 0..4usize {
            let nv = if y > 0 { px[(y - 1) * 4 + x] } else { 0 };
            let wv = if x > 0 { px[y * 4 + x - 1] } else { nv };
            let nwv = if x > 0 && y > 0 { px[(y - 1) * 4 + x - 1] } else { wv };
            let r = px[y * 4 + x] - (nv + wv - nwv).clamp(nv.min(wv), nv.max(wv));
            residuals.push(if r >= 0 { (r as u16) * 2 } else { ((-r - 1) as u16) * 2 + 1 });
        }
    }
    let mut body = LsbBw::new();
    for &(v, n) in &[(0u64, 5usize), (0, 2), (4, 3), (0, 6), (0b1000, 4), (2, 3), (1, 2), (0b1011, 4), (0b1011, 4), (0b0111, 4), (0b11111, 5), (0, 1), (0b01111, 5), (0, 28)] {
        body.bits(v, n);
    }
    let (counts, cum) = ([512u32, 512, 1024, 0, 0, 0, 0, 2048], [0u32, 512, 1024, 2048, 2048, 2048, 2048, 2048]);
    let (mut state, mut words16) = (0x0001_0000u32, Vec::new());
    for &t in residuals.iter().rev() {
        let (f, st) = (counts[t as usize], cum[t as usize]);
        if (state >> 20) >= f { words16.push((state & 0xFFFF) as u16); state >>= 16; }
        state = ((state / f) << 12) | (st + (state % f));
    }
    body.bits((state >> 16) as u64, 16);
    body.bits((state & 0xFFFF) as u64, 16);
    for &w in words16.iter().rev() { body.bits(w as u64, 16); }
    body.align();

    let mut hdr = LsbBw::new();
    for &(v, n) in &[(0x0AFFu64, 16usize), (0, 1), (12, 11), (0, 3), (12, 11), (0, 5), (1, 1), (0, 5), (1, 2), (1, 2), (0, 1), (26, 6), (1, 2), (0, 2), (1, 1)] {
        hdr.bits(v, n);
    }
    hdr.align();
    for &(v, n) in &[(0u64, 3usize), (1, 1), (0, 5), (1, 2), (0, 5), (1, 1), (0, 2), (1, 1), (0, 2)] { hdr.bits(v, n); }
    hdr.bits(0, 1);
    hdr.align();
    hdr.bits(0, 2);
    hdr.bits(body.buf.len() as u64, 10);
    hdr.align();
    let mut jxl_cs = hdr.buf;
    jxl_cs.extend_from_slice(&body.buf);

    let mut jxl_out = [0u8; 16];
    let dec_cs = decode_jxl_codestream_into(&jxl_cs, 4, 4, 1, 8, 8, true, JxlLimits::default(), &mut jxl_out).unwrap();
    assert_eq!(dec_cs.encoding, JxlEncoding::Modular);
    for (i, &p) in px.iter().enumerate() { assert_eq!(jxl_out[i] as i64, p); }
    for ts_uid in [TS_JXL_LOSSLESS, TS_JXL_JPEG_RECOMPRESSION, TS_JXL] {
        let dcm_jxl = build_part10_encapsulated(ts_uid, 1, "MONOCHROME2", 4, 4, 8, 8, &[&jxl_cs]);
        let mut p10_out = [0u8; 16];
        let info = decode_jxl_part10_into(&dcm_jxl, &mut p10_out, JxlLimits::default()).unwrap();
        assert!(info.transfer_syntax_uid == ts_uid && p10_out == jxl_out);
    }

    let mut jpeg8 = vec![0xFF, 0xD8, 0xFF, 0xFE, 0x00, 0x04, b'Q', b'4', 0xFF, 0xDB, 0x00, 0x43, 0x00];
    jpeg8.extend_from_slice(&[2u8; 64]);
    jpeg8.extend_from_slice(&[
        0xFF, 0xC0, 0x00, 0x0B, 8, 0x00, 0x08, 0x00, 0x08, 1, 1, 0x11, 0,
        0xFF, 0xC4, 0x00, 0x15, 0x00, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x03,
        0xFF, 0xC4, 0x00, 0x16, 0x10, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF0, 0x22,
        0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, 0x00, 0x3F, 0x00, 0xDB, 0x15, 0xFF, 0xD9,
    ]);
    let (jbrd8, ext8) = encode_jbrd_box_from_jpeg(&jpeg8).unwrap();
    assert_eq!(reconstruct_jpeg_from_jbrd(&jbrd8, &ext8.blocks, JxlLimits::default()).unwrap(), jpeg8);

    let jpeg12 = sample_jpeg12();
    let (jbrd12, ext12) = encode_jbrd_box_from_jpeg(&jpeg12).unwrap();
    let mut container = vec![0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A];
    container.extend_from_slice(&((8 + jbrd12.len()) as u32).to_be_bytes());
    container.extend_from_slice(b"jbrd");
    container.extend_from_slice(&jbrd12);
    container.extend_from_slice(&((8 + jxl_cs.len()) as u32).to_be_bytes());
    container.extend_from_slice(b"jxlc");
    container.extend_from_slice(&jxl_cs);
    assert_eq!(extract_jbrd_payload(&container, JxlLimits::default()).unwrap(), jbrd12.as_slice());
    let recon12 = reconstruct_jpeg_from_jxl_container(&container, &ext12.blocks, JxlLimits::default()).unwrap();
    assert_eq!(recon12, jpeg12);
    assert_eq!(decode_jpeg_12bit_frame(&[jpeg12.as_slice()], 8, 16, 1, "MONOCHROME2").unwrap(), decode_jpeg_12bit_frame(&[recon12.as_slice()], 8, 16, 1, "MONOCHROME2").unwrap());
    let mut bad_blocks = ext12.blocks.clone();
    bad_blocks[0][0] ^= 1;
    assert!(matches!(reconstruct_jpeg_from_jbrd(&jbrd12, &bad_blocks, JxlLimits::default()), Err(JxlError::CorruptStream(_))));
}

#[test]
fn gate_reconstruction_fft_pow2_and_bluestein_non_pow2_roundtrip() {
    for n in [16usize, 64, 37, 53] {
        let sig: Vec<Cplx> = (0..n).map(|k| ((k as f64 * 0.37).cos() - 0.25, (k as f64 * 0.61).sin() * 0.75)).collect();
        let (spec, oracle) = (fft_1d(&sig).unwrap(), dft(&sig));
        assert_eq!(spec.len(), n);
        for (a, b) in spec.iter().zip(oracle.iter()) { assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9); }
        let (rec, rec_oracle) = (ifft_1d(&spec).unwrap(), idft(&oracle));
        for ((a, b), o) in rec.iter().zip(rec_oracle.iter()).zip(sig.iter()) {
            assert!((a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9 && (a.0 - o.0).abs() < 1e-10 && (a.1 - o.1).abs() < 1e-10);
        }
        let (mut sh, mut unsh) = (vec![(0.0, 0.0); n], vec![(0.0, 0.0); n]);
        fftshift_1d(&sig, &mut sh);
        ifftshift_1d(&sh, &mut unsh);
        assert_eq!(unsh, sig);
    }
    let (rows, cols) = (13usize, 19usize);
    let img2d: Vec<Cplx> = (0..rows * cols).map(|i| (((i * 5) % 23) as f64 * 0.2 - 1.0, ((i * 11) % 17) as f64 * -0.15)).collect();
    let k2d = fft_2d(&img2d, rows, cols, false).unwrap();
    let r2d = fft_2d(&k2d, rows, cols, true).unwrap();
    assert_eq!(r2d.len(), rows * cols);
    for (a, b) in img2d.iter().zip(r2d.iter()) { assert!((a.0 - b.0).abs() < 1e-10 && (a.1 - b.1).abs() < 1e-10); }
    let env = analytic_hilbert_envelope_1d(&(0..64).map(|i| (2.0 * PI * 4.0 * i as f64 / 64.0).cos()).collect::<Vec<_>>()).unwrap();
    assert!((env[32] - 1.0).abs() < 1e-10 && (apodization_weight(WindowKind::Hamming, 0.0) - 1.0).abs() < 1e-12);
}

#[test]
fn gate_reconstruction_ct_mri_pet_us_inference_and_anatomy_reg() {
    let (radius, n_det, det_sp, img_dim, pix_sp) = (6.0f64, 41usize, 0.5f64, 21usize, 0.75f64);
    let det_mid = 0.5 * (n_det - 1) as f64;
    let angles: Vec<f64> = (0..60).map(|i| PI * i as f64 / 60.0).collect();
    let mut sino = vec![0.0; 60 * n_det];
    for ai in 0..60 {
        for di in 0..n_det {
            let t = (di as f64 - det_mid) * det_sp;
            if t.abs() < radius { sino[ai * n_det + di] = 2.0 * (radius * radius - t * t).sqrt(); }
        }
    }
    for filt in [FbpFilter::RamLak, FbpFilter::SheppLogan] {
        let mut ct_out = vec![0.0; img_dim * img_dim];
        let prov = reconstruct_ct_fbp_into(
            &SinogramView { source_blake3: [0x11; 32], n_angles: 60, n_detectors: n_det, detector_spacing_mm: det_sp, center_detector_offset: 0.0, angles_rad: &angles, projections: &sino, units: ValueUnits::RescaleType("1/mm") },
            &FbpConfig { image_width: img_dim, image_height: img_dim, pixel_spacing_mm: [pix_sp, pix_sp], filter: filt, cutoff_ratio: 1.0, geometry: ProjectionGeometry::Parallel, apply_parker_short_scan: false },
            &mut ct_out,
        ).unwrap();
        assert!((ct_out[(img_dim / 2) * img_dim + (img_dim / 2)] - 1.0).abs() < 0.12 && prov.derivation_blake3 != [0x11; 32]);
    }

    let mri_obj: Vec<Cplx> = (0..16).map(|i| (i as f64 * 0.5 + 1.0, -(i as f64) * 0.25)).collect();
    let k2 = fft_2d(&mri_obj, 4, 4, false).unwrap();
    let rec2 = reconstruct_mri_kspace_2d(&k2, 4, 4, [0x22; 32], false, false, false).unwrap();
    let sos = combine_coils_sos(&[&rec2.complex, &rec2.complex], Some(&[1.0, 1.0])).unwrap();
    assert!((sos[0] - (2.0f64).sqrt() * mri_obj[0].0.hypot(mri_obj[0].1)).abs() < 1e-10);
    let k3 = fft_3d(&mri_obj, 2, 2, 4, false).unwrap();
    let rec3 = reconstruct_mri_kspace_3d(&k3, MriKspaceConfig { slices: 2, rows: 2, cols: 4, ifftshift_kspace: false, fftshift_image: false, unitary_scaling: false }, [0x23; 32]).unwrap();
    let sense = combine_coils_sense(&[&rec3.complex[..8], &rec3.complex[..8]], &[&vec![(1.0, 0.0); 8], &vec![(1.0, 0.0); 8]], 2, 4, 1, None, 0.0, [0x24; 32]).unwrap();
    assert!((sense.complex[0].0 - mri_obj[0].0).abs() < 1e-9);

    let mut pet_truth = vec![0.2f64; 81];
    pet_truth[40] = 8.0;
    let pet_angles: Vec<f64> = (0..12).map(|i| PI * i as f64 / 12.0).collect();
    let ones = vec![1.0f64; 12 * 13];
    let tmp_pet = PetSinogram { source_blake3: [0x33; 32], n_angles: 12, n_bins: 13, bin_spacing_mm: 2.0, angles_rad: &pet_angles, prompts: &ones, attenuation_factors: Some(&ones), normalization_factors: Some(&ones) };
    let prompts = forward_project_pet(&pet_truth, &tmp_pet, 2.0, 9, 9).unwrap();
    let mut pet_out = vec![0.0; 81];
    let osem = reconstruct_pet_osem_into(
        &PetSinogram { prompts: &prompts, ..tmp_pet },
        &OsemConfig { image_width: 9, image_height: 9, pixel_size_mm: 2.0, subsets: 3, iterations: 8, min_sensitivity: 1e-3, initial_activity: 1.0 },
        &mut pet_out,
    ).unwrap();
    assert!(pet_out[40] > pet_out[39] * 2.0 && (osem.total_forward_counts - osem.total_measured_counts).abs() / osem.total_measured_counts < 0.02);

    let elem_x: Vec<f64> = (0..16).map(|i| (i as f64 - 7.5) * 0.5).collect();
    let elem_z = vec![0.0; 16];
    let (c_mm_s, fs, n_samp) = (1_540_000.0, 20_000_000.0, 512usize);
    let mut rf = vec![0.0; 16 * n_samp];
    for e in 0..16 {
        let t_tot = 15.0 / c_mm_s + elem_x[e].hypot(15.0) / c_mm_s;
        for s in 0..n_samp {
            let dt = (s as f64 / fs) - t_tot;
            if dt.abs() < 3e-7 { rf[e * n_samp + s] = (-dt * dt / 2e-14).exp() * (2.0 * PI * 5e6 * dt).cos(); }
        }
    }
    let mut us_db = vec![0.0; 4];
    let us_res = beamform_ultrasound_das_into(
        &rf, n_samp,
        &UltrasoundArray { elem_x_mm: &elem_x, elem_z_mm: &elem_z, tx_mode: TxFocusMode::PlaneWave { angle_rad: 0.0 } },
        &DasConfig { sound_speed_mps: 1540.0, sample_rate_hz: fs, f_number: 1.5, min_aperture_half_mm: 2.0, apodization: WindowKind::Hanning, dynamic_range_db: 60.0, pixel_x_mm: &[0.0, 4.0], pixel_z_mm: &[15.0, 18.0] },
        [0x44; 32], &mut us_db,
    ).unwrap();
    assert!(us_res.peak_envelope > 0.5 && us_db[0] > us_db[1] + 6.0);

    let ds = Part10Dataset::parse(include_bytes!("../src/medical/dicom_dataset/fixtures/enhanced-defined.dcm"), ParseLimits::default()).unwrap();
    let fset = FrameSet::extract(&ds, FrameLimits::default()).unwrap();
    let tf = fset.frames[0].quantitative().modality.unwrap();
    let (mut cal, mut unc) = ([0.0; 3], [0.0; 3]);
    let inf = calibrate_and_infer_into(
        &[-500.0, 0.0, 500.0], &tf, ds.source_blake3(), 1,
        NonVisualDomain::Spectroscopy { axis_values: &[1.0, 2.0, 3.0], axis_units: "ppm" },
        None, UncertaintyBudget { stored_measurement_std: 1.0, epistemic_std: 0.5, ..Default::default() },
        "gate_cal", &mut cal, &mut unc,
    ).unwrap();
    assert!(cal[0] < cal[1] && cal[1] < cal[2] && inf.integral_trapezoid.unwrap().is_finite());
    assert_eq!(calibrate_and_infer_into(&[0.0], &tf, [0xEE; 32], 1, NonVisualDomain::ScalarSeries, None, UncertaintyBudget::default(), "bad", &mut cal, &mut unc).unwrap_err(), InferenceError::SourceIdentityMismatch);

    let plane = fset.frames[0].plane().unwrap();
    let rigid = AnatomyTransform::Rigid { rotation_3x3: [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], translation_mm: [1.0, 2.0, 3.0] };
    assert_eq!(register_patient_to_anatomy(&plane, ds.source_blake3(), 1, 1, "1.2.840.999.2", &rigid, None, 1e-6).unwrap_err(), AnatomyRegError::FrameOfReferenceMismatch);
    let cert = CrossFrameRegistrationCertificate { source_frame_of_reference_uid: plane.frame_of_reference_uid().unwrap(), target_frame_of_reference_uid: "1.2.840.999.2", certificate_blake3: [0xAB; 32] };
    let reg = register_patient_to_anatomy(&plane, ds.source_blake3(), 1, 1, "1.2.840.999.2", &rigid, Some(&cert), 1e-6).unwrap();
    assert!(reg.roundtrip_residual_mm < 1e-10 && reg.derivation_blake3 != ds.source_blake3());
}
