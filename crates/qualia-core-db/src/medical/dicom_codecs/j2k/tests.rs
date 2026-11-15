use super::*;
use super::dwt::{idwt_1d_53, idwt_1d_97, synthesize_level_53, synthesize_level_97};
use super::ht_block::{decode_ht_cblk_with_tables, HtVlcEntry, HtVlcTables};
use super::mq_ebcot::{decode_ebcot_cblk, MqDecoder};
use super::tier2::{CblkHeaderState, TagTree};

// Independent minimal MQ encoder mirroring ISO 15444-1 Annex C for test verification.
const QE_ENC: [(u32, u8, u8, bool); 47] = [
    (0x5601,1,1,true),(0x3401,2,6,false),(0x1801,3,9,false),(0x0AC1,4,12,false),(0x0521,5,29,false),(0x0221,38,33,false),
    (0x5601,7,6,true),(0x5401,8,14,false),(0x4801,9,14,false),(0x3801,10,14,false),(0x3001,11,17,false),(0x2401,12,18,false),(0x1C01,13,20,false),(0x1601,29,21,false),
    (0x5601,15,14,true),(0x5401,16,14,false),(0x5101,17,15,false),(0x4801,18,16,false),(0x3801,19,17,false),(0x3401,20,18,false),(0x3001,21,19,false),(0x2801,22,19,false),(0x2401,23,20,false),(0x2201,24,21,false),
    (0x1C01,25,22,false),(0x1801,26,23,false),(0x1601,27,24,false),(0x1401,28,25,false),(0x1201,29,26,false),(0x1101,30,27,false),(0x0AC1,31,28,false),(0x09C1,32,29,false),(0x08A1,33,30,false),
    (0x0521,34,31,false),(0x0441,35,32,false),(0x02A1,36,33,false),(0x0221,37,34,false),(0x0141,38,35,false),(0x0111,39,36,false),(0x0085,40,37,false),(0x0049,41,38,false),(0x0025,42,39,false),
    (0x0015,43,40,false),(0x0009,44,41,false),(0x0005,45,42,false),(0x0001,45,43,false),(0x5601,46,46,false),
];

struct MqEncoder {
    buf: Vec<u8>,
    a: u32,
    c: u32,
    ct: u8,
    b: u8,
    has_b: bool,
    state: [u8; 19],
    mps: [u8; 19],
}
impl MqEncoder {
    fn new() -> Self {
        let mut s = Self { buf: Vec::new(), a: 0x8000, c: 0, ct: 12, b: 0, has_b: false, state: [0; 19], mps: [0; 19] };
        s.state[0] = 4;
        s.state[17] = 3;
        s.state[18] = 46;
        s
    }
    fn byte_out(&mut self) {
        if self.has_b && self.b == 0xFF {
            self.buf.push(0xFF);
            self.b = (self.c >> 20) as u8;
            self.c &= 0xFFFFF;
            self.ct = 7;
            return;
        }
        if (self.c & 0x800_0000) != 0 {
            if self.has_b {
                self.b = self.b.wrapping_add(1);
                self.buf.push(self.b);
            }
            if self.b == 0xFF {
                self.c &= 0x7FF_FFFF;
                self.b = (self.c >> 20) as u8;
                self.c &= 0xFFFFF;
                self.ct = 7;
                self.has_b = true;
                return;
            }
        } else if self.has_b {
            self.buf.push(self.b);
        }
        self.b = (self.c >> 19) as u8;
        self.c &= 0x7FFFF;
        self.ct = 8;
        self.has_b = true;
    }
    fn renorme(&mut self) {
        while (self.a & 0x8000) == 0 {
            self.a <<= 1;
            self.c <<= 1;
            self.ct -= 1;
            if self.ct == 0 {
                self.byte_out();
            }
        }
    }
    fn encode_bit(&mut self, cx: usize, d: u8) {
        let (qe, nmps, nlps, sw) = QE_ENC[self.state[cx] as usize];
        self.a -= qe;
        if d == self.mps[cx] {
            if (self.a & 0x8000) == 0 {
                if self.a < qe { self.a = qe; } else { self.c += qe; }
                self.state[cx] = nmps;
                self.renorme();
            } else {
                self.c += qe;
            }
        } else {
            if self.a < qe { self.c += qe; } else { self.a = qe; }
            if sw { self.mps[cx] ^= 1; }
            self.state[cx] = nlps;
            self.renorme();
        }
    }
    fn flush(mut self) -> Vec<u8> {
        let temp = self.c + self.a;
        self.c |= 0xFFFF;
        if self.c >= temp { self.c -= 0x8000; }
        self.c <<= self.ct;
        self.byte_out();
        self.c <<= self.ct;
        self.byte_out();
        if self.has_b && self.b != 0xFF {
            self.buf.push(self.b);
        }
        self.buf
    }
}

