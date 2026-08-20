//! Quantity dimension algebra and unit conversions (T73).
//!
//! SI base dimensions and derived unit conversions computed via dimensional exponents.

use std::ops::{Div, Mul};

/// SI Base Dimensions: Length (m), Mass (kg), Time (s), Current (A),
/// Temperature (K), Amount of substance (mol), Luminous intensity (cd).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Dimension {
    pub length: i8,      // m (L)
    pub mass: i8,        // kg (M)
    pub time: i8,        // s (T)
    pub current: i8,     // A (I)
    pub temperature: i8, // K (Θ)
    pub amount: i8,      // mol (N)
    pub luminous: i8,    // cd (J)
}

impl Dimension {
    pub const fn dimensionless() -> Self {
        Self {
            length: 0,
            mass: 0,
            time: 0,
            current: 0,
            temperature: 0,
            amount: 0,
            luminous: 0,
        }
    }

    pub const fn length() -> Self {
        let mut d = Self::dimensionless();
        d.length = 1;
        d
    }

    pub const fn mass() -> Self {
        let mut d = Self::dimensionless();
        d.mass = 1;
        d
    }

    pub const fn time() -> Self {
        let mut d = Self::dimensionless();
        d.time = 1;
        d
    }

    pub const fn current() -> Self {
        let mut d = Self::dimensionless();
        d.current = 1;
        d
    }

    pub const fn temperature() -> Self {
        let mut d = Self::dimensionless();
        d.temperature = 1;
        d
    }

    pub const fn amount() -> Self {
        let mut d = Self::dimensionless();
        d.amount = 1;
        d
    }

    pub const fn luminous() -> Self {
        let mut d = Self::dimensionless();
        d.luminous = 1;
        d
    }

    // Common derived dimensions
    pub const fn area() -> Self {
        let mut d = Self::dimensionless();
        d.length = 2;
        d
    }

    pub const fn volume() -> Self {
        let mut d = Self::dimensionless();
        d.length = 3;
        d
    }

    pub const fn velocity() -> Self {
        let mut d = Self::dimensionless();
        d.length = 1;
        d.time = -1;
        d
    }

    pub const fn acceleration() -> Self {
        let mut d = Self::dimensionless();
        d.length = 1;
        d.time = -2;
        d
    }

    pub const fn force() -> Self {
        let mut d = Self::dimensionless();
        d.mass = 1;
        d.length = 1;
        d.time = -2;
        d
    }

    pub const fn pressure() -> Self {
        let mut d = Self::dimensionless();
        d.mass = 1;
        d.length = -1;
        d.time = -2;
        d
    }

    pub const fn energy() -> Self {
        let mut d = Self::dimensionless();
        d.mass = 1;
        d.length = 2;
        d.time = -2;
        d
    }

    pub const fn power() -> Self {
        let mut d = Self::dimensionless();
        d.mass = 1;
        d.length = 2;
        d.time = -3;
        d
    }
}

impl Mul for Dimension {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            length: self.length + rhs.length,
            mass: self.mass + rhs.mass,
            time: self.time + rhs.time,
            current: self.current + rhs.current,
            temperature: self.temperature + rhs.temperature,
            amount: self.amount + rhs.amount,
            luminous: self.luminous + rhs.luminous,
        }
    }
}

impl Div for Dimension {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self {
            length: self.length - rhs.length,
            mass: self.mass - rhs.mass,
            time: self.time - rhs.time,
            current: self.current - rhs.current,
            temperature: self.temperature - rhs.temperature,
            amount: self.amount - rhs.amount,
            luminous: self.luminous - rhs.luminous,
        }
    }
}

/// A physical unit with a dimension, scaling factor to SI base units, and optional affine offset.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    pub symbol: String,
    pub dimension: Dimension,
    pub scale: f64,
    pub offset: f64,
}

impl Unit {
    pub fn new(symbol: impl Into<String>, dimension: Dimension, scale: f64) -> Self {
        Self {
            symbol: symbol.into(),
            dimension,
            scale,
            offset: 0.0,
        }
    }

