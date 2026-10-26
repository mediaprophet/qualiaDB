//! Deterministic W3C CBOR-LD (`0xD9 0x0501`) binary codec for VibeScript [`Value`].

use std::collections::BTreeMap;

use super::super::codec::{CborDecoder, CborEncoder};
use super::super::DecodeError;
use super::terms::*;
use crate::value::{Duration, Instant, Quantity, QuinRef, TimeScale, Value};

/// Encodes a runtime [`Value`] into deterministic compressed W3C CBOR-LD (`0xD9 0x0501`).
pub fn encode_value_cbor_ld(value: &Value, context_iri: &str) -> Vec<u8> {
    let mut enc = CborEncoder::new();
    enc.tag(TAG_CBOR_LD_COMPRESSED);
    enc.map(2);
    enc.uint(1); // 1 = "@context"
    write_term_or_str(&mut enc, context_iri);
    enc.uint(5); // 5 = "@graph" (payload)
    encode_val(&mut enc, value);
    enc.finish()
}

/// Decodes compressed (`0x0501`) or uncompressed (`0x0500`) CBOR-LD bytes into `(context_iri, Value)`.
pub fn decode_value_cbor_ld(bytes: &[u8]) -> Result<(String, Value), DecodeError> {
    let mut dec = CborDecoder::new(bytes);
    let tag = dec.tag()?;
    if tag != TAG_CBOR_LD_COMPRESSED && tag != TAG_CBOR_LD_UNCOMPRESSED {
        return Err(DecodeError::UnexpectedTag(tag));
    }
    let map_len = dec.map()?;
    let mut context_iri = DEFAULT_CONTEXT_IRI.to_string();
    let mut payload: Option<Value> = None;
    for _ in 0..map_len {
        let key = read_term_or_str(&mut dec, &context_iri)?;
        match key.as_str() {
            "@context" => {
                context_iri = read_term_or_str(&mut dec, "")?;
            }
            "@graph" | "@value" => {
                payload = Some(decode_val(&mut dec, &context_iri)?);
            }
            _ => {
                dec.skip()?;
            }
        }
    }
    let value = payload.ok_or(DecodeError::MissingField("@graph"))?;
    Ok((context_iri, value))
}

fn write_f64(enc: &mut CborEncoder, v: f64) {
    enc.write_u8(0xFB); // major 7, ai 27 (IEEE 754 binary64)
    for b in v.to_bits().to_be_bytes() {
        enc.write_u8(b);
    }
}

fn read_f64(dec: &mut CborDecoder<'_>) -> Result<f64, DecodeError> {
    let b = dec.read_u8()?;
    if b != 0xFB {
        return Err(DecodeError::UnexpectedType("expected f64 (0xFB)"));
    }
    let mut arr = [0u8; 8];
    for slot in &mut arr {
        *slot = dec.read_u8()?;
    }
    Ok(f64::from_bits(u64::from_be_bytes(arr)))
}

fn write_bytes(enc: &mut CborEncoder, bytes: &[u8]) {
    enc.write_type_and_len(2, bytes.len() as u64);
    for &b in bytes {
        enc.write_u8(b);
    }
}

fn read_bytes(dec: &mut CborDecoder<'_>) -> Result<Vec<u8>, DecodeError> {
    let (major, len) = dec.read_type_and_len()?;
    if major != 2 {
        return Err(DecodeError::UnexpectedType("expected byte string"));
    }
    let mut out = Vec::with_capacity(len as usize);
    for _ in 0..len {
        out.push(dec.read_u8()?);
    }
    Ok(out)
}

fn write_term_or_str(enc: &mut CborEncoder, s: &str) {
    if let Some(id) = lookup_static_term_id(s) {
        enc.uint(id);
    } else {
        enc.str(s);
    }
}

fn read_term_or_str(dec: &mut CborDecoder<'_>, context_iri: &str) -> Result<String, DecodeError> {
    let next = dec.peek()?;
    if (next >> 5) == 0 {
        let id = dec.uint()?;
        expand_term(id, context_iri)
            .ok_or_else(|| DecodeError::InvalidCbor(format!("unknown term id {id}")))
    } else {
        dec.str()
    }
}

