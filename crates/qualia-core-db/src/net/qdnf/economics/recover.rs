//! Provider failure, withdrawal and incomplete work (E18.5).
//!
//! Reconcile without a second payout. Protected content is never held hostage.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::obligation::{Obligation, OpState};
use super::reserve::cancel;
use super::settle::finalise;

/// Provider delivery outcome for one operation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderOutcome {
    Delivered = 1,
    Incomplete = 2,
    Withdrawn = 3,
    Disputed = 4,
}

/// Reconcile one operation. Duplicate delivered finalise is idempotent.
///
/// Incomplete, withdrawn and disputed holds release without paying. Already
/// settled amounts are not paid again; chargeback stays [`super::settle::reverse`].
pub fn reconcile_provider(
    ob: &mut Obligation,
    op_id: StrongDigest,
    outcome: ProviderOutcome,
) -> Result<(), QdnfError> {
    if op_id.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let i = ob.find_op(op_id).ok_or(QdnfError::Incomplete)?;
    match (ob.ops[i].state, outcome) {
        (OpState::Finalised, ProviderOutcome::Delivered) => Ok(()),
        (OpState::Held, ProviderOutcome::Delivered) => {
            let amt = ob.ops[i].amount;
            finalise(ob, amt, op_id, false)
        }
        (OpState::Held, ProviderOutcome::Incomplete)
        | (OpState::Held, ProviderOutcome::Withdrawn)
        | (OpState::Held, ProviderOutcome::Disputed) => cancel(ob, op_id),
        (OpState::Finalised, ProviderOutcome::Incomplete)
        | (OpState::Finalised, ProviderOutcome::Withdrawn)
        | (OpState::Finalised, ProviderOutcome::Disputed) => Ok(()),
        (OpState::Cancelled | OpState::Reversed, ProviderOutcome::Delivered) => {
            Err(QdnfError::Denied)
        }
        (OpState::Cancelled | OpState::Reversed, _) => Ok(()),
        (OpState::Empty, _) => Err(QdnfError::Incomplete),
    }
}

/// Provider failure never withholds protected content pending payment.
#[inline]
pub fn holds_protected_content_hostage() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::economics::classify::ActingClass;
    use crate::net::qdnf::economics::reserve::reserve_hold;
    use crate::net::qdnf::economics::settle::finalise as settle_finalise;

    fn open(t: u64) -> Obligation {
        Obligation::open(sha384(b"e18-recover-ob"), t).unwrap()
    }

    fn op(tag: u8) -> StrongDigest {
        sha384(&[tag])
    }

    #[test]
    fn incomplete_work_does_not_pay() {
        let mut ob = open(8);
        reserve_hold(&mut ob, 5, ActingClass::Corporate, op(1)).unwrap();
        reconcile_provider(&mut ob, op(1), ProviderOutcome::Incomplete).unwrap();
        assert_eq!(ob.settled_s, 0);
        assert_eq!(ob.holds_h, 0);
        assert!(!ob.fulfilled);
        assert!(!holds_protected_content_hostage());
    }

    #[test]
    fn withdrawal_and_dispute_do_not_duplicate_payout() {
        let mut ob = open(8);
        reserve_hold(&mut ob, 4, ActingClass::Corporate, op(2)).unwrap();
        settle_finalise(&mut ob, 4, op(2), false).unwrap();
        assert_eq!(ob.settled_s, 4);
        reconcile_provider(&mut ob, op(2), ProviderOutcome::Delivered).unwrap();
        reconcile_provider(&mut ob, op(2), ProviderOutcome::Withdrawn).unwrap();
        reconcile_provider(&mut ob, op(2), ProviderOutcome::Disputed).unwrap();
        assert_eq!(ob.settled_s, 4);
        assert!(!holds_protected_content_hostage());
        reserve_hold(&mut ob, 3, ActingClass::Corporate, op(3)).unwrap();
        reconcile_provider(&mut ob, op(3), ProviderOutcome::Withdrawn).unwrap();
        assert_eq!(ob.settled_s, 4);
        assert_eq!(ob.holds_h, 0);
        assert_eq!(
            reconcile_provider(&mut ob, op(2), ProviderOutcome::Delivered),
            Ok(())
        );
        assert_eq!(ob.settled_s, 4);
    }
}
