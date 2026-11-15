use crate::net::qdnf::types::StrongDigest;
use crate::semantic_instruments::resolve::dependency::DependencyKind;

pub const MAX_CLOSURE_NODES: usize = 16;
pub const MAX_CLOSURE_EDGES: usize = 256;
pub const MAX_CLOSURE_ORIGINAL_BYTES: usize = 1024 * 1024;

/// Borrowed untrusted artifact. Kind reuses the existing semantic model.
/// Digest is SHA384 of EXACT original bytes, not a legacy textual lock digest.
/// Labels/issuer roles/versions must be bound by the later authenticated manifest.
#[derive(Clone, Copy, Debug)]
pub struct ClosureNode<'a> {
    pub digest: StrongDigest,
    pub kind: DependencyKind,
    pub original_bytes: &'a [u8],
}

/// Explicit required dependency: dependent -> dependency.
/// Both endpoints use complete security digests, never compact Quin identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClosureEdge {
    pub dependent: StrongDigest,
    pub dependency: StrongDigest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosureError {
    NodeLimit,
    EdgeLimit,
    ByteLimit,
    ZeroDigest,
    DuplicateNode,
    MissingRoot,
    MissingEndpoint,
    DuplicateEdge,
    Cycle,
    UnreachableNode,
    DigestMismatch,
}

/// Complete structural coverage of the SUPPLIED graph and matching original bytes.
/// This is NOT AuthenticatedClosure, policy admission or proof that a producer
/// declared every dependency. An authenticated manifest must establish that later.
/// All originals remain immutably borrowed; no source clone or secret is retained.
#[derive(Debug)]
pub struct ClosureCandidate<'nodes, 'data> {
    pub(super) root: StrongDigest,
    pub(super) nodes: &'nodes [ClosureNode<'data>],
    pub(super) edges: &'nodes [ClosureEdge],
    pub(super) order: [u8; MAX_CLOSURE_NODES],
    pub(super) original_bytes: usize,
}
impl<'nodes, 'data> ClosureCandidate<'nodes, 'data> {
    pub const fn root_digest(&self) -> StrongDigest {
        self.root
    }
    pub const fn nodes(&self) -> &'nodes [ClosureNode<'data>] {
        self.nodes
    }
    pub const fn edges(&self) -> &'nodes [ClosureEdge] {
        self.edges
    }
    pub const fn original_byte_count(&self) -> usize {
        self.original_bytes
    }

    /// Deterministic dependency-first order; equal-ready nodes sort by full digest.
    pub fn dependency_order(&self) -> impl Iterator<Item = &ClosureNode<'data>> {
        self.order[..self.nodes.len()]
            .iter()
            .map(|i| &self.nodes[*i as usize])
    }
}
