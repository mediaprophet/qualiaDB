use super::*;
use crate::medical::dicom_codecs::encapsulated::EncapLimits;

struct T87BitWriter {
    out: Vec<u8>,
    acc: u32,
    bits: u8,
    prev_ff: bool,
}
impl T87BitWriter {
    fn new() -> Self {
        Self { out: Vec::new(), acc: 0, bits: 0, prev_ff: false }
    }
    fn put_bit(&mut self, bit: u32) {
        self.acc = (self.acc << 1) | (bit & 1);
        self.bits += 1;
        let cap = if self.prev_ff { 7 } else { 8 };
        if self.bits == cap {
            let byte = self.acc as u8;
            self.out.push(byte);
            self.prev_ff = !self.prev_ff && byte == 0xFF;
            self.acc = 0;
            self.bits = 0;
        }
    }
    fn put_bits(&mut self, val: u32, n: u8) {
        for shift in (0..n).rev() {
            self.put_bit((val >> shift) & 1);
        }
    }
    fn put_golomb(&mut self, k: u8, glimit: u8, qbpp: u8, merr: i32) {
        let q = (merr as u32) >> k;
        if q < glimit as u32 {
            for _ in 0..q {
                self.put_bit(0);
            }
            self.put_bit(1);
            self.put_bits(merr as u32, k);
        } else {
            for _ in 0..glimit {
                self.put_bit(0);
            }
            self.put_bit(1);
            self.put_bits((merr - 1) as u32, qbpp);
        }
    }
    fn flush(mut self) -> Vec<u8> {
        if self.bits > 0 {
            let cap = if self.prev_ff { 7 } else { 8 };
            while self.bits < cap {
                self.put_bit(0);
            }
        }
        self.out
    }
}

