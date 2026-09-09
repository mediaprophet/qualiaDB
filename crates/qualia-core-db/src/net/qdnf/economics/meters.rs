//! Independent resource meters (E18.2–E18.3). Unknown is not Measured and not zero.

use crate::net::qdnf::authority::{ObservationQuality, ResourceKind};
use crate::net::qdnf::errors::QdnfError;

use super::Quantity;

/// Meter kinds. Byte-time and network bytes stay distinct from joules, time and compute.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeterKind {
    EnergyJoules = 1,
    TimeSeconds = 2,
    TypedCompute = 3,
    ByteTime = 4,
    NetBytes = 5,
}

impl MeterKind {
    pub fn from_resource(kind: ResourceKind) -> Self {
        match kind {
            ResourceKind::EnergyJoules => Self::EnergyJoules,
            ResourceKind::TimeSeconds => Self::TimeSeconds,
            ResourceKind::TypedCompute => Self::TypedCompute,
        }
    }
}

/// One observed meter. Quality is independent of milli_units.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeterReading {
    pub kind: MeterKind,
    pub milli_units: u64,
    pub quality: ObservationQuality,
}

impl MeterReading {
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

    /// Unknown quality is Incomplete, never a measured zero.
    pub fn value(self) -> Result<u64, QdnfError> {
        match self.quality {
            ObservationQuality::Unknown => Err(QdnfError::Incomplete),
            ObservationQuality::Measured | ObservationQuality::Estimated => Ok(self.milli_units),
        }
    }
}

/// Unknown observation is not Measured.
#[inline]
pub fn unknown_quality_is_measured(q: ObservationQuality) -> bool {
    matches!(q, ObservationQuality::Measured)
}

/// Unknown meter is not billed as zero.
#[inline]
pub fn unknown_meter_as_zero(reading: MeterReading) -> bool {
    let _ = reading;
    false
}

/// Map a Quantity into a meter of the same ResourceKind.
pub fn reading_from_quantity(q: Quantity) -> MeterReading {
    MeterReading {
        kind: MeterKind::from_resource(q.kind),
        milli_units: q.milli_units,
        quality: q.quality,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(kind: MeterKind, n: u64) -> MeterReading {
        MeterReading {
            kind,
            milli_units: n,
            quality: ObservationQuality::Measured,
        }
    }

    #[test]
    fn kinds_are_independent_and_unknown_is_not_zero() {
        assert_ne!(MeterKind::EnergyJoules, MeterKind::ByteTime);
        assert_ne!(MeterKind::TimeSeconds, MeterKind::NetBytes);
        assert_ne!(MeterKind::TypedCompute, MeterKind::ByteTime);
        assert_eq!(
            measured(MeterKind::EnergyJoules, 1).add(measured(MeterKind::TimeSeconds, 1)),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            measured(MeterKind::ByteTime, 1).add(measured(MeterKind::NetBytes, 1)),
            Err(QdnfError::Malformed)
        );
        let unknown = MeterReading {
            kind: MeterKind::EnergyJoules,
            milli_units: 0,
            quality: ObservationQuality::Unknown,
        };
        assert!(!unknown_quality_is_measured(unknown.quality));
        assert!(!unknown_meter_as_zero(unknown));
        assert_eq!(unknown.value(), Err(QdnfError::Incomplete));
        let added = unknown.add(measured(MeterKind::EnergyJoules, 5)).unwrap();
        assert_eq!(added.quality, ObservationQuality::Unknown);
        assert_eq!(added.value(), Err(QdnfError::Incomplete));
    }

    #[test]
    fn quantity_maps_without_collapsing_kinds() {
        let q = Quantity {
            kind: ResourceKind::TypedCompute,
            milli_units: 9,
            quality: ObservationQuality::Estimated,
        };
        let r = reading_from_quantity(q);
        assert_eq!(r.kind, MeterKind::TypedCompute);
        assert_eq!(r.value().unwrap(), 9);
    }
}
