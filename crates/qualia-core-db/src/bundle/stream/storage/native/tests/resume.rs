use super::*;
#[test]
fn checkpoint_is_only_hint_then_actual_read_rewrite_rebuilds_current_marks() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, binding| {
            write(&mut store, workspace, rt, pins, bound, f, payload, 1).unwrap();
            let mut store =
                NativeSparseStore::new(store.into_staging(), binding, workspace).unwrap();
            assert!(!workspace.has_integrity_mark(0).unwrap());
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let mut op = ticket(rt, 0, 2);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                let hint = store
                    .resume_hint_for_leaf(
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
                assert!(hint.claims_leaf);
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            let mut op = ticket(rt, 0, 3);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                store
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
            }
            finish(rt, &mut op);
            assert!(workspace.has_integrity_mark(0).unwrap());
            let mut op = ticket(rt, 0, 4);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                let length = store
                    .read_verified_leaf(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1),
                        |view| {
                            assert_eq!(view.bytes(), payload);
                            view.bytes().len()
                        },
                    )
                    .unwrap();
                assert_eq!(length, payload.len());
            }
            finish(rt, &mut op);
            store
        },
    );
}
#[test]
fn sparse_holes_changed_bytes_and_nonzero_proof_padding_never_become_marks() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, _binding| {
            store
                .session
                .write_exact_at(RangeFile::Payload, payload.len() as u64 - 1, &[0])
                .unwrap();
            store
                .session
                .write_exact_at(RangeFile::Proof, 0, &[0; PROOF_SLOT_BYTES])
                .unwrap();
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let mut op = ticket(rt, 0, 1);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                assert!(store
                    .reverify_and_rewrite_leaf(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1)
                    )
                    .is_err());
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            write(&mut store, workspace, rt, pins, bound, f, payload, 2).unwrap();
            store
                .session
                .write_exact_at(RangeFile::Proof, 10, &[1])
                .unwrap();
            let mut op = ticket(rt, 0, 3);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                assert!(store
                    .read_verified_leaf(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1),
                        |_| ()
                    )
                    .is_err());
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            store
        },
    );
}
#[test]
fn torn_or_wrong_identity_checkpoint_is_not_trusted_and_hints_never_initialize_page() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, binding| {
            write(&mut store, workspace, rt, pins, bound, f, payload, 1).unwrap();
            store
                .session
                .write_exact_at(RangeFile::Checkpoint, CHECKPOINT_BANK_BYTES, &[0])
                .unwrap();
            let mut store =
                NativeSparseStore::new(store.into_staging(), binding, workspace).unwrap();
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut op = ticket(rt, 0, 2);
            let mut owner = AuthorityOwner::new();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                let hint = store
                    .resume_hint_for_leaf(
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
                assert_eq!(hint.sequence, None);
                assert!(!hint.claims_leaf);
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            store
        },
    );
}

#[test]
fn intact_unkeyed_hint_over_changed_actual_bytes_stays_unverified() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, binding| {
            write(&mut store, workspace, rt, pins, bound, f, payload, 1).unwrap();
            let zeros = vec![0; payload.len()];
            store
                .session
                .write_exact_at(RangeFile::Payload, 0, &zeros)
                .unwrap();
            let mut store =
                NativeSparseStore::new(store.into_staging(), binding, workspace).unwrap();
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let mut op = ticket(rt, 0, 2);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                assert!(
                    store
                        .resume_hint_for_leaf(
                            &mut stage,
                            &mut grant,
                            pins,
                            workspace,
                            0,
                            &mut nodes,
                            &|| 20,
                            &|| Generation(1)
                        )
                        .unwrap()
                        .claims_leaf
                );
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            let mut op = ticket(rt, 0, 3);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                assert!(store
                    .reverify_and_rewrite_leaf(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1)
                    )
                    .is_err());
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            store
        },
    );
}
#[test]
fn attacker_repaired_checkpoint_digest_cannot_splice_policy_identity() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, binding| {
            write(&mut store, workspace, rt, pins, bound, f, payload, 1).unwrap();
            let mut frame = vec![0; CHECKPOINT_BANK_BYTES as usize];
            store
                .session
                .read_exact_at(RangeFile::Checkpoint, CHECKPOINT_BANK_BYTES, &mut frame)
                .unwrap();
            // Phase-A identity policy generation at236, identity begins at16.
            frame[252..260].copy_from_slice(&2u64.to_be_bytes());
            let digest = crate::crypto::network::digest::sha384(&frame[..352]);
            frame[352..400].copy_from_slice(digest.as_bytes());
            store
                .session
                .write_exact_at(RangeFile::Checkpoint, CHECKPOINT_BANK_BYTES, &frame)
                .unwrap();
            let mut store =
                NativeSparseStore::new(store.into_staging(), binding, workspace).unwrap();
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut op = ticket(rt, 0, 2);
            let mut owner = AuthorityOwner::new();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                let hint = store
                    .resume_hint_for_leaf(
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
                assert_eq!(hint.sequence, None);
                assert!(!hint.claims_leaf);
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            store
        },
    );
}
