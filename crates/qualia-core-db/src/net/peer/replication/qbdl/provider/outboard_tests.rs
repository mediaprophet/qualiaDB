use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{admit_header, admit_index, EntryScratch, ExtentLimit, IndexReadLimits},
    policy::{tests::Fixture, CLOSURE_PURPOSE},
    proof::{decode_proof, verify_leaf, OutboardNode, TreeBuilder, LEAF_BYTES, MAX_SIBLINGS},
    selection::{select_entry, SelectedPlainEntry},
};
use crate::net::qdnf::types::StrongDigest;
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

fn select(fixture: &Fixture) -> SelectedPlainEntry<'_> {
    let (pins, _, pin, publisher) = fixture.installed(CLOSURE_PURPOSE);
    let closure = fixture.bound(&pins, pin, publisher);
    let header = admit_header(
        &fixture.source()[..64],
        fixture.source().len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut index_scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &header,
        &fixture.source()[header.index_offset() as usize..],
        IndexReadLimits::default(),
        &mut index_scratch,
    )
    .unwrap();
    select_entry(
        &closure,
        fixture.body(),
        &fixture.source()[..64],
        &index,
        fixture.entry_page(),
        0,
        &mut [EntryRecord::EMPTY; 128],
        &mut vec![0; HASH_SCRATCH_BYTES],
    )
    .unwrap()
}

fn build_outboard(
    fixture: &Fixture,
    selected: SelectedPlainEntry<'_>,
) -> (Vec<OutboardNode>, crate::net::qdnf::types::StrongDigest) {
    let start = usize::try_from(selected.offset()).unwrap();
    let length = usize::try_from(selected.logical_length()).unwrap();
    let entry = &fixture.source()[start..start + length];
    let spec = selected.tree_spec().unwrap();
    let mut builder = TreeBuilder::new(spec);
    let mut nodes = Vec::new();
    for leaf in entry.chunks(LEAF_BYTES as usize) {
        builder
            .append(leaf, |node| {
                nodes.push(node);
                Ok(())
            })
            .unwrap();
    }
    let root = builder
        .finish(|node| {
            nodes.push(node);
            Ok(())
        })
        .unwrap();
    assert_eq!(root, selected.root());
    nodes.sort_unstable_by_key(|node| (node.start_leaf, node.leaf_count));
    (nodes, root)
}

fn limits(nodes: &[OutboardNode]) -> OutboardLimits {
    OutboardLimits {
        max_nodes: nodes.len(),
        max_bytes: nodes.len() * std::mem::size_of::<OutboardNode>(),
    }
}

#[test]
fn actual_non_power_of_two_outboard_proofs_verify_for_each_leaf() {
    let payload: Vec<u8> = (0..(LEAF_BYTES as usize * 4 + 19))
        .map(|index| (index.wrapping_mul(13) & 0xff) as u8)
        .collect();
    let fixture = Fixture::new_with_payload(&payload);
    let selected = select(&fixture);
    let spec = selected.tree_spec().unwrap();
    let (nodes, root) = build_outboard(&fixture, selected);
    let catalog = OutboardCatalog::new(selected, spec, root, &nodes, limits(&nodes)).unwrap();
    assert_eq!(catalog.selected().entry_id(), selected.entry_id());
    assert_eq!(catalog.tree_spec(), &spec);
    assert_eq!(catalog.root(), selected.root());
    assert_eq!(catalog.node_count(), nodes.len());

    let source_start = selected.offset() as usize;
    let entry = &fixture.source()[source_start..source_start + selected.logical_length() as usize];
    let mut encoded = [0u8; 1 + MAX_SIBLINGS * 48];
    let mut sibling_scratch = [StrongDigest::ZERO; MAX_SIBLINGS];
    let mut decoded = [StrongDigest::ZERO; MAX_SIBLINGS];
    for index in 0..spec.leaf_count() {
        let encoded_len = catalog
            .proof_into(index, &mut encoded, &mut sibling_scratch)
            .unwrap();
        let proof = decode_proof(&encoded[..encoded_len], &mut decoded).unwrap();
        let start = index as usize * LEAF_BYTES as usize;
        let end = (start + LEAF_BYTES as usize).min(entry.len());
        verify_leaf(&spec, root, index, &entry[start..end], proof).unwrap();
    }

    assert_zero_alloc("qbdl_outboard_proof_lookup", || {
        catalog
            .proof_into(0, &mut encoded, &mut sibling_scratch)
            .unwrap();
    });
}

