use minicbor::Decoder;
use qualia_core_db::QualiaQuin;
use std::hash::{Hash, Hasher};

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn hash_bytes(b: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    b.hash(&mut hasher);
    hasher.finish()
}

/// Parses a CBOR-LD stream and streams directly into the external sorter.
/// No massive allocations are made.
pub fn parse_cbor_ld_stream(
    bytes: &[u8],
    context_hash: u64,
    sorter: &mut super::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut decoder = Decoder::new(bytes);
    let mut count = 0;

    // Typically CBOR-LD is an array of objects or a single object.
    // Let's assume an array of objects for this implementation.

    // Check if it's an array
    if let Ok(Some(len)) = decoder.array() {
        for _ in 0..len {
            parse_cbor_object(&mut decoder, context_hash, sorter, &mut count)?;
        }
    } else {
        // Might be an indefinite array or a single object
        // For simplicity, let's just attempt to parse a single object
        parse_cbor_object(&mut decoder, context_hash, sorter, &mut count)?;
    }

    Ok(count)
}

fn parse_cbor_object(
    decoder: &mut Decoder,
    context_hash: u64,
    sorter: &mut super::external_sort::ExternalSorter,
    count: &mut u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read the map
    let map_len = decoder.map()?;
    let mut subject_hash = 0;

    // We might need a tiny buffer for properties since we need the @id first if it's not the first key.
    // But in a strict zero-alloc streaming pull parser, we can parse it in one pass if we enforce
    // @id is first, OR we can just use a temporary struct or Vec for the properties of ONE object.
    // Since it's ONE object, it's fine to store its properties temporarily (very small).

    let mut properties = Vec::new();

    // Iterate map entries
    let iter_count = map_len.unwrap_or(u64::MAX);
    let mut i = 0;
    while i < iter_count {
        if map_len.is_none() && decoder.datatype()? == minicbor::data::Type::Break {
            decoder.skip()?;
            break;
        }

        // Key
        let key_str = decoder.str()?;
        let is_id = key_str == "@id";
        let pred_hash = hash_str(key_str);

        // Value
        let dt = decoder.datatype()?;
        let obj_hash = match dt {
            minicbor::data::Type::String => hash_bytes(decoder.bytes()?),
            minicbor::data::Type::Bytes => hash_bytes(decoder.bytes()?),
            minicbor::data::Type::U8
            | minicbor::data::Type::U16
            | minicbor::data::Type::U32
            | minicbor::data::Type::U64 => {
                let val = decoder.u64()?;
                hash_bytes(&val.to_le_bytes())
            }
            minicbor::data::Type::I8
            | minicbor::data::Type::I16
            | minicbor::data::Type::I32
            | minicbor::data::Type::I64 => {
                let val = decoder.i64()?;
                hash_bytes(&val.to_le_bytes())
            }
            minicbor::data::Type::F32 => {
                let val = decoder.f32()?;
                hash_bytes(&val.to_le_bytes())
            }
            minicbor::data::Type::F64 => {
                let val = decoder.f64()?;
                hash_bytes(&val.to_le_bytes())
            }
            minicbor::data::Type::Bool => {
                let val = decoder.bool()?;
                hash_bytes(&[val as u8])
            }
            _ => {
                decoder.skip()?;
                0
            }
        };

        if is_id {
            subject_hash = obj_hash;
        } else if key_str != "@type" && key_str != "@context" {
            properties.push((pred_hash, obj_hash));
        }

        i += 1;
    }

    // If no @id was found, we generate a blank node ID
    if subject_hash == 0 {
        // Just use a random/blank node approach
        subject_hash = hash_str(&format!("blank_{}", *count));
    }

    // Emit all properties
    for (p, o) in properties {
        let quin = QualiaQuin {
            subject: subject_hash,
            predicate: p,
            object: o,
            context: context_hash,
            metadata: 0b10 << 61,
            parity: 0,
        };
        sorter.push(quin)?;
        *count += 1;
    }

    Ok(())
}
