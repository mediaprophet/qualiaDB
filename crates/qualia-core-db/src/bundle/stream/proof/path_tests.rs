use super::*;
use crate::bundle::stream::proof::{
    empty_root, encode_proof, verify_leaf, OutboardNode, Representation, TreeBuilder, TreeSpec,
    LEAF_BYTES, MAX_SIBLINGS,
};
use crate::net::qdnf::types::StrongDigest;

#[test]
fn five_leaf_path_ranges_match_treebuilder_nodes_and_verify() {
    let bytes: Vec<u8> = (0..(LEAF_BYTES as usize * 4 + 37))
        .map(|index| (index.wrapping_mul(29) & 0xff) as u8)
        .collect();
    let spec = TreeSpec::new(
        StrongDigest([0x42; 48]),
        9,
        3,
        Representation::Plaintext,
        0,
        bytes.len() as u64,
    )
    .unwrap();
    let mut builder = TreeBuilder::new(spec);
    let mut nodes = Vec::<OutboardNode>::new();
    for leaf in bytes.chunks(LEAF_BYTES as usize) {
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

    let mut ranges = [ProofSiblingRange::EMPTY; MAX_SIBLINGS];
    let mut digests = [StrongDigest::ZERO; MAX_SIBLINGS];
    let mut encoded = [0u8; 1 + MAX_SIBLINGS * 48];
    for leaf_index in 0..spec.leaf_count() {
        let count = sibling_ranges(spec.leaf_count(), leaf_index, &mut ranges).unwrap();
        for (dst, range) in digests[..count].iter_mut().zip(&ranges[..count]) {
            *dst = nodes
                .iter()
                .find(|node| {
                    node.start_leaf == range.start_leaf && node.leaf_count == range.leaf_count
                })
                .unwrap()
                .digest;
        }
        let encoded_len = encode_proof(&digests[..count], &mut encoded).unwrap();
        assert_eq!(encoded_len, 1 + count * 48);
        let byte_start = leaf_index as usize * LEAF_BYTES as usize;
        let byte_end = (byte_start + LEAF_BYTES as usize).min(bytes.len());
        verify_leaf(
            &spec,
            root,
            leaf_index,
            &bytes[byte_start..byte_end],
            &digests[..count],
        )
        .unwrap();
    }

    assert_eq!(sibling_ranges(5, 0, &mut ranges).unwrap(), 3);
    assert_eq!(
        &ranges[..3],
        &[
            ProofSiblingRange {
                start_leaf: 1,
                leaf_count: 1,
                side: SiblingSide::Right
            },
            ProofSiblingRange {
                start_leaf: 2,
                leaf_count: 2,
                side: SiblingSide::Right
            },
            ProofSiblingRange {
                start_leaf: 4,
                leaf_count: 1,
                side: SiblingSide::Right
            },
        ]
    );
    assert_eq!(sibling_ranges(5, 4, &mut ranges).unwrap(), 1);
    assert_eq!(
        ranges[0],
        ProofSiblingRange {
            start_leaf: 0,
            leaf_count: 4,
            side: SiblingSide::Left
        }
    );
}

#[test]
fn empty_tree_has_no_leaf_path() {
    let spec = TreeSpec::new(
        StrongDigest([0x51; 48]),
        1,
        0,
        Representation::Plaintext,
        0,
        0,
    )
    .unwrap();
    let mut builder = TreeBuilder::new(spec);
    let mut observed = 0;
    let root = builder
        .finish(|_| {
            observed += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(observed, 0);
    assert!(matches!(
        sibling_ranges(0, 0, &mut []),
        Err(ProofError::Index)
    ));
    assert!(matches!(
        sibling_ranges(0, 1, &mut []),
        Err(ProofError::Index)
    ));
    assert_eq!(root, empty_root(&spec).unwrap());
}

#[test]
fn invalid_indices_capacity_and_excess_depth_refuse() {
    let mut ranges = [ProofSiblingRange::EMPTY; MAX_SIBLINGS];
    assert!(matches!(
        sibling_ranges(0, 0, &mut ranges),
        Err(ProofError::Index)
    ));
    assert!(matches!(
        sibling_ranges(5, 5, &mut ranges),
        Err(ProofError::Index)
    ));
    let before = ranges;
    assert!(matches!(
        sibling_ranges(5, 0, &mut ranges[..2]),
        Err(ProofError::Capacity)
    ));
    assert_eq!(ranges, before, "capacity refusal is transactional");
    assert!(matches!(
        sibling_ranges((1u64 << 33), 0, &mut ranges),
        Err(ProofError::Limit)
    ));
    let max_leaves = crate::bundle::stream::proof::MAX_LOGICAL_BYTES / LEAF_BYTES;
    // The rightmost leaf would need only one sibling by tree shape, but this
    // count cannot be produced by any actual bounded TreeSpec.
    assert!(matches!(
        sibling_ranges(max_leaves + 1, max_leaves, &mut ranges),
        Err(ProofError::Limit)
    ));
}
