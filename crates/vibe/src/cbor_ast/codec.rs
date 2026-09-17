//! Minimal CBOR encoder/decoder.

use super::DecodeError;

pub(crate) struct CborEncoder {
    buf: Vec<u8>,
}

impl CborEncoder {
    pub(crate) fn new() -> Self {
        Self { buf: Vec::new() }
    }
    pub(crate) fn finish(self) -> Vec<u8> {
        self.buf
    }

    #[allow(dead_code)]
    pub(crate) fn write_u8(&mut self, b: u8) {
        self.buf.push(b);
    }

    pub(crate) fn write_type_and_len(&mut self, major: u8, len: u64) {
        if len < 24 {
            self.buf.push((major << 5) | len as u8);
        } else if len < 256 {
            self.buf.push((major << 5) | 24);
            self.buf.push(len as u8);
        } else if len < 65536 {
            self.buf.push((major << 5) | 25);
            self.buf.extend_from_slice(&(len as u16).to_be_bytes());
        } else if len < 4294967296 {
            self.buf.push((major << 5) | 26);
            self.buf.extend_from_slice(&(len as u32).to_be_bytes());
        } else {
            self.buf.push((major << 5) | 27);
            self.buf.extend_from_slice(&len.to_be_bytes());
        }
    }

    pub(crate) fn uint(&mut self, n: u64) {
        self.write_type_and_len(0, n);
    }
    pub(crate) fn int(&mut self, n: i64) {
        if n >= 0 {
            self.uint(n as u64);
        } else {
            self.write_type_and_len(1, (-1 - n) as u64);
        }
    }
    pub(crate) fn str(&mut self, s: &str) {
        self.write_type_and_len(3, s.len() as u64);
        self.buf.extend_from_slice(s.as_bytes());
    }
    pub(crate) fn bool(&mut self, b: bool) {
        self.buf.push(if b { 0xF5 } else { 0xF4 });
    }
    pub(crate) fn null(&mut self) {
        self.buf.push(0xF6);
    }
    pub(crate) fn array(&mut self, len: u64) {
        self.write_type_and_len(4, len);
    }
    pub(crate) fn map(&mut self, len: u64) {
        self.write_type_and_len(5, len);
    }
    pub(crate) fn tag(&mut self, tag: u64) {
        self.write_type_and_len(6, tag);
    }
}

// â”€â”€ Minimal CBOR decoder (pure Rust, no deps) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

pub(crate) struct CborDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> CborDecoder<'a> {
    pub(crate) fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    #[allow(dead_code)]
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn read_u8(&mut self) -> Result<u8, DecodeError> {
        if self.pos >= self.buf.len() {
            return Err(DecodeError::Eof);
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    pub(crate) fn peek(&self) -> Result<u8, DecodeError> {
        if self.pos >= self.buf.len() {
            return Err(DecodeError::Eof);
        }
        Ok(self.buf[self.pos])
    }

    pub(crate) fn read_type_and_len(&mut self) -> Result<(u8, u64), DecodeError> {
        let b = self.read_u8()?;
        let major = b >> 5;
        let ai = b & 0x1F;
        let len = match ai {
            0..=23 => ai as u64,
            24 => self.read_u8()? as u64,
            25 => {
                if self.pos + 2 > self.buf.len() {
                    return Err(DecodeError::Eof);
                }
                let v = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
                self.pos += 2;
                v as u64
            }
            26 => {
                if self.pos + 4 > self.buf.len() {
                    return Err(DecodeError::Eof);
                }
                let v = u32::from_be_bytes([
                    self.buf[self.pos],
                    self.buf[self.pos + 1],
                    self.buf[self.pos + 2],
                    self.buf[self.pos + 3],
                ]);
                self.pos += 4;
                v as u64
            }
            27 => {
                if self.pos + 8 > self.buf.len() {
                    return Err(DecodeError::Eof);
                }
                let v = u64::from_be_bytes([
                    self.buf[self.pos],
                    self.buf[self.pos + 1],
                    self.buf[self.pos + 2],
                    self.buf[self.pos + 3],
                    self.buf[self.pos + 4],
                    self.buf[self.pos + 5],
                    self.buf[self.pos + 6],
                    self.buf[self.pos + 7],
                ]);
                self.pos += 8;
                v
            }
            _ => {
                return Err(DecodeError::InvalidCbor(format!(
                    "invalid additional info {ai}"
                )))
            }
        };
        Ok((major, len))
    }

    pub(crate) fn uint(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 0 {
            return Err(DecodeError::UnexpectedType("expected uint"));
        }
        Ok(len)
    }

    pub(crate) fn int(&mut self) -> Result<i64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        match major {
            0 => Ok(len as i64),
            1 => Ok(-1 - len as i64),
            _ => Err(DecodeError::UnexpectedType("expected int")),
        }
    }

    pub(crate) fn str(&mut self) -> Result<String, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 3 {
            return Err(DecodeError::UnexpectedType("expected string"));
        }
        let len = len as usize;
        if self.pos + len > self.buf.len() {
            return Err(DecodeError::Eof);
        }
        let s = std::str::from_utf8(&self.buf[self.pos..self.pos + len])
            .map_err(|e| DecodeError::InvalidCbor(format!("invalid UTF-8: {e}")))?;
        self.pos += len;
        Ok(s.to_string())
    }

    pub(crate) fn bool(&mut self) -> Result<bool, DecodeError> {
        let b = self.read_u8()?;
        match b {
            0xF5 => Ok(true),
            0xF4 => Ok(false),
            _ => Err(DecodeError::UnexpectedType("expected bool")),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn null(&mut self) -> Result<(), DecodeError> {
        let b = self.read_u8()?;
        if b == 0xF6 {
            Ok(())
        } else {
            Err(DecodeError::UnexpectedType("expected null"))
        }
    }

    pub(crate) fn is_null(&self) -> Result<bool, DecodeError> {
        Ok(self.peek()? == 0xF6)
    }

    pub(crate) fn array(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 4 {
            return Err(DecodeError::UnexpectedType("expected array"));
        }
        Ok(len)
    }

    pub(crate) fn map(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 5 {
            return Err(DecodeError::UnexpectedType("expected map"));
        }
        Ok(len)
    }

    pub(crate) fn tag(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 6 {
            return Err(DecodeError::UnexpectedType("expected tag"));
        }
        Ok(len)
    }

    pub(crate) fn skip(&mut self) -> Result<(), DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        match major {
            0 | 1 | 6 => Ok(()),
            2 => {
                self.pos += len as usize;
                if self.pos > self.buf.len() {
                    Err(DecodeError::Eof)
                } else {
                    Ok(())
                }
            }
            3 => {
                self.pos += len as usize;
                if self.pos > self.buf.len() {
                    Err(DecodeError::Eof)
                } else {
                    Ok(())
                }
            }
            4 => {
                for _ in 0..len {
                    self.skip()?;
                }
                Ok(())
            }
            5 => {
                for _ in 0..len {
                    self.skip()?;
                    self.skip()?;
                }
                Ok(())
            }
            7 => Ok(()),
            _ => Err(DecodeError::InvalidCbor(format!(
                "unknown major type {major}"
            ))),
        }
    }
}
