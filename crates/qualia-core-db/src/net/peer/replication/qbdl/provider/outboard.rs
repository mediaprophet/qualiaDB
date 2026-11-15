//! Borrowed bounded lookup over actual `TreeBuilder` outboard nodes.
use crate::bundle::stream::{
    proof::{
        empty_root, encode_proof, sibling_ranges, OutboardNode, ProofError, ProofSiblingRange,
        TreeSpec, MAX_SIBLINGS,
    },
    selection::SelectedPlainEntry,
};
use crate::net::qdnf::types::StrongDigest;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutboardLimits {
    pub max_nodes: usize,
    pub max_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutboardError {
    Identity,
    Bounds,
    Limit,
    Capacity,
    DuplicateNode,
    UnsortedNodes,
    MissingNode,
    Proof(ProofError),
}

impl From<ProofError> for OutboardError {
    fn from(error: ProofError) -> Self {
        Self::Proof(error)
    }
}

/// A borrowed index of the exact candidate nodes emitted by `TreeBuilder`.
/// Construction is cold and bounded. Proof bytes remain candidates until the
/// caller verifies them against the actual leaf and root with `verify_leaf`.
pub struct OutboardCatalog<'p, 'n> {
    selected: SelectedPlainEntry<'p>,
    spec: TreeSpec,
    root: StrongDigest,
    nodes: &'n [OutboardNode],
    charged_bytes: usize,
}

impl<'p, 'n> OutboardCatalog<'p, 'n> {
    /// Bind a caller-sorted emitted-node slice to the exact selected
    /// entry/spec/root and an explicit cold count/byte budget. Sort the actual
    /// `TreeBuilder` output once in cold construction, then lend it here; this
    /// catalog does not copy or allocate.
    pub fn new(
        selected: SelectedPlainEntry<'p>,
        spec: TreeSpec,
        root: StrongDigest,
        nodes: &'n [OutboardNode],
        limits: OutboardLimits,
    ) -> Result<Self, OutboardError> {
        let selected_spec = selected.tree_spec().map_err(|_| OutboardError::Identity)?;
        if selected_spec != spec || selected.root() != root {
            return Err(OutboardError::Identity);
        }
        if nodes.len() > limits.max_nodes {
            return Err(OutboardError::Limit);
        }
        let charged_bytes = nodes
            .len()
            .checked_mul(std::mem::size_of::<OutboardNode>())
            .ok_or(OutboardError::Bounds)?;
        if charged_bytes > limits.max_bytes {
            return Err(OutboardError::Limit);
        }

        let leaf_count = spec.leaf_count();
        let mut previous_key: Option<(u64, u64)> = None;
        for node in nodes {
            let end = node
                .start_leaf
                .checked_add(node.leaf_count)
                .ok_or(OutboardError::Bounds)?;
            if node.leaf_count == 0 || end > leaf_count {
                return Err(OutboardError::Bounds);
            }
            let key = (node.start_leaf, node.leaf_count);
            if let Some(previous) = previous_key {
                if key == previous {
                    return Err(OutboardError::DuplicateNode);
                }
                if key < previous {
                    return Err(OutboardError::UnsortedNodes);
                }
            }
            previous_key = Some(key);
        }

        if leaf_count == 0 {
            if !nodes.is_empty() || empty_root(&spec)? != root {
                return Err(OutboardError::Identity);
            }
        } else {
            let root_node = find_node(nodes, 0, leaf_count).ok_or(OutboardError::MissingNode)?;
            if root_node.digest != root {
                return Err(OutboardError::Identity);
            }
        }

        Ok(Self {
            selected,
            spec,
            root,
            nodes,
            charged_bytes,
        })
    }

    pub const fn selected(&self) -> SelectedPlainEntry<'p> {
        self.selected
    }

    pub const fn tree_spec(&self) -> &TreeSpec {
        &self.spec
    }

    pub const fn root(&self) -> StrongDigest {
        self.root
    }

    pub const fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub const fn charged_bytes(&self) -> usize {
        self.charged_bytes
    }

    /// Encode the canonical sibling proof for one leaf into caller buffers.
    /// No bytes are read or authenticated here; the caller must verify the
    /// returned proof against the actual leaf using canonical `verify_leaf`.
    pub fn proof_into(
        &self,
        leaf_index: u64,
        out: &mut [u8],
        sibling_scratch: &mut [StrongDigest],
    ) -> Result<usize, OutboardError> {
        let mut path = [ProofSiblingRange::EMPTY; MAX_SIBLINGS];
        let count = sibling_ranges(self.spec.leaf_count(), leaf_index, &mut path)?;
        let required = 1usize
            .checked_add(count.checked_mul(48).ok_or(OutboardError::Bounds)?)
            .ok_or(OutboardError::Bounds)?;
        if out.len() < required || sibling_scratch.len() < count {
            return Err(OutboardError::Capacity);
        }

        // Check complete path coverage before changing output or scratch.
        for range in &path[..count] {
            if self.find_node(range).is_none() {
                return Err(OutboardError::MissingNode);
            }
        }
        for (slot, range) in sibling_scratch[..count].iter_mut().zip(&path[..count]) {
            *slot = self.find_node(range).expect("path was preflighted").digest;
        }
        Ok(encode_proof(&sibling_scratch[..count], out)?)
    }

    fn find_node(&self, range: &ProofSiblingRange) -> Option<&OutboardNode> {
        find_node(self.nodes, range.start_leaf, range.leaf_count)
    }
}

fn find_node(nodes: &[OutboardNode], start_leaf: u64, leaf_count: u64) -> Option<&OutboardNode> {
    nodes
        .binary_search_by_key(&(start_leaf, leaf_count), |node| {
            (node.start_leaf, node.leaf_count)
        })
        .ok()
        .map(|index| &nodes[index])
}
