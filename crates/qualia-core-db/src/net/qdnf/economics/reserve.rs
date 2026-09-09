//! Reserve A against remaining T − S − W − H (E17.4–E17.5). Quote O/F/A stay split.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::classify::{recovery_for_class, ActingClass};
use super::obligation::{remaining_recovery, Obligation, OpState};

/// Operating cost O, fees F and positive recovery A are independent amounts.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quote {
    pub operating_o: u64,
    pub fees_f: u64,
    pub recovery_a: u64,
}

impl Quote {
    pub const ZERO: Self = Self {
        operating_o: 0,
        fees_f: 0,
        recovery_a: 0,
    };
}

/// Reserve A = min(quote_a, remaining after class). Duplicate `op_id` is idempotent.
///
/// Overflow of S+W+H+A or A that would exceed T is Denied. Offline allotment uses
/// the same remaining cap.
pub fn reserve_hold(
    ob: &mut Obligation,
    quote_a: u64,
    class: ActingClass,
    op_id: StrongDigest,
) -> Result<u64, QdnfError> {
    if op_id.is_zero() {
        return Err(QdnfError::Malformed);
    }
    if let Some(i) = ob.find_op(op_id) {
        let rec = ob.ops[i];
        return match rec.state {
            OpState::Held | OpState::Finalised => Ok(rec.amount),
            OpState::Cancelled | OpState::Reversed => Err(QdnfError::Conflict),
            OpState::Empty => Err(QdnfError::Incomplete),
        };
    }
    let remaining = remaining_recovery(ob)?;
    let recoverable = recovery_for_class(class, ob.fulfilled, remaining);
    let amount = quote_a.min(recoverable);
    if quote_a > 0 && amount == 0 && recoverable == 0 && !ob.fulfilled {
        if matches!(class, ActingClass::Corporate) {
            return Err(QdnfError::Denied);
        }
        return Ok(0);
    }
    if amount == 0 {
        return Ok(0);
    }
    let used = ob
        .settled_s
        .checked_add(ob.discharged_w)
        .and_then(|x| x.checked_add(ob.holds_h))
        .and_then(|x| x.checked_add(amount))
        .ok_or(QdnfError::Denied)?;
    if used > ob.target_t {
        return Err(QdnfError::Denied);
    }
    ob.insert_op(op_id, amount, OpState::Held)?;
    ob.holds_h = ob.holds_h.checked_add(amount).ok_or(QdnfError::Denied)?;
    ob.bump_revision()?;
    Ok(amount)
}

/// Finite offline allotment: cannot exceed remaining. Same owner and remaining cap.
#[inline]
pub fn allot_offline(
    ob: &mut Obligation,
    amount: u64,
    class: ActingClass,
    op_id: StrongDigest,
) -> Result<u64, QdnfError> {
    reserve_hold(ob, amount, class, op_id)
}

/// Release a live hold. Duplicate cancel is idempotent.
pub fn cancel(ob: &mut Obligation, op_id: StrongDigest) -> Result<(), QdnfError> {
    let i = ob.find_op(op_id).ok_or(QdnfError::Incomplete)?;
    match ob.ops[i].state {
        OpState::Held => {
            let amt = ob.ops[i].amount;
            ob.holds_h = ob.holds_h.checked_sub(amt).ok_or(QdnfError::Range)?;
            ob.ops[i].state = OpState::Cancelled;
            ob.bump_revision()
        }
        OpState::Cancelled => Ok(()),
        OpState::Empty => Err(QdnfError::Incomplete),
        OpState::Finalised | OpState::Reversed => Err(QdnfError::Denied),
    }
}

