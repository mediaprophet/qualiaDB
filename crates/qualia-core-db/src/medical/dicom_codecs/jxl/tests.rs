//! Tests for ISO/IEC 18181 JPEG-XL (.110 / .111 / .112) decoder, container unwrapping,
//! entropy coding, Modular transforms, VarDCT transforms, and XYB color recovery.
use super::*;
use crate::medical::dicom_codecs::jxl::color::{linear_to_transfer, xyb_to_linear_srgb};
use crate::medical::dicom_codecs::jxl::container::default_opsin_inverse_matrix;
use crate::medical::dicom_codecs::jxl::modular::{apply_inverse_rct, smooth_tendency};
use crate::medical::dicom_codecs::jxl::modular_transforms::ModularChannel;
use crate::medical::dicom_codecs::jxl::vardct::{apply_inverse_block_transform, fdct_1d_into, VarDctTransform};

struct BitWriter {
    buf: Vec<u8>,
    bit_pos: usize,
}

impl BitWriter {
    fn new() -> Self { Self { buf: Vec::new(), bit_pos: 0 } }
    fn write_bits(&mut self, val: u64, n: usize) {
        for i in 0..n {
            let (byte_idx, bit_idx) = (self.bit_pos >> 3, self.bit_pos & 7);
            if byte_idx >= self.buf.len() { self.buf.push(0); }
            if ((val >> i) & 1) != 0 { self.buf[byte_idx] |= 1u8 << bit_idx; }
            self.bit_pos += 1;
        }
    }
    fn write_bool(&mut self, v: bool) { self.write_bits(u64::from(v), 1); }
    fn align_byte(&mut self) {
        let rem = self.bit_pos & 7;
        if rem != 0 { self.write_bits(0, 8 - rem); }
    }
    fn write_u32_sel(&mut self, sel: u64, val_minus_base: u64, bits: usize) {
        self.write_bits(sel, 2);
        if bits > 0 { self.write_bits(val_minus_base, bits); }
    }
    fn write_enum(&mut self, val: u32) {
        match val {
            0 => self.write_bits(0, 2),
            1 => self.write_bits(1, 2),
            2..=17 => self.write_u32_sel(2, (val - 2) as u64, 4),
            _ => self.write_u32_sel(3, (val - 18) as u64, 6),
        }
    }
    fn write_varlen_u16(&mut self, v: u32) {
        if v == 0 {
            self.write_bool(false);
        } else {
            self.write_bool(true);
            if v == 1 {
                self.write_bits(0, 4);
            } else {
                let n = 31 - v.leading_zeros() as usize;
                self.write_bits(n as u64, 4);
                self.write_bits((v - (1 << n)) as u64, n);
            }
        }
    }
}

fn pack_signed(v: i64) -> u32 {
    if v >= 0 { (v as u32) * 2 } else { ((-v - 1) as u32) * 2 + 1 }
}

/// Encode arbitrary signed residuals in `-8192..8191` via complex PrefixCode (16 uniform
/// 4-bit tokens) + `HybridUintConfig(split_exponent = 0, msb = 0, lsb = 0)`.
fn write_entropy_prefix_full(bw: &mut BitWriter, vals: &[i64]) {
    bw.write_bool(false); // lz77 = false
    bw.write_bool(true); // use_prefix = true
    // HybridUintConfig(log_alpha=15): split_exp=0 (4 bits), msb=0 (0 bits), lsb=0 (0 bits)
    bw.write_bits(0, 4);
    bw.write_varlen_u16(15); // alphabet_size = 16
    // Complex prefix header (hskip = 0): CL_ORDER = [1,2,3,4, 0,5,...]
    bw.write_bits(0, 2); // simple_or_skip = 0 (hskip = 0)
    // Position 0..3 => sym 1,2,3: len 0; Position 3 => sym 4: len 1 (3, 1, 0); Position 4..18 => len 0
    for pos in 0..18usize {
        if pos == 3 { bw.write_bits(3, 2); bw.write_bits(1, 1); bw.write_bool(false); }
        else { bw.write_bits(0, 2); }
    }
    let rev4 = |x: usize| -> u64 {
        (0..4).fold(0u64, |r, b| if ((x >> b) & 1) != 0 { r | (1 << (3 - b)) } else { r })
    };
    for &v in vals {
        let u = pack_signed(v);
        if u == 0 {
            bw.write_bits(rev4(0), 4);
        } else {
            let shift = (31 - u.leading_zeros()) as usize;
            let tok = shift + 1;
            assert!(tok < 16, "residual exceeds 14-bit HybridUint test budget: {v}");
            bw.write_bits(rev4(tok), 4);
            if shift > 0 { bw.write_bits((u - (1u32 << shift)) as u64, shift); }
        }
    }
}

