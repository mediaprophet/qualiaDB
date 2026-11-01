//! Allocation-free preflight for the bounded, definite-length bundle index profile.
//! The legacy reader remains available for historical encodings. This profile
//! admits only known fields, definite containers and untagged scalar values.
use super::super::format::{BundleEntry, BundleError, BUNDLE_HEADER_SIZE};
use minicbor::{data::Type, Decoder};

/// Caller-owned cold admission limits. `decoded_index_bytes` is a conservative
/// logical reservation (four times retained index storage), not measured RSS or
/// an allocator/stack guarantee. Payload bytes continue to borrow the input.
#[derive(Clone, Copy, Debug)]
pub struct BundleReadLimits {
    pub file_bytes: usize,
    pub index_bytes: usize,
    pub entries: usize,
    pub key_bytes: usize,
    pub kind_bytes: usize,
    pub meta_bytes: usize,
    pub decoded_index_bytes: usize,
}
impl Default for BundleReadLimits {
    fn default() -> Self {
        Self { file_bytes: 128 * 1024 * 1024, index_bytes: 1024 * 1024,
            entries: 4096, key_bytes: 4096, kind_bytes: 64, meta_bytes: 64 * 1024,
            decoded_index_bytes: 8 * 1024 * 1024 }
    }
}

fn refused() -> BundleError {
    BundleError::Cbor("Bundle index exceeds bounded definite-field profile".into())
}
fn checked<T>(value: Result<T, minicbor::decode::Error>) -> Result<T, BundleError> {
    value.map_err(|_| refused())
}
fn reserve(total: &mut usize, bytes: usize, cap: usize) -> Result<(), BundleError> {
    *total = total.checked_add(bytes.checked_mul(4).ok_or_else(refused)?)
        .filter(|v| *v <= cap).ok_or_else(refused)?;
    Ok(())
}
fn byte_vector(d: &mut Decoder<'_>, cap: usize) -> Result<usize, BundleError> {
    match checked(d.datatype())? {
        Type::Bytes => {
            let value = checked(d.bytes())?;
            if value.len() > cap { return Err(refused()); }
            Ok(value.len())
        }
        Type::Array => {
            let count = checked(d.array())?.ok_or_else(refused)?;
            if count > cap as u64 { return Err(refused()); }
            for _ in 0..count { checked(d.u8())?; }
            usize::try_from(count).map_err(|_| refused())
        }
        _ => Err(refused()),
    }
}

pub(super) fn preflight(bytes: &[u8], limits: BundleReadLimits) -> Result<usize, BundleError> {
    if bytes.len() < BUNDLE_HEADER_SIZE { return Err(BundleError::TooShort); }
    if bytes.len() > limits.file_bytes { return Err(refused()); }
    let read = |offset: usize| -> u64 {
        u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("fixed header"))
    };
    let start = usize::try_from(read(12)).map_err(|_| refused())?;
    let length = usize::try_from(read(20)).map_err(|_| refused())?;
    let end = start.checked_add(length).ok_or_else(refused)?;
    let count = u32::from_le_bytes(bytes[8..12].try_into().expect("fixed header")) as usize;
    if start < BUNDLE_HEADER_SIZE || end != bytes.len() || length > limits.index_bytes
        || count > limits.entries || read(28) != bytes.len() as u64
        || bytes[6..8] != [0, 0] || bytes[40..64].iter().any(|v| *v != 0)
    { return Err(refused()); }
    let index = &bytes[start..end];
    let mut d = Decoder::new(index);
    let declared = checked(d.array())?.ok_or_else(refused)?;
    if declared != count as u64 { return Err(refused()); }
    let mut decoded = 0usize;
    reserve(&mut decoded, count.checked_mul(std::mem::size_of::<BundleEntry>())
        .ok_or_else(refused)?, limits.decoded_index_bytes)?;
    for _ in 0..count {
        let fields = checked(d.map())?.ok_or_else(refused)?;
        if !(5..=6).contains(&fields) { return Err(refused()); }
        let mut seen = 0u8;
        for _ in 0..fields {
            // Borrowed string decode performs no allocation, even on a bad size.
            let field = checked(d.str())?;
            let bit = match field {
                "key" => 1, "kind" => 2, "offset" => 4, "length" => 8,
                "sha256" => 16, "meta" => 32, _ => return Err(refused()),
            };
            if seen & bit != 0 { return Err(refused()); }
            seen |= bit;
            match field {
                "key" | "kind" => {
                    let value = checked(d.str())?;
                    let cap = if field == "key" { limits.key_bytes } else { limits.kind_bytes };
                    if value.is_empty() || value.len() > cap { return Err(refused()); }
                    reserve(&mut decoded, value.len(), limits.decoded_index_bytes)?;
                }
                "offset" | "length" => { checked(d.u64())?; }
                "sha256" => {
                    if byte_vector(&mut d, 32)? != 32 { return Err(refused()); }
                    reserve(&mut decoded, 32, limits.decoded_index_bytes)?;
                }
                "meta" => {
                    if checked(d.datatype())? == Type::Null { checked(d.null())?; }
                    else {
                        let size = byte_vector(&mut d, limits.meta_bytes)?;
                        reserve(&mut decoded, size, limits.decoded_index_bytes)?;
                    }
                }
                _ => unreachable!(),
            }
        }
        if seen & 31 != 31 { return Err(refused()); }
    }
    if d.position() != index.len() { return Err(refused()); }
    Ok(count)
}
