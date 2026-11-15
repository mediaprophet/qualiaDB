use super::*;
#[test]
fn physical_golden_values_cover_radio_ir_visible_uv_xray_and_gamma() {
    for (hz, wavelength, energy) in [
        (1e-9, 2.99792458e17, 6.62607015e-43),
        (1.0, 299792458.0, 6.62607015e-34),
        (30.0, 9993081.933333334, 1.987821045e-32),
        (2.4e9, 0.12491352416666667, 1.590256836e-24),
        (2.402e9, 0.12480951623646961, 1.59158205003e-24),
        (2.99792458e11, 1e-3, 1.9864458571489286e-22),
        (2.99792458e13, 1e-5, 1.9864458571489286e-20),
        (2.99792458e14, 1e-6, 1.9864458571489286e-19),
        (5.99584916e14, 5e-7, 3.972891714297857e-19),
        (2.99792458e15, 1e-7, 1.9864458571489286e-18),
        (2.99792458e18, 1e-10, 1.9864458571489286e-15),
        (2.99792458e20, 1e-12, 1.9864458571489286e-13),
    ] {
        approx(frequency_hz_to_vacuum_wavelength_m(hz).unwrap(), wavelength);
        approx(vacuum_wavelength_m_to_frequency_hz(wavelength).unwrap(), hz);
        approx(frequency_hz_to_photon_energy_j(hz).unwrap(), energy);
        approx(
            vacuum_wavelength_m_to_photon_energy_j(wavelength).unwrap(),
            energy,
        );
        approx(photon_energy_j_to_frequency_hz(energy).unwrap(), hz);
        approx(
            photon_energy_j_to_vacuum_wavelength_m(energy).unwrap(),
            wavelength,
        );
    }
    approx(
        frequency_hz_to_photon_energy_ev(5.99584916e14).unwrap(),
        2.479683968664005,
    );
}
#[test]
fn zero_nonfinite_and_unrepresentable_values_never_become_carriers() {
    for value in [0.0, -0.0, -1.0] {
        assert_eq!(
            frequency_hz_to_vacuum_wavelength_m(value),
            Err(Error::NonPositive)
        );
        assert_eq!(
            vacuum_wavelength_m_to_frequency_hz(value),
            Err(Error::NonPositive)
        );
        assert_eq!(
            frequency_hz_to_photon_energy_j(value),
            Err(Error::NonPositive)
        );
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            frequency_hz_to_vacuum_wavelength_m(value),
            Err(Error::NonFinite)
        );
        assert_eq!(
            photon_energy_j_to_frequency_hz(value),
            Err(Error::NonFinite)
        );
    }
    assert_eq!(
        frequency_hz_to_vacuum_wavelength_m(f64::from_bits(1)),
        Err(Error::Unrepresentable)
    );
    assert_eq!(
        frequency_hz_to_photon_energy_j(f64::from_bits(1)),
        Err(Error::Unrepresentable)
    );
    assert_eq!(
        photon_energy_j_to_frequency_hz(f64::MAX),
        Err(Error::Unrepresentable)
    );
    assert!(frequency_hz_to_vacuum_wavelength_m(f64::MAX).unwrap() > 0.0);
    assert!(frequency_hz_to_photon_energy_j(f64::MAX)
        .unwrap()
        .is_finite());
    // Direct hc/lambda avoids an unnecessary overflowing intermediate frequency.
    assert!(vacuum_wavelength_m_to_photon_energy_j(1e-310)
        .unwrap()
        .is_finite());
}
#[test]
fn density_jacobians_preserve_measure_and_per_nm_is_explicit() {
    let q = Quantity::SpectralIrradiance;
    let p = SampleSemantics::PointDensity;
    let hz = 6e14;
    let wavelength = VACUUM_SPEED_OF_LIGHT_M_PER_S / hz;
    let metre = density_coordinate_jacobian(
        q,
        p,
        DensityCoordinate::Hertz,
        DensityCoordinate::VacuumMetre,
        hz,
    )
    .unwrap();
    assert_eq!(metre.order, AxisOrder::Reversed);
    approx(
        metre.factor,
        VACUUM_SPEED_OF_LIGHT_M_PER_S / (wavelength * wavelength),
    );
    let nm = density_coordinate_jacobian(
        q,
        p,
        DensityCoordinate::Hertz,
        DensityCoordinate::VacuumNanometre,
        hz,
    )
    .unwrap();
    approx(nm.factor, metre.factor * 1e-9);
    let inverse = density_coordinate_jacobian(
        q,
        p,
        DensityCoordinate::VacuumMetre,
        DensityCoordinate::Hertz,
        wavelength,
    )
    .unwrap();
    approx(metre.factor * inverse.factor, 1.0);
    let source = 2.0;
    let density = metre.apply(source).unwrap();
    approx(
        density_per_metre_to_per_nanometre(q, p, density).unwrap(),
        nm.apply(source).unwrap(),
    );
    approx(
        density_per_nanometre_to_per_metre(q, p, nm.apply(source).unwrap()).unwrap(),
        density,
    );
    assert_eq!(metre.apply(-1.0), Err(Error::NonPositive));
    assert_eq!(metre.apply(f64::NAN), Err(Error::NonFinite));
    assert_eq!(metre.apply(-0.0).unwrap().to_bits(), (-0.0f64).to_bits());
    for quantity in [
        Quantity::DimensionlessReflectance,
        Quantity::BinIntegratedRadiance,
        Quantity::BinIntegratedIrradiance,
    ] {
        assert_eq!(
            density_coordinate_jacobian(
                quantity,
                p,
                DensityCoordinate::Hertz,
                DensityCoordinate::VacuumMetre,
                hz
            ),
            Err(Error::NotDensity)
        );
        assert_eq!(
            density_per_metre_to_per_nanometre(quantity, p, 1.0),
            Err(Error::NotDensity)
        );
    }
    assert_eq!(
        density_coordinate_jacobian(
            q,
            SampleSemantics::BinIntegral,
            DensityCoordinate::Hertz,
            DensityCoordinate::VacuumMetre,
            hz
        ),
        Err(Error::NotDensity)
    );
    assert_eq!(
        density_coordinate_jacobian(
            q,
            SampleSemantics::BinAverage,
            DensityCoordinate::Hertz,
            DensityCoordinate::VacuumMetre,
            hz
        ),
        Err(Error::BinAverageNeedsIntegral)
    );
}
#[test]
fn independent_integral_oracle_and_bin_averages_require_new_width() {
    let c = VACUUM_SPEED_OF_LIGHT_M_PER_S;
    let lo = 1e12;
    let hi = 2e12;
    let density = 2.0;
    let bin = convert_bin_average(
        Quantity::SpectralIrradiance,
        DensityCoordinate::Hertz,
        DensityCoordinate::VacuumMetre,
        lo,
        hi,
        density,
    )
    .unwrap();
    let analytic_hz_integral = density * (hi - lo);
    let analytic_wavelength_integral = density * c * (1.0 / bin.lower - 1.0 / bin.upper);
    approx(bin.integral, analytic_hz_integral);
    approx(analytic_wavelength_integral, analytic_hz_integral);
    approx(bin.average * (bin.upper - bin.lower), analytic_hz_integral);
    assert_eq!(bin.order, AxisOrder::Reversed);
    assert!(
        (density * (bin.upper - bin.lower) - analytic_hz_integral).abs()
            > analytic_hz_integral * 0.99
    );
    let nm = convert_bin_average(
        Quantity::SpectralIrradiance,
        DensityCoordinate::Hertz,
        DensityCoordinate::VacuumNanometre,
        lo,
        hi,
        density,
    )
    .unwrap();
    approx(nm.integral, analytic_hz_integral);
    approx(nm.average, bin.average * 1e-9);
    approx(nm.average * (nm.upper - nm.lower), analytic_hz_integral);
    let back = convert_bin_average(
        Quantity::SpectralIrradiance,
        DensityCoordinate::VacuumMetre,
        DensityCoordinate::Hertz,
        bin.lower,
        bin.upper,
        bin.average,
    )
    .unwrap();
    approx(back.average, density);
    assert_eq!(
        convert_bin_average(
            Quantity::DimensionlessReflectance,
            DensityCoordinate::Hertz,
            DensityCoordinate::VacuumMetre,
            lo,
            hi,
            0.5
        ),
        Err(Error::NotDensity)
    );
    assert_eq!(AxisOrder::Reversed.source_index(0, 3).unwrap(), 2);
    assert_eq!(AxisOrder::Reversed.source_index(3, 3), Err(Error::Index));
}
