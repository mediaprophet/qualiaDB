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

pub fn compile_decision(
    bundle: &ContractBundle,
    supported: bool,
    unknown_required: bool,
) -> Result<PolicyOutcome, QdnfError> {
    if bundle.exact_bytes_digest.0.iter().all(|&b| b == 0) {
        return Err(QdnfError::Malformed);
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

    #[test]
    fn unknown_required_is_not_allow() {
        let bundle = ContractBundle {
            exact_bytes_digest: StrongDigest([1u8; 48]),
            context_digest: StrongDigest::ZERO,
            ontology_digest: StrongDigest::ZERO,
            shape_digest: StrongDigest::ZERO,
            rules_digest: StrongDigest::ZERO,
        };
        assert_eq!(
            compile_decision(&bundle, true, true).unwrap(),
            PolicyOutcome::Incomplete
        );
    }
}