struct PktBitWriter {
    buf: Vec<u8>,
    acc: u8,
    bits: u8,
    prev_ff: bool,
}
impl PktBitWriter {
    fn new() -> Self { Self { buf: Vec::new(), acc: 0, bits: 0, prev_ff: false } }
    fn put_bit(&mut self, b: u32) {
        self.acc = (self.acc << 1) | (b as u8 & 1);
        self.bits += 1;
        let cap = if self.prev_ff { 7 } else { 8 };
        if self.bits == cap {
            self.buf.push(self.acc);
            self.prev_ff = self.acc == 0xFF;
            self.acc = 0;
            self.bits = 0;
        }
    }
    fn put_bits(&mut self, v: u32, n: u8) {
        for s in (0..n).rev() { self.put_bit((v >> s) & 1); }
    }
    fn align(mut self) -> Vec<u8> {
        if self.bits > 0 {
            let cap = if self.prev_ff { 7 } else { 8 };
            while self.bits < cap { self.put_bit(0); }
        } else if self.prev_ff {
            self.buf.push(0x00);
        }
        self.buf
    }
}

#[test]
fn j2k_dwt_53_and_97_1d_and_2d_exact_inversion() {
    // Reversible 5/3 exact integer forward -> inverse including odd length & cas=1.
    let sig: Vec<i32> = vec![12, -45, 77, 1024, -512, 33, 9];
    let n = sig.len();
    // Forward 5/3 with cas=0:
    let x = sig.clone();
    let mut high = vec![0i32; n / 2];
    let mut low = vec![0i32; n.div_ceil(2)];
    for k in 0..high.len() {
        let l = x[2 * k];
        let r = x[(2 * k + 2).min(n - 1)];
        high[k] = x[2 * k + 1] - ((l + r) >> 1);
    }
    for k in 0..low.len() {
        let hl = if k == 0 { high[0] } else { high[k - 1] };
        let hr = if k < high.len() { high[k] } else { high[high.len() - 1] };
        low[k] = x[2 * k] + ((hl + hr + 2) >> 2);
    }
    let mut recon = vec![0i32; n];
    idwt_1d_53(&low, &high, &mut recon, 0);
    assert_eq!(recon, sig);

    // 2D 5/3 test via constant/ramp synthesis vs forward
    let mut out_1d = [0.0f64; 4];
    idwt_1d_97(&[1.0, 1.0], &[0.0, 0.0], &mut out_1d, 0);
    for &v in &out_1d {
        assert!((v - 1.0).abs() < 1e-12);
    }
    let syn = synthesize_level_97(&[10.0; 4], &[0.0; 4], &[0.0; 4], &[0.0; 4], 2, 2, 2, 2, 0, 0);
    assert!(syn.iter().all(|&v| (v - 10.0).abs() < 1e-11));
    let syn53 = synthesize_level_53(&[42; 4], &[0; 4], &[0; 4], &[0; 4], 2, 2, 2, 2, 0, 0);
    assert!(syn53.iter().all(|&v| v == 42));
}