fn build_jxl_gray_modular_codestream(w: u16, h: u16, bps: u8, pixels: &[i64], with_squeeze: bool) -> Vec<u8> {
    let mut bw = BitWriter::new();
    bw.write_bits(0xFF, 8);
    bw.write_bits(0x0A, 8);
    // SizeHeader: small = false
    bw.write_bool(false);
    bw.write_u32_sel(0, (h - 1) as u64, 9);
    bw.write_bits(0, 3); // ratio = 0 -> explicit width
    bw.write_u32_sel(0, (w - 1) as u64, 9);

    // ImageMetadata: all_default = false
    bw.write_bool(false);
    bw.write_bool(false); // extra_fields
    bw.write_bool(false); // fp = false
    if bps == 8 {
        bw.write_u32_sel(0, 0, 0);
    } else {
        bw.write_u32_sel(3, (bps - 1) as u64, 6);
    }
    bw.write_bool(true); // modular_16bit_suff
    bw.write_u32_sel(0, 0, 0); // 0 extra channels
    bw.write_bool(false); // xyb_encoded = false
    // ColourEncoding: all_default = false, Grayscale
    bw.write_bool(false);
    bw.write_bool(false); // want_icc = false
    bw.write_enum(1); // cspace = Grayscale
    bw.write_enum(1); // wp = D65
    bw.write_bool(false); // has_gamma = false
    bw.write_enum(8); // Linear
    bw.write_enum(1); // Relative intent
    bw.write_u32_sel(0, 0, 0); // extensions = 0 (U64 sel 0 -> 0)
    bw.write_bool(true); // TransformData all_default = true

    // FrameHeader: Regular, Modular
    bw.align_byte();
    bw.write_bool(false);
    bw.write_bits(0, 2); // Regular
    bw.write_bits(1, 1); // Modular
    bw.write_u32_sel(0, 0, 0); // flags = 0
    bw.write_bool(false); // do_ycbcr = false
    bw.write_u32_sel(0, 0, 0); // upsampling = 1
    bw.write_bits(1, 2); // group_size_shift = 1
    bw.write_u32_sel(0, 0, 0); // num_passes = 1
    bw.write_bool(false); // custom_size = false
    bw.write_bits(0, 2); // blend = Replace
    bw.write_bool(true); // is_last = true
    bw.write_u32_sel(0, 0, 0); // name_len = 0
    bw.write_bool(true); // RestorationFilter default
    bw.write_u32_sel(0, 0, 0); // FrameHeader extensions = 0

    // Encode modular frame body separately so we know exact TOC byte length!
    let mut body = BitWriter::new();
    body.write_bool(false); // have_global_tree = false -> Grad (pred=5) default leaf
    if with_squeeze {
        body.write_u32_sel(1, 0, 0); // 1 transform
        body.write_bits(2, 2); // Squeeze
        body.write_u32_sel(1, 0, 4); // num_steps = 1
        body.write_bool(true); // horizontal
        body.write_bool(true); // in_place
        body.write_u32_sel(0, 0, 3); // begin_c = 0
        body.write_u32_sel(0, 0, 0); // num_c = 1
    } else {
        body.write_u32_sel(0, 0, 0); // 0 transforms
    }

    let ww = w as usize;
    let hh = h as usize;
    let mut stream_vals = Vec::new();
    if with_squeeze {
        // Compute forward smooth Haar for each row, then residuals via Grad predictor!
        let w0 = ww.div_ceil(2);
        let w1 = ww / 2;
        let mut avg = vec![0i64; w0 * hh];
        let mut res = vec![0i64; w1 * hh];
        for y in 0..hh {
            for x in 0..w1 {
                let first = pixels[y * ww + 2 * x];
                let second = pixels[y * ww + 2 * x + 1];
                let diff = first - second;
                avg[y * w0 + x] = second + (diff >> 1);
            }
            if ww % 2 == 1 {
                avg[y * w0 + w0 - 1] = pixels[y * ww + ww - 1];
            }
            for x in 0..w1 {
                let a = avg[y * w0 + x];
                let b = if x > 0 { avg[y * w0 + x - 1] } else { a };
                let n = if x + 1 < w0 { avg[y * w0 + x + 1] } else { a };
                let diff = pixels[y * ww + 2 * x] - pixels[y * ww + 2 * x + 1];
                res[y * w1 + x] = diff - smooth_tendency(b, a, n);
            }
        }
        for (cw, ch_d) in [(w0, avg), (w1, res)] {
            for y in 0..hh {
                for x in 0..cw {
                    let nv = if y > 0 { ch_d[(y - 1) * cw + x] } else { 0 };
                    let wv = if x > 0 { ch_d[y * cw + x - 1] } else { nv };
                    let nwv = if x > 0 && y > 0 { ch_d[(y - 1) * cw + x - 1] } else { wv };
                    let pred = (nv + wv - nwv).clamp(nv.min(wv), nv.max(wv));
                    stream_vals.push(ch_d[y * cw + x] - pred);
                }
            }
        }
    } else {
        for y in 0..hh {
            for x in 0..ww {
                let nv = if y > 0 { pixels[(y - 1) * ww + x] } else { 0 };
                let wv = if x > 0 { pixels[y * ww + x - 1] } else { nv };
                let nwv = if x > 0 && y > 0 { pixels[(y - 1) * ww + x - 1] } else { wv };
                let pred = (nv + wv - nwv).clamp(nv.min(wv), nv.max(wv));
                stream_vals.push(pixels[y * ww + x] - pred);
            }
        }
    }
    write_entropy_prefix_full(&mut body, &stream_vals);
    body.align_byte();

    // TOC (1 entry)
    bw.write_bool(false); // permuted = false
    bw.align_byte();
    assert!(body.buf.len() < 1024);
    bw.write_u32_sel(0, body.buf.len() as u64, 10);
    bw.align_byte();

    let mut out = bw.buf;
    out.extend_from_slice(&body.buf);
    out
}

