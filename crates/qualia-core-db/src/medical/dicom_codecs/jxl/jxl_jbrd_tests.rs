//! Unit tests for Gap #3 (ISO/IEC 18181-1 general multi-bucket rANS histogram decoding & stream
//! roundtrip) and Gap #4 (`.111` `jbrd` byte-exact 8-bit/12-bit legacy JPEG reconstruction).
use super::*;
use crate::medical::dicom_codecs::jpeg_12bit::decode_jpeg_12bit_frame;
use crate::medical::dicom_codecs::jxl::entropy::{JxlBitReader, JxlEntropyDecoder};
use crate::medical::dicom_codecs::jxl::entropy_tables::{
    logcount_bucket_for_count, read_ans_table, AnsTable, PrefixTable, ANS_LOGCOUNT_PREFIX_LENGTHS,
};

struct LsbBitWriter {
    buf: Vec<u8>,
    bit_pos: usize,
}

impl LsbBitWriter {
    fn new() -> Self {
        Self { buf: Vec::new(), bit_pos: 0 }
    }
    fn write_bits(&mut self, val: u64, n: usize) {
        for i in 0..n {
            let (byte_idx, bit_idx) = (self.bit_pos >> 3, self.bit_pos & 7);
            if byte_idx >= self.buf.len() {
                self.buf.push(0);
            }
            if ((val >> i) & 1) != 0 {
                self.buf[byte_idx] |= 1u8 << bit_idx;
            }
            self.bit_pos += 1;
        }
    }
    fn write_bool(&mut self, v: bool) {
        self.write_bits(u64::from(v), 1);
    }
    fn write_varlen_u8(&mut self, v: usize) {
        if v == 0 {
            self.write_bool(false);
        } else {
            self.write_bool(true);
            if v == 1 {
                self.write_bits(0, 3);
            } else {
                let n = (usize::BITS - (v as u32).leading_zeros() - 1) as usize;
                self.write_bits(n as u64, 3);
                self.write_bits((v - (1usize << n)) as u64, n);
            }
        }
    }
}

fn write_multi_bucket_ans_header(bw: &mut LsbBitWriter, counts: &[u16], shift: usize, use_rle: bool) {
    assert!(counts.len() >= 3);
    bw.write_bool(false); // is_simple = false
    bw.write_bool(false); // is_flat = false
    if shift == 0 {
        bw.write_bool(false);
    } else if shift <= 4 {
        bw.write_bool(true);
        bw.write_bool(false);
        bw.write_bits((shift - 1) as u64, 2);
    } else {
        bw.write_bool(true);
        bw.write_bool(true);
        bw.write_bits((shift - 5) as u64, 3);
    }
    bw.write_varlen_u8(counts.len() - 3);

    let lc_tab = PrefixTable::from_lengths(&ANS_LOGCOUNT_PREFIX_LENGTHS).unwrap();
    let mut logcounts = Vec::with_capacity(counts.len());
    for &c in counts {
        let (lc, _, _) = logcount_bucket_for_count(c).unwrap();
        logcounts.push(lc);
    }

    let mut i = 0usize;
    while i < logcounts.len() {
        if use_rle && logcounts[i] == 0 {
            let mut run = 0usize;
            while i + run < logcounts.len() && logcounts[i + run] == 0 {
                run += 1;
            }
            if run >= 4 {
                let (cwd, len) = lc_tab.encode_symbol_lsb(13).unwrap();
                bw.write_bits(cwd as u64, len as usize);
                bw.write_varlen_u8(run - 4);
                i += run;
                continue;
            }
        }
        let (cwd, len) = lc_tab.encode_symbol_lsb(logcounts[i] as u16).unwrap();
        bw.write_bits(cwd as u64, len as usize);
        i += 1;
    }

    let mut omit_pos = 0usize;
    for (idx, &lc) in logcounts.iter().enumerate() {
        if lc > logcounts[omit_pos] {
            omit_pos = idx;
        }
    }
    for (idx, (&c, &lc)) in counts.iter().zip(logcounts.iter()).enumerate() {
        if idx == omit_pos || lc <= 1 {
            continue;
        }
        let rbits = (lc - 1) as usize;
        let prec = rbits.saturating_sub(shift);
        if prec > 0 {
            let extra = (c - (1u16 << rbits)) >> (rbits - prec);
            bw.write_bits(extra as u64, prec);
        }
    }
}

