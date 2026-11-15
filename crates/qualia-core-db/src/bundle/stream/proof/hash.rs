use super::{ProofError, TreeSpec};
use crate::net::qdnf::types::StrongDigest;
use sha2::{Digest, Sha384};

fn bind(h: &mut Sha384, spec: &TreeSpec) {
    h.update(spec.context.as_bytes());
    h.update(spec.entry.to_be_bytes());
    h.update(spec.generation.to_be_bytes());
    h.update([spec.representation as u8]);
    h.update(spec.epoch.to_be_bytes());
    h.update(spec.length.to_be_bytes());
}
fn finish(h: Sha384) -> StrongDigest {
    StrongDigest(h.finalize().into())
}

pub fn leaf_hash(spec: &TreeSpec, index: u64, bytes: &[u8]) -> Result<StrongDigest, ProofError> {
    let logical = spec.logical_leaf_length(index)?;
    if bytes.len() != spec.encoded_leaf_length(index)? {
        return Err(ProofError::Length);
    }
    let mut h = Sha384::new();
    h.update(b"\x00qbdl-stream-pq-1/leaf/v1\0");
    bind(&mut h, spec);
    h.update(index.to_be_bytes());
    h.update((logical as u32).to_be_bytes());
    h.update((bytes.len() as u32).to_be_bytes());
    h.update(bytes);
    Ok(finish(h))
}

pub fn empty_root(spec: &TreeSpec) -> Result<StrongDigest, ProofError> {
    if spec.leaf_count() != 0 {
        return Err(ProofError::InvalidSpec);
    }
    let mut h = Sha384::new();
    h.update(b"\x02qbdl-stream-pq-1/empty/v1\0");
    bind(&mut h, spec);
    Ok(finish(h))
}

pub(super) fn parent(left: StrongDigest, right: StrongDigest) -> StrongDigest {
    let mut h = Sha384::new();
    h.update(b"\x01qbdl-stream-pq-1/node/v1\0");
    h.update(left.as_bytes());
    h.update(right.as_bytes());
    finish(h)
}
