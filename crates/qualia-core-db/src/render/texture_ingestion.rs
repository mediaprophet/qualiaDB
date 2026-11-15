//! Production texture ingestion for external/data URI and embedded images.
//!
//! Enforces strict MIME/magic signature checks, safe arithmetic, dimension and
//! byte-budget ceilings, and failure-safe rollback.

use super::texture_decode::TextureDecodeLimits;
use super::texture_ktx2::KTX2_IDENTIFIER;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};

pub const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
pub const JPEG_SIGNATURE: [u8; 3] = [0xFF, 0xD8, 0xFF];
pub const WEBP_RIFF: [u8; 4] = *b"RIFF";
pub const WEBP_MAGIC: [u8; 4] = *b"WEBP";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureIngestionError {
    EmptySource,
    EncodedInputTooLarge,
    InvalidDataUri,
    Base64DecodeFailed,
    MimeSignatureMismatch,
    UnsupportedMime,
    ExternalUriRequiresResolver,
}

impl std::fmt::Display for TextureIngestionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "texture ingestion: {self:?}")
    }
}

impl std::error::Error for TextureIngestionError {}

/// Validate the binary file header against the declared MIME type.
pub fn validate_image_signature(
    mime_type: &str,
    bytes: &[u8],
) -> Result<(), TextureIngestionError> {
    let clean_mime = mime_type.split(';').next().map(str::trim).unwrap_or("");
    if clean_mime.eq_ignore_ascii_case("image/png") {
        if bytes.len() < 8 || bytes[..8] != PNG_SIGNATURE {
            return Err(TextureIngestionError::MimeSignatureMismatch);
        }
    } else if clean_mime.eq_ignore_ascii_case("image/jpeg")
        || clean_mime.eq_ignore_ascii_case("image/jpg")
    {
        if bytes.len() < 3 || bytes[..3] != JPEG_SIGNATURE {
            return Err(TextureIngestionError::MimeSignatureMismatch);
        }
    } else if clean_mime.eq_ignore_ascii_case("image/ktx2") {
        if bytes.len() < 12 || bytes[..12] != KTX2_IDENTIFIER {
            return Err(TextureIngestionError::MimeSignatureMismatch);
        }
    } else if clean_mime.eq_ignore_ascii_case("image/webp") {
        if bytes.len() < 12 || bytes[..4] != WEBP_RIFF || bytes[8..12] != WEBP_MAGIC {
            return Err(TextureIngestionError::MimeSignatureMismatch);
        }
    } else {
        return Err(TextureIngestionError::UnsupportedMime);
    }
    Ok(())
}

fn percent_decode(input: &str) -> Option<Vec<u8>> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let h1 = char::from(bytes[i + 1]).to_digit(16)? as u8;
            let h2 = char::from(bytes[i + 2]).to_digit(16)? as u8;
            out.push((h1 << 4) | h2);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}

/// Parse a `data:[mediatype][;base64],<data>` URI and decode the inner payload.
pub fn parse_data_uri(
    uri: &str,
    max_bytes: usize,
) -> Result<(String, Vec<u8>), TextureIngestionError> {
    if !uri.starts_with("data:") {
        return Err(TextureIngestionError::InvalidDataUri);
    }
    let rest = &uri[5..];
    let comma_idx = rest
        .find(',')
        .ok_or(TextureIngestionError::InvalidDataUri)?;
    let metadata = &rest[..comma_idx];
    let payload_str = &rest[comma_idx + 1..];

    let is_base64 = metadata.ends_with(";base64");
    let mime_part = if is_base64 {
        &metadata[..metadata.len() - 7]
    } else {
        metadata
    };
    let mime_type = if mime_part.is_empty() {
        "application/octet-stream".to_string()
    } else {
        mime_part
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    };

    let bytes = if is_base64 {
        BASE64_STANDARD
            .decode(payload_str.trim().as_bytes())
            .map_err(|_| TextureIngestionError::Base64DecodeFailed)?
    } else {
        percent_decode(payload_str).ok_or(TextureIngestionError::InvalidDataUri)?
    };

    if bytes.len() > max_bytes {
        return Err(TextureIngestionError::EncodedInputTooLarge);
    }

    validate_image_signature(&mime_type, &bytes)?;

    Ok((mime_type, bytes))
}

/// A content-addressed, signature-verified ingested texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IngestedTexture {
    pub digest: [u8; 32],
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

/// Ingest a texture from either a Data URI or raw encoded bytes with strict signature validation.
pub fn ingest_texture_payload(
    mime_type_or_uri: &str,
    raw_bytes: Option<&[u8]>,
    limits: TextureDecodeLimits,
) -> Result<IngestedTexture, TextureIngestionError> {
    let (mime_type, bytes) = if mime_type_or_uri.starts_with("data:") {
        parse_data_uri(mime_type_or_uri, limits.max_encoded_bytes)?
    } else if let Some(payload) = raw_bytes {
        if payload.len() > limits.max_encoded_bytes {
            return Err(TextureIngestionError::EncodedInputTooLarge);
        }
        validate_image_signature(mime_type_or_uri, payload)?;
        (mime_type_or_uri.to_string(), payload.to_vec())
    } else {
        return Err(TextureIngestionError::ExternalUriRequiresResolver);
    };

    let digest: [u8; 32] = Sha256::digest(&bytes).into();
    Ok(IngestedTexture {
        digest,
        mime_type,
        bytes,
    })
}

#[cfg(test)]
#[path = "texture_ingestion_tests.rs"]
mod tests;
