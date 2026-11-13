use super::*;
#[test]
fn actual_gated_native_write_success_and_preflight_reject_have_zero_raw_allocations() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    fixture(
        |mut store, workspace, rt, pins, bound, f, payload, _driver, _quota, _binding| {
            write(&mut store, workspace, rt, pins, bound, f, payload, 1).unwrap(); // warm actual owners/native paths.
            let mut op = ticket(rt, payload.len(), 2);
            let mut scratch = vec![0; READ_SCRATCH_BYTES];
            let mut nodes = [StrongDigest::ZERO; 32];
            let mut owner = AuthorityOwner::new();
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, payload, &[0], &mut scratch, 20, Generation(1))
                    .unwrap();
                let positive = AllocGuard::begin("native-positive", true);
                let mut memory = Vec::<u8>::with_capacity(64);
                std::hint::black_box(&mut memory);
                drop(memory);
                assert!(positive.check().is_err());
                let guard = AllocGuard::begin("native-gated-write", true);
                store
                    .write_leaf(
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
                guard
                    .check()
                    .expect("gated real native write must not allocate");
            }
            finish(rt, &mut op);
            let mut op = ticket(rt, payload.len(), 3);
            {
                let mut grant =
                    adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
                let mut stage = rt
                    .storage_stage(&op, payload, &[1], &mut scratch, 20, Generation(1))
                    .unwrap();
                let guard = AllocGuard::begin("native-malformed-proof", true);
                assert!(store
                    .write_leaf(
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
                guard.check().expect("native rejection must not allocate");
            }
            finish(rt, &mut op);
            store
        },
    );
}
