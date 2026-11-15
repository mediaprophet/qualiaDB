use super::*;

#[test]
fn success_reserves_every_component_before_one_shot_credit() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let base = rt.parent_used();
        let rows = [spec(1)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        assert!(rt.parent_used().host.bytes > base.host.bytes);
        assert_eq!(rt.leased_slots(), 2);
        assert_eq!(
            rt.issue_credit(&ticket, 0, Generation(1)).err(),
            Some(QdnfError::WouldBlock)
        );
        let request = ready(rt, &ticket);
        assert_eq!(request.bytes(), 16);
        assert_eq!(
            rt.issue_credit(&ticket, 0, Generation(1)).err(),
            Some(QdnfError::Replay)
        );
        assert_eq!(
            rt.complete_io(&mut ticket, request.completion_key(), 1),
            Ok(FinishOutcome::Released)
        );
        assert_eq!(rt.parent_used(), base);
        assert_eq!(rt.leased_slots(), 1);
        assert_eq!(
            rt.complete_io(&mut ticket, request.completion_key(), 1),
            Err(QdnfError::DoubleRelease)
        );
    });
}
#[test]
fn cancel_and_timeout_retain_io_until_late_completion() {
    for timeout in [false, true] {
        with_runtime(RuntimeProfile::Constrained, |rt| {
            let mut row = spec(2);
            row.deadline = 10;
            let rows = [row];
            let plan = ColdPlan::compile(&rows).unwrap();
            let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
            let request = ready(rt, &ticket);
            let held = rt.parent_used();
            assert_eq!(
                request.cancel_epoch(),
                crate::net::peer::runtime::CancelEpoch::ZERO
            );
            if timeout {
                assert_eq!(rt.expire(10), Ok(1));
                assert_eq!(rt.expire(10), Ok(0));
            } else {
                rt.cancel(&ticket).unwrap();
                rt.cancel(&ticket).unwrap();
            }
            assert_eq!(rt.cancel_epoch(&ticket).unwrap().0, 1);
            assert_eq!(rt.parent_used(), held);
            assert_eq!(rt.leased_slots(), 2);
            assert_eq!(rt.close(), Err(QdnfError::WouldBlock));
            assert!(rt.issue_credit(&ticket, 10, Generation(1)).is_err());
            assert_eq!(
                rt.complete_io(&mut ticket, request.completion_key(), 10),
                Ok(if timeout {
                    FinishOutcome::Expired
                } else {
                    FinishOutcome::Cancelled
                })
            );
            assert_eq!(rt.leased_slots(), 1);
        });
    }
}
#[test]
fn expired_and_revoked_admission_and_duplicate_operation_leave_no_charge() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let rows = [spec(3)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let base = rt.parent_used();
        assert_eq!(
            rt.admit(&plan, 0, 100, Generation(1)).err(),
            Some(QdnfError::Expired)
        );
        assert_eq!(
            rt.admit(&plan, 0, 0, Generation(2)).err(),
            Some(QdnfError::Revoked)
        );
        assert_eq!(rt.parent_used(), base);
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        let held = rt.parent_used();
        assert_eq!(
            rt.admit(&plan, 0, 0, Generation(1)).err(),
            Some(QdnfError::Conflict)
        );
        assert_eq!(rt.parent_used(), held);
        assert_eq!(rt.leased_slots(), 2);
        rt.cancel(&ticket).unwrap();
        let key = ticket.completion_key();
        assert_eq!(
            rt.complete_io(&mut ticket, key, 0),
            Err(QdnfError::WouldBlock)
        );
        assert_eq!(rt.close(), Err(QdnfError::WouldBlock));
        assert!(matches!(
            rt.poll(0),
            Ok(RuntimePoll::Cancelled(_, CancelReason::Cancelled))
        ));
        rt.complete_io(&mut ticket, key, 0).unwrap();
    });
}

