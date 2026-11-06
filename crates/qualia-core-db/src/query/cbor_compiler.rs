use crate::NQuin;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnsupportedMediaType, // HTTP 415 equivalent
    MalformedPayload,
    BufferOverflow,
}

/// The Strict Binary Gatekeeper for Qualia-DB network payloads.
/// Explicitly rejects text-based semantic payloads and enforces binary
/// CBOR-framed ingestion, with the intended semantic payload profile being
/// CBOR-LD.
pub fn ingest_network_payload(payload: &[u8]) -> Result<&[u8], ParseError> {
    if payload.is_empty() {
        return Err(ParseError::MalformedPayload);
    }

    // Read payload[0] to determine the encoding format.
    let first_byte = payload[0];

    // Reject common text-based semantic web formats:
    // b'{' (0x7B) -> JSON-LD / JSON
    // b'<' (0x3C) -> XML / RDF/XML
    // b'@' (0x40) -> Turtle / N3 prefixes
    if first_byte == b'{' || first_byte == b'<' || first_byte == b'@' {
        return Err(ParseError::UnsupportedMediaType);
    }

    // CBOR Map (0xA0 to 0xB7) or CBOR Array (0x80 to 0x97) headers
    // If it falls outside standard CBOR composite boundaries (for a root object), we reject it.
    let is_cbor_map = (0xA0..=0xB7).contains(&first_byte);
    let is_cbor_array = (0x80..=0x97).contains(&first_byte);
    let is_cbor_indefinite = first_byte == 0xBF || first_byte == 0x9F;

    if is_cbor_map || is_cbor_array || is_cbor_indefinite {
        Ok(payload)
    } else {
        Err(ParseError::UnsupportedMediaType)
    }
}

#[inline]
fn read_cbor_int(valid_payload: &[u8], cursor: &mut usize) -> Result<u64, ParseError> {
    if *cursor >= valid_payload.len() {
        return Err(ParseError::BufferOverflow);
    }
    let byte = valid_payload[*cursor];
    *cursor += 1;

    let major_type = byte >> 5;
    // Only accept unsigned integers (major type 0) for Lexicon tags
    if major_type != 0 {
        // In a full implementation, we'd handle tags and strings.
        // For the Strict Binary Dictionary, all URIs are pre-compressed to integers.
        return Ok(0);
    }

    let additional_info = byte & 0x1F;
    match additional_info {
        0..=23 => Ok(additional_info as u64),
        24 => {
            if *cursor + 1 > valid_payload.len() {
                return Err(ParseError::BufferOverflow);
            }
            let val = valid_payload[*cursor] as u64;
            *cursor += 1;
            Ok(val)
        }
        25 => {
            if *cursor + 2 > valid_payload.len() {
                return Err(ParseError::BufferOverflow);
            }
            let mut bytes = [0u8; 2];
            bytes.copy_from_slice(&valid_payload[*cursor..*cursor + 2]);
            *cursor += 2;
            Ok(u16::from_be_bytes(bytes) as u64)
        }
        26 => {
            if *cursor + 4 > valid_payload.len() {
                return Err(ParseError::BufferOverflow);
            }
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&valid_payload[*cursor..*cursor + 4]);
            *cursor += 4;
            Ok(u32::from_be_bytes(bytes) as u64)
        }
        27 => {
            if *cursor + 8 > valid_payload.len() {
                return Err(ParseError::BufferOverflow);
            }
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&valid_payload[*cursor..*cursor + 8]);
            *cursor += 8;
            Ok(u64::from_be_bytes(bytes))
        }
        _ => Err(ParseError::MalformedPayload),
    }
}

