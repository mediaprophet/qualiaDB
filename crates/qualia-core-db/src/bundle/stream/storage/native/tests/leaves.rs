use super::fixtures::fixture_with_payload;
use super::*;
use crate::bundle::stream::proof::{encode_proof, TreeBuilder, LEAF_BYTES};

#[test]
fn signed_legitimate_zero_leaf_is_explicitly_written_and_reverified_rewritten() {
    fixture_with_payload(
        &[0; 7],
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, binding| {
            assert_eq!(payload, &[0; 7]);
            write(&mut store, workspace, rt, pins, bound, f, payload, 1).unwrap();
            assert_eq!(
                store
                    .session
                    .observed_file_length(RangeFile::Payload)
                    .unwrap(),
                7
            );
            let mut store =
                NativeSparseStore::new(store.into_staging(), binding, workspace).unwrap();
            assert!(!workspace.has_integrity_mark(0).unwrap());
            let mut op = ticket(rt, 0, 2);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                let observed = store
                    .reverify_and_rewrite_leaf(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1),
                    )
                    .unwrap();
                assert_eq!(observed.sequence, 1);
            }
            finish(rt, &mut op);
            assert!(workspace.has_integrity_mark(0).unwrap());
            store
        },
    );
}
#[test]
fn two_real_signed_leaves_use_canonical_outboard_proofs_and_exact_short_final_byte() {
    let mut data = vec![13; LEAF_BYTES as usize + 1];
    data[LEAF_BYTES as usize] = 99;
    fixture_with_payload(
        &data,
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, _binding| {
            let tree = store.identity().tree_spec();
            let mut builder = TreeBuilder::new(tree);
            let mut outboard = Vec::new();
            for bytes in payload.chunks(LEAF_BYTES as usize) {
                builder
                    .append(bytes, |node| {
                        outboard.push(node);
                        Ok(())
                    })
                    .unwrap();
            }
            assert_eq!(
                builder
                    .finish(|node| {
                        outboard.push(node);
                        Ok(())
                    })
                    .unwrap(),
                store.identity().fields().root
            );
            for leaf in 0..2u64 {
                // Two leaves need exactly the other leaf node emitted by the real builder.
                let other = outboard
                    .iter()
                    .find(|n| n.start_leaf == 1 - leaf && n.leaf_count == 1)
                    .unwrap()
                    .digest;
                let mut proof = [0; 49];
                assert_eq!(encode_proof(&[other], &mut proof).unwrap(), 49);
                let offset = (leaf * LEAF_BYTES) as usize;
                let bytes = &payload[offset..(offset + LEAF_BYTES as usize).min(payload.len())];
                let mut op = ticket(rt, bytes.len(), leaf as u8 + 1);
                let mut scratch = vec![0; READ_SCRATCH_BYTES];
                let mut nodes = [StrongDigest::ZERO; 32];
                let mut owner = AuthorityOwner::new();
                {
                    let mut grant =
                        adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                    let mut stage = rt
                        .storage_stage(&op, bytes, &proof, &mut scratch, 20, Generation(1))
                        .unwrap();
                    store
                        .write_leaf(
                            &mut stage,
                            &mut grant,
                            pins,
                            workspace,
                            leaf,
                            &mut nodes,
                            &|| 20,
                            &|| Generation(1),
                        )
                        .unwrap();
                }
                finish(rt, &mut op);
            }
            assert!(workspace.has_integrity_mark(0).unwrap());
            assert!(workspace.has_integrity_mark(1).unwrap());
            assert_eq!(
                store
                    .session
                    .observed_file_length(RangeFile::Payload)
                    .unwrap(),
                LEAF_BYTES + 1
            );
            let mut op = ticket(rt, 0, 3);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                store
                    .read_verified_leaf(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        1,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1),
                        |view| {
                            assert_eq!(view.bytes(), &[99]);
                            assert_eq!(view.logical_length(), 1)
                        },
                    )
                    .unwrap();
            }
            finish(rt, &mut op);
            store
        },
    );
}
