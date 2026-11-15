use super::*;
#[test]
fn full_spectrum_axes_have_no_instrument_or_visible_band_cutoff() {
    let frequencies = [
        1e-9,
        30.0,
        1e3,
        2.4e9,
        2.402e9,
        5.8e9,
        2.99792458e11,
        2.99792458e13,
        2.99792458e14,
        5.99584916e14,
        2.99792458e15,
        2.99792458e18,
        2.99792458e20,
    ];
    let fields = axis_fields(
        "frequency_hz",
        "point_density",
        "spectral_irradiance",
        "W m^-2 Hz^-1",
        &frequencies,
    );
    with_axis(&fields, frequencies.len() as u64, |result| {
        let axis = result.unwrap();
        assert_eq!(axis.coordinates().collect::<Vec<_>>(), frequencies);
        assert_eq!(
            axis.conversion_order(AxisKind::VacuumWavelengthMetres),
            AxisOrder::Reversed
        );
        for i in 0..axis.coordinate_count() {
            let source = axis
                .source_coordinate_index_for_ascending(AxisKind::VacuumWavelengthMetres, i)
                .unwrap();
            assert_eq!(source, axis.coordinate_count() - 1 - i);
            approx(
                axis.ascending_derived_coordinate(AxisKind::VacuumWavelengthMetres, i)
                    .unwrap(),
                VACUUM_SPEED_OF_LIGHT_M_PER_S / frequencies[source as usize],
            );
        }
        assert_eq!(axis.coordinates().collect::<Vec<_>>(), frequencies);
        assert_eq!(axis.coordinate(axis.coordinate_count()), Err(Error::Index));
    });
    let extremes = [f64::from_bits(1), f64::MIN_POSITIVE, 1.0, f64::MAX];
    with_axis(
        &axis_fields(
            "frequency_hz",
            "point_density",
            "spectral_irradiance",
            "W m^-2 Hz^-1",
            &extremes,
        ),
        4,
        |r| assert!(r.is_ok()),
    );
    let wavelengths = [
        1e-12,
        1e-10,
        1e-7,
        5e-7,
        1e-6,
        1e-5,
        1e-3,
        0.12491352416666667,
        1e3,
    ];
    with_axis(
        &axis_fields(
            "vacuum_wavelength_m",
            "point_density",
            "spectral_radiance",
            "W sr^-1 m^-2 m^-1",
            &wavelengths,
        ),
        wavelengths.len() as u64,
        |r| assert!(r.is_ok()),
    );
}
#[test]
fn exact_quantity_unit_and_sample_semantics_pairs() {
    for kind in [AxisKind::FrequencyHz, AxisKind::VacuumWavelengthMetres] {
        for q in [
            Quantity::SpectralRadiance,
            Quantity::SpectralIrradiance,
            Quantity::BinIntegratedRadiance,
            Quantity::BinIntegratedIrradiance,
            Quantity::DimensionlessReflectance,
        ] {
            let integrated = matches!(
                q,
                Quantity::BinIntegratedRadiance | Quantity::BinIntegratedIrradiance
            );
            let name = if kind == AxisKind::FrequencyHz {
                "frequency_hz"
            } else {
                "vacuum_wavelength_m"
            };
            let semantics = if integrated {
                "bin_integral"
            } else {
                "point_density"
            };
            let coords = if integrated {
                &[1.0, 2.0][..]
            } else {
                &[1.0][..]
            };
            with_axis(
                &axis_fields(name, semantics, q.name(), q.unit(kind), coords),
                1,
                |r| assert!(r.is_ok()),
            );
        }
    }
    with_axis(
        &axis_fields(
            "frequency_hz",
            "bin_average",
            "spectral_irradiance",
            "W m^-2 Hz^-1",
            &[1.0, 2.0],
        ),
        1,
        |r| assert!(r.is_ok()),
    );
    for (semantics, quantity, unit) in [
        ("bin_integral", "spectral_irradiance", "W m^-2 Hz^-1"),
        ("point_density", "bin_integrated_irradiance", "W m^-2"),
        ("bin_integral", "dimensionless_reflectance", "1"),
        ("point_density", "Spectral_Irradiance", "W m^-2 Hz^-1"),
        ("point_density", "spectral_irradiance", "W m^-2 nm^-1"),
    ] {
        with_axis(
            &axis_fields("frequency_hz", semantics, quantity, unit, &[1.0, 2.0]),
            1,
            |r| assert_eq!(r.unwrap_err(), Error::QuantityUnit),
        );
    }
}
#[test]
fn counts_coordinates_rules_and_budgets_fail_closed() {
    let good = axis_fields(
        "frequency_hz",
        "point_density",
        "spectral_irradiance",
        "W m^-2 Hz^-1",
        &[1.0, 2.0],
    );
    with_axis(&good, 1, |r| assert_eq!(r.unwrap_err(), Error::Count));
    with_axis(&good, 0, |r| assert_eq!(r.unwrap_err(), Error::Count));
    for coords in [
        &[2.0, 1.0][..],
        &[1.0, 1.0][..],
        &[0.0, 1.0][..],
        &[-1.0, 1.0][..],
    ] {
        with_axis(
            &axis_fields(
                "frequency_hz",
                "point_density",
                "spectral_irradiance",
                "W m^-2 Hz^-1",
                coords,
            ),
            2,
            |r| assert!(r.is_err()),
        );
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        // The structural envelope rejects nonfinite primitive metadata before a Block exists.
        let fields = axis_fields(
            "frequency_hz",
            "point_density",
            "spectral_irradiance",
            "W m^-2 Hz^-1",
            &[value],
        );
        let metadata = block_bytes(1, 132, &fields);
        assert!(EnvelopeInput {
            family: Family::EmfV4Candidate,
            flags: 0,
            metadata: &metadata,
            entries: &[]
        }
        .encoded_len(StorageLimits::default())
        .is_err());
    }
    let mut missing = good.clone();
    missing.remove(6);
    with_axis(&missing, 2, |r| assert!(matches!(r, Err(Error::Shared(_)))));
    let mut critical = good.clone();
    critical[0][3] = 0;
    with_axis(&critical, 2, |r| {
        assert_eq!(
            r.unwrap_err(),
            Error::Shared(super::super::super::field_semantics::Error::RequiredCritical)
        )
    });
    let mut unknown = good.clone();
    unknown.push(text_field(99, "unsupported"));
    with_axis(&unknown, 2, |r| {
        assert_eq!(
            r.unwrap_err(),
            Error::Shared(super::super::super::field_semantics::Error::UnknownCriticalField)
        )
    });
    let bytes = encoded(&good);
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StorageLimits::default()).unwrap();
    let block = view.blocks().next().unwrap();
    assert_eq!(
        admit_axis(
            block,
            2,
            Limits {
                work: 0,
                ..Limits::default()
            },
            &NoResponse
        )
        .unwrap_err(),
        Error::Limit
    );
    assert_eq!(
        admit_axis(
            block,
            2,
            Limits {
                samples: 1,
                ..Limits::default()
            },
            &NoResponse
        )
        .unwrap_err(),
        Error::Limit
    );
    let bins = axis_fields(
        "frequency_hz",
        "bin_integral",
        "bin_integrated_irradiance",
        "W m^-2",
        &[1.0, 2.0],
    );
    let bytes = encoded(&bins);
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StorageLimits::default()).unwrap();
    assert_eq!(
        admit_axis(
            view.blocks().next().unwrap(),
            u64::MAX,
            Limits {
                samples: u64::MAX,
                ..Limits::default()
            },
            &NoResponse
        )
        .unwrap_err(),
        Error::Overflow
    );
}
#[test]
fn sigma_profiles_are_explicit_and_legacy_display_is_not_physical() {
    for name in [
        "legacy_display_400_700nm",
        "legacy_etalon_display_380_750nm",
    ] {
        let mut fields = axis_fields(
            "frequency_hz",
            "point_density",
            "spectral_irradiance",
            "W m^-2 Hz^-1",
            &[1.0],
        );
        fields[6] = text_field(8, name);
        with_axis(&fields, 1, |r| {
            assert_eq!(r.unwrap().log_frequency_sigma(1.0), Err(Error::Sigma))
        });
        fields.push(float_field(9, &[1.0, 2.0]));
        with_axis(&fields, 1, |r| assert_eq!(r.unwrap_err(), Error::Sigma));
    }
    let mut fields = axis_fields(
        "frequency_hz",
        "point_density",
        "spectral_irradiance",
        "W m^-2 Hz^-1",
        &[1e-100, 1.0, 1e100],
    );
    fields[6] = text_field(8, "log_frequency_v1");
    fields.push(float_field(9, &[1e-100, 1e100]));
    with_axis(&fields, 3, |r| {
        let axis = r.unwrap();
        assert_eq!(axis.log_frequency_sigma(1e-100).unwrap(), 0.0);
        assert_eq!(axis.log_frequency_sigma(1e100).unwrap(), 1.0);
        approx(axis.log_frequency_sigma(1.0).unwrap(), 0.5);
        assert_eq!(axis.log_frequency_sigma(1e101), Err(Error::Sigma));
    });
    fields[7] = float_field(9, &[2.0, 1.0]);
    with_axis(&fields, 3, |r| assert_eq!(r.unwrap_err(), Error::Sigma));
    let near = 1.0 + f64::EPSILON;
    let mut fields = axis_fields(
        "frequency_hz",
        "point_density",
        "spectral_irradiance",
        "W m^-2 Hz^-1",
        &[1.0, near],
    );
    fields[6] = text_field(8, "log_frequency_v1");
    fields.push(float_field(9, &[1.0, near]));
    with_axis(&fields, 2, |r| {
        assert_eq!(r.unwrap().log_frequency_sigma(near).unwrap(), 1.0)
    });
}
#[test]
fn response_needs_typed_profile_and_explicit_normalization() {
    struct Profile<'a> {
        block: Block<'a>,
    }
    impl ResponseValidator for Profile<'_> {
        fn validate(&self, id: u32, budget: &mut WorkBudget) -> Result<(), Error> {
            budget.charge(self.block.raw_bytes().len() as u64)?;
            if id != self.block.id()
                || self.block.schema_type() != 990
                || self.block.revision() != 1
            {
                return Err(Error::UnsupportedResponse);
            }
            if field(self.block, 1).map(|f| f.value()) != Some(&b"unit-peak"[..]) {
                return Err(Error::UnsupportedResponse);
            }
            Ok(())
        }
    }
    let mut fields = axis_fields(
        "frequency_hz",
        "point_density",
        "spectral_irradiance",
        "W m^-2 Hz^-1",
        &[1.0],
    );
    fields.insert(6, field_bytes(7, 8, 1, true, &2u32.to_le_bytes()));
    let mut metadata = block_bytes(1, 132, &fields);
    metadata.extend_from_slice(&block_bytes(2, 990, &[text_field(1, "unit-peak")]));
    let bytes = storage(&metadata);
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StorageLimits::default()).unwrap();
    let block = view.blocks().next().unwrap();
    assert_eq!(
        admit_axis(block, 1, Limits::default(), &NoResponse).unwrap_err(),
        Error::UnsupportedResponse
    );
    let profile = Profile {
        block: view.blocks().nth(1).unwrap(),
    };
    assert_eq!(
        admit_axis(block, 1, Limits::default(), &profile)
            .unwrap()
            .response_metadata_id(),
        Some(2)
    );
}

#[test]
fn bin_edges_and_sample_values_have_separate_explicit_permutations() {
    let fields = axis_fields(
        "frequency_hz",
        "bin_average",
        "spectral_irradiance",
        "W m^-2 Hz^-1",
        &[1.0, 2.0, 3.0],
    );
    with_axis(&fields, 2, |r| {
        let axis = r.unwrap();
        let target = AxisKind::VacuumWavelengthMetres;
        assert_eq!(axis.coordinate_count(), 3);
        assert_eq!(axis.sample_count(), 2);
        assert_eq!(
            axis.source_coordinate_index_for_ascending(target, 0)
                .unwrap(),
            2
        );
        assert_eq!(
            axis.source_sample_index_for_ascending(target, 0).unwrap(),
            1
        );
        assert_eq!(
            axis.source_sample_index_for_ascending(target, 2),
            Err(Error::Index)
        );
    });
}
