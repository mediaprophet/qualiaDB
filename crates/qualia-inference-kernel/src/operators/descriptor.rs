//! Immutable operator identity: kind, shape, scale convention, digest.

use super::error::OperatorError;

/// Registered representation. Unknown values must fail closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OperatorKind {
    /// Row-major IEEE-754 f32 matrix, little-endian bytes.
    DenseF32 = 1,
    /// GGML Q4_K superblocks (256 weights / 144 bytes). Not DirectML 20-byte blocks.
    Q4KBitPlane = 2,
}

/// Accumulation type for `apply_into`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AccumKind {
    F32 = 1,
}

/// How packed scales are stored in the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ScaleLayout {
    /// Dense f32 has no separate scale plane.
    None = 0,
    /// GGML Q4_K: f16 `d`/`dmin` plus 12-byte 6-bit packed sub-scales (`get_scale_min_k4`).
    GgmlQ4K = 1,
}

/// Declared operator. Validated before any output write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatorDescriptor {
    pub kind: OperatorKind,
    pub in_features: u32,
    pub out_features: u32,
    pub batch_hint: u32,
    /// Independent decode tile in elements (256 for Q4_K; `in_features` for dense).
    pub tile_elems: u32,
    pub scale_layout: ScaleLayout,
    pub accum: AccumKind,
    pub max_workspace_bytes: u32,
    /// Content digest of the representation (not a routing hash). Zero is allowed only in tests.
    pub representation_digest: u64,
}

impl OperatorDescriptor {
    /// Structural checks that do not inspect payload bytes.
    pub fn check_shape(&self) -> Result<(), OperatorError> {
        if self.in_features == 0 || self.out_features == 0 {
            return Err(OperatorError::UnsupportedShape);
        }
        if self.tile_elems == 0 {
            return Err(OperatorError::MalformedDescriptor);
        }
        if self.accum != AccumKind::F32 {
            return Err(OperatorError::UnsupportedKind);
        }
        match self.kind {
            OperatorKind::DenseF32 => {
                if self.scale_layout != ScaleLayout::None {
                    return Err(OperatorError::MalformedDescriptor);
                }
                if self.tile_elems != self.in_features && self.tile_elems != 1 {
                    return Err(OperatorError::MalformedDescriptor);
                }
            }
            OperatorKind::Q4KBitPlane => {
                if self.scale_layout != ScaleLayout::GgmlQ4K {
                    return Err(OperatorError::MalformedDescriptor);
                }
                if self.tile_elems != Q4K_SUPERBLOCK_ELEMS {
                    return Err(OperatorError::MalformedDescriptor);
                }
            }
        }
        Ok(())
    }

    /// `out_features * in_features` as `usize`, or `UnsupportedShape` on overflow.
    pub fn matrix_elems(&self) -> Result<usize, OperatorError> {
        (self.out_features as usize)
            .checked_mul(self.in_features as usize)
            .ok_or(OperatorError::UnsupportedShape)
    }
}

/// GGML Q4_K superblock element count.
pub const Q4K_SUPERBLOCK_ELEMS: u32 = 256;
/// GGML Q4_K superblock size in bytes.
pub const Q4K_SUPERBLOCK_BYTES: usize = 144;

#[cfg(test)]
mod tests {
    use super::*;

    fn dense() -> OperatorDescriptor {
        OperatorDescriptor {
            kind: OperatorKind::DenseF32,
            in_features: 3,
            out_features: 2,
            batch_hint: 1,
            tile_elems: 3,
            scale_layout: ScaleLayout::None,
            accum: AccumKind::F32,
            max_workspace_bytes: 0,
            representation_digest: 1,
        }
    }

    #[test]
    fn dense_ok() {
        assert!(dense().check_shape().is_ok());
    }

    #[test]
    fn zero_features_rejected() {
        let mut d = dense();
        d.in_features = 0;
        assert_eq!(d.check_shape(), Err(OperatorError::UnsupportedShape));
    }

    #[test]
    fn q4k_requires_ggml_scale_layout() {
        let d = OperatorDescriptor {
            kind: OperatorKind::Q4KBitPlane,
            in_features: 256,
            out_features: 1,
            batch_hint: 1,
            tile_elems: Q4K_SUPERBLOCK_ELEMS,
            scale_layout: ScaleLayout::None,
            accum: AccumKind::F32,
            max_workspace_bytes: 0,
            representation_digest: 1,
        };
        assert_eq!(d.check_shape(), Err(OperatorError::MalformedDescriptor));
    }
}
