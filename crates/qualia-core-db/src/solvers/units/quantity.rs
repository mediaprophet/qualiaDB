//! A dimensioned quantity: a value with a physical [`Dimension`], and arithmetic that
//! is **dimensionally checked**. Adding incompatible dimensions fails closed; products
//! and quotients compose dimensions automatically.

use super::dimension::Dimension;
use super::UnitsError;

/// A value expressed in SI base units, tagged with its physical dimension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    /// Magnitude in coherent SI base units (e.g. metres, kilograms, seconds).
    pub value: f64,
    pub dimension: Dimension,
}

impl Quantity {
    pub const fn new(value: f64, dimension: Dimension) -> Self {
        Self { value, dimension }
    }

    /// A dimensionless number.
    pub const fn scalar(value: f64) -> Self {
        Self {
            value,
            dimension: Dimension::DIMENSIONLESS,
        }
    }

    /// Sum of two quantities — requires matching dimensions (fail closed otherwise).
    pub fn add(&self, other: &Quantity) -> Result<Quantity, UnitsError> {
        if self.dimension != other.dimension {
            return Err(UnitsError::IncompatibleDimensions);
        }
        Ok(Quantity {
            value: self.value + other.value,
            dimension: self.dimension,
        })
    }

    /// Difference — requires matching dimensions.
    pub fn sub(&self, other: &Quantity) -> Result<Quantity, UnitsError> {
        if self.dimension != other.dimension {
            return Err(UnitsError::IncompatibleDimensions);
        }
        Ok(Quantity {
            value: self.value - other.value,
            dimension: self.dimension,
        })
    }

    /// Product — values multiply, dimensions compose.
    pub fn mul(&self, other: &Quantity) -> Quantity {
        Quantity {
            value: self.value * other.value,
            dimension: self.dimension.mul(&other.dimension),
        }
    }

    /// Quotient — values divide, dimensions subtract. `None` on divide-by-zero.
    pub fn div(&self, other: &Quantity) -> Option<Quantity> {
        if other.value == 0.0 {
            return None;
        }
        Some(Quantity {
            value: self.value / other.value,
            dimension: self.dimension.div(&other.dimension),
        })
    }

    /// Scale by a dimensionless factor.
    pub fn scale(&self, factor: f64) -> Quantity {
        Quantity {
            value: self.value * factor,
            dimension: self.dimension,
        }
    }

    /// Integer power — value and dimension both raised to `n`.
    pub fn powi(&self, n: i32) -> Quantity {
        Quantity {
            value: self.value.powi(n),
            dimension: self.dimension.powi(n),
        }
    }

    /// `true` iff dimensionally compatible with `other` (can be added/compared).
    pub fn compatible_with(&self, other: &Quantity) -> bool {
        self.dimension == other.dimension
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f64 = 1e-9;

    fn metres(v: f64) -> Quantity {
        Quantity::new(v, Dimension::LENGTH)
    }
    fn seconds(v: f64) -> Quantity {
        Quantity::new(v, Dimension::TIME)
    }

    #[test]
    fn adding_like_dimensions_works_unlike_fails() {
        let total = metres(3.0).add(&metres(4.0)).unwrap();
        assert!((total.value - 7.0).abs() < EPS);
        assert_eq!(total.dimension, Dimension::LENGTH);
        // length + time is a dimensional error.
        assert_eq!(
            metres(1.0).add(&seconds(1.0)).unwrap_err(),
            UnitsError::IncompatibleDimensions
        );
    }

    #[test]
    fn products_derive_new_dimensions() {
        // distance / time = velocity
        let v = metres(100.0).div(&seconds(10.0)).unwrap();
        assert!((v.value - 10.0).abs() < EPS);
        assert_eq!(v.dimension, Dimension::VELOCITY);
        // force × distance = energy
        let force = Quantity::new(5.0, Dimension::FORCE);
        let work = force.mul(&metres(2.0));
        assert!((work.value - 10.0).abs() < EPS);
        assert_eq!(work.dimension, Dimension::ENERGY);
    }

    #[test]
    fn kinetic_energy_is_dimensionally_consistent() {
        // ½ m v²  →  mass × velocity² = energy.
        let m = Quantity::new(2.0, Dimension::MASS);
        let v = Quantity::new(3.0, Dimension::VELOCITY);
        let ke = m.mul(&v.powi(2)).scale(0.5);
        assert!((ke.value - 9.0).abs() < EPS); // ½·2·9
        assert_eq!(ke.dimension, Dimension::ENERGY);
    }

    #[test]
    fn divide_by_zero_fails_closed() {
        assert!(metres(1.0).div(&seconds(0.0)).is_none());
    }
}