fn encode_rans_tokens(bw: &mut LsbBitWriter, counts: &[u16], tokens: &[u16]) {
    let tab = AnsTable::build(counts).unwrap();
    let mut cum = vec![0u32; counts.len()];
    let mut acc = 0u32;
    for (i, &c) in counts.iter().enumerate() {
        cum[i] = acc;
        acc += c as u32;
    }
    let mut state = 0x0001_0000u32;
    let mut words16: Vec<u16> = Vec::new();
    for &s in tokens.iter().rev() {
        let f = tab.freq[s as usize] as u32;
        let start = cum[s as usize];
        if (state >> 20) >= f {
            words16.push((state & 0xFFFF) as u16);
            state >>= 16;
        }
        let q = state / f;
        let j = state % f;
        state = (q << 12) | (start + j);
    }
    bw.write_bits((state >> 16) as u64, 16);
    bw.write_bits((state & 0xFFFF) as u64, 16);
    for &w in words16.iter().rev() {
        bw.write_bits(w as u64, 16);
    }
}

#[test]
fn jxl_multi_bucket_ans_histogram_and_stream_roundtrip_and_guards() {
    // 1. 5-symbol non-flat multi-bucket distribution (shift = 0):
    let counts5 = [500u16, 1500, 0, 800, 1296];
    let mut bw5 = LsbBitWriter::new();
    write_multi_bucket_ans_header(&mut bw5, &counts5, 0, false);
    let mut br5 = JxlBitReader::new(&bw5.buf);
    let tab5 = read_ans_table(&mut br5, 32).unwrap();
    assert_eq!(&tab5.freq[..5], &counts5);
    assert_eq!(tab5.alias_sym.len(), 4096);

    // 2. 10-symbol multi-bucket distribution with zero-run RLE (sym 13) and shift = 2:
    let counts10 = [256u16, 0, 0, 0, 0, 0, 512, 1024, 256, 2048];
    let mut bw10 = LsbBitWriter::new();
    write_multi_bucket_ans_header(&mut bw10, &counts10, 2, true);
    let mut br10 = JxlBitReader::new(&bw10.buf);
    let tab10 = read_ans_table(&mut br10, 32).unwrap();
    assert_eq!(&tab10.freq[..10], &counts10);

    // 3. Full JxlEntropyDecoder multi-bucket rANS stream roundtrip (`use_prefix = false`):
    let counts4 = [512u16, 1024, 1536, 1024];
    let tokens: [u16; 16] = [0, 2, 1, 3, 2, 2, 0, 1, 3, 2, 1, 0, 3, 2, 1, 2];
    let mut s_bw = LsbBitWriter::new();
    s_bw.write_bool(false); // lz77_enabled = false
    s_bw.write_bool(false); // use_prefix = false -> rANS!
    s_bw.write_bits(0, 2); // log_alpha_ans = 5 + 0 = 5 (max_sym = 32)
    // HybridUintConfig(log_alpha=5): split_exponent = 2 (ceil_log2(6) = 3 bits), msb = 0 (2 bits), lsb = 0 (2 bits)
    // With split_exponent = 2, tokens 0..4 decode directly to 0..4 without extra bits!
    s_bw.write_bits(2, 3);
    s_bw.write_bits(0, 2);
    s_bw.write_bits(0, 2);
    write_multi_bucket_ans_header(&mut s_bw, &counts4, 0, false);
    encode_rans_tokens(&mut s_bw, &counts4, &tokens);

    let mut s_br = JxlBitReader::new(&s_bw.buf);
    let mut dec = JxlEntropyDecoder::read(&mut s_br, 1, 1).unwrap();
    let mut decoded = Vec::with_capacity(tokens.len());
    for _ in 0..tokens.len() {
        decoded.push(dec.read_uint(&mut s_br, 0).unwrap() as u16);
    }
    dec.finalize().unwrap();
    assert_eq!(decoded, tokens);

    // 4. Fail-closed guards: non-4096 sum and overflowed non-omitted counts rejected.
    assert!(AnsTable::build(&[1000, 1000, 1000]).is_err());
    let mut bad_bw = LsbBitWriter::new();
    // Write 3 symbols where non-omitted counts sum to 2048 + 2048 = 4096 (>= 4096):
    bad_bw.write_bool(false);
    bad_bw.write_bool(false);
    bad_bw.write_bool(false); // shift = 0
    bad_bw.write_varlen_u8(0); // length = 3
    let lc_tab = PrefixTable::from_lengths(&ANS_LOGCOUNT_PREFIX_LENGTHS).unwrap();
    for _ in 0..3 {
        let (cwd, len) = lc_tab.encode_symbol_lsb(12).unwrap(); // lc=12 -> [2048..4095]
        bad_bw.write_bits(cwd as u64, len as usize);
    }
    // omit_pos = 0; symbols 1 and 2 each have rbits = 11 -> 2048 + 0 = 2048 -> sum_others = 4096 -> CorruptStream!
    bad_bw.write_bits(0, 11);
    bad_bw.write_bits(0, 11);
    let mut bad_br = JxlBitReader::new(&bad_bw.buf);
    assert!(matches!(read_ans_table(&mut bad_br, 32), Err(JxlError::CorruptStream(_))));
}