fn encode_val(enc: &mut CborEncoder, val: &Value) {
    match val {
        Value::Null => enc.null(),
        Value::Bool(b) => enc.bool(*b),
        Value::I64(n) => enc.int(*n),
        Value::U64(n) => {
            enc.tag(TAG_VIBE_U64);
            enc.uint(*n);
        }
        Value::F64(f) => write_f64(enc, *f),
        Value::String(s) => enc.str(s),
        Value::Iri(iri) => {
            enc.tag(TAG_VIBE_IRI);
            write_term_or_str(enc, iri);
        }
        Value::Prefixed(prefix, local) => {
            enc.tag(TAG_VIBE_PREFIXED);
            enc.array(2);
            enc.str(prefix);
            enc.str(local);
        }
        Value::Blank(b) => {
            enc.tag(TAG_VIBE_BLANK);
            enc.str(b);
        }
        Value::Var(v) => {
            enc.tag(TAG_VIBE_VAR);
            enc.str(v);
        }
        Value::List(items) => {
            enc.array(items.len() as u64);
            for item in items {
                encode_val(enc, item);
            }
        }
        Value::Record(map) => {
            // RFC 8949 §4.2 deterministic CBOR: sort keys by encoded key bytes.
            let mut encoded_entries: Vec<(Vec<u8>, &Value)> = Vec::with_capacity(map.len());
            for (k, v) in map {
                let mut k_enc = CborEncoder::new();
                write_term_or_str(&mut k_enc, k);
                encoded_entries.push((k_enc.finish(), v));
            }
            encoded_entries.sort_by(|a, b| a.0.cmp(&b.0));
            enc.map(encoded_entries.len() as u64);
            for (k_bytes, v) in encoded_entries {
                for b in k_bytes {
                    enc.write_u8(b);
                }
                encode_val(enc, v);
            }
        }
        Value::Quantity(q) => {
            enc.tag(TAG_VIBE_QUANTITY);
            enc.array(2);
            write_f64(enc, q.value);
            write_term_or_str(enc, &q.unit);
        }
        Value::Instant(inst) => {
            enc.tag(TAG_VIBE_INSTANT);
            enc.array(5);
            let scale_id = match inst.scale {
                TimeScale::Unix => 0,
                TimeScale::Tai => 1,
                TimeScale::Gps => 2,
                TimeScale::Monotonic => 3,
                TimeScale::Proper => 4,
            };
            enc.uint(scale_id);
            enc.int(inst.secs);
            enc.uint(inst.nanos as u64);
            match &inst.frame {
                Some(f) => enc.str(f),
                None => enc.null(),
            }
            match &inst.seal {
                Some(sig) => write_bytes(enc, sig),
                None => enc.null(),
            }
        }
        Value::Duration(dur) => {
            enc.tag(TAG_VIBE_DURATION);
            enc.array(2);
            enc.int(dur.secs);
            enc.uint(dur.nanos as u64);
        }
        Value::QuinRef(q) => {
            enc.tag(TAG_VIBE_QUIN_REF);
            let mut raw = [0u8; 48];
            for (i, word) in q.raw_fields().iter().enumerate() {
                raw[i * 8..(i + 1) * 8].copy_from_slice(&word.to_be_bytes());
            }
            write_bytes(enc, &raw);
        }
        Value::DisclosureDenied { capability, reason } => {
            enc.tag(TAG_VIBE_DISCLOSURE_DENIED);
            enc.array(2);
            enc.str(capability);
            enc.str(reason);
        }
        Value::Receipt => {
            enc.tag(TAG_VIBE_RECEIPT);
            enc.null();
        }
        Value::Ok(inner) => {
            enc.tag(TAG_VIBE_OK);
            encode_val(enc, inner);
        }
        Value::Err(inner) => {
            enc.tag(TAG_VIBE_ERR);
            encode_val(enc, inner);
        }
        other => enc.str(&other.to_string()),
    }
}

fn decode_val(dec: &mut CborDecoder<'_>, context_iri: &str) -> Result<Value, DecodeError> {
    let peek = dec.peek()?;
    let major = peek >> 5;
    match major {
        0 | 1 => Ok(Value::I64(dec.int()?)),
        3 => Ok(Value::String(dec.str()?)),
        4 => {
            let len = dec.array()?;
            let mut items = Vec::with_capacity(len as usize);
            for _ in 0..len {
                items.push(decode_val(dec, context_iri)?);
            }
            Ok(Value::List(items))
        }
        5 => {
            let len = dec.map()?;
            let mut map = BTreeMap::new();
            for _ in 0..len {
                let k = read_term_or_str(dec, context_iri)?;
                let v = decode_val(dec, context_iri)?;
                map.insert(k, v);
            }
            Ok(Value::Record(map))
        }
        6 => decode_tagged_val(dec, context_iri),
        7 => match peek {
            0xF4 | 0xF5 => Ok(Value::Bool(dec.bool()?)),
            0xF6 => {
                dec.null()?;
                Ok(Value::Null)
            }
            0xFB => Ok(Value::F64(read_f64(dec)?)),
            _ => Err(DecodeError::UnexpectedType("unsupported major 7 value")),
        },
        _ => Err(DecodeError::UnexpectedType("unsupported CBOR major type")),
    }
}

