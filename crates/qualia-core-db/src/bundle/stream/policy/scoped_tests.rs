use super::*;
use std::cell::Cell;

#[test]
fn scoped_effect_rechecks_expiry_after_io_and_cannot_recover_from_failure() {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
    let now = Cell::new(20);
    let mut checkpoints = 0;
    assert_eq!(
        grant.accept_local_scoped(
            &pins,
            || now.get(),
            |gate| {
                gate.recheck()?;
                assert_eq!(
                    gate.closure().identity().descriptor_id(),
                    f.identity.descriptor_id()
                );
                now.set(90); // The bounded disk barrier consumed the remaining lifetime.
                assert_eq!(
                    gate.recheck(),
                    Err(PolicyError::Authority(QdnfError::Expired))
                );
                now.set(20); // A faulty host clock cannot revive this failed effect.
                assert_eq!(gate.recheck(), Err(PolicyError::Spent));
                Ok(()) // Swallowing a failure cannot publish successful acceptance.
            }
        ),
        Err(PolicyError::Spent)
    );
    assert_eq!(
        grant.accept_local_scoped(
            &pins,
            || 20,
            |_| {
                checkpoints += 1;
                Ok(())
            }
        ),
        Err(PolicyError::Spent)
    );
    assert_eq!(checkpoints, 0);
}

#[test]
fn scoped_effect_cancel_and_unwind_are_terminal_and_owner_slots_reclaim() {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    for _ in 0..12 {
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
        assert_eq!(
            grant.accept_local_scoped(
                &pins,
                || 20,
                |gate| {
                    gate.cancel()?;
                    assert_eq!(gate.recheck(), Err(PolicyError::Spent));
                    Ok(())
                }
            ),
            Err(PolicyError::Spent)
        );
        grant.close().unwrap();
    }
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        grant.accept_local_scoped::<()>(
            &pins,
            || 20,
            |gate| {
                gate.recheck()?;
                panic!("injected effect unwind")
            },
        )
    }));
    assert!(result.is_err());
    assert_eq!(
        grant.accept_local_scoped(&pins, || 20, |_| Ok(())),
        Err(PolicyError::Spent)
    );
    grant.close().unwrap();
}

#[test]
fn scoped_effect_final_check_refuses_expiry_without_callback_recheck() {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
    let now = Cell::new(20);
    assert_eq!(
        grant.accept_local_scoped(
            &pins,
            || now.get(),
            |_| {
                now.set(90);
                Ok(7)
            }
        ),
        Err(PolicyError::Authority(QdnfError::Expired))
    );
}
