//! Zero-heap memory layout types: `NQuin48`, typed `BorrowedSliceView`,
//! caller-buffered `CallerOutputSlice`, and generational `ResourceHandle`.

use crate::lifecycle::VibeTrap;

/// Canonical 48-byte, 16-byte aligned Super-Quin representation (`[u64; 6]`).
///
/// Layout matches `qualia_core_db::NQuin` bit-for-bit:
/// `[subject, predicate, object, context, metadata, parity]`, where
/// `parity == subject ^ predicate ^ object ^ context`.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NQuin48(pub [u64; 6]);

const _: () = {
    assert!(core::mem::size_of::<NQuin48>() == 48);
    assert!(core::mem::align_of::<NQuin48>() == 16);
};

impl NQuin48 {
    /// Constructs a parity-sealed `NQuin48` from its five semantic fields.
    #[inline]
    pub const fn from_parts(
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        metadata: u64,
    ) -> Self {
        let parity = subject ^ predicate ^ object ^ context;
        Self([subject, predicate, object, context, metadata, parity])
    }

    /// Subject field (`[0]`).
    #[inline]
    pub const fn subject(&self) -> u64 {
        self.0[0]
    }

    /// Predicate field (`[1]`).
    #[inline]
    pub const fn predicate(&self) -> u64 {
        self.0[1]
    }

    /// Object field (`[2]`).
    #[inline]
    pub const fn object(&self) -> u64 {
        self.0[2]
    }

    /// Context field (`[3]`).
    #[inline]
    pub const fn context(&self) -> u64 {
        self.0[3]
    }

    /// Metadata field (`[4]`).
    #[inline]
    pub const fn metadata(&self) -> u64 {
        self.0[4]
    }

    /// Parity field (`[5]`).
    #[inline]
    pub const fn parity(&self) -> u64 {
        self.0[5]
    }

    /// Expected XOR parity (`subject ^ predicate ^ object ^ context`).
    #[inline]
    pub const fn computed_parity(&self) -> u64 {
        self.0[0] ^ self.0[1] ^ self.0[2] ^ self.0[3]
    }

    /// Returns `true` when `parity == subject ^ predicate ^ object ^ context`.
    #[inline]
    pub const fn verify_parity(&self) -> bool {
        self.0[5] == self.computed_parity()
    }
}

/// Element type tag for safe borrowed slice views and caller-owned output buffers.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    U8 = 1,
    I32 = 2,
    U32 = 3,
    I64 = 4,
    U64 = 5,
    F32 = 6,
    F64 = 7,
    Bool = 8,
    NQuin48 = 9,
    ResourceHandle = 10,
    Utf8Bytes = 11,
}

impl PrimitiveKind {
    /// Exact byte size of a single element of this kind.
    #[inline]
    pub const fn elem_size(self) -> usize {
        match self {
            Self::U8 | Self::Bool | Self::Utf8Bytes => 1,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I64 | Self::U64 | Self::F64 => 8,
            Self::ResourceHandle => 24,
            Self::NQuin48 => 48,
        }
    }

    /// Required byte alignment when casting directly to a typed Rust slice.
    #[inline]
    pub const fn elem_align(self) -> usize {
        match self {
            Self::U8 | Self::Bool | Self::Utf8Bytes => 1,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::I64 | Self::U64 | Self::F64 | Self::ResourceHandle => 8,
            Self::NQuin48 => 16,
        }
    }
}

/// Generational, provider-scoped opaque handle to a host or domain resource.
///
/// Prevents raw pointer forging, cross-provider handle reuse, and use-after-close
/// stale slot bugs.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceHandle {
    /// `q_hash` of the owning provider's canonical IRI.
    pub owner_provider_hash: u64,
    /// Domain-defined resource kind tag.
    pub kind: u16,
    /// Access flags (e.g. read-only vs mutable).
    pub flags: u16,
    /// Slot generation counter; must match the table's live slot generation.
    pub generation: u32,
    /// Arena/table slot index.
    pub slot: u32,
    /// Reserved field (must be 0).
    pub reserved: u32,
}

const _: () = {
    assert!(core::mem::size_of::<ResourceHandle>() == 24);
    assert!(core::mem::align_of::<ResourceHandle>() == 8);
};

