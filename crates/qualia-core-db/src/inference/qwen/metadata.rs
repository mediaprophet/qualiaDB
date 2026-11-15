//! Allocation-free preflight before legacy cold metadata deserializers.
use minicbor::{data::Type, Decoder};
use super::native_asset::{Qwen4ExpNativeDescriptor, Qwen4ExpNativeAssetError};

fn charge(total: &mut usize, count: usize, width: usize, budget: usize) -> Result<(), String> {
    *total = count.checked_mul(width).and_then(|n| total.checked_add(n))
        .filter(|n| *n <= budget).ok_or("Qwen decoded metadata budget exceeded")?;
    Ok(())
}

fn cbor(data: &[u8], budget: usize) -> Result<(), String> {
    let mut d = Decoder::new(data);
    let mut pending = [0u64; 128];
    pending[0] = 1;
    let mut depth = 0usize;
    let mut decoded = 0usize;
    loop {
        if pending[depth] == 0 {
            if depth == 0 { break; }
            depth -= 1;
            continue;
        }
        let kind = d.datatype().map_err(|_| "Invalid Qwen metadata CBOR")?;
        if pending[depth] == u64::MAX && kind == Type::Break {
            d.set_position(d.position() + 1);
            pending[depth] = 0;
            continue;
        }
        if pending[depth] == u64::MAX { charge(&mut decoded, 1, 128, budget)?; }
        else { pending[depth] -= 1; }
        let child = match kind {
            Type::Array | Type::Map | Type::ArrayIndef | Type::MapIndef => {
                let map = matches!(kind, Type::Map | Type::MapIndef);
                let count = if map { d.map() } else { d.array() }
                    .map_err(|_| "Invalid Qwen metadata container")?;
                if let Some(count) = count {
                    let count = if map { count.checked_mul(2).ok_or("Qwen CBOR count overflow")? } else { count };
                    charge(&mut decoded, usize::try_from(count).map_err(|_| "Qwen CBOR count overflow")?, 128, budget)?;
                    Some(count)
                } else { Some(u64::MAX) }
            }
            Type::Tag => { d.tag().map_err(|_| "Invalid Qwen CBOR tag")?; Some(1) }
            Type::String | Type::StringIndef => {
                for text in d.str_iter().map_err(|_| "Invalid Qwen CBOR string")? {
                    charge(&mut decoded, text.map_err(|_| "Invalid Qwen string chunk")?.len(), 8, budget)?;
                }
                None
            }
            Type::Bytes | Type::BytesIndef => {
                for value in d.bytes_iter().map_err(|_| "Invalid Qwen CBOR bytes")? {
                    charge(&mut decoded, value.map_err(|_| "Invalid Qwen bytes chunk")?.len(), 8, budget)?;
                }
                None
            }
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::I8 | Type::I16
            | Type::I32 | Type::I64 | Type::Int | Type::F16 | Type::F32 | Type::F64
            | Type::Bool | Type::Null | Type::Undefined | Type::Simple => {
                d.skip().map_err(|_| "Invalid Qwen CBOR scalar")?; None
            }
            _ => return Err("Invalid Qwen metadata encoding".into()),
        };
        if let Some(count) = child {
            if depth + 1 == pending.len() { return Err("Qwen metadata depth budget exceeded".into()); }
            depth += 1;
            pending[depth] = count;
        }
    }
    Ok(())
}

