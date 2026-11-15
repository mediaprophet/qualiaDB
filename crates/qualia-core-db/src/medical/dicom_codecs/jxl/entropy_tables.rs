//! ISO/IEC 18181-1 Annex C JPEG-XL `HybridUintConfig`, canonical LSB-first `PrefixTable`,
//! and 12-bit 4096-state rANS `AnsTable` builders and readers.
use super::container::JxlError;
use super::entropy::JxlBitReader;

pub fn ceil_log2(x: usize) -> usize {
    if x <= 1 {
        0
    } else {
        (usize::BITS - (x - 1).leading_zeros()) as usize
    }
}

pub fn unpack_signed(u: u32) -> i64 {
    if (u & 1) == 0 {
        (u >> 1) as i64
    } else {
        -((u >> 1) as i64) - 1
    }
}

#[derive(Clone, Copy, Debug)]
pub struct HybridUintConfig {
    pub split_exponent: usize,
    pub msb_in_token: usize,
    pub lsb_in_token: usize,
}

impl HybridUintConfig {
    pub fn read(br: &mut JxlBitReader<'_>, log_alpha_size: usize) -> Result<Self, JxlError> {
        let split_bits = ceil_log2(log_alpha_size + 1);
        let split_exponent = br.read_bits(split_bits)? as usize;
        if split_exponent > log_alpha_size {
            return Err(JxlError::CorruptStream("JXL HybridUint split_exponent out of range"));
        }
        if split_exponent == log_alpha_size {
            return Ok(Self { split_exponent, msb_in_token: 0, lsb_in_token: 0 });
        }
        let msb_bits = ceil_log2(split_exponent + 1);
        let msb_in_token = br.read_bits(msb_bits)? as usize;
        if msb_in_token > split_exponent {
            return Err(JxlError::CorruptStream("JXL HybridUint msb_in_token overflow"));
        }
        let lsb_bits = ceil_log2(split_exponent - msb_in_token + 1);
        let lsb_in_token = br.read_bits(lsb_bits)? as usize;
        if msb_in_token + lsb_in_token > split_exponent {
            return Err(JxlError::CorruptStream("JXL HybridUint lsb_in_token overflow"));
        }
        Ok(Self { split_exponent, msb_in_token, lsb_in_token })
    }

    pub fn decode_uint(&self, br: &mut JxlBitReader<'_>, token: u32) -> Result<u32, JxlError> {
        let split = 1u32 << self.split_exponent;
        if token < split {
            return Ok(token);
        }
        let n = self.split_exponent - self.msb_in_token - self.lsb_in_token;
        let low = token & ((1u32 << self.lsb_in_token) - 1);
        let tok_shifted = token.wrapping_sub(split);
        let msb = (tok_shifted >> self.lsb_in_token) & ((1u32 << self.msb_in_token) - 1);
        let shift = (tok_shifted >> (self.msb_in_token + self.lsb_in_token)) as usize + n;
        if shift >= 31 {
            return Err(JxlError::Overflow);
        }
        let mid = br.read_bits(shift)? as u32;
        Ok((((1u32 << self.msb_in_token) | msb) << (shift + self.lsb_in_token))
            | (mid << self.lsb_in_token)
            | low)
    }
}

#[derive(Clone, Debug)]
pub struct PrefixTable {
    pub entries: Vec<(u16, u8)>,
    pub max_len: usize,
}