impl ResourceHandle {
    /// Creates a new `ResourceHandle`.
    #[inline]
    pub const fn new(
        owner_provider_hash: u64,
        kind: u16,
        flags: u16,
        generation: u32,
        slot: u32,
    ) -> Self {
        Self {
            owner_provider_hash,
            kind,
            flags,
            generation,
            slot,
            reserved: 0,
        }
    }

    /// Validates that this handle belongs to `expected_owner`, has `expected_kind`,
    /// and matches `expected_generation`.
    #[inline]
    pub const fn validate(
        &self,
        expected_owner: u64,
        expected_kind: u16,
        expected_generation: u32,
    ) -> Result<(), VibeTrap> {
        if self.owner_provider_hash != expected_owner || self.kind != expected_kind {
            return Err(VibeTrap::InvalidResourceHandleOwner);
        }
        if self.generation == 0 || self.generation != expected_generation {
            return Err(VibeTrap::StaleResourceHandleGeneration);
        }
        Ok(())
    }
}

/// Checked immutable view over a caller-supplied input buffer.
#[derive(Debug, Clone, Copy)]
pub struct BorrowedSliceView<'a> {
    bytes: &'a [u8],
    kind: PrimitiveKind,
    elem_count: usize,
}

impl<'a> BorrowedSliceView<'a> {
    /// Constructs a validated `BorrowedSliceView` from raw bytes and an element count.
    #[inline]
    pub fn from_raw_bytes(
        bytes: &'a [u8],
        kind: PrimitiveKind,
        elem_count: usize,
    ) -> Result<Self, VibeTrap> {
        let expected_bytes = elem_count
            .checked_mul(kind.elem_size())
            .ok_or(VibeTrap::SliceBoundsOverflow)?;
        if bytes.len() != expected_bytes {
            return Err(VibeTrap::SliceLengthMismatch);
        }
        Ok(Self {
            bytes,
            kind,
            elem_count,
        })
    }

    /// Constructs a `BorrowedSliceView` from a slice of parity-verified `NQuin48` records.
    #[inline]
    pub fn from_nquins(quins: &'a [NQuin48]) -> Result<Self, VibeTrap> {
        let mut i = 0;
        while i < quins.len() {
            if !quins[i].verify_parity() {
                return Err(VibeTrap::InvalidQuinParity);
            }
            i += 1;
        }
        let byte_len = quins
            .len()
            .checked_mul(core::mem::size_of::<NQuin48>())
            .ok_or(VibeTrap::SliceBoundsOverflow)?;
        // SAFETY: `NQuin48` is `#[repr(C, align(16))]` of `[u64; 6]` with no padding bytes.
        let bytes = unsafe { core::slice::from_raw_parts(quins.as_ptr() as *const u8, byte_len) };
        Ok(Self {
            bytes,
            kind: PrimitiveKind::NQuin48,
            elem_count: quins.len(),
        })
    }

    /// Element kind.
    #[inline]
    pub const fn kind(&self) -> PrimitiveKind {
        self.kind
    }

    /// Alias for `kind()`.
    #[inline]
    pub const fn primitive(&self) -> PrimitiveKind {
        self.kind
    }

    /// Number of typed elements.
    #[inline]
    pub const fn elem_count(&self) -> usize {
        self.elem_count
    }

    /// Alias for `elem_count()`.
    #[inline]
    pub const fn len(&self) -> usize {
        self.elem_count
    }