fn wrap_isobmff_jxl(boxes: &[(&[u8; 4], &[u8])]) -> Vec<u8> {
    let mut out = vec![0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A];
    for (tag, payload) in boxes {
        let len = (8 + payload.len()) as u32;
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(*tag);
        out.extend_from_slice(payload);
    }
    out
}

fn make_part10_jxl_fixture(ts_uid: &str, rows: u16, cols: u16, ba: u16, bs: u16, frames: &[&[u8]]) -> Vec<u8> {
    let mut out = vec![0u8; 128];
    out.extend_from_slice(b"DICM");
    let put = |v: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], data: &[u8]| {
        v.extend_from_slice(&g.to_le_bytes());
        v.extend_from_slice(&e.to_le_bytes());
        v.extend_from_slice(vr);
        if matches!(vr, b"OB" | b"OW") {
            v.extend_from_slice(&[0, 0]);
            v.extend_from_slice(&(data.len() as u32).to_le_bytes());
            v.extend_from_slice(data);
        } else {
            let pad = data.len() % 2;
            v.extend_from_slice(&((data.len() + pad) as u16).to_le_bytes());
            v.extend_from_slice(data);
            if pad == 1 {
                v.push(if vr == b"UI" { 0 } else { b' ' });
            }
        }
    };
    let mut meta = Vec::new();
    put(&mut meta, 0x0002, 0x0002, b"UI", b"1.2.840.10008.5.1.4.1.1.2");
    put(&mut meta, 0x0002, 0x0003, b"UI", b"1.2.3.4");
    put(&mut meta, 0x0002, 0x0010, b"UI", ts_uid.as_bytes());
    put(&mut out, 0x0002, 0x0000, b"UL", &(meta.len() as u32).to_le_bytes());
    out.extend_from_slice(&meta);

    put(&mut out, 0x0028, 0x0002, b"US", &1u16.to_le_bytes());
    put(&mut out, 0x0028, 0x0004, b"CS", b"MONOCHROME2");
    put(&mut out, 0x0028, 0x0008, b"IS", frames.len().to_string().as_bytes());
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
    // Empty BOT
    out.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0, 0, 0, 0, 0]);
    for frag in frames {
        out.extend_from_slice(&[0xFE, 0xFF, 0x00, 0xE0]);
        out.extend_from_slice(&((frag.len() as u32) + (frag.len() % 2) as u32).to_le_bytes());
        out.extend_from_slice(frag);
        if frag.len() % 2 == 1 {
            out.push(0);
        }
    }
    out.extend_from_slice(&[0xFE, 0xFF, 0xDD, 0xE0, 0, 0, 0, 0]);
    out
}

