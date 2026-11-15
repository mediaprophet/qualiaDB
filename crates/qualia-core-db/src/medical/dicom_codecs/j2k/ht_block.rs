//! ISO/IEC 15444-15 (ITU-T T.814) High-Throughput (HTJ2K / FBCOT) block coder:
//! MagSgn forward stream, MEL run-length coder, backward VLC stream with >0x8F
//! byte unstuffing, UVLC exponent decoder and HT SigProp/MagRef refinement passes.
use super::markers::J2kError;
use super::tier2::CblkHeaderState;

/// ISO/IEC 15444-15 §7.3.6 Table 4 MEL exponent schedule.
pub const MEL_E: [u8; 13] = [0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 4, 5];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HtVlcEntry {
    pub rho: u8,
    pub u_off: u8,
    pub e_k: u8,
    pub e_1: u8,
    pub cwd_len: u8,
}

/// Caller-supplied ITU-T T.814 Annex C CxtVLC lookup tables indexed by
/// `(context << 7) | (lsb_codeword_7bits)`.
pub struct HtVlcTables<'a> {
    /// Table 0 (`y_q == 0` initial quad row), 8 contexts * 128 entries = 1024.
    pub tbl0: &'a [HtVlcEntry; 1024],
    /// Table 1 (`y_q > 0` subsequent quad rows), 8 contexts * 128 entries = 1024.
    pub tbl1: &'a [HtVlcEntry; 1024],
}

pub struct MelDecoder<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    bits: u8,
    prev_ff: bool,
    state: usize,
    run: u32,
}

impl<'a> MelDecoder<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, acc: 0, bits: 0, prev_ff: false, state: 0, run: 0 }
    }
    fn read_bit(&mut self) -> u32 {
        if self.bits == 0 {
            let b = self.data.get(self.pos).copied().unwrap_or(0xFF);
            self.pos += usize::from(self.pos < self.data.len());
            if self.prev_ff {
                self.acc = (b & 0x7F) as u32;
                self.bits = 7;
            } else {
                self.acc = b as u32;
                self.bits = 8;
            }
            self.prev_ff = b == 0xFF;
        }
        self.bits -= 1;
        (self.acc >> self.bits) & 1
    }
    fn read_bits(&mut self, n: u8) -> u32 {
        let mut v = 0u32;
        for _ in 0..n {
            v = (v << 1) | self.read_bit();
        }
        v
    }
    /// Decode next MEL symbol bit (0 = both zero quads skip, 1 = decode VLC).
    pub fn decode_symbol(&mut self) -> u32 {
        if self.run == 0 {
            let k = MEL_E[self.state];
            if self.read_bit() == 1 {
                self.run = (1u32 << k).saturating_sub(1);
                if self.state < 12 {
                    self.state += 1;
                }
                if self.run == 0 {
                    return 0;
                }
            } else {
                self.run = self.read_bits(k);
                if self.state > 0 {
                    self.state -= 1;
                }
                return 1;
            }
        }
        self.run -= 1;
        0
    }
}

pub struct RevVlcReader<'a> {
    data: &'a [u8],
    pos: usize, // points next byte left of cursor (decrements downward)
    acc: u32,
    bits: u8,
    prev_gt_8f: bool,
}

