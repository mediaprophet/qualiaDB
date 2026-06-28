//! Physical dimension as the 7-vector of SI base-dimension exponents.
//!
//! Order: length (m), mass (kg), time (s), electric current (A), thermodynamic
//! temperature (K), amount of substance (mol), luminous intensity (cd). A `Dimension`
//! is these seven signed exponents; products add exponents, quotients subtract,
//! powers scale. All-zero is dimensionless.

/// The seven SI base-dimension exponents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimension {
    pub exponents: [i8; 7],
}

/// Indices into the exponent vector.
pub const LENGTH: usize = 0;
pub const MASS: usize = 1;
pub const TIME: usize = 2;
pub const CURRENT: usize = 3;
pub const TEMPERATURE: usize = 4;
pub const AMOUNT: usize = 5;
pub const LUMINOSITY: usize = 6;

impl Dimension {
    pub const fn new(exponents: [i8; 7]) -> Self {
        Self { exponents }
    }

    /// A base dimension with exponent 1 at `index`.
    const fn base(index: usize) -> Self {
        let mut e = [0i8; 7];
        e[index] = 1;
        Self { exponents: e }
    }

    pub const DIMENSIONLESS: Dimension = Dimension::new([0; 7]);
    pub const LENGTH: Dimension = Dimension::base(LENGTH);
    pub const MASS: Dimension = Dimension::base(MASS);
    pub const TIME: Dimension = Dimension::base(TIME);
    pub const CURRENT: Dimension = Dimension::base(CURRENT);
    pub const TEMPERATURE: Dimension = Dimension::base(TEMPERATURE);
    pub const AMOUNT: Dimension = Dimension::base(AMOUNT);
    pub const LUMINOSITY: Dimension = Dimension::base(LUMINOSITY);

    /// Area = L², Volume = L³.
    pub const AREA: Dimension = Dimension::new([2, 0, 0, 0, 0, 0, 0]);
    pub const VOLUME: Dimension = Dimension::new([3, 0, 0, 0, 0, 0, 0]);
    /// Velocity = L·T⁻¹, Acceleration = L·T⁻².
    pub const VELOCITY: Dimension = Dimension::new([1, 0, -1, 0, 0, 0, 0]);
    pub const ACCELERATION: Dimension = Dimension::new([1, 0, -2, 0, 0, 0, 0]);
    /// Force = M·L·T⁻² (newton).
    pub const FORCE: Dimension = Dimension::new([1, 1, -2, 0, 0, 0, 0]);
    /// Energy = M·L²·T⁻² (joule); Power = M·L²·T⁻³ (watt).
    pub const ENERGY: Dimension = Dimension::new([2, 1, -2, 0, 0, 0, 0]);
    pub const POWER: Dimension = Dimension::new([2, 1, -3, 0, 0, 0, 0]);
    /// Pressure = M·L⁻¹·T⁻² (pascal).
    pub const PRESSURE: Dimension = Dimension::new([-1, 1, -2, 0, 0, 0, 0]);
    /// Electric charge = T·A (coulomb).
    pub const CHARGE: Dimension = Dimension::new([0, 0, 1, 1, 0, 0, 0]);
    /// Frequency = T⁻¹ (hertz).
    pub const FREQUENCY: Dimension = Dimension::new([0, 0, -1, 0, 0, 0, 0]);

    pub fn is_dimensionless(&self) -> bool {
        self.exponents == [0; 7]
    }

    /// Dimension of a product: exponents add (saturating to keep `i8`).
    pub fn mul(&self, other: &Dimension) -> Dimension {
        let mut e = [0i8; 7];
        for i in 0..7 {
            e[i] = self.exponents[i].saturating_add(other.exponents[i]);
        }
        Dimension { exponents: e }
    }

    /// Dimension of a quotient: exponents subtract.
    pub fn div(&self, other: &Dimension) -> Dimension {
        let mut e = [0i8; 7];
        for i in 0..7 {
            e[i] = self.exponents[i].saturating_sub(other.exponents[i]);
        }
        Dimension { exponents: e }
    }

    /// Dimension raised to an integer power: exponents scale.
    pub fn powi(&self, n: i32) -> Dimension {
        let mut e = [0i8; 7];
        for i in 0..7 {
            e[i] = (self.exponents[i] as i32 * n).clamp(i8::MIN as i32, i8::MAX as i32) as i8;
        }
        Dimension { exponents: e }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn products_and_quotients_compose() {
        // velocity = length / time
        assert_eq!(Dimension::LENGTH.div(&Dimension::TIME), Dimension::VELOCITY);
        // force × length = energy (work)
        assert_eq!(Dimension::FORCE.mul(&Dimension::LENGTH), Dimension::ENERGY);
        // energy / time = power
        assert_eq!(Dimension::ENERGY.div(&Dimension::TIME), Dimension::POWER);
        // force / area = pressure
        assert_eq!(Dimension::FORCE.div(&Dimension::AREA), Dimension::PRESSURE);
    }

    #[test]
    fn powers_scale_exponents() {
        assert_eq!(Dimension::LENGTH.powi(2), Dimension::AREA);
        assert_eq!(Dimension::LENGTH.powi(3), Dimension::VOLUME);
        // velocity² has dimension L²T⁻².
        assert_eq!(
            Dimension::VELOCITY.powi(2),
            Dimension::new([2, 0, -2, 0, 0, 0, 0])
        );
    }

    #[test]
    fn dimensionless_detection() {
        assert!(Dimension::DIMENSIONLESS.is_dimensionless());
        // velocity / velocity = dimensionless
        assert!(Dimension::VELOCITY
            .div(&Dimension::VELOCITY)
            .is_dimensionless());
        assert!(!Dimension::FORCE.is_dimensionless());
    }
}
