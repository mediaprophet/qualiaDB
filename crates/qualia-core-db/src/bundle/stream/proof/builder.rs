use super::{empty_root, hash::parent, leaf_hash, ProofError, TreeSpec, MAX_SIBLINGS};
use crate::net::qdnf::types::StrongDigest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutboardNode {
    pub start_leaf: u64,
    pub leaf_count: u64,
    pub digest: StrongDigest,
}

/// Fixed-memory sequential builder. A failed or panicking observer poisons the
/// builder: no root can subsequently be obtained. Sink durability/commit remains
/// a separate publication gate; emitted nodes are only candidate digests.
pub struct TreeBuilder {
    spec: TreeSpec,
    count: u64,
    frontier: [StrongDigest; MAX_SIBLINGS + 1],
    poisoned: bool,
    finished: Option<StrongDigest>,
}
impl TreeBuilder {
    pub fn new(spec: TreeSpec) -> Self {
        Self {
            spec,
            count: 0,
            frontier: [StrongDigest::ZERO; MAX_SIBLINGS + 1],
            poisoned: false,
            finished: None,
        }
    }
    pub const fn appended(&self) -> u64 {
        self.count
    }
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }
    pub fn append(
        &mut self,
        bytes: &[u8],
        mut observe: impl FnMut(OutboardNode) -> Result<(), ProofError>,
    ) -> Result<(), ProofError> {
        if self.poisoned {
            return Err(ProofError::Poisoned);
        }
        if self.count == self.spec.leaf_count() {
            return Err(ProofError::Complete);
        }
        let mut digest = leaf_hash(&self.spec, self.count, bytes)?;
        let mut level = 0;
        let mut width = 1u64;
        self.poisoned = true;
        observe(OutboardNode {
            start_leaf: self.count,
            leaf_count: width,
            digest,
        })
        .map_err(|_| ProofError::Sink)?;
        while self.count & width != 0 {
            digest = parent(self.frontier[level], digest);
            level += 1;
            width <<= 1;
            observe(OutboardNode {
                start_leaf: self.count + 1 - width,
                leaf_count: width,
                digest,
            })
            .map_err(|_| ProofError::Sink)?;
        }
        self.frontier[level] = digest;
        self.count += 1;
        self.poisoned = false;
        Ok(())
    }
    pub fn finish(
        &mut self,
        mut observe: impl FnMut(OutboardNode) -> Result<(), ProofError>,
    ) -> Result<StrongDigest, ProofError> {
        if self.poisoned {
            return Err(ProofError::Poisoned);
        }
        if let Some(root) = self.finished {
            return Ok(root);
        }
        if self.count != self.spec.leaf_count() {
            return Err(ProofError::Incomplete);
        }
        if self.count == 0 {
            let root = empty_root(&self.spec)?;
            self.finished = Some(root);
            return Ok(root);
        }
        self.poisoned = true;
        let mut result = None;
        let mut right_count = 0u64;
        for level in 0..=MAX_SIBLINGS {
            let width = 1u64 << level;
            if self.count & width == 0 {
                continue;
            }
            let digest = match result {
                None => self.frontier[level],
                Some(right) => parent(self.frontier[level], right),
            };
            if result.is_some() {
                observe(OutboardNode {
                    start_leaf: self.count - right_count - width,
                    leaf_count: right_count + width,
                    digest,
                })
                .map_err(|_| ProofError::Sink)?;
            }
            result = Some(digest);
            right_count += width;
        }
        let root = result.ok_or(ProofError::Incomplete)?;
        self.finished = Some(root);
        self.poisoned = false;
        Ok(root)
    }
}
