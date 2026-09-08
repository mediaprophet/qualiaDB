//! Typed resource quantities. Payment cannot enlarge consent or budgets.

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

/// Finite remaining compensation target. Zero target refuses further recovery.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RemainingTarget {
    pub milli_units: u64,
    pub class: CompensationClass,
}

impl RemainingTarget {
    pub fn apply_payment(&self, amount: u64) -> Result<Self, QdnfError> {
        if self.milli_units == 0 {
            return Err(QdnfError::Denied);
        }
        Ok(Self {
            milli_units: self.milli_units.saturating_sub(amount),
            class: self.class,
        })
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
}
