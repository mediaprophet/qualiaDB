//! Fixed bitsets and iterative traversal; no recursion, allocations or network.
use super::{
    ClosureCandidate, ClosureEdge, ClosureError as E, ClosureNode, MAX_CLOSURE_EDGES,
    MAX_CLOSURE_NODES, MAX_CLOSURE_ORIGINAL_BYTES,
};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;

fn locate(nodes: &[ClosureNode<'_>], digest: StrongDigest) -> Option<usize> {
    nodes.iter().position(|node| node.digest == digest)
}

/// Validate one explicit required-edge graph and its immutable original bytes.
/// Every supplied node must be reachable from root. Duplicate nodes are refused
/// even when bytes agree; manifest adapters must supply one row per digest.
/// Kinds are retained unchanged and are NOT authenticated by this operation.
pub fn verify_closure<'nodes, 'data>(
    root: StrongDigest,
    nodes: &'nodes [ClosureNode<'data>],
    edges: &'nodes [ClosureEdge],
) -> Result<ClosureCandidate<'nodes, 'data>, E> {
    if nodes.is_empty() || nodes.len() > MAX_CLOSURE_NODES {
        return Err(E::NodeLimit);
    }
    if edges.len() > MAX_CLOSURE_EDGES {
        return Err(E::EdgeLimit);
    }
    if root.is_zero() {
        return Err(E::ZeroDigest);
    }

    // All cheap capacity, geometry and membership checks precede hashing.
    let mut original_bytes = 0usize;
    for (i, node) in nodes.iter().enumerate() {
        if node.digest.is_zero() {
            return Err(E::ZeroDigest);
        }
        if nodes[..i].iter().any(|old| old.digest == node.digest) {
            return Err(E::DuplicateNode);
        }
        original_bytes = original_bytes
            .checked_add(node.original_bytes.len())
            .filter(|n| *n <= MAX_CLOSURE_ORIGINAL_BYTES)
            .ok_or(E::ByteLimit)?;
    }
    let root_index = locate(nodes, root).ok_or(E::MissingRoot)?;
    let mut adjacency = [0u16; MAX_CLOSURE_NODES];
    for edge in edges {
        let from = locate(nodes, edge.dependent).ok_or(E::MissingEndpoint)?;
        let to = locate(nodes, edge.dependency).ok_or(E::MissingEndpoint)?;
        if from == to {
            return Err(E::Cycle);
        }
        let bit = 1u16 << to;
        if adjacency[from] & bit != 0 {
            return Err(E::DuplicateEdge);
        }
        adjacency[from] |= bit;
    }

    let all = ((1u32 << nodes.len()) - 1) as u16;
    let mut reached = 0u16;
    let mut pending = 1u16 << root_index;
    while pending != 0 {
        let index = pending.trailing_zeros() as usize;
        let bit = 1u16 << index;
        pending &= !bit;
        reached |= bit;
        pending |= adjacency[index] & !reached;
    }
    if reached != all {
        return Err(E::UnreachableNode);
    }

    // Kahn ordering with dependencies emitted before dependents. Each iteration
    // emits one node; absence of a ready node proves a cycle in this finite graph.
    let mut order = [0u8; MAX_CLOSURE_NODES];
    let mut emitted = 0u16;
    for slot in &mut order[..nodes.len()] {
        let mut next: Option<usize> = None;
        for i in 0..nodes.len() {
            if emitted & (1u16 << i) != 0 || adjacency[i] & !emitted != 0 {
                continue;
            }
            if next.is_none_or(|old| nodes[i].digest.0 < nodes[old].digest.0) {
                next = Some(i);
            }
        }
        let index = next.ok_or(E::Cycle)?;
        *slot = index as u8;
        emitted |= 1u16 << index;
    }

    for node in nodes {
        if sha384(node.original_bytes) != node.digest {
            return Err(E::DigestMismatch);
        }
    }
    Ok(ClosureCandidate {
        root,
        nodes,
        edges,
        order,
        original_bytes,
    })
}
