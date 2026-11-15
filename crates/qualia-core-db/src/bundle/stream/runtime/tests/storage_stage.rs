use super::*;
#[test]
fn storage_stage_borrows_existing_credit_and_retains_charge_until_completion() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let rows = [spec(1)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        let mut scratch = [0; 16];
        let payload = [7; 16];
        let proof = [8; 16];
        assert!(matches!(
            rt.storage_stage(&ticket, &payload, &proof, &mut scratch, 0, Generation(1)),
            Err(QdnfError::WouldBlock)
        ));
        let _credit = ready(rt, &ticket);
        for (p, q, n) in [
            (&[0; 17][..], &proof[..], 16),
            (&payload[..], &[0; 17][..], 16),
            (&payload[..], &proof[..], 17),
        ] {
            let mut extra = [0; 17];
            assert!(matches!(
                rt.storage_stage(&ticket, p, q, &mut extra[..n], 0, Generation(1)),
                Err(QdnfError::Capacity)
            ));
        }
        {
            let mut stage = rt
                .storage_stage(&ticket, &payload, &proof, &mut scratch, 0, Generation(1))
                .unwrap();
            let (p, q, s) = stage.buffers(0, Generation(1)).unwrap();
            assert_eq!(p.as_ptr(), payload.as_ptr());
            assert_eq!(q.as_ptr(), proof.as_ptr());
            s[0] = 9;
            stage.cancel().unwrap();
            assert_eq!(stage.recheck(1, Generation(1)), Err(QdnfError::Cancelled));
            assert!(stage.buffers(1, Generation(1)).is_err());
        }
        assert_eq!(scratch[0], 9);
        assert_eq!(rt.live_transfers(), 1);
        let key = ticket.completion_key();
        assert_eq!(
            rt.complete_io(&mut ticket, key, 1),
            Ok(FinishOutcome::Cancelled)
        );
    });
}
#[test]
fn storage_stage_checks_fresh_deadline_and_policy_before_effects() {
    for (time, policy, error, outcome) in [
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
                    .storage_stage(&ticket, &[0; 16], &[], &mut scratch, 0, Generation(1))
                    .unwrap();
                assert_eq!(stage.recheck(time, policy), Err(error));
            }
            assert_eq!(rt.live_transfers(), 1);
            let key = ticket.completion_key();
            assert_eq!(rt.complete_io(&mut ticket, key, time), Ok(outcome));
        });
    }
}
