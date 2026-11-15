use super::*;
/// Bounded block view; schema/revision/critical flags are NOT semantically admitted.
#[derive(Clone, Copy, Debug)]
pub struct Block<'a> {
    raw: &'a [u8],
    body: &'a [u8],
}
impl<'a> Block<'a> {
    pub fn id(self) -> u32 {
        u32le(self.raw, 0)
    }
    pub fn schema_type(self) -> u16 {
        u16le(self.raw, 4)
    }
    pub fn revision(self) -> u16 {
        u16le(self.raw, 6)
    }
    pub fn flags(self) -> u32 {
        u32le(self.raw, 12)
    }
    pub fn fields(self) -> Fields<'a> {
        Fields {
            remaining: self.body,
        }
    }
    pub fn raw_bytes(self) -> &'a [u8] {
        self.raw
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Field<'a> {
    raw: &'a [u8],
    value: &'a [u8],
}
impl<'a> Field<'a> {
    pub fn key(self) -> u16 {
        u16le(self.raw, 0)
    }
    pub fn type_tag(self) -> u8 {
        self.raw[2]
    }
    pub fn flags(self) -> u8 {
        self.raw[3]
    }
    pub fn element_count(self) -> u32 {
        u32le(self.raw, 4)
    }
    pub fn value(self) -> &'a [u8] {
        self.value
    }
    pub fn raw_bytes(self) -> &'a [u8] {
        self.raw
    }
}
/// Infallible iterators are only constructed after complete structural validation.
#[derive(Clone, Debug)]
pub struct Blocks<'a> {
    remaining: &'a [u8],
}
impl<'a> Blocks<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }
}
impl<'a> Iterator for Blocks<'a> {
    type Item = Block<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        let n = u32le(self.remaining, 8) as usize;
        let total = 16 + ((n + 7) & !7);
        let (raw, tail) = self.remaining.split_at(total);
        self.remaining = tail;
        Some(Block {
            raw,
            body: &raw[16..16 + n],
        })
    }
}
#[derive(Clone, Debug)]
pub struct Fields<'a> {
    remaining: &'a [u8],
}
impl<'a> Iterator for Fields<'a> {
    type Item = Field<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }
        let n = u32le(self.remaining, 8) as usize;
        let total = 16 + ((n + 7) & !7);
        let (raw, tail) = self.remaining.split_at(total);
        self.remaining = tail;
        Some(Field {
            raw,
            value: &raw[16..16 + n],
        })
    }
}
fn item(bytes: &[u8]) -> Result<(&[u8], &[u8], &[u8]), Error> {
    let h = bytes.get(..16).ok_or(Error::Metadata)?;
    let n = u64::from(u32le(h, 8));
    let padded = align(n, 8)?;
    let value = extent(bytes, 16, n)?;
    zero(extent(bytes, 16 + n, padded - n)?)?;
    let total = 16u64.checked_add(padded).ok_or(Error::Overflow)?;
    let tail = bytes
        .get(usize::try_from(total).map_err(|_| Error::Extent)?..)
        .ok_or(Error::Metadata)?;
    Ok((h, value, tail))
}
/// Validate grammar, primitive sizes and finite numeric metadata. This deliberately
/// does not resolve references or recognize required/unknown schema fields.
pub(super) fn validate(mut bytes: &[u8], limits: Limits) -> Result<(), Error> {
    if bytes.len() as u64 > limits.metadata_bytes {
        return Err(Error::Limit);
    }
    let mut previous = 0;
    let mut blocks = 0u32;
    while !bytes.is_empty() {
        blocks = blocks.checked_add(1).ok_or(Error::Overflow)?;
        if blocks > limits.blocks {
            return Err(Error::Limit);
        }
        let (h, mut body, tail) = item(bytes)?;
        bytes = tail;
        let id = u32le(h, 0);
        if id <= previous || u32le(h, 12) & !1 != 0 {
            return Err(Error::Metadata);
        }
        previous = id;
        let mut previous_key = 0;
        let mut fields = 0u32;
        while !body.is_empty() {
            fields = fields.checked_add(1).ok_or(Error::Overflow)?;
            if fields > limits.fields_per_block {
                return Err(Error::Limit);
            }
            let (h, value, tail) = item(body)?;
            body = tail;
            let key = u16le(h, 0);
            if key <= previous_key || h[3] & !1 != 0 || u32le(h, 12) != 0 {
                return Err(Error::Metadata);
            }
            previous_key = key;
            validate_primitive(h[2], u32le(h, 4), value, limits)?;
        }
    }
    Ok(())
}

/// Primitive grammar shared by the reader and caller-buffered metadata writer.
pub(super) fn validate_primitive(
    tag: u8,
    count: u32,
    value: &[u8],
    limits: Limits,
) -> Result<(), Error> {
    let width = match tag {
        1 | 4 | 8 | 9 | 11 => Some(4u64),
        2 | 3 | 5 => Some(8),
        6 | 7 => Some(1),
        10 => Some(2),
        _ => None,
    };
    if let Some(width) = width {
        let expected = u64::from(count).checked_mul(width).ok_or(Error::Overflow)?;
        if expected != value.len() as u64 {
            return Err(Error::Numeric);
        }
    }
    match tag {
        4 => {
            for v in value.chunks_exact(4) {
                if !f32::from_le_bytes(v.try_into().unwrap()).is_finite() {
                    return Err(Error::Numeric);
                }
            }
        }
        5 => {
            for v in value.chunks_exact(8) {
                if !f64::from_le_bytes(v.try_into().unwrap()).is_finite() {
                    return Err(Error::Numeric);
                }
            }
        }
        6 => {
            if value.len() as u64 > u64::from(limits.string_bytes) {
                return Err(Error::Limit);
            }
            if value.iter().any(u8::is_ascii_control) || std::str::from_utf8(value).is_err() {
                return Err(Error::Metadata);
            }
        }
        _ => {}
    }
    Ok(())
}