/// A lightweight, `no_std`-compatible loop to read variable-length integers from CBOR-LD,
/// returning both the decoded [`NQuin`] (with canonical 5-field XOR parity) and the exact
/// byte cursor consumed from `payload`.
///
/// Supports both 4-integer frames `[Subject, Predicate, Object, Context]` (defaulting
/// `metadata` to `0`) and 5-integer frames `[Subject, Predicate, Object, Context, Metadata]`
/// (preserving Core-1 N3Logic routing bits `0b10 << 61` and `FrameLayout` tags).
pub fn parse_cbor_ld_to_quin_sized(payload: &[u8]) -> Result<(NQuin, usize), ParseError> {
    // 1. Enforce the Strict Binary Gatekeeper
    let valid_payload = ingest_network_payload(payload)?;
    let first_byte = valid_payload[0];

    let mut cursor = 1; // Skip the root map/array byte

    let subject = read_cbor_int(valid_payload, &mut cursor)?;
    let predicate = read_cbor_int(valid_payload, &mut cursor)?;
    let object = read_cbor_int(valid_payload, &mut cursor)?;
    let context = read_cbor_int(valid_payload, &mut cursor)?;

    // Read optional 5th CBOR integer for `metadata` when present.
    let metadata = if (0x80..=0x97).contains(&first_byte) {
        let count = (first_byte & 0x1F) as usize;
        if count < 4 {
            return Err(ParseError::MalformedPayload);
        }
        if count >= 5 {
            let m = read_cbor_int(valid_payload, &mut cursor)?;
            for _ in 5..count {
                let _ = read_cbor_int(valid_payload, &mut cursor)?;
            }
            m
        } else {
            0b00 << 61
        }
    } else if (0xA0..=0xB7).contains(&first_byte) {
        let count = (first_byte & 0x1F) as usize;
        if count >= 5 {
            read_cbor_int(valid_payload, &mut cursor)?
        } else {
            0b00 << 61
        }
    } else {
        // Indefinite-length array (0x9F) or map (0xBF)
        let m = if cursor < valid_payload.len() && valid_payload[cursor] != 0xFF {
            read_cbor_int(valid_payload, &mut cursor)?
        } else {
            0b00 << 61
        };
        if cursor < valid_payload.len() && valid_payload[cursor] == 0xFF {
            cursor += 1;
        }
        m
    };

    let parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);

    Ok((
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        },
        cursor,
    ))
}

/// A lightweight, `no_std`-compatible loop to read variable-length integers from CBOR-LD.
/// Maps the extracted dictionary tags directly into the 64-bit Lexicon registers
/// without allocating a single `String` or `Vec`, and computes canonical 5-field XOR parity.
pub fn parse_cbor_ld_to_quin(payload: &[u8]) -> Result<NQuin, ParseError> {
    parse_cbor_ld_to_quin_sized(payload).map(|(quin, _)| quin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gatekeeper_rejects_json() {
        let json_payload = b"{\"@context\": \"http://schema.org\"}";
        assert_eq!(
            ingest_network_payload(json_payload),
            Err(ParseError::UnsupportedMediaType)
        );
    }

    #[test]
    fn test_gatekeeper_rejects_turtle() {
        let turtle_payload = b"@prefix qualia: <urn:qualia:> .";
        assert_eq!(
            ingest_network_payload(turtle_payload),
            Err(ParseError::UnsupportedMediaType)
        );
    }

    #[test]
    fn test_parse_cbor_ld_dictionary() {
        // CBOR Array of 4 integers: [1000, 2000, 3000, 4000]
        // Array header: 0x84
        // 1000 = 0x19 0x03 0xE8
        // 2000 = 0x19 0x07 0xD0
        // 3000 = 0x19 0x0B 0xB8
        // 4000 = 0x19 0x0F 0xA0
        let cbor_payload: [u8; 13] = [
            0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0,
        ];

        let (quin, consumed) = parse_cbor_ld_to_quin_sized(&cbor_payload).unwrap();

        assert_eq!(consumed, 13);
        assert_eq!(quin.subject, 1000);
        assert_eq!(quin.predicate, 2000);
        assert_eq!(quin.object, 3000);
        assert_eq!(quin.context, 4000);
        assert_eq!(quin.metadata, 0);
        assert_eq!(
            quin.parity,
            1000u64 ^ 2000u64 ^ 3000u64 ^ 4000u64 ^ 0u64,
            "must compute canonical 5-field XOR parity"
        );
        assert!(quin.verify_ecc_parity());
    }

    #[test]
    fn test_parse_cbor_ld_with_fifth_metadata_integer_and_parity() {
        // CBOR Array of 5 integers: [1000, 2000, 3000, 4000, metadata]
        // metadata = (0b10 << 61) | 42 = 0x4000_0000_0000_002A
        let meta: u64 = (0b10u64 << 61) | 42;
        let mut cbor_payload = [0u8; 22];
        cbor_payload[0] = 0x85; // 5-element array
        cbor_payload[1..13].copy_from_slice(&[
            0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0,
        ]);
        cbor_payload[13] = 27; // 8-byte uint follows
        cbor_payload[14..22].copy_from_slice(&meta.to_be_bytes());

        let (quin, consumed) = parse_cbor_ld_to_quin_sized(&cbor_payload).unwrap();
        assert_eq!(consumed, 22);
        assert_eq!(quin.metadata, meta);
        assert_eq!(
            quin.parity,
            NQuin::calculate_parity(1000, 2000, 3000, 4000, meta)
        );
        assert!(quin.verify_ecc_parity());
    }

    #[test]
    fn test_parse_cbor_buffer_overflow() {
        // Truncated array header
        let cbor_payload: [u8; 4] = [0x84, 0x19, 0x03, 0xE8];
        assert_eq!(
            parse_cbor_ld_to_quin(&cbor_payload),
            Err(ParseError::BufferOverflow)
        );
    }
}
