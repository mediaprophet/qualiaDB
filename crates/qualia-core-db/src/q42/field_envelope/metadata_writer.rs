//! Caller-buffered canonical TLV grammar, without schema/reference interpretation.
use super::{align, metadata, Error, Limits};

/// Numeric slices are encoded explicitly little endian, preserving finite bits.
#[derive(Clone, Copy, Debug)]
pub enum FieldValue<'a> {
    U32(&'a [u32]),
    U64(&'a [u64]),
    I64(&'a [i64]),
    F32(&'a [f32]),
    F64(&'a [f64]),
    Utf8(&'a str),
    Bytes(&'a [u8]),
    MetadataRefs(&'a [u32]),
    EntryRefs(&'a [u32]),
    U16(&'a [u16]),
    I32(&'a [i32]),
    /// Exact primitive/unknown wire value. Known tags obey the same primitive rules
    /// as typed variants; unknown tags stay bounded uninterpreted bytes. Critical
    /// unknown semantics still require rejection by the family semantic adapter.
    Raw {
        type_tag: u8,
        element_count: u32,
        bytes: &'a [u8],
    },
}
impl FieldValue<'_> {
    fn layout(self) -> Result<(u8, u32, usize), Error> {
        let (tag, count, width) = match self {
            Self::U32(v) => (1, v.len(), 4),
            Self::U64(v) => (2, v.len(), 8),
            Self::I64(v) => (3, v.len(), 8),
            Self::F32(v) => (4, v.len(), 4),
            Self::F64(v) => (5, v.len(), 8),
            Self::Utf8(v) => (6, v.len(), 1),
            Self::Bytes(v) => (7, v.len(), 1),
            Self::MetadataRefs(v) => (8, v.len(), 4),
            Self::EntryRefs(v) => (9, v.len(), 4),
            Self::U16(v) => (10, v.len(), 2),
            Self::I32(v) => (11, v.len(), 4),
            Self::Raw {
                type_tag,
                element_count,
                bytes,
            } => {
                u32::try_from(bytes.len()).map_err(|_| Error::Overflow)?;
                return Ok((type_tag, element_count, bytes.len()));
            }
        };
        let n = u32::try_from(count).map_err(|_| Error::Overflow)?;
        let bytes = count.checked_mul(width).ok_or(Error::Overflow)?;
        u32::try_from(bytes).map_err(|_| Error::Overflow)?;
        Ok((tag, n, bytes))
    }
    fn validate(self, limits: Limits) -> Result<(), Error> {
        match self {
            Self::F32(v) => {
                if v.iter().any(|x| !x.is_finite()) {
                    return Err(Error::Numeric);
                }
            }
            Self::F64(v) => {
                if v.iter().any(|x| !x.is_finite()) {
                    return Err(Error::Numeric);
                }
            }
            Self::Utf8(v) => metadata::validate_primitive(
                6,
                u32::try_from(v.len()).map_err(|_| Error::Overflow)?,
                v.as_bytes(),
                limits,
            )?,
            Self::Raw {
                type_tag,
                element_count,
                bytes,
            } => metadata::validate_primitive(type_tag, element_count, bytes, limits)?,
            _ => {}
        }
        Ok(())
    }
    fn write(self, out: &mut [u8]) {
        macro_rules! le {
            ($values:expr,$width:expr) => {
                for (dst, value) in out.chunks_exact_mut($width).zip($values) {
                    dst.copy_from_slice(&value.to_le_bytes());
                }
            };
        }
        match self {
            Self::U32(v) | Self::MetadataRefs(v) | Self::EntryRefs(v) => le!(v, 4),
            Self::U64(v) => le!(v, 8),
            Self::I64(v) => le!(v, 8),
            Self::F32(v) => le!(v, 4),
            Self::F64(v) => le!(v, 8),
            Self::U16(v) => le!(v, 2),
            Self::I32(v) => le!(v, 4),
            Self::Utf8(v) => out.copy_from_slice(v.as_bytes()),
            Self::Bytes(v) => out.copy_from_slice(v),
            Self::Raw { bytes, .. } => out.copy_from_slice(bytes),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct FieldInput<'a> {
    pub key: u16,
    pub flags: u8,
    pub value: FieldValue<'a>,
}
#[derive(Clone, Copy, Debug)]
pub struct BlockInput<'a> {
    pub id: u32,
    pub schema_type: u16,
    pub revision: u16,
    pub flags: u32,
    pub fields: &'a [FieldInput<'a>],
}
#[derive(Clone, Copy, Debug)]
pub struct MetadataInput<'a> {
    pub blocks: &'a [BlockInput<'a>],
}
fn field_len(field: FieldInput<'_>) -> Result<usize, Error> {
    let n = align(field.value.layout()?.2 as u64, 8)?
        .checked_add(16)
        .ok_or(Error::Overflow)?;
    usize::try_from(n).map_err(|_| Error::Extent)
}
fn body_len(block: BlockInput<'_>) -> Result<usize, Error> {
    let mut n = 0usize;
    for field in block.fields {
        n = n.checked_add(field_len(*field)?).ok_or(Error::Overflow)?;
    }
    u32::try_from(n).map_err(|_| Error::Overflow)?;
    Ok(n)
}
impl MetadataInput<'_> {
    /// Validate complete input, canonical nonzero ID/key ordering, bounds, primitive
    /// sizes/finiteness and UTF-8 before output is touched. This enforces metadata,
    /// block, field and string limits; unrelated file/payload limits belong to the
    /// enclosing EnvelopeInput. No allocations or semantic schema admission.
    pub fn encoded_len(&self, limits: Limits) -> Result<usize, Error> {
        if self.blocks.len() as u64 > u64::from(limits.blocks) {
            return Err(Error::Limit);
        }
        let mut previous = 0u32;
        let mut len = 0usize;
        for b in self.blocks {
            if b.id <= previous || b.flags & !1 != 0 {
                return Err(Error::Metadata);
            }
            previous = b.id;
            if b.fields.len() as u64 > u64::from(limits.fields_per_block) {
                return Err(Error::Limit);
            }
            let mut key = 0u16;
            for f in b.fields {
                if f.key <= key || f.flags & !1 != 0 {
                    return Err(Error::Metadata);
                }
                key = f.key;
            }
            let body = body_len(*b)?;
            len = len
                .checked_add(16)
                .and_then(|v| v.checked_add(body))
                .ok_or(Error::Overflow)?;
            if len as u64 > limits.metadata_bytes {
                return Err(Error::Limit);
            }
        }
        // Size admission precedes potentially large float/string scans.
        for b in self.blocks {
            for f in b.fields {
                f.value.validate(limits)?;
            }
        }
        Ok(len)
    }
    /// Every error leaves the entire output unchanged. Successful output has zero
    /// reserved/padding bytes; the unused caller tail is unchanged. Borrowing keeps
    /// input slices disjoint from mutable output. No hidden staging allocation.
    pub fn encode_into<'out>(
        &self,
        output: &'out mut [u8],
        limits: Limits,
    ) -> Result<&'out mut [u8], Error> {
        let len = self.encoded_len(limits)?;
        if output.len() < len {
            return Err(Error::Extent);
        }
        let out = &mut output[..len];
        out.fill(0);
        let mut cursor = 0usize;
        for b in self.blocks {
            let body = body_len(*b).expect("prevalidated metadata layout");
            out[cursor..cursor + 4].copy_from_slice(&b.id.to_le_bytes());
            out[cursor + 4..cursor + 6].copy_from_slice(&b.schema_type.to_le_bytes());
            out[cursor + 6..cursor + 8].copy_from_slice(&b.revision.to_le_bytes());
            out[cursor + 8..cursor + 12].copy_from_slice(&(body as u32).to_le_bytes());
            out[cursor + 12..cursor + 16].copy_from_slice(&b.flags.to_le_bytes());
            cursor += 16;
            for f in b.fields {
                let (tag, count, bytes) = f.value.layout().expect("prevalidated field layout");
                out[cursor..cursor + 2].copy_from_slice(&f.key.to_le_bytes());
                out[cursor + 2] = tag;
                out[cursor + 3] = f.flags;
                out[cursor + 4..cursor + 8].copy_from_slice(&count.to_le_bytes());
                out[cursor + 8..cursor + 12].copy_from_slice(&(bytes as u32).to_le_bytes());
                f.value.write(&mut out[cursor + 16..cursor + 16 + bytes]);
                cursor += field_len(*f).expect("prevalidated field length");
            }
        }
        Ok(out)
    }
}
