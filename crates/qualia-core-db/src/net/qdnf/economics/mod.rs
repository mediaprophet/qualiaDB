//! Typed resource quantities. Payment cannot enlarge consent or budgets.
//!
//! Finite commons recovery is owned by [`Obligation`]. [`RemainingTarget`] remains
//! a compatibility snapshot of copied milli-units and is not the owner.

pub mod adapter;
pub mod classify;
pub mod consent;
pub mod meters;
pub mod obligation;
pub mod recover;
pub mod reserve;
pub mod roles;
pub mod router;
pub mod settle;

#[cfg(test)]
mod concurrent;

pub use adapter::*;
pub use classify::*;
pub use consent::*;
pub use meters::*;
pub use obligation::*;
pub use recover::*;
pub use reserve::*;
pub use roles::*;
pub use router::*;
pub use settle::*;

use crate::net::qdnf::authority::{CompensationClass, ObservationQuality, ResourceKind};
use crate::net::qdnf::errors::QdnfError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quantity {
    pub kind: ResourceKind,
    pub milli_units: u64,
    pub quality: ObservationQuality,
}

impl Quantity {
    pub fn add(self, other: Self) -> Result<Self, QdnfError> {
        if self.kind != other.kind {
            return Err(QdnfError::Malformed);
        }
        Ok(Self {
            kind: self.kind,
            milli_units: self
                .milli_units
                .checked_add(other.milli_units)
                .ok_or(QdnfError::Range)?,
            quality: match (self.quality, other.quality) {
                (ObservationQuality::Unknown, _) | (_, ObservationQuality::Unknown) => {
                    ObservationQuality::Unknown
                }
                (ObservationQuality::Estimated, _) | (_, ObservationQuality::Estimated) => {
                    ObservationQuality::Estimated
                }
                _ => ObservationQuality::Measured,
            },
        })
    }
}

/// Compatibility snapshot of a remaining amount. Not the obligation owner.
/// Copied `milli_units` cannot settle; use [`Obligation`] + [`reserve_hold`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemainingTarget {
    pub milli_units: u64,
    pub class: CompensationClass,
}

impl RemainingTarget {
    /// Always Denied. A copied RemainingTarget is not the owner (E17 / R17).
    pub fn apply_payment(&self, amount: u64) -> Result<Self, QdnfError> {
        let _ = (self, amount);
        Err(QdnfError::Denied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_target_refuses_recovery() {
        let t = RemainingTarget {
            milli_units: 0,
            class: CompensationClass::Humanitarian,
        };
        assert_eq!(t.apply_payment(1), Err(QdnfError::Denied));
    }

    #[test]
    fn copied_remaining_target_is_not_owner() {
        let t = RemainingTarget {
            milli_units: 100,
            class: CompensationClass::CorporateDelegated,
        };
        let copy = t;
        assert_eq!(t.apply_payment(1), Err(QdnfError::Denied));
        assert_eq!(copy.apply_payment(t.milli_units), Err(QdnfError::Denied));
        assert_eq!(copy.milli_units, 100);
    }

    #[test]
    fn energy_and_time_do_not_add() {
        let e = Quantity {
            kind: ResourceKind::EnergyJoules,
            milli_units: 1,
            quality: ObservationQuality::Measured,
        };
        let t = Quantity {
            kind: ResourceKind::TimeSeconds,
            milli_units: 1,
            quality: ObservationQuality::Measured,
        };
        assert_eq!(e.add(t), Err(QdnfError::Malformed));
    }

    #[test]
    fn fulfilment_cannot_bypass_bilateral_identity_lock() {
        use crate::{
            evaluate_permissive_runtime_gate, MASK_AUTHENTICATED_NATURAL_PERSON,
            MASK_BILATERAL_IDENTITY_LOCKED, MASK_WORK_OBLIGATION_SATISFIED,
        };
        let entry = MASK_BILATERAL_IDENTITY_LOCKED | MASK_WORK_OBLIGATION_SATISFIED;
        assert!(
            !evaluate_permissive_runtime_gate(entry, 0),
            "work-obligation satisfied must not open bilateral-locked data"
        );
        assert!(evaluate_permissive_runtime_gate(
            entry,
            MASK_AUTHENTICATED_NATURAL_PERSON
        ));
        assert!(evaluate_permissive_runtime_gate(
            MASK_WORK_OBLIGATION_SATISFIED,
            0
        ));
    }
}
