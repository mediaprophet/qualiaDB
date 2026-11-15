//! ISO/IEC 18181-1 Annex C JPEG-XL entropy stream: LSB-first `JxlBitReader`,
//! `ContextMap` (with inverse Move-To-Front), and LZ77 sliding ring decoder.
use super::container::JxlError;
use super::entropy_tables::{
    read_ans_table, read_prefix_table, AnsTable, HybridUintConfig, PrefixTable,
};

pub use super::entropy_tables::unpack_signed;

pub struct JxlBitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}

impl<'a> JxlBitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, bit_pos: 0 }
    }

    pub fn bit_pos(&self) -> usize {
        self.bit_pos
    }

    pub fn bits_remaining(&self) -> usize {
        self.data.len().saturating_mul(8).saturating_sub(self.bit_pos)
    }

    pub fn peek_lsb_bits(&self, max_len: usize) -> Result<(usize, usize), JxlError> {
        let rem = self.bits_remaining();
        let peek_bits = rem.min(max_len);
        if peek_bits == 0 {
            return Err(JxlError::CorruptStream("unexpected EOF in prefix symbol"));
        }
        let mut idx = 0usize;
        for i in 0..peek_bits {
            let p = self.bit_pos + i;
            let bit = ((self.data[p >> 3] >> (p & 7)) & 1) as usize;
            idx |= bit << i;
        }
        Ok((idx, rem))
    }

    pub fn advance_bits(&mut self, n: usize) {
        self.bit_pos += n;
    }

    pub fn read_bits(&mut self, n: usize) -> Result<u64, JxlError> {
        if n > 56 {
            return Err(JxlError::CorruptStream("JXL read_bits > 56"));
        }
        if self.bits_remaining() < n {
            return Err(JxlError::CorruptStream("unexpected EOF in JXL bitstream"));
        }
        let mut out = 0u64;
        for i in 0..n {
            let p = self.bit_pos + i;
            let bit = ((self.data[p >> 3] >> (p & 7)) & 1) as u64;
            out |= bit << i;
        }
        self.bit_pos += n;
        Ok(out)
    }

    pub fn read_bool(&mut self) -> Result<bool, JxlError> {
        Ok(self.read_bits(1)? == 1)
    }

    pub fn read_u32_dist(&mut self, dist: &[(u32, usize); 4]) -> Result<u32, JxlError> {
        let sel = self.read_bits(2)? as usize;
        let (base, extra) = dist[sel];
        let add = if extra > 0 { self.read_bits(extra)? as u32 } else { 0 };
        base.checked_add(add).ok_or(JxlError::Overflow)
    }

    pub fn read_enum(&mut self) -> Result<u32, JxlError> {
        self.read_u32_dist(&[(0, 0), (1, 0), (2, 4), (18, 6)])
    }

    pub fn read_u64(&mut self) -> Result<u64, JxlError> {
        match self.read_bits(2)? {
            0 => Ok(0),
            1 => Ok(1 + self.read_bits(4)?),
            2 => Ok(17 + self.read_bits(8)?),
            _ => {
                let mut val = self.read_bits(12)?;
                let mut shift = 12usize;
                while self.read_bool()? {
                    if shift < 60 {
                        val |= self.read_bits(8)? << shift;
                        shift += 8;
                    } else {
                        val |= self.read_bits(4)? << 60;
                        break;
                    }
                }
                Ok(val)
            }
        }
    }

    pub fn read_f16(&mut self) -> Result<f32, JxlError> {
        let bits = self.read_bits(16)? as u16;
        let v = half::f16::from_bits(bits).to_f32();
        if !v.is_finite() {
            return Err(JxlError::CorruptStream("non-finite f16 in JXL header"));
        }
        Ok(v)
    }

    pub fn skip_pad_to_byte(&mut self) -> Result<(), JxlError> {
        let rem = self.bit_pos & 7;
        if rem != 0 {
            let skip = 8 - rem;
            if self.bits_remaining() < skip {
                return Err(JxlError::CorruptStream("truncated JXL byte alignment"));
            }
            self.bit_pos += skip;
        }
        Ok(())
    }

    pub fn read_varlen_uint8(&mut self) -> Result<u16, JxlError> {
        if !self.read_bool()? {
            return Ok(0);
        }
        let nbits = self.read_bits(3)? as usize;
        if nbits == 0 {
            Ok(1)
        } else {
            Ok(((1usize << nbits) + self.read_bits(nbits)? as usize) as u16)
        }
    }

    pub fn read_varlen_uint16(&mut self) -> Result<u32, JxlError> {
        if !self.read_bool()? {
            return Ok(0);
        }
        let nbits = self.read_bits(4)? as usize;
        if nbits == 0 {
            Ok(1)
        } else {
            Ok((1u32 << nbits) + self.read_bits(nbits)? as u32)
        }
    }
}

