//! Deterministic fake clock. Monotonic, wall, and energy stay independent.

use crate::net::qdnf::authority::ObservationQuality;
use crate::net::qdnf::errors::QdnfError;

/// Test clock with two time domains and an energy sample that is never inferred.
#[derive(Clone, Copy, Debug)]
pub struct FakeClock {
    /// Steady elapsed time. Never converted into unix seconds.
    pub monotonic_micros: u64,
    /// Explicit wall seconds. Not derived from monotonic advances.
    pub unix_seconds: u64,
    /// Set when monotonic advances without a matching wall update.
    pub wall_uncertain: bool,
    energy_milli_joules: u64,
    energy_quality: ObservationQuality,
}

impl FakeClock {
    pub const fn new(unix_seconds: u64) -> Self {
        Self {
            monotonic_micros: 0,
            unix_seconds,
            wall_uncertain: false,
            energy_milli_joules: 0,
            energy_quality: ObservationQuality::Unknown,
        }
    }

    /// Advance the monotonic domain only. Wall is not implied; energy is unchanged.
    pub fn advance_micros(&mut self, delta: u64) {
        if delta == 0 {
            return;
        }
        self.monotonic_micros = self.monotonic_micros.saturating_add(delta);
        self.wall_uncertain = true;
    }

    /// Advance wall time explicitly. Does not move monotonic or energy.
    pub fn advance_unix_seconds(&mut self, delta: u64) {
        self.unix_seconds = self.unix_seconds.saturating_add(delta);
        self.wall_uncertain = false;
    }

    pub fn mark_wall_uncertain(&mut self) {
        self.wall_uncertain = true;
    }

    pub fn set_energy_measured(&mut self, milli_joules: u64) {
        self.energy_milli_joules = milli_joules;
        self.energy_quality = ObservationQuality::Measured;
    }

    pub fn set_energy_estimated(&mut self, milli_joules: u64) {
        self.energy_milli_joules = milli_joules;
        self.energy_quality = ObservationQuality::Estimated;
    }

    pub fn set_energy_unknown(&mut self) {
        self.energy_quality = ObservationQuality::Unknown;
    }

    pub const fn energy_quality(self) -> ObservationQuality {
        self.energy_quality
    }

    /// Unknown energy is incomplete, never treated as zero joules.
    pub fn energy_milli_joules(self) -> Result<u64, QdnfError> {
        match self.energy_quality {
            ObservationQuality::Unknown => Err(QdnfError::Incomplete),
            ObservationQuality::Measured | ObservationQuality::Estimated => {
                Ok(self.energy_milli_joules)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_domains_distinct() {
        let mut clock = FakeClock::new(1_700_000_000);
        clock.advance_micros(1_500_000);
        assert_eq!(clock.monotonic_micros, 1_500_000);
        assert_eq!(clock.unix_seconds, 1_700_000_000);
        assert!(clock.wall_uncertain);

        clock.advance_unix_seconds(3);
        assert_eq!(clock.unix_seconds, 1_700_000_003);
        assert_eq!(clock.monotonic_micros, 1_500_000);
        assert!(!clock.wall_uncertain);
    }

    #[test]
    fn advance_micros_does_not_zero_unknown_energy() {
        let mut clock = FakeClock::new(10);
        clock.set_energy_unknown();
        clock.advance_micros(2_000_000);
        assert_eq!(clock.energy_quality(), ObservationQuality::Unknown);
        assert_eq!(clock.energy_milli_joules(), Err(QdnfError::Incomplete));
        assert_eq!(clock.unix_seconds, 10);
    }

    #[test]
    fn measured_energy_survives_time_advance() {
        let mut clock = FakeClock::new(0);
        clock.set_energy_measured(42);
        clock.advance_micros(5);
        clock.advance_unix_seconds(9);
        assert_eq!(clock.energy_milli_joules(), Ok(42));
        assert_eq!(clock.energy_quality(), ObservationQuality::Measured);
    }
}
