//! ADR 0014 `FLAG_PAYLOAD_CATALOGUE` (`0x0080`) section encoding and decoding for `.q42` volumes.
//!
//! Provides byte-exact binary and CBOR parity with [`crate::bundle::cbor_ld_manifest::PayloadCatalogueEntry`]
//! for multi-language prose definitions, SHACL shape pack descriptors, and provenance sidecars
//! stored inside `.q42` v4 volumes.
//!
//! Header reserved slot layout when `FLAG_PAYLOAD_CATALOGUE` (`0x0080`) is set:
//! - `_reserved[48..56]` = `payload_catalogue_offset` (`u64` little-endian)
//! - `_reserved[56..64]` = `payload_catalogue_length` (`u64` little-endian)

#![allow(dead_code)]

use sha2::{Digest, Sha256};

use crate::bundle::cbor_ld_manifest::PayloadCatalogueEntry;
use crate::container_10d::crc32c;
use crate::q42_volume::{Q42VolumeHeader, FLAG_PAYLOAD_CATALOGUE};

pub const PAYLOAD_CATALOGUE_MAGIC: [u8; 4] = *b"PCAT";
pub const PAYLOAD_CATALOGUE_VERSION: u32 = 1;
pub const PAYLOAD_CATALOGUE_HEADER_BYTES: usize = 16;

/// FourCC magic and kind tag (`"GTOP"`) for a POET Geometry-Topology Complex payload in `.q42`.
pub const GTOP_SECTION_MAGIC: [u8; 4] = *b"GTOP";
#[allow(dead_code)]
pub const PAYLOAD_KIND_GEOMETRY_TOPOLOGY: u32 = 0x4754_4F50;
pub const GTOP_SECTION_VERSION: u32 = 1;
pub const GTOP_SECTION_HEADER_BYTES: usize = 16;
pub const GTOP_CATALOGUE_ROLE: &str = "geometry_topology_complex";
pub const GTOP_MIME_TYPE: &str = "application/x-qualia-gtop";

/// A [`PayloadCatalogueEntry`] paired with its verified UTF-8 prose/descriptor payload body.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PayloadCatalogueBlob {
    pub entry: PayloadCatalogueEntry,
    pub utf8_body: String,
}

impl std::ops::Deref for PayloadCatalogueBlob {
    type Target = PayloadCatalogueEntry;

    fn deref(&self) -> &Self::Target {
        &self.entry
    }
}

impl PayloadCatalogueBlob {
    /// Construct a validated UTF-8 [`PayloadCatalogueBlob`], computing `byte_length` and
    /// `sha256_hex` and rejecting forbidden media/weight MIME types (`ADR 0014`).
    pub fn new_utf8(
        payload_key: impl Into<String>,
        mime_type: impl Into<String>,
        language: Option<String>,
        role: impl Into<String>,
        utf8_body: impl Into<String>,
    ) -> Result<Self, String> {
        let utf8_body = utf8_body.into();
        let sha256_hex = sha256_hex_bytes(utf8_body.as_bytes());
        let entry = PayloadCatalogueEntry {
            payload_key: payload_key.into(),
            mime_type: mime_type.into(),
            language,
            role: role.into(),
            byte_length: utf8_body.len() as u64,
            sha256_hex,
        };
        validate_payload_catalogue_entry(&entry)?;
        Ok(Self { entry, utf8_body })
    }

    /// Borrow the UTF-8 payload body.
    pub fn body_str(&self) -> &str {
        &self.utf8_body
    }
}

/// Compute lowercase 64-character SHA-256 hex digest of `bytes`.
pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut sha256_hex = String::with_capacity(64);
    for b in digest {
        use core::fmt::Write;
        let _ = write!(&mut sha256_hex, "{b:02x}");
    }
    sha256_hex
}

