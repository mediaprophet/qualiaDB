//! Bitstream cursors, Huffman decoding, restart handling, and IEEE-1180 12-bit IDCT for `jpeg_12bit`.

use super::jpeg::JpegError;
use core::f64::consts::{FRAC_1_SQRT_2, PI};

#[rustfmt::skip]
const ZIGZAG: [usize; 64] = [
     0,  1,  8, 16,  9,  2,  3, 10,
    17, 24, 32, 25, 18, 11,  4,  5,
    12, 19, 26, 33, 40, 48, 41, 34,
    27, 20, 13,  6,  7, 14, 21, 28,
    35, 42, 49, 56, 57, 50, 43, 36,
    29, 22, 15, 23, 30, 37, 44, 51,
    58, 59, 52, 45, 38, 31, 39, 46,
    53, 60, 61, 54, 47, 55, 62, 63,
];

pub(super) struct FragmentCursor<'a, 'b> {
    fragments: &'b [&'a [u8]],
    frag: usize,
    pos: usize,
}

impl<'a, 'b> FragmentCursor<'a, 'b> {
    pub(super) fn new(fragments: &'b [&'a [u8]]) -> Self {
        Self { fragments, frag: 0, pos: 0 }
    }
    pub(super) fn next_u8(&mut self) -> Result<u8, JpegError> {
        while self.frag < self.fragments.len() {
            let f = self.fragments[self.frag];
            if self.pos < f.len() {
                let b = f[self.pos];
                self.pos += 1;
                return Ok(b);
            }
            self.frag += 1;
            self.pos = 0;
        }
        Err(JpegError::CorruptStream("truncated 12-bit JPEG stream"))
    }
    pub(super) fn next_u16_be(&mut self) -> Result<u16, JpegError> {
        let hi = self.next_u8()?;
        let lo = self.next_u8()?;
        Ok(u16::from_be_bytes([hi, lo]))
    }
    pub(super) fn skip_exact(&mut self, mut n: usize) -> Result<(), JpegError> {
        while n > 0 {
            while self.frag < self.fragments.len() && self.pos >= self.fragments[self.frag].len() {
                self.frag += 1;
                self.pos = 0;
            }
            if self.frag >= self.fragments.len() {
                return Err(JpegError::CorruptStream("truncated 12-bit JPEG marker payload"));
            }
            let avail = self.fragments[self.frag].len() - self.pos;
            let step = avail.min(n);
            self.pos += step;
            n -= step;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(super) struct HuffTable {
    mincode: [i32; 17],
    maxcode: [i32; 17],
    valptr: [usize; 17],
    huffval: Vec<u8>,
}

impl HuffTable {
    pub(super) fn build(bits: &[u8; 16], huffval: Vec<u8>) -> Result<Self, JpegError> {
        let mut mincode = [0i32; 17];
        let mut maxcode = [-1i32; 17];
        let mut valptr = [0usize; 17];
        let mut code = 0i32;
        let mut p = 0usize;
        for len in 1..=16usize {
            let count = bits[len - 1] as usize;
            if count > 0 {
                if (code as usize).checked_add(count).is_none_or(|e| e > (1usize << len)) {
                    return Err(JpegError::CorruptStream("oversubscribed 12-bit JPEG Huffman table"));
                }
                valptr[len] = p;
                mincode[len] = code;
                p += count;
                code += count as i32;
                maxcode[len] = code - 1;
            }
            code <<= 1;
        }
        Ok(Self { mincode, maxcode, valptr, huffval })
    }
}

pub(super) struct ScanBitReader<'a, 'b, 'c> {
    cur: &'c mut FragmentCursor<'a, 'b>,
    bit_buf: u8,
    bit_cnt: u8,
    pending_marker: Option<u8>,
}

impl<'a, 'b, 'c> ScanBitReader<'a, 'b, 'c> {
    pub(super) fn new(cur: &'c mut FragmentCursor<'a, 'b>) -> Self {
        Self { cur, bit_buf: 0, bit_cnt: 0, pending_marker: None }
    }
    fn next_data_byte(&mut self) -> Result<u8, JpegError> {
        if let Some(m) = self.pending_marker {
            return Err(JpegError::CorruptStream(if m == 0xD9 {
                "premature EOI inside 12-bit JPEG scan"
            } else {
                "unexpected marker inside 12-bit JPEG scan"
            }));
        }
        let b = self.cur.next_u8()?;
        if b != 0xFF {
            return Ok(b);
        }
        let mut next = self.cur.next_u8()?;
        while next == 0xFF {
            next = self.cur.next_u8()?;
        }
        if next == 0x00 {
            return Ok(0xFF);
        }
        self.pending_marker = Some(next);
        Err(JpegError::CorruptStream("unexpected marker inside 12-bit JPEG entropy scan"))
    }
    fn read_bit(&mut self) -> Result<u32, JpegError> {
        if self.bit_cnt == 0 {
            self.bit_buf = self.next_data_byte()?;
            self.bit_cnt = 8;
        }
        self.bit_cnt -= 1;
        Ok(((self.bit_buf >> self.bit_cnt) & 1) as u32)
    }
    fn receive_extend(&mut self, n: usize) -> Result<i32, JpegError> {
        if n == 0 {
            return Ok(0);
        }
        if n > 15 {
            return Err(JpegError::CorruptStream("12-bit JPEG coefficient category > 15"));
        }
        let mut v = 0i32;
        for _ in 0..n {
            v = (v << 1) | (self.read_bit()? as i32);
        }
        if v < (1i32 << (n - 1)) {
            Ok(v - ((1i32 << n) - 1))
        } else {
            Ok(v)
        }
    }
    fn decode_huff(&mut self, tbl: &HuffTable) -> Result<u8, JpegError> {
        let mut len = 1usize;
        let mut code = self.read_bit()? as i32;
        while code > tbl.maxcode[len] {
            len += 1;
            if len > 16 {
                return Err(JpegError::CorruptStream("invalid 12-bit JPEG Huffman code"));
            }
            code = (code << 1) | (self.read_bit()? as i32);
        }
        let idx = tbl.valptr[len] + (code - tbl.mincode[len]) as usize;
        tbl.huffval.get(idx).copied().ok_or(JpegError::CorruptStream("12-bit Huffman table out of bounds"))
    }
    pub(super) fn consume_restart(&mut self, expected_idx: usize) -> Result<(), JpegError> {
        self.bit_cnt = 0;
        let marker = if let Some(m) = self.pending_marker.take() {
            m
        } else {
            let mut b = self.cur.next_u8()?;
            while b != 0xFF {
                b = self.cur.next_u8()?;
            }
            let mut m = self.cur.next_u8()?;
            while m == 0xFF {
                m = self.cur.next_u8()?;
            }
            m
        };
        let want = 0xD0 + (expected_idx & 7) as u8;
        if marker != want {
            return Err(JpegError::CorruptStream("mismatched RSTm restart marker"));
        }
        Ok(())
    }
    pub(super) fn finish_scan(&mut self) -> Result<Option<u8>, JpegError> {
        self.bit_cnt = 0;
        Ok(self.pending_marker.take())
    }
}

pub(super) fn idct_basis() -> [[f64; 8]; 8] {
    let mut c = [[0.0f64; 8]; 8];
    for x in 0..8 {
        for u in 0..8 {
            let cu = if u == 0 { FRAC_1_SQRT_2 } else { 1.0 };
            c[x][u] = 0.5 * cu * (((2 * x + 1) * u) as f64 * PI / 16.0).cos();
        }
    }
    c
}

fn idct_8x8_12bit(block: &[f64; 64], c: &[[f64; 8]; 8]) -> [u16; 64] {
    let mut tmp = [0.0f64; 64];
    for row in 0..8 {
        let base = row * 8;
        for x in 0..8 {
            let mut s = 0.0;
            for u in 0..8 {
                s += block[base + u] * c[x][u];
            }
            tmp[base + x] = s;
        }
    }
    let mut out = [0u16; 64];
    for x in 0..8 {
        for y in 0..8 {
            let mut s = 0.0;
            for v in 0..8 {
                s += tmp[v * 8 + x] * c[y][v];
            }
            out[y * 8 + x] = (s + 2048.0).round().clamp(0.0, 4095.0) as u16;
        }
    }
    out
}

pub(super) fn decode_block_into(
    rdr: &mut ScanBitReader<'_, '_, '_>,
    dc_tbl: &HuffTable,
    ac_tbl: &HuffTable,
    qtbl: &[u16; 64],
    dc_pred: &mut i32,
    basis: &[[f64; 8]; 8],
) -> Result<[u16; 64], JpegError> {
    let mut zz = [0i32; 64];
    let ssss = rdr.decode_huff(dc_tbl)? as usize;
    if ssss > 15 {
        return Err(JpegError::CorruptStream("12-bit JPEG DC category > 15"));
    }
    let diff = rdr.receive_extend(ssss)?;
    *dc_pred = dc_pred.checked_add(diff).ok_or(JpegError::Overflow)?;
    zz[0] = *dc_pred;

    let mut k = 1usize;
    while k < 64 {
        let rs = rdr.decode_huff(ac_tbl)?;
        if rs == 0x00 {
            break;
        }
        let run = (rs >> 4) as usize;
        let size = (rs & 0x0F) as usize;
        if size == 0 {
            if rs != 0xF0 {
                return Err(JpegError::CorruptStream("invalid AC symbol in 12-bit JPEG"));
            }
            k += 16;
            continue;
        }
        k += run;
        if k >= 64 {
            return Err(JpegError::CorruptStream("12-bit JPEG AC run past block end"));
        }
        zz[k] = rdr.receive_extend(size)?;
        k += 1;
    }

    let mut coeffs = [0.0f64; 64];
    for idx in 0..64 {
        coeffs[ZIGZAG[idx]] = (zz[idx] as f64) * (qtbl[idx] as f64);
    }
    Ok(idct_8x8_12bit(&coeffs, basis))
}

pub(super) fn write_block_to_plane(
    plane: &mut [u16],
    pw: usize,
    ph: usize,
    bx: usize,
    by: usize,
    blk: &[u16; 64],
) {
    for dy in 0..8 {
        let y = by * 8 + dy;
        if y >= ph {
            break;
        }
        for dx in 0..8 {
            let x = bx * 8 + dx;
            if x < pw {
                plane[y * pw + x] = blk[dy * 8 + dx];
            }
        }
    }
}
