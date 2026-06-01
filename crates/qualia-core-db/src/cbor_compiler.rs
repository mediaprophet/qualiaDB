use crate::QualiaQuin;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    UnsupportedMediaType, // HTTP 415 equivalent
    MalformedPayload,
    BufferOverflow,
}

/// The Strict Binary Gatekeeper for Qualia-DB network payloads.
/// Explicitly rejects text-based semantic payloads and enforces CBOR/CBOR-LD binary ingestion.
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

/// A lightweight, no_std compatible loop to read variable-length integers from CBOR-LD.
/// Maps the extracted dictionary tags directly into the 64-bit Lexicon registers
/// without allocating a single String or Vec.
pub fn parse_cbor_ld_to_quin(payload: &[u8]) -> Result<QualiaQuin, ParseError> {
    // 1. Enforce the Strict Binary Gatekeeper
    let valid_payload = ingest_network_payload(payload)?;

    let mut cursor = 1; // Skip the root map/array byte
    
    // Helper closure to safely read variable-length CBOR integers
    let mut read_cbor_int = || -> Result<u64, ParseError> {
        if cursor >= valid_payload.len() {
            return Err(ParseError::BufferOverflow);
        }
        let byte = valid_payload[cursor];
        cursor += 1;

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
                if cursor + 1 > valid_payload.len() { return Err(ParseError::BufferOverflow); }
                let val = valid_payload[cursor] as u64;
                cursor += 1;
                Ok(val)
            },
            25 => {
                if cursor + 2 > valid_payload.len() { return Err(ParseError::BufferOverflow); }
                let mut bytes = [0u8; 2];
                bytes.copy_from_slice(&valid_payload[cursor..cursor+2]);
                cursor += 2;
                Ok(u16::from_be_bytes(bytes) as u64)
            },
            26 => {
                if cursor + 4 > valid_payload.len() { return Err(ParseError::BufferOverflow); }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&valid_payload[cursor..cursor+4]);
                cursor += 4;
                Ok(u32::from_be_bytes(bytes) as u64)
            },
            27 => {
                if cursor + 8 > valid_payload.len() { return Err(ParseError::BufferOverflow); }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&valid_payload[cursor..cursor+8]);
                cursor += 8;
                Ok(u64::from_be_bytes(bytes))
            },
            _ => Err(ParseError::MalformedPayload)
        }
    };

    // We assume the CBOR-LD payload is an array of 4 integers: [Subject, Predicate, Object, Context]
    // representing the dictionary-compressed Lexicon tags.
    let subject = read_cbor_int()?;
    let predicate = read_cbor_int()?;
    let object = read_cbor_int()?;
    let context = read_cbor_int()?;
    
    // Hardcode metadata to Passthrough for this base compilation layer
    let metadata = 0b00 << 61;

    Ok(QualiaQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: 0, // In production, ECC checksum calculated here
    })
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
            0x84, 
            0x19, 0x03, 0xE8, 
            0x19, 0x07, 0xD0, 
            0x19, 0x0B, 0xB8, 
            0x19, 0x0F, 0xA0
        ];

        let quin = parse_cbor_ld_to_quin(&cbor_payload).unwrap();
        
        assert_eq!(quin.subject, 1000);
        assert_eq!(quin.predicate, 2000);
        assert_eq!(quin.object, 3000);
        assert_eq!(quin.context, 4000);
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
