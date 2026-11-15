use super::Error;
/// Immutable original snapshot. Hashing is admitted before reading bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Original<'a> {
    bytes: &'a [u8],
    digest: [u8; 32],
}
impl<'a> Original<'a> {
    pub fn admit(bytes: &'a [u8], max_bytes: usize) -> Result<Self, Error> {
        if bytes.len() > max_bytes {
            return Err(Error::Budget);
        }
        Ok(Self {
            bytes,
            digest: *blake3::hash(bytes).as_bytes(),
        })
    }
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn digest(self) -> [u8; 32] {
        self.digest
    }
    pub fn region(self, offset: usize, length: usize) -> Result<Region<'a>, Error> {
        let end = offset.checked_add(length).ok_or(Error::Overflow)?;
        let bytes = self.bytes.get(offset..end).ok_or(Error::Bounds)?;
        Ok(Region {
            original: self,
            offset,
            bytes,
        })
    }
}
/// Selected byte range always retains its complete original, including other metadata.
#[derive(Clone, Copy, Debug)]
pub struct Region<'a> {
    original: Original<'a>,
    offset: usize,
    bytes: &'a [u8],
}
impl<'a> Region<'a> {
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
    pub fn original(self) -> Original<'a> {
        self.original
    }
    pub fn offset(self) -> usize {
        self.offset
    }
}
