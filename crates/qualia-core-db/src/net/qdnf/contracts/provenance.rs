//! Claim provenance, issuer scope and disagreement.
//!
//! Relationship confidence and a caller-selected NaturalAgent enum cannot
//! certify identity.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Tracked claim. Confidence is observational; it never becomes identity.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClaimRecord {
    pub issuer: StrongDigest,
    pub scope: u64,
    pub disagreement: bool,
    pub confidence: u8,
}

impl ClaimRecord {
    pub const fn blank() -> Self {
        Self {
            issuer: StrongDigest::ZERO,
            scope: 0,
            disagreement: false,
            confidence: 0,
        }
    }
}

/// Confidence and a NaturalAgent enum never certify identity.
///
/// Always [`QdnfError::Unauthorized`]: neither argument is an identity proof.
pub fn certify_identity(_confidence: u8, _natural_agent_enum: bool) -> Result<(), QdnfError> {
    Err(QdnfError::Unauthorized)
}

/// Admit a claim as a scoped assertion, never as certified identity.
pub fn admit_claim(record: &ClaimRecord) -> Result<(), QdnfError> {
    if record.issuer.is_zero() {
        return Err(QdnfError::Unauthorized);
    }
    if record.scope == 0 {
        return Err(QdnfError::Unauthorized);
    }
    if record.disagreement {
        return Err(QdnfError::Conflict);
    }
    let _ = certify_identity(record.confidence, false);
    Ok(())
}

/// Contradictory claims in the same scope cannot collapse to one person.
pub fn contradicting_claims(a: &ClaimRecord, b: &ClaimRecord) -> Result<(), QdnfError> {
    if a.disagreement || b.disagreement {
        return Err(QdnfError::Conflict);
    }
    if a.scope == b.scope && a.issuer != b.issuer {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issuer(b: u8) -> StrongDigest {
        StrongDigest([b; 48])
    }

    fn claim(iss: u8, scope: u64, confidence: u8) -> ClaimRecord {
        ClaimRecord {
            issuer: issuer(iss),
            scope,
            disagreement: false,
            confidence,
        }
    }

    #[test]
    fn max_confidence_does_not_certify_identity() {
        assert_eq!(certify_identity(255, false), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn natural_agent_enum_does_not_certify_identity() {
        assert_eq!(certify_identity(255, true), Err(QdnfError::Unauthorized));
        assert_eq!(certify_identity(0, true), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn zero_issuer_claim_is_unauthorized() {
        let mut rec = claim(1, 7, 200);
        rec.issuer = StrongDigest::ZERO;
        assert_eq!(admit_claim(&rec), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn disagreement_is_conflict() {
        let mut rec = claim(1, 7, 200);
        rec.disagreement = true;
        assert_eq!(admit_claim(&rec), Err(QdnfError::Conflict));
    }

    #[test]
    fn scoped_claim_admits_without_certifying() {
        let rec = claim(3, 11, 255);
        assert!(admit_claim(&rec).is_ok());
        assert_eq!(
            certify_identity(rec.confidence, true),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn contradictory_issuers_in_scope_conflict() {
        assert_eq!(
            contradicting_claims(&claim(1, 4, 10), &claim(2, 4, 10)),
            Err(QdnfError::Conflict)
        );
    }
}