#[test]
fn jxl_modular_squeeze_12bit_ct_lossless_roundtrips_via_part10() {
    // 4x4 smooth 12-bit ramp encoded via Haar Squeeze + Grad predictor (residuals in -1..1!)
    let px: Vec<i64> = (0..16).map(|i| 1000 + (i as i64 % 4) + (i as i64 / 4)).collect();
    let cs = build_jxl_gray_modular_codestream(4, 4, 12, &px, true);
    let dcm = make_part10_jxl_fixture(TS_JXL_LOSSLESS, 4, 4, 16, 12, &[&cs]);
    let mut out = vec![0xAAu8; 4 * 4 * 2];
    let info = decode_jxl_part10_into(&dcm, &mut out, JxlLimits::default()).expect("decode JXL .110");
    assert!(!info.has_jbrd_box);
    assert_eq!(info.bits_stored, 12);
    for (i, &orig) in px.iter().enumerate() {
        let got = u16::from_le_bytes([out[2 * i], out[2 * i + 1]]) as i64;
        assert_eq!(got, orig, "mismatch at sample {i}");
    }
}

#[test]
fn jxl_isobmff_jxlp_out_of_order_reassembly_and_jbrd_flag() {
    let px: Vec<i64> = (0..16).map(|i| 20 + (i as i64 % 4)).collect();
    let cs = build_jxl_gray_modular_codestream(4, 4, 8, &px, false);
    let mid = cs.len() / 2;
    let mut p1 = (1u32 | 0x8000_0000).to_be_bytes().to_vec();
    p1.extend_from_slice(&cs[mid..]);
    let mut p0 = 0u32.to_be_bytes().to_vec();
    p0.extend_from_slice(&cs[..mid]);
    // Put jbrd + out-of-order jxlp chunks inside container!
    let container = wrap_isobmff_jxl(&[(b"jbrd", b"\x00\x01"), (b"jxlp", &p1), (b"jxlp", &p0)]);
    let dcm = make_part10_jxl_fixture(TS_JXL_JPEG_RECOMPRESSION, 4, 4, 8, 8, &[&container]);
    let mut out = vec![0u8; 16];
    let info = decode_jxl_part10_into(&dcm, &mut out, JxlLimits::default()).expect("decode JXL .111 container");
    assert!(info.has_jbrd_box);
    for (i, &orig) in px.iter().enumerate() {
        assert_eq!(out[i] as i64, orig);
    }
}

#[test]
fn jxl_vardct_fdct_idct_and_xyb_roundtrip_and_rct_exactness() {
    for &n in &[4usize, 8, 16] {
        let mut coeffs = vec![0.0; n * n];
        let mut spat = vec![0.0; n * n];
        let orig: Vec<f64> = (0..n * n).map(|i| ((i * 7) % 13) as f64 * 0.25 - 1.0).collect();
        let mut tmp = vec![0.0; n * n];
        for r in 0..n {
            fdct_1d_into(&orig[r * n..], &mut tmp[r * n..], 1, 1, n);
        }
        for c in 0..n {
            fdct_1d_into(&tmp[c..], &mut coeffs[c..], n, n, n);
        }
        apply_inverse_block_transform(VarDctTransform::Dct(n, n), &coeffs, &mut spat);
        for (a, b) in orig.iter().zip(spat.iter()) {
            assert!((a - b).abs() < 1e-10, "FDCT/IDCT {n}x{n} mismatch: {a} vs {b}");
        }
    }

    // XYB roundtrip on neutral D65 white [1.0, 1.0, 1.0]
    let inv = default_opsin_inverse_matrix();
    let bias = [0.003_793_073_255_275_449_3_f64; 3];
    let c0 = bias[0].cbrt();
    let y_white = (1.0 + bias[0]).cbrt() - c0;
    let lin = xyb_to_linear_srgb(0.0, y_white, y_white, &inv, &bias);
    for ch in lin {
        assert!((ch - 1.0).abs() < 1e-10, "XYB white roundtrip failed: {ch}");
    }
    assert!((linear_to_transfer(1.0, JxlTransferFunction::Srgb) - 1.0).abs() < 1e-12);

    // Modular SubtractGreen RCT roundtrip
    let mut chans = vec![
        ModularChannel { width: 2, height: 1, hshift: 0, vshift: 0, data: vec![100, 50] },
        ModularChannel { width: 2, height: 1, hshift: 0, vshift: 0, data: vec![20, -10] },
        ModularChannel { width: 2, height: 1, hshift: 0, vshift: 0, data: vec![-5, 15] },
    ];
    apply_inverse_rct(&mut chans, 0, 3).unwrap(); // G=100, R=120, B=95; G=50, R=40, B=65
    assert_eq!(chans[0].data, vec![100, 50]);
    assert_eq!(chans[1].data, vec![120, 40]);
    assert_eq!(chans[2].data, vec![95, 65]);
}

