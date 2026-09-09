//! Verify exact original bytes into a [`VerifiedLabel`]. No “signed” boolean.

use super::decode::decode_label_into;
use super::types::{LabelFields, VerifiedLabel};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Bind untrusted fields to exact original bytes. Fields must decode from
/// `original_bytes`; a digest without those bytes is malformed.
pub fn verify_label(
    fields: LabelFields,
    original_bytes: &[u8],
) -> Result<VerifiedLabel, QdnfError> {
    if original_bytes.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if fields.issuer.is_zero() {
        return Err(QdnfError::Unauthorized);
    }
    if fields.confidentiality.is_unknown() {
        return Err(QdnfError::Conflict);
    }
    if fields.confidentiality.requires_audience() && fields.audience.is_zero() {
        return Err(QdnfError::Conflict);
    }
    let mut decoded = LabelFields::blank();
    decode_label_into(original_bytes, &mut decoded)?;
    if decoded != fields {
        return Err(QdnfError::Conflict);
    }
    let digest = sha384(original_bytes);
    if digest.is_zero() {
        return Err(QdnfError::CryptoFailure);
    }
    Ok(VerifiedLabel::from_verified(decoded, digest))
}

pub fn verify_label_digest(
    fields: LabelFields,
    original_bytes: &[u8],
    expected: StrongDigest,
) -> Result<VerifiedLabel, QdnfError> {
    let label = verify_label(fields, original_bytes)?;
    if label.exact_bytes_digest() != expected {
        return Err(QdnfError::Conflict);
    }
    Ok(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::policy_labels::decode::encode_label_into;
    use crate::net::qdnf::policy_labels::types::Confidentiality;

    fn encode(fields: &LabelFields) -> ([u8; 256], usize) {
        let mut buf = [0u8; 256];
        let n = encode_label_into(fields, &mut buf).unwrap();
        (buf, n)
    }

    #[test]
    fn empty_bytes_are_malformed() {
        let fields = LabelFields::request(Confidentiality::C1Private, StrongDigest([1u8; 48]));
        assert_eq!(verify_label(fields, b"").unwrap_err(), QdnfError::Malformed);
    }

    #[test]
    fn fields_must_match_decoded_bytes() {
        let fields = LabelFields::request(Confidentiality::C1Private, StrongDigest([1u8; 48]));
        let mut other = fields;
        other.confidentiality = Confidentiality::C0Public;
        let (buf, n) = encode(&other);
        assert_eq!(
            verify_label(fields, &buf[..n]).unwrap_err(),
            QdnfError::Conflict
        );
    }
}
