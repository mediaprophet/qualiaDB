//! ISO/IEC 15444-1 Annex B Tier-2 packet header reader, TagTree, precincts & progression orders.
use super::markers::{CodingStyle, J2kError, ProgressionOrder};

pub struct PacketBitReader<'a> {
    pub data: &'a [u8],
    pub pos: usize,
    pub acc: u32,
    pub bits: u8,
    pub prev_was_ff: bool,
}

impl<'a> PacketBitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, acc: 0, bits: 0, prev_was_ff: false }
    }
    #[allow(dead_code)]
    pub fn pos(&self) -> usize {
        self.pos
    }
    pub fn remaining(&self) -> &'a [u8] {
        &self.data[self.pos..]
    }
    pub fn advance(&mut self, bytes: usize) -> Result<(), J2kError> {
        let next = self.pos.checked_add(bytes).ok_or(J2kError::Overflow)?;
        if next > self.data.len() {
            return Err(J2kError::CorruptStream("truncated packet body"));
        }
        self.pos = next;
        Ok(())
    }
    pub fn read_bit(&mut self) -> Result<u32, J2kError> {
        if self.bits == 0 {
            let b = *self.data.get(self.pos).ok_or(J2kError::CorruptStream("truncated packet header"))?;
            self.pos += 1;
            if self.prev_was_ff {
                self.acc = (b & 0x7F) as u32;
                self.bits = 7;
            } else {
                self.acc = b as u32;
                self.bits = 8;
            }
            self.prev_was_ff = b == 0xFF;
        }
        self.bits -= 1;
        Ok((self.acc >> self.bits) & 1)
    }
    pub fn read_bits(&mut self, n: u8) -> Result<u32, J2kError> {
        let mut v = 0u32;
        for _ in 0..n {
            v = (v << 1) | self.read_bit()?;
        }
        Ok(v)
    }
    pub fn byte_align(&mut self) -> Result<(), J2kError> {
        self.bits = 0;
        self.acc = 0;
        if self.prev_was_ff {
            // Trailing 0xFF requires consuming the stuffed 7-bit byte on byte alignment (B.10.1).
            if self.pos < self.data.len() {
                self.pos += 1;
            }
            self.prev_was_ff = false;
        }
        Ok(())
    }
    pub fn consume_marker_pair(&mut self, expected_second: u8, exact_len: usize) -> Result<(), J2kError> {
        if self.data.get(self.pos..self.pos + 2) == Some(&[0xFF, expected_second]) {
            let end = self.pos.checked_add(exact_len).ok_or(J2kError::Overflow)?;
            if end > self.data.len() {
                return Err(J2kError::CorruptStream("truncated SOP/EPH marker"));
            }
            self.pos = end;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TagTreeLevel {
    pub w: usize,
    pub h: usize,
    pub vals: Vec<u16>,
    pub states: Vec<u8>, // 0 = unknown, 1 = finalized
}

#[derive(Clone, Debug)]
pub struct TagTree {
    pub levels: Vec<TagTreeLevel>,
}

impl TagTree {
    pub fn new(w: usize, h: usize) -> Self {
        let mut levels = Vec::new();
        let (mut cw, mut ch) = (w.max(1), h.max(1));
        loop {
            let n = cw * ch;
            levels.push(TagTreeLevel { w: cw, h: ch, vals: vec![0; n], states: vec![0; n] });
            if cw == 1 && ch == 1 {
                break;
            }
            cw = cw.div_ceil(2);
            ch = ch.div_ceil(2);
        }
        levels.reverse(); // root at index 0, leaves at last()
        Self { levels }
    }

    pub fn decode_node(&mut self, x: usize, y: usize, threshold: u16, reader: &mut PacketBitReader<'_>) -> Result<u16, J2kError> {
        let num_levels = self.levels.len();
        let mut coords = Vec::with_capacity(num_levels);
        let (mut cx, mut cy) = (x, y);
        for _ in 0..num_levels {
            coords.push((cx, cy));
            cx >>= 1;
            cy >>= 1;
        }
        coords.reverse();
        let mut low = 0u16;
        for (lvl_idx, &(lx, ly)) in coords.iter().enumerate() {
            let lvl = &mut self.levels[lvl_idx];
            let idx = ly * lvl.w + lx;
            if lvl.vals[idx] < low {
                lvl.vals[idx] = low;
            }
            while lvl.states[idx] == 0 && lvl.vals[idx] < threshold {
                if reader.read_bit()? == 0 {
                    lvl.vals[idx] = lvl.vals[idx].saturating_add(1);
                } else {
                    lvl.states[idx] = 1;
                }
            }
            if lvl.states[idx] == 0 {
                return Ok(lvl.vals[idx]);
            }
            low = lvl.vals[idx];
        }
        Ok(low)
    }
    pub fn is_finalized(&self, x: usize, y: usize) -> bool {
        let leaf = self.levels.last().unwrap();
        leaf.states[y * leaf.w + x] == 1
    }
}

#[derive(Clone, Debug)]
pub struct CblkHeaderState {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
    pub included: bool,
    pub zero_bitplanes: u8,
    pub lblock: u32,
    pub num_passes: u32,
    pub segments: Vec<(u32 /* passes */, Vec<u8>)>,
}

#[derive(Clone, Debug)]
pub struct SubbandGrid {
    pub subband_id: usize, // 0=LL, 1=HL, 2=LH, 3=HH
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
    pub cbx_num: usize,
    pub cby_num: usize,
    pub cblks: Vec<CblkHeaderState>,
    pub incl_tree: TagTree,
    pub zbp_tree: TagTree,
}

#[inline]
#[allow(dead_code)]
pub fn ceil_div_pow2(val: u32, shift: u8) -> u32 {
    if shift == 0 { return val; }
    let d = 1u32 << shift;
    val.div_ceil(d)
}

#[inline]
#[allow(dead_code)]
pub fn floor_div_pow2(val: u32, shift: u8) -> u32 {
    val >> shift
}

pub fn build_subband_grid(
    comp_x0: u32,
    comp_y0: u32,
    comp_x1: u32,
    comp_y1: u32,
    res: u8,
    decomp_levels: u8,
    subband_id: usize,
    cblk_w_shift: u8,
    cblk_h_shift: u8,
) -> SubbandGrid {
    let nb = if res == 0 { decomp_levels } else { decomp_levels - res + 1 };
    let xo = if matches!(subband_id, 1 | 3) { 1u32 } else { 0u32 };
    let yo = if matches!(subband_id, 2 | 3) { 1u32 } else { 0u32 };
    let step = 1u32 << nb;
    let half = 1u32 << (nb - if res > 0 { 1 } else { 0 });
    let x0 = if res == 0 { comp_x0.div_ceil(step) } else { (comp_x0.saturating_sub(xo * half)).div_ceil(step) };
    let x1 = if res == 0 { comp_x1.div_ceil(step) } else { (comp_x1.saturating_sub(xo * half)).div_ceil(step) };
    let y0 = if res == 0 { comp_y0.div_ceil(step) } else { (comp_y0.saturating_sub(yo * half)).div_ceil(step) };
    let y1 = if res == 0 { comp_y1.div_ceil(step) } else { (comp_y1.saturating_sub(yo * half)).div_ceil(step) };
    let cbw = 1u32 << ((cblk_w_shift + 2).min(10));
    let cbh = 1u32 << ((cblk_h_shift + 2).min(10));
    if x1 <= x0 || y1 <= y0 {
        return SubbandGrid {
            subband_id, x0, y0, x1, y1, cbx_num: 0, cby_num: 0,
            cblks: Vec::new(), incl_tree: TagTree::new(1, 1), zbp_tree: TagTree::new(1, 1),
        };
    }
    let cbx0 = x0 / cbw;
    let cbx1 = x1.div_ceil(cbw);
    let cby0 = y0 / cbh;
    let cby1 = y1.div_ceil(cbh);
    let cbx_num = (cbx1 - cbx0) as usize;
    let cby_num = (cby1 - cby0) as usize;
    let mut cblks = Vec::with_capacity(cbx_num * cby_num);
    for cby in cby0..cby1 {
        for cbx in cbx0..cbx1 {
            cblks.push(CblkHeaderState {
                x0: (cbx * cbw).max(x0),
                y0: (cby * cbh).max(y0),
                x1: ((cbx + 1) * cbw).min(x1),
                y1: ((cby + 1) * cbh).min(y1),
                included: false,
                zero_bitplanes: 0,
                lblock: 3,
                num_passes: 0,
                segments: Vec::new(),
            });
        }
    }
    SubbandGrid {
        subband_id, x0, y0, x1, y1, cbx_num, cby_num, cblks,
        incl_tree: TagTree::new(cbx_num, cby_num),
        zbp_tree: TagTree::new(cbx_num, cby_num),
    }
}

pub fn read_coding_passes(r: &mut PacketBitReader<'_>) -> Result<u32, J2kError> {
    if r.read_bit()? == 0 { return Ok(1); }
    if r.read_bit()? == 0 { return Ok(2); }
    let two = r.read_bits(2)?;
    if two != 3 { return Ok(3 + two); }
    let five = r.read_bits(5)?;
    if five != 31 { return Ok(6 + five); }
    let seven = r.read_bits(7)?;
    Ok(37 + seven)
}

pub fn decode_packet(
    r: &mut PacketBitReader<'_>,
    cod: &CodingStyle,
    layer: u16,
    subbands: &mut [&mut SubbandGrid],
) -> Result<(), J2kError> {
    if cod.sop_markers {
        r.consume_marker_pair(0x91, 6)?;
    }
    let non_empty = r.read_bit()? == 1;
    if !non_empty {
        r.byte_align()?;
        if cod.eph_markers {
            r.consume_marker_pair(0x92, 2)?;
        }
        return Ok(());
    }
    let mut body_lengths: Vec<(usize /* sb */, usize /* cb */, u32 /* passes */, usize /* len */)> = Vec::new();
    let restart = (cod.cblk_style & 0x04) != 0;
    let bypass = (cod.cblk_style & 0x01) != 0;

    for (sb_idx, sb) in subbands.iter_mut().enumerate() {
        for cby in 0..sb.cby_num {
            for cbx in 0..sb.cbx_num {
                let idx = cby * sb.cbx_num + cbx;
                let was_included = sb.cblks[idx].included;
                let inc = if !was_included {
                    sb.incl_tree.decode_node(cbx, cby, layer + 1, r)?;
                    sb.incl_tree.is_finalized(cbx, cby) && sb.incl_tree.levels.last().unwrap().vals[idx] <= layer
                } else {
                    r.read_bit()? == 1
                };
                if !inc {
                    continue;
                }
                if !was_included {
                    sb.cblks[idx].included = true;
                    let mut zbp = 0u16;
                    loop {
                        sb.zbp_tree.decode_node(cbx, cby, zbp + 1, r)?;
                        if sb.zbp_tree.is_finalized(cbx, cby) {
                            zbp = sb.zbp_tree.levels.last().unwrap().vals[idx];
                            break;
                        }
                        zbp += 1;
                        if zbp > 64 {
                            return Err(J2kError::CorruptStream("zero-bitplane tagtree runaway"));
                        }
                    }
                    sb.cblks[idx].zero_bitplanes = zbp as u8;
                }
                let passes = read_coding_passes(r)?;
                while r.read_bit()? == 1 {
                    sb.cblks[idx].lblock = sb.cblks[idx].lblock.saturating_add(1);
                }
                // Determine segment lengths (single segment unless TERMALL / BYPASS splits passes)
                let seg_passes = split_segment_passes(passes, sb.cblks[idx].num_passes, restart, bypass, cod.uses_htj2k_only());
                for sp in seg_passes {
                    let bits = sb.cblks[idx].lblock + (31 - sp.leading_zeros());
                    if bits > 28 {
                        return Err(J2kError::CorruptStream("packet segment length overflow"));
                    }
                    let len = r.read_bits(bits as u8)? as usize;
                    body_lengths.push((sb_idx, idx, sp, len));
                }
                sb.cblks[idx].num_passes += passes;
            }
        }
    }
    r.byte_align()?;
    if cod.eph_markers {
        r.consume_marker_pair(0x92, 2)?;
    }
    for (sb_idx, cb_idx, passes, len) in body_lengths {
        let slice = r.remaining().get(..len).ok_or(J2kError::CorruptStream("truncated code-block data"))?;
        let mut owned = Vec::with_capacity(len);
        owned.extend_from_slice(slice);
        subbands[sb_idx].cblks[cb_idx].segments.push((passes, owned));
        r.advance(len)?;
    }
    Ok(())
}

pub fn split_segment_passes(new_passes: u32, prev_passes: u32, restart: bool, bypass: bool, ht_only: bool) -> Vec<u32> {
    if restart {
        return vec![1; new_passes as usize];
    }
    if ht_only && prev_passes == 0 && new_passes > 1 {
        return vec![1, new_passes - 1];
    }
    if !bypass {
        return vec![new_passes];
    }
    // ISO 15444-1 §B.10.7.2: first 10 passes use MQ (1 segment), then alternate 2 raw / 1 MQ.
    let mut segs = Vec::new();
    let mut cur = prev_passes;
    let mut rem = new_passes;
    while rem > 0 {
        let take = if cur < 10 {
            (10 - cur).min(rem)
        } else if (cur - 10) % 3 < 2 {
            (2 - ((cur - 10) % 3)).min(rem)
        } else {
            1
        };
        segs.push(take);
        cur += take;
        rem -= take;
    }
    segs
}

/// Produce ordered `(layer, res, comp)` packet iteration tuples for a single-precinct tile.
pub fn packet_iteration_order(
    order: ProgressionOrder,
    layers: u16,
    max_res: u8,
    num_comps: usize,
) -> Vec<(u16, u8, usize)> {
    let mut out = Vec::new();
    match order {
        ProgressionOrder::Lrcp => {
            for l in 0..layers {
                for r in 0..=max_res {
                    for c in 0..num_comps {
                        out.push((l, r, c));
                    }
                }
            }
        }
        ProgressionOrder::Rlcp => {
            for r in 0..=max_res {
                for l in 0..layers {
                    for c in 0..num_comps {
                        out.push((l, r, c));
                    }
                }
            }
        }
        ProgressionOrder::Rpcl | ProgressionOrder::Pcrl => {
            for r in 0..=max_res {
                for c in 0..num_comps {
                    for l in 0..layers {
                        out.push((l, r, c));
                    }
                }
            }
        }
        ProgressionOrder::Cprl => {
            for c in 0..num_comps {
                for r in 0..=max_res {
                    for l in 0..layers {
                        out.push((l, r, c));
                    }
                }
            }
        }
    }
    out
}
