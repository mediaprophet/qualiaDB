//! Unit and physics tests for common envelope/registry guarantees plus Q03 & Q05 profiles.

use std::f64::consts::PI;

use super::elastic::*;
use super::envelope::*;
use super::hoa::*;
use super::hrir::*;
use super::pressure_velocity::*;
use super::ray_rir::*;
use super::registry::*;
use super::types::*;
use crate::audio::audio_spectral_sheet::SPECTRAL_SIDECAR_MAGIC;
use crate::audio::audio_spectral_sheet_v3::{
    Q4AuHeaderV3, SofaHrirEntry, NORMALIZATION_SN3D_ACN, Q4AU_VERSION_V3, Q4AU_V3_HEADER_BYTES,
    SOFA_DEFAULT_FIR_TAPS, TRANSFORM_KIND_HOA_STFT,
};
use crate::q42::field_semantics::DomainRegistry;

pub(super) fn test_clock() -> ClockSpec {
    ClockSpec {
        scale: 0, // TAI
        epoch_seconds: 1_700_000_000,
        epoch_nanoseconds: 0,
        ticks_per_second: RationalRate::new(48_000, 1).expect("rate"),
    }
}

pub(super) fn test_provenance() -> ProvenanceSpec<'static> {
    ProvenanceSpec {
        role: 0,
        source_digests: &[],
        algorithm: Some("urn:qac:test-fixture"),
        algorithm_version: Some("1"),
        instrument_uri: Some("urn:qac:instrument:ideal"),
        authority_receipt_digest: None,
        lineage: Some("unit-test"),
    }
}

pub(super) fn air_20c_params() -> (f64, f64, f64, f64) {
    let rho: f64 = 1.2041;
    let bulk: f64 = 142_000.0;
    let c = (bulk / rho).sqrt();
    let z = rho * c;
    (rho, bulk, c, z)
}

#[test]
fn test_common_wire_rejects_corruption_and_non_unit_quaternion() {
    let q_bad = QuaternionXyzw::from_xyzw([0.0, 0.0, 0.0, 2.0]);
    assert!(matches!(q_bad, Err(ProfileError::InvalidQuaternion)));

    let t1 = InstantSpec::new(1, 100, 1, 2).expect("instant 1");
    let t2 = InstantSpec::new(1, 100, 2, 3).expect("instant 2");
    let clk = test_clock();
    assert_eq!(t1.cmp_exact(&clk, &t2, &clk).expect("cmp"), std::cmp::Ordering::Less);

    let mut clk_utc = clk;
    clk_utc.scale = 1;
    assert!(matches!(
        t1.cmp_exact(&clk, &t2, &clk_utc),
        Err(ProfileError::ClockScaleMismatch)
    ));

    // Masked row with non-zero bits (including -0.0 or positive finite) must fail closed.
    let (rho, bulk, _, _) = air_20c_params();
    let med = FluidMediumModel::FluidLinear {
        density_kg_m3: rho,
        bulk_modulus_pa: bulk,
        temperature_k: Some(293.15),
        ambient_pressure_pa: Some(101_325.0),
        flow_m_s: None,
        matter_digest: None,
        material_table_digest: None,
        material_table_epoch: None,
        material_ids: Vec::new(),
    };
    let reg = QacProfileRegistry;
    assert_eq!(reg.descriptor_target_schema(4), Some(200));
    assert_eq!(reg.descriptor_target_schema(5), Some(201));
    assert_eq!(reg.descriptor_target_schema(6), Some(202));

    let r_valid = plane_wave_record(
        [0.0, 0.0, 0.0],
        0.0,
        1.2,
        UnitDirection3::new([1.0, 0.0, 0.0]).expect("unit"),
        500.0,
        rho,
        bulk,
    );
    let neg_zero_mask = [0u8, 1u8];
    let mut r_neg_zero = PressureVelocityRecord {
        position_m: [-0.0, 0.0, 0.0],
        pressure_pa: 0.0,
        particle_velocity_m_s: [0.0; 3],
        energy_density_j_m3: 0.0,
        intensity_w_m2: [0.0; 3],
        material_ref: 0,
        validity_bits: 0,
    };
    assert!(matches!(
        write_pressure_velocity_qac(
            &[r_neg_zero, r_valid],
            &med,
            &StaggeringOffsetsPV::collocated(),
            &clk,
            RationalRate::new(48_000, 1).expect("sr"),
            &FrameSpec::local_default(),
            &test_provenance(),
            Some(&neg_zero_mask),
            ProfileLimits::default(),
        ),
        Err(ProfileError::MaskedLaneNonZero)
    ));
    r_neg_zero.position_m = [0.0, 0.0, 0.0];

    let mut qac = write_pressure_velocity_qac(
        &[r_neg_zero, r_valid],
        &med,
        &StaggeringOffsetsPV::collocated(),
        &clk,
        RationalRate::new(48_000, 1).expect("sr"),
        &FrameSpec::local_default(),
        &test_provenance(),
        Some(&neg_zero_mask),
        ProfileLimits::default(),
    )
    .expect("write clean masked pv");

    // Flip last byte inside entry payload -> digest verification must reject.
    let len = qac.len();
    qac[len - 1] ^= 0xFF;
    assert!(read_pressure_velocity_qac(&qac, 1, ProfileLimits::default()).is_err());

    // Corrupt header magic -> reject.
    qac[0] = b'X';
    assert!(read_pressure_velocity_qac(&qac, 1, ProfileLimits::default()).is_err());
}

