//! Units and conversion. A [`Unit`] maps its own scale to coherent SI by an affine
//! transform `si = value·factor + offset` (the offset is only non-zero for the
//! temperature scales — Celsius, Fahrenheit). Conversion between two units requires
//! matching dimensions and fails closed otherwise.

use super::dimension::Dimension;
use super::quantity::Quantity;
use super::UnitsError;

/// A named unit of a given dimension, defined by its affine map to coherent SI base
/// units: `si_value = value * to_si_factor + to_si_offset`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub name: &'static str,
    pub dimension: Dimension,
    pub to_si_factor: f64,
    pub to_si_offset: f64,
}

impl Unit {
    pub const fn linear(name: &'static str, dimension: Dimension, factor: f64) -> Self {
        Self { name, dimension, to_si_factor: factor, to_si_offset: 0.0 }
    }
    pub const fn affine(name: &'static str, dimension: Dimension, factor: f64, offset: f64) -> Self {
        Self { name, dimension, to_si_factor: factor, to_si_offset: offset }
    }

    /// Convert a magnitude in this unit to coherent SI.
    pub fn to_si(&self, value: f64) -> f64 {
        value * self.to_si_factor + self.to_si_offset
    }
    /// Convert a magnitude in coherent SI back to this unit.
    pub fn from_si(&self, si: f64) -> f64 {
        (si - self.to_si_offset) / self.to_si_factor
    }

    /// A magnitude in this unit as a dimensioned [`Quantity`] (in SI).
    pub fn quantity(&self, value: f64) -> Quantity {
        Quantity::new(self.to_si(value), self.dimension)
    }

    // ── Length ──
    pub const METRE: Unit = Unit::linear("m", Dimension::LENGTH, 1.0);
    pub const KILOMETRE: Unit = Unit::linear("km", Dimension::LENGTH, 1000.0);
    pub const CENTIMETRE: Unit = Unit::linear("cm", Dimension::LENGTH, 0.01);
    pub const MILLIMETRE: Unit = Unit::linear("mm", Dimension::LENGTH, 0.001);
    pub const INCH: Unit = Unit::linear("in", Dimension::LENGTH, 0.0254);
    pub const FOOT: Unit = Unit::linear("ft", Dimension::LENGTH, 0.3048);
    pub const MILE: Unit = Unit::linear("mi", Dimension::LENGTH, 1609.344);
    // ── Mass ──
    pub const KILOGRAM: Unit = Unit::linear("kg", Dimension::MASS, 1.0);
    pub const GRAM: Unit = Unit::linear("g", Dimension::MASS, 0.001);
    pub const POUND: Unit = Unit::linear("lb", Dimension::MASS, 0.45359237);
    // ── Time ──
    pub const SECOND: Unit = Unit::linear("s", Dimension::TIME, 1.0);
    pub const MINUTE: Unit = Unit::linear("min", Dimension::TIME, 60.0);
    pub const HOUR: Unit = Unit::linear("h", Dimension::TIME, 3600.0);
    // ── Force / energy / pressure ──
    pub const NEWTON: Unit = Unit::linear("N", Dimension::FORCE, 1.0);
    pub const JOULE: Unit = Unit::linear("J", Dimension::ENERGY, 1.0);
    pub const KILOWATT_HOUR: Unit = Unit::linear("kWh", Dimension::ENERGY, 3.6e6);
    pub const PASCAL: Unit = Unit::linear("Pa", Dimension::PRESSURE, 1.0);
    pub const BAR: Unit = Unit::linear("bar", Dimension::PRESSURE, 1.0e5);
    // ── Temperature (affine) ──
    pub const KELVIN: Unit = Unit::linear("K", Dimension::TEMPERATURE, 1.0);
    pub const CELSIUS: Unit = Unit::affine("°C", Dimension::TEMPERATURE, 1.0, 273.15);
    pub const FAHRENHEIT: Unit = Unit::affine("°F", Dimension::TEMPERATURE, 5.0 / 9.0, 255.372_222_222_222_2);
}

/// Convert `value` from one unit to another. Fails closed if the units have different
/// dimensions (e.g. metres → seconds).
pub fn convert(value: f64, from: &Unit, to: &Unit) -> Result<f64, UnitsError> {
    if from.dimension != to.dimension {
        return Err(UnitsError::IncompatibleDimensions);
    }
    Ok(to.from_si(from.to_si(value)))
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-6;

    #[test]
    fn length_conversions() {
        assert!((convert(1.0, &Unit::INCH, &Unit::CENTIMETRE).unwrap() - 2.54).abs() < EPS);
        assert!((convert(1.0, &Unit::MILE, &Unit::KILOMETRE).unwrap() - 1.609344).abs() < EPS);
        assert!((convert(3.0, &Unit::FOOT, &Unit::METRE).unwrap() - 0.9144).abs() < EPS);
    }

    #[test]
    fn temperature_is_affine() {
        // 0 °C = 273.15 K
        assert!((convert(0.0, &Unit::CELSIUS, &Unit::KELVIN).unwrap() - 273.15).abs() < EPS);
        // 100 °C = 212 °F
        assert!((convert(100.0, &Unit::CELSIUS, &Unit::FAHRENHEIT).unwrap() - 212.0).abs() < 1e-3);
        // 32 °F = 0 °C
        assert!((convert(32.0, &Unit::FAHRENHEIT, &Unit::CELSIUS).unwrap()).abs() < 1e-3);
        // −40 °C = −40 °F (the classic crossover)
        assert!((convert(-40.0, &Unit::CELSIUS, &Unit::FAHRENHEIT).unwrap() + 40.0).abs() < 1e-3);
    }

    #[test]
    fn energy_conversion() {
        // 1 kWh = 3.6 MJ
        assert!((convert(1.0, &Unit::KILOWATT_HOUR, &Unit::JOULE).unwrap() - 3.6e6).abs() < 1.0);
    }

    #[test]
    fn cross_dimension_conversion_fails_closed() {
        assert_eq!(
            convert(1.0, &Unit::METRE, &Unit::SECOND).unwrap_err(),
            UnitsError::IncompatibleDimensions
        );
    }
}
