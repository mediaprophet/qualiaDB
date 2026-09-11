//! In-process settlement adapter (E18.4). Live rails stay Unsupported.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::classify::ActingClass;
use super::obligation::Obligation;
use super::reserve::reserve_hold;
use super::settle::{finalise, live_payment_rail, settlement_receipt, SettlementReceipt};

/// Selected settlement systems. Only [`Self::InProcess`] may settle here.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettlementRail {
    InProcess = 1,
    LiveStripe = 2,
    LiveIlp = 3,
}

/// Capped local micro-charges. Pending cannot exceed `cap`.
#[derive(Debug, PartialEq, Eq)]
pub struct MicropaymentAggregate {
    pending: u64,
    cap: u64,
}

impl MicropaymentAggregate {
    pub fn new(cap: u64) -> Result<Self, QdnfError> {
        if cap == 0 {
            return Err(QdnfError::Malformed);
        }
        Ok(Self { pending: 0, cap })
    }

    #[inline]
    pub const fn pending(&self) -> u64 {
        self.pending
    }

    #[inline]
    pub const fn cap(&self) -> u64 {
        self.cap
    }
}

/// Live Stripe adapter. Never a fake success path.
#[inline]
pub fn live_stripe_adapter() -> Result<(), QdnfError> {
    Err(QdnfError::Unsupported)
}

/// Live Interledger adapter. Never a fake success path.
#[inline]
pub fn live_ilp_adapter() -> Result<(), QdnfError> {
    Err(QdnfError::Unsupported)
}

/// Any live rail API is Unsupported. In-process settlement is not a live rail.
#[inline]
pub fn live_rail(_rail: SettlementRail) -> Result<(), QdnfError> {
    Err(QdnfError::Unsupported)
}

/// Accumulate a micro charge. Exceeding the cap is Denied, never a silent enlarge.
pub fn aggregate_micro(agg: &mut MicropaymentAggregate, amount: u64) -> Result<u64, QdnfError> {
    if amount == 0 {
        return Ok(agg.pending);
    }
    let next = agg.pending.checked_add(amount).ok_or(QdnfError::Range)?;
    if next > agg.cap {
        return Err(QdnfError::Denied);
    }
    agg.pending = next;
    Ok(agg.pending)
}

/// Funded in-process settlement. Live rails do not mutate the obligation.
pub fn settle_in_process(
    ob: &mut Obligation,
    rail: SettlementRail,
    class: ActingClass,
    quote_a: u64,
    op_id: StrongDigest,
) -> Result<SettlementReceipt, QdnfError> {
    match rail {
        SettlementRail::InProcess => {}
        SettlementRail::LiveStripe | SettlementRail::LiveIlp => {
            return Err(QdnfError::Unsupported);
        }
    }
    let held = reserve_hold(ob, quote_a, class, op_id)?;
    if held == 0 {
        return Err(QdnfError::Denied);
    }
    finalise(ob, held, op_id, false)?;
    Ok(settlement_receipt(ob, op_id, held))
}

/// Drain a capped aggregate through the in-process adapter.
pub fn settle_aggregate(
    ob: &mut Obligation,
    agg: &mut MicropaymentAggregate,
    class: ActingClass,
    op_id: StrongDigest,
) -> Result<SettlementReceipt, QdnfError> {
    if agg.pending == 0 {
        return Err(QdnfError::Incomplete);
    }
    let amount = agg.pending;
    let receipt = settle_in_process(ob, SettlementRail::InProcess, class, amount, op_id)?;
    agg.pending = 0;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    fn open(t: u64) -> Obligation {
        Obligation::open(sha384(b"e18-adapter-ob"), t).unwrap()
    }

    fn op(tag: u8) -> StrongDigest {
        sha384(&[tag])
    }

    #[test]
    fn live_rails_stay_unsupported_and_do_not_mutate() {
        let mut ob = open(8);
        let rev = ob.revision;
        assert_eq!(live_payment_rail(), Err(QdnfError::Unsupported));
        assert_eq!(live_stripe_adapter(), Err(QdnfError::Unsupported));
        assert_eq!(live_ilp_adapter(), Err(QdnfError::Unsupported));
        assert_eq!(
            live_rail(SettlementRail::InProcess),
            Err(QdnfError::Unsupported)
        );
        assert_eq!(
            live_rail(SettlementRail::LiveStripe),
            Err(QdnfError::Unsupported)
        );
        assert_eq!(
            live_rail(SettlementRail::LiveIlp),
            Err(QdnfError::Unsupported)
        );
        assert_eq!(
            settle_in_process(
                &mut ob,
                SettlementRail::LiveStripe,
                ActingClass::Corporate,
                4,
                op(1)
            ),
            Err(QdnfError::Unsupported)
        );
        assert_eq!(
            settle_in_process(
                &mut ob,
                SettlementRail::LiveIlp,
                ActingClass::Corporate,
                4,
                op(2)
            ),
            Err(QdnfError::Unsupported)
        );
        assert_eq!(ob.revision, rev);
        assert_eq!(ob.holds_h, 0);
        assert_eq!(ob.settled_s, 0);
    }

    #[test]
    fn in_process_settle_emits_receipt() {
        let mut ob = open(6);
        let rcpt = settle_in_process(
            &mut ob,
            SettlementRail::InProcess,
            ActingClass::Corporate,
            6,
            op(3),
        )
        .unwrap();
        assert_eq!(rcpt.obligation_id, ob.id);
        assert_eq!(rcpt.op_id, op(3));
        assert_eq!(rcpt.amount, 6);
        assert_eq!(ob.settled_s, 6);
        assert!(ob.fulfilled);
        assert_eq!(
            settle_in_process(
                &mut ob,
                SettlementRail::InProcess,
                ActingClass::Corporate,
                1,
                op(4)
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn aggregation_cap_then_in_process_settle() {
        let mut agg = MicropaymentAggregate::new(10).unwrap();
        assert_eq!(aggregate_micro(&mut agg, 4).unwrap(), 4);
        assert_eq!(aggregate_micro(&mut agg, 3).unwrap(), 7);
        assert_eq!(aggregate_micro(&mut agg, 4), Err(QdnfError::Denied));
        assert_eq!(agg.pending(), 7);
        let mut ob = open(10);
        let rcpt = settle_aggregate(&mut ob, &mut agg, ActingClass::Corporate, op(5)).unwrap();
        assert_eq!(rcpt.amount, 7);
        assert_eq!(agg.pending(), 0);
        assert_eq!(ob.settled_s, 7);
        assert_eq!(
            settle_aggregate(&mut ob, &mut agg, ActingClass::Corporate, op(6)),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(MicropaymentAggregate::new(0), Err(QdnfError::Malformed));
    }
}
