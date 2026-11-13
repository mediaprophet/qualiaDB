//! Bounded contiguous complete-leaf proofs over actual immutable caller bytes.
//! This establishes inclusion against a supplied root, not publisher authority,
//! disclosure permission, complete-container verification or persisted presence.
use super::{decode_proof, verify_leaf, ProofError as E, Representation, TreeSpec, LEAF_BYTES};
use crate::net::qdnf::types::StrongDigest;

pub const MAX_SPAN_LEAVES: usize = 2048;
pub const MAX_SPAN_BYTES: usize = MAX_SPAN_LEAVES * (LEAF_BYTES as usize + 16);

/// Caller policy bounds only. Actual buffers need their existing runtime charge;
/// these numeric ceilings are never a resource reservation or authority token.
pub struct SpanLimits {
    pub max_leaves: usize,
    pub max_encoded_bytes: usize,
}
pub struct VerifiedLeafSpan<'a> {
    spec: TreeSpec,
    root: StrongDigest,
    first: u64,
    count: usize,
    bytes: &'a [u8],
}
impl VerifiedLeafSpan<'_> {
    pub const fn tree_spec(&self) -> &TreeSpec {
        &self.spec
    }
    pub const fn root(&self) -> StrongDigest {
        self.root
    }
    pub const fn first_leaf(&self) -> u64 {
        self.first
    }
    pub const fn leaf_count(&self) -> usize {
        self.count
    }
    pub fn encoded_bytes(&self) -> &[u8] {
        self.bytes
    }
    /// Slice entry-relative plaintext ONLY after all covering full leaves pass.
    /// Ciphertext proof bytes still contain tags and never become plaintext here.
    pub fn plaintext_range(&self, offset: u64, length: usize) -> Result<&[u8], E> {
        if self.spec.representation() != Representation::Plaintext {
            return Err(E::InvalidSpec);
        }
        let base = self.first.checked_mul(LEAF_BYTES).ok_or(E::Index)?;
        let start = offset.checked_sub(base).ok_or(E::Index)?;
        let start = usize::try_from(start).map_err(|_| E::Length)?;
        let end = start.checked_add(length).ok_or(E::Length)?;
        self.bytes.get(start..end).ok_or(E::Length)
    }
}

/// Each proof uses the existing exact sibling codec. Proof i authenticates leaf
/// first+i; omitted/trailing bytes, partial boundary leaves and trailing proof
/// bytes refuse. Typed sibling scratch is reused, not allocated. The caller must
/// match BOTH the complete TreeSpec and root to its recognized entry selection.
pub fn verify_leaf_span<'a>(
    spec: &TreeSpec,
    root: StrongDigest,
    first: u64,
    bytes: &'a [u8],
    proofs: &[&[u8]],
    sibling_scratch: &mut [StrongDigest],
    limits: SpanLimits,
) -> Result<VerifiedLeafSpan<'a>, E> {
    if limits.max_leaves == 0
        || limits.max_leaves > MAX_SPAN_LEAVES
        || limits.max_encoded_bytes == 0
        || limits.max_encoded_bytes > MAX_SPAN_BYTES
        || proofs.len() > limits.max_leaves
        || bytes.len() > limits.max_encoded_bytes
    {
        return Err(E::Limit);
    }
    if proofs.is_empty() {
        return Err(E::Incomplete);
    }
    let count = u64::try_from(proofs.len()).map_err(|_| E::Limit)?;
    let end = first.checked_add(count).ok_or(E::Index)?;
    if first >= spec.leaf_count() || end > spec.leaf_count() {
        return Err(E::Index);
    }
    let mut expected = 0usize;
    for index in first..end {
        expected = expected
            .checked_add(spec.encoded_leaf_length(index)?)
            .ok_or(E::Length)?;
    }
    if bytes.len() != expected {
        return Err(E::Length);
    }
    let mut position = 0usize;
    for (i, proof) in proofs.iter().enumerate() {
        let index = first + i as u64;
        let next = position + spec.encoded_leaf_length(index)?;
        let siblings = decode_proof(proof, sibling_scratch)?;
        verify_leaf(spec, root, index, &bytes[position..next], siblings)?;
        position = next;
    }
    Ok(VerifiedLeafSpan {
        spec: *spec,
        root,
        first,
        count: proofs.len(),
        bytes,
    })
}
