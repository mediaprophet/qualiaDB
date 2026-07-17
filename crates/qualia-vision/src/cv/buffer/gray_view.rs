//! Gray8 image view (borrowed).

#[derive(Debug, Clone, Copy)]
pub struct GrayView<'a> {
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub bytes: &'a [u8],
}

impl<'a> GrayView<'a> {
    pub fn new(width: u32, height: u32, stride: u32, bytes: &'a [u8]) -> Option<Self> {
        let need = stride.checked_mul(height)? as usize;
        if bytes.len() < need || width == 0 || height == 0 || stride < width {
            return None;
        }
        Some(Self {
            width,
            height,
            stride,
            bytes,
        })
    }

    #[inline]
    pub fn pixel(&self, x: u32, y: u32) -> u8 {
        self.bytes[(y * self.stride + x) as usize]
    }
}