const SPECIAL_DIST_OFFSETS: [(i32, i32); 16] = [
    (0, 1), (1, 0), (1, 1), (-1, 1), (0, 2), (2, 0), (1, 2), (-1, 2),
    (2, 1), (-2, 1), (2, 2), (-2, 2), (0, 3), (3, 0), (1, 3), (-1, 3),
];

pub struct JxlEntropyDecoder {
    lz77_enabled: bool,
    min_symbol: u32,
    min_length: u32,
    dist_multiplier: usize,
    lz77_window: Vec<u32>,
    lz77_pending: usize,
    lz77_copy_pos: usize,
    context_map: Vec<u8>,
    configs: Vec<HybridUintConfig>,
    prefix_tables: Option<Vec<PrefixTable>>,
    ans_tables: Option<Vec<AnsTable>>,
    ans_state: u32,
    ans_initialized: bool,
}

impl JxlEntropyDecoder {
    pub fn read(br: &mut JxlBitReader<'_>, num_contexts: usize, dist_multiplier: usize) -> Result<Self, JxlError> {
        let lz77_enabled = br.read_bool()?;
        let (min_symbol, min_length) = if lz77_enabled {
            (
                br.read_u32_dist(&[(224, 0), (512, 0), (4096, 0), (8, 15)])?,
                br.read_u32_dist(&[(3, 0), (4, 0), (5, 2), (9, 8)])?,
            )
        } else {
            (0, 0)
        };
        let total_ctx = num_contexts + usize::from(lz77_enabled);
        let context_map = read_context_map(br, total_ctx)?;
        let num_hist = context_map.iter().copied().max().unwrap_or(0) as usize + 1;

        let use_prefix = br.read_bool()?;
        let log_alpha_ans = if use_prefix { 15 } else { 5 + br.read_bits(2)? as usize };
        let mut configs = Vec::with_capacity(num_hist);
        for _ in 0..num_hist {
            configs.push(HybridUintConfig::read(br, log_alpha_ans)?);
        }

        let (prefix_tables, ans_tables) = if use_prefix {
            let mut sizes = Vec::with_capacity(num_hist);
            for _ in 0..num_hist {
                sizes.push(br.read_varlen_uint16()? as usize + 1);
            }
            let mut tabs = Vec::with_capacity(num_hist);
            for s in sizes {
                tabs.push(read_prefix_table(br, s)?);
            }
            (Some(tabs), None)
        } else {
            let max_sym = 1usize << log_alpha_ans;
            let mut tabs = Vec::with_capacity(num_hist);
            for _ in 0..num_hist {
                tabs.push(read_ans_table(br, max_sym)?);
            }
            (None, Some(tabs))
        };

        Ok(Self {
            lz77_enabled,
            min_symbol,
            min_length,
            dist_multiplier: dist_multiplier.max(1),
            lz77_window: Vec::with_capacity(if lz77_enabled { 1 << 16 } else { 0 }),
            lz77_pending: 0,
            lz77_copy_pos: 0,
            context_map,
            configs,
            prefix_tables,
            ans_tables,
            ans_state: 0,
            ans_initialized: false,
        })
    }

