//! ISO/IEC 15444-1 Annex D EBCOT Tier-1 decoder (delegating MQ decoding to `mq_coder`).
use super::markers::J2kError;
pub use super::mq_coder::MqDecoder;
use super::tier2::CblkHeaderState;

struct RawBitReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    bits: u8,
    prev_ff: bool,
}
impl<'a> RawBitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, acc: 0, bits: 0, prev_ff: false }
    }
    fn read_bit(&mut self) -> u8 {
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
        ((self.acc >> self.bits) & 1) as u8
    }
}

// Per-sample flag bits for Tier-1 state tracking.
const SIG: u8 = 1 << 0;
const SIGN: u8 = 1 << 1;
const CODED: u8 = 1 << 2;
const REFINED: u8 = 1 << 3;

#[inline]
fn sample_state(flags: &[u8], w: usize, h: usize, x: isize, y: isize, causal: bool, stripe_y0: usize) -> u8 {
    if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
        return 0;
    }
    if causal && y >= (stripe_y0 + 4) as isize {
        return 0;
    }
    flags[y as usize * w + x as usize]
}

fn zc_context(flags: &[u8], w: usize, h: usize, x: usize, y: usize, sb_id: usize, causal: bool, sy0: usize) -> usize {
    let xi = x as isize;
    let yi = y as isize;
    let l = (sample_state(flags, w, h, xi - 1, yi, causal, sy0) & SIG) != 0;
    let r = (sample_state(flags, w, h, xi + 1, yi, causal, sy0) & SIG) != 0;
    let u = (sample_state(flags, w, h, xi, yi - 1, causal, sy0) & SIG) != 0;
    let d = (sample_state(flags, w, h, xi, yi + 1, causal, sy0) & SIG) != 0;
    let ul = (sample_state(flags, w, h, xi - 1, yi - 1, causal, sy0) & SIG) != 0;
    let ur = (sample_state(flags, w, h, xi + 1, yi - 1, causal, sy0) & SIG) != 0;
    let dl = (sample_state(flags, w, h, xi - 1, yi + 1, causal, sy0) & SIG) != 0;
    let dr = (sample_state(flags, w, h, xi + 1, yi + 1, causal, sy0) & SIG) != 0;
    let kh = l as usize + r as usize;
    let kv = u as usize + d as usize;
    let kd = ul as usize + ur as usize + dl as usize + dr as usize;
    match sb_id {
        0 | 2 => {
            if kh == 2 { 8 }
            else if kh == 1 && kv >= 1 { 7 }
            else if kh == 1 && kd >= 1 { 6 }
            else if kh == 1 { 5 }
            else if kv == 2 { 4 }
            else if kv == 1 { 3 }
            else if kd >= 2 { 2 }
            else if kd == 1 { 1 }
            else { 0 }
        }
        1 => {
            if kv == 2 { 8 }
            else if kv == 1 && kh >= 1 { 7 }
            else if kv == 1 && kd >= 1 { 6 }
            else if kv == 1 { 5 }
            else if kh == 2 { 4 }
            else if kh == 1 { 3 }
            else if kd >= 2 { 2 }
            else if kd == 1 { 1 }
            else { 0 }
        }
        _ => {
            let khv = kh + kv;
            if kd >= 3 { 8 }
            else if kd == 2 && khv >= 1 { 7 }
            else if kd == 2 { 6 }
            else if kd == 1 && khv >= 2 { 5 }
            else if kd == 1 && khv == 1 { 4 }
            else if kd == 1 { 3 }
            else if khv >= 2 { 2 }
            else if khv == 1 { 1 }
            else { 0 }
        }
    }
}

fn has_sig_neighbor(flags: &[u8], w: usize, h: usize, x: usize, y: usize, causal: bool, sy0: usize) -> bool {
    for dy in -1..=1isize {
        for dx in -1..=1isize {
            if dx == 0 && dy == 0 { continue; }
            if (sample_state(flags, w, h, x as isize + dx, y as isize + dy, causal, sy0) & SIG) != 0 {
                return true;
            }
        }
    }
    false
}