/// Hold expiry follows the same release as cancel.
#[inline]
pub fn expire(ob: &mut Obligation, op_id: StrongDigest) -> Result<(), QdnfError> {
    cancel(ob, op_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::authority::CompensationClass;
    use crate::net::qdnf::economics::obligation::remaining_recovery;
    use crate::net::qdnf::economics::RemainingTarget;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn open(t: u64) -> Obligation {
        Obligation::open(sha384(b"reserve-ob"), t).unwrap()
    }

    fn op(tag: u8) -> StrongDigest {
        sha384(&[tag])
    }

    #[test]
    fn econ_c_reserve_finalise_duplicate_and_cancel() {
        let mut ob = open(10);
        let q = Quote {
            operating_o: 4,
            fees_f: 1,
            recovery_a: 3,
        };
        let held = reserve_hold(&mut ob, q.recovery_a, ActingClass::Corporate, op(1)).unwrap();
        assert_eq!(held, 3);
        assert_eq!(ob.holds_h, 3);
        assert_eq!(remaining_recovery(&ob).unwrap(), 7);
        assert_eq!(
            reserve_hold(&mut ob, 99, ActingClass::Corporate, op(1)).unwrap(),
            3
        );
        assert_eq!(ob.holds_h, 3);
        assert_eq!(ob.revision, 1);
        cancel(&mut ob, op(1)).unwrap();
        assert_eq!(ob.holds_h, 0);
        expire(&mut ob, op(1)).unwrap();
        let t = RemainingTarget {
            milli_units: 10,
            class: CompensationClass::CorporateDelegated,
        };
        let mixed = q.operating_o + q.fees_f + q.recovery_a;
        assert_eq!(t.apply_payment(mixed), Err(QdnfError::Denied));
    }

    #[test]
    fn exemption_reserve_is_zero_and_does_not_hold() {
        let mut ob = open(10);
        assert_eq!(
            reserve_hold(&mut ob, 5, ActingClass::Personal, op(2)).unwrap(),
            0
        );
        assert_eq!(
            reserve_hold(&mut ob, 5, ActingClass::Humanitarian, op(3)).unwrap(),
            0
        );
        assert_eq!(ob.holds_h, 0);
        assert_eq!(remaining_recovery(&ob).unwrap(), 10);
    }

    #[test]
    fn econ_d_last_unit_race_only_one_succeeds() {
        let mut ob = open(1);
        let a = reserve_hold(&mut ob, 1, ActingClass::Corporate, op(10));
        let b = reserve_hold(&mut ob, 1, ActingClass::Corporate, op(11));
        assert_eq!(a, Ok(1));
        assert_eq!(b, Err(QdnfError::Denied));
        assert_eq!(ob.holds_h, 1);
        assert!(!ob.fulfilled);
    }

    #[test]
    fn econ_d_threaded_last_unit_race() {
        let ob = Arc::new(Mutex::new(open(1)));
        let left = {
            let ob = Arc::clone(&ob);
            thread::spawn(move || {
                reserve_hold(&mut ob.lock().unwrap(), 1, ActingClass::Corporate, op(20))
            })
        };
        let right = {
            let ob = Arc::clone(&ob);
            thread::spawn(move || {
                reserve_hold(&mut ob.lock().unwrap(), 1, ActingClass::Corporate, op(21))
            })
        };
        let ra = left.join().unwrap();
        let rb = right.join().unwrap();
        let wins = [ra, rb].iter().filter(|r| **r == Ok(1)).count();
        let denied = [ra, rb]
            .iter()
            .filter(|r| **r == Err(QdnfError::Denied))
            .count();
        assert_eq!(wins, 1);
        assert_eq!(denied, 1);
        assert_eq!(ob.lock().unwrap().holds_h, 1);
    }

    #[test]
    fn offline_allotment_cannot_exceed_remaining() {
        let mut ob = open(2);
        assert_eq!(
            allot_offline(&mut ob, 2, ActingClass::Corporate, op(30)).unwrap(),
            2
        );
        assert_eq!(
            allot_offline(&mut ob, 1, ActingClass::Corporate, op(31)),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn zero_op_id_is_malformed() {
        let mut ob = open(1);
        assert_eq!(
            reserve_hold(&mut ob, 1, ActingClass::Corporate, StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
    }
}