#[test]
fn jxl_truncation_and_dimension_mismatch_preserve_sentinel() {
    let px: Vec<i64> = (0..16).map(|i| 10 + (i as i64 % 2)).collect();
    let mut cs = build_jxl_gray_modular_codestream(4, 4, 8, &px, false);
    cs.truncate(cs.len() - 3);
    let dcm_trunc = make_part10_jxl_fixture(TS_JXL_LOSSLESS, 4, 4, 8, 8, &[&cs]);
    let mut out = vec![0x5Au8; 32];
    assert!(decode_jxl_part10_into(&dcm_trunc, &mut out, JxlLimits::default()).is_err());
    assert!(out.iter().all(|&b| b == 0x5A));

    let cs_ok = build_jxl_gray_modular_codestream(4, 4, 8, &px, false);
    let dcm_bad_dim = make_part10_jxl_fixture(TS_JXL, 8, 4, 8, 8, &[&cs_ok]);
    assert_eq!(
        decode_jxl_part10_into(&dcm_bad_dim, &mut out, JxlLimits::default()).unwrap_err(),
        JxlError::DimensionMismatch
    );
    assert!(out.iter().all(|&b| b == 0x5A));

    // Also verify VarDCT frame fails closed under TS_JXL_LOSSLESS (.110) and decodes under TS_JXL (.112)
    let mut vbw = BitWriter::new();
    vbw.write_bits(0xFF, 8);
    vbw.write_bits(0x0A, 8);
    vbw.write_bool(true); // small 8x8 -> height=8
    vbw.write_bits(0, 5);
    vbw.write_bits(1, 3); // 1:1 -> width=8
    vbw.write_bool(true); // ImageMetadata default (8-bit XYB sRGB)
    vbw.write_bool(true); // TransformData default
    vbw.align_byte();
    vbw.write_bool(false); // FrameHeader explicit
    vbw.write_bits(0, 2); // Regular
    vbw.write_bits(0, 1); // VarDCT!
    vbw.write_u32_sel(0, 0, 0); // flags = 0
    vbw.write_u32_sel(0, 0, 0); // upsampling = 1
    vbw.write_bits(3, 3); // x_qm_scale = 3
    vbw.write_bits(2, 3); // b_qm_scale = 2
    vbw.write_u32_sel(0, 0, 0); // num_passes = 1
    vbw.write_bool(false); // custom_size = false
    vbw.write_bits(0, 2); // blend = Replace
    vbw.write_bool(true); // is_last = true
    vbw.write_u32_sel(0, 0, 0); // name_len = 0
    vbw.write_bool(true); // RestorationFilter default
    vbw.write_u32_sel(0, 0, 0); // extensions = 0
    let mut vbody = BitWriter::new();
    vbody.write_u32_sel(0, 63, 11); // global_scale = 64 / 65536
    vbody.write_u32_sel(0, 0, 0); // quant_lf = 16
    // 4-ctx simple context map (1 bit per ctx -> 0) + 1 prefix table encoding 0 for CfL, tf=DCT8x8(0), & DC+AC coefs
    let mut vvals = vec![0i64; 2 + 1 + 3 * 64]; // 2 cfl + 1 block_tf + 3*64 coeffs (neutral gray XYB)
    vvals[3] = 20;
    vvals[3 + 128] = 20;
    vbody.write_bool(false); // lz77 = false
    vbody.write_bool(true); // simple context map
    vbody.write_bits(0, 2); // 0 bits/entry -> all ctx -> hist 0
    vbody.write_bool(true); // use_prefix
    vbody.write_bits(0, 4); // split=0
    vbody.write_varlen_u16(15);
    vbody.write_bits(0, 2);
    for pos in 0..18usize {
        if pos == 3 { vbody.write_bits(3, 2); vbody.write_bits(1, 1); vbody.write_bool(false); }
        else { vbody.write_bits(0, 2); }
    }
    let rev4 = |x: usize| -> u64 {
        let mut r = 0u64;
        for b in 0..4 { if ((x >> b) & 1) != 0 { r |= 1 << (3 - b); } }
        r
    };
    for &val in &vvals {
        let u = pack_signed(val);
        if u == 0 { vbody.write_bits(rev4(0), 4); }
        else {
            let s = (31 - u.leading_zeros()) as usize;
            vbody.write_bits(rev4(s + 1), 4);
            if s > 0 { vbody.write_bits((u - (1 << s)) as u64, s); }
        }
    }
    vbody.align_byte();
    vbw.write_bool(false); // permuted = false
    vbw.align_byte();
    vbw.write_u32_sel(0, vbody.buf.len() as u64, 10);
    vbw.align_byte();
    let mut vcs = vbw.buf;
    vcs.extend_from_slice(&vbody.buf);

    let mut rgb_out = vec![0x77u8; 8 * 8 * 3];
    assert_eq!(
        decode_jxl_codestream_into(&vcs, 8, 8, 3, 8, 8, true, JxlLimits::default(), &mut rgb_out).unwrap_err(),
        JxlError::LosslessModeViolation
    );
    assert!(rgb_out.iter().all(|&b| b == 0x77));
    let info_lossy = decode_jxl_codestream_into(&vcs, 8, 8, 3, 8, 8, false, JxlLimits::default(), &mut rgb_out)
        .expect("decode VarDCT lossy frame");
    assert_eq!(info_lossy.encoding, JxlEncoding::VarDct);
    assert!(rgb_out[0] > 0 && rgb_out[0] == rgb_out[1] && rgb_out[1] == rgb_out[2]);
}

