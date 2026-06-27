//! Physical constants as dimensioned [`Quantity`]s (CODATA / SI-2019 defined values
//! where exact). Each carries its dimension so it composes correctly in unit-checked
//! arithmetic — e.g. `½·m·v²` divided by `k_B·T` is dimensionless by construction.

use super::dimension::Dimension;
use super::quantity::Quantity;

/// Speed of light in vacuum, `c` (exact, SI-2019). 299 792 458 m/s.
pub const SPEED_OF_LIGHT: Quantity = Quantity::new(299_792_458.0, Dimension::VELOCITY);

/// Newtonian constant of gravitation, `G`. m³·kg⁻¹·s⁻².
pub const GRAVITATIONAL: Quantity =
    Quantity::new(6.674_30e-11, Dimension::new([3, -1, -2, 0, 0, 0, 0]));

/// Planck constant, `h` (exact, SI-2019). J·s = m²·kg·s⁻¹.
pub const PLANCK: Quantity = Quantity::new(6.626_070_15e-34, Dimension::new([2, 1, -1, 0, 0, 0, 0]));

/// Reduced Planck constant, `ħ = h/2π`. Same dimension as `h`.
pub const REDUCED_PLANCK: Quantity = Quantity::new(
    6.626_070_15e-34 / (2.0 * core::f64::consts::PI),
    Dimension::new([2, 1, -1, 0, 0, 0, 0]),
);

/// Boltzmann constant, `k_B` (exact, SI-2019). J/K = m²·kg·s⁻²·K⁻¹.
pub const BOLTZMANN: Quantity = Quantity::new(1.380_649e-23, Dimension::new([2, 1, -2, 0, -1, 0, 0]));

/// Avogadro constant, `N_A` (exact, SI-2019). mol⁻¹.
pub const AVOGADRO: Quantity = Quantity::new(6.022_140_76e23, Dimension::new([0, 0, 0, 0, 0, -1, 0]));

/// Elementary charge, `e` (exact, SI-2019). Coulomb = A·s.
pub const ELEMENTARY_CHARGE: Quantity =
    Quantity::new(1.602_176_634e-19, Dimension::CHARGE);

/// Molar gas constant, `R = N_A·k_B` (exact). J·mol⁻¹·K⁻¹.
pub const GAS_CONSTANT: Quantity =
    Quantity::new(8.314_462_618_153_24, Dimension::new([2, 1, -2, 0, -1, -1, 0]));

/// Stefan–Boltzmann constant, `σ`. W·m⁻²·K⁻⁴ = kg·s⁻³·K⁻⁴.
pub const STEFAN_BOLTZMANN: Quantity =
    Quantity::new(5.670_374_419e-8, Dimension::new([0, 1, -3, 0, -4, 0, 0]));

/// Standard gravity, `g₀` (defined). m/s².
pub const STANDARD_GRAVITY: Quantity = Quantity::new(9.806_65, Dimension::ACCELERATION);

/// Standard atmosphere (defined). 101 325 Pa.
pub const STANDARD_ATMOSPHERE: Quantity = Quantity::new(101_325.0, Dimension::PRESSURE);

/// Electron mass, `m_e`. kg.
pub const ELECTRON_MASS: Quantity = Quantity::new(9.109_383_7015e-31, Dimension::MASS);
/// Proton mass, `m_p`. kg.
pub const PROTON_MASS: Quantity = Quantity::new(1.672_621_923_69e-27, Dimension::MASS);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_carry_correct_dimensions() {
        assert_eq!(SPEED_OF_LIGHT.dimension, Dimension::VELOCITY);
        assert_eq!(STANDARD_GRAVITY.dimension, Dimension::ACCELERATION);
        assert_eq!(ELEMENTARY_CHARGE.dimension, Dimension::CHARGE);
        assert_eq!(STANDARD_ATMOSPHERE.dimension, Dimension::PRESSURE);
    }

    #[test]
    fn gas_constant_is_avogadro_times_boltzmann() {
        // R = N_A · k_B, both value and dimension.
        let r = AVOGADRO.mul(&BOLTZMANN);
        assert!((r.value - GAS_CONSTANT.value).abs() / GAS_CONSTANT.value < 1e-9);
        assert_eq!(r.dimension, GAS_CONSTANT.dimension);
    }

    #[test]
    fn photon_energy_e_equals_h_nu_is_dimensionally_energy() {
        // E = h·ν, with ν a frequency (s⁻¹) → energy.
        let nu = Quantity::new(5.0e14, Dimension::FREQUENCY); // visible light
        let e = PLANCK.mul(&nu);
        assert_eq!(e.dimension, Dimension::ENERGY);
        assert!(e.value > 0.0);
    }

    #[test]
    fn thermal_energy_kt_is_energy() {
        let t = Quantity::new(300.0, Dimension::TEMPERATURE);
        let kt = BOLTZMANN.mul(&t);
        assert_eq!(kt.dimension, Dimension::ENERGY);
    }
}