impl PrefixTable {
    pub fn from_lengths(lengths: &[u8]) -> Result<Self, JxlError> {
        let mut counts = [0usize; 16];
        let mut max_len = 0usize;
        let mut single_sym = None;
        let mut nz = 0usize;
        for (sym, &l) in lengths.iter().enumerate() {
            if l > 0 {
                if l as usize > 15 {
                    return Err(JxlError::CorruptStream("JXL Huffman length > 15"));
                }
                counts[l as usize] += 1;
                max_len = max_len.max(l as usize);
                single_sym = Some(sym as u16);
                nz += 1;
            }
        }
        if nz <= 1 {
            return Ok(Self { entries: vec![(single_sym.unwrap_or(0), 0)], max_len: 0 });
        }
        let mut next_code = [0usize; 16];
        let mut code = 0usize;
        for bits in 1..=max_len {
            code = (code + counts[bits - 1]) << 1;
            if code + counts[bits] > (1usize << bits) {
                return Err(JxlError::CorruptStream("oversubscribed JXL prefix code"));
            }
            next_code[bits] = code;
        }
        let tab_size = 1usize << max_len;
        let mut entries = vec![(0u16, 0u8); tab_size];
        for (sym, &len) in lengths.iter().enumerate() {
            if len == 0 {
                continue;
            }
            let l = len as usize;
            let canonical = next_code[l];
            next_code[l] += 1;
            let mut rev = 0usize;
            for b in 0..l {
                if ((canonical >> b) & 1) != 0 {
                    rev |= 1usize << (l - 1 - b);
                }
            }
            let step = 1usize << l;
            let mut idx = rev;
            while idx < tab_size {
                entries[idx] = (sym as u16, len);
                idx += step;
            }
        }
        Ok(Self { entries, max_len })
    }

    pub fn decode(&self, br: &mut JxlBitReader<'_>) -> Result<u16, JxlError> {
        if self.max_len == 0 {
            return Ok(self.entries[0].0);
        }
        let (idx, rem) = br.peek_lsb_bits(self.max_len)?;
        let (sym, len) = self.entries[idx];
        if len == 0 || len as usize > rem {
            return Err(JxlError::CorruptStream("invalid JXL Huffman prefix code"));
        }
        br.advance_bits(len as usize);
        Ok(sym)
    }

    #[allow(dead_code)]
    pub fn encode_symbol_lsb(&self, sym: u16) -> Option<(u16, u8)> {
        for (idx, &(s, len)) in self.entries.iter().enumerate() {
            if s == sym && (self.max_len == 0 || len > 0) {
                let mask = (1usize << len) - 1;
                return Some(((idx & mask) as u16, len));
            }
        }
        None
    }
}