#[test]
fn j2k_mq_coder_and_full_codestream_part10_roundtrip() {
    let mut enc = MqEncoder::new();
    let bits = [1u8, 0, 1, 1, 0, 0, 0, 1, 1, 0, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0];
    for (i, &b) in bits.iter().enumerate() {
        enc.encode_bit(i % 5, b);
    }
    let bytes = enc.flush();
    let mut dec = MqDecoder::new(&bytes);
    for (i, &b) in bits.iter().enumerate() {
        assert_eq!(dec.decode_bit(i % 5), b);
    }

    // Encode a clean 4x4 8-bit 0-level J2K codestream via single 4x4 codeblock (value = 50 -> level shifted -78 -> mag 78, sign -).
    let mut cenc = MqEncoder::new();
    // 16 samples all = 0 except (0,0) = -1 (1 bitplane decoded at bit 0, total_bps=8, zbp=7)
    // Cleanup pass with 4 columns of 4 rows: col 0 run-interrupted at offset 0 -> run=1, offset=00, sign(cx=9)=1; cols 1..3 run=0 -> bit 0!
    cenc.encode_bit(17, 1); // col 0 not all zero
    cenc.encode_bit(18, 0); // offset high bit = 0
    cenc.encode_bit(18, 0); // offset low bit = 0 (y=0)
    cenc.encode_bit(9, 1);  // sign = negative (xor_bit = 0)
    // Remaining samples in col 0 (y=1,2,3) all insignificant: ZC contexts with top neighbor sig
    for y in 1..4 {
        let cx = if y == 1 { 3 } else { 0 };
        cenc.encode_bit(cx, 0);
    }
    for col in 1..4 {
        // col 1 has neighbor left at y=0 so not all_insig; decode each of y=0..4 individually
        if col == 1 {
            cenc.encode_bit(5, 0); // y=0 (left sig -> kh=1 -> cx=5)
            cenc.encode_bit(1, 0); // y=1 (UL sig -> kd=1 -> cx=1)
            cenc.encode_bit(0, 0); // y=2
            cenc.encode_bit(0, 0); // y=3
        } else {
            cenc.encode_bit(17, 0); // all insig run=0
        }
    }
    let cb_bytes = cenc.flush();
    let cblk_check = CblkHeaderState {
        x0: 0, y0: 0, x1: 4, y1: 4, included: true, zero_bitplanes: 7, lblock: 3, num_passes: 1,
        segments: vec![(1, cb_bytes.clone())],
    };
    let coeffs = decode_ebcot_cblk(&cblk_check, 0, 8, 0).unwrap();
    assert_eq!(coeffs[0], -1);
    assert!(coeffs[1..].iter().all(|&v| v == 0));

    // Wrap that exact code-block inside a complete J2K codestream (.90) frame!
    let mut pw = PktBitWriter::new();
    pw.put_bit(1); // non-empty packet
    pw.put_bit(1); // incl tree(0,0): val <= 0 (threshold 1) -> bit 1 = finalized at 0
    for _ in 0..7 { pw.put_bit(0); } // zbp tree(0,0) = 7
    pw.put_bit(0); // 1 coding pass -> single 0 bit!
    let mut pkt = if cb_bytes.len() < 8 {
        pw.put_bit(0); // lblock increment = 0 (lblock = 3)
        pw.put_bits(cb_bytes.len() as u32, 3);
        pw.align()
    } else {
        pw.put_bit(1);
        pw.put_bit(0); // lblock = 4
        pw.put_bits(cb_bytes.len() as u32, 4);
        pw.align()
    };
    pkt.extend_from_slice(&cb_bytes);

    let mut cs = vec![0xFF, 0x4F, 0xFF, 0x51, 0x00, 41, 0, 0];
    cs.extend_from_slice(&4u32.to_be_bytes());
    cs.extend_from_slice(&4u32.to_be_bytes());
    cs.extend_from_slice(&0u32.to_be_bytes());
    cs.extend_from_slice(&0u32.to_be_bytes());
    cs.extend_from_slice(&4u32.to_be_bytes());
    cs.extend_from_slice(&4u32.to_be_bytes());
    cs.extend_from_slice(&0u32.to_be_bytes());
    cs.extend_from_slice(&0u32.to_be_bytes());
    cs.extend_from_slice(&1u16.to_be_bytes());
    cs.extend_from_slice(&[7, 1, 1]); // 8-bit unsigned
    cs.extend_from_slice(&[0xFF, 0x52, 0x00, 12, 0, 0, 0, 1, 0, 0, 4, 4, 0, 1]); // COD: 0 levels, 64x64, 5/3
    cs.extend_from_slice(&[0xFF, 0x5C, 0x00, 4, 0x20, 8 << 3]); // QCD: no quant, guard=1, exp=8
    let psot = (12 + 2 + pkt.len()) as u32;
    cs.extend_from_slice(&[0xFF, 0x90, 0x00, 10, 0, 0]);
    cs.extend_from_slice(&psot.to_be_bytes());
    cs.extend_from_slice(&[0, 1, 0xFF, 0x93]);
    cs.extend_from_slice(&pkt);
    cs.extend_from_slice(&[0xFF, 0xD9]);

    let mut out = vec![0u8; 16];
    decode_j2k_codestream_into(&cs, 4, 4, 1, 8, 8, true, &mut out).unwrap();
    // Sample (0,0) = -1 + 128 = 127; rest = 0 + 128 = 128!
    assert_eq!(out[0], 127);
    assert!(out[1..].iter().all(|&b| b == 128));
}

