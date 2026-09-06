//! Deterministic binary codec for [`Q42AssetEnvelope`].

use super::envelope::{
    AssetRoutingLane, AssetSensitivity, ChunkSpec, Q42AssetEnvelope, RecordCounts,
    ToolchainVersions, UpstreamRelease, ASSET_ENVELOPE_VERSION, MAX_ENVELOPE_BYTES,
};
use super::error::AssetEnvelopeError;
use super::licence::{
    LicenceClass, LicenceObligations, LicencePolicy, RedistributionClass, UseClass,
};

pub const ASSET_ENVELOPE_MAGIC: [u8; 8] = *b"Q42AST\0\0";

fn write_u16(buf: &mut Vec<u8>, value: u16) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(buf: &mut Vec<u8>, value: u32) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_u64(buf: &mut Vec<u8>, value: u64) {
    buf.extend_from_slice(&value.to_le_bytes());
}

fn write_str(buf: &mut Vec<u8>, value: &str) -> Result<(), AssetEnvelopeError> {
    let bytes = value.as_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| AssetEnvelopeError::Oversize)?;
    write_u16(buf, len);
    buf.extend_from_slice(bytes);
    Ok(())
}

fn write_str_list(buf: &mut Vec<u8>, values: &[String]) -> Result<(), AssetEnvelopeError> {
    let len = u16::try_from(values.len()).map_err(|_| AssetEnvelopeError::Oversize)?;
    write_u16(buf, len);
    for value in values {
        write_str(buf, value)?;
    }
    Ok(())
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, AssetEnvelopeError> {
    if *cursor + 2 > bytes.len() {
        return Err(AssetEnvelopeError::Truncated);
    }
    let value = u16::from_le_bytes(bytes[*cursor..*cursor + 2].try_into().unwrap());
    *cursor += 2;
    Ok(value)
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, AssetEnvelopeError> {
    if *cursor + 4 > bytes.len() {
        return Err(AssetEnvelopeError::Truncated);
    }
    let value = u32::from_le_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap());
    *cursor += 4;
    Ok(value)
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, AssetEnvelopeError> {
    if *cursor + 8 > bytes.len() {
        return Err(AssetEnvelopeError::Truncated);
    }
    let value = u64::from_le_bytes(bytes[*cursor..*cursor + 8].try_into().unwrap());
    *cursor += 8;
    Ok(value)
}

fn read_bytes32(bytes: &[u8], cursor: &mut usize) -> Result<[u8; 32], AssetEnvelopeError> {
    if *cursor + 32 > bytes.len() {
        return Err(AssetEnvelopeError::Truncated);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes[*cursor..*cursor + 32]);
    *cursor += 32;
    Ok(out)
}

fn read_str(bytes: &[u8], cursor: &mut usize) -> Result<String, AssetEnvelopeError> {
    let len = read_u16(bytes, cursor)? as usize;
    if *cursor + len > bytes.len() {
        return Err(AssetEnvelopeError::Truncated);
    }
    let slice = &bytes[*cursor..*cursor + len];
    *cursor += len;
    std::str::from_utf8(slice)
        .map(|s| s.to_string())
        .map_err(|_| AssetEnvelopeError::InvalidUtf8)
}

fn read_str_list(bytes: &[u8], cursor: &mut usize) -> Result<Vec<String>, AssetEnvelopeError> {
    let count = read_u16(bytes, cursor)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(read_str(bytes, cursor)?);
    }
    Ok(out)
}

impl Q42AssetEnvelope {
    /// Deterministic little-endian encode. Fails if validation or size budget fails.
    pub fn encode(&self) -> Result<Vec<u8>, AssetEnvelopeError> {
        self.validate()?;
        let mut buf = Vec::with_capacity(512);
        buf.extend_from_slice(&ASSET_ENVELOPE_MAGIC);
        write_u16(&mut buf, ASSET_ENVELOPE_VERSION);
        write_u16(&mut buf, 0); // reserved
        write_str(&mut buf, &self.asset_id)?;
        write_str(&mut buf, &self.upstream.source_name)?;
        write_str(&mut buf, &self.upstream.release_id)?;
        write_str(&mut buf, &self.upstream.source_url)?;
        write_u64(&mut buf, self.upstream.retrieved_unix);
        write_u64(&mut buf, self.upstream.byte_length);
        buf.extend_from_slice(&self.upstream.sha256);
        buf.push(self.licence.class as u8);
        buf.push(self.licence.use_class as u8);
        buf.push(self.licence.redistribution as u8);
        buf.push(0); // pad
        write_u16(&mut buf, self.licence.obligations.0);
        write_str(&mut buf, &self.licence.terms_url)?;
        write_str(&mut buf, &self.licence.attribution)?;
        write_str(&mut buf, &self.toolchain.parser_version)?;
        write_str(&mut buf, &self.toolchain.mapping_version)?;
        write_str(&mut buf, &self.raw_format)?;
        write_str(&mut buf, &self.media_type)?;
        write_u64(&mut buf, self.counts.source);
        write_u64(&mut buf, self.counts.accepted);
        write_u64(&mut buf, self.counts.quarantined);
        write_str_list(&mut buf, &self.rejection_reasons)?;
        write_str_list(&mut buf, &self.identifier_namespaces)?;
        write_str(&mut buf, &self.cross_reference_policy)?;
        write_str(&mut buf, &self.evidence_grade)?;
        write_str(&mut buf, &self.citation)?;
        write_str(&mut buf, &self.curation_status)?;
        buf.push(self.sensitivity as u8);
        buf.push(self.routing_lane as u8);
        write_u16(&mut buf, 0); // pad
        write_str_list(&mut buf, &self.derived_from)?;
        write_str(&mut buf, &self.shacl_profile)?;
        write_str(&mut buf, &self.validation_report)?;
        let chunk_len = u16::try_from(self.chunk_plan.len()).map_err(|_| AssetEnvelopeError::Oversize)?;
        write_u16(&mut buf, chunk_len);
        for chunk in &self.chunk_plan {
            write_u32(&mut buf, chunk.index);
            write_u64(&mut buf, chunk.byte_budget);
            write_u64(&mut buf, chunk.record_budget);
        }
        if buf.len() > MAX_ENVELOPE_BYTES {
            return Err(AssetEnvelopeError::Oversize);
        }
        Ok(buf)
    }

    /// Decode and re-validate.
    pub fn decode(bytes: &[u8]) -> Result<Self, AssetEnvelopeError> {
        if bytes.len() < 12 {
            return Err(AssetEnvelopeError::Truncated);
        }
        if bytes.len() > MAX_ENVELOPE_BYTES {
            return Err(AssetEnvelopeError::Oversize);
        }
        if bytes[0..8] != ASSET_ENVELOPE_MAGIC {
            return Err(AssetEnvelopeError::InvalidMagic);
        }
        let mut cursor = 8;
        let version = read_u16(bytes, &mut cursor)?;
        if version != ASSET_ENVELOPE_VERSION {
            return Err(AssetEnvelopeError::UnsupportedVersion);
        }
        let _reserved = read_u16(bytes, &mut cursor)?;
        let asset_id = read_str(bytes, &mut cursor)?;
        let upstream = UpstreamRelease {
            source_name: read_str(bytes, &mut cursor)?,
            release_id: read_str(bytes, &mut cursor)?,
            source_url: read_str(bytes, &mut cursor)?,
            retrieved_unix: read_u64(bytes, &mut cursor)?,
            byte_length: read_u64(bytes, &mut cursor)?,
            sha256: read_bytes32(bytes, &mut cursor)?,
        };
        if cursor + 4 > bytes.len() {
            return Err(AssetEnvelopeError::Truncated);
        }
        let class = LicenceClass::from_u8(bytes[cursor]);
        let use_class = UseClass::from_u8(bytes[cursor + 1]);
        let redistribution = RedistributionClass::from_u8(bytes[cursor + 2]);
        cursor += 4;
        let obligations = LicenceObligations(read_u16(bytes, &mut cursor)?);
        let terms_url = read_str(bytes, &mut cursor)?;
        let attribution = read_str(bytes, &mut cursor)?;
        // Reconstruct via try_new so Unknown fails closed even if wire was tampered.
        let mut licence = LicencePolicy::try_new(
            class,
            use_class,
            redistribution,
            terms_url,
            attribution,
        )?;
        // Preserve exact obligation bitfield from the wire (union may have added flags).
        licence.obligations = obligations;
        let toolchain = ToolchainVersions {
            parser_version: read_str(bytes, &mut cursor)?,
            mapping_version: read_str(bytes, &mut cursor)?,
        };
        let raw_format = read_str(bytes, &mut cursor)?;
        let media_type = read_str(bytes, &mut cursor)?;
        let counts = RecordCounts {
            source: read_u64(bytes, &mut cursor)?,
            accepted: read_u64(bytes, &mut cursor)?,
            quarantined: read_u64(bytes, &mut cursor)?,
        };
        let rejection_reasons = read_str_list(bytes, &mut cursor)?;
        let identifier_namespaces = read_str_list(bytes, &mut cursor)?;
        let cross_reference_policy = read_str(bytes, &mut cursor)?;
        let evidence_grade = read_str(bytes, &mut cursor)?;
        let citation = read_str(bytes, &mut cursor)?;
        let curation_status = read_str(bytes, &mut cursor)?;
        if cursor + 4 > bytes.len() {
            return Err(AssetEnvelopeError::Truncated);
        }
        let sensitivity = AssetSensitivity::from_u8(bytes[cursor])?;
        let routing_lane = AssetRoutingLane::from_u8(bytes[cursor + 1])?;
        cursor += 4;
        let derived_from = read_str_list(bytes, &mut cursor)?;
        let shacl_profile = read_str(bytes, &mut cursor)?;
        let validation_report = read_str(bytes, &mut cursor)?;
        let chunk_count = read_u16(bytes, &mut cursor)? as usize;
        let mut chunk_plan = Vec::with_capacity(chunk_count);
        for _ in 0..chunk_count {
            chunk_plan.push(ChunkSpec {
                index: read_u32(bytes, &mut cursor)?,
                byte_budget: read_u64(bytes, &mut cursor)?,
                record_budget: read_u64(bytes, &mut cursor)?,
            });
        }
        if cursor != bytes.len() {
            return Err(AssetEnvelopeError::Truncated);
        }
        let envelope = Self {
            asset_id,
            upstream,
            licence,
            toolchain,
            raw_format,
            media_type,
            counts,
            rejection_reasons,
            identifier_namespaces,
            cross_reference_policy,
            evidence_grade,
            citation,
            curation_status,
            sensitivity,
            routing_lane,
            derived_from,
            shacl_profile,
            validation_report,
            chunk_plan,
        };
        envelope.validate()?;
        Ok(envelope)
    }

    /// Deterministic digest of the encoded envelope bytes (not the payload).
    pub fn envelope_digest(&self) -> Result<[u8; 32], AssetEnvelopeError> {
        let encoded = self.encode()?;
        Ok(super::envelope::sha256_of(&encoded))
    }
}
