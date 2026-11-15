//! Minimal nonrecursive canonical CBOR for this fixed profile only.
use crate::crypto::network::errors::CryptoError;

pub(super) struct Writer<'a> {
    out: &'a mut [u8],
    pos: usize,
}
impl<'a> Writer<'a> {
    pub fn new(out: &'a mut [u8]) -> Self {
        Self { out, pos: 0 }
    }
    pub fn len(&self) -> usize {
        self.pos
    }
    pub fn raw(&mut self, bytes: &[u8]) -> Result<(), CryptoError> {
        let end = self
            .pos
            .checked_add(bytes.len())
            .ok_or(CryptoError::Range)?;
        self.out
            .get_mut(self.pos..end)
            .ok_or(CryptoError::Capacity)?
            .copy_from_slice(bytes);
        self.pos = end;
        Ok(())
    }
    pub fn head(&mut self, major: u8, value: u64) -> Result<(), CryptoError> {
        let prefix = major << 5;
        if value < 24 {
            self.raw(&[prefix | value as u8])
        } else if value <= u8::MAX as u64 {
            self.raw(&[prefix | 24, value as u8])
        } else if value <= u16::MAX as u64 {
            self.raw(&[prefix | 25])?;
            self.raw(&(value as u16).to_be_bytes())
        } else if value <= u32::MAX as u64 {
            self.raw(&[prefix | 26])?;
            self.raw(&(value as u32).to_be_bytes())
        } else {
            self.raw(&[prefix | 27])?;
            self.raw(&value.to_be_bytes())
        }
    }
    pub fn integer(&mut self, value: i64) -> Result<(), CryptoError> {
        if value >= 0 {
            self.head(0, value as u64)
        } else {
            self.head(1, (-1i128 - value as i128) as u64)
        }
    }
    pub fn bytes(&mut self, value: &[u8]) -> Result<(), CryptoError> {
        self.head(2, value.len() as u64)?;
        self.raw(value)
    }
    pub fn text(&mut self, value: &str) -> Result<(), CryptoError> {
        self.head(3, value.len() as u64)?;
        self.raw(value.as_bytes())
    }
}

pub(super) struct Reader<'a> {
    input: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, pos: 0 }
    }
    fn take(&mut self, len: usize) -> Result<&'a [u8], CryptoError> {
        let end = self.pos.checked_add(len).ok_or(CryptoError::Range)?;
        let result = self
            .input
            .get(self.pos..end)
            .ok_or(CryptoError::Truncated)?;
        self.pos = end;
        Ok(result)
    }
    pub fn head(&mut self, major: u8) -> Result<u64, CryptoError> {
        let first = self.take(1)?[0];
        if first >> 5 != major {
            return Err(CryptoError::Malformed);
        }
        let (n, min) = match first & 31 {
            n @ 0..=23 => return Ok(n as u64),
            24 => (1, 24),
            25 => (2, 256),
            26 => (4, 65536),
            27 => (8, 4294967296),
            _ => return Err(CryptoError::Malformed),
        };
        let mut value = 0u64;
        for byte in self.take(n)? {
            value = (value << 8) | u64::from(*byte);
        }
        if value < min {
            return Err(CryptoError::Malformed);
        }
        Ok(value)
    }
    pub fn expect(&mut self, major: u8, value: u64) -> Result<(), CryptoError> {
        if self.head(major)? != value {
            return Err(CryptoError::Malformed);
        }
        Ok(())
    }
    pub fn negative(&mut self, value: i64) -> Result<(), CryptoError> {
        self.expect(1, (-1i128 - value as i128) as u64)
    }
    pub fn bytes(&mut self, max: usize) -> Result<&'a [u8], CryptoError> {
        let len = self.head(2)?;
        if len > max as u64 {
            return Err(CryptoError::Capacity);
        }
        self.take(len as usize)
    }
    pub fn text_eq(&mut self, expected: &str) -> Result<(), CryptoError> {
        if self.head(3)? != expected.len() as u64 {
            return Err(CryptoError::UnknownProfile);
        }
        if self.take(expected.len())? != expected.as_bytes() {
            return Err(CryptoError::UnknownProfile);
        }
        Ok(())
    }
    pub fn finish(&self) -> Result<(), CryptoError> {
        if self.pos != self.input.len() {
            return Err(CryptoError::Malformed);
        }
        Ok(())
    }
}