#[test]
fn stale_completion_cannot_release_reused_slot_or_other_runtime() {
    with_runtime(RuntimeProfile::Constrained, |a| {
        with_runtime(RuntimeProfile::Constrained, |b| {
            let rows = [spec(4)];
            let plan = ColdPlan::compile(&rows).unwrap();
            let mut first = a.admit(&plan, 0, 0, Generation(1)).unwrap();
            let old = ready(a, &first).completion_key();
            a.complete_io(&mut first, old, 0).unwrap();
            let mut second = a.admit(&plan, 0, 0, Generation(1)).unwrap();
            let new = ready(a, &second).completion_key();
            let held = a.parent_used();
            assert_eq!(
                a.complete_io(&mut second, old, 0),
                Err(QdnfError::StaleGeneration)
            );
            assert_eq!(a.parent_used(), held);
            let mut foreign = b.admit(&plan, 0, 0, Generation(1)).unwrap();
            let key = ready(b, &foreign).completion_key();
            assert_eq!(
                b.complete_io(&mut foreign, new, 0),
                Err(QdnfError::StaleGeneration)
            );
            b.complete_io(&mut foreign, key, 0).unwrap();
            a.complete_io(&mut second, new, 0).unwrap();
        });
    });
}
#[test]
fn canceled_queued_record_must_be_polled_before_slot_reuse() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let rows = [spec(5)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut old = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        let stale = old.completion_key();
        rt.cancel(&old).unwrap();
        let held = rt.parent_used();
        assert_eq!(
            rt.complete_io(&mut old, stale, 0),
            Err(QdnfError::WouldBlock)
        );
        assert_eq!(rt.parent_used(), held);
        assert!(matches!(
            rt.poll(0),
            Ok(RuntimePoll::Cancelled(_, CancelReason::Cancelled))
        ));
        rt.complete_io(&mut old, stale, 0).unwrap();
        let mut current = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        let request = ready(rt, &current);
        assert_ne!(request.completion_key(), stale);
        assert_eq!(
            rt.complete_io(&mut current, stale, 0),
            Err(QdnfError::StaleGeneration)
        );
        rt.complete_io(&mut current, request.completion_key(), 0)
            .unwrap();
    });
}

#[test]
fn scheduler_capacity_rejects_atomically_and_profiles_do_not_claim_sixty_four_io() {
    for profile in [RuntimeProfile::Constrained, RuntimeProfile::Desktop] {
        with_runtime(profile, |rt| {
            let rows: [TransferSpec; 64] = core::array::from_fn(|i| spec((i + 1) as u8));
            let plan = ColdPlan::compile(&rows).unwrap();
            assert_eq!(plan.len(), 64);
            let mut tickets: [Option<TransferTicket>; 8] = core::array::from_fn(|_| None);
            for (i, t) in tickets.iter_mut().enumerate() {
                *t = Some(rt.admit(&plan, i, 0, Generation(1)).unwrap());
            }
            let held = rt.parent_used();
            let bytes = rt.charged_window_bytes();
            assert_eq!(
                rt.admit(&plan, 8, 0, Generation(1)).err(),
                Some(QdnfError::Capacity)
            );
            assert_eq!(rt.parent_used(), held);
            assert_eq!(rt.charged_window_bytes(), bytes);
            assert_eq!(rt.leased_slots(), 9);
            assert_eq!(
                rt.supported_inflight(),
                if profile == RuntimeProfile::Constrained {
                    8
                } else {
                    30
                }
            );
            for t in tickets.iter().flatten() {
                rt.cancel(t).unwrap();
            }
            for _ in 0..8 {
                assert!(matches!(
                    rt.poll(0),
                    Ok(RuntimePoll::Cancelled(_, CancelReason::Cancelled))
                ));
            }
            for t in tickets.iter_mut().flatten() {
                let key = t.completion_key();
                rt.complete_io(t, key, 0).unwrap();
            }
        });
    }
}
#[test]
fn policy_change_after_ready_cancels_without_issuing_credit() {
    with_runtime(RuntimeProfile::Constrained, |rt| {
        let rows = [spec(6)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = rt.admit(&plan, 0, 0, Generation(1)).unwrap();
        rt.poll(0).unwrap();
        let held = rt.parent_used();
        assert_eq!(
            rt.issue_credit(&ticket, 0, Generation(2)).err(),
            Some(QdnfError::Revoked)
        );
        assert_eq!(rt.parent_used(), held);
        let key = ticket.completion_key();
        assert_eq!(
            rt.complete_io(&mut ticket, key, 0),
            Ok(FinishOutcome::Revoked)
        );
    });
}
#[test]
fn malformed_overflow_and_control_class_plans_refuse() {
    for which in 0..7 {
        let mut row = spec(7);
        match which {
            0 => row.class = WorkClass::Control,
            1 => row.requirements.proof.bytes = 0,
            2 => row.credit_bytes = 17,
            3 => row.operation = OperationId::ZERO,
            4 => row.requirements.consumer.bytes = u64::MAX,
            5 => row.policy_generation = Generation::ZERO,
            _ => row.cost = 0,
        }
        assert!(ColdPlan::compile(&[row]).is_err());
    }
    let rows = [spec(7), spec(7)];
    assert!(ColdPlan::compile(&rows).is_err());
}
