use super::*;
use crate::NQuin;

fn quin(subject: u64, predicate: u64, object: u64, context: u64) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata: 0,
        parity: subject ^ predicate ^ object ^ context,
    }
}

#[test]
fn native_query_filters_and_binds_prefix_identity() {
    let graph = [
        quin(1, 10, 100, 7),
        quin(2, 10, 200, 7),
        quin(3, 11, 300, 7),
    ];
    let query = GraphQuery {
        context: 7,
        subject: 0,
        predicate: 10,
        object: 0,
    };
    let mut out = [NQuin::default(); 2];
    let receipt = query_graph_into(
        &graph,
        &query,
        GraphAssistPolicy::default(),
        41,
        42,
        43,
        &mut out,
    )
    .unwrap();
    assert_eq!(
        (receipt.scanned, receipt.matched, receipt.written),
        (3, 2, 2)
    );
    assert_ne!(receipt.prefix_identity, PrefixIdentity::default());
}

#[test]
fn identity_changes_with_graph_revision_and_fact_order() {
    let a = quin(1, 2, 3, 4);
    let b = quin(5, 6, 7, 4);
    assert_ne!(
        derive_prefix_identity(1, 2, 4, 9, &[a, b]),
        derive_prefix_identity(1, 2, 4, 10, &[a, b])
    );
    assert_ne!(
        derive_prefix_identity(1, 2, 4, 9, &[a, b]),
        derive_prefix_identity(1, 2, 4, 9, &[b, a])
    );
}

#[test]
fn fixed_registry_round_trips_and_replaces_without_growth() {
    let id1 = PrefixIdentity { words: [1; 4] };
    let id2 = PrefixIdentity { words: [2; 4] };
    let mut registry = PrefixPageRegistry::<1, 4>::new();
    registry
        .insert(PrefixPageSet::new(id1, &[3, 4], 32).unwrap())
        .unwrap();
    assert_eq!(registry.get(id1).unwrap().page_slice(), &[3, 4]);
    registry
        .insert(PrefixPageSet::new(id2, &[8], 16).unwrap())
        .unwrap();
    assert!(registry.get(id1).is_none());
    assert_eq!(registry.get(id2).unwrap().token_count, 16);
}

#[test]
fn sensitivity_and_parity_fail_closed() {
    let mut bad_parity = quin(1, 2, 3, 0);
    bad_parity.parity ^= 1;
    let restricted = quin(2, 2, 3, 1u64 << 56);
    let mut out = [NQuin::default(); 2];
    let receipt = query_graph_into(
        &[bad_parity, restricted],
        &GraphQuery {
            context: 0,
            subject: 0,
            predicate: 0,
            object: 0,
        },
        GraphAssistPolicy::default(),
        1,
        1,
        1,
        &mut out,
    )
    .unwrap();
    assert_eq!(receipt.written, 0);
    assert_eq!(receipt.rejected_parity, 1);
    assert_eq!(receipt.rejected_sensitivity, 1);
}
