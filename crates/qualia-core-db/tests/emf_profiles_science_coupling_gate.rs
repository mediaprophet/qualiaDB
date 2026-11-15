//! Wave-3 Integration Gate `W3-04` (`GATE-EMF-SCIENCE-COUPLING`) under `docs/plans/emf-qac-evolution-2026-11-13`.
use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_4, PI};
use qualia_core_db::q42::emf_operators::{
    admit_stokes_vector, admit_yee_timestep,
    convert_iq_emf_to_qac_conjugated, dft_forward_unscaled, instantaneous_poynting_and_energy, jones_to_stokes_candidate_v1,
    lossy_flags, mode0_carrier_wavelength_m, mode6_photon_attenuation_and_independent_dose, mode7_quasi_static_dipole_potential_v,
    mueller_apply, mueller_ideal_linear_polarizer_x, peak_phasor_time_average_poynting, project_mode0_to_display_voxel64,
    ray_geometric_delay_s, reconstruct_rf_emf_negative_carrier, reconstruct_rf_qac_positive_carrier,
    rms_phasor_time_average_poynting, rotate_eb_hamilton_xyzw, rotate_stokes_basis_passive, run_yee_periodic_plane_wave_fixture,
    spectral_density_wavelength_midpoint_integral, verify_cpu_f64_vs_gpu_f32_yee_parity, verify_discrete_charge_conservation,
    verify_discrete_poynting_power_balance, verify_div_curl_zero_2nd, verify_stokes_circulation_cell, yee_cfl_limit_3d,
    yee_curl_e_2nd, yee_curl_e_4th, yee_step_leapfrog_in_place, BoundaryCondition, CpmlAdeState, CpmlGradingSpec,
    DebyePole as OpDebyePole, DrudePole as OpDrudePole, EmfMode0InstantaneousRecord128, EmfOperatorError, Grid3dSpec,
    LorentzPole as OpLorentzPole, SealedCpmlAdeCheckpoint, YeeMaterialParams, YeeSolverProfile, C0_M_PER_S, EPS0_REF_F_PER_M, Z0_REF_OHM,
};
use qualia_core_db::q42::emf_operators::poynting_energy::{
    c02_preview_f32_budget, c02_reduction_f64_budget, c02_small_op_f64_budget,
};
use qualia_core_db::q42::emf_profiles::{
    absorbed_dose_gy, acquire_biofield_to_emf, acquire_iq_to_emf, admit_emf_or_acquisition_entry, build_log_band_edges,
    dipole_potential_infinite_homogeneous, encode_dose_records, encode_field_samples, encode_mueller_records, encode_yee_records,
    equivalent_dose_sv, integrate_spectrum_over_axis, iq_record_stride, jones_to_mueller, kev_to_joules, open_emf_mixed_container,
    verify_derivation_schema7_lineage, verify_empirical_join_allowed, verify_source_entry_lineage, write_extended_envelope,
    AcquisitionSpec, BioContextSpec, BiofieldRecord, CalibrationSpec, ConstitutiveSpec, DoseTransportRecord, EmfAcquisitionRegistry,
    Error as EmfError, ExtendedModeSpec, FieldClock, FieldFrame, FieldProvenance, FieldSample, FieldWriteLimits, GridIndexing,
    GridSpec, IqHeader, Limits as EmfLimits, MaterialSpec, ModelEvidenceSpec, MuellerRecord, ProfileReader, SpectralAxisSpec,
    YeeRecord, BIO_FLAG_CARRIER, BIO_FLAG_POTENTIAL, BIO_FLAG_UNCERTAINTY, DOSE_VALID_ABSORBED_GY, DOSE_VALID_EQUIVALENT_SV,
    DOSE_VALID_FLUENCE, DOSE_VALID_LINEAR_ATTEN, DOSE_VALID_MASS_ATTEN, DOSE_VALID_PHOTON_ENERGY, DOSE_VALID_UNCERTAINTY,
    FIELD_PROFILE, FLAG_LO_PHASE_KNOWN, IQ_CONVENTION, RAY_PROFILE, SPECTRUM_PROFILE, STFT_FFT_NORMALIZATION,
};
use qualia_core_db::q42::field_envelope::{
    BlockInput, EntryInput, EnvelopeInput, Family, FieldInput, FieldValue as V, Limits as EnvelopeLimits, MetadataInput, StructuralView,
};
use qualia_core_db::q42::field_semantics::{NodeState, Scratch};
use qualia_core_db::q42::m64_matter::{
    BankedConstituentLut, M64ChunkDirectoryEntry, M64Header, M64PayloadInterval, M64ReadLimits, M64StrataColumnRecord, M64_MAGIC, M64_MODE_STRATA,
};
use qualia_core_db::q42::material_coupling::{
    compute_blake3_256, convert_iq_carrier_convention, evaluate_brillouin_energy_density_j_per_m3, evaluate_fluid_solid_oblique_partition,
    evaluate_isotropic_eb_state, evaluate_planar_interface_scattering, evaluate_radar_sonar_normal_interface, evaluate_thermoelastic_stress,
    reconstruct_real_passband_sample, reject_spatial_key_only_join, resolve_emf_material_entry, verify_kramers_kronig_consistency,
    ActiveHamiltonRotation, CauchyStress6, Complex64 as MatComplex64, ConstitutiveModel, ContinuumMedium, CoordinateTransform4x4,
    CoordinateTransformKind, DebyePole as MatDebyePole, EmAmplitudeConvention, EmfMaterialZeroPolicy, HalfOpenValidityInterval,
    IqCarrierConvention, LorentzPole as MatLorentzPole, MaterialBinding, MaterialBindingError, MaterialEntryIdentity,
    PhotoacousticParameters, PiezoelectricConstitutiveModel, RationalClock, RationalInstant, ResolvedEmfMaterial, TensorStrain6,
    TransductionError, EPSILON_0_F_PER_M,
};
use qualia_core_db::q42::science_validation::{
    apply_mueller_matrix, apply_or_verify_affine_calibration, compose_hamilton_quaternions_xyzw, convert_and_verify_iq_carrier_bridge,
    evaluate_absorbed_dose_gy, evaluate_acoustic_interface_oracle, evaluate_bioelectric_dipole_potential_v, evaluate_ece_and_brier,
    evaluate_em_mode0_instantaneous_oracle, evaluate_exact_sample_instant, evaluate_gaussian_nll, evaluate_haptic_mechanical_oracle,
    evaluate_ionizing_beer_lambert_oracle, evaluate_qac_elastic_mode01_oracle, evaluate_qac_fluid_mode00_oracle,
    evaluate_ultrasound_mode07_oracle, jones_to_mueller_matrix, jones_vector_to_stokes, mahalanobis_nees_3d, rotate_covariance_3x3,
    rotate_stokes_transverse_basis, run_all_science_validations, transform_local_to_parent, verify_anatomical_region_registration,
    verify_and_derive_quaternion_rotation, verify_covariance_3x3_spd, verify_cross_modal_clock_join, verify_f64_closed_form_budget,
    verify_gaussian_coverage_factor, verify_grid_convergence_order, verify_half_open_calibration_window,
    verify_jones_to_mueller_equivalence, verify_material_and_anatomy_coupling, verify_mueller_realizability,
    verify_poynting_conservation_balance, verify_scattering_matrix_passivity, verify_stationary_proper_rotation_matrix,
    verify_stokes_realizability, verify_uncertainty_descriptor, verify_yee_cfl_and_dispersion, CalibrationStage,
    Complex64 as SciComplex64, ConservationValidationError, HapticRole, MaterialDomainFamily, PolarimetricValidationError,
    RationalClockDescriptor, RationalSeconds, SpecializedOracleError, UncertaintyDescriptor, UncertaintyKind, VACUUM_IMPEDANCE_OHM,
    VACUUM_SPEED_OF_LIGHT_M_PER_S,
};

