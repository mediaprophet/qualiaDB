use super::*;
use crate::storage_driver::ranges::recovery::tests::{locator, material};
#[test]
fn untrusted_retained_files_reverify_rewrite_into_distinct_actual_charged_destination() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, driver, quota, binding| {
            let layout = StoreLayout::for_binding(&binding).unwrap();
            let allowance = layout.aggregate_allowance().unwrap();
            drop(quota.admit_staging([4; 32], allowance).unwrap());
            let parent = store.session.staging_path().parent().unwrap().to_path_buf();
            // Actual untrusted file fixture, not a producer-crash qualification.
            let old = material(&parent, [4; 32], true, false);
            std::fs::write(old.path().join("payload"), payload).unwrap();
            std::fs::write(old.path().join("proof"), vec![0; PROOF_SLOT_BYTES]).unwrap();
            let scope = quota.recovery_root_scope().unwrap();
            let item = locator(driver, &scope, [4; 32]);
            let mut source = rt
                .acquire_recovery_source(driver, &scope, item, layout.range_limits())
                .unwrap();
            assert!(!source.checkpoint_available());
            assert_eq!(rt.recovery_inflight_ceiling(), 28);
            let charge = quota.charged_usage();
            assert_eq!(charge, (allowance * 2, 2));
            let mut op = ticket(rt, 0, 1);
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
                    .recover_leaf_from(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        &mut source,
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
            assert_eq!(
                store
                    .session
                    .observed_file_length(RangeFile::Payload)
                    .unwrap(),
                payload.len() as u64
            );
            assert!(scratch[..PROOF_SLOT_BYTES].iter().all(|b| *b == 0));
            rt.release_recovery_source(source).unwrap();
            drop(old);
            assert_eq!(quota.charged_usage(), charge);
            store
        },
    );
}
#[test]
fn corrupted_old_material_never_marks_or_grows_destination_and_clears_transient_bytes() {
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, driver, quota, binding| {
            let layout = StoreLayout::for_binding(&binding).unwrap();
            drop(
                quota
                    .admit_staging([4; 32], layout.aggregate_allowance().unwrap())
                    .unwrap(),
            );
            let parent = store.session.staging_path().parent().unwrap().to_path_buf();
            let old = material(&parent, [4; 32], true, false);
            std::fs::write(old.path().join("payload"), vec![0; payload.len()]).unwrap();
            std::fs::write(old.path().join("proof"), vec![0; PROOF_SLOT_BYTES]).unwrap();
            let scope = quota.recovery_root_scope().unwrap();
            let mut source = rt
                .acquire_recovery_source(
                    driver,
                    &scope,
                    locator(driver, &scope, [4; 32]),
                    layout.range_limits(),
                )
                .unwrap();
            let mut op = ticket(rt, 0, 1);
            let mut scratch = vec![9; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            let charge = quota.charged_usage();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                    .unwrap();
                assert!(store
                    .recover_leaf_from(
                        &mut stage,
                        &mut grant,
                        pins,
                        workspace,
                        &mut source,
                        0,
                        &mut nodes,
                        &|| 20,
                        &|| Generation(1)
                    )
                    .is_err());
            }
            finish(rt, &mut op);
            assert!(!workspace.has_integrity_mark(0).unwrap());
            assert_eq!(
                store
                    .session
                    .observed_file_length(RangeFile::Payload)
                    .unwrap(),
                0
            );
            assert!(scratch[..PROOF_SLOT_BYTES].iter().all(|b| *b == 0));
            assert!(
                scratch[WRITE_SCRATCH_BYTES..WRITE_SCRATCH_BYTES + payload.len()]
                    .iter()
                    .all(|b| *b == 0)
            );
            assert_eq!(quota.charged_usage(), charge);
            rt.release_recovery_source(source).unwrap();
            store
        },
    );
}
#[test]
fn actual_recovery_hot_path_has_zero_raw_allocation_calls() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, driver, quota, binding| {
            let layout = StoreLayout::for_binding(&binding).unwrap();
            drop(
                quota
                    .admit_staging([4; 32], layout.aggregate_allowance().unwrap())
                    .unwrap(),
            );
            let parent = store.session.staging_path().parent().unwrap().to_path_buf();
            let old = material(&parent, [4; 32], true, false);
            std::fs::write(old.path().join("payload"), payload).unwrap();
            std::fs::write(old.path().join("proof"), vec![0; PROOF_SLOT_BYTES]).unwrap();
            let scope = quota.recovery_root_scope().unwrap();
            let mut source = rt
                .acquire_recovery_source(
                    driver,
                    &scope,
                    locator(driver, &scope, [4; 32]),
                    layout.range_limits(),
                )
                .unwrap();
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            for n in 1..=2 {
                let mut op = ticket(rt, 0, n);
                {
                    let mut grant =
                        adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                    let mut stage = rt
                        .storage_stage(&op, &[], &[], &mut scratch, 20, Generation(1))
                        .unwrap();
                    if n == 1 {
                        store
                            .recover_leaf_from(
                                &mut stage,
                                &mut grant,
                                pins,
                                workspace,
                                &mut source,
                                0,
                                &mut nodes,
                                &|| 20,
                                &|| Generation(1),
                            )
                            .unwrap();
                    } else {
                        let guard = AllocGuard::begin("actual-source-recovery", true);
                        store
                            .recover_leaf_from(
                                &mut stage,
                                &mut grant,
                                pins,
                                workspace,
                                &mut source,
                                0,
                                &mut nodes,
                                &|| 20,
                                &|| Generation(1),
                            )
                            .unwrap();
                        guard
                            .check()
                            .expect("actual source proof/rewrite must not allocate");
                    }
                }
                finish(rt, &mut op);
            }
            rt.release_recovery_source(source).unwrap();
            store
        },
    );
}
