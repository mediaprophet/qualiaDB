//! Borrowed definite-field decoding. No recursion or owned decoder graph.
use super::{IndexError, IndexReadLimits};
use minicbor::{Decoder, data::Type};

pub(super) fn checked<T>(v: Result<T, minicbor::decode::Error>) -> Result<T, IndexError> {
    v.map_err(|_| IndexError::Malformed)
}

#[derive(Clone, Copy, Debug)]
pub(super) enum ByteField<'a> {
    Bytes(&'a [u8]),
    Array { encoded: &'a [u8], len: usize },
}
impl ByteField<'_> {
    pub(super) fn len(self) -> usize {
        match self {
            Self::Bytes(b) => b.len(),
            Self::Array { len, .. } => len,
        }
    }
    pub(super) fn copy_into(self, out: &mut [u8]) -> Result<usize, IndexError> {
        let len = self.len();
        if out.len() < len {
            return Err(IndexError::Scratch);
        }
        match self {
            Self::Bytes(b) => out[..len].copy_from_slice(b),
            Self::Array { encoded, .. } => {
                let mut d = Decoder::new(encoded);
                checked(d.array())?.ok_or(IndexError::Malformed)?;
                for v in &mut out[..len] {
                    *v = checked(d.u8())?;
                }
            }
        }
        Ok(len)
    }
}

fn byte_field<'a>(d: &mut Decoder<'a>, cap: usize) -> Result<ByteField<'a>, IndexError> {
    match checked(d.datatype())? {
        Type::Bytes => {
            let bytes = checked(d.bytes())?;
            if bytes.len() > cap {
                return Err(IndexError::ResidentLimit);
            }
            Ok(ByteField::Bytes(bytes))
        }
        Type::Array => {
            let start = d.position();
            let n = checked(d.array())?.ok_or(IndexError::Malformed)?;
            let len = usize::try_from(n).map_err(|_| IndexError::Overflow)?;
            if len > cap {
                return Err(IndexError::ResidentLimit);
            }
            for _ in 0..len {
                checked(d.u8())?;
            }
            Ok(ByteField::Array {
                encoded: &d.input()[start..d.position()],
                len,
            })
        }
        _ => Err(IndexError::Malformed),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct EntryView<'a> {
    pub(super) key: &'a str,
    pub(super) kind: &'a str,
    pub(super) offset: u64,
    pub(super) length: u64,
    pub(super) sha256: [u8; 32],
    pub(super) meta: Option<ByteField<'a>>,
}
impl<'a> EntryView<'a> {
    pub const fn key(&self) -> &'a str {
        self.key
    }
    pub const fn kind(&self) -> &'a str {
        self.kind
    }
    pub const fn offset(&self) -> u64 {
        self.offset
    }
    pub const fn length(&self) -> u64 {
        self.length
    }
    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
    pub fn metadata_len(&self) -> Option<usize> {
        self.meta.map(ByteField::len)
    }
    pub fn copy_metadata_into(&self, out: &mut [u8]) -> Result<Option<usize>, IndexError> {
        self.meta.map(|m| m.copy_into(out)).transpose()
    }
    pub(super) fn retained_bytes(&self) -> Result<usize, IndexError> {
        self.key
            .len()
            .checked_add(self.kind.len())
            .and_then(|n| n.checked_add(32))
            .and_then(|n| n.checked_add(self.metadata_len().unwrap_or(0)))
            .ok_or(IndexError::Overflow)
    }
}

pub(super) fn decode_row<'a>(
    d: &mut Decoder<'a>,
    l: IndexReadLimits,
) -> Result<EntryView<'a>, IndexError> {
    let fields = checked(d.map())?.ok_or(IndexError::Malformed)?;
    if !(5..=6).contains(&fields) {
        return Err(IndexError::MissingField);
    }
    let mut row = EntryView {
        key: "",
        kind: "",
        offset: 0,
        length: 0,
        sha256: [0; 32],
        meta: None,
    };
    let mut seen = 0u8;
    for _ in 0..fields {
        let field = checked(d.str())?;
        let bit = match field {
            "key" => 1,
            "kind" => 2,
            "offset" => 4,
            "length" => 8,
            "sha256" => 16,
            "meta" => 32,
            _ => return Err(IndexError::Field),
        };
        if seen & bit != 0 {
            return Err(IndexError::DuplicateField);
        }
        seen |= bit;
        match bit {
            1 | 2 => {
                let value = checked(d.str())?;
                let cap = if bit == 1 { l.key_bytes } else { l.kind_bytes };
                if value.is_empty() || value.len() > cap {
                    return Err(IndexError::Text);
                }
                if bit == 1 {
                    row.key = value;
                } else {
                    row.kind = value;
                }
            }
            4 => row.offset = checked(d.u64())?,
            8 => row.length = checked(d.u64())?,
            16 => {
                let digest = byte_field(d, 32).map_err(|_| IndexError::Digest)?;
                if digest.len() != 32 {
                    return Err(IndexError::Digest);
                }
                digest.copy_into(&mut row.sha256)?;
            }
            32 => {
                if checked(d.datatype())? == Type::Null {
                    checked(d.null())?;
                } else {
                    row.meta = Some(byte_field(d, l.meta_bytes)?);
                }
            }
            _ => return Err(IndexError::Field),
        }
    }
    if seen & 31 != 31 {
        return Err(IndexError::MissingField);
    }
    Ok(row)
}

pub struct EntryIter<'a> {
    decoder: Decoder<'a>,
    remaining: usize,
    limits: IndexReadLimits,
}
impl<'a> EntryIter<'a> {
    pub(super) fn new(bytes: &'a [u8], count: usize, limits: IndexReadLimits) -> Self {
        let mut decoder = Decoder::new(bytes);
        // The immutable directory was already admitted; iteration still propagates decode errors.
        let _ = decoder.array();
        Self {
            decoder,
            remaining: count,
            limits,
        }
    }
}
impl<'a> Iterator for EntryIter<'a> {
    type Item = Result<EntryView<'a>, IndexError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let row = decode_row(&mut self.decoder, self.limits);
        if row.is_err() {
            self.remaining = 0;
        }
        Some(row)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.remaining))
    }
}