    fn read_token(&mut self, br: &mut JxlBitReader<'_>, hist: usize) -> Result<u32, JxlError> {
        if let Some(tabs) = &self.prefix_tables {
            return Ok(tabs[hist].decode(br)? as u32);
        }
        let tabs = self.ans_tables.as_ref().ok_or(JxlError::CorruptStream("missing ANS table"))?;
        if !self.ans_initialized {
            let hi = br.read_bits(16)? as u32;
            let lo = br.read_bits(16)? as u32;
            self.ans_state = (hi << 16) | lo;
            self.ans_initialized = true;
        }
        let tab = &tabs[hist];
        let slot = (self.ans_state & 0x0FFF) as usize;
        let sym = tab.alias_sym[slot];
        let freq = tab.freq[sym as usize] as u32;
        if freq == 0 {
            return Err(JxlError::CorruptStream("zero-frequency JXL ANS symbol"));
        }
        self.ans_state = freq * (self.ans_state >> 12) + tab.alias_offset[slot] as u32;
        if self.ans_state < (1 << 16) {
            self.ans_state = (self.ans_state << 16) | (br.read_bits(16)? as u32);
        }
        Ok(sym as u32)
    }

    pub fn read_uint(&mut self, br: &mut JxlBitReader<'_>, ctx: usize) -> Result<u32, JxlError> {
        if self.lz77_pending > 0 {
            let v = self.lz77_window[self.lz77_copy_pos];
            self.lz77_copy_pos += 1;
            self.lz77_pending -= 1;
            self.lz77_window.push(v);
            return Ok(v);
        }
        let h = *self.context_map.get(ctx).ok_or(JxlError::CorruptStream("context out of range"))? as usize;
        let tok = self.read_token(br, h)?;
        if self.lz77_enabled && tok >= self.min_symbol {
            let len = self.min_length + self.configs[h].decode_uint(br, tok - self.min_symbol)?;
            let dh = *self.context_map.last().unwrap_or(&0) as usize;
            let dtok = self.read_token(br, dh)?;
            let dcode = self.configs[dh].decode_uint(br, dtok)?;
            let dist = if (dcode as usize) < SPECIAL_DIST_OFFSETS.len() {
                let (dx, dy) = SPECIAL_DIST_OFFSETS[dcode as usize];
                (dy * self.dist_multiplier as i32 + dx).max(1) as usize
            } else {
                (dcode as usize).saturating_sub(119).max(1)
            };
            let wlen = self.lz77_window.len();
            if dist > wlen || len == 0 {
                return Err(JxlError::CorruptStream("invalid JXL LZ77 backreference"));
            }
            let start = wlen - dist;
            let v = self.lz77_window[start];
            self.lz77_window.push(v);
            self.lz77_copy_pos = start + 1;
            self.lz77_pending = len as usize - 1;
            return Ok(v);
        }
        let val = self.configs[h].decode_uint(br, tok)?;
        if self.lz77_enabled {
            self.lz77_window.push(val);
        }
        Ok(val)
    }

    pub fn finalize(&self) -> Result<(), JxlError> {
        Ok(())
    }
}

fn read_context_map(br: &mut JxlBitReader<'_>, num_contexts: usize) -> Result<Vec<u8>, JxlError> {
    if num_contexts <= 1 {
        return Ok(vec![0u8; num_contexts]);
    }
    if br.read_bool()? {
        let bits = br.read_bits(2)? as usize;
        let mut map = Vec::with_capacity(num_contexts);
        for _ in 0..num_contexts {
            map.push(if bits == 0 { 0 } else { br.read_bits(bits)? as u8 });
        }
        return Ok(map);
    }
    let use_mtf = br.read_bool()?;
    let mut inner = JxlEntropyDecoder::read(br, 1, 1)?;
    let mut map = Vec::with_capacity(num_contexts);
    for _ in 0..num_contexts {
        map.push(inner.read_uint(br, 0)? as u8);
    }
    if use_mtf {
        let mut mtf: [u8; 256] = core::array::from_fn(|i| i as u8);
        for v in &mut map {
            let idx = *v as usize;
            let sym = mtf[idx];
            mtf.copy_within(0..idx, 1);
            mtf[0] = sym;
            *v = sym;
        }
    }
    Ok(map)
}

pub fn decode_prefix_or_ans_blob(
    br: &mut JxlBitReader<'_>,
    len: usize,
    _num_contexts: usize,
    max_bytes: usize,
) -> Result<Vec<u8>, JxlError> {
    if len > max_bytes {
        return Err(JxlError::ByteLimit);
    }
    let mut dec = JxlEntropyDecoder::read(br, 1, 1)?;
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        out.push(dec.read_uint(br, 0)? as u8);
    }
    Ok(out)
}