/// Enforce `ADR 0014` non-goals: reject media (`image/*`, `audio/*`, `video/*`) and binary
/// weight/manifold containers (`.p64`, `.10d`, `.gguf`, `.glb`, `.hmc`) from `PCAT`.
pub fn validate_payload_catalogue_entry(entry: &PayloadCatalogueEntry) -> Result<(), String> {
    if entry.payload_key.trim().is_empty() {
        return Err("payload catalogue entry has an empty payload_key".to_string());
    }
    let mime = entry.mime_type.trim().to_ascii_lowercase();
    if mime.starts_with("image/")
        || mime.starts_with("audio/")
        || mime.starts_with("video/")
        || mime.starts_with("model/")
    {
        return Err(format!(
            "ADR 0014 violation: media MIME type '{}' is forbidden in Q42 PCAT; use .hmc container instead",
            entry.mime_type
        ));
    }
    let key_lower = entry.payload_key.trim().to_ascii_lowercase();
    for forbidden_ext in [".p64", ".10d", ".gguf", ".glb", ".hmc", ".mp4", ".mp3", ".png", ".jpg", ".jpeg", ".wav"] {
        if key_lower.ends_with(forbidden_ext) || mime.contains(forbidden_ext) {
            return Err(format!(
                "ADR 0014 violation: binary container/media payload '{}' ('{}') is forbidden in Q42 PCAT; use .hmc / .p64 / .10d instead",
                entry.payload_key, entry.mime_type
            ));
        }
    }
    if entry.sha256_hex.len() != 64 || !entry.sha256_hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "payload catalogue entry '{}' has invalid sha256_hex '{}'",
            entry.payload_key, entry.sha256_hex
        ));
    }
    Ok(())
}

/// Encode a slice of [`PayloadCatalogueEntry`] records into a self-describing `PCAT` v1 section.
///
/// Layout:
/// - `[0..4]`: `"PCAT"` magic
/// - `[4..8]`: version (`1u32` LE)
/// - `[8..12]`: entry count (`u32` LE)
/// - `[12..16]`: CRC-32C of the CBOR payload (`u32` LE)
/// - `[16..]`: deterministic CBOR array of `PayloadCatalogueEntry`
pub fn encode_payload_catalogue_section(
    entries: &[PayloadCatalogueEntry],
) -> Result<Vec<u8>, String> {
    for entry in entries {
        validate_payload_catalogue_entry(entry)?;
    }
    let count = u32::try_from(entries.len())
        .map_err(|_| "payload catalogue entry count exceeds u32::MAX".to_string())?;
    let mut cbor_bytes = Vec::new();
    ciborium::into_writer(entries, &mut cbor_bytes)
        .map_err(|e| format!("failed to encode payload catalogue CBOR: {e}"))?;
    let crc = crc32c::crc32c(&cbor_bytes);

    let mut out = Vec::with_capacity(PAYLOAD_CATALOGUE_HEADER_BYTES + cbor_bytes.len());
    out.extend_from_slice(&PAYLOAD_CATALOGUE_MAGIC);
    out.extend_from_slice(&PAYLOAD_CATALOGUE_VERSION.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&cbor_bytes);
    Ok(out)
}