#[test]
fn test_q03_pressure_velocity_plane_wave_standing_wave_vacuum_and_staggered() {
    let (rho, bulk, _c, z) = air_20c_params();
    let dir = UnitDirection3::new([1.0, 0.0, 0.0]).expect("unit x");
    let med = FluidMediumModel::FluidLinear {
        density_kg_m3: rho,
        bulk_modulus_pa: bulk,
        temperature_k: Some(293.15),
        ambient_pressure_pa: Some(101_325.0),
        flow_m_s: None,
        matter_digest: None,
        material_table_digest: None,
        material_table_epoch: None,
        material_ids: vec![1],
    };

    // Traveling plane wave: verify v = p / Z, E, I = p * v.
    let rec = plane_wave_record([0.0, 0.0, 0.0], 0.0, 2.0, dir, 500.0, rho, bulk);
    let collocated = StaggeringOffsetsPV::collocated();
    let qac = write_pressure_velocity_qac(
        &[rec],
        &med,
        &collocated,
        &test_clock(),
        RationalRate::new(48_000, 1).expect("sr"),
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write pv");
    let view = read_pressure_velocity_qac(&qac, 1, ProfileLimits::default()).expect("read pv");
    let got = view.records.get(0).expect("rec 0");
    assert!((got.pressure_pa - 2.0).abs() < 1e-6);
    assert!((f64::from(got.particle_velocity_m_s[0]) - 2.0 / z).abs() < 1e-6);
    let intensity = derive_collocated_intensity(&got, &view.medium, &view.staggering, false)
        .expect("collocated intensity");
    let energy = derive_collocated_energy_density(&got, &view.medium, &view.staggering, false)
        .expect("collocated energy");
    assert!((intensity[0] - 4.0 / z).abs() < 1e-5);
    assert!((energy - (0.5 * rho * (2.0 / z).powi(2) + 4.0 / (2.0 * bulk))).abs() < 1e-8);

    // Standing wave pressure node (p=0, v!=0) and velocity node (p!=0, v=0): I = 0 while E > 0.
    let p_node = PressureVelocityRecord {
        position_m: [0.1, 0.0, 0.0],
        pressure_pa: 0.0,
        particle_velocity_m_s: [0.004, 0.0, 0.0],
        energy_density_j_m3: (0.5 * rho * 0.004_f64.powi(2)) as f32,
        intensity_w_m2: [0.0; 3],
        material_ref: 0,
        validity_bits: PV_VALID_ALL,
    };
    let v_node = PressureVelocityRecord {
        position_m: [0.2, 0.0, 0.0],
        pressure_pa: 1.5,
        particle_velocity_m_s: [0.0; 3],
        energy_density_j_m3: (1.5_f64.powi(2) / (2.0 * bulk)) as f32,
        intensity_w_m2: [0.0; 3],
        material_ref: 0,
        validity_bits: PV_VALID_ALL,
    };
    assert_eq!(
        derive_collocated_intensity(&p_node, &med, &collocated, false).expect("p node I"),
        [0.0, 0.0, 0.0]
    );
    assert!(derive_collocated_energy_density(&p_node, &med, &collocated, false).expect("p node E") > 0.0);
    assert_eq!(
        derive_collocated_intensity(&v_node, &med, &collocated, false).expect("v node I"),
        [0.0, 0.0, 0.0]
    );
    assert!(derive_collocated_energy_density(&v_node, &med, &collocated, false).expect("v node E") > 0.0);

    // Yee staggered grid must fail closed for un-interpolated instantaneous p*v and energy.
    let yee = StaggeringOffsetsPV::yee_staggered(0.01, 0.01, 0.01);
    let staggered_qac = write_pressure_velocity_qac(
        &[rec],
        &med,
        &yee,
        &test_clock(),
        RationalRate::new(48_000, 1).expect("sr"),
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write yee pv");
    let st_view = read_pressure_velocity_qac(&staggered_qac, 1, ProfileLimits::default()).expect("read yee");
    assert!(matches!(
        derive_collocated_intensity(&rec, &st_view.medium, &st_view.staggering, false),
        Err(ProfileError::NonCollocatedProduct(_))
    ));
    assert!(matches!(
        derive_collocated_energy_density(&rec, &st_view.medium, &st_view.staggering, false),
        Err(ProfileError::NonCollocatedProduct(_))
    ));
    assert!(derive_collocated_intensity(&rec, &st_view.medium, &st_view.staggering, true).is_ok());

    // VacuumMask vs UnknownMedium distinction: VacuumMask requires validity_mask, UnknownMedium refuses c/intensity.
    let vac = FluidMediumModel::VacuumMask { mask_entry_index: 2 };
    assert!(matches!(
        write_pressure_velocity_qac(
            &[rec],
            &vac,
            &collocated,
            &test_clock(),
            RationalRate::new(48_000, 1).expect("sr"),
            &FrameSpec::local_default(),
            &test_provenance(),
            None,
            ProfileLimits::default(),
        ),
        Err(ProfileError::MediumMissingOrUnknown(_))
    ));
    let vac_qac = write_pressure_velocity_qac(
        &[rec],
        &vac,
        &collocated,
        &test_clock(),
        RationalRate::new(48_000, 1).expect("sr"),
        &FrameSpec::local_default(),
        &test_provenance(),
        Some(&[1]),
        ProfileLimits::default(),
    )
    .expect("vacuum mask ok with mask");
    let vac_view = read_pressure_velocity_qac(&vac_qac, 1, ProfileLimits::default()).expect("read vac");
    assert!(matches!(vac_view.medium, FluidMediumModel::VacuumMask { mask_entry_index: 2 }));
    assert!(matches!(
        vac_view.medium.speed_of_sound_m_s(),
        Err(ProfileError::MediumMissingOrUnknown(_))
    ));

    let unk_qac = write_pressure_velocity_qac(
        &[rec],
        &FluidMediumModel::UnknownMedium,
        &collocated,
        &test_clock(),
        RationalRate::new(48_000, 1).expect("sr"),
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write unknown medium");
    let unk_view = read_pressure_velocity_qac(&unk_qac, 1, ProfileLimits::default()).expect("read unk");
    assert!(matches!(unk_view.medium, FluidMediumModel::UnknownMedium));
    assert!(matches!(
        derive_collocated_intensity(&rec, &unk_view.medium, &unk_view.staggering, true),
        Err(ProfileError::MediumMissingOrUnknown(_))
    ));
}

#[test]
fn test_q03_elastic_stress_tensor_voigt_rotation_traction_and_interface() {
    let steel = ElasticMediumSpec::steel();
    let (lambda, mu) = steel.lame_lambda_mu().expect("lame");
    let (cp, cs) = steel.wave_speeds_m_s().expect("wave speeds");
    assert!(cp > cs && cs > 2_500.0);

    // Tensor shear eps_xy = 1e-4 corresponds to Voigt engineering shear gamma_xy = 2e-4.
    let s_tensor = CauchyStress6::from_isotropic_tensor_strain(lambda, mu, [1e-4, -0.5e-4, 0.0, 1e-4, 0.0, 0.0]);
    let s_voigt = CauchyStress6::from_voigt_engineering_strain(lambda, mu, [1e-4, -0.5e-4, 0.0, 0.0, 0.0, 2e-4]);
    assert!((s_tensor.xy - s_voigt.xy).abs() < 1e-3);
    assert!((s_tensor.hydrostatic_pressure_pa() - (-(s_tensor.xx + s_tensor.yy + s_tensor.zz) / 3.0)).abs() < 1e-3);

    // Active rotation R*sigma*R^T about Z by +90 deg maps xx -> yy, yy -> xx, xy -> -xy.
    let q90 = QuaternionXyzw::from_axis_angle([0.0, 0.0, 1.0], PI * 0.5).expect("q90");
    let s_rot = s_tensor.rotate(&q90);
    assert!((s_rot.xx - s_tensor.yy).abs() < 1e-3);
    assert!((s_rot.yy - s_tensor.xx).abs() < 1e-3);
    assert!((s_rot.xy + s_tensor.xy).abs() < 1e-3);
    assert!((s_rot.hydrostatic_pressure_pa() - s_tensor.hydrostatic_pressure_pa()).abs() < 1e-4);

    // Free surface normal n=[0,0,1]: traction vanishes when xz=yz=zz=0.
    let free_surface_stress = CauchyStress6 {
        xx: 120_000.0,
        yy: -40_000.0,
        zz: 0.0,
        xy: 15_000.0,
        yz: 0.0,
        xz: 0.0,
    };
    let t = free_surface_stress.traction_pa([0.0, 0.0, 1.0]);
    assert_eq!(t, [0.0, 0.0, 0.0]);

    // Normal-incidence P-wave interface reflection/transmission satisfies energy conservation.
    let z_air = 413.0;
    let z_water = 1.48e6;
    let (r_p, t_p) = normal_incidence_p_reflection_transmission(z_air, z_water).expect("interface");
    assert!((1.0 + r_p - t_p).abs() < 1e-12);
    let e_balance = r_p * r_p + (z_air / z_water) * t_p * t_p;
    assert!((e_balance - 1.0).abs() < 1e-12);

    // Wire roundtrip for ElasticStressVelocityRecord (mode 0x0001).
    let rec = ElasticStressVelocityRecord {
        velocity_m_s: [0.01, -0.02, 0.03],
        material_ref: 0,
        stress_xx_yy_zz_xy_yz_xz_pa: s_tensor.to_wire_f32(),
        displacement_m: [1e-6, 0.0, -2e-6],
        validity_bits: ELASTIC_VALID_ALL,
    };
    let qac = write_elastic_stress_qac(
        &[rec],
        &steel,
        None,
        &test_clock(),
        RationalRate::new(100_000, 1).expect("sr"),
        &FrameSpec::local_default(),
        &test_provenance(),
        None,
        ProfileLimits::default(),
    )
    .expect("write elastic");
    let view = read_elastic_stress_qac(&qac, 1, ProfileLimits::default()).expect("read elastic");
    assert_eq!(view.records.len(), 1);
    let got = view.records.get(0).expect("rec");
    assert!((got.stress().xy - s_tensor.xy).abs() < 1.0);

    // Anisotropic wave_speeds_m_s must fail closed (never guess isotropic speeds).
    let aniso = ElasticMediumSpec::Anisotropic {
        model_uri: "urn:qac:medium:orthotropic-wood:v1".to_owned(),
        material_ids: vec![7],
    };
    assert!(matches!(
        aniso.wave_speeds_m_s(),
        Err(ProfileError::UnsupportedConstitutiveModel(_))
    ));
}

include!("tests.rs");

