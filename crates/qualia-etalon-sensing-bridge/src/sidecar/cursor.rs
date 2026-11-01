use super::{SidecarError, SIDECAR_SIZE};

pub(super) struct Writer {
    bytes: [u8; SIDECAR_SIZE],
    cursor: usize,
    overflow: bool,
}
impl Writer {
    pub fn new() -> Self {
        Self {
            bytes: [0; SIDECAR_SIZE],
            cursor: 0,
            overflow: false,
        }
    }
    pub fn bytes(&mut self, value: &[u8]) {
        let Some(end) = self.cursor.checked_add(value.len()) else {
            self.overflow = true;
            return;
        };
        let Some(target) = self.bytes.get_mut(self.cursor..end) else {
            self.overflow = true;
            return;
        };
        target.copy_from_slice(value);
        self.cursor = end;
    }
    pub fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }
    pub fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }
    pub fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }
    pub fn finish(self) -> Result<[u8; SIDECAR_SIZE], SidecarError> {
        if self.overflow {
            Err(SidecarError::BufferSize)
        } else {
            Ok(self.bytes)
        }
    }
}

pub(super) struct Reader<'a> {
    input: &'a [u8],
    cursor: usize,
}
impl<'a> Reader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self { input, cursor: 0 }
    }
    pub fn bytes<const N: usize>(&mut self) -> Result<[u8; N], SidecarError> {
        let end = self
            .cursor
            .checked_add(N)
            .ok_or(SidecarError::InvalidEncoding)?;
        let slice = self
            .input
            .get(self.cursor..end)
            .ok_or(SidecarError::InvalidEncoding)?;
        self.cursor = end;
        slice.try_into().map_err(|_| SidecarError::InvalidEncoding)
    }
    pub fn u8(&mut self) -> Result<u8, SidecarError> {
        Ok(self.bytes::<1>()?[0])
    }
    pub fn u32(&mut self) -> Result<u32, SidecarError> {
        Ok(u32::from_le_bytes(self.bytes()?))
    }
    pub fn u64(&mut self) -> Result<u64, SidecarError> {
        Ok(u64::from_le_bytes(self.bytes()?))
    }
}
