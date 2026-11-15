use super::*;
#[test]
fn replacing_each_parent_at_same_address_refuses_and_original_owner_restores() {
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let rows = [spec(3)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = parents
            .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
            .unwrap();
        let key = ticket.completion_key();
        // Recreate matching numeric ledger slots/generations/budgets deliberately.
        let data = state.data.as_ref().unwrap();
        let mut replacement = ledger();
        let _a = replacement
            .reserve(
                ResourceBudget {
                    bytes: data.static_charge_bytes,
                    work: 0,
                    io: 0,
                },
                true,
            )
            .unwrap();
        let _b = replacement.reserve(data.control_budget, true).unwrap();
        let _c = replacement.reserve(data.slots[0].budget, true).unwrap();
        let old = std::mem::replace(&mut *parents.ledger.lock().unwrap(), replacement);
        assert_eq!(
            parents.try_with_runtime(&mut state, |_| Ok(())),
            Err(E::StaleGeneration)
        );
        let _foreign = std::mem::replace(&mut *parents.ledger.lock().unwrap(), old);
        // Matching occupied leases at the SAME mutex value address cannot revive state.
        let mut replacement = LeaseTable::new();
        let data = state.data.as_ref().unwrap();
        let _a = replacement
            .acquire(data.control_budget.bytes as u32, true)
            .unwrap();
        let _b = replacement
            .acquire(data.slots[0].budget.bytes as u32, true)
            .unwrap();
        let old = std::mem::replace(&mut *parents.leases.lock().unwrap(), replacement);
        assert_eq!(
            parents.try_with_runtime(&mut state, |_| Ok(())),
            Err(E::StaleGeneration)
        );
        let _foreign = std::mem::replace(&mut *parents.leases.lock().unwrap(), old);
        let mut replacement = OperationTable::new();
        let data = state.data.as_ref().unwrap();
        let _a = replacement
            .admit(
                data.control_operation.id,
                crate::net::peer::runtime::LeaseHandle {
                    slot: 0,
                    generation: Generation(1),
                },
            )
            .unwrap();
        let _b = replacement
            .admit(
                data.slots[0].operation.unwrap().id,
                crate::net::peer::runtime::LeaseHandle {
                    slot: 1,
                    generation: Generation(1),
                },
            )
            .unwrap();
        let old = std::mem::replace(&mut *parents.operations.lock().unwrap(), replacement);
        assert_eq!(
            parents.try_with_runtime(&mut state, |_| Ok(())),
            Err(E::StaleGeneration)
        );
        let _foreign = std::mem::replace(&mut *parents.operations.lock().unwrap(), old);
        let mut replacement = FairScheduler::new();
        replacement
            .enqueue(state.data.as_ref().unwrap().slots[0].work.unwrap())
            .unwrap();
        let old = std::mem::replace(&mut *parents.scheduler.lock().unwrap(), replacement);
        assert_eq!(
            parents.try_with_runtime(&mut state, |_| Ok(())),
            Err(E::StaleGeneration)
        );
        let _foreign = std::mem::replace(&mut *parents.scheduler.lock().unwrap(), old);
        parents
            .try_with_runtime(&mut state, |rt| {
                assert_eq!(rt.state(&ticket)?, TransferState::Queued);
                assert_eq!(ticket.completion_key(), key);
                Ok(())
            })
            .unwrap();
        ready(parents, &mut state, &ticket);
        finish(parents, &mut state, &mut ticket);
        state.try_close().unwrap();
    });
}
#[test]
fn foreign_registration_even_with_equal_caps_cannot_attach_original_state() {
    with_parents(false, |original| {
        let mut state = original.admit(RuntimeProfile::Desktop, control()).unwrap();
        with_parents(false, |other| {
            assert_eq!(
                other.try_with_runtime(&mut state, |_| Ok(())),
                Err(E::StaleGeneration)
            );
        });
        state.try_close().unwrap();
    });
}
