//! Borrowed scalar access independent of pointer alignment.
/// A validated sequence of finite little-endian IEEE-754 f32 values.
#[derive(Clone, Copy, Debug)]
pub struct F32LeSlice<'a>(pub(super) &'a [u8]);
impl<'a> F32LeSlice<'a> {
    pub fn len(self) -> usize {
        self.0.len() / 4
    }
    pub fn is_empty(self) -> bool {
        self.0.is_empty()
    }
    pub fn as_bytes(self) -> &'a [u8] {
        self.0
    }
    pub fn get(self, index: usize) -> Option<f32> {
        let start = index.checked_mul(4)?;
        let raw = self.0.get(start..start.checked_add(4)?)?;
        Some(f32::from_le_bytes(raw.try_into().ok()?))
    }
    pub fn iter(self) -> impl ExactSizeIterator<Item = f32> + DoubleEndedIterator + 'a {
        self.0
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    /// Native borrowing is optional; get/iter also work on unaligned buffers.
    pub fn as_native_slice(self) -> Option<&'a [f32]> {
        if cfg!(target_endian = "little") {
            bytemuck::try_cast_slice(self.0).ok()
        } else {
            None
        }
    }
}