pub(super) fn descriptor(bytes: &[u8], budget: usize) -> Result<Qwen4ExpNativeDescriptor, String> {
    use crate::bundle::BundleReader;
    if bytes.len() < 64 || bytes.len() > budget { return Err("Qwen package byte budget exceeded".into()); }
    let start = usize::try_from(u64::from_le_bytes(bytes[12..20].try_into().expect("fixed word")))
        .map_err(|_| "Qwen index offset overflow")?;
    let len = usize::try_from(u64::from_le_bytes(bytes[20..28].try_into().expect("fixed word")))
        .map_err(|_| "Qwen index length overflow")?;
    let end = start.checked_add(len).ok_or("Qwen index range overflow")?;
    let index = bytes.get(start..end).ok_or("Truncated Qwen bundle index")?;
    cbor(index, budget / 4)?;
    let reader = BundleReader::parse(bytes).map_err(|e| format!("Qwen package: {e:?}"))?;
    let data = reader.get("qwen4exp/placement.cbor").ok_or("Qwen descriptor missing")?;
    cbor(data, budget / 4)?;
    let descriptor = ciborium::from_reader(data).map_err(|_| "Invalid Qwen descriptor fields")?;
    super::native_asset::validate_qwen4exp_descriptor(descriptor)
        .map_err(|e: Qwen4ExpNativeAssetError| e.to_string())
}

struct Cursor<'a> { bytes: &'a [u8], position: usize }
impl Cursor<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.position.checked_add(n).ok_or("Qwen metadata offset overflow")?;
        let value = self.bytes.get(self.position..end).ok_or("Truncated Qwen metadata")?;
        self.position = end;
        Ok(value)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("fixed word")))
    }
    fn size(&mut self) -> Result<usize, String> {
        usize::try_from(u64::from_le_bytes(self.take(8)?.try_into().expect("fixed word")))
            .map_err(|_| "Qwen metadata size overflow".into())
    }
    fn string(&mut self, decoded: &mut usize, budget: usize) -> Result<(), String> {
        let n = self.size()?;
        self.take(n)?;
        charge(decoded, n, 16, budget)
    }
    fn value(&mut self, kind: u32, decoded: &mut usize, budget: usize) -> Result<(), String> {
        let mut kinds = [0u32; 128];
        let mut remaining = [0usize; 128];
        kinds[0] = kind;
        remaining[0] = 1;
        let mut depth = 0usize;
        loop {
            if remaining[depth] == 0 {
                if depth == 0 { return Ok(()); }
                depth -= 1;
                continue;
            }
            remaining[depth] -= 1;
            let kind = kinds[depth];
            if kind == 8 { self.string(decoded, budget)?; continue; }
            if kind == 9 {
                let element = self.u32()?;
                let count = self.size()?;
                charge(decoded, count, if element == 8 { 320 } else { 8 }, budget)?;
                if depth + 1 == remaining.len() { return Err("Qwen GGUF depth budget exceeded".into()); }
                depth += 1;
                kinds[depth] = element;
                remaining[depth] = count;
                continue;
            }
            let width = match kind { 0 | 1 | 7 => 1, 2 | 3 => 2, 4 | 5 | 6 => 4,
                10 | 11 | 12 => 8, _ => return Err("Unknown Qwen GGUF value type".into()) };
            self.take(width)?;
        }
    }
}

/// Reserves a conservative logical upper bound for directory/tokenizer storage,
/// including derived string copies/maps and transient raw merge strings. This
/// is an admission ledger, not an allocator/RSS guarantee for legacy parsers.
pub(super) fn gguf(bytes: &[u8], budget: usize) -> Result<(), String> {
    let mut c = Cursor { bytes, position: 0 };
    if c.take(4)? != b"GGUF" || c.u32()? < 2 {
        return Err("Unsupported Qwen GGUF header".into());
    }
    let tensors = c.size()?;
    let kv = c.size()?;
    let cap = budget / 2;
    let mut decoded = 256 * 512usize;
    charge(&mut decoded, tensors, 256, cap)?;
    charge(&mut decoded, kv, 32, cap)?;
    for _ in 0..kv {
        c.string(&mut decoded, cap)?;
        let kind = c.u32()?;
        c.value(kind, &mut decoded, cap)?;
    }
    for _ in 0..tensors {
        c.string(&mut decoded, cap)?;
        let dims = c.u32()? as usize;
        c.take(dims.checked_mul(8).ok_or("Qwen dimension size overflow")?)?;
        c.take(12)?;
    }
    Ok(())
}
