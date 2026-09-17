//! Piecewise stardate morphism (OCS §12.1).
//!
//! Converts Star Trek stardates to approximate Gregorian years and back.
//!
//! Reference: OCS Specification v2.2.0 §12.1.

/// Stardate era (OCS §12.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StardateEra {
    /// TOS: 0 ≤ S < 10000, Gregorian(2265.0 + 0.1·S)
    Tos,
    /// TNG/DS9/VOY: 41000 ≤ S < 60000, Gregorian(2364.0 + (S-41000)/1000)
    Tng,
    /// 32nd Century: S ≥ 860000, Gregorian(3188.0 + (S-860000)/1000)
    Century32,
}

/// A parsed stardate with its era.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stardate {
    pub value: f64,
    pub era: StardateEra,
}

impl Stardate {
    /// Create a stardate from a raw value, auto-detecting the era.
    pub fn new(s: f64) -> Self {
        let era = if s >= 860_000.0 {
            StardateEra::Century32
        } else if s >= 41_000.0 {
            StardateEra::Tng
        } else {
            StardateEra::Tos
        };
        Self { value: s, era }
    }

    /// Convert stardate to approximate Gregorian year (OCS §12.1).
    pub fn to_gregorian_year(&self) -> f64 {
        match self.era {
            StardateEra::Tos => 2265.0 + 0.1 * self.value,
            StardateEra::Tng => 2364.0 + (self.value - 41_000.0) / 1000.0,
            StardateEra::Century32 => 3188.0 + (self.value - 860_000.0) / 1000.0,
        }
    }

    /// Convert a Gregorian year to the nearest stardate in the given era.
    pub fn from_gregorian_year(year: f64, era: StardateEra) -> Self {
        let value = match era {
            StardateEra::Tos => (year - 2265.0) / 0.1,
            StardateEra::Tng => (year - 2364.0) * 1000.0 + 41_000.0,
            StardateEra::Century32 => (year - 3188.0) * 1000.0 + 860_000.0,
        };
        Self { value, era }
    }
}

impl StardateEra {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Tos => "TOS",
            Self::Tng => "TNG",
            Self::Century32 => "32nd Century",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tos_stardate_1312() {
        // TOS stardate 1312.4 → ~2265.0 + 131.24 = 2396.24... wait
        // 2265.0 + 0.1 * 1312.4 = 2265.0 + 131.24 = 2396.24
        // Actually TOS stardates are roughly 2265-2269 range
        // 0.1 * 1312.4 = 131.24 → year 2396.24? That seems wrong.
        // The spec says Gregorian(2265.0 + 0.1·S), so S=1312.4 → 2265+131.24 = 2396.24
        // This is the spec formula, even if it doesn't match fan chronologies exactly.
        let s = Stardate::new(1312.4);
        assert_eq!(s.era, StardateEra::Tos);
        let year = s.to_gregorian_year();
        assert!((year - 2396.24).abs() < 0.01);
    }

    #[test]
    fn tng_stardate_47634() {
        // TNG stardate 47634.44 → 2364.0 + (47634.44-41000)/1000 = 2364 + 6.63444 = 2370.634
        let s = Stardate::new(47634.44);
        assert_eq!(s.era, StardateEra::Tng);
        let year = s.to_gregorian_year();
        assert!((year - 2370.63444).abs() < 0.001);
    }

    #[test]
    fn century32_stardate_865211() {
        // 32nd century stardate 865211.2 → 3188.0 + (865211.2-860000)/1000 = 3188 + 5.2112 = 3193.211
        let s = Stardate::new(865211.2);
        assert_eq!(s.era, StardateEra::Century32);
        let year = s.to_gregorian_year();
        assert!((year - 3193.2112).abs() < 0.001);
    }

    #[test]
    fn round_trip_tng() {
        let original = Stardate::new(47634.44);
        let year = original.to_gregorian_year();
        let recovered = Stardate::from_gregorian_year(year, StardateEra::Tng);
        assert!((recovered.value - original.value).abs() < 0.001);
    }

    #[test]
    fn round_trip_tos() {
        let original = Stardate::new(1312.4);
        let year = original.to_gregorian_year();
        let recovered = Stardate::from_gregorian_year(year, StardateEra::Tos);
        assert!((recovered.value - original.value).abs() < 0.001);
    }

    #[test]
    fn round_trip_century32() {
        let original = Stardate::new(865211.2);
        let year = original.to_gregorian_year();
        let recovered = Stardate::from_gregorian_year(year, StardateEra::Century32);
        assert!((recovered.value - original.value).abs() < 0.001);
    }

    #[test]
    fn era_detection() {
        assert_eq!(Stardate::new(500.0).era, StardateEra::Tos);
        assert_eq!(Stardate::new(50000.0).era, StardateEra::Tng);
        assert_eq!(Stardate::new(900000.0).era, StardateEra::Century32);
    }
}