    /// Returns `true` when `elem_count == 0`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.elem_count == 0
    }

    /// Underlying byte slice.
    #[inline]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Reads the `index`-th `f64` (little-endian) without requiring 8-byte pointer alignment.
    #[inline]
    pub fn read_f64_le(&self, index: usize) -> Result<f64, VibeTrap> {
        if self.kind != PrimitiveKind::F64 {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        let offset = index * 8;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.bytes[offset..offset + 8]);
        Ok(f64::from_le_bytes(buf))
    }

    /// Reads the `index`-th `f32` (little-endian).
    #[inline]
    pub fn read_f32_le(&self, index: usize) -> Result<f32, VibeTrap> {
        if self.kind != PrimitiveKind::F32 {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        let offset = index * 4;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.bytes[offset..offset + 4]);
        Ok(f32::from_le_bytes(buf))
    }

    /// Reads the `index`-th `u64` (little-endian).
    #[inline]
    pub fn read_u64_le(&self, index: usize) -> Result<u64, VibeTrap> {
        if self.kind != PrimitiveKind::U64 {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        let offset = index * 8;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.bytes[offset..offset + 8]);
        Ok(u64::from_le_bytes(buf))
    }

    /// Reads the `index`-th `i64` (little-endian).
    #[inline]
    pub fn read_i64_le(&self, index: usize) -> Result<i64, VibeTrap> {
        if self.kind != PrimitiveKind::I64 {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        let offset = index * 8;
        let mut buf = [0u8; 8];
        buf.copy_from_slice(&self.bytes[offset..offset + 8]);
        Ok(i64::from_le_bytes(buf))
    }

    /// Reads the `index`-th `u32` (little-endian).
    #[inline]
    pub fn read_u32_le(&self, index: usize) -> Result<u32, VibeTrap> {
        if self.kind != PrimitiveKind::U32 {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        let offset = index * 4;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.bytes[offset..offset + 4]);
        Ok(u32::from_le_bytes(buf))
    }

    /// Reads the `index`-th `i32` (little-endian).
    #[inline]
    pub fn read_i32_le(&self, index: usize) -> Result<i32, VibeTrap> {
        if self.kind != PrimitiveKind::I32 {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        let offset = index * 4;
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&self.bytes[offset..offset + 4]);
        Ok(i32::from_le_bytes(buf))
    }

    /// Reads the `index`-th `u8`.
    #[inline]
    pub fn read_u8(&self, index: usize) -> Result<u8, VibeTrap> {
        if self.kind != PrimitiveKind::U8 {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        Ok(self.bytes[index])
    }

    /// Reads the `index`-th `bool`.
    #[inline]
    pub fn read_bool(&self, index: usize) -> Result<bool, VibeTrap> {
        if self.kind != PrimitiveKind::Bool {
            return Err(VibeTrap::TypeMismatch);
        }
        if index >= self.elem_count {
            return Err(VibeTrap::SliceBoundsOverflow);
        }
        Ok(self.bytes[index] != 0)
    }
}

/// Caller-supplied mutable output buffer with pre-dispatch capacity checks and explicit written count.
#[derive(Debug)]
pub struct CallerOutputSlice<'a> {
    bytes: &'a mut [u8],
    kind: PrimitiveKind,
    capacity_elems: usize,
    written_elems: usize,
}

impl<'a> CallerOutputSlice<'a> {
    /// Wraps a caller-owned mutable byte buffer for elements of `kind`.
    #[inline]
    pub fn new(bytes: &'a mut [u8], kind: PrimitiveKind) -> Result<Self, VibeTrap> {
        let elem_size = kind.elem_size();
        if bytes.len() % elem_size != 0 {
            return Err(VibeTrap::SliceLengthMismatch);
        }
        let capacity_elems = bytes.len() / elem_size;
        Ok(Self {
            bytes,
            kind,
            capacity_elems,
            written_elems: 0,
        })
    }

    /// Expected element kind.
    #[inline]
    pub const fn kind(&self) -> PrimitiveKind {
        self.kind
    }

    /// Alias for `kind()`.
    #[inline]
    pub const fn primitive(&self) -> PrimitiveKind {
        self.kind
    }

    /// Total capacity in elements.
    #[inline]
    pub const fn capacity_elems(&self) -> usize {
        self.capacity_elems
    }

    /// Total capacity in bytes.
    #[inline]
    pub const fn capacity_bytes(&self) -> usize {
        self.bytes.len()
    }

    /// Number of elements written so far.
    #[inline]
    pub const fn written_elems(&self) -> usize {
        self.written_elems
    }

    /// Number of bytes written so far.
    #[inline]
    pub const fn written_bytes(&self) -> usize {
        self.written_elems * self.kind.elem_size()
    }

    /// Verifies that the output buffer can hold at least `required_elems` before execution begins.
    #[inline]
    pub const fn ensure_elem_capacity(&self, required_elems: usize) -> Result<(), VibeTrap> {
        if required_elems > self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        Ok(())
    }