fn sc_context_and_xor(flags: &[u8], w: usize, h: usize, x: usize, y: usize, causal: bool, sy0: usize) -> (usize, u8) {
    let pair = |a: u8, b: u8| -> i8 {
        let sa = if (a & SIG) == 0 { 0 } else if (a & SIGN) != 0 { -1 } else { 1 };
        let sb = if (b & SIG) == 0 { 0 } else if (b & SIGN) != 0 { -1 } else { 1 };
        (sa + sb).clamp(-1, 1)
    };
    let mut hc = pair(
        sample_state(flags, w, h, x as isize - 1, y as isize, causal, sy0),
        sample_state(flags, w, h, x as isize + 1, y as isize, causal, sy0),
    );
    let mut vc = pair(
        sample_state(flags, w, h, x as isize, y as isize - 1, causal, sy0),
        sample_state(flags, w, h, x as isize, y as isize + 1, causal, sy0),
    );
    let xor_bit = if hc < 0 || (hc == 0 && vc < 0) {
        hc = -hc;
        vc = -vc;
        1u8
    } else {
        0u8
    };
    let ctx = match (hc, vc) {
        (1, 1) => 13,
        (1, 0) => 12,
        (1, -1) => 11,
        (0, 1) => 10,
        _ => 9,
    };
    (ctx, xor_bit)
}