/// Encode [`PayloadCatalogueBlob`] records (CBOR descriptor array + length-prefixed UTF-8 payload
/// body arena) into a self-describing `PCAT` v1 section covered by a single CRC-32C checksum.
pub fn encode_payload_catalogue_with_bodies(
    blobs: &[PayloadCatalogueBlob],
) -> Result<Vec<u8>, String> {
    let count = u32::try_from(blobs.len())
        .map_err(|_| "payload catalogue entry count exceeds u32::MAX".to_string())?;
    let mut entries = Vec::with_capacity(blobs.len());
    let mut total_body_bytes = 0usize;
    for (idx, blob) in blobs.iter().enumerate() {
        validate_payload_catalogue_entry(&blob.entry)?;
        if blobs[..idx]
            .iter()
            .any(|prev| prev.entry.payload_key == blob.entry.payload_key)
        {
            return Err(format!(
                "duplicate payload_key '{}' in payload catalogue",
                blob.entry.payload_key
            ));
        }
        let body_bytes = blob.utf8_body.as_bytes();
        if body_bytes.len() as u64 != blob.entry.byte_length {
            return Err(format!(
                "payload catalogue entry '{}' byte_length mismatch: entry declares {}, body has {}",
                blob.entry.payload_key,
                blob.entry.byte_length,
                body_bytes.len()
            ));
        }
        let actual_sha = sha256_hex_bytes(body_bytes);
        if !actual_sha.eq_ignore_ascii_case(&blob.entry.sha256_hex) {
            return Err(format!(
                "payload catalogue entry '{}' SHA-256 mismatch: entry declares {}, body hashes to {}",
                blob.entry.payload_key, blob.entry.sha256_hex, actual_sha
            ));
        }
        total_body_bytes = total_body_bytes
            .checked_add(8 + body_bytes.len())
            .ok_or_else(|| "payload catalogue body arena length overflow".to_string())?;
        entries.push(blob.entry.clone());
    }

    let mut payload_bytes = Vec::new();
    ciborium::into_writer(&entries, &mut payload_bytes)
        .map_err(|e| format!("failed to encode payload catalogue CBOR: {e}"))?;
    payload_bytes.reserve(total_body_bytes);
    for blob in blobs {
        let body_bytes = blob.utf8_body.as_bytes();
        payload_bytes.extend_from_slice(&(body_bytes.len() as u64).to_le_bytes());
        payload_bytes.extend_from_slice(body_bytes);
    }

    let crc = crc32c::crc32c(&payload_bytes);
    let mut out = Vec::with_capacity(PAYLOAD_CATALOGUE_HEADER_BYTES + payload_bytes.len());
    out.extend_from_slice(&PAYLOAD_CATALOGUE_MAGIC);
    out.extend_from_slice(&PAYLOAD_CATALOGUE_VERSION.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(&payload_bytes);
    Ok(out)
}

/// Decode and CRC-32C-verify a `PCAT` v1 section back into [`PayloadCatalogueEntry`] records.
/// If a UTF-8 body arena follows the CBOR descriptor array, every body is also verified against
/// its entry's `byte_length` and `sha256_hex`.
pub fn decode_payload_catalogue_section(
    section_bytes: &[u8],
) -> Result<Vec<PayloadCatalogueEntry>, String> {
    let (entries, _) = decode_payload_catalogue_inner(section_bytes, false)?;
    Ok(entries)
}

/// Decode and verify a `PCAT` v1 section along with its UTF-8 payload body arena.
pub fn decode_payload_catalogue_with_bodies(
    section_bytes: &[u8],
) -> Result<Vec<PayloadCatalogueBlob>, String> {
    let (entries, bodies) = decode_payload_catalogue_inner(section_bytes, false)?;
    if let Some(bodies) = bodies {
        Ok(entries
            .into_iter()
            .zip(bodies)
            .map(|(entry, utf8_body)| PayloadCatalogueBlob { entry, utf8_body })
            .collect())
    } else {
        Ok(entries
            .into_iter()
            .map(|entry| PayloadCatalogueBlob {
                entry,
                utf8_body: String::new(),
            })
            .collect())
    }
}

fn decode_payload_catalogue_inner(
    section_bytes: &[u8],
    require_bodies: bool,
) -> Result<(Vec<PayloadCatalogueEntry>, Option<Vec<String>>), String> {
    if section_bytes.len() < PAYLOAD_CATALOGUE_HEADER_BYTES {
        return Err("payload catalogue section shorter than 16-byte header".to_string());
    }
    if &section_bytes[0..4] != &PAYLOAD_CATALOGUE_MAGIC {
        return Err("invalid payload catalogue magic (expected PCAT)".to_string());
    }
    let version = u32::from_le_bytes(section_bytes[4..8].try_into().unwrap());
    if version != PAYLOAD_CATALOGUE_VERSION {
        return Err(format!("unsupported payload catalogue version {version}"));
    }
    let expected_count = u32::from_le_bytes(section_bytes[8..12].try_into().unwrap()) as usize;
    let expected_crc = u32::from_le_bytes(section_bytes[12..16].try_into().unwrap());
    let payload = &section_bytes[PAYLOAD_CATALOGUE_HEADER_BYTES..];

    let actual_crc = crc32c::crc32c(payload);
    if actual_crc != expected_crc {
        return Err(format!(
            "payload catalogue CRC-32C mismatch: expected {expected_crc:#010x}, got {actual_crc:#010x}"
        ));
    }

    let mut cursor: &[u8] = payload;
    let entries: Vec<PayloadCatalogueEntry> = ciborium::from_reader(&mut cursor)
        .map_err(|e| format!("failed to decode payload catalogue CBOR: {e}"))?;
    if entries.len() != expected_count {
        return Err(format!(
            "payload catalogue entry count mismatch: header {expected_count} vs decoded {}",
            entries.len()
        ));
    }
    for entry in &entries {
        validate_payload_catalogue_entry(entry)?;
    }

    if cursor.is_empty() {
        if require_bodies && !entries.is_empty() {
            return Err("payload catalogue section is missing UTF-8 body arena".to_string());
        }
        return Ok((entries, None));
    }

    let mut bodies = Vec::with_capacity(entries.len());
    for entry in &entries {
        if cursor.len() < 8 {
            return Err(format!(
                "truncated payload catalogue body length prefix for '{}'",
                entry.payload_key
            ));
        }
        let body_len = u64::from_le_bytes(cursor[0..8].try_into().unwrap());
        cursor = &cursor[8..];
        if body_len != entry.byte_length {
            return Err(format!(
                "payload catalogue body length mismatch for '{}': descriptor {}, arena prefix {}",
                entry.payload_key, entry.byte_length, body_len
            ));
        }
        let body_len_usize = usize::try_from(body_len)
            .map_err(|_| "payload catalogue body exceeds usize".to_string())?;
        if cursor.len() < body_len_usize {
            return Err(format!(
                "truncated payload catalogue body for '{}': expected {} bytes, got {}",
                entry.payload_key,
                body_len_usize,
                cursor.len()
            ));
        }
        let body_bytes = &cursor[..body_len_usize];
        cursor = &cursor[body_len_usize..];
        let body_str = std::str::from_utf8(body_bytes).map_err(|e| {
            format!(
                "payload catalogue body for '{}' is not valid UTF-8: {e}",
                entry.payload_key
            )
        })?;
        let actual_sha = sha256_hex_bytes(body_bytes);
        if !actual_sha.eq_ignore_ascii_case(&entry.sha256_hex) {
            return Err(format!(
                "payload catalogue body SHA-256 mismatch for '{}': expected {}, got {}",
                entry.payload_key, entry.sha256_hex, actual_sha
            ));
        }
        bodies.push(body_str.to_string());
    }
    if !cursor.is_empty() {
        return Err(format!(
            "payload catalogue has {} trailing unindexed bytes after body arena",
            cursor.len()
        ));
    }
    Ok((entries, Some(bodies)))
}

/// Stamp `FLAG_PAYLOAD_CATALOGUE` (`0x0080`) and the `(offset, length)` pair into `header._reserved[48..64]`.
pub fn set_header_payload_catalogue_range(
    header: &mut Q42VolumeHeader,
    offset: u64,
    length: u64,
) {
    header.flags |= FLAG_PAYLOAD_CATALOGUE;
    header._reserved[48..56].copy_from_slice(&offset.to_le_bytes());
    header._reserved[56..64].copy_from_slice(&length.to_le_bytes());
}

/// Read the `(offset, length)` range for `FLAG_PAYLOAD_CATALOGUE` (`0x0080`) from `header._reserved[48..64]`.
pub fn header_payload_catalogue_range(header: &Q42VolumeHeader) -> Option<(u64, u64)> {
    let flags = { header.flags };
    if flags & FLAG_PAYLOAD_CATALOGUE == 0 {
        return None;
    }
    let reserved = header._reserved;
    let offset = u64::from_le_bytes(reserved[48..56].try_into().unwrap());
    let length = u64::from_le_bytes(reserved[56..64].try_into().unwrap());
    if length == 0 {
        return None;
    }
    Some((offset, length))
}

/// Construct a deterministic [`PayloadCatalogueEntry`] describing a `GTOP` geometry-topology segment.
pub fn encode_gtop_catalogue_entry(payload_key: &str, gtop_bytes: &[u8]) -> PayloadCatalogueEntry {
    let mut hasher = Sha256::new();
    hasher.update(gtop_bytes);
    let digest = hasher.finalize();
    let mut sha256_hex = String::with_capacity(64);
    for b in digest {
        use core::fmt::Write;
        let _ = write!(&mut sha256_hex, "{b:02x}");
    }
    PayloadCatalogueEntry {
        payload_key: payload_key.to_string(),
        mime_type: GTOP_MIME_TYPE.to_string(),
        language: None,
        role: GTOP_CATALOGUE_ROLE.to_string(),
        byte_length: gtop_bytes.len() as u64,
        sha256_hex,
    }
}

/// Returns `true` if a [`PayloadCatalogueEntry`] represents a `GTOP` geometry-topology complex.
#[inline]
pub fn is_gtop_catalogue_entry(entry: &PayloadCatalogueEntry) -> bool {
    entry.role == GTOP_CATALOGUE_ROLE || entry.mime_type == GTOP_MIME_TYPE
}

/// Returns `true` if `section_bytes` starts with the 4-byte `"GTOP"` magic tag.
#[inline]
pub fn is_gtop_section(section_bytes: &[u8]) -> bool {
    section_bytes.len() >= 4 && &section_bytes[0..4] == &GTOP_SECTION_MAGIC
}

/// Wrap a raw 64B-aligned `GTOP` geometry segment in a self-describing 16-byte `"GTOP"` v1 envelope.
pub fn encode_gtop_section(segment_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let len = u32::try_from(segment_bytes.len())
        .map_err(|_| "GTOP segment byte length exceeds u32::MAX".to_string())?;
    let crc = crc32c::crc32c(segment_bytes);
    let mut out = Vec::with_capacity(GTOP_SECTION_HEADER_BYTES + segment_bytes.len());
    out.extend_from_slice(&GTOP_SECTION_MAGIC);
    out.extend_from_slice(&GTOP_SECTION_VERSION.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&crc.to_le_bytes());
    out.extend_from_slice(segment_bytes);
    Ok(out)
}

/// Verify and unwrap a `"GTOP"` v1 section envelope back into its inner segment slice.
pub fn decode_gtop_section(section_bytes: &[u8]) -> Result<&[u8], String> {
    if section_bytes.len() < GTOP_SECTION_HEADER_BYTES {
        return Err("GTOP section shorter than 16-byte header".to_string());
    }
    if !is_gtop_section(section_bytes) {
        return Err("invalid GTOP section magic (expected GTOP)".to_string());
    }
    let version = u32::from_le_bytes(section_bytes[4..8].try_into().unwrap());
    if version != GTOP_SECTION_VERSION {
        return Err(format!("unsupported GTOP section version {version}"));
    }
    let expected_len = u32::from_le_bytes(section_bytes[8..12].try_into().unwrap()) as usize;
    let expected_crc = u32::from_le_bytes(section_bytes[12..16].try_into().unwrap());
    let payload = &section_bytes[GTOP_SECTION_HEADER_BYTES..];
    if payload.len() != expected_len {
        return Err(format!(
            "GTOP section length mismatch: header {expected_len} vs payload {}",
            payload.len()
        ));
    }
    let actual_crc = crc32c::crc32c(payload);
    if actual_crc != expected_crc {
        return Err(format!(
            "GTOP section CRC-32C mismatch: expected {expected_crc:#010x}, got {actual_crc:#010x}"
        ));
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtop_catalogue_entry_and_section_roundtrip() {
        let fake_segment = [0x42u8; 128];
        let entry = encode_gtop_catalogue_entry("topology/catchment.gtop", &fake_segment);
        assert!(is_gtop_catalogue_entry(&entry));
        assert_eq!(entry.byte_length, 128);
        assert_eq!(entry.sha256_hex.len(), 64);

        let cat_bytes = encode_payload_catalogue_section(&[entry.clone()]).unwrap();
        let decoded_cat = decode_payload_catalogue_section(&cat_bytes).unwrap();
        assert_eq!(decoded_cat.len(), 1);
        assert!(is_gtop_catalogue_entry(&decoded_cat[0]));

        let section = encode_gtop_section(&fake_segment).unwrap();
        assert!(is_gtop_section(&section));
        let unwrapped = decode_gtop_section(&section).unwrap();
        assert_eq!(unwrapped, &fake_segment);
    }

    #[test]
    fn payload_catalogue_section_roundtrips_with_crc32c_and_header_range() {
        let entries = vec![
            PayloadCatalogueEntry {
                payload_key: "ns/human-rights/udhr-article-1.md".to_string(),
                mime_type: "text/markdown".to_string(),
                language: Some("en".to_string()),
                role: "normative_prose".to_string(),
                byte_length: 1024,
                sha256_hex: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                    .to_string(),
            },
            PayloadCatalogueEntry {
                payload_key: "shapes/natural_agent.shacl.ttl".to_string(),
                mime_type: "text/turtle".to_string(),
                language: None,
                role: "shacl_shape_pack".to_string(),
                byte_length: 4096,
                sha256_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .to_string(),
            },
        ];

        let encoded = encode_payload_catalogue_section(&entries).unwrap();
        let decoded = decode_payload_catalogue_section(&encoded).unwrap();
        assert_eq!(decoded, entries);

        // Verify tamper detection via CRC-32C
        let mut corrupted = encoded.clone();
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x55;
        assert!(decode_payload_catalogue_section(&corrupted).is_err());

        // Verify header range stamping
        let mut header = Q42VolumeHeader::new_v4(256, 64, 320, 64, 384, 16, 400, 4096, 1, 40960, 853);
        assert_eq!(header_payload_catalogue_range(&header), None);
        set_header_payload_catalogue_range(&mut header, 4496, encoded.len() as u64);
        assert_eq!(
            header_payload_catalogue_range(&header),
            Some((4496, encoded.len() as u64))
        );
    }
}