pub fn read_prefix_table(br: &mut JxlBitReader<'_>, alphabet_size: usize) -> Result<PrefixTable, JxlError> {
    if alphabet_size <= 1 {
        return PrefixTable::from_lengths(&[0u8]);
    }
    let alpha_bits = ceil_log2(alphabet_size).max(1);
    let simple_or_skip = br.read_bits(2)? as usize;
    let mut lengths = vec![0u8; alphabet_size];
    if simple_or_skip == 1 {
        let nsym = br.read_bits(2)? as usize + 1;
        let mut syms = [0usize; 4];
        for s in syms.iter_mut().take(nsym) {
            let v = br.read_bits(alpha_bits)? as usize;
            if v >= alphabet_size {
                return Err(JxlError::CorruptStream("simple prefix symbol out of range"));
            }
            *s = v;
        }
        match nsym {
            1 => return Ok(PrefixTable { entries: vec![(syms[0] as u16, 0)], max_len: 0 }),
            2 => {
                lengths[syms[0]] = 1;
                lengths[syms[1]] = 1;
            }
            3 => {
                lengths[syms[0]] = 1;
                lengths[syms[1]] = 2;
                lengths[syms[2]] = 2;
            }
            _ => {
                if br.read_bool()? {
                    lengths[syms[0]] = 1;
                    lengths[syms[1]] = 2;
                    lengths[syms[2]] = 3;
                    lengths[syms[3]] = 3;
                } else {
                    for &s in &syms {
                        lengths[s] = 2;
                    }
                }
            }
        }
        return PrefixTable::from_lengths(&lengths);
    }

    const CL_ORDER: [usize; 18] = [1, 2, 3, 4, 0, 5, 17, 6, 16, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let mut cl_lengths = [0u8; 18];
    let mut sum = 0usize;
    let mut nonzero = 0usize;
    for &idx in &CL_ORDER[simple_or_skip..] {
        let len = match br.read_bits(2)? {
            0 => 0u8,
            1 => 4u8,
            2 => 3u8,
            _ => match br.read_bits(1)? {
                0 => 2u8,
                _ => if br.read_bool()? { 5u8 } else { 1u8 },
            },
        };
        cl_lengths[idx] = len;
        if len > 0 {
            nonzero += 1;
            sum += 32usize >> len;
            if sum >= 32 {
                break;
            }
        }
    }
    if nonzero > 1 && sum != 32 {
        return Err(JxlError::CorruptStream("invalid JXL code-length prefix sum"));
    }
    let cl_tab = PrefixTable::from_lengths(&cl_lengths)?;
    let mut i = 0usize;
    let mut prev_len = 8u8;
    while i < alphabet_size {
        let sym = cl_tab.decode(br)? as u8;
        match sym {
            0..=15 => {
                lengths[i] = sym;
                if sym > 0 {
                    prev_len = sym;
                }
                i += 1;
            }
            16 => {
                let rep = 3 + br.read_bits(2)? as usize;
                for _ in 0..rep {
                    if i < alphabet_size {
                        lengths[i] = prev_len;
                        i += 1;
                    }
                }
            }
            _ => {
                let rep = 3 + br.read_bits(3)? as usize;
                i = (i + rep).min(alphabet_size);
            }
        }
    }
    PrefixTable::from_lengths(&lengths)
}

#[derive(Clone, Debug)]
pub struct AnsTable {
    pub freq: Vec<u16>,
    pub alias_sym: Vec<u16>,
    pub alias_offset: Vec<u16>,
}

impl AnsTable {
    pub fn build(counts: &[u16]) -> Result<Self, JxlError> {
        let sum: u32 = counts.iter().map(|&c| c as u32).sum();
        if sum != 4096 {
            return Err(JxlError::CorruptStream("JXL ANS distribution must sum to 4096"));
        }
        let mut cum = Vec::with_capacity(counts.len());
        let mut acc = 0u16;
        for &c in counts {
            cum.push(acc);
            acc = acc.wrapping_add(c);
        }
        let mut alias_sym = vec![0u16; 4096];
        let mut alias_offset = vec![0u16; 4096];
        for (sym, &c) in counts.iter().enumerate() {
            let start = cum[sym] as usize;
            for j in 0..c as usize {
                alias_sym[start + j] = sym as u16;
                alias_offset[start + j] = j as u16;
            }
        }
        Ok(Self { freq: counts.to_vec(), alias_sym, alias_offset })
    }
}

/// Canonical ISO/IEC 18181-1 Annex C.2.5 prefix code lengths for ANS `logcount` symbols `0..=13`
/// (`0..=12` = log2 frequency buckets, `13` = zero-run RLE repeat marker).
pub const ANS_LOGCOUNT_PREFIX_LENGTHS: [u8; 14] = [3, 4, 4, 4, 4, 4, 4, 3, 3, 4, 4, 4, 5, 5];

/// Compute `(logcount, mantissa, rbits)` for a symbol frequency `c` in `0..4096`.
pub fn logcount_bucket_for_count(c: u16) -> Result<(u8, u16, usize), JxlError> {
    if c >= 4096 {
        return Err(JxlError::CorruptStream("ANS multi-symbol count must be < 4096"));
    }
    if c == 0 {
        return Ok((0, 0, 0));
    }
    if c == 1 {
        return Ok((1, 0, 0));
    }
    let rbits = (15 - c.leading_zeros()) as usize;
    let logcount = (rbits + 1) as u8;
    let mantissa = c - (1u16 << rbits);
    Ok((logcount, mantissa, rbits))
}

pub fn read_ans_table(br: &mut JxlBitReader<'_>, max_sym: usize) -> Result<AnsTable, JxlError> {
    if br.read_bool()? {
        let nsym = br.read_bool()? as usize + 1;
        let s0 = br.read_varlen_uint8()? as usize;
        if s0 >= max_sym {
            return Err(JxlError::CorruptStream("ANS symbol out of bounds"));
        }
        let mut counts = vec![0u16; max_sym.max(s0 + 1)];
        if nsym == 1 {
            counts[s0] = 4096;
            return AnsTable::build(&counts);
        }
        let s1 = br.read_varlen_uint8()? as usize;
        if s1 >= max_sym || s1 == s0 {
            return Err(JxlError::CorruptStream("ANS symbol out of bounds"));
        }
        let c0 = br.read_bits(12)? as u16;
        if c0 == 0 || c0 >= 4096 {
            return Err(JxlError::CorruptStream("ANS weight out of 1..4095 bounds"));
        }
        counts[s0] = c0;
        counts[s1] = 4096 - c0;
        return AnsTable::build(&counts);
    }
    if br.read_bool()? {
        let alpha = br.read_varlen_uint8()? as usize + 1;
        if alpha > max_sym {
            return Err(JxlError::CorruptStream("ANS flat alphabet out of bounds"));
        }
        let mut counts = vec![0u16; max_sym];
        let base = (4096 / alpha) as u16;
        let rem = 4096 % alpha;
        for (i, c) in counts.iter_mut().take(alpha).enumerate() {
            *c = base + if i < rem { 1 } else { 0 };
        }
        return AnsTable::build(&counts);
    }

    // General multi-bucket ISO/IEC 18181-1 Annex C.2.5 rANS distribution (`length >= 3`).
    let shift = if !br.read_bool()? {
        0usize
    } else if !br.read_bool()? {
        1 + br.read_bits(2)? as usize
    } else {
        5 + br.read_bits(3)? as usize
    };
    if shift > 12 {
        return Err(JxlError::CorruptStream("ANS shift > 12"));
    }
    let length = br.read_varlen_uint8()? as usize + 3;
    if length > max_sym {
        return Err(JxlError::CorruptStream("ANS multi-bucket alphabet exceeds max_sym"));
    }
    let lc_tab = PrefixTable::from_lengths(&ANS_LOGCOUNT_PREFIX_LENGTHS)?;
    let mut logcounts = Vec::with_capacity(length);
    while logcounts.len() < length {
        let sym = lc_tab.decode(br)? as u8;
        if sym == 13 {
            let run = br.read_varlen_uint8()? as usize + 4;
            if logcounts.len() + run > length {
                return Err(JxlError::CorruptStream("ANS zero-run exceeds histogram length"));
            }
            logcounts.resize(logcounts.len() + run, 0u8);
        } else {
            logcounts.push(sym);
        }
    }

    let mut omit_pos = 0usize;
    for (i, &lc) in logcounts.iter().enumerate() {
        if lc > logcounts[omit_pos] {
            omit_pos = i;
        }
    }
    if logcounts[omit_pos] == 0 {
        return Err(JxlError::CorruptStream("ANS multi-bucket distribution has all-zero logcounts"));
    }

    let mut counts = vec![0u16; max_sym];
    let mut sum_others = 0u32;
    for (i, &lc) in logcounts.iter().enumerate() {
        if i == omit_pos || lc == 0 {
            continue;
        }
        if lc == 1 {
            counts[i] = 1;
        } else {
            let rbits = (lc - 1) as usize;
            let prec = rbits.saturating_sub(shift);
            let extra = if prec > 0 { br.read_bits(prec)? as u16 } else { 0 };
            counts[i] = (1u16 << rbits) + (extra << (rbits - prec));
        }
        sum_others += counts[i] as u32;
        if sum_others >= 4096 {
            return Err(JxlError::CorruptStream("ANS multi-bucket non-omitted counts >= 4096"));
        }
    }
    let rem_count = (4096 - sum_others) as u16;
    let (rem_lc, _, _) = logcount_bucket_for_count(rem_count)?;
    if rem_lc != logcounts[omit_pos] {
        return Err(JxlError::CorruptStream("ANS omitted symbol count mismatches logcount bucket"));
    }
    counts[omit_pos] = rem_count;
    AnsTable::build(&counts)
}
