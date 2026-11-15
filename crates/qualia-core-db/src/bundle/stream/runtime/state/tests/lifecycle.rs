use super::*;
#[test]
fn original_ticket_survives_polls_and_independent_canonical_admission_between_them() {
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let rows = [spec(1)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = parents
            .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
            .unwrap();
        let key = ticket.completion_key();
        let extra = {
            let mut actual = parents.ledger.lock().unwrap();
            ResourceGovernor::charge_unknown(
                &mut actual,
                ResourceBudget {
                    bytes: 16,
                    work: 1,
                    io: 1,
                },
            )
            .unwrap()
        };
        parents
            .try_with_runtime(&mut state, |rt| {
                assert_eq!(rt.state(&ticket)?, TransferState::Queued);
                assert_eq!(rt.supported_inflight(), 29);
                Ok(())
            })
            .unwrap();
        parents.ledger.lock().unwrap().release(extra).unwrap();
        ready(parents, &mut state, &ticket);
        for _ in 0..16 {
            parents
                .try_with_runtime(&mut state, |rt| {
                    assert_eq!(rt.state(&ticket)?, TransferState::InFlight);
                    assert_eq!(ticket.completion_key(), key);
                    Ok(())
                })
                .unwrap();
            assert!(parents.ledger.try_lock().is_ok());
            assert!(parents.operations.try_lock().is_ok());
        }
        finish(parents, &mut state, &mut ticket);
        state.try_close().unwrap();
    });
}
#[test]
fn callback_panic_restores_same_owner_and_does_not_poison_parent_mutexes() {
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let rows = [spec(2)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = parents
            .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
            .unwrap();
        let key = ticket.completion_key();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _: Result<(), E> = parents.try_with_runtime(&mut state, |mut rt| {
                rt.cancel(&ticket)?;
                panic!("state callback unwind")
            });
        }));
        assert!(outcome.is_err());
        assert!(parents.ledger.try_lock().is_ok());
        assert!(parents.leases.try_lock().is_ok());
        assert!(parents.operations.try_lock().is_ok());
        assert!(parents.scheduler.try_lock().is_ok());
        parents
            .try_with_runtime(&mut state, |mut rt| {
                assert_eq!(ticket.completion_key(), key);
                assert_eq!(
                    rt.state(&ticket)?,
                    TransferState::Cancelling(CancelReason::Cancelled)
                );
                assert!(matches!(
                    rt.poll(0)?,
                    RuntimePoll::Cancelled(_, CancelReason::Cancelled)
                ));
                assert_eq!(
                    rt.complete_io(&mut ticket, key, 0)?,
                    FinishOutcome::Cancelled
                );
                Ok(())
            })
            .unwrap();
        let mut next = parents
            .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
            .unwrap();
        assert_ne!(next.completion_key(), key);
        ready(parents, &mut state, &next);
        parents
            .try_with_runtime(&mut state, |mut rt| {
                assert_eq!(rt.complete_io(&mut next, key, 0), Err(E::StaleGeneration));
                Ok(())
            })
            .unwrap();
        finish(parents, &mut state, &mut next);
        state.try_close().unwrap();
    });
}
#[test]
fn later_lock_refusal_preserves_original_state_and_releases_earlier_locks() {
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let before = parents.ledger.lock().unwrap().used();
        let held = parents.scheduler.lock().unwrap();
        assert_eq!(
            parents.try_with_runtime(&mut state, |_| Ok(())),
            Err(E::WouldBlock)
        );
        assert_eq!(parents.ledger.try_lock().unwrap().used(), before);
        assert!(parents.operations.try_lock().is_ok());
        drop(held);
        parents.try_with_runtime(&mut state, |_| Ok(())).unwrap();
        state.try_close().unwrap();
    });
}
#[test]
fn close_refusal_returns_original_owner_and_lost_state_keeps_charge() {
    with_parents(true, |parents| {
        let state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let before = parents.ledger.lock().unwrap().used();
        drop(state);
        assert_eq!(parents.ledger.lock().unwrap().used(), before);
        assert!(matches!(
            parents.admit(RuntimeProfile::Desktop, control()),
            Err(E::Capacity)
        ));
    });
}
#[test]
fn partial_close_progress_never_replays_consumed_control_operation() {
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        // Inject an exact parent checked-release refusal AFTER actual control completion/release.
        let result = parents.with_runtime_raw(&mut state, |rt| {
            rt.static_charge_bytes += 1;
            rt.close()
        });
        assert_eq!(result, Err(E::Conflict));
        let data = state.data.as_mut().unwrap();
        assert_eq!(
            data.close_phase,
            super::super::data::RuntimeClosePhase::ControlChargeReleased
        );
        assert!(data.control_charge.is_none());
        data.static_charge_bytes -= 1;
        state.try_close().unwrap(); // control operation is already gone, not completed twice
    });
}

#[test]
fn canonical_operation_cancel_between_polls_is_observed_without_new_credit() {
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let rows = [spec(5)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = parents
            .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
            .unwrap();
        let operation = ticket.completion_key().operation();
        parents
            .operations
            .lock()
            .unwrap()
            .cancel(operation)
            .unwrap();
        parents
            .try_with_runtime(&mut state, |mut rt| {
                assert_eq!(
                    rt.state(&ticket)?,
                    TransferState::Cancelling(CancelReason::Cancelled)
                );
                assert_eq!(
                    rt.issue_credit(&ticket, 0, Generation(1)).unwrap_err(),
                    E::Cancelled
                );
                assert!(matches!(
                    rt.poll(0)?,
                    RuntimePoll::Cancelled(_, CancelReason::Cancelled)
                ));
                let key = ticket.completion_key();
                assert_eq!(
                    rt.complete_io(&mut ticket, key, 0)?,
                    FinishOutcome::Cancelled
                );
                Ok(())
            })
            .unwrap();
        state.try_close().unwrap();
    });
}

#[test]
fn rejected_admission_leaves_actual_parent_usage_and_registration_reusable() {
    with_parents(false, |parents| {
        let before = parents.ledger.lock().unwrap().used();
        let mut invalid = control();
        invalid.bytes = 0;
        assert!(matches!(
            parents.admit(RuntimeProfile::Desktop, invalid),
            Err(E::Malformed)
        ));
        assert_eq!(parents.ledger.lock().unwrap().used(), before);
        let state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        state.try_close().unwrap();
    });
}
