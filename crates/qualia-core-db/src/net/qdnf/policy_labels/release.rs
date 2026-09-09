//! Reviewed release cannot let one issuer relax another source's constraints.

use super::types::{Confidentiality, LabelFields, VerifiedLabel};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Lower confidentiality only with the originating issuer. Restriction bits,
/// compartments, purposes and audience are never dropped on this path.
/// Default deny.
pub fn release_derivation(
    original: &VerifiedLabel,
    proposed: &LabelFields,
    release_issuer: StrongDigest,
) -> Result<LabelFields, QdnfError> {
    if release_issuer.is_zero() || original.issuer().is_zero() {
        return Err(QdnfError::Unauthorized);
    }
    if original.issuer() != release_issuer {
        return Err(QdnfError::Denied);
    }
    if proposed.confidentiality.is_unknown() || original.confidentiality().is_unknown() {
        return Err(QdnfError::Conflict);
    }
    let orig_rank = original.confidentiality().lattice_rank()?;
    let new_rank = proposed.confidentiality.lattice_rank()?;
    if new_rank > orig_rank {
        return Err(QdnfError::Denied);
    }
    let original_bits = original.fields().restriction_bits;
    if (proposed.restriction_bits & original_bits) != original_bits {
        return Err(QdnfError::Denied);
    }
    let mut out = *original.fields();
    out.confidentiality = proposed.confidentiality;
    out.restriction_bits = original_bits | proposed.restriction_bits;
    if out.confidentiality == Confidentiality::Unknown {
        return Err(QdnfError::Conflict);
    }
    Ok(out)
}