#[test]
fn empty_tree_binds_canonical_root_and_has_no_proof_request() {
    let fixture = Fixture::new_with_payload(&[]);
    let selected = select(&fixture);
    let spec = selected.tree_spec().unwrap();
    assert_eq!(spec.leaf_count(), 0);
    let mut builder = TreeBuilder::new(spec);
    let mut observed = 0usize;
    let root = builder
        .finish(|_| {
            observed += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(observed, 0);
    assert_eq!(root, selected.root());
    let nodes = [];
    let catalog = OutboardCatalog::new(
        selected,
        spec,
        root,
        &nodes,
        OutboardLimits {
            max_nodes: 0,
            max_bytes: 0,
        },
    )
    .unwrap();
    assert_eq!(catalog.node_count(), 0);
    assert!(matches!(
        catalog.proof_into(0, &mut [0; 1], &mut []),
        Err(OutboardError::Proof(
            crate::bundle::stream::proof::ProofError::Index
        ))
    ));
}

#[test]
fn bounds_identity_duplicates_missing_nodes_and_capacity_refuse() {
    let payload = vec![0x5a; LEAF_BYTES as usize * 4 + 3];
    let fixture = Fixture::new_with_payload(&payload);
    let selected = select(&fixture);
    let spec = selected.tree_spec().unwrap();
    let (nodes, root) = build_outboard(&fixture, selected);

    let good_limits = limits(&nodes);
    assert!(matches!(
        OutboardCatalog::new(
            selected,
            spec,
            root,
            &nodes,
            OutboardLimits {
                max_nodes: nodes.len() - 1,
                ..good_limits
            }
        ),
        Err(OutboardError::Limit)
    ));
    assert!(matches!(
        OutboardCatalog::new(
            selected,
            spec,
            root,
            &nodes,
            OutboardLimits {
                max_bytes: good_limits.max_bytes - 1,
                ..good_limits
            }
        ),
        Err(OutboardError::Limit)
    ));
    let wrong_spec = crate::bundle::stream::proof::TreeSpec::new(
        spec.context(),
        spec.entry_id().wrapping_add(1),
        spec.generation(),
        spec.representation(),
        spec.key_epoch(),
        spec.logical_length(),
    )
    .unwrap();
    assert!(matches!(
        OutboardCatalog::new(selected, wrong_spec, root, &nodes, good_limits),
        Err(OutboardError::Identity)
    ));
    assert!(matches!(
        OutboardCatalog::new(
            selected,
            spec,
            StrongDigest([0x99; 48]),
            &nodes,
            good_limits
        ),
        Err(OutboardError::Identity)
    ));

    let overflow = [OutboardNode {
        start_leaf: u64::MAX,
        leaf_count: 2,
        digest: StrongDigest::ZERO,
    }];
    assert!(matches!(
        OutboardCatalog::new(selected, spec, root, &overflow, limits(&overflow)),
        Err(OutboardError::Bounds)
    ));

    let mut duplicate = nodes.clone();
    duplicate.push(nodes[0]);
    duplicate.sort_unstable_by_key(|node| (node.start_leaf, node.leaf_count));
    assert!(matches!(
        OutboardCatalog::new(selected, spec, root, &duplicate, limits(&duplicate)),
        Err(OutboardError::DuplicateNode)
    ));
    let mut unsorted = nodes.clone();
    unsorted.swap(0, 1);
    assert!(matches!(
        OutboardCatalog::new(selected, spec, root, &unsorted, limits(&unsorted)),
        Err(OutboardError::UnsortedNodes)
    ));

    let missing_range = crate::bundle::stream::proof::ProofSiblingRange {
        start_leaf: 1,
        leaf_count: 1,
        side: crate::bundle::stream::proof::SiblingSide::Right,
    };
    let missing_nodes: Vec<_> = nodes
        .iter()
        .copied()
        .filter(|node| {
            node.start_leaf != missing_range.start_leaf
                || node.leaf_count != missing_range.leaf_count
        })
        .collect();
    let missing_catalog =
        OutboardCatalog::new(selected, spec, root, &missing_nodes, limits(&missing_nodes)).unwrap();
    assert!(matches!(
        missing_catalog.proof_into(
            0,
            &mut [0; 1 + MAX_SIBLINGS * 48],
            &mut [StrongDigest::ZERO; MAX_SIBLINGS]
        ),
        Err(OutboardError::MissingNode)
    ));

    let catalog = OutboardCatalog::new(selected, spec, root, &nodes, good_limits).unwrap();
    assert!(matches!(
        catalog.proof_into(
            spec.leaf_count(),
            &mut [0; 1537],
            &mut [StrongDigest::ZERO; MAX_SIBLINGS]
        ),
        Err(OutboardError::Proof(
            crate::bundle::stream::proof::ProofError::Index
        ))
    ));
    assert!(matches!(
        catalog.proof_into(0, &mut [0; 1], &mut [StrongDigest::ZERO; MAX_SIBLINGS]),
        Err(OutboardError::Capacity)
    ));
    assert!(matches!(
        catalog.proof_into(0, &mut [0; 1537], &mut []),
        Err(OutboardError::Capacity)
    ));
}
