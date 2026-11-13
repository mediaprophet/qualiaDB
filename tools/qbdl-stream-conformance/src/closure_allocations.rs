use crate::allocation::measured;
use crate::crypto::network::digest::sha384;
use crate::semantic_instruments::resolve::{bounded_closure::*, dependency::DependencyKind};
#[test]
fn bounded_graph_success_and_denials_allocate_nothing() {
    let root = sha384(b"root");
    let policy = sha384(b"policy");
    let consumer = sha384(b"consumer");
    let nodes = [
        ClosureNode {
            digest: root,
            kind: DependencyKind::Ontology,
            original_bytes: b"root",
        },
        ClosureNode {
            digest: policy,
            kind: DependencyKind::LawPackage,
            original_bytes: b"policy",
        },
        ClosureNode {
            digest: consumer,
            kind: DependencyKind::Instrument,
            original_bytes: b"consumer",
        },
    ];
    let edges = [
        ClosureEdge {
            dependent: root,
            dependency: policy,
        },
        ClosureEdge {
            dependent: root,
            dependency: consumer,
        },
    ];
    let cycle = [
        edges[0],
        edges[1],
        ClosureEdge {
            dependent: policy,
            dependency: root,
        },
    ];
    let mut corrupt = nodes;
    corrupt[1].original_bytes = b"changed";
    assert_eq!(
        measured(|| {
            let candidate = verify_closure(root, &nodes, &edges).unwrap();
            assert_eq!(candidate.dependency_order().last().unwrap().digest, root);
            assert_eq!(
                verify_closure(root, &nodes, &cycle).err(),
                Some(ClosureError::Cycle)
            );
            assert_eq!(
                verify_closure(root, &corrupt, &edges).err(),
                Some(ClosureError::DigestMismatch)
            );
            assert_eq!(
                verify_closure(root, &nodes, &[]).err(),
                Some(ClosureError::UnreachableNode)
            );
        }),
        0
    );
}
