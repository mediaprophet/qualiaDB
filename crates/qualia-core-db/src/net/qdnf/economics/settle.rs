//! Finalise, chargeback and fulfilment (E17.6–E17.7). Live rails are Unsupported.
//!
//! Funded in-process settlement is [`crate::net::qdnf::economics::adapter`].
//! `live_payment_rail` stays Unsupported: no fake Stripe or ILP.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::obligation::{Obligation, OpState};

/// Move a live hold into settled S. Duplicate callback is idempotent.
///
/// Fulfil when S + W == T after checked add. Fulfilment does not skip privacy.
pub fn finalise(
    ob: &mut Obligation,
    hold_amount: u64,
    op_id: StrongDigest,
    duplicate: bool,
) -> Result<(), QdnfError> {
    if op_id.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let i = match ob.find_op(op_id) {
        Some(i) => i,
        None => return Err(QdnfError::Incomplete),
    };
    match ob.ops[i].state {
        OpState::Finalised => {
            if ob.ops[i].amount != hold_amount {
                return Err(QdnfError::Conflict);
            }
            let _ = duplicate;
            Ok(())
        }
        OpState::Held => {
            if ob.ops[i].amount != hold_amount {
                return Err(QdnfError::Conflict);
            }
            ob.holds_h = ob
                .holds_h
                .checked_sub(hold_amount)
                .ok_or(QdnfError::Range)?;
            ob.settled_s = ob
                .settled_s
                .checked_add(hold_amount)
                .ok_or(QdnfError::Denied)?;
            ob.ops[i].state = OpState::Finalised;
            let sw = ob
                .settled_s
                .checked_add(ob.discharged_w)
                .ok_or(QdnfError::Range)?;
            if sw == ob.target_t {
                ob.fulfilled = true;
            }
            ob.bump_revision()
        }
        OpState::Empty => Err(QdnfError::Incomplete),
        OpState::Cancelled | OpState::Reversed => Err(QdnfError::Denied),
    }
}

/// Reverse settled S only. T is unchanged; fulfilment stays terminal.
///
/// Cannot resurrect unlimited creation debt. Duplicate reverse is idempotent.
pub fn reverse(ob: &mut Obligation, amount: u64, op_id: StrongDigest) -> Result<(), QdnfError> {
    if op_id.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let i = ob.find_op(op_id).ok_or(QdnfError::Incomplete)?;
    match ob.ops[i].state {
        OpState::Reversed => {
            if ob.ops[i].amount != amount {
                return Err(QdnfError::Conflict);
            }
            Ok(())
        }
        OpState::Finalised => {
            if ob.ops[i].amount != amount {
                return Err(QdnfError::Conflict);
            }
            ob.settled_s = ob.settled_s.checked_sub(amount).ok_or(QdnfError::Denied)?;
            ob.ops[i].state = OpState::Reversed;
            ob.bump_revision()
        }
        OpState::Held | OpState::Cancelled | OpState::Empty => Err(QdnfError::Denied),
    }
}

/// Fulfilment never returns a flag that skips bilateral privacy.
#[inline]
pub fn fulfilment_bypasses_bilateral() -> bool {
    false
}

/// E18.4 live settlement adapters are not in this foundation.
#[inline]
pub fn live_payment_rail() -> Result<(), QdnfError> {
    Err(QdnfError::Unsupported)
}

/// Public settlement evidence. No patient / clinical label fields.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettlementReceipt {
    pub obligation_id: StrongDigest,
    pub op_id: StrongDigest,
    pub amount: u64,
}

pub fn settlement_receipt(ob: &Obligation, op_id: StrongDigest, amount: u64) -> SettlementReceipt {
    SettlementReceipt {
        obligation_id: ob.id,
        op_id,
        amount,
    }
}

/// Public settlement must not include patient labels.
#[inline]
pub fn settlement_contains_patient_label() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::economics::classify::recovery_for_class;
    use crate::net::qdnf::economics::classify::ActingClass;
    use crate::net::qdnf::economics::reserve::reserve_hold;

    fn open(t: u64) -> Obligation {
        Obligation::open(sha384(b"settle-ob"), t).unwrap()
    }

    fn op(tag: u8) -> StrongDigest {
        sha384(&[tag])
    }

    #[test]
    fn econ_c_finalise_and_duplicate_callback() {
        let mut ob = open(5);
        let held = reserve_hold(&mut ob, 5, ActingClass::Corporate, op(1)).unwrap();
        assert_eq!(held, 5);
        finalise(&mut ob, 5, op(1), false).unwrap();
        assert!(ob.fulfilled);
        assert_eq!(ob.settled_s, 5);
        assert_eq!(ob.holds_h, 0);
        finalise(&mut ob, 5, op(1), true).unwrap();
        assert_eq!(ob.settled_s, 5);
        assert_eq!(ob.revision, 2);
        assert!(!fulfilment_bypasses_bilateral());
    }

    #[test]
    fn cross_provider_replay_same_op_id_is_idempotent() {
        let mut ob = open(4);
        reserve_hold(&mut ob, 2, ActingClass::Corporate, op(9)).unwrap();
        finalise(&mut ob, 2, op(9), false).unwrap();
        assert_eq!(
            reserve_hold(&mut ob, 2, ActingClass::Corporate, op(9)).unwrap(),
            2
        );
        assert_eq!(ob.settled_s, 2);
        assert_eq!(ob.holds_h, 0);
        finalise(&mut ob, 2, op(9), true).unwrap();
        assert_eq!(ob.settled_s, 2);
    }

    #[test]
    fn chargeback_reverses_s_only_and_does_not_grow_t() {
        let mut ob = open(8);
        reserve_hold(&mut ob, 8, ActingClass::Corporate, op(2)).unwrap();
        finalise(&mut ob, 8, op(2), false).unwrap();
        assert!(ob.fulfilled);
        let t = ob.target_t;
        reverse(&mut ob, 8, op(2)).unwrap();
        assert_eq!(ob.settled_s, 0);
        assert_eq!(ob.target_t, t);
        assert!(ob.fulfilled);
        assert_eq!(
            recovery_for_class(ActingClass::Corporate, ob.fulfilled, 8),
            0
        );
        reverse(&mut ob, 8, op(2)).unwrap();
        assert_eq!(ob.target_t, 8);
        assert_eq!(
            reserve_hold(&mut ob, 8, ActingClass::Corporate, op(3)).unwrap(),
            0
        );
        assert_eq!(ob.holds_h, 0);
        assert_eq!(ob.target_t, 8);
    }

    #[test]
    fn econ_f_fulfilment_does_not_bypass_bilateral() {
        let mut ob = open(1);
        reserve_hold(&mut ob, 1, ActingClass::Corporate, op(4)).unwrap();
        finalise(&mut ob, 1, op(4), false).unwrap();
        assert!(ob.fulfilled);
        assert!(!fulfilment_bypasses_bilateral());
        assert!(!settlement_contains_patient_label());
        let rcpt = settlement_receipt(&ob, op(4), 1);
        assert_eq!(rcpt.obligation_id, ob.id);
        assert_eq!(live_payment_rail(), Err(QdnfError::Unsupported));
    }

    #[test]
    fn unknown_finalise_is_incomplete() {
        let mut ob = open(1);
        assert_eq!(
            finalise(&mut ob, 1, op(7), false),
            Err(QdnfError::Incomplete)
        );
    }
}
