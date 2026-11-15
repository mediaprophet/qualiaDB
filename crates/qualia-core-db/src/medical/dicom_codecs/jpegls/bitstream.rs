//! ITU-T T.87 §9.1 JPEG-LS bit reader with 0xFF/7-bit byte unstuffing,
//! limited-length Golomb-Rice decoding and exact DRI/RSTm resynchronization.
use super::headers::JpegLsError;

pub(super) struct LsBitReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u32,
    bits: u8,
    prev_was_ff: bool,
    hit_marker: bool,
}

impl<'a> LsBitReader<'a> {
    pub fn new(entropy: &'a [u8]) -> Self {
        Self {
            data: entropy,
            pos: 0,
            acc: 0,
            bits: 0,
            prev_was_ff: false,
            hit_marker: false,
        }
    }

    #[inline]
    fn fill_byte(&mut self) -> Result<(), JpegLsError> {
        if self.pos >= self.data.len() || self.hit_marker {
            return Err(JpegLsError::CorruptStream("unexpected EOF in JPEG-LS scan"));
        }
        let b = self.data[self.pos];
        if self.prev_was_ff {
            if b >= 0x80 {
                self.hit_marker = true;
                return Err(JpegLsError::CorruptStream("unexpected marker inside JPEG-LS bits"));
            }
            self.pos += 1;
            self.acc = (self.acc << 7) | (b & 0x7F) as u32;
            self.bits += 7;
            self.prev_was_ff = false;
        } else {
            self.pos += 1;
            self.acc = (self.acc << 8) | b as u32;
            self.bits += 8;
            self.prev_was_ff = b == 0xFF;
        }
        Ok(())
    }

    #[inline]
    pub fn read_bit(&mut self) -> Result<u32, JpegLsError> {
        if self.bits == 0 {
            self.fill_byte()?;
        }
        self.bits -= 1;
        Ok((self.acc >> self.bits) & 1)
    }

    #[inline]
    pub fn read_bits(&mut self, n: u8) -> Result<u32, JpegLsError> {
        if n == 0 {
            return Ok(0);
        }
        if n > 24 {
            return Err(JpegLsError::CorruptStream("JPEG-LS bit count exceeds 24"));
        }
        while self.bits < n {
            self.fill_byte()?;
        }
        self.bits -= n;
        Ok((self.acc >> self.bits) & ((1u32 << n) - 1))
    }

    /// ITU-T T.87 §A.5.3 limited-length Golomb-Rice decoding: `LG(k, glimit)`.
    #[inline]
    pub fn read_limited_golomb(&mut self, k: u8, glimit: u8, qbpp: u8) -> Result<i32, JpegLsError> {
        let mut q = 0u32;
        let glim = glimit as u32;
        while q < glim && self.read_bit()? == 0 {
            q += 1;
        }
        if q < glim {
            let rem = self.read_bits(k)?;
            Ok(((q << k) | rem) as i32)
        } else {
            let val = self.read_bits(qbpp)?;
            Ok(val as i32 + 1)
        }
    }

    /// Byte-align at a DRI restart boundary and consume `0xFF RSTm`.
    pub fn consume_restart(&mut self, rst_idx: u8) -> Result<(), JpegLsError> {
        let expected_rst = 0xD0 + (rst_idx & 7);
        self.bits = 0;
        self.acc = 0;
        if self.hit_marker {
            let m = *self.data.get(self.pos).ok_or(JpegLsError::CorruptStream("truncated RST"))?;
            if m != expected_rst {
                return Err(JpegLsError::CorruptStream("mismatched JPEG-LS RST marker"));
            }
            self.pos += 1;
        } else {
            if self.prev_was_ff && self.data.get(self.pos) == Some(&0x00) {
                self.pos += 1;
            }
            let b0 = *self.data.get(self.pos).ok_or(JpegLsError::CorruptStream("missing RST marker"))?;
            let b1 = *self.data.get(self.pos + 1).ok_or(JpegLsError::CorruptStream("truncated RST marker"))?;
            if b0 != 0xFF || b1 != expected_rst {
                return Err(JpegLsError::CorruptStream("mismatched JPEG-LS RST marker"));
            }
            self.pos += 2;
        }
        self.prev_was_ff = false;
        self.hit_marker = false;
        Ok(())
    }
}
