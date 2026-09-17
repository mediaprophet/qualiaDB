//! Reviewed declassification as a new derivation. The original label is preserved.

use super::decode::encode_label_into;
use super::release::release_derivation;
use super::types::{Confidentiality, LabelFields, VerifiedLabel, MAX_LABEL_BYTES};
use super::verify::verify_label;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Provenance for a reviewed release. Original bytes stay bound to `parent`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeclassifyRecord {
    pub derived: VerifiedLabel,
    pub parent_digest: StrongDigest,
    pub authority: StrongDigest,
    pub issued_at: u64,
}

/// Create a new derivation. Confidentiality may lower only when `authority`
/// matches the originating issuer (release authority). Foreign issuers Denied.
pub fn declassify(
    original: &VerifiedLabel,
    authority: StrongDigest,
    now: u64,
) -> Result<VerifiedLabel, QdnfError> {
    Ok(declassify_record(original, authority, now)?.derived)
}

/// Same gate as [`declassify`], with parent digest retained for provenance.
pub fn declassify_record(
    original: &VerifiedLabel,
    authority: StrongDigest,
    now: u64,
) -> Result<DeclassifyRecord, QdnfError> {
    declassify_to(original, authority, now, Confidentiality::C0Public)
}

/// Lower (or keep) confidentiality to `target` under matching release authority.
pub fn declassify_to(
    original: &VerifiedLabel,
    authority: StrongDigest,
    now: u64,
    target: Confidentiality,
) -> Result<DeclassifyRecord, QdnfError> {
    if now == 0 {
        return Err(QdnfError::Malformed);
    }
    if authority.is_zero() {
        return Err(QdnfError::Unauthorized);
    }
    let mut proposed = *original.fields();
    proposed.confidentiality = target;
    let released = release_derivation(original, &proposed, authority)?;
    let derived = bind_derivation(&released)?;
    Ok(DeclassifyRecord {
        derived,
        parent_digest: original.exact_bytes_digest(),
        authority,
        issued_at: now,
    })
}

fn bind_derivation(fields: &LabelFields) -> Result<VerifiedLabel, QdnfError> {
    let mut buf = [0u8; MAX_LABEL_BYTES];
    let n = encode_label_into(fields, &mut buf)?;
    verify_label(*fields, &buf[..n])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::policy_labels::types::{LabelFields, NO_TRAINING};

    fn issuer(b: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = b;
        d.0[47] = 1;
        d
    }

    fn verified(conf: Confidentiality, iss: StrongDigest, restrictions: u16) -> VerifiedLabel {
        let mut fields = LabelFields::request(conf, iss);
        fields.restriction_bits = restrictions;
        if conf.requires_audience() {
            fields.audience = iss;
        }
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        verify_label(fields, &buf[..n]).unwrap()
    }

    #[test]
    fn matching_issuer_declassifies_to_public() {
        let original = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        let record = declassify_to(&original, issuer(1), 100, Confidentiality::C0Public).unwrap();
        assert_eq!(record.derived.confidentiality(), Confidentiality::C0Public);
        assert_eq!(record.parent_digest, original.exact_bytes_digest());
        assert_eq!(original.confidentiality(), Confidentiality::C2Sensitive);
        assert_eq!(
            record.derived.fields().restriction_bits & NO_TRAINING,
            NO_TRAINING
        );
        assert_ne!(
            record.derived.exact_bytes_digest(),
            original.exact_bytes_digest()
        );
        assert!(declassify(&original, issuer(1), 100).is_ok());
    }

    #[test]
    fn foreign_issuer_cannot_declassify() {
        let original = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        assert_eq!(
            declassify(&original, issuer(2), 100),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn zero_authority_is_unauthorized() {
        let original = verified(Confidentiality::C1Private, issuer(1), 0);
        assert_eq!(
            declassify(&original, StrongDigest::ZERO, 100),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn forged_release_raising_confidentiality_is_denied() {
        let original = verified(Confidentiality::C1Private, issuer(1), 0);
        assert_eq!(
            declassify_to(&original, issuer(1), 100, Confidentiality::C3Compartmented).unwrap_err(),
            QdnfError::Denied
        );
    }

    #[test]
    fn original_label_object_is_unchanged() {
        let original = verified(Confidentiality::C2Sensitive, issuer(1), NO_TRAINING);
        let before = original;
        let _ = declassify(&original, issuer(1), 50).unwrap();
        assert_eq!(original, before);
        assert_eq!(original.confidentiality(), Confidentiality::C2Sensitive);
    }
}
