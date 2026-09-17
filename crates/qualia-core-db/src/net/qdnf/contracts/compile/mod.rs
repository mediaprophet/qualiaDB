//! Compiled contract admission. Unknown required semantics cannot become Allow.

pub mod verify;

use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub use verify::verify_exact_bytes;

/// Pinned contract view. Vocabulary membership is stored as digests filled by
/// the verifier from a known-vocab table, not as a caller boolean.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractBundle {
    pub exact_bytes_digest: StrongDigest,
    pub context_digest: StrongDigest,
    pub ontology_digest: StrongDigest,
    pub shape_digest: StrongDigest,
    pub rules_digest: StrongDigest,
    /// Vocabulary the artifact requires. Zero means unknown required semantics.
    pub required_vocab_digest: StrongDigest,
    /// Local known-vocabulary table digest bound by the verifier.
    pub known_vocab_digest: StrongDigest,
}

impl ContractBundle {
    #[inline]
    pub const fn exact_bytes_unsigned(&self) -> bool {
        self.exact_bytes_digest.is_zero()
    }

    /// Bind the local known-vocabulary table. Callers of [`compile_decision`]
    /// must not invent a matching pair to mean “signed”.
    #[inline]
    pub fn bind_known_vocabulary(&mut self, known_vocab_digest: StrongDigest) {
        self.known_vocab_digest = known_vocab_digest;
    }

    #[inline]
    pub fn required_vocabulary_known(&self) -> bool {
        !self.required_vocab_digest.is_zero()
            && !self.known_vocab_digest.is_zero()
            && self.required_vocab_digest == self.known_vocab_digest
    }
}

/// Admit a compiled view only when exact original bytes hash to the bundle
/// digest, expected shape/rules/context/ontology/vocabulary match, and the
/// required vocabulary is present in the verifier's known table.
///
/// There is no `signed` / `supported` boolean. Empty bytes are malformed.
/// Vocabulary mismatch is [`PolicyOutcome::Incomplete`] (no `Unsupported`
/// variant exists on [`PolicyOutcome`]); it is never [`PolicyOutcome::Allow`].
pub fn compile_decision(
    bundle: &ContractBundle,
    expected: &ContractBundle,
    original_bytes: &[u8],
) -> Result<PolicyOutcome, QdnfError> {
    verify_exact_bytes(bundle.exact_bytes_digest, original_bytes)?;
    if bundle.exact_bytes_unsigned() {
        return Err(QdnfError::Malformed);
    }
    if bundle.exact_bytes_digest != expected.exact_bytes_digest
        || bundle.context_digest != expected.context_digest
        || bundle.ontology_digest != expected.ontology_digest
        || bundle.shape_digest != expected.shape_digest
        || bundle.rules_digest != expected.rules_digest
        || bundle.required_vocab_digest != expected.required_vocab_digest
    {
        return Err(QdnfError::Conflict);
    }
    if bundle.required_vocab_digest.is_zero() || bundle.known_vocab_digest.is_zero() {
        return Ok(PolicyOutcome::Incomplete);
    }
    if bundle.required_vocab_digest != bundle.known_vocab_digest {
        // Unknown or unsupported required vocabulary. PolicyOutcome has no
        // Unsupported variant; Incomplete must never become Allow.
        return Ok(PolicyOutcome::Incomplete);
    }
    Ok(PolicyOutcome::Allow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    const BYTES: &[u8] = b"qdnf-contract-v1-artifact";

    fn vocab() -> StrongDigest {
        StrongDigest([0x11; 48])
    }

    fn other_vocab() -> StrongDigest {
        StrongDigest([0x22; 48])
    }

    fn pinned_bundle(bytes: &[u8]) -> ContractBundle {
        let digest = sha384(bytes);
        ContractBundle {
            exact_bytes_digest: digest,
            context_digest: StrongDigest([0xA1; 48]),
            ontology_digest: StrongDigest([0xA2; 48]),
            shape_digest: StrongDigest([0xA3; 48]),
            rules_digest: StrongDigest([0xA4; 48]),
            required_vocab_digest: vocab(),
            known_vocab_digest: vocab(),
        }
    }

    #[test]
    fn real_bytes_allow() {
        let bundle = pinned_bundle(BYTES);
        assert_eq!(
            compile_decision(&bundle, &bundle, BYTES).unwrap(),
            PolicyOutcome::Allow
        );
    }

    #[test]
    fn attacker_matching_digest_without_bytes_is_malformed() {
        let bundle = pinned_bundle(BYTES);
        assert_eq!(
            compile_decision(&bundle, &bundle, b""),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn tampered_bytes_are_conflict() {
        let bundle = pinned_bundle(BYTES);
        assert_eq!(
            compile_decision(&bundle, &bundle, b"tampered-not-original"),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn forged_digest_with_other_bytes_is_conflict() {
        let mut bundle = pinned_bundle(BYTES);
        bundle.exact_bytes_digest = sha384(b"different-preimage");
        let mut expected = bundle;
        expected.exact_bytes_digest = bundle.exact_bytes_digest;
        assert_eq!(
            compile_decision(&bundle, &expected, BYTES),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn zero_exact_bytes_are_malformed() {
        let mut bundle = pinned_bundle(BYTES);
        bundle.exact_bytes_digest = StrongDigest::ZERO;
        assert_eq!(
            compile_decision(&bundle, &bundle, BYTES),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn unknown_required_vocab_is_incomplete_not_allow() {
        let mut bundle = pinned_bundle(BYTES);
        bundle.required_vocab_digest = other_vocab();
        bundle.bind_known_vocabulary(vocab());
        let expected = bundle;
        let outcome = compile_decision(&bundle, &expected, BYTES).unwrap();
        assert_eq!(outcome, PolicyOutcome::Incomplete);
        assert_ne!(outcome, PolicyOutcome::Allow);
    }

    #[test]
    fn zero_required_vocab_is_incomplete_not_allow() {
        let mut bundle = pinned_bundle(BYTES);
        bundle.required_vocab_digest = StrongDigest::ZERO;
        let expected = bundle;
        assert_eq!(
            compile_decision(&bundle, &expected, BYTES).unwrap(),
            PolicyOutcome::Incomplete
        );
    }

    #[test]
    fn shape_digest_mismatch_is_conflict() {
        let bundle = pinned_bundle(BYTES);
        let mut expected = bundle;
        expected.shape_digest = StrongDigest([9u8; 48]);
        assert_eq!(
            compile_decision(&bundle, &expected, BYTES),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn rules_digest_mismatch_is_conflict() {
        let bundle = pinned_bundle(BYTES);
        let mut expected = bundle;
        expected.rules_digest = StrongDigest([9u8; 48]);
        assert_eq!(
            compile_decision(&bundle, &expected, BYTES),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn context_digest_mismatch_is_conflict() {
        let bundle = pinned_bundle(BYTES);
        let mut expected = bundle;
        expected.context_digest = StrongDigest([9u8; 48]);
        assert_eq!(
            compile_decision(&bundle, &expected, BYTES),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn ontology_digest_mismatch_is_conflict() {
        let bundle = pinned_bundle(BYTES);
        let mut expected = bundle;
        expected.ontology_digest = StrongDigest([9u8; 48]);
        assert_eq!(
            compile_decision(&bundle, &expected, BYTES),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn required_vocab_pin_mismatch_is_conflict() {
        let bundle = pinned_bundle(BYTES);
        let mut expected = bundle;
        expected.required_vocab_digest = other_vocab();
        assert_eq!(
            compile_decision(&bundle, &expected, BYTES),
            Err(QdnfError::Conflict)
        );
    }
}
