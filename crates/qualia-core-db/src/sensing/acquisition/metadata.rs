use super::{Error, ExactTime, Rate, Timing};
use crate::sensing::SemanticId;
/// Full-content reference; identifier is a registry key, not an integrity proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactLink {
    pub digest: [u8; 32],
    pub identifier: SemanticId,
}
impl ArtifactLink {
    pub fn validate(self) -> Result<(), Error> {
        self.identifier.value().ok_or(Error::Identity).map(|_| ())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quantity {
    pub kind: SemanticId,
    pub unit: SemanticId,
}
impl Quantity {
    pub fn require_known(self) -> Result<(), Error> {
        self.kind.value().ok_or(Error::Semantics)?;
        self.unit.value().ok_or(Error::Semantics)?;
        Ok(())
    }
}
/// References are reported lineage only: no implicit affine conversion or traceability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleStage {
    RawCodes,
    ReportedPhysical,
    DerivedPhysical,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UncertaintyLink {
    pub artifact: ArtifactLink,
    pub target: Quantity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub quantity: Quantity,
    pub stage: SampleStage,
    pub sensor: SemanticId,
    pub geometry: Option<ArtifactLink>,
    pub calibration: Option<ArtifactLink>,
    /// Uncertainty artifact owns target quantity/component, unit, estimator and correlations.
    pub uncertainty: Option<UncertaintyLink>,
}
impl Context {
    pub fn validate(self) -> Result<(), Error> {
        if let Some(u) = self.uncertainty {
            u.artifact.validate()?;
            u.target.require_known()?;
            if u.target != self.quantity {
                return Err(Error::Semantics);
            }
        }
        for link in [self.geometry, self.calibration].into_iter().flatten() {
            link.validate()?;
        }
        Ok(())
    }
}
/// Clock origin/restart identity is mandatory; equal numeric epochs do not imply a join.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    origin: SemanticId,
    restart: u64,
    timing: Timing,
}
impl Clock {
    pub fn new(origin: SemanticId, restart: u64, timing: Timing) -> Result<Self, Error> {
        origin.value().ok_or(Error::Identity)?;
        timing.time_at(0).map_err(|_| Error::Clock)?;
        Ok(Self {
            origin,
            restart,
            timing,
        })
    }
    pub fn origin(self) -> SemanticId {
        self.origin
    }
    pub fn restart(self) -> u64 {
        self.restart
    }
    pub fn timing(self) -> Timing {
        self.timing
    }
    pub fn time_at(self, index: u64) -> Result<ExactTime, Error> {
        self.timing.time_at(index).map_err(|_| Error::Clock)
    }
    pub fn tick_time(self, tick: i64) -> Result<ExactTime, Error> {
        Timing {
            start_tick: tick,
            ..self.timing
        }
        .time_at(0)
        .map_err(|_| Error::Clock)
    }
    pub fn require_same_domain(self, other: Self) -> Result<(), Error> {
        if self.origin != other.origin
            || self.restart != other.restart
            || self.timing.scale != other.timing.scale
            || self.timing.epoch_seconds != other.timing.epoch_seconds
            || self.timing.epoch_nanoseconds != other.timing.epoch_nanoseconds
            || self.timing.ticks != other.timing.ticks
        {
            return Err(Error::Clock);
        }
        Ok(())
    }
}
/// Absence, unknown carrier, and a reported static field are distinct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Carrier {
    Unknown,
    AperiodicOrStatic,
    Hertz(f64),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComplexConvention {
    RealPlusIImagPositiveExponent,
    RealPlusIImagNegativeExponent,
}
/// Baseband bounds are signed offsets; never interchangeable with carrier or cadence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SignalAxes {
    pub carrier: Carrier,
    pub baseband_hz: Option<[f64; 2]>,
    pub pulse_rate: Option<Rate>,
    pub complex_convention: Option<ComplexConvention>,
    pub phase_origin: Option<ArtifactLink>,
}
impl SignalAxes {
    pub fn validate(self) -> Result<(), Error> {
        if let Carrier::Hertz(hz) = self.carrier {
            if !hz.is_finite() || hz <= 0.0 {
                return Err(Error::Semantics);
            }
        }
        if let Some([lo, hi]) = self.baseband_hz {
            if !lo.is_finite() || !hi.is_finite() || lo >= hi {
                return Err(Error::Semantics);
            }
        }
        if let Some(r) = self.pulse_rate {
            Rate::new(r.numerator, r.denominator).map_err(|_| Error::Clock)?;
        }
        if let Some(r) = self.phase_origin {
            r.validate()?;
        }
        Ok(())
    }
}