#[test]
fn jxl_extract_jbrd_payload_and_truncation_guards() {
    let px: Vec<i64> = (0..16).map(|i| 20 + (i as i64 % 4)).collect();
    let cs = build_jxl_gray_modular_codestream(4, 4, 8, &px, false);
    let jbrd_expected = b"\xAA\x55\x00\x01";
    let with_jbrd = wrap_isobmff_jxl(&[(b"jbrd", jbrd_expected), (b"jxlc", &cs)]);
    assert_eq!(extract_jbrd_payload(&with_jbrd, JxlLimits::default()).unwrap(), jbrd_expected);

    // Missing jbrd fails closed.
    let without_jbrd = wrap_isobmff_jxl(&[(b"jxlc", &cs)]);
    assert!(matches!(extract_jbrd_payload(&without_jbrd, JxlLimits::default()), Err(JxlError::CorruptStream(_))));

    // Truncated jbrd box fails closed cleanly.
    let mut truncated = Vec::new();
    truncated.extend_from_slice(&[0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A]);
    truncated.extend_from_slice(&32u32.to_be_bytes());
    truncated.extend_from_slice(b"jbrd");
    truncated.extend_from_slice(b"\x01\x02");
    assert!(matches!(extract_jbrd_payload(&truncated, JxlLimits::default()), Err(JxlError::CorruptStream(_))));

    // Duplicate or empty jbrd box fails closed cleanly.
    let empty_jbrd = wrap_isobmff_jxl(&[(b"jbrd", b""), (b"jxlc", &cs)]);
    assert!(matches!(extract_jbrd_payload(&empty_jbrd, JxlLimits::default()), Err(JxlError::CorruptStream(_))));
    let dup_jbrd = wrap_isobmff_jxl(&[(b"jbrd", jbrd_expected), (b"jbrd", b"\x02"), (b"jxlc", &cs)]);
    assert!(matches!(extract_jbrd_payload(&dup_jbrd, JxlLimits::default()), Err(JxlError::CorruptStream(_))));
}