/// Decode one MQ-EBCOT code-block returning signed quantized coefficient magnitudes
/// aligned to integer subband units (`bit 0` = LSB after all `total_bps` bitplanes).
pub fn decode_ebcot_cblk(
    cblk: &CblkHeaderState,
    subband_id: usize,
    total_bps: u8,
    cblk_style: u8,
) -> Result<Vec<i32>, J2kError> {
    let w = (cblk.x1 - cblk.x0) as usize;
    let h = (cblk.y1 - cblk.y0) as usize;
    let mut mag = vec![0u32; w * h];
    let mut flags = vec![0u8; w * h];
    if cblk.segments.is_empty() || cblk.num_passes == 0 || total_bps <= cblk.zero_bitplanes {
        return Ok(vec![0i32; w * h]);
    }

    let causal = (cblk_style & 0x08) != 0;
    let reset = (cblk_style & 0x02) != 0;
    let segmark = (cblk_style & 0x20) != 0;
    let bypass = (cblk_style & 0x01) != 0;

    let mut seg_idx = 0usize;
    let mut seg_passes_left = 0u32;
    let mut mq = MqDecoder::new(&[]);
    let mut raw = RawBitReader::new(&[]);
    let mut use_raw = false;

    let mut bp = (total_bps - 1 - cblk.zero_bitplanes) as i32;
    let mut pass_type = 2u8; // First pass of first coded bitplane is ALWAYS Cleanup (2).

    for pass_idx in 0..cblk.num_passes {
        if bp < 0 {
            break;
        }
        if seg_passes_left == 0 {
            let (sp, ref bytes) = cblk.segments[seg_idx];
            seg_idx += 1;
            seg_passes_left = sp;
            use_raw = bypass && pass_idx >= 10 && ((pass_idx - 10) % 3) < 2;
            if use_raw {
                raw = RawBitReader::new(bytes);
            } else {
                mq.reinit_stream(bytes);
            }
        }
        let bit_val = 1u32 << bp;
        match pass_type {
            0 => {
                // Significance Propagation Pass (SPP)
                for sy0 in (0..h).step_by(4) {
                    let y_end = (sy0 + 4).min(h);
                    for x in 0..w {
                        for y in sy0..y_end {
                            let idx = y * w + x;
                            if (flags[idx] & SIG) != 0 || !has_sig_neighbor(&flags, w, h, x, y, causal, sy0) {
                                continue;
                            }
                            let sig = if use_raw {
                                raw.read_bit()
                            } else {
                                let cx = zc_context(&flags, w, h, x, y, subband_id, causal, sy0);
                                mq.decode_bit(cx)
                            };
                            flags[idx] |= CODED;
                            if sig == 1 {
                                mag[idx] |= bit_val;
                                let s = if use_raw {
                                    raw.read_bit()
                                } else {
                                    let (sc_cx, xorb) = sc_context_and_xor(&flags, w, h, x, y, causal, sy0);
                                    mq.decode_bit(sc_cx) ^ xorb
                                };
                                flags[idx] |= SIG | if s != 0 { SIGN } else { 0 };
                            }
                        }
                    }
                }
            }
            1 => {
                // Magnitude Refinement Pass (MRP)
                for sy0 in (0..h).step_by(4) {
                    let y_end = (sy0 + 4).min(h);
                    for x in 0..w {
                        for y in sy0..y_end {
                            let idx = y * w + x;
                            if (flags[idx] & SIG) == 0 || (flags[idx] & CODED) != 0 {
                                continue;
                            }
                            let b = if use_raw {
                                raw.read_bit()
                            } else {
                                let cx = if (flags[idx] & REFINED) != 0 {
                                    16
                                } else if has_sig_neighbor(&flags, w, h, x, y, causal, sy0) {
                                    15
                                } else {
                                    14
                                };
                                mq.decode_bit(cx)
                            };
                            if b == 1 {
                                mag[idx] |= bit_val;
                            }
                            flags[idx] |= REFINED;
                        }
                    }
                }
            }
            _ => {
                // Cleanup Pass (CU)
                for sy0 in (0..h).step_by(4) {
                    let y_end = (sy0 + 4).min(h);
                    for x in 0..w {
                        let full_stripe = y_end - sy0 == 4;
                        let mut first_y = sy0;
                        if full_stripe {
                            let mut all_insig = true;
                            for y in sy0..y_end {
                                let idx = y * w + x;
                                if (flags[idx] & (SIG | CODED)) != 0 || has_sig_neighbor(&flags, w, h, x, y, causal, sy0) {
                                    all_insig = false;
                                    break;
                                }
                            }
                            if all_insig {
                                if mq.decode_bit(17) == 0 {
                                    continue;
                                }
                                let offset = ((mq.decode_bit(18) << 1) | mq.decode_bit(18)) as usize;
                                first_y = sy0 + offset;
                                let idx = first_y * w + x;
                                mag[idx] |= bit_val;
                                let (sc_cx, xorb) = sc_context_and_xor(&flags, w, h, x, first_y, causal, sy0);
                                let s = mq.decode_bit(sc_cx) ^ xorb;
                                flags[idx] |= SIG | if s != 0 { SIGN } else { 0 };
                                first_y += 1;
                            }
                        }
                        for y in first_y..y_end {
                            let idx = y * w + x;
                            if (flags[idx] & (SIG | CODED)) != 0 {
                                continue;
                            }
                            let cx = zc_context(&flags, w, h, x, y, subband_id, causal, sy0);
                            if mq.decode_bit(cx) == 1 {
                                mag[idx] |= bit_val;
                                let (sc_cx, xorb) = sc_context_and_xor(&flags, w, h, x, y, causal, sy0);
                                let s = mq.decode_bit(sc_cx) ^ xorb;
                                flags[idx] |= SIG | if s != 0 { SIGN } else { 0 };
                            }
                        }
                    }
                }
                for f in &mut flags {
                    *f &= !CODED;
                }
                if segmark {
                    let mut mark = 0u8;
                    for _ in 0..4 {
                        mark = (mark << 1) | mq.decode_bit(18);
                    }
                    if mark != 0x0A {
                        return Err(J2kError::CorruptStream("EBCOT SEGMARK verification failed"));
                    }
                }
                bp -= 1;
            }
        }
        if reset {
            mq.reset_contexts();
        }
        seg_passes_left -= 1;
        pass_type = (pass_type + 1) % 3;
    }

    let mut out = vec![0i32; w * h];
    for i in 0..w * h {
        let m = mag[i] as i32;
        out[i] = if (flags[i] & SIGN) != 0 { -m } else { m };
    }
    Ok(out)
}
