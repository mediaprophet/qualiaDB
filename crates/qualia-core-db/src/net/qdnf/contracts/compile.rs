//! Compiled contract admission. Unknown required semantics cannot become Allow.

use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractBundle {
    pub exact_bytes_digest: StrongDigest,
    pub context_digest: StrongDigest,
    pub ontology_digest: StrongDigest,
    pub shape_digest: StrongDigest,
    pub rules_digest: StrongDigest,
}

impl ContractBundle {
    #[inline]
    pub const fn exact_bytes_unsigned(&self) -> bool {
        let mut i = 0;
        while i < 48 {
            if self.exact_bytes_digest.0[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

/// Admit a compiled view only when exact bytes are present, expected shape/rules
/// match, required semantics are known, and the fragment is supported.
pub fn compile_decision(
    bundle: &ContractBundle,
    expected: &ContractBundle,
    signed: bool,
    supported: bool,
    unknown_required: bool,
) -> Result<PolicyOutcome, QdnfError> {
    if !signed || bundle.exact_bytes_unsigned() {
        return Err(QdnfError::Malformed);
    }
    if bundle.shape_digest != expected.shape_digest || bundle.rules_digest != expected.rules_digest
    {
        return Err(QdnfError::Conflict);
    }
    if unknown_required {
        return Ok(PolicyOutcome::Incomplete);
    }
    if !supported {
        return Ok(PolicyOutcome::Error);
    }
    Ok(PolicyOutcome::Allow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signed_bundle() -> ContractBundle {
        ContractBundle {
            exact_bytes_digest: StrongDigest([1u8; 48]),
            context_digest: StrongDigest::ZERO,
            ontology_digest: StrongDigest::ZERO,
            shape_digest: StrongDigest([2u8; 48]),
            rules_digest: StrongDigest([3u8; 48]),
        }
    }

    #[test]
    fn unknown_required_is_not_allow() {
        let bundle = signed_bundle();
        assert_eq!(
            compile_decision(&bundle, &bundle, true, true, true).unwrap(),
            PolicyOutcome::Incomplete
        );
    }

    #[test]
    fn zero_exact_bytes_are_malformed() {
        let mut bundle = signed_bundle();
        bundle.exact_bytes_digest = StrongDigest::ZERO;
        assert_eq!(
            compile_decision(&bundle, &bundle, true, true, false),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn unsigned_bundle_is_malformed() {
        let bundle = signed_bundle();
        assert_eq!(
            compile_decision(&bundle, &bundle, false, true, false),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn shape_digest_mismatch_is_conflict() {
        let bundle = signed_bundle();
        let mut expected = bundle;
        expected.shape_digest = StrongDigest([9u8; 48]);
        assert_eq!(
            compile_decision(&bundle, &expected, true, true, false),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn rules_digest_mismatch_is_conflict() {
        let bundle = signed_bundle();
        let mut expected = bundle;
        expected.rules_digest = StrongDigest([9u8; 48]);
        assert_eq!(
            compile_decision(&bundle, &expected, true, true, false),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn matching_signed_supported_bundle_allows() {
        let bundle = signed_bundle();
        assert_eq!(
            compile_decision(&bundle, &bundle, true, true, false).unwrap(),
            PolicyOutcome::Allow
        );
    }
}
