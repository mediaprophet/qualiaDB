use super::layout::{P64TensorEntry, P64_MANIFOLD_ENTRY_BYTES};

#[derive(Clone, Copy, Debug)]
pub struct P64TensorView<'a> {
    pub(super) entry: P64TensorEntry,
    pub(super) name: &'a [u8],
    pub(super) blob: &'a [u8],
}

impl<'a> P64TensorView<'a> {
    pub fn entry(&self) -> P64TensorEntry {
        self.entry
    }
    pub fn name_bytes(&self) -> &'a [u8] {
        self.name
    }
    pub fn blob(&self) -> &'a [u8] {
        self.blob
    }
}

/// Raw manifold bytes. No semantic or finite-value interpretation is imposed.
#[derive(Clone, Copy, Debug)]
pub struct P64ManifoldRecord<'a> {
    pub(super) bytes: &'a [u8; P64_MANIFOLD_ENTRY_BYTES],
}

impl<'a> P64ManifoldRecord<'a> {
    /// Wrap a borrowed 64-byte manifold record.
    #[inline]
    pub const fn from_bytes(bytes: &'a [u8; P64_MANIFOLD_ENTRY_BYTES]) -> Self {
        Self { bytes }
    }

    /// Borrow the first 64 bytes of `slice` as a [`P64ManifoldRecord`].
    #[inline]
    pub fn from_slice(slice: &'a [u8]) -> Option<Self> {
        let bytes: &'a [u8; P64_MANIFOLD_ENTRY_BYTES] =
            slice.get(..P64_MANIFOLD_ENTRY_BYTES)?.try_into().ok()?;
        Some(Self { bytes })
    }

    pub fn bytes(&self) -> &'a [u8; P64_MANIFOLD_ENTRY_BYTES] {
        self.bytes
    }

    pub fn values_le(&self) -> [f32; 10] {
        core::array::from_fn(|index| {
            let start = index * 4;
            f32::from_le_bytes(self.bytes[start..start + 4].try_into().unwrap())
        })
    }

    /// Decode the 10 little-endian `f32` coordinates and verify all 10 are finite.
    pub fn values_le_finite(&self) -> Option<[f32; 10]> {
        let values = self.values_le();
        if values.iter().all(|v| v.is_finite()) {
            Some(values)
        } else {
            None
        }
    }

    /// Serialize 10 little-endian `f32` values + 24 zero padding bytes into a 64-byte record slot.
    pub fn write_values_le(values: &[f32; 10], out: &mut [u8]) {
        assert!(out.len() >= P64_MANIFOLD_ENTRY_BYTES);
        out[..P64_MANIFOLD_ENTRY_BYTES].fill(0);
        for (index, value) in values.iter().enumerate() {
            let start = index * 4;
            out[start..start + 4].copy_from_slice(&value.to_le_bytes());
        }
    }
}
