use super::*;
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
use crate::semantic_instruments::resolve::dependency::DependencyKind;

fn node(bytes: &[u8]) -> ClosureNode<'_> {
    ClosureNode {
        digest: sha384(bytes),
        kind: DependencyKind::Ontology,
        original_bytes: bytes,
    }
}
fn edge(a: ClosureNode<'_>, b: ClosureNode<'_>) -> ClosureEdge {
    ClosureEdge {
        dependent: a.digest,
        dependency: b.digest,
    }
}

#[test]
fn diamond_preserves_originals_and_dependency_order_under_permutation() {
    let nodes = [
        node(b"root"),
        node(b"left"),
        node(b"right"),
        node(b"shared"),
    ];
    let edges = [
        edge(nodes[0], nodes[1]),
        edge(nodes[0], nodes[2]),
        edge(nodes[1], nodes[3]),
        edge(nodes[2], nodes[3]),
    ];
    let candidate = verify_closure(nodes[0].digest, &nodes, &edges).unwrap();
    assert_eq!(candidate.root_digest(), nodes[0].digest);
    assert_eq!(candidate.original_byte_count(), 19);
    assert_eq!(
        candidate.nodes()[0].original_bytes.as_ptr(),
        nodes[0].original_bytes.as_ptr()
    );
    assert_eq!(candidate.edges(), &edges);
    let order: Vec<_> = candidate.dependency_order().map(|n| n.digest).collect();
    assert_eq!(order.first(), Some(&nodes[3].digest));
    assert_eq!(order.last(), Some(&nodes[0].digest));
    let shuffled_nodes = [nodes[2], nodes[3], nodes[0], nodes[1]];
    let shuffled_edges = [edges[3], edges[1], edges[0], edges[2]];
    let shuffled = verify_closure(nodes[0].digest, &shuffled_nodes, &shuffled_edges).unwrap();
    assert_eq!(
        order,
        shuffled
            .dependency_order()
            .map(|n| n.digest)
            .collect::<Vec<_>>()
    );
}

#[test]
fn missing_duplicate_disconnected_and_cycle_inputs_refuse() {
    let nodes = [node(b"r"), node(b"a"), node(b"b")];
    let chain = [edge(nodes[0], nodes[1]), edge(nodes[1], nodes[2])];
    let duplicate_nodes = [nodes[0], nodes[0]];
    assert!(matches!(
        verify_closure(nodes[0].digest, &duplicate_nodes, &[]),
        Err(ClosureError::DuplicateNode)
    ));
    assert!(matches!(
        verify_closure(sha384(b"missing"), &nodes, &chain),
        Err(ClosureError::MissingRoot)
    ));
    assert!(matches!(
        verify_closure(nodes[0].digest, &nodes, &chain[..1]),
        Err(ClosureError::UnreachableNode)
    ));
    let duplicate = [chain[0], chain[0], chain[1]];
    assert!(matches!(
        verify_closure(nodes[0].digest, &nodes, &duplicate),
        Err(ClosureError::DuplicateEdge)
    ));
    let self_edge = [edge(nodes[0], nodes[0])];
    assert!(matches!(
        verify_closure(nodes[0].digest, &nodes, &self_edge),
        Err(ClosureError::Cycle)
    ));
    let cycle = [chain[0], chain[1], edge(nodes[2], nodes[1])];
    assert!(matches!(
        verify_closure(nodes[0].digest, &nodes, &cycle),
        Err(ClosureError::Cycle)
    ));
    let mut missing = nodes[2].digest;
    missing.0[47] ^= 1;
    let wrong_endpoint = [
        chain[0],
        ClosureEdge {
            dependent: nodes[1].digest,
            dependency: missing,
        },
    ];
    assert!(matches!(
        verify_closure(nodes[0].digest, &nodes, &wrong_endpoint),
        Err(ClosureError::MissingEndpoint)
    ));
}

#[test]
fn exact_hash_and_zero_digest_checks_do_not_mint_authority() {
    let mut nodes = [node(b"original")];
    nodes[0].original_bytes = b"changed";
    assert!(matches!(
        verify_closure(nodes[0].digest, &nodes, &[]),
        Err(ClosureError::DigestMismatch)
    ));
    nodes[0].digest = StrongDigest::ZERO;
    assert!(matches!(
        verify_closure(sha384(b"original"), &nodes, &[]),
        Err(ClosureError::ZeroDigest)
    ));
    let mut nodes = [node(b"")];
    assert!(verify_closure(nodes[0].digest, &nodes, &[]).is_ok());
    // Kind is candidate metadata only; changing it cannot become authenticated.
    nodes[0].kind = DependencyKind::LawPackage;
    let candidate = verify_closure(nodes[0].digest, &nodes, &[]).unwrap();
    assert_eq!(candidate.nodes()[0].kind, DependencyKind::LawPackage);
}

#[test]
fn sixteen_nodes_and_dense_dag_fit_but_seventeen_or_257_edges_refuse() {
    let originals: [[u8; 1]; 17] = core::array::from_fn(|i| [i as u8]);
    let nodes: [ClosureNode<'_>; 17] = core::array::from_fn(|i| node(&originals[i]));
    let mut edges = Vec::new();
    for dependent in 1..16 {
        for dependency in 0..dependent {
            edges.push(edge(nodes[dependent], nodes[dependency]));
        }
    }
    let candidate = verify_closure(nodes[15].digest, &nodes[..16], &edges).unwrap();
    assert_eq!(candidate.dependency_order().count(), 16);
    assert_eq!(candidate.original_byte_count(), 16);
    assert!(matches!(
        verify_closure(nodes[16].digest, &nodes, &edges),
        Err(ClosureError::NodeLimit)
    ));
    let too_many = vec![edges[0]; MAX_CLOSURE_EDGES + 1];
    assert!(matches!(
        verify_closure(nodes[15].digest, &nodes[..16], &too_many),
        Err(ClosureError::EdgeLimit)
    ));
    assert!(matches!(
        verify_closure(nodes[0].digest, &[], &[]),
        Err(ClosureError::NodeLimit)
    ));
}

#[test]
fn original_byte_budget_is_cumulative_and_accepts_exact_boundary() {
    let large = vec![3; MAX_CLOSURE_ORIGINAL_BYTES];
    let first = node(&large);
    let nodes = [first];
    assert_eq!(
        verify_closure(first.digest, &nodes, &[])
            .unwrap()
            .original_byte_count(),
        MAX_CLOSURE_ORIGINAL_BYTES
    );
    let nodes = [first, node(b"x")];
    let edges = [edge(nodes[0], nodes[1])];
    assert!(matches!(
        verify_closure(first.digest, &nodes, &edges),
        Err(ClosureError::ByteLimit)
    ));
    let oversized = vec![4; MAX_CLOSURE_ORIGINAL_BYTES + 1];
    let nodes = [node(&oversized)];
    assert!(matches!(
        verify_closure(nodes[0].digest, &nodes, &[]),
        Err(ClosureError::ByteLimit)
    ));
}