    pub fn with_offset(
        symbol: impl Into<String>,
        dimension: Dimension,
        scale: f64,
        offset: f64,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            dimension,
            scale,
            offset,
        }
    }

    /// Convert a numeric value from `self` to `target`.
    /// Returns Err if the dimensions do not match.
    pub fn convert(&self, value: f64, target: &Unit) -> Result<f64, String> {
        if self.dimension != target.dimension {
            return Err(format!(
                "incompatible dimensions: cannot convert '{}' ({:?}) to '{}' ({:?})",
                self.symbol, self.dimension, target.symbol, target.dimension
            ));
        }

        // To SI base: (value + self.offset) * self.scale
        let si_base = (value + self.offset) * self.scale;
        // From SI base: (si_base / target.scale) - target.offset
        let target_val = (si_base / target.scale) - target.offset;
        Ok(target_val)
    }
}

/// Look up standard units by symbol.
pub fn lookup_unit(symbol: &str) -> Option<Unit> {
    match symbol {
        "m" => Some(Unit::new("m", Dimension::length(), 1.0)),
        "km" => Some(Unit::new("km", Dimension::length(), 1_000.0)),
        "cm" => Some(Unit::new("cm", Dimension::length(), 0.01)),
        "mm" => Some(Unit::new("mm", Dimension::length(), 0.001)),
        "s" => Some(Unit::new("s", Dimension::time(), 1.0)),
        "ms" => Some(Unit::new("ms", Dimension::time(), 0.001)),
        "kg" => Some(Unit::new("kg", Dimension::mass(), 1.0)),
        "g" => Some(Unit::new("g", Dimension::mass(), 0.001)),
        "A" => Some(Unit::new("A", Dimension::current(), 1.0)),
        "K" => Some(Unit::new("K", Dimension::temperature(), 1.0)),
        "°C" | "degC" => Some(Unit::with_offset(
            symbol,
            Dimension::temperature(),
            1.0,
            273.15,
        )),
        "N" => Some(Unit::new("N", Dimension::force(), 1.0)),
        "kN" => Some(Unit::new("kN", Dimension::force(), 1_000.0)),
        "Pa" => Some(Unit::new("Pa", Dimension::pressure(), 1.0)),
        "kPa" => Some(Unit::new("kPa", Dimension::pressure(), 1_000.0)),
        "MPa" => Some(Unit::new("MPa", Dimension::pressure(), 1_000_000.0)),
        "N/m²" => Some(Unit::new("N/m²", Dimension::pressure(), 1.0)),
        "J" => Some(Unit::new("J", Dimension::energy(), 1.0)),
        "kJ" => Some(Unit::new("kJ", Dimension::energy(), 1_000.0)),
        "W" => Some(Unit::new("W", Dimension::power(), 1.0)),
        "kW" => Some(Unit::new("kW", Dimension::power(), 1_000.0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_multiplication() {
        let force = Dimension::force();
        let length = Dimension::length();
        let energy = force * length;
        assert_eq!(energy, Dimension::energy());
    }

    #[test]
    fn kpa_to_pa() {
        let kpa = lookup_unit("kPa").unwrap();
        let pa = lookup_unit("Pa").unwrap();
        let result = kpa.convert(1.0, &pa).unwrap();
        assert!((result - 1000.0).abs() < 1e-9);
    }

    #[test]
    fn pa_to_n_per_m2() {
        let pa = lookup_unit("Pa").unwrap();
        let n_m2 = lookup_unit("N/m²").unwrap();
        let result = pa.convert(1.0, &n_m2).unwrap();
        assert!((result - 1.0).abs() < 1e-9);
    }

    #[test]
    fn dimension_mismatch_fails() {
        let pa = lookup_unit("Pa").unwrap();
        let m = lookup_unit("m").unwrap();
        assert!(pa.convert(1.0, &m).is_err());
    }

    #[test]
    fn celsius_to_kelvin() {
        let c = lookup_unit("°C").unwrap();
        let k = lookup_unit("K").unwrap();
        let result = c.convert(0.0, &k).unwrap();
        assert!((result - 273.15).abs() < 1e-9);
    }
}
