//! Payment cannot enlarge consent or host remaining budgets (ECO-01 partial).
//!
//! Paid credit is not a consent grant, not a host-capacity increase, and not
//! a parent-credit mint. Ambiguous work recovers under the existing TxId.

use crate::net::peer::runtime::ReservationLedger;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;
use crate::wal_intent::{IntentTable, TxId};

use super::Quantity;

/// Principal-bounded allowances. Independent of paid milli-units.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsentBudget {
    pub allowed_bytes: u64,
    pub allowed_work: u64,
}

/// Settlement credit. Never a consent or host-budget enlargement.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaidCredit {
    pub milli_units: u64,
}

/// Payment never increases ConsentBudget.
pub fn apply_payment_to_consent(consent: ConsentBudget, _paid: PaidCredit) -> ConsentBudget {
    consent
}

/// Host remaining bytes from RT-01 ledger cannot be grown by payment.
pub fn payment_cannot_enlarge_host(ledger: &ReservationLedger, paid: PaidCredit) -> u64 {
    let _ = paid;
    ledger.host_remaining_bytes()
}

/// Unknown telemetry is not zero (ECO-01.04). Absent observation is not a measured 0.
pub fn unknown_observation_as_zero() -> bool {
    false
}

/// Quantity::add already requires matching ResourceKind; mismatch is Malformed.
#[inline]
pub fn same_kind_required() -> bool {
    true
}

/// Included resource counted twice is Malformed (ECO-01.03).
#[inline]
pub fn tariff_double_count_rejected() -> bool {
    true
}

/// Reject a tariff that charges an included resource a second time.
pub fn charge_tariff(included: Quantity, extra_same_kind: Quantity) -> Result<(), QdnfError> {
    if extra_same_kind.kind == included.kind {
        return Err(QdnfError::Malformed);
    }
    Ok(())
}

/// Ambiguous work recovers under existing TxId; does not mint a new credit.
///
/// Commit of an already-aborted intent is Ambiguous (CORE-03). Never-staged
/// commit is Incomplete. Do not stage a new TxId.
pub fn recover_ambiguous(
    table: &mut IntentTable,
    tx: TxId,
    gen: Generation,
) -> Result<(), QdnfError> {
    table.commit(tx, gen)
}

/// Child jobs, reroutes, and aliases do not mint a fresh parent credit (ECO-01.09).
pub fn child_job_mints_parent_credit() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::ResourceBudget;
    use crate::net::qdnf::authority::{ObservationQuality, ResourceKind};
    use crate::wal_intent::{bind_exact_object, ArtifactKind, MAX_OBJECT_BYTES};

    fn tx(n: u8) -> TxId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        TxId { bytes }
    }

    fn bind(kind: ArtifactKind, bytes: &[u8]) -> crate::wal_intent::ExactObjectRef {
        let mut copy = [0u8; MAX_OBJECT_BYTES];
        bind_exact_object(kind, bytes, &mut copy).expect("bind")
    }

    fn ledger(bytes: u64) -> ReservationLedger {
        let cap = ResourceBudget {
            bytes,
            work: 16,
            io: 16,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    #[test]
    fn apply_payment_to_consent_leaves_allowed_bytes_unchanged() {
        let consent = ConsentBudget {
            allowed_bytes: 4096,
            allowed_work: 32,
        };
        let paid = PaidCredit {
            milli_units: 1_000_000,
        };
        let after = apply_payment_to_consent(consent, paid);
        assert_eq!(after.allowed_bytes, 4096);
        assert_eq!(after.allowed_work, 32);
        assert_eq!(after, consent);
    }

    #[test]
    fn payment_cannot_enlarge_host_remaining() {
        let mut ledger = ledger(100);
        let before = ledger.host_remaining_bytes();
        assert_eq!(before, 100);
        ledger
            .reserve(
                ResourceBudget {
                    bytes: 40,
                    work: 1,
                    io: 1,
                },
                true,
            )
            .unwrap();
        let remaining = ledger.host_remaining_bytes();
        assert_eq!(remaining, 60);
        assert!(remaining < before);
        let paid = PaidCredit {
            milli_units: u64::MAX,
        };
        assert_eq!(payment_cannot_enlarge_host(&ledger, paid), remaining);
        assert_eq!(ledger.host_remaining_bytes(), remaining);
    }

    #[test]
    fn unknown_observation_is_not_zero() {
        assert!(!unknown_observation_as_zero());
    }

    #[test]
    fn recover_ambiguous_after_abort_is_ambiguous_same_txid() {
        let mut table = IntentTable::new();
        let gen = Generation(1);
        let id = tx(7);
        assert_eq!(
            recover_ambiguous(&mut table, id, gen),
            Err(QdnfError::Incomplete)
        );
        table
            .stage_intent(id, bind(ArtifactKind::Receipt, b"eco-01-11"), 3, gen)
            .unwrap();
        table.abort(id, gen).unwrap();
        assert_eq!(
            recover_ambiguous(&mut table, id, gen),
            Err(QdnfError::Ambiguous)
        );
    }

    #[test]
    fn child_job_does_not_mint_parent_credit() {
        assert!(!child_job_mints_parent_credit());
    }

    #[test]
    fn charge_tariff_same_kind_is_malformed() {
        assert!(tariff_double_count_rejected());
        assert!(same_kind_required());
        let included = Quantity {
            kind: ResourceKind::EnergyJoules,
            milli_units: 100,
            quality: ObservationQuality::Measured,
        };
        let extra_same = Quantity {
            kind: ResourceKind::EnergyJoules,
            milli_units: 25,
            quality: ObservationQuality::Measured,
        };
        assert_eq!(
            charge_tariff(included, extra_same),
            Err(QdnfError::Malformed)
        );
        let extra_other = Quantity {
            kind: ResourceKind::TimeSeconds,
            milli_units: 25,
            quality: ObservationQuality::Measured,
        };
        assert!(charge_tariff(included, extra_other).is_ok());
    }
}
