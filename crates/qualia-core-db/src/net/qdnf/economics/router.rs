//! Funded in-process “turn on router” (E18.6). Live rails stay Unsupported.

use crate::net::peer::runtime::ResourceBudget;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::adapter::{settle_in_process, SettlementRail};
use super::classify::ActingClass;
use super::meters::{meter_admits_service, MeterReading};
use super::obligation::Obligation;
use super::roles::{role_admits_router, RoleGrant};
use super::settle::SettlementReceipt;

/// In-process router. Off until a funded settlement admits the routing role.
#[derive(Debug, PartialEq, Eq)]
pub struct InProcessRouter {
    on: bool,
    remaining: ResourceBudget,
    funding_source: StrongDigest,
}

impl InProcessRouter {
    pub const fn new() -> Self {
        Self {
            on: false,
            remaining: ResourceBudget::ZERO,
            funding_source: StrongDigest::ZERO,
        }
    }

    #[inline]
    pub const fn is_on(&self) -> bool {
        self.on
    }

    #[inline]
    pub const fn remaining_budget(&self) -> ResourceBudget {
        self.remaining
    }

    #[inline]
    pub const fn funding_source(&self) -> StrongDigest {
        self.funding_source
    }
}

impl Default for InProcessRouter {
    fn default() -> Self {
        Self::new()
    }
}

fn subtract_budget(
    have: ResourceBudget,
    cost: ResourceBudget,
) -> Result<ResourceBudget, QdnfError> {
    Ok(ResourceBudget {
        bytes: have
            .bytes
            .checked_sub(cost.bytes)
            .ok_or(QdnfError::BudgetExhausted)?,
        work: have
            .work
            .checked_sub(cost.work)
            .ok_or(QdnfError::BudgetExhausted)?,
        io: have
            .io
            .checked_sub(cost.io)
            .ok_or(QdnfError::BudgetExhausted)?,
    })
}

/// Admit routing from a funded in-process settlement. Role and meter gates apply first.
pub fn turn_on_router(
    router: &mut InProcessRouter,
    ob: &mut Obligation,
    grant: RoleGrant,
    meter: MeterReading,
    class: ActingClass,
    quote_a: u64,
    op_id: StrongDigest,
) -> Result<SettlementReceipt, QdnfError> {
    if router.on {
        return Err(QdnfError::Conflict);
    }
    role_admits_router(&grant)?;
    let _ = meter_admits_service(meter)?;
    let receipt = settle_in_process(ob, SettlementRail::InProcess, class, quote_a, op_id)?;
    router.on = true;
    router.remaining = grant.budget;
    router.funding_source = grant.funding_source;
    Ok(receipt)
}