fn decode_tagged_val(dec: &mut CborDecoder<'_>, context_iri: &str) -> Result<Value, DecodeError> {
    let tag = dec.tag()?;
    match tag {
        TAG_VIBE_U64 => Ok(Value::U64(dec.uint()?)),
        TAG_VIBE_IRI => Ok(Value::Iri(read_term_or_str(dec, context_iri)?)),
        TAG_VIBE_PREFIXED => {
            if dec.array()? != 2 {
                return Err(DecodeError::InvalidCbor("prefixed requires len 2".into()));
            }
            Ok(Value::Prefixed(dec.str()?, dec.str()?))
        }
        TAG_VIBE_BLANK => Ok(Value::Blank(dec.str()?)),
        TAG_VIBE_VAR => Ok(Value::Var(dec.str()?)),
        TAG_VIBE_QUANTITY => {
            if dec.array()? != 2 {
                return Err(DecodeError::InvalidCbor("quantity requires len 2".into()));
            }
            let value = read_f64(dec)?;
            let unit = read_term_or_str(dec, context_iri)?;
            Ok(Value::Quantity(Quantity { value, unit }))
        }
        TAG_VIBE_INSTANT => {
            if dec.array()? != 5 {
                return Err(DecodeError::InvalidCbor("instant requires len 5".into()));
            }
            let scale = match dec.uint()? {
                0 => TimeScale::Unix,
                1 => TimeScale::Tai,
                2 => TimeScale::Gps,
                3 => TimeScale::Monotonic,
                4 => TimeScale::Proper,
                s => return Err(DecodeError::InvalidCbor(format!("invalid TimeScale {s}"))),
            };
            let secs = dec.int()?;
            let nanos = dec.uint()? as u32;
            let frame = if dec.is_null()? {
                dec.null()?;
                None
            } else {
                Some(dec.str()?)
            };
            let seal = if dec.is_null()? {
                dec.null()?;
                None
            } else {
                let raw = read_bytes(dec)?;
                let arr: [u8; 64] = raw
                    .try_into()
                    .map_err(|_| DecodeError::InvalidCbor("seal must be 64 bytes".into()))?;
                Some(arr)
            };
            Ok(Value::Instant(Instant {
                scale,
                secs,
                nanos,
                frame,
                seal,
            }))
        }
        TAG_VIBE_DURATION => {
            if dec.array()? != 2 {
                return Err(DecodeError::InvalidCbor("duration requires len 2".into()));
            }
            Ok(Value::Duration(Duration {
                secs: dec.int()?,
                nanos: dec.uint()? as u32,
            }))
        }
        TAG_VIBE_QUIN_REF => {
            let raw = read_bytes(dec)?;
            if raw.len() != 48 {
                return Err(DecodeError::InvalidCbor("QuinRef must be 48 bytes".into()));
            }
            let mut words = [0u64; 6];
            for (i, w) in words.iter_mut().enumerate() {
                let mut b = [0u8; 8];
                b.copy_from_slice(&raw[i * 8..(i + 1) * 8]);
                *w = u64::from_be_bytes(b);
            }
            Ok(Value::QuinRef(QuinRef::from_raw(
                words[0], words[1], words[2], words[3], words[4], words[5],
            )))
        }
        TAG_VIBE_DISCLOSURE_DENIED => {
            if dec.array()? != 2 {
                return Err(DecodeError::InvalidCbor(
                    "DisclosureDenied requires len 2".into(),
                ));
            }
            Ok(Value::DisclosureDenied {
                capability: dec.str()?,
                reason: dec.str()?,
            })
        }
        TAG_VIBE_RECEIPT => {
            dec.null()?;
            Ok(Value::Receipt)
        }
        TAG_VIBE_OK => Ok(Value::Ok(Box::new(decode_val(dec, context_iri)?))),
        TAG_VIBE_ERR => Ok(Value::Err(Box::new(decode_val(dec, context_iri)?))),
        other => Err(DecodeError::UnexpectedTag(other)),
    }
}