impl<'a> RevVlcReader<'a> {
    pub fn new(segment: &'a [u8], scup: usize) -> Result<Self, J2kError> {
        if segment.len() < 2 || scup < 2 || scup > segment.len() {
            return Err(J2kError::CorruptStream("invalid HTJ2K Lcup/Scup bounds"));
        }
        let b_last = segment[segment.len() - 1];
        let b_prev = segment[segment.len() - 2];
        let mut r = Self {
            data: &segment[segment.len() - scup..],
            pos: scup - 2,
            acc: 0,
            bits: 0,
            prev_gt_8f: b_last > 0x8F,
        };
        let init_bits = if r.prev_gt_8f { 3 } else { 4 };
        r.acc = ((b_prev >> 4) & ((1 << init_bits) - 1)) as u32;
        r.bits = init_bits;
        r.prev_gt_8f = b_prev > 0x8F;
        Ok(r)
    }
    #[inline]
    fn fill(&mut self) {
        while self.bits <= 24 && self.pos > 0 {
            self.pos -= 1;
            let b = self.data[self.pos];
            let (avail, mask) = if self.prev_gt_8f {
                (7, 0x7Fu8)
            } else {
                (8, 0xFFu8)
            };
            self.acc |= ((b & mask) as u32) << self.bits;
            self.bits += avail;
            self.prev_gt_8f = b > 0x8F;
        }
    }
    #[inline]
    pub fn peek_bits(&mut self, n: u8) -> u32 {
        self.fill();
        if self.bits < n {
            // Virtual infinite 1-padding at reverse EOF per ISO 15444-15 §7.3.5.
            let missing = n - self.bits;
            self.acc |= ((1u32 << missing) - 1) << self.bits;
            self.bits = n;
        }
        self.acc & ((1u32 << n) - 1)
    }
    #[inline]
    pub fn consume(&mut self, n: u8) {
        self.peek_bits(n);
        self.acc >>= n;
        self.bits -= n;
    }
    pub fn read_bits(&mut self, n: u8) -> u32 {
        let v = self.peek_bits(n);
        self.consume(n);
        v
    }
    pub fn decode_uvlc_pair(&mut self, y_q0: bool, u_off0: u8, u_off1: u8) -> (u32, u32) {
        if u_off0 == 0 && u_off1 == 0 {
            return (1, 1);
        }
        let read_single = |vr: &mut Self| -> u32 {
            let pfx3 = vr.peek_bits(3);
            if (pfx3 & 1) == 1 {
                vr.consume(1);
                1
            } else if (pfx3 & 3) == 2 {
                vr.consume(2);
                2
            } else if pfx3 == 4 {
                vr.consume(3);
                3 + vr.read_bits(1)
            } else {
                vr.consume(3);
                if vr.read_bits(1) == 1 {
                    5 + vr.read_bits(5)
                } else {
                    37 + vr.read_bits(5) // extended UVLC prefix
                }
            }
        };
        if u_off0 == 1 && u_off1 == 1 {
            if y_q0 {
                // ISO 15444-15 §7.3.6 mode 4 initial row differential.
                if self.read_bits(1) == 1 {
                    let v0 = read_single(self);
                    return (v0, v0);
                }
            }
            let u0 = read_single(self);
            let u1 = read_single(self);
            (u0, u1)
        } else if u_off0 == 1 {
            (read_single(self), 1)
        } else {
            (1, read_single(self))
        }
    }
}

pub struct MagSgnReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u64,
    bits: u8,
    prev_ff: bool,
}
impl<'a> MagSgnReader<'a> {
    pub fn new(magsgn: &'a [u8]) -> Self {
        Self { data: magsgn, pos: 0, acc: 0, bits: 0, prev_ff: false }
    }
    pub fn read_bits(&mut self, n: u8) -> u32 {
        while self.bits < n {
            let b = self.data.get(self.pos).copied().unwrap_or(0xFF);
            self.pos += usize::from(self.pos < self.data.len());
            let (avail, mask) = if self.prev_ff {
                (7, 0x7Fu8)
            } else {
                (8, 0xFFu8)
            };
            self.acc |= ((b & mask) as u64) << self.bits;
            self.bits += avail;
            self.prev_ff = b == 0xFF;
        }
        let v = (self.acc & ((1u64 << n) - 1)) as u32;
        self.acc >>= n;
        self.bits -= n;
        v
    }
}