/// Charge remaining provider budget. Exhaustion does not turn the router off.
pub fn admit_router_work(
    router: &mut InProcessRouter,
    cost: ResourceBudget,
) -> Result<(), QdnfError> {
    if !router.on {
        return Err(QdnfError::Denied);
    }
    router.remaining = subtract_budget(router.remaining, cost)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::authority::ObservationQuality;
    use crate::net::qdnf::economics::adapter::{live_ilp_adapter, live_rail, live_stripe_adapter};
    use crate::net::qdnf::economics::meters::MeterKind;
    use crate::net::qdnf::economics::roles::{
        activate_role, natural_person_wallet_required, FundedRole,
    };
    use crate::net::qdnf::economics::settle::live_payment_rail;

    fn funded_ob() -> Obligation {
        Obligation::open(sha384(b"e18-router-ob"), 10).unwrap()
    }

    fn budget() -> ResourceBudget {
        ResourceBudget {
            bytes: 64,
            work: 4,
            io: 4,
        }
    }

    fn routing_grant() -> RoleGrant {
        activate_role(FundedRole::Routing, budget(), sha384(b"commons-fund")).unwrap()
    }

    fn measured_net() -> MeterReading {
        MeterReading {
            kind: MeterKind::NetBytes,
            milli_units: 8,
            quality: ObservationQuality::Measured,
        }
    }

    fn op(tag: u8) -> StrongDigest {
        sha384(&[tag])
    }

    #[test]
    fn funded_obligation_turns_on_in_process_router_and_emits_receipt() {
        let mut router = InProcessRouter::new();
        let mut ob = funded_ob();
        let grant = routing_grant();
        let rcpt = turn_on_router(
            &mut router,
            &mut ob,
            grant,
            measured_net(),
            ActingClass::Corporate,
            5,
            op(1),
        )
        .unwrap();
        assert!(router.is_on());
        assert_eq!(rcpt.obligation_id, ob.id);
        assert_eq!(rcpt.op_id, op(1));
        assert_eq!(rcpt.amount, 5);
        assert_eq!(ob.settled_s, 5);
        assert_eq!(router.remaining_budget(), budget());
        assert_eq!(router.funding_source(), grant.funding_source);
        assert!(!natural_person_wallet_required());
        assert_eq!(live_payment_rail(), Err(QdnfError::Unsupported));
        assert_eq!(live_stripe_adapter(), Err(QdnfError::Unsupported));
        assert_eq!(live_ilp_adapter(), Err(QdnfError::Unsupported));
        assert_eq!(
            live_rail(SettlementRail::LiveStripe),
            Err(QdnfError::Unsupported)
        );
        assert_eq!(
            turn_on_router(
                &mut router,
                &mut ob,
                grant,
                measured_net(),
                ActingClass::Corporate,
                1,
                op(9)
            ),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn unfunded_obligation_cannot_turn_router_on() {
        let mut router = InProcessRouter::new();
        let mut empty = Obligation::open(sha384(b"e18-unfunded"), 0).unwrap();
        assert_eq!(
            turn_on_router(
                &mut router,
                &mut empty,
                routing_grant(),
                measured_net(),
                ActingClass::Corporate,
                5,
                op(2)
            ),
            Err(QdnfError::Denied)
        );
        assert!(!router.is_on());
        let mut humanitarian = funded_ob();
        assert_eq!(
            turn_on_router(
                &mut router,
                &mut humanitarian,
                routing_grant(),
                measured_net(),
                ActingClass::Humanitarian,
                5,
                op(3)
            ),
            Err(QdnfError::Denied)
        );
        assert!(!router.is_on());
        assert_eq!(humanitarian.settled_s, 0);
        assert_eq!(humanitarian.holds_h, 0);
    }

    #[test]
    fn role_and_meter_gates_still_apply() {
        let src = sha384(b"commons-fund");
        let mut router = InProcessRouter::new();
        let mut ob = funded_ob();
        let relay = activate_role(FundedRole::Relay, budget(), src).unwrap();
        assert_eq!(
            turn_on_router(
                &mut router,
                &mut ob,
                relay,
                measured_net(),
                ActingClass::Corporate,
                5,
                op(4)
            ),
            Err(QdnfError::Denied)
        );
        assert!(!router.is_on());
        assert_eq!(ob.settled_s, 0);

        let unknown = MeterReading {
            kind: MeterKind::NetBytes,
            milli_units: 8,
            quality: ObservationQuality::Unknown,
        };
        assert_eq!(
            turn_on_router(
                &mut router,
                &mut ob,
                routing_grant(),
                unknown,
                ActingClass::Corporate,
                5,
                op(5)
            ),
            Err(QdnfError::Incomplete)
        );
        assert!(!router.is_on());
        assert_eq!(ob.holds_h, 0);

        let empty_budget = activate_role(FundedRole::Routing, ResourceBudget::ZERO, src).unwrap();
        assert_eq!(
            turn_on_router(
                &mut router,
                &mut ob,
                empty_budget,
                measured_net(),
                ActingClass::Corporate,
                5,
                op(6)
            ),
            Err(QdnfError::BudgetExhausted)
        );
        assert!(!router.is_on());

        let grant = routing_grant();
        turn_on_router(
            &mut router,
            &mut ob,
            grant,
            measured_net(),
            ActingClass::Corporate,
            5,
            op(7),
        )
        .unwrap();
        admit_router_work(&mut router, grant.budget).unwrap();
        assert_eq!(
            admit_router_work(
                &mut router,
                ResourceBudget {
                    bytes: 1,
                    work: 0,
                    io: 0
                }
            ),
            Err(QdnfError::BudgetExhausted)
        );
        assert!(router.is_on());
        assert_eq!(router.remaining_budget(), ResourceBudget::ZERO);
    }
}
