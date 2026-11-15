#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatWidth {
    F32,
    F64,
}
impl FloatWidth {
    pub fn bytes(self) -> usize {
        match self {
            Self::F32 => 4,
            Self::F64 => 8,
        }
    }
    pub fn pixel_tag(self) -> u32 {
        match self {
            Self::F32 => 0x7fe00008,
            Self::F64 => 0x7fe00009,
        }
    }
}
/// Raw IEEE754 lanes. No float arithmetic/canonicalization is needed to retain
/// signed zero, infinities, signaling/quiet NaNs or their payload/sign bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatBits {
    F32(u32),
    F64(u64),
}
impl FloatBits {
    pub fn width(self) -> FloatWidth {
        match self {
            Self::F32(_) => FloatWidth::F32,
            Self::F64(_) => FloatWidth::F64,
        }
    }
    pub fn is_nan(self) -> bool {
        match self {
            Self::F32(b) => (b & 0x7f800000) == 0x7f800000 && (b & 0x007fffff) != 0,
            Self::F64(b) => {
                (b & 0x7ff0000000000000) == 0x7ff0000000000000 && (b & 0x000fffffffffffff) != 0
            }
        }
    }
    pub fn is_finite(self) -> bool {
        match self {
            Self::F32(b) => (b & 0x7f800000) != 0x7f800000,
            Self::F64(b) => (b & 0x7ff0000000000000) != 0x7ff0000000000000,
        }
    }
    pub fn finite_f64(self) -> Option<f64> {
        if !self.is_finite() {
            return None;
        }
        self.ordered_f64()
    }
    /// Numeric comparison helper; NaNs remain unordered, not zero or a sentinel.
    pub fn ordered_f64(self) -> Option<f64> {
        if self.is_nan() {
            return None;
        }
        Some(match self {
            Self::F32(b) => f32::from_bits(b) as f64,
            Self::F64(b) => f64::from_bits(b),
        })
    }
}