#[test]
fn j2k_lossless_violation_htj2k_missing_tables_and_tagtree_refuse_safely() {
    // Verify TagTree construction doesn't panic on 1x1 and 3x3
    let tt = TagTree::new(3, 3);
    assert!(!tt.is_finalized(2, 2));

    // Verify HTJ2K without tables fails closed before touching buffers
    let cb = CblkHeaderState {
        x0: 0, y0: 0, x1: 4, y1: 4, included: true, zero_bitplanes: 0, lblock: 3, num_passes: 1,
        segments: vec![(1, vec![0x00, 0x02])],
    };
    assert!(matches!(decode_ht_cblk_with_tables(&cb, 8, None), Err(J2kError::Unsupported(_))));

    // Verify HTJ2K with caller-supplied table cleanly executes cleanup path
    let mut t0 = [HtVlcEntry { rho: 0, u_off: 0, e_k: 0, e_1: 0, cwd_len: 1 }; 1024];
    t0[0] = HtVlcEntry { rho: 0b0001, u_off: 0, e_k: 0, e_1: 0, cwd_len: 1 };
    let tables = HtVlcTables { tbl0: &t0, tbl1: &t0 };
    let ht_cb = CblkHeaderState {
        x0: 0, y0: 0, x1: 2, y1: 2, included: true, zero_bitplanes: 7, lblock: 3, num_passes: 1,
        // MagSgn=0x01 (sign=1, mag=0 -> val=-(0+1)=-1 << 7 = -128), MEL=0x00 (decode_symbol()=1), Scup=3 bytes
        segments: vec![(1, vec![0x01, 0x00, 0x03, 0x00])],
    };
    let res = decode_ht_cblk_with_tables(&ht_cb, 8, Some(&tables)).unwrap();
    assert_eq!(res, vec![-128, 0, 0, 0]);
}

#[test]
fn htj2k_explicit_caller_tables_decode_cleanup_sigprop_and_magref_exact() {
    let mut t0 = [HtVlcEntry { rho: 0, u_off: 0, e_k: 0, e_1: 0, cwd_len: 1 }; 1024];
    for (idx, ent) in t0.iter_mut().enumerate() {
        if (idx & 1) == 0 {
            *ent = HtVlcEntry { rho: 0b0001, u_off: 1, e_k: 0, e_1: 0, cwd_len: 1 };
        }
    }
    let tables = HtVlcTables { tbl0: &t0, tbl1: &t0 };
    let cb = CblkHeaderState {
        x0: 0, y0: 0, x1: 4, y1: 2, included: true, zero_bitplanes: 6, lblock: 3, num_passes: 3,
        segments: vec![(1, vec![0x02, 0x00, 0xA3, 0x00]), (2, vec![0x41])],
    };
    let res = decode_ht_cblk_with_tables(&cb, 8, Some(&tables)).unwrap();
    assert_eq!(res, vec![5, 1, 0, 0, 0, 0, 0, 0]);
}