pub struct HtRawBitReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    bits: u8,
    prev_ff: bool,
}
impl<'a> HtRawBitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, acc: 0, bits: 0, prev_ff: false }
    }
    pub fn read_bit(&mut self) -> u32 {
        if self.bits == 0 {
            let b = self.data.get(self.pos).copied().unwrap_or(0x00);
            self.pos += usize::from(self.pos < self.data.len());
            let (avail, mask) = if self.prev_ff {
                (7, 0x7Fu8)
            } else {
                (8, 0xFFu8)
            };
            self.acc = (b & mask) as u32;
            self.bits = avail;
            self.prev_ff = b == 0xFF;
        }
        let bit = self.acc & 1;
        self.acc >>= 1;
        self.bits -= 1;
        bit
    }
}

/// Decode one HTJ2K code-block using explicit caller-verified ITU-T T.814 Annex C
/// `HtVlcTables` (fail-closed when tables are not provided so no fake Huffman tables
/// ever misdecode external HTJ2K streams).
pub fn decode_ht_cblk_with_tables(
    cblk: &CblkHeaderState,
    total_bps: u8,
    tables: Option<&HtVlcTables<'_>>,
) -> Result<Vec<i32>, J2kError> {
    let tables = tables.ok_or(J2kError::Unsupported(
        "HTJ2K (.201-.203) block decoding requires verified ITU-T T.814 Annex C CxtVLC tables",
    ))?;
    let w = (cblk.x1 - cblk.x0) as usize;
    let h = (cblk.y1 - cblk.y0) as usize;
    let mut out = vec![0i32; w * h];
    if cblk.segments.is_empty() || cblk.num_passes == 0 || total_bps <= cblk.zero_bitplanes {
        return Ok(out);
    }
    let bp = (total_bps - 1 - cblk.zero_bitplanes) as i32;
    let cleanup_bytes = &cblk.segments[0].1;
    if cleanup_bytes.len() < 2 {
        return Err(J2kError::CorruptStream("truncated HT cleanup segment"));
    }
    let lcup = cleanup_bytes.len();
    let scup = ((cleanup_bytes[lcup - 1] as usize) << 4) | ((cleanup_bytes[lcup - 2] & 0x0F) as usize);
    if scup < 2 || scup > lcup {
        return Err(J2kError::CorruptStream("invalid HT Scup offset"));
    }

    let mut magsgn = MagSgnReader::new(&cleanup_bytes[..lcup - scup]);
    let mut mel = MelDecoder::new(&cleanup_bytes[lcup - scup..]);
    let mut vlc = RevVlcReader::new(cleanup_bytes, scup)?;

    let qw = w.div_ceil(2);
    let qh = h.div_ceil(2);
    let mut q_sigma = vec![0u8; qw * qh];
    let mut q_e_max = vec![0u8; qw * qh];
    let mut sig_grid = vec![false; w * h];

    let mut bp_shift = bp;
    for qy in 0..qh {
        let tbl = if qy == 0 { tables.tbl0 } else { tables.tbl1 };
        let mut qx = 0usize;
        while qx < qw {
            let pair = (qx + 1 < qw).then_some(qx + 1);
            let ctx0 = if qy == 0 {
                if qx == 0 { 0 } else { usize::from(q_sigma[qx - 1] != 0) }
            } else {
                let up = usize::from(q_sigma[(qy - 1) * qw + qx] != 0);
                let left = usize::from(qx > 0 && q_sigma[qy * qw + qx - 1] != 0);
                (up | (left << 1)) & 7
            };
            let ctx1 = pair.map(|q2| {
                if qy == 0 { usize::from(q_sigma[q2 - 1] != 0) } else {
                    let up = usize::from(q_sigma[(qy - 1) * qw + q2] != 0);
                    let ul = usize::from(q_sigma[(qy - 1) * qw + q2 - 1] != 0);
                    (up | (ul << 1)) & 7
                }
            }).unwrap_or(0);

            if ctx0 == 0 && (pair.is_none() || ctx1 == 0) && mel.decode_symbol() == 0 {
                qx += if pair.is_some() { 2 } else { 1 };
                continue;
            }
            let e0 = tbl[(ctx0 << 7) | (vlc.peek_bits(7) as usize)];
            if e0.cwd_len == 0 || e0.cwd_len > 7 {
                return Err(J2kError::CorruptStream("invalid HT CxtVLC entry"));
            }
            vlc.consume(e0.cwd_len);
            q_sigma[qy * qw + qx] = e0.rho;

            let e1 = if let Some(q2) = pair {
                let entry = tbl[(ctx1 << 7) | (vlc.peek_bits(7) as usize)];
                if entry.cwd_len == 0 || entry.cwd_len > 7 {
                    return Err(J2kError::CorruptStream("invalid HT CxtVLC entry"));
                }
                vlc.consume(entry.cwd_len);
                q_sigma[qy * qw + q2] = entry.rho;
                entry
            } else {
                HtVlcEntry::default()
            };
            let (u0, u1) = vlc.decode_uvlc_pair(qy == 0, e0.u_off, e1.u_off);
            let quads = [(qx, e0, u0), (qx + 1, e1, u1)];
            for (k_idx, &(cur_qx, ent, u_q)) in quads.iter().enumerate() {
                if k_idx == 1 && pair.is_none() {
                    break;
                }
                if ent.rho == 0 {
                    continue;
                }
                let u_val = u_q.max(1) as u8;
                q_e_max[qy * qw + cur_qx] = u_val;
                for sub in 0..4usize {
                    if (ent.rho & (1 << sub)) == 0 {
                        continue;
                    }
                    let sx = cur_qx * 2 + (sub >> 1);
                    let sy = qy * 2 + (sub & 1);
                    let embk = ((ent.e_k >> sub) & 1) as u32;
                    let emb1 = ((ent.e_1 >> sub) & 1) as u32;
                    let n_bits = u_val.saturating_sub(embk as u8);
                    let raw = magsgn.read_bits(n_bits.max(1));
                    let sign_bit = raw & 1;
                    let mag_bits = (raw >> 1) | (emb1 << n_bits.saturating_sub(1));
                    let val = ((mag_bits + 1) as i32) << bp_shift.max(0);
                    if sx < w && sy < h {
                        out[sy * w + sx] = if sign_bit != 0 { -val } else { val };
                        sig_grid[sy * w + sx] = true;
                    }
                }
            }
            qx += if pair.is_some() { 2 } else { 1 };
        }
    }

    if cblk.segments.len() > 1 && cblk.num_passes > 1 {
        let mut rdr = HtRawBitReader::new(&cblk.segments[1].1);
        let refine_bp = (bp_shift - 1).max(0);
        // Pass 2: HT SigProp
        for y in 0..h {
            for x in 0..w {
                let idx = y * w + x;
                if sig_grid[idx] { continue; }
                let has_nb = (-1..=1isize).any(|dy| (-1..=1isize).any(|dx| {
                    if dx == 0 && dy == 0 { return false; }
                    let (nx, ny) = (x as isize + dx, y as isize + dy);
                    nx >= 0 && ny >= 0 && nx < w as isize && ny < h as isize && sig_grid[ny as usize * w + nx as usize]
                }));
                if has_nb && rdr.read_bit() == 1 {
                    let s = rdr.read_bit();
                    let val = 1i32 << refine_bp;
                    out[idx] = if s != 0 { -val } else { val };
                    sig_grid[idx] = true;
                }
            }
        }
        // Pass 3: HT MagRef
        if cblk.num_passes >= 3 {
            bp_shift = refine_bp;
            for y in 0..h {
                for x in 0..w {
                    let idx = y * w + x;
                    if out[idx] != 0 {
                        let bit = rdr.read_bit() as i32;
                        let delta = bit << bp_shift;
                        out[idx] += if out[idx] < 0 { -delta } else { delta };
                    }
                }
            }
        }
    }
    Ok(out)
}