    /// Appends one `f64` (little-endian) into the output buffer.
    #[inline]
    pub fn push_f64_le(&mut self, value: f64) -> Result<(), VibeTrap> {
        if self.kind != PrimitiveKind::F64 {
            return Err(VibeTrap::TypeMismatch);
        }
        if self.written_elems >= self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        let offset = self.written_elems * 8;
        self.bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        self.written_elems += 1;
        Ok(())
    }

    /// Appends one `f32` (little-endian) into the output buffer.
    #[inline]
    pub fn push_f32_le(&mut self, value: f32) -> Result<(), VibeTrap> {
        if self.kind != PrimitiveKind::F32 {
            return Err(VibeTrap::TypeMismatch);
        }
        if self.written_elems >= self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        let offset = self.written_elems * 4;
        self.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        self.written_elems += 1;
        Ok(())
    }

    /// Appends one `u64` (little-endian) into the output buffer.
    #[inline]
    pub fn push_u64_le(&mut self, value: u64) -> Result<(), VibeTrap> {
        if self.kind != PrimitiveKind::U64 {
            return Err(VibeTrap::TypeMismatch);
        }
        if self.written_elems >= self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        let offset = self.written_elems * 8;
        self.bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        self.written_elems += 1;
        Ok(())
    }

    /// Appends one `i64` (little-endian) into the output buffer.
    #[inline]
    pub fn push_i64_le(&mut self, value: i64) -> Result<(), VibeTrap> {
        if self.kind != PrimitiveKind::I64 {
            return Err(VibeTrap::TypeMismatch);
        }
        if self.written_elems >= self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        let offset = self.written_elems * 8;
        self.bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        self.written_elems += 1;
        Ok(())
    }

    /// Appends one `u8` into the output buffer.
    #[inline]
    pub fn push_u8(&mut self, value: u8) -> Result<(), VibeTrap> {
        if self.kind != PrimitiveKind::U8 {
            return Err(VibeTrap::TypeMismatch);
        }
        if self.written_elems >= self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        self.bytes[self.written_elems] = value;
        self.written_elems += 1;
        Ok(())
    }

    /// Appends one `bool` into the output buffer.
    #[inline]
    pub fn push_bool(&mut self, value: bool) -> Result<(), VibeTrap> {
        if self.kind != PrimitiveKind::Bool {
            return Err(VibeTrap::TypeMismatch);
        }
        if self.written_elems >= self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        self.bytes[self.written_elems] = if value { 1 } else { 0 };
        self.written_elems += 1;
        Ok(())
    }

    /// Writes `elem_count` elements directly from `raw_bytes` into the output buffer.
    #[inline]
    pub fn write_all(&mut self, raw_bytes: &[u8], elem_count: usize) -> Result<(), VibeTrap> {
        let expected_bytes = elem_count
            .checked_mul(self.kind.elem_size())
            .ok_or(VibeTrap::SliceBoundsOverflow)?;
        if raw_bytes.len() != expected_bytes {
            return Err(VibeTrap::SliceLengthMismatch);
        }
        if self.written_elems.saturating_add(elem_count) > self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        let offset = self.written_bytes();
        self.bytes[offset..offset + expected_bytes].copy_from_slice(raw_bytes);
        self.written_elems += elem_count;
        Ok(())
    }

    /// Appends one parity-verified `NQuin48` into the output buffer.
    #[inline]
    pub fn push_nquin(&mut self, quin: &NQuin48) -> Result<(), VibeTrap> {
        if self.kind != PrimitiveKind::NQuin48 {
            return Err(VibeTrap::TypeMismatch);
        }
        if !quin.verify_parity() {
            return Err(VibeTrap::InvalidQuinParity);
        }
        if self.written_elems >= self.capacity_elems {
            return Err(VibeTrap::OutputBufferTooSmall);
        }
        let offset = self.written_elems * 48;
        let mut word = 0;
        while word < 6 {
            let w_off = offset + word * 8;
            self.bytes[w_off..w_off + 8].copy_from_slice(&quin.0[word].to_le_bytes());
            word += 1;
        }
        self.written_elems += 1;
        Ok(())
    }

    /// Returns the written prefix of the output buffer as an immutable byte slice.
    #[inline]
    pub fn written_byte_slice(&self) -> &[u8] {
        &self.bytes[..self.written_bytes()]
    }
}