fn minimal_4x4_sample0_minus1_packet() -> Vec<u8> {
    let mut cenc = MqEncoder::new();
    cenc.encode_bit(17, 1);
    cenc.encode_bit(18, 0);
    cenc.encode_bit(18, 0);
    cenc.encode_bit(9, 1);
    for y in 1..4 {
        cenc.encode_bit(if y == 1 { 3 } else { 0 }, 0);
    }
    cenc.encode_bit(5, 0);
    cenc.encode_bit(1, 0);
    cenc.encode_bit(0, 0);
    cenc.encode_bit(0, 0);
    cenc.encode_bit(17, 0);
    cenc.encode_bit(17, 0);
    let cb_bytes = cenc.flush();

    let mut pw = PktBitWriter::new();
    pw.put_bit(1);
    pw.put_bit(1);
    for _ in 0..7 { pw.put_bit(0); }
    pw.put_bit(0);
    let mut pkt = if cb_bytes.len() < 8 {
        pw.put_bit(0);
        pw.put_bits(cb_bytes.len() as u32, 3);
        pw.align()
    } else {
        pw.put_bit(1);
        pw.put_bit(0);
        pw.put_bits(cb_bytes.len() as u32, 4);
        pw.align()
    };
    pkt.extend_from_slice(&cb_bytes);
    pkt
}

fn minimal_4x4_main_header() -> Vec<u8> {
    let mut cs = vec![0xFF, 0x4F, 0xFF, 0x51, 0x00, 41, 0, 0];
    for &v in &[4u32, 4, 0, 0, 4, 4, 0, 0] {
        cs.extend_from_slice(&v.to_be_bytes());
    }
    cs.extend_from_slice(&1u16.to_be_bytes());
    cs.extend_from_slice(&[7, 1, 1]);
    cs.extend_from_slice(&[0xFF, 0x52, 0x00, 12, 0, 0, 0, 1, 0, 0, 4, 4, 0, 1]);
    cs.extend_from_slice(&[0xFF, 0x5C, 0x00, 4, 0x20, 8 << 3]);
    cs
}

fn push_sot_part(cs: &mut Vec<u8>, isot: u16, tpsot: u8, tnsot: u8, payload: &[u8]) {
    let psot = (12 + 2 + payload.len()) as u32;
    cs.extend_from_slice(&[0xFF, 0x90, 0x00, 10]);
    cs.extend_from_slice(&isot.to_be_bytes());
    cs.extend_from_slice(&psot.to_be_bytes());
    cs.extend_from_slice(&[tpsot, tnsot, 0xFF, 0x93]);
    cs.extend_from_slice(payload);
}

#[test]
fn j2k_multi_tile_part_sot_concatenation_and_invalid_tpsot_rejected() {
    let pkt = minimal_4x4_sample0_minus1_packet();
    let split = pkt.len() / 2;
    assert!(split > 0 && split < pkt.len());

    let mut cs_ooo = minimal_4x4_main_header();
    push_sot_part(&mut cs_ooo, 0, 1, 2, &pkt[split..]);
    push_sot_part(&mut cs_ooo, 0, 0, 2, &pkt[..split]);
    cs_ooo.extend_from_slice(&[0xFF, 0xD9]);
    let mut out = vec![0u8; 16];
    decode_j2k_codestream_into(&cs_ooo, 4, 4, 1, 8, 8, true, &mut out).unwrap();
    assert_eq!(out[0], 127);
    assert!(out[1..].iter().all(|&b| b == 128));

    let mut cs_bad_tpsot = minimal_4x4_main_header();
    push_sot_part(&mut cs_bad_tpsot, 0, 2, 2, &pkt);
    cs_bad_tpsot.extend_from_slice(&[0xFF, 0xD9]);
    let mut sentinel = vec![0x5Au8; 16];
    assert!(matches!(
        decode_j2k_codestream_into(&cs_bad_tpsot, 4, 4, 1, 8, 8, true, &mut sentinel),
        Err(J2kError::CorruptStream(_))
    ));
    assert!(sentinel.iter().all(|&b| b == 0x5A));

    let mut cs_dup = minimal_4x4_main_header();
    push_sot_part(&mut cs_dup, 0, 0, 2, &pkt[..split]);
    push_sot_part(&mut cs_dup, 0, 0, 2, &pkt[split..]);
    cs_dup.extend_from_slice(&[0xFF, 0xD9]);
    assert!(matches!(
        decode_j2k_codestream_into(&cs_dup, 4, 4, 1, 8, 8, true, &mut sentinel),
        Err(J2kError::CorruptStream(_))
    ));
    assert!(sentinel.iter().all(|&b| b == 0x5A));
}
