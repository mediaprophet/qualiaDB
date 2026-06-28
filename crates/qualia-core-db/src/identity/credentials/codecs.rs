//! Credential transport codecs (from `0.0.19-g3-cbor-ld`) — encode/decode a
//! [`Credential`](super::Credential) to alternative carriers. Uses the unified
//! `Credential` type from the parent module (the branch's duplicate was dropped).

use super::Credential;

#[derive(Debug)]
pub enum CodecError {
    SerializationError(String),
}

/// Encode/decode a Verifiable Credential to/from a transport representation.
pub trait CredentialCodec {
    fn encode(&self, credential: &Credential) -> Result<Vec<u8>, CodecError>;
    fn decode(&self, payload: &[u8]) -> Result<Credential, CodecError>;
}

/// OpenBadges v3 carrier: adds the OB context + type, then JSON-encodes.
pub struct OpenBadgeCodec;

const OB_CONTEXT: &str = "https://purl.imsglobal.org/spec/ob/v3p0/context-3.0.3.json";

impl CredentialCodec for OpenBadgeCodec {
    fn encode(&self, credential: &Credential) -> Result<Vec<u8>, CodecError> {
        let mut c = credential.clone();
        if !c.context.iter().any(|s| s == OB_CONTEXT) {
            c.context.push(OB_CONTEXT.to_string());
        }
        if !c.types.iter().any(|s| s == "OpenBadgeCredential") {
            c.types.push("OpenBadgeCredential".to_string());
        }
        serde_json::to_vec(&c)
            .map_err(|e| CodecError::SerializationError(format!("OpenBadge encoding failed: {e}")))
    }

    fn decode(&self, payload: &[u8]) -> Result<Credential, CodecError> {
        let credential: Credential = serde_json::from_slice(payload).map_err(|e| {
            CodecError::SerializationError(format!("OpenBadge decoding failed: {e}"))
        })?;
        if !credential.types.iter().any(|s| s == "OpenBadgeCredential") {
            return Err(CodecError::SerializationError(
                "Not a valid OpenBadgeCredential".to_string(),
            ));
        }
        Ok(credential)
    }
}

/// PDF carrier: embeds the credential JSON as a `/Metadata` stream in a minimal PDF.
pub struct PdfCodec;

impl CredentialCodec for PdfCodec {
    fn encode(&self, credential: &Credential) -> Result<Vec<u8>, CodecError> {
        let json = serde_json::to_string(credential)
            .map_err(|e| CodecError::SerializationError(format!("PDF encoding failed: {e}")))?;

        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.7\n");
        pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [] /Count 0 >>\nendobj\n");
        pdf.extend_from_slice(b"3 0 obj\n");
        pdf.extend_from_slice(
            format!(
                "<< /Type /Metadata /Subtype /XML /Length {} >>\n",
                json.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(b"stream\n");
        pdf.extend_from_slice(json.as_bytes());
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        pdf.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n0000000015 00000 n \n0000000066 00000 n \n0000000125 00000 n \n");
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\nstartxref\n250\n%%EOF\n");
        Ok(pdf)
    }

    fn decode(&self, payload: &[u8]) -> Result<Credential, CodecError> {
        let s = String::from_utf8_lossy(payload);
        if let Some(start) = s.find("stream\n{") {
            let json_start = start + 7;
            if let Some(end) = s[json_start..].find("\nendstream") {
                return serde_json::from_str(&s[json_start..json_start + end]).map_err(|e| {
                    CodecError::SerializationError(format!("PDF decoding failed: {e}"))
                });
            }
        }
        Err(CodecError::SerializationError(
            "Could not locate VC in PDF payload".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_credential() -> Credential {
        let mut subject = HashMap::new();
        subject.insert("id".to_string(), "did:example:123".to_string());
        subject.insert("achievement".to_string(), "Course Completion".to_string());
        Credential {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            id: "http://example.edu/credentials/3732".to_string(),
            types: vec!["VerifiableCredential".to_string()],
            issuer: "https://example.edu/issuers/565049".to_string(),
            issuance_date: "2010-01-01T19:23:24Z".to_string(),
            credential_subject: subject,
            proof: None,
        }
    }

    #[test]
    fn openbadge_roundtrip() {
        let codec = OpenBadgeCodec;
        let cred = test_credential();
        let encoded = codec.encode(&cred).expect("encode");
        let decoded = codec.decode(&encoded).expect("decode");
        assert!(decoded.types.iter().any(|s| s == "OpenBadgeCredential"));
        assert!(decoded.context.iter().any(|s| s == OB_CONTEXT));
        assert_eq!(decoded.id, cred.id);
    }

    #[test]
    fn pdf_roundtrip() {
        let codec = PdfCodec;
        let cred = test_credential();
        let encoded = codec.encode(&cred).expect("encode");
        assert!(encoded.starts_with(b"%PDF-1.7"));
        assert!(encoded.ends_with(b"%%EOF\n"));
        let decoded = codec.decode(&encoded).expect("decode");
        assert_eq!(decoded.id, cred.id);
        assert_eq!(decoded.issuer, cred.issuer);
    }
}