fn sample_8bit_sof0_jpeg_with_ac_and_zrl() -> Vec<u8> {
    let mut j = Vec::new();
    j.extend_from_slice(&[0xFF, 0xD8]); // SOI
    j.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x04, b'Q', b'4']); // COM
    j.extend_from_slice(&[0xFF, 0xDB, 0x00, 0x43, 0x00]); // DQT 8-bit
    j.extend_from_slice(&[2u8; 64]);
    j.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 8, 0x00, 0x08, 0x00, 0x08, 0x01, 0x01, 0x11, 0x00]); // SOF0 8x8
    // DC table: code 0 -> 0x00, code 1 -> 0x03 (cat 3)
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x00, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x03]);
    // AC table: len 1: '0' -> 0x00 (EOB); len 2: '10' -> 0xF0 (ZRL), '11' -> 0x22 (run=2, cat=2)
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x16, 0x10, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF0, 0x22]);
    j.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]); // SOS
    // Scan bits:
    // DC: cat 3 ('1'), bits '101' (+5) -> 4 bits: 1101
    // AC1: ZRL 0xF0 ('10') -> 16 zeros (k=1..=16)
    // AC2: 0x22 ('11'), bits '00' (-3 at k=19) -> 4 bits: 1100
    // EOB: 0x00 ('0') -> 1 bit: 0
    // Total = 11 bits: 1101_1011 (0xDB), then 000 + 5 pad bits '10101' -> 0001_0101 (0x15)
    j.extend_from_slice(&[0xDB, 0x15, 0xFF, 0xD9]);
    j
}

fn sample_12bit_sof1_jpeg_16x8_dri() -> Vec<u8> {
    let mut j = vec![0xFF, 0xD8, 0xFF, 0xDB, 0x00, 0x83, 0x10, 0x00, 0x08];
    for _ in 1..64 {
        j.extend_from_slice(&[0x00, 0x01]);
    }
    j.extend_from_slice(&[0xFF, 0xC1, 0x00, 0x0B, 12, 0x00, 0x08, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00]);
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x00, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0x0B]);
    j.extend_from_slice(&[0xFF, 0xC4, 0x00, 0x15, 0x10, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x00, 0xF0]);
    j.extend_from_slice(&[0xFF, 0xDD, 0x00, 0x04, 0x00, 0x01]); // DRI = 1
    j.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00]);
    j.extend_from_slice(&[0xC0, 0x07, 0xFF, 0xD0, 0x3F, 0xFF, 0xD9]);
    j
}

