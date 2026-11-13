use super::{hash::parent, leaf_hash, ProofError, TreeSpec, MAX_SIBLINGS};
use crate::net::qdnf::types::StrongDigest;

/// Verify exact bytes against a supplied root. This does not authorize consumption.
pub fn verify_leaf(
    spec: &TreeSpec,
    root: StrongDigest,
    index: u64,
    bytes: &[u8],
    siblings: &[StrongDigest],
) -> Result<(), ProofError> {
    let mut digest = leaf_hash(spec, index, bytes)?;
    if siblings.len() > MAX_SIBLINGS {
        return Err(ProofError::ProofLength);
    }
    let mut path = [super::ProofSiblingRange::EMPTY; MAX_SIBLINGS];
    let depth = super::sibling_ranges(spec.leaf_count(), index, &mut path)?;
    if siblings.len() != depth {
        return Err(ProofError::ProofLength);
    }
    for (range, sibling) in path[..depth].iter().zip(siblings) {
        digest = match range.side {
            super::SiblingSide::Left => parent(*sibling, digest),
            super::SiblingSide::Right => parent(digest, *sibling),
        };
    }
    // Roots are public commitments; equality does not compare secret material.
    if digest != root {
        return Err(ProofError::RootMismatch);
    }
    Ok(())
}