fn encode_scan_ilv0(plane: &[i32], w: usize, h: usize, prec: u8, near: u8, dri: u16) -> Vec<u8> {
    let coding = headers::derive_coding(prec, near, None).unwrap();
    let mut ctx = context::ContextModel::new(coding);
    let j_tab = [0usize,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,5,5,6,6,7,7,8,9,10,11,12,13,14,15];
    let mut bw = T87BitWriter::new();
    let mut prev = vec![0i32; w + 2];
    let mut curr = vec![0i32; w + 2];
    let mut run_index = 0usize;
    let mut rst_num = 0u8;

    for y in 0..h {
        if dri > 0 && y > 0 && (y % dri as usize) == 0 {
            if bw.bits > 0 {
                let cap = if bw.prev_ff { 7 } else { 8 };
                bw.acc <<= cap - bw.bits;
                let b = bw.acc as u8;
                bw.out.push(b);
                bw.prev_ff = !bw.prev_ff && b == 0xFF;
                bw.bits = 0;
                bw.acc = 0;
            }
            if bw.prev_ff {
                bw.out.push(0x00);
                bw.prev_ff = false;
            }
            bw.out.push(0xFF);
            bw.out.push(0xD0 + (rst_num & 7));
            rst_num = rst_num.wrapping_add(1);
            ctx.reset_state();
            prev.fill(0);
            run_index = 0;
        }
        curr[0] = prev[1];
        prev[w + 1] = prev[w];
        let mut x = 0usize;
        while x < w {
            let px_idx = x + 1;
            let (ra, rb, rc, rd) = (curr[px_idx - 1], prev[px_idx], prev[px_idx - 1], prev[px_idx + 1]);
            let (d1, d2, d3) = (rd - rb, rb - rc, rc - ra);
            if d1.abs() <= near as i32 && d2.abs() <= near as i32 && d3.abs() <= near as i32 {
                let mut run_cnt = 0usize;
                while x < w {
                    let ix = plane[y * w + x];
                    if (ix - ra).abs() <= near as i32 {
                        curr[x + 1] = ra;
                        run_cnt += 1;
                        x += 1;
                    } else {
                        break;
                    }
                }
                while run_cnt >= (1usize << j_tab[run_index]) {
                    bw.put_bit(1);
                    run_cnt -= 1usize << j_tab[run_index];
                    if run_index < 31 {
                        run_index += 1;
                    }
                }
                if x == w {
                    if run_cnt > 0 {
                        bw.put_bit(1);
                    }
                } else {
                    bw.put_bit(0);
                    bw.put_bits(run_cnt as u32, j_tab[run_index] as u8);
                    let prev_ri = run_index;
                    if run_index > 0 {
                        run_index -= 1;
                    }
                    let ix = plane[y * w + x];
                    let rb_i = prev[x + 1];
                    let ri = usize::from((ra - rb_i).abs() <= near as i32);
                    let px = if ri == 1 { ra } else { rb_i };
                    let sign = if ri == 0 && ra > rb_i { -1 } else { 1 };
                    let mut err = sign * (ix - px);
                    if near > 0 {
                        err = if err > 0 { (err + near as i32) / (2 * near as i32 + 1) } else { (err - near as i32) / (2 * near as i32 + 1) };
                    }
                    err = mod_range_err(err, coding.range);
                    let mut rx = px + sign * err * (2 * near as i32 + 1);
                    let mspan = coding.range * (2 * near as i32 + 1);
                    if rx < -(near as i32) { rx += mspan; } else if rx > coding.maxval + near as i32 { rx -= mspan; }
                    curr[x + 1] = rx.clamp(0, coding.maxval);

                    let qctx = 365 + ri;
                    let temp = ctx.a[qctx] + if ri == 1 { ctx.n[qctx] >> 1 } else { 0 };
                    let mut k = 0u8;
                    while (ctx.n[qctx] << k) < temp { k += 1; }
                    let cond = k == 0 && 2 * ctx.nn[ri] < ctx.n[qctx];
                    let map = (err > 0 && cond) || (err < 0 && !cond);
                    let em = 2 * err.abs() - ri as i32 - i32::from(map);
                    let glim = coding.limit - j_tab[prev_ri] as u8 - 1;
                    bw.put_golomb(k, glim, coding.qbpp, em);
                    if err < 0 { ctx.nn[ri] += 1; }
                    ctx.a[qctx] += (em + 1 - ri as i32) >> 1;
                    if ctx.n[qctx] == coding.reset {
                        ctx.a[qctx] >>= 1;
                        ctx.n[qctx] >>= 1;
                        ctx.nn[ri] >>= 1;
                    }
                    ctx.n[qctx] += 1;
                    x += 1;
                }
            } else {
                let quant = |d: i32| -> i32 {
                    if d <= -coding.t3 { -4 } else if d <= -coding.t2 { -3 } else if d <= -coding.t1 { -2 }
                    else if d < -(near as i32) { -1 } else if d <= near as i32 { 0 } else if d < coding.t1 { 1 }
                    else if d < coding.t2 { 2 } else if d < coding.t3 { 3 } else { 4 }
                };
                let (mut q1, mut q2, mut q3) = (quant(d1), quant(d2), quant(d3));
                let sign = if q1 < 0 || (q1 == 0 && q2 < 0) || (q1 == 0 && q2 == 0 && q3 < 0) {
                    q1 = -q1; q2 = -q2; q3 = -q3; -1
                } else { 1 };
                let q = ((q1 * 9 + q2) * 9 + q3) as usize;
                let med = if rc >= ra.max(rb) { ra.min(rb) } else if rc <= ra.min(rb) { ra.max(rb) } else { ra + rb - rc };
                let px = (med + sign * ctx.c[q]).clamp(0, coding.maxval);
                let mut err = sign * (plane[y * w + x] - px);
                if near > 0 {
                    err = if err > 0 { (err + near as i32) / (2 * near as i32 + 1) } else { (err - near as i32) / (2 * near as i32 + 1) };
                }
                err = mod_range_err(err, coding.range);
                let mut rx = px + sign * err * (2 * near as i32 + 1);
                let mspan = coding.range * (2 * near as i32 + 1);
                if rx < -(near as i32) { rx += mspan; } else if rx > coding.maxval + near as i32 { rx -= mspan; }
                curr[px_idx] = rx.clamp(0, coding.maxval);

                let mut k = 0u8;
                while (ctx.n[q] << k) < ctx.a[q] { k += 1; }
                let special = near == 0 && k == 0 && 2 * ctx.b[q] <= -ctx.n[q];
                let merr = if err >= 0 { 2 * err + i32::from(special) } else { -2 * err - i32::from(!special) };
                bw.put_golomb(k, coding.limit - coding.qbpp - 1, coding.qbpp, merr);
                ctx.b[q] += err * (2 * near as i32 + 1);
                ctx.a[q] += err.abs();
                if ctx.n[q] == coding.reset {
                    ctx.a[q] >>= 1; ctx.b[q] >>= 1; ctx.n[q] >>= 1;
                }
                ctx.n[q] += 1;
                if ctx.b[q] <= -ctx.n[q] {
                    ctx.b[q] += ctx.n[q];
                    if ctx.c[q] > -128 { ctx.c[q] -= 1; }
                    if ctx.b[q] <= -ctx.n[q] { ctx.b[q] = -ctx.n[q] + 1; }
                } else if ctx.b[q] > 0 {
                    ctx.b[q] -= ctx.n[q];
                    if ctx.c[q] < 127 { ctx.c[q] += 1; }
                    if ctx.b[q] > 0 { ctx.b[q] = 0; }
                }
                x += 1;
            }
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    bw.flush()
}

fn mod_range_err(mut e: i32, range: i32) -> i32 {
    if e < 0 { e += range; }
    if e >= (range + 1) / 2 { e -= range; }
    e
}

fn build_single_scan_jpegls(w: u16, h: u16, prec: u8, near: u8, dri: u16, pixels: &[i32]) -> Vec<u8> {
    let mut s = vec![0xFF, 0xD8];
    s.extend_from_slice(&[0xFF, 0xF7, 0x00, 11, prec]);
    s.extend_from_slice(&h.to_be_bytes());
    s.extend_from_slice(&w.to_be_bytes());
    s.extend_from_slice(&[1, 1, 0x11, 0]);
    if dri > 0 {
        s.extend_from_slice(&[0xFF, 0xDD, 0x00, 0x04]);
        s.extend_from_slice(&dri.to_be_bytes());
    }
    s.extend_from_slice(&[0xFF, 0xDA, 0x00, 0x08, 1, 1, 0x00, near, 0, 0]);
    s.extend_from_slice(&encode_scan_ilv0(pixels, w as usize, h as usize, prec, near, dri));
    s.extend_from_slice(&[0xFF, 0xD9]);
    s
}

fn wrap_part10(ts_uid: &str, rows: u16, cols: u16, spp: u16, photo: &str, ba: u16, bs: u16, frag: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 128];
    out.extend_from_slice(b"DICM");
    let push_elem = |dst: &mut Vec<u8>, g: u16, e: u16, vr: &[u8; 2], val: &[u8]| {
        dst.extend_from_slice(&g.to_le_bytes());
        dst.extend_from_slice(&e.to_le_bytes());
        dst.extend_from_slice(vr);
        let pad = if val.len() % 2 == 1 { 1 } else { 0 };
        let tot = (val.len() + pad) as u16;
        dst.extend_from_slice(&tot.to_le_bytes());
        dst.extend_from_slice(val);
        if pad == 1 {
            dst.push(if matches!(vr, b"UI") { 0 } else { b' ' });
        }
    };
    let mut g2 = Vec::new();
    push_elem(&mut g2, 0x0002, 0x0010, b"UI", ts_uid.as_bytes());
    let g2_len = (g2.len() as u32).to_le_bytes();
    push_elem(&mut out, 0x0002, 0x0000, b"UL", &g2_len);
    out.extend_from_slice(&g2);
    push_elem(&mut out, 0x0028, 0x0002, b"US", &spp.to_le_bytes());
    push_elem(&mut out, 0x0028, 0x0004, b"CS", photo.as_bytes());
    push_elem(&mut out, 0x0028, 0x0010, b"US", &rows.to_le_bytes());
    push_elem(&mut out, 0x0028, 0x0011, b"US", &cols.to_le_bytes());
    push_elem(&mut out, 0x0028, 0x0100, b"US", &ba.to_le_bytes());
    push_elem(&mut out, 0x0028, 0x0101, b"US", &bs.to_le_bytes());
    push_elem(&mut out, 0x0028, 0x0102, b"US", &(bs - 1).to_le_bytes());
    push_elem(&mut out, 0x0028, 0x0103, b"US", &0u16.to_le_bytes());
    out.extend_from_slice(&0x7FE0u16.to_le_bytes());
    out.extend_from_slice(&0x0010u16.to_le_bytes());
    out.extend_from_slice(b"OB");
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.extend_from_slice(&0xFFFEu16.to_le_bytes());
    out.extend_from_slice(&0xE000u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    let fpad = frag.len() % 2;
    out.extend_from_slice(&0xFFFEu16.to_le_bytes());
    out.extend_from_slice(&0xE000u16.to_le_bytes());
    out.extend_from_slice(&((frag.len() + fpad) as u32).to_le_bytes());
    out.extend_from_slice(frag);
    if fpad == 1 { out.push(0); }
    out.extend_from_slice(&0xFFFEu16.to_le_bytes());
    out.extend_from_slice(&0xE0DDu16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out
}

#[test]
fn jpegls_8bit_lossless_and_12bit_ct_lossless_with_runs_roundtrip_bit_exact() {
    let (w, h) = (16usize, 12usize);
    let mut img8 = vec![0i32; w * h];
    for y in 0..h {
        for x in 0..w {
            img8[y * w + x] = if y < 4 { 42 } else if (y + x) % 3 == 0 { 255 } else { ((x * 17 + y * 29) & 255) as i32 };
        }
    }
    let cs8 = build_single_scan_jpegls(w as u16, h as u16, 8, 0, 4, &img8);
    let dcm8 = wrap_part10(TS_JPEGLS_LOSSLESS, h as u16, w as u16, 1, "MONOCHROME2", 8, 8, &cs8);
    let mut out8 = vec![0xAAu8; w * h + 16];
    let info8 = decode_jpegls_part10_into(&dcm8, &mut out8, JpegLsLimits::default()).unwrap();
    assert_eq!(info8.near_max, 0);
    for (i, &px) in img8.iter().enumerate() {
        assert_eq!(out8[i] as i32, px);
    }
    assert!(out8[w * h..].iter().all(|&b| b == 0xAA));

    let mut img12 = vec![0i32; w * h];
    for y in 0..h {
        for x in 0..w {
            img12[y * w + x] = if x < 6 { 1024 } else { ((y * 317 + x * 211) & 4095) as i32 };
        }
    }
    let cs12 = build_single_scan_jpegls(w as u16, h as u16, 12, 0, 0, &img12);
    let dcm12 = wrap_part10(TS_JPEGLS_LOSSLESS, h as u16, w as u16, 1, "MONOCHROME2", 16, 12, &cs12);
    let mut out16 = vec![0xA5u8; w * h * 2];
    decode_jpegls_part10_into(&dcm12, &mut out16, JpegLsLimits::default()).unwrap();
    for (i, &px) in img12.iter().enumerate() {
        assert_eq!(u16::from_le_bytes([out16[2 * i], out16[2 * i + 1]]) as i32, px);
    }
}

#[test]
fn jpegls_near_lossless_bound_lossless_violation_and_sentinel_guard() {
    let (w, h) = (12usize, 10usize);
    let img: Vec<i32> = (0..w * h).map(|i| ((i * 37) & 255) as i32).collect();
    let cs_near = build_single_scan_jpegls(w as u16, h as u16, 8, 3, 0, &img);

    let dcm_bad_lossless = wrap_part10(TS_JPEGLS_LOSSLESS, h as u16, w as u16, 1, "MONOCHROME2", 8, 8, &cs_near);
    let mut sentinel = vec![0x5Au8; w * h];
    assert_eq!(
        decode_jpegls_part10_into(&dcm_bad_lossless, &mut sentinel, JpegLsLimits::default()),
        Err(JpegLsError::LosslessModeViolation)
    );
    assert!(sentinel.iter().all(|&b| b == 0x5A));

    let dcm_ok_near = wrap_part10(TS_JPEGLS_NEAR_LOSSLESS, h as u16, w as u16, 1, "MONOCHROME2", 8, 8, &cs_near);
    let info = decode_jpegls_part10_into(&dcm_ok_near, &mut sentinel, JpegLsLimits::default()).unwrap();
    assert_eq!(info.near_max, 3);
    for (i, &orig) in img.iter().enumerate() {
        assert!((sentinel[i] as i32 - orig).abs() <= 3);
    }
    let truncated = &dcm_ok_near[..dcm_ok_near.len() - 6];
    let mut sentinel2 = vec![0x3Cu8; w * h];
    assert!(decode_jpegls_part10_into(truncated, &mut sentinel2, JpegLsLimits::default()).is_err());
    assert!(sentinel2.iter().all(|&b| b == 0x3C));
}