#[test]
fn jxl_jbrd_reconstructs_8bit_sof0_and_12bit_sof1_jpeg_bit_identical() {
    // 1. 8-bit SOF0 Baseline JPEG bit-identical roundtrip through jbrd + DCT coefficients:
    let jpeg8 = sample_8bit_sof0_jpeg_with_ac_and_zrl();
    let (jbrd8, ext8) = encode_jbrd_box_from_jpeg(&jpeg8).expect("encode 8-bit jbrd");
    assert_eq!(ext8.precision, 8);
    assert_eq!(ext8.width, 8);
    assert_eq!(ext8.height, 8);
    assert_eq!(ext8.blocks.len(), 1);
    assert_eq!(ext8.blocks[0][0], 5);
    assert_eq!(ext8.blocks[0][19], -3);
    let recon8 = reconstruct_jpeg_from_jbrd(&jbrd8, &ext8.blocks, JxlLimits::default()).expect("reconstruct 8-bit JPEG");
    assert_eq!(recon8, jpeg8);

    // 2. 12-bit SOF1 Extended JPEG (with DRI / RST0) bit-identical roundtrip + container extraction:
    let jpeg12 = sample_12bit_sof1_jpeg_16x8_dri();
    let (jbrd12, ext12) = encode_jbrd_box_from_jpeg(&jpeg12).expect("encode 12-bit jbrd");
    assert_eq!(ext12.precision, 12);
    assert_eq!(ext12.restart_interval, 1);
    assert_eq!(ext12.blocks.len(), 2);
    assert_eq!(ext12.blocks[0][0], 1024);
    assert_eq!(ext12.blocks[1][0], 0);

    // Wrap inside ISOBMFF JXL container (`jbrd` + minimal `jxlc` FF0A header):
    let mut container = vec![0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A];
    container.extend_from_slice(&((8 + jbrd12.len()) as u32).to_be_bytes());
    container.extend_from_slice(b"jbrd");
    container.extend_from_slice(&jbrd12);
    container.extend_from_slice(&12u32.to_be_bytes());
    container.extend_from_slice(b"jxlc\xFF\x0A\x00\x00");

    let recon12 = reconstruct_jpeg_from_jxl_container(&container, &ext12.blocks, JxlLimits::default())
        .expect("reconstruct 12-bit JPEG from JXL .111 container");
    assert_eq!(recon12, jpeg12);

    // Verify reconstructed 12-bit JPEG decodes to exact 12-bit pixel values via decode_jpeg_12bit_frame:
    let px12 = decode_jpeg_12bit_frame(&[recon12.as_slice()], 8, 16, 1, "MONOCHROME2").unwrap();
    assert_eq!(u16::from_le_bytes([px12[0], px12[1]]), 3072);
    assert_eq!(u16::from_le_bytes([px12[16], px12[17]]), 2048);

    // 3. Fail-closed verification: modified DCT coefficient or truncated jbrd fails closed!
    let mut tampered = ext12.blocks.clone();
    tampered[0][0] += 1;
    assert!(matches!(
        reconstruct_jpeg_from_jbrd(&jbrd12, &tampered, JxlLimits::default()),
        Err(JxlError::CorruptStream(_))
    ));
    assert!(matches!(
        reconstruct_jpeg_from_jbrd(&jbrd12, &ext12.blocks[..1], JxlLimits::default()),
        Err(JxlError::CorruptStream(_))
    ));
    assert!(matches!(
        reconstruct_jpeg_from_jbrd(&jbrd12[..jbrd12.len() - 3], &ext12.blocks, JxlLimits::default()),
        Err(JxlError::CorruptStream(_))
    ));
}