fn gate_clock() -> FieldClock {
    FieldClock { scale: 0, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks_per_second_num: 1_000, ticks_per_second_den: 1 }
}
fn gate_frame(grid: Option<GridSpec>) -> FieldFrame {
    FieldFrame { origin_m: [0.0, 0.0, 0.0], rotation_xyzw: [0.0, 0.0, 0.0, 1.0], grid }
}
fn gate_prov() -> FieldProvenance<'static> {
    FieldProvenance { role: 1, algorithm: Some("w3-04-emf-science-coupling-gate"), algorithm_version: Some("1.0"), lineage: Some("gate-emf-science-coupling") }
}
fn gate_cal(unit: &'static str, validity: [i64; 2]) -> CalibrationSpec<'static> {
    CalibrationSpec { instrument_identifier: "gate-cal-01", content_digest: [0x42; 32], quantity_unit: unit, scale: 1.0, offset: 0.0, uncertainty: Some((0.01, "k=1_standard")), validity_ticks: Some(validity) }
}

#[test]
fn emf_v4_all_eight_modes_roundtrip_and_mixed_acquisition_bridge() {
    let (mut mb, mut st, mut out) = ([0u8; 4096], [0u8; 16384], [0u8; 16384]);
    let (mut md, mut ed) = ([NodeState::default(); 64], [NodeState::default(); 16]);
    let mut scratch = Scratch { metadata: &mut md, entries: &mut ed };
    let w_lim = FieldWriteLimits::default();

    // Mode 0x0100: Field (instantaneous_E_B_v1)
    let sample0 = FieldSample { electric_v_per_m: [2.0, -1.0, 0.5], magnetic_tesla: [0.0, 2.0 / C0_M_PER_S, 0.0], position_m: [1.0, 2.0, 3.0], tick: 0, carrier_hz: Some(2.4e9), stored_poynting_w_per_m2: Some([0.0, 0.0, 4.0 / Z0_REF_OHM]), confidence: Some(0.95) };
    let mut pay0 = [0u8; 128];
    encode_field_samples(&[sample0], &mut pay0, EmfLimits::default()).unwrap();
    let b0 = write_extended_envelope(gate_clock(), gate_frame(None), gate_prov(), 0x0100, None, 0, 0, None, 1, &pay0, &ExtendedModeSpec::Field { carrier_hz_note: true }, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v0 = StructuralView::open(b0, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    let mut r0 = ProfileReader::open(&v0, &mut scratch, EmfLimits::default(), 65536).unwrap();
    let c0 = r0.chunk(1).unwrap();
    assert_eq!((c0.canonical_mode(), c0.profile(), c0.field(0).unwrap().unwrap().electric_v_per_m()), (0x0100, FIELD_PROFILE, sample0.electric_v_per_m));

    // Mode 0x0101: Polarimetric Ray (polarimetric_ray_v1)
    let mut pay1 = [0u8; 128];
    pay1[40..48].copy_from_slice(&1.0f64.to_le_bytes());
    pay1[48..56].copy_from_slice(&1.0f64.to_le_bytes());
    pay1[72..80].copy_from_slice(&(-1.0f64).to_le_bytes());
    pay1[80..88].copy_from_slice(&2.4e9f64.to_le_bytes());
    pay1[88..96].copy_from_slice(&20e6f64.to_le_bytes());
    pay1[96..104].copy_from_slice(&0.299_792_458f64.to_le_bytes());
    pay1[112..120].copy_from_slice(&0.1f64.to_le_bytes());
    let b1 = write_extended_envelope(gate_clock(), gate_frame(None), gate_prov(), 0x0101, None, 0, 0, None, 1, &pay1, &ExtendedModeSpec::Ray { units: "W m^-2" }, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v1 = StructuralView::open(b1, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    let mut r1 = ProfileReader::open(&v1, &mut scratch, EmfLimits::default(), 65536).unwrap();
    let c1 = r1.chunk(1).unwrap();
    assert_eq!((c1.canonical_mode(), c1.profile(), c1.ray(0).unwrap().unwrap().stokes()), (0x0101, RAY_PROFILE, [1.0, 0.0, 0.0, -1.0]));

    // Mode 0x0102: Yee cell (yee_cartesian_cell_v1)
    let yee_grid = GridSpec { dimensions: [1, 1, 1], brick_dimensions: [1, 1, 1], spacing_m: [0.01; 3], indexing: GridIndexing::Linear };
    let yee_rec = YeeRecord::vacuum([5.0, 0.0, 0.0], [0.0, 0.01, 0.0], 6);
    let mut pay2 = [0u8; 128];
    encode_yee_records(&[yee_rec], None, &mut pay2).unwrap();
    let spec2 = ExtendedModeSpec::Yee { constitutive: ConstitutiveSpec { model: "vacuum", epsilon_infinity_relative: 1.0, mu_relative: 1.0, conductivity_s_per_m: 0.0, delta_epsilon: &[], relaxation_time_s: &[], resonance_rad_s: &[], damping_s1: &[] }, time_offset_num: &[0, 0, 0, 1, 1, 1], time_offset_den: &[1, 1, 1, 2, 2, 2], material: None };
    let b2 = write_extended_envelope(gate_clock(), gate_frame(Some(yee_grid)), gate_prov(), 0x0102, Some(10), 0, 1, Some(0), 1, &pay2, &spec2, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v2 = StructuralView::open(b2, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    assert_eq!(ProfileReader::open(&v2, &mut scratch, EmfLimits::default(), 65536).unwrap().chunk(1).unwrap().yee(0).unwrap().unwrap(), yee_rec);

    // Mode 0x0103: Sampled Spectrum (sampled_spectrum_v1)
    let mut edges24 = [0.0f64; 25];
    build_log_band_edges(1.0e9, 1.0e12, &mut edges24).unwrap();
    let mut pay3 = [0u8; 256];
    pay3[24..32].copy_from_slice(&10i64.to_le_bytes());
    pay3[36..40].copy_from_slice(&6u32.to_le_bytes());
    for i in 0..24 { pay3[64 + i * 8..72 + i * 8].copy_from_slice(&1.0e-12f64.to_le_bytes()); }
    let spec3 = ExtendedModeSpec::Spectrum { axis: SpectralAxisSpec { axis: "frequency_hz", sample_semantics: "bin_average", physical_quantity: "spectral_irradiance", unit: "W m^-2 Hz^-1", coordinates: &edges24, sigma_interpretation: "none", sigma_bounds: None }, bins: 24, response_cal: Some(gate_cal("V/W", [0, 100])) };
    let b3 = write_extended_envelope(gate_clock(), gate_frame(None), gate_prov(), 0x0103, Some(10), 10, 1, None, 1, &pay3, &spec3, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v3 = StructuralView::open(b3, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    let mut r3 = ProfileReader::open(&v3, &mut scratch, EmfLimits::default(), 65536).unwrap();
    let c3 = r3.chunk(1).unwrap();
    assert_eq!(c3.profile(), SPECTRUM_PROFILE);
    let vals3: Vec<f64> = c3.spectrum(0).unwrap().unwrap().values().collect();
    let tot3 = integrate_spectrum_over_axis(c3.spectral_axis().unwrap(), &vals3).unwrap();
    assert!(((tot3 - 1.0e-12 * (1.0e12 - 1.0e9)) / tot3).abs() < 1e-12);

    // Mode 0x0104: Mueller scattering matrix (mueller_scattering_v1)
    let qwp = jones_to_mueller([[1.0, 0.0], [0.0, 0.0]], [[0.0, 0.0], [0.0, 1.0]]).unwrap();
    let m_rec = MuellerRecord { position_m: [1.0, 2.0, 3.0], incident_direction: [0.0, 0.0, 1.0], outgoing_direction: [0.0, 1.0, 0.0], frequency_hz: 5.0e14, material_entry: 12, flags: 0, mueller_matrix_m: qwp };
    let mut pay4 = [0u8; 256];
    encode_mueller_records(&[m_rec], None, &mut pay4).unwrap();
    let spec4 = ExtendedModeSpec::Mueller {
        evidence: ModelEvidenceSpec { model_identifier_version: "mueller-v1", domain_applicability: "optics", parameter_source_digest: [0x99; 32], numerical_convention: "I_Q_U_V_forward_backward", epistemic_class: "model_estimate", uncertainty: Some("2%"), external_reference: None, qualification_receipt_id: None, particle_species: None, integration_ticks: None, weighting_derating_model: None },
        units: "sr^-1",
        material: Some(MaterialSpec { container_digest: [0x55; 32], table_digest: [0x66; 32], table_epoch: 1, table_schema_version: "v1", material_ids: &[12] }),
    };
    let b4 = write_extended_envelope(gate_clock(), gate_frame(None), gate_prov(), 0x0104, None, 0, 0, None, 1, &pay4, &spec4, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v4 = StructuralView::open(b4, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    assert_eq!(ProfileReader::open(&v4, &mut scratch, EmfLimits::default(), 65536).unwrap().chunk(1).unwrap().mueller(0).unwrap().unwrap(), m_rec);

    // Mode 0x0105: RF I/Q packet (rf_iq_packet_v1) via acquire_iq_to_emf
    let iq_samples = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
    let mut pay5 = vec![0u8; iq_record_stride(4).unwrap()];
    let iq_hdr = IqHeader { first_sample_tick: 100, sample_count: 4, channel_index: 0, centre_hz: 2.45e9, lo_phase_rad: 0.25, lo_phase_known: true, units_per_sample: 0.5, calibration_metadata_id: 6, acquisition_metadata_id: 7, flags: FLAG_LO_PHASE_KNOWN, record_crc32c: 0 };
    let acq_spec5 = AcquisitionSpec { sample_rate_num: 4000, sample_rate_den: 1, signal_kind: "complex_stft", unit: "counts", complex_convention: IQ_CONVENTION, window: Some("hann"), fft_size: Some(4), hop: Some(2), fft_normalization: Some(STFT_FFT_NORMALIZATION), carrier_hz: Some(2.45e9), channel_count: Some(2) };
    let b5 = acquire_iq_to_emf(gate_clock(), gate_frame(None), gate_prov(), &iq_hdr, &iq_samples, acq_spec5, gate_cal("V", [90, 120]), &mut pay5, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v5 = StructuralView::open(b5, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    assert_eq!(ProfileReader::open(&v5, &mut scratch, EmfLimits::default(), 65536).unwrap().chunk(1).unwrap().iq(0).unwrap().unwrap().calibrated_sample(1).unwrap(), (0.0, 0.5));

    // Mode 0x0106: Ionizing Radiation Dose (ionizing_dose_transport_v1)
    let dose_rec = DoseTransportRecord {
        position_m: [0.0, 0.0, 1.0], tick: 15, photon_energy_j: Some(kev_to_joules(662.0).unwrap()),
        linear_attenuation_per_m: Some(8.57), mass_attenuation_m2_per_kg: Some(0.00857), photon_fluence_per_m2: Some(5.0e11),
        absorbed_dose_gy: Some(absorbed_dose_gy(0.24, 2.0).unwrap()), equivalent_dose_sv: Some(equivalent_dose_sv(0.12, 1.0).unwrap()),
        linear_energy_transfer_j_per_m: None, uncertainty: Some(0.004), material_entry: 3, calibration_ref: 6, model_ref: 7,
        validity_flags: DOSE_VALID_PHOTON_ENERGY | DOSE_VALID_LINEAR_ATTEN | DOSE_VALID_MASS_ATTEN | DOSE_VALID_FLUENCE | DOSE_VALID_ABSORBED_GY | DOSE_VALID_EQUIVALENT_SV | DOSE_VALID_UNCERTAINTY,
    };
    let mut pay6 = [0u8; 128];
    encode_dose_records(&[dose_rec], None, &mut pay6).unwrap();
    let spec6 = ExtendedModeSpec::Dose {
        evidence: ModelEvidenceSpec { model_identifier_version: "geant4-mc-em-v11", domain_applicability: "photon_kerma", parameter_source_digest: [0x19; 32], numerical_convention: "icrp103_si", epistemic_class: "model_estimate", uncertainty: Some("3%"), external_reference: Some("ICRP-103"), qualification_receipt_id: None, particle_species: Some("photon"), integration_ticks: Some([10, 20]), weighting_derating_model: Some("ICRP-103 w_R=1") },
        calibration: gate_cal("Gy", [10, 25]),
        material: Some(MaterialSpec { container_digest: [0xaa; 32], table_digest: [0xbb; 32], table_epoch: 1, table_schema_version: "v1", material_ids: &[3] }),
    };
    let b6 = write_extended_envelope(gate_clock(), gate_frame(None), gate_prov(), 0x0106, Some(5), 15, 1, None, 1, &pay6, &spec6, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v6 = StructuralView::open(b6, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    assert_eq!(ProfileReader::open(&v6, &mut scratch, EmfLimits::default(), 65536).unwrap().chunk(1).unwrap().dose(0).unwrap().unwrap(), dose_rec);

    // Mode 0x0107: Biofield Dipole (biofield_current_dipole_v1) via acquire_biofield_to_emf
    let phi7 = dipole_potential_infinite_homogeneous([0.0; 3], [0.0, 0.0, 1.0e-7], [0.0, 0.0, 0.02], 0.33).unwrap();
    let bio_rec = BiofieldRecord { position_m: [0.01, -0.02, 0.05], current_dipole_moment_a_m: [1.0e-8, -2.0e-8, 3.0e-8], extracellular_potential_v: Some(phi7), tick: 12, carrier_hz: Some(40.0), uncertainty: Some(1.0e-6), anatomy_metadata_ref: 8, model_ref: 7, calibration_ref: 6, consent_provenance_ref: 9, flags: BIO_FLAG_POTENTIAL | BIO_FLAG_CARRIER | BIO_FLAG_UNCERTAINTY };
    let mut pay7 = [0u8; 128];
    let bio_spec7 = BioContextSpec { anatomical_ontology_version: "FMA-5.0", anatomical_identifier: "FMA:50801", observation_type: "current_dipole", consent_receipt_digest: Some([0x7c; 32]) };
    let ev_spec7 = ModelEvidenceSpec { model_identifier_version: "meg-forward-v1", domain_applicability: "cortical_dipole", parameter_source_digest: [0x22; 32], numerical_convention: "si_dipole_a_m", epistemic_class: "empirical", uncertainty: Some("1e-6 V"), external_reference: None, qualification_receipt_id: Some("irb-2026-01"), particle_species: None, integration_ticks: Some([0, 50]), weighting_derating_model: None };
    let b7 = acquire_biofield_to_emf(gate_clock(), gate_frame(None), gate_prov(), &[bio_rec], Some(1), 12, bio_spec7, ev_spec7, gate_cal("A m", [0, 50]), &mut pay7, &mut mb, &mut st, &mut out, &mut scratch, w_lim).unwrap();
    let v7 = StructuralView::open(b7, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    assert_eq!(ProfileReader::open(&v7, &mut scratch, EmfLimits::default(), 65536).unwrap().chunk(1).unwrap().biofield(0).unwrap().unwrap(), bio_rec);
    assert!(verify_empirical_join_allowed(v7.blocks().find(|b| b.id() == 7).unwrap()).is_ok());

    // Mixed-domain acquisition bridge (Mode 0x0104 raw original + Mode 0x0100 derived EMF + Schema 7 lineage)
    let acq_bytes = b"RAW-ACQUISITION-SENSOR-PACKET-DATA-W3-04-BRIDGE-64-BYTES-0123456";
    let acq_dig = compute_blake3_256(acq_bytes);
    fn fi<'a>(key: u16, crit: bool, value: V<'a>) -> FieldInput<'a> {
        FieldInput { key, flags: u8::from(crit), value }
    }
    let b_clk = [fi(1, true, V::U32(&[0])), fi(2, true, V::I64(&[0])), fi(3, true, V::U32(&[0])), fi(4, true, V::U64(&[1000])), fi(5, true, V::U64(&[1]))];
    let b_frm = [fi(1, true, V::U32(&[0])), fi(3, true, V::F64(&[0.0; 3])), fi(4, true, V::F64(&[0.0, 0.0, 0.0, 1.0])), fi(8, true, V::Utf8("right-handed-X-forward-Y-left-Z-up"))];
    let b_ext = [fi(1, true, V::U32(&[0x0100])), fi(2, true, V::U32(&[1])), fi(3, true, V::MetadataRefs(&[1])), fi(4, true, V::MetadataRefs(&[2])), fi(5, true, V::MetadataRefs(&[31])), fi(6, true, V::Utf8(FIELD_PROFILE)), fi(7, true, V::Utf8("linear")), fi(8, true, V::U64(&[1, 1, 1])), fi(9, true, V::U32(&[1, 1, 1])), fi(10, true, V::U32(&[0])), fi(11, true, V::I64(&[0]))];
    let b_dsc_acq = [fi(3, true, V::MetadataRefs(&[11])), fi(7, true, V::Utf8("original_bytes")), fi(8, true, V::Utf8("1")), fi(9, true, V::U32(&[1])), fi(10, true, V::Utf8("byte")), fi(11, true, V::U32(&[7])), fi(12, true, V::Utf8("urn:qualia:acquisition:original:1")), fi(18, true, V::MetadataRefs(&[20]))];
    let b_prv_acq = [fi(1, true, V::U32(&[4]))];
    let b_s6 = [fi(1, true, V::U32(&[1])), fi(2, true, V::EntryRefs(&[1])), fi(3, true, V::Bytes(&acq_dig)), fi(4, true, V::U64(&[0])), fi(5, true, V::U64(&[64])), fi(8, true, V::U32(&[0])), fi(10, true, V::U32(&[7])), fi(11, true, V::U32(&[0])), fi(12, true, V::U32(&[0])), fi(13, true, V::U32(&[1])), fi(14, true, V::U32(&[64])), fi(15, true, V::U64(&[1])), fi(16, true, V::U32(&[0])), fi(17, true, V::U64(&[0])), fi(32, true, V::U32(&[0]))];
    let b_dsc_emf = [fi(2, true, V::MetadataRefs(&[2])), fi(3, true, V::MetadataRefs(&[31])), fi(7, true, V::Utf8("electromagnetic_field_snapshot")), fi(8, true, V::Utf8("mixed:mode-registry")), fi(12, true, V::Utf8("urn:qualia:emf:instantaneous_E_B_v1")), fi(17, true, V::EntryRefs(&[1])), fi(18, true, V::MetadataRefs(&[4]))];
    let b_prv_emf = [fi(1, true, V::U32(&[2])), fi(2, true, V::Bytes(&acq_dig))];
    let blocks = [
        BlockInput { id: 1, schema_type: 1, revision: 1, flags: 1, fields: &b_clk },
        BlockInput { id: 2, schema_type: 2, revision: 1, flags: 1, fields: &b_frm },
        BlockInput { id: 4, schema_type: 103, revision: 1, flags: 1, fields: &b_ext },
        BlockInput { id: 10, schema_type: 3, revision: 1, flags: 1, fields: &b_dsc_acq },
        BlockInput { id: 11, schema_type: 4, revision: 1, flags: 1, fields: &b_prv_acq },
        BlockInput { id: 20, schema_type: 6, revision: 1, flags: 1, fields: &b_s6 },
        BlockInput { id: 30, schema_type: 3, revision: 1, flags: 1, fields: &b_dsc_emf },
        BlockInput { id: 31, schema_type: 4, revision: 1, flags: 1, fields: &b_prv_emf },
    ];
    let m_slice = MetadataInput { blocks: &blocks }.encode_into(&mut mb, EnvelopeLimits::default()).unwrap();
    let entries = [
        EntryInput { mode: 0x0104, revision: 1, flags: 0, stream_id: 1, frame_id: 0, start_tick: 0, time_count: 0, spatial_key: 0, record_count: 64, stride: 1, codec: 0, descriptor_id: 10, payload: acq_bytes },
        EntryInput { mode: 0x0100, revision: 1, flags: 0, stream_id: 2, frame_id: 2, start_tick: 0, time_count: 0, spatial_key: 0, record_count: 1, stride: 128, codec: 0, descriptor_id: 30, payload: &pay0 },
    ];
    let mix_bytes = EnvelopeInput { family: Family::EmfV4Candidate, flags: 0, metadata: m_slice, entries: &entries }.encode_into(&mut out, EnvelopeLimits::default()).unwrap();
    let mix_view = StructuralView::open(mix_bytes, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    let mut mix_reader = open_emf_mixed_container(&mix_view, &mut scratch, EmfLimits::default(), 1_000_000).unwrap();
    let _ = mix_reader.registry() as &EmfAcquisitionRegistry;
    assert_eq!(mix_reader.chunk(1).unwrap().as_acquisition().unwrap().subrange().unwrap(), acq_bytes);
    assert_eq!(admit_emf_or_acquisition_entry(&mix_view, mix_reader.context(), 2, EmfLimits::default()).unwrap().as_emf().unwrap().canonical_mode(), 0x0100);
    verify_source_entry_lineage(&mix_view, mix_reader.context(), 2, 1).unwrap();

    let (mut s7_mb, mut s7_out) = ([0u8; 512], [0u8; 1024]);
    let b_s7 = [fi(1, true, V::EntryRefs(&[2])), fi(7, true, V::EntryRefs(&[1])), fi(8, true, V::Bytes(&acq_dig))];
    let s7_m = MetadataInput { blocks: &[BlockInput { id: 25, schema_type: 7, revision: 1, flags: 1, fields: &b_s7 }] }.encode_into(&mut s7_mb, EnvelopeLimits::default()).unwrap();
    let s7_env = EnvelopeInput { family: Family::EmfV4Candidate, flags: 0, metadata: s7_m, entries: &entries[..1] }.encode_into(&mut s7_out, EnvelopeLimits::default()).unwrap();
    let s7_view = StructuralView::open(s7_env, Family::EmfV4Candidate, EnvelopeLimits::default()).unwrap();
    let s7_blk = s7_view.blocks().next().unwrap();
    verify_derivation_schema7_lineage(mix_reader.context(), s7_blk, 2, 1).unwrap();
    assert_eq!(verify_derivation_schema7_lineage(mix_reader.context(), s7_blk, 2, 99), Err(EmfError::Index));
}

#[test]
fn material_coupling_dispersion_kramers_kronig_transduction_and_m64_binding() {
    let tissue_like = ConstitutiveModel {
        epsilon_infinity_relative: 2.2, mu_infinity_relative: 1.0, conductivity_s_per_m: 1e-4,
        electric_debye_poles: vec![MatDebyePole { delta_chi: 1.5, tau_s: 1.0e-9 }],
        electric_lorentz_poles: vec![MatLorentzPole { delta_chi: 0.8, omega_0_rad_s: 5.0e9, gamma_rad_s: 1.0e9 }],
        magnetic_debye_poles: vec![MatDebyePole { delta_chi: 0.4, tau_s: 2.0e-9 }], magnetic_lorentz_poles: vec![],
    };
    let kk = verify_kramers_kronig_consistency(&tissue_like, &[2.0e8, 5.0e8, 1.0e9, 2.5e9], 1.0e5, 2.0e11, 20_000, 2.5e-2).unwrap();
    assert!(kk.passivity_satisfied && kk.causality_satisfied);
    assert!(evaluate_brillouin_energy_density_j_per_m3(&tissue_like, 1.0e9, 50.0, 0.1).unwrap() > 0.0);

    let q_z90 = ActiveHamiltonRotation::admit([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]).unwrap();
    let eb_snap = evaluate_isotropic_eb_state([1.0, 0.0, 0.0], [0.0, 1.0 / C0_M_PER_S, 0.0], 1.0, 1.0, EmAmplitudeConvention::InstantaneousSnapshot).unwrap();
    assert!((eb_snap.poynting_w_per_m2[2] - 1.0 / Z0_REF_OHM).abs() <= 1e-9);
    assert!((q_z90.rotate_vector([1.0, 0.0, 0.0])[1] - 1.0).abs() <= 1e-14);

    let water = ContinuumMedium::new_fluid(1000.0, 2.25e9, 101_325.0).unwrap();
    let bone = ContinuumMedium::new_isotropic_solid(2000.0, 8.0e9, 3.0e9, 101_325.0).unwrap();
    let part = evaluate_fluid_solid_oblique_partition(&water, &bone, 0.3).unwrap();
    assert!((part.energy_reflection_p + part.energy_transmission_p + part.energy_transmission_sv - 1.0).abs() <= 1e-14);
    assert_eq!(resolve_emf_material_entry(0, 4, EmfMaterialZeroPolicy::ExplicitVacuumAdmitted), Ok(ResolvedEmfMaterial::ExplicitVacuum));

    let pa = PhotoacousticParameters { optical_absorption_per_m: 100.0, thermal_expansion_per_k: 2.1e-4, specific_heat_cp_j_per_kg_k: 4184.0, density_kg_per_m3: 1000.0, sound_speed_m_per_s: 1500.0, thermal_diffusivity_m2_per_s: 1.4e-7, characteristic_length_m: 1.0e-3 };
    let pa_resp = pa.evaluate_confined_excitation(200.0, 10.0e-9).unwrap();
    assert!((pa_resp.initial_pressure_pa - pa.gruneisen_parameter() * 20_000.0).abs() <= 1e-9);
    assert!(matches!(pa.evaluate_confined_excitation(200.0, 1.0e-3), Err(TransductionError::StressConfinementViolated { .. })));

    let mut c_e = [[0.0; 6]; 6];
    for m in 0..6 { c_e[m][m] = 100.0e9; }
    let eps_s = [[100.0 * EPSILON_0_F_PER_M, 0.0, 0.0], [0.0, 100.0 * EPSILON_0_F_PER_M, 0.0], [0.0, 0.0, 100.0 * EPSILON_0_F_PER_M]];
    let mut piezo_e = [[0.0; 6]; 3];
    piezo_e[2][2] = 5.0;
    let piezo = PiezoelectricConstitutiveModel { stiffness_c_e_pa: c_e, permittivity_eps_s_f_per_m: eps_s, piezo_e_c_per_m2: piezo_e };
    let p_state = piezo.evaluate_coupled_state(&TensorStrain6 { xx_yy_zz_xy_yz_xz: [0.0, 0.0, 1e-5, 0.0, 0.0, 0.0] }, [0.0, 0.0, 1e4]).unwrap();
    assert!((p_state.total_internal_energy_j_per_m3 - (p_state.mechanical_energy_j_per_m3 + p_state.dielectric_energy_j_per_m3)).abs() <= 1e-12);
    assert!((evaluate_thermoelastic_stress(&bone, &TensorStrain6::default(), 1.0e-5, 10.0).unwrap().mechanical_pressure_pa() - 2.4e6).abs() <= 1e-6);
    let _ = CauchyStress6::from_isotropic_strain(&bone, &TensorStrain6::default());
    let _ = evaluate_planar_interface_scattering(&water, &water, 0.0).unwrap();
    let rs = evaluate_radar_sonar_normal_interface(&ConstitutiveModel::vacuum(), &water, &tissue_like, &water, 2.0 * PI * 1e9).unwrap();
    assert!(rs.acoustic.energy_reflection.abs() <= 1e-15 && rs.em_power_reflection > 0.0);

    let z_emf = MatComplex64::new(0.75, -1.25);
    let z_qac = convert_iq_carrier_convention(z_emf, IqCarrierConvention::EmfNegativeCarrier, 0.2, IqCarrierConvention::QacPositiveCarrier, -0.3).unwrap();
    assert!((reconstruct_real_passband_sample(z_emf, 1e3, 0.2, 1.5e-4, IqCarrierConvention::EmfNegativeCarrier) - reconstruct_real_passband_sample(z_qac, 1e3, -0.3, 1.5e-4, IqCarrierConvention::QacPositiveCarrier)).abs() <= 1e-14);

    let mut rec = M64StrataColumnRecord::default();
    rec.update_crc();
    let mut hdr = M64Header { magic: M64_MAGIC, total_file_size: 192, chunk_count: 1, chunk_table_offset: 64, ..M64Header::default() };
    hdr.update_crc();
    let mut dir = M64ChunkDirectoryEntry { chunk_uuid: [1; 16], spatial_hilbert_index: 0x1234_5678, byte_offset: 128, byte_length: 64, record_count: 1, srid_geodetic_code: 0, lut_bank_epoch_id: 7, chunk_record_mode: M64_MODE_STRATA, compression_flags: 0, bao_root_hash_prefix: [0; 12], entry_crc32c: 0 };
    dir.update_crc();
    let mut m64_bytes = vec![0u8; 192];
    m64_bytes[..64].copy_from_slice(bytemuck::bytes_of(&hdr));
    m64_bytes[64..128].copy_from_slice(bytemuck::bytes_of(&dir));
    m64_bytes[128..192].copy_from_slice(bytemuck::bytes_of(&rec));

    let mut lut = BankedConstituentLut::new_with_terrestrial_defaults();
    lut.active_bank_1_epoch_id = 7;
    lut.entry_mut(3).provenance_did_hash = 0xCAFE_BABE_1234_5678;
    lut.entry_mut(3).update_crc();
    let clk = RationalClock { clock_id: 10, scale: 0, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks_per_second_num: 96_000, ticks_per_second_den: 1 };
    let binding = MaterialBinding {
        container_blake3: compute_blake3_256(&m64_bytes), material_table_blake3: compute_blake3_256(lut.as_gpu_bytes()),
        table_schema_revision: 1, table_epoch_id: 7, material_frame_id: 1, field_frame_id: 2,
        transform: CoordinateTransform4x4 {
            source_frame_id: 1, target_frame_id: 2, kind: CoordinateTransformKind::ProperRigid,
            matrix_row_major: [[0.0, -1.0, 0.0, 1.0], [1.0, 0.0, 0.0, 2.0], [0.0, 0.0, 1.0, 3.0], [0.0, 0.0, 0.0, 1.0]],
            validity: HalfOpenValidityInterval::new(clk, RationalInstant::from_integral_tick(10, 100).unwrap(), RationalInstant::from_integral_tick(10, 200).unwrap()).unwrap(),
        },
        entries: vec![MaterialEntryIdentity { lut_slot_index: 3, canonical_q42_material_hash: lut.entry(3).canonical_q42_material_hash, provenance_did_hash: 0xCAFE_BABE_1234_5678, spatial_hilbert_key: 0x1234_5678 }],
    };
    let mut m64_scratch = [M64PayloadInterval::default(); 4];
    let m64_lim = M64ReadLimits { max_file_bytes: 1024, max_chunks: 4, max_records: 4 };
    let resolved = binding.verify_and_resolve_entry(&m64_bytes, m64_lim, &mut m64_scratch, &lut, 1, 1, &clk, &RationalInstant::from_integral_tick(10, 150).unwrap()).unwrap();
    assert_eq!(resolved.identity.lut_slot_index, 3);
    assert_eq!(reject_spatial_key_only_join(0x1234_5678), Err(MaterialBindingError::SpatialKeyAloneInsufficient));
}

#[test]
fn emf_operators_poynting_polarization_yee_vector_calculus_and_cpml_ade_restart() {
    let mat = YeeMaterialParams::vacuum();
    let (s, u) = instantaneous_poynting_and_energy([1.0, 0.0, 0.0], [0.0, 1.0 / C0_M_PER_S, 0.0], &mat).unwrap();
    assert!((s[2] - 1.0 / Z0_REF_OHM).abs() <= c02_small_op_f64_budget(1.0 / Z0_REF_OHM));
    assert!((u - EPS0_REF_F_PER_M).abs() <= c02_small_op_f64_budget(EPS0_REF_F_PER_M));
    let s_pk = peak_phasor_time_average_poynting([(1.0, 0.0), (0.0, 0.0), (0.0, 0.0)], [(0.0, 0.0), (1.0 / Z0_REF_OHM, 0.0), (0.0, 0.0)]).unwrap();
    let s_rms = rms_phasor_time_average_poynting([(FRAC_1_SQRT_2, 0.0), (0.0, 0.0), (0.0, 0.0)], [(0.0, 0.0), (FRAC_1_SQRT_2 / Z0_REF_OHM, 0.0), (0.0, 0.0)]).unwrap();
    assert!((s_pk[2] - s_rms[2]).abs() <= c02_small_op_f64_budget(1.0 / Z0_REF_OHM));
    assert_eq!(mode0_carrier_wavelength_m(0.0, false), Err(EmfOperatorError::StaticCarrierNoWavelength));
    let (e_rot, _, _) = rotate_eb_hamilton_xyzw([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2], [1.0, 0.0, 0.0], [0.0, 1.0 / C0_M_PER_S, 0.0]).unwrap();
    assert!((e_rot[1] - 1.0).abs() <= c02_small_op_f64_budget(1.0));

    let stokes_c = jones_to_stokes_candidate_v1((FRAC_1_SQRT_2, 0.0), (0.0, FRAC_1_SQRT_2)).unwrap();
    assert!((stokes_c[3] + 1.0).abs() <= c02_small_op_f64_budget(1.0));
    assert!((rotate_stokes_basis_passive([1.0, 1.0, 0.0, 0.0], FRAC_PI_4).unwrap()[2] + 1.0).abs() <= c02_small_op_f64_budget(1.0));
    assert!((ray_geometric_delay_s(0.299_792_458, &mat).unwrap() - 1e-9).abs() <= c02_small_op_f64_budget(1e-9));
    assert!(admit_stokes_vector([1.0, 1.001, 0.0, 0.0]).is_err());
    let (analytic_int, mid_int) = spectral_density_wavelength_midpoint_integral(1e12, 2e12, 1e-12, 128).unwrap();
    assert!((analytic_int - 1.0).abs() <= c02_small_op_f64_budget(1.0) && (mid_int - 1.0).abs() < 1e-4);
    assert_eq!(mueller_apply(&mueller_ideal_linear_polarizer_x(), [2.0, 0.0, 0.0, 0.0]).unwrap(), [1.0, 1.0, 0.0, 0.0]);
    let mut z8 = [(0.0, 0.0); 8];
    for (n, z) in z8.iter_mut().enumerate() { let a = 2.0 * PI * (n as f64) / 8.0; *z = (a.cos(), a.sin()); }
    let mut bins8 = [(0.0, 0.0); 8];
    dft_forward_unscaled(&z8, None, &mut bins8).unwrap();
    assert!((bins8[1].0 - 8.0).hypot(bins8[1].1) <= c02_reduction_f64_budget(8, 8.0));
    let (iq_e, iq_q) = ((0.6, -0.8), convert_iq_emf_to_qac_conjugated((0.6, -0.8)));
    assert!((reconstruct_rf_emf_negative_carrier(iq_e, 1e4, 2.5e-5, 0.3) - reconstruct_rf_qac_positive_carrier(iq_q, 1e4, 2.5e-5, 0.3)).abs() <= c02_small_op_f64_budget(1.0));
    let (mu, _, dose_gy, _) = mode6_photon_attenuation_and_independent_dose(1000.0, 0.01, 0.2, 100.0, 0.002, 0.01, 1e5).unwrap();
    assert!((mu - 10.0).abs() <= 1e-12 && (dose_gy - 0.2).abs() <= 1e-12);
    assert!(mode7_quasi_static_dipole_potential_v([0.0, 0.0, 1e-8], [0.0; 3], [0.0, 0.0, 0.1], 1.0).unwrap() > 0.0);

    let grid = Grid3dSpec { nx: 16, ny: 16, nz: 16, dx: 1.0 / 16.0, dy: 1.0 / 16.0, dz: 1.0 / 16.0 };
    let n = grid.validate().unwrap();
    let cfl = yee_cfl_limit_3d(&grid, &mat).unwrap();
    assert!(admit_yee_timestep(&grid, &mat, 1.01 * cfl).is_err());
    let dt = 0.5 * cfl;
    let (mut ex, mut ey, mut ez, mut hx, mut hy, mut hz) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let two_pi = 2.0 * PI;
    for iz in 0..grid.nz { for iy in 0..grid.ny { for ix in 0..grid.nx {
        let (c, x, y, z) = (grid.idx(ix, iy, iz), (ix as f64) * grid.dx, (iy as f64) * grid.dy, (iz as f64) * grid.dz);
        ex[c] = (two_pi * x).sin() * (two_pi * y).cos(); ey[c] = (two_pi * y).sin() * (two_pi * z).cos(); ez[c] = (two_pi * z).sin() * (two_pi * x).cos();
        hx[c] = (two_pi * (x + y)).cos(); hy[c] = (two_pi * (y + z)).sin(); hz[c] = (two_pi * (z + x)).cos();
    }}}
    let (mut cx, mut cy, mut cz, mut div_s) = (vec![0.0; n], vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    let (dc_e, dc_h) = verify_div_curl_zero_2nd(&grid, BoundaryCondition::Periodic, &ex, &ey, &ez, &hx, &hy, &hz, &mut cx, &mut cy, &mut cz, &mut div_s).unwrap();
    assert!(dc_e <= 1e-12 && dc_h <= 1e-12);
    yee_curl_e_2nd(&grid, BoundaryCondition::Periodic, &ex, &ey, &ez, &mut cx, &mut cy, &mut cz).unwrap();
    assert!(verify_stokes_circulation_cell(&grid, BoundaryCondition::Periodic, &ex, &ey, &ez, &cx, &cy, &cz, 3, 5, 7).unwrap().iter().all(|&r| r <= 1e-14));
    yee_curl_e_4th(&grid, BoundaryCondition::Periodic, &ex, &ey, &ez, &mut cx, &mut cy, &mut cz).unwrap();

    let (ex0, ey0, ez0, hx0, hy0, hz0) = (ex.clone(), ey.clone(), ez.clone(), hx.clone(), hy.clone(), hz.clone());
    let jx: Vec<f64> = (0..n).map(|i| 1e-4 * ((i as f64) * 0.1).sin()).collect();
    let jy: Vec<f64> = (0..n).map(|i| 1e-4 * ((i as f64) * 0.2).cos()).collect();
    let jz: Vec<f64> = (0..n).map(|i| -1e-4 * ((i as f64) * 0.15).sin()).collect();
    yee_step_leapfrog_in_place(&grid, YeeSolverProfile::LosslessPeriodicNoPhiV1, BoundaryCondition::Periodic, &mat, dt, &mut ex, &mut ey, &mut ez, &mut hx, &mut hy, &mut hz, &jx, &jy, &jz, &mut cx, &mut cy, &mut cz).unwrap();
    let eps = mat.epsilon_f_per_m();
    let dx0: Vec<f64> = ex0.iter().map(|&v| eps * v).collect();
    let dy0: Vec<f64> = ey0.iter().map(|&v| eps * v).collect();
    let dz0: Vec<f64> = ez0.iter().map(|&v| eps * v).collect();
    let dx1: Vec<f64> = ex.iter().map(|&v| eps * v).collect();
    let dy1: Vec<f64> = ey.iter().map(|&v| eps * v).collect();
    let dz1: Vec<f64> = ez.iter().map(|&v| eps * v).collect();
    let (mut s1, mut s2, mut s3) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    assert!(verify_discrete_charge_conservation(&grid, BoundaryCondition::Periodic, dt, &dx0, &dy0, &dz0, &dx1, &dy1, &dz1, &jx, &jy, &jz, &mut s1, &mut s2, &mut s3).unwrap() <= 1e-13);
    assert!(verify_discrete_poynting_power_balance(&grid, BoundaryCondition::Periodic, &mat, dt, &ex0, &ey0, &ez0, &ex, &ey, &ez, &hx0, &hy0, &hz0, &jx, &jy, &jz, &mut cx, &mut cy, &mut cz).unwrap() <= 1e-12);
    assert!(run_yee_periodic_plane_wave_fixture(16, 1.0, 0.5, &mat).unwrap().one_step_rel_error <= 1e-10);

    let mut cpml_st = CpmlAdeState::new(
        Grid3dSpec { nx: 6, ny: 6, nz: 6, dx: 1e-3, dy: 1e-3, dz: 1e-3 },
        YeeMaterialParams { epsilon_r: 2.0, mu_r: 1.0, sigma_s_per_m: 0.005 },
        BoundaryCondition::DirichletPec,
        CpmlGradingSpec { thickness_cells: 2, m_order: 3.0, sigma_max_s_per_m: 1.0, kappa_max: 4.0, alpha_max_s_per_m: 0.04 },
        1e-12,
        vec![OpDebyePole { delta_epsilon: 0.4, tau_s: 4e-12 }],
        vec![OpDrudePole { omega_p_rad_s: 1.5e10, gamma_rad_s: 1e9 }],
        vec![OpLorentzPole { delta_epsilon: 0.15, omega_0_rad_s: 4e10, gamma_rad_s: 1.5e9 }],
    ).unwrap();
    cpml_st.e[2][cpml_st.grid.idx(3, 3, 3)] = 1.0;
    cpml_st.step_in_place().unwrap();
    let mut sealed = SealedCpmlAdeCheckpoint::seal_from_state(&cpml_st).unwrap();
    assert_eq!(sealed.verify_and_restore().unwrap(), cpml_st);
    sealed.state_blake3[0] ^= 0xFF;
    assert_eq!(sealed.verify_and_restore(), Err(EmfOperatorError::RestartDigestMismatch));

    let proj = project_mode0_to_display_voxel64(&EmfMode0InstantaneousRecord128 {
        electric_field_v_per_m: [1.0, 0.0, 0.0], magnetic_field_tesla: [0.0, 1.0 / C0_M_PER_S, 0.0],
        local_position_m: [0.0; 3], record_tick: 1, carrier_hz: 5e14, coherence_phase_rad: 0.0,
        stored_poynting_w_per_m2: [0.0, 0.0, 1.0 / Z0_REF_OHM], flags: 0b1011, confidence: 1.0,
    }, &mat).unwrap();
    assert_ne!(proj.lossy_flags & lossy_flags::DISPLAY_SIGMA_DERIVED_NON_AUTHORITATIVE, 0);
    assert!(proj.max_s_abs_error <= c02_preview_f32_budget(1.0 / Z0_REF_OHM));
    let parity = verify_cpu_f64_vs_gpu_f32_yee_parity(16, 2).unwrap();
    assert!(parity.max_e_abs_diff <= parity.e_budget && parity.max_h_abs_diff <= parity.h_budget);
}

#[test]
fn science_validation_realizability_passivity_ece_oracles_and_registration() {
    run_all_science_validations().unwrap();
    let stokes = jones_vector_to_stokes(SciComplex64::new(FRAC_1_SQRT_2, 0.0), SciComplex64::new(0.0, FRAC_1_SQRT_2));
    assert!((verify_stokes_realizability(stokes).unwrap().degree_of_polarization - 1.0).abs() <= 1e-12);
    assert!(matches!(verify_stokes_realizability([1.0, 1.001, 0.0, 0.0]), Err(PolarimetricValidationError::OverPolarizedState { .. })));
    let _ = rotate_stokes_transverse_basis([1.0, 1.0, 0.0, 0.0], FRAC_PI_4);
    let j_pol = [[SciComplex64::ONE, SciComplex64::ZERO], [SciComplex64::ZERO, SciComplex64::ZERO]];
    let m_pol = jones_to_mueller_matrix(&j_pol);
    verify_jones_to_mueller_equivalence(&m_pol, &j_pol, 1.0).unwrap();
    assert!((verify_mueller_realizability(&m_pol, true).unwrap().max_transmittance_gain - 1.0).abs() <= 1e-12);
    assert_eq!(apply_mueller_matrix(&m_pol, [2.0, 0.0, 0.0, 0.0])[0], 1.0);
    let m_unphys = [[1.0, 0.0, 0.0, 0.0], [0.0, 2.0, 0.0, 0.0], [0.0, 0.0, 2.0, 0.0], [0.0, 0.0, 0.0, 2.0]];
    assert!(matches!(verify_mueller_realizability(&m_unphys, false), Err(PolarimetricValidationError::CloudeNegativeEigenvalue { .. })));

    let em0 = evaluate_em_mode0_instantaneous_oracle([1.0, 0.0, 0.0], [0.0, 1.0 / VACUUM_SPEED_OF_LIGHT_M_PER_S, 0.0], 1.0, 1.0).unwrap();
    verify_f64_closed_form_budget(em0.poynting_w_per_m2[2], 1.0 / VACUUM_IMPEDANCE_OHM, 1.0 / VACUUM_IMPEDANCE_OHM).unwrap();
    verify_poynting_conservation_balance(-0.5, 0.2, 0.3, 1.0).unwrap();
    assert!(matches!(verify_poynting_conservation_balance(1.0, 0.0, -1.0, 1.0), Err(ConservationValidationError::PassivityOrConservationViolation { .. })));
    assert!(verify_yee_cfl_and_dispersion([1e-3; 3], 1e-12, 1.0, 1.0, [10.0, 0.0, 0.0]).is_ok());
    assert!(evaluate_qac_fluid_mode00_oracle(1000.0, 2.25e9, 1500.0, [0.001, 0.0, 0.0], true).is_ok());
    assert!(evaluate_qac_elastic_mode01_oracle(2000.0, 8e9, 3e9, [1e-6, -1e-6, 0.0, 0.0, 0.0, 0.0]).is_ok());
    let (_, _, cap_r, cap_t) = evaluate_acoustic_interface_oracle(1.5e6, 3.0e6).unwrap();
    verify_f64_closed_form_budget(cap_r + cap_t, 1.0, 1.0).unwrap();
    let mut s_mat = [[SciComplex64::ZERO; 4]; 4];
    s_mat[0][0] = SciComplex64::new(0.6, 0.0);
    s_mat[0][1] = SciComplex64::new(0.8, 0.0);
    s_mat[1][0] = SciComplex64::new(0.8, 0.0);
    s_mat[1][1] = SciComplex64::new(-0.6, 0.0);
    verify_f64_closed_form_budget(verify_scattering_matrix_passivity(&s_mat, 2).unwrap(), 1.0, 1.0).unwrap();
    verify_f64_closed_form_budget(verify_grid_convergence_order(1.0 / 32.0, 0.008, 1.0 / 64.0, 0.002, 1.8).unwrap(), 2.0, 2.0).unwrap();

    verify_gaussian_coverage_factor(2.0, 0.954_499_736_103_641_6, 1e-12).unwrap();
    verify_uncertainty_descriptor(&UncertaintyDescriptor { target_quantity: "absorbed_dose", target_unit: "Gy", kind: UncertaintyKind::ExpandedAbsolute { coverage_factor_k: 2.0, confidence_probability: 0.954_499_74 }, value: 0.01, record_has_mixed_units: true, broadcast_across_all_fields: false }, 1e-6).unwrap();
    let (v_cal, stg) = apply_or_verify_affine_calibration(3.0, CalibrationStage::RawCounts, 1.0, 0.0, 2.0, 0.0).unwrap();
    assert_eq!(apply_or_verify_affine_calibration(v_cal, stg, 1.0, 0.0, 2.0, 0.0).unwrap().0, 6.0);
    verify_half_open_calibration_window(100, 100, 200).unwrap();
    let cov = [[4.0, 1.0, 0.0], [1.0, 2.0, 0.0], [0.0, 0.0, 1.0]];
    assert_eq!(verify_covariance_3x3_spd(&cov).unwrap(), (7.0, 7.0));
    let _ = rotate_covariance_3x3(&cov, &[[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]]).unwrap();
    assert!((mahalanobis_nees_3d([2.0, 0.0, 0.0], [0.0; 3], &[[4.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]).unwrap() - 1.0).abs() <= 1e-12);
    let ece_rep = evaluate_ece_and_brier(&[0.25, 0.25, 0.25, 0.25, 0.75, 0.75, 0.75, 0.75], &[true, false, false, false, true, true, true, false], 4, 1e-12).unwrap();
    assert!(ece_rep.expected_calibration_error.abs() <= 1e-12 && (ece_rep.brier_score - 0.1875).abs() <= 1e-12);
    assert!(evaluate_gaussian_nll(&[0.0], &[0.0], &[1.0]).unwrap() > 0.9);

    assert!((evaluate_ionizing_beer_lambert_oracle(1000.0, 0.01, 0.2, 100.0, false).unwrap().linear_attenuation_per_m - 10.0).abs() <= 1e-12);
    assert_eq!(evaluate_ionizing_beer_lambert_oracle(1000.0, 0.01, 0.2, 100.0, true), Err(SpecializedOracleError::DoseFromAttenuationAloneForbidden));
    assert!((evaluate_absorbed_dose_gy(0.002, 0.01).unwrap() - 0.2).abs() <= 1e-12);
    assert!(evaluate_bioelectric_dipole_potential_v([0.0, 0.0, 1e-8], [0.0; 3], Some([0.0, 0.0, 0.1]), true, 1.0).unwrap() > 0.0);
    let hap = evaluate_haptic_mechanical_oracle(2.0, 1e-4, 1000.0, 0.002, [2.0, 0.0, 0.0], &[[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]], HapticRole::ActuatorCommand, true, true, 99, 100).unwrap();
    assert!((hap.contact_pressure_pa - 20_000.0).abs() <= 1e-6);
    let us = evaluate_ultrasound_mode07_oracle(1500.0, 0.015, 20e6, 4e6, 1e6, 0.15, None, None, false).unwrap();
    assert!((us.mechanical_index - 0.5).abs() <= 1e-12);
    convert_and_verify_iq_carrier_bridge(SciComplex64::I, SciComplex64::I.conj(), 1000.0, 0.00025, 0.0).unwrap();

    let rot_z90 = verify_and_derive_quaternion_rotation([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]).unwrap();
    verify_stationary_proper_rotation_matrix(&rot_z90.rotation_matrix, false, false).unwrap();
    let (pos_p, _) = transform_local_to_parent([10.0, 20.0, 30.0], &rot_z90.rotation_matrix, [1.0, 0.0, 0.0], [1.0, 0.0, 0.0]);
    assert!((pos_p[0] - 10.0).abs() <= 1e-12 && (pos_p[1] - 21.0).abs() <= 1e-12);
    let _ = compose_hamilton_quaternions_xyzw([0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2], [0.0, 0.0, FRAC_1_SQRT_2, FRAC_1_SQRT_2]);
    let clk_96k = RationalClockDescriptor { origin_id: 42, scale: 1, epoch_seconds: 1_700_000_000, epoch_nanoseconds: 0, ticks_per_second_num: 96_000, ticks_per_second_den: 1 };
    assert_eq!(evaluate_exact_sample_instant(&clk_96k, 0, 1001, 48_000_000, 1001, 0, 1).unwrap(), RationalSeconds { num: 1_002_001, den: 48_000_000 });
    verify_cross_modal_clock_join(&clk_96k, &clk_96k, true, 0.0, 1e6, None).unwrap();
    let dig = [0xAA; 32];
    verify_material_and_anatomy_coupling(MaterialDomainFamily::EmfV4 { explicit_vacuum: true }, 0, &dig, &dig, &dig, &dig, 1, 1, true, true).unwrap();
    verify_anatomical_region_registration([0.0, 0.0, 0.05], [-0.1, -0.1, -0.1], [0.1, 0.1, 0.1], [0.0, 0.0, 1.0]).unwrap();
}
