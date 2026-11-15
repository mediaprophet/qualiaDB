use super::*;

#[test]
fn storage_control_rechecks_with_live_payload_and_proof_borrows() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let rows = [spec(1)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        let _credit = ready(rt, &ticket);
        let payload = [7; 16];
        let proof = [8; 16];
        let mut scratch = [0; 16];
        {
            let mut stage = rt
                .storage_stage(&ticket, &payload, &proof, &mut scratch, 0, Generation(1))
                .unwrap();
            stage
                .with_buffers(0, Generation(1), |control, buffers| {
                    let live_payload = &buffers.payload[..8];
                    let live_proof = &buffers.proof[..8];
                    assert_eq!(live_payload.as_ptr(), payload.as_ptr());
                    control.recheck(1, Generation(1)).unwrap();
                    buffers.scratch[..8].copy_from_slice(live_payload);
                    control.cancel().unwrap();
                    assert_eq!(control.recheck(2, Generation(1)), Err(QdnfError::Cancelled));
                    // Borrowed data remains readable but grants no accepted effect.
                    assert_eq!(live_proof, &[8; 8]);
                })
                .unwrap();
            assert!(
                stage
                    .with_buffers(2, Generation(1), |_, _| panic!("late effect"))
                    .is_err()
            );
        }
        assert_eq!(&scratch[..8], &[7; 8]);
        assert_eq!(rt.live_transfers(), 1);
        let key = ticket.completion_key();
        assert_eq!(
            rt.complete_io(&mut ticket, key, 2),
            Ok(FinishOutcome::Cancelled)
        );
    });
}

#[test]
fn storage_control_rechecks_expiration_and_changed_policy_between_effects() {
    for (now, generation, error, outcome) in [
        (
            100,
            Generation(1),
            QdnfError::Expired,
            FinishOutcome::Expired,
        ),
        (1, Generation(2), QdnfError::Revoked, FinishOutcome::Revoked),
    ] {
        with_runtime(RuntimeProfile::Constrained, |rt| {
            let rows = [spec(1)];
            let plan = ColdPlan::compile(&rows).unwrap();
            let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
            let _credit = ready(rt, &ticket);
            let mut scratch = [0; 16];
            {
                let mut stage = rt
                    .storage_stage(&ticket, &[7; 16], &[8; 16], &mut scratch, 0, Generation(1))
                    .unwrap();
                stage
                    .with_buffers(0, Generation(1), |control, buffers| {
                        let view = &buffers.payload[..];
                        control.recheck(0, Generation(1)).unwrap();
                        assert_eq!(view[0], 7);
                        assert_eq!(control.recheck(now, generation), Err(error));
                        assert_eq!(view[15], 7);
                    })
                    .unwrap();
            }
            assert_eq!(rt.live_transfers(), 1);
            let key = ticket.completion_key();
            assert_eq!(rt.complete_io(&mut ticket, key, now), Ok(outcome));
        });
    }
}

#[test]
fn typed_proof_scratch_counts_against_the_same_storage_charge() {
    use crate::net::qdnf::types::StrongDigest;
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let mut row = spec(1);
        row.requirements.storage.bytes = 16 + 32 * 48;
        let rows = [row];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        let _credit = ready(rt, &ticket);
        let mut scratch = [0; 17];
        let mut nodes = [StrongDigest([0; 48]); 33];
        {
            let mut stage = rt
                .storage_stage(&ticket, &[7; 16], &[], &mut scratch, 0, Generation(1))
                .unwrap();
            assert_eq!(
                stage.with_proof_buffers(0, Generation(1), &mut nodes[..32], |_, _| ()),
                Err(QdnfError::Capacity)
            );
        }
        {
            let mut stage = rt
                .storage_stage(&ticket, &[7; 16], &[], &mut scratch[..16], 0, Generation(1))
                .unwrap();
            assert_eq!(
                stage.with_proof_buffers(0, Generation(1), &mut nodes, |_, _| ()),
                Err(QdnfError::Capacity)
            );
            stage
                .with_proof_buffers(0, Generation(1), &mut nodes[..32], |control, buffers| {
                    assert_eq!(buffers.siblings.len(), 32);
                    control.recheck(1, Generation(1)).unwrap();
                    buffers.siblings[0] = StrongDigest([9; 48]);
                    buffers.bytes.scratch[0] = buffers.bytes.payload[0];
                })
                .unwrap();
        }
        assert_eq!(nodes[0], StrongDigest([9; 48]));
        assert_eq!(scratch[0], 7);
        let key = ticket.completion_key();
        rt.complete_io(&mut ticket, key, 1).unwrap();
    });
}
