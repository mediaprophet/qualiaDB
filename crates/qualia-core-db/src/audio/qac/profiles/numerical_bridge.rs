//! Bidirectional typed bridges between QAC v1 container profiles (`audio::qac::profiles`)
//! and the continuum / spatial DSP numerical engines (`audio::qac::numerical`, `etalon_sim`).

use super::elastic::{
    CauchyStress6 as ProfileCauchyStress6, ElasticStressChunkView, ElasticStressVelocityRecord,
    ELASTIC_VALID_DISPLACEMENT, ELASTIC_VALID_STRESS, ELASTIC_VALID_VELOCITY,
};
use super::pressure_velocity::{
    PressureVelocityChunkView, PressureVelocityRecord, PV_VALID_ENERGY, PV_VALID_INTENSITY,
    PV_VALID_PRESSURE, PV_VALID_VELOCITY,
};
use super::types::ProfileError;
use crate::audio::qac::numerical::{
    elastic_virieux::{
        CauchyStress6 as NumericalCauchyStress6,
        ElasticStressVelocityRecord as NumericalElasticRecord, ESV_VALID_DISPLACEMENT,
        ESV_VALID_STRESS, ESV_VALID_VELOCITY,
    },
    fdtd_fluid::{
        collocate_fluid_cell, FluidFdtdConfig, FluidFdtdState,
        PressureVelocityRecord as NumericalPvRecord, PV_VALID_ENERGY_DENSITY,
        PV_VALID_INTENSITY as NUM_PV_VALID_INTENSITY, PV_VALID_POSITION,
        PV_VALID_PRESSURE as NUM_PV_VALID_PRESSURE, PV_VALID_VELOCITY as NUM_PV_VALID_VELOCITY,
    },
    NumericalError,
};

/// Translates profile Mode `0x0000` validity bits (`PV_VALID_*`) to numerical continuum bits.
pub const fn profile_pv_mask_to_numerical(profile_mask: u32) -> u32 {
    let mut out = PV_VALID_POSITION;
    if (profile_mask & PV_VALID_PRESSURE) != 0 {
        out |= NUM_PV_VALID_PRESSURE;
    }
    if (profile_mask & PV_VALID_VELOCITY) != 0 {
        out |= NUM_PV_VALID_VELOCITY;
    }
    if (profile_mask & PV_VALID_ENERGY) != 0 {
        out |= PV_VALID_ENERGY_DENSITY;
    }
    if (profile_mask & PV_VALID_INTENSITY) != 0 {
        out |= NUM_PV_VALID_INTENSITY;
    }
    out
}

/// Translates numerical Mode `0x0000` validity bits back to storage profile `PV_VALID_*` bits.
pub const fn numerical_pv_mask_to_profile(num_mask: u32) -> u32 {
    let mut out = 0u32;
    if (num_mask & NUM_PV_VALID_PRESSURE) != 0 {
        out |= PV_VALID_PRESSURE;
    }
    if (num_mask & NUM_PV_VALID_VELOCITY) != 0 {
        out |= PV_VALID_VELOCITY;
    }
    if (num_mask & PV_VALID_ENERGY_DENSITY) != 0 {
        out |= PV_VALID_ENERGY;
    }
    if (num_mask & NUM_PV_VALID_INTENSITY) != 0 {
        out |= PV_VALID_INTENSITY;
    }
    out
}

impl From<&PressureVelocityRecord> for NumericalPvRecord {
    fn from(r: &PressureVelocityRecord) -> Self {
        Self {
            position_m: r.position_m,
            pressure_pa: r.pressure_pa,
            particle_velocity_m_s: r.particle_velocity_m_s,
            energy_density_j_m3: r.energy_density_j_m3,
            intensity_w_m2: r.intensity_w_m2,
            material_ref: r.material_ref,
            validity_bits: profile_pv_mask_to_numerical(r.validity_bits),
        }
    }
}

impl From<&NumericalPvRecord> for PressureVelocityRecord {
    fn from(r: &NumericalPvRecord) -> Self {
        Self {
            position_m: r.position_m,
            pressure_pa: r.pressure_pa,
            particle_velocity_m_s: r.particle_velocity_m_s,
            energy_density_j_m3: r.energy_density_j_m3,
            intensity_w_m2: r.intensity_w_m2,
            material_ref: r.material_ref,
            validity_bits: numerical_pv_mask_to_profile(r.validity_bits),
        }
    }
}

impl From<ProfileCauchyStress6> for NumericalCauchyStress6 {
    fn from(s: ProfileCauchyStress6) -> Self {
        Self::new([s.xx, s.yy, s.zz, s.xy, s.yz, s.xz])
    }
}

impl From<NumericalCauchyStress6> for ProfileCauchyStress6 {
    fn from(s: NumericalCauchyStress6) -> Self {
        Self::from_wire_f32([
            s.components[0] as f32,
            s.components[1] as f32,
            s.components[2] as f32,
            s.components[3] as f32,
            s.components[4] as f32,
            s.components[5] as f32,
        ])
    }
}

impl From<&ElasticStressVelocityRecord> for NumericalElasticRecord {
    fn from(r: &ElasticStressVelocityRecord) -> Self {
        let mut mask = 0u32;
        if (r.validity_bits & ELASTIC_VALID_VELOCITY) != 0 {
            mask |= ESV_VALID_VELOCITY;
        }
        if (r.validity_bits & ELASTIC_VALID_STRESS) != 0 {
            mask |= ESV_VALID_STRESS;
        }
        if (r.validity_bits & ELASTIC_VALID_DISPLACEMENT) != 0 {
            mask |= ESV_VALID_DISPLACEMENT;
        }
        Self {
            velocity_m_s: r.velocity_m_s,
            material_ref: r.material_ref,
            stress_xx_yy_zz_xy_yz_xz_pa: r.stress_xx_yy_zz_xy_yz_xz_pa,
            displacement_m: r.displacement_m,
            validity_bits: mask,
        }
    }
}

impl From<&NumericalElasticRecord> for ElasticStressVelocityRecord {
    fn from(r: &NumericalElasticRecord) -> Self {
        let mut mask = 0u32;
        if (r.validity_bits & ESV_VALID_VELOCITY) != 0 {
            mask |= ELASTIC_VALID_VELOCITY;
        }
        if (r.validity_bits & ESV_VALID_STRESS) != 0 {
            mask |= ELASTIC_VALID_STRESS;
        }
        if (r.validity_bits & ESV_VALID_DISPLACEMENT) != 0 {
            mask |= ELASTIC_VALID_DISPLACEMENT;
        }
        Self {
            velocity_m_s: r.velocity_m_s,
            material_ref: r.material_ref,
            stress_xx_yy_zz_xy_yz_xz_pa: r.stress_xx_yy_zz_xy_yz_xz_pa,
            displacement_m: r.displacement_m,
            validity_bits: mask,
        }
    }
}

/// Decodes an admitted `0x0000` profile chunk directly into validated `numerical` solver records.
pub fn chunk_to_numerical_pv_records(
    chunk: &PressureVelocityChunkView<'_>,
) -> Result<Vec<NumericalPvRecord>, ProfileError> {
    let mut out = Vec::with_capacity(chunk.len());
    for i in 0..chunk.len() {
        let rec = chunk.get(i)?;
        out.push(NumericalPvRecord::from(&rec));
    }
    Ok(out)
}

/// Decodes an admitted `0x0001` profile chunk directly into validated `numerical` Virieux records.
pub fn chunk_to_numerical_elastic_records(
    chunk: &ElasticStressChunkView<'_>,
) -> Result<Vec<NumericalElasticRecord>, ProfileError> {
    let mut out = Vec::with_capacity(chunk.len());
    for i in 0..chunk.len() {
        let rec = chunk.get(i)?;
        out.push(NumericalElasticRecord::from(&rec));
    }
    Ok(out)
}

/// Exports collocated `FluidFdtdState` fields as verified Mode `0x0000` storage profile records
/// using staggered spatial/temporal interpolation via [`collocate_fluid_cell`].
pub fn export_fdtd_cell_records(
    cfg: &FluidFdtdConfig,
    state: &FluidFdtdState,
) -> Result<Vec<PressureVelocityRecord>, NumericalError> {
    cfg.validate()?;
    let [nx, ny, nz] = cfg.dims;
    let total = cfg.cell_count();
    if state.p.len() != total {
        return Err(NumericalError::BufferLengthMismatch {
            buffer: "FluidFdtdState.p",
            expected: total,
            actual: state.p.len(),
        });
    }
    let mut out = Vec::with_capacity(total);
    for x in 0..nx {
        for y in 0..ny {
            for z in 0..nz {
                let (p, col_v, energy, intensity) = collocate_fluid_cell(
                    cfg,
                    &state.p,
                    (&state.prev_vx, &state.prev_vy, &state.prev_vz),
                    (&state.vx, &state.vy, &state.vz),
                    [x, y, z],
                )?;
                let pos = [
                    x as f32 * cfg.spacing_m[0] as f32,
                    y as f32 * cfg.spacing_m[1] as f32,
                    z as f32 * cfg.spacing_m[2] as f32,
                ];
                let nrec = NumericalPvRecord {
                    position_m: pos,
                    pressure_pa: p as f32,
                    particle_velocity_m_s: [col_v[0] as f32, col_v[1] as f32, col_v[2] as f32],
                    energy_density_j_m3: energy as f32,
                    intensity_w_m2: [intensity[0] as f32, intensity[1] as f32, intensity[2] as f32],
                    material_ref: cfg.medium.material_ref,
                    validity_bits: PV_VALID_POSITION
                        | NUM_PV_VALID_PRESSURE
                        | NUM_PV_VALID_VELOCITY
                        | PV_VALID_ENERGY_DENSITY
                        | NUM_PV_VALID_INTENSITY,
                };
                nrec.validate()?;
                out.push(PressureVelocityRecord::from(&nrec));
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::envelope::{AdmittedEnvelope, AdmittedEntryKind};
    use crate::audio::qac::metadata::SignalRegistry;
    use crate::q42::field_semantics::DomainRegistry;

    #[test]
    fn gap_023_signal_registry_delegates_domain_schemas_and_s201_keys() {
        let reg = SignalRegistry;
        assert_eq!(reg.descriptor_target_schema(4), Some(200));
        assert_eq!(reg.descriptor_target_schema(5), Some(201));
        assert_eq!(reg.descriptor_target_schema(6), Some(202));
        for schema in [200u16, 201, 202, 203, 204, 205, 206, 207, 208, 209, 210] {
            assert!(reg.rules(schema, 1).is_some(), "missing schema {schema}");
        }
        let s201 = super::super::registry::QacProfileRegistry.rules(201, 1).unwrap();
        assert!(s201.iter().any(|r| r.key == 16 && r.type_tag == 4));
        assert!(s201.iter().any(|r| r.key == 17 && r.type_tag == 6));
    }

    #[test]
    fn gap_026_pv_and_elastic_profile_numerical_roundtrip_and_entry_summary() {
        let pv = PressureVelocityRecord {
            position_m: [0.1, 0.2, 0.3],
            pressure_pa: 2.5,
            particle_velocity_m_s: [0.005, -0.001, 0.0],
            energy_density_j_m3: 0.04,
            intensity_w_m2: [0.0125, -0.0025, 0.0],
            material_ref: 7,
            validity_bits: PV_VALID_PRESSURE | PV_VALID_VELOCITY | PV_VALID_ENERGY | PV_VALID_INTENSITY,
        };
        let npv = NumericalPvRecord::from(&pv);
        assert_eq!(npv.pressure_pa, pv.pressure_pa);
        assert_eq!(npv.material_ref, 7);
        assert_ne!(npv.validity_bits & PV_VALID_POSITION, 0);
        let back = PressureVelocityRecord::from(&npv);
        assert_eq!(back, pv);

        let es = ElasticStressVelocityRecord {
            velocity_m_s: [0.1, 0.2, 0.3],
            material_ref: 3,
            stress_xx_yy_zz_xy_yz_xz_pa: [10.0, 20.0, 30.0, 1.0, 2.0, 3.0],
            displacement_m: [0.001, 0.002, 0.003],
            validity_bits: ELASTIC_VALID_VELOCITY | ELASTIC_VALID_STRESS | ELASTIC_VALID_DISPLACEMENT,
        };
        let nes = NumericalElasticRecord::from(&es);
        assert_eq!(ElasticStressVelocityRecord::from(&nes), es);
        assert_eq!(AdmittedEntryKind::ModalResonance.is_signal_waveform_entry(), false);
        assert_eq!(AdmittedEntryKind::PcmPreservation.is_signal_waveform_entry(), true);
    }

    #[test]
    fn gap_024_and_026_fdtd_step_export_qac_roundtrip_and_summary_admission() {
        use super::super::pressure_velocity::{
            read_pressure_velocity_qac, write_pressure_velocity_qac, FluidMediumModel,
            StaggeringOffsetsPV,
        };
        use super::super::types::{
            ClockSpec, FrameSpec, ProfileLimits, ProvenanceSpec, RationalRate,
        };
        use crate::audio::qac::numerical::fdtd_fluid::{
            step_fluid_fdtd_in_place, FluidBoundaryCondition, FluidMedium,
        };

        let cfg = FluidFdtdConfig {
            dims: [4, 4, 4],
            spacing_m: [0.05, 0.05, 0.05],
            dt_s: 2.0e-5,
            medium: FluidMedium::new(1.2041, 142_000.0, 0.0, 0).expect("air"),
            boundary: FluidBoundaryCondition::Rigid,
        };
        let mut state = FluidFdtdState::new(&cfg).expect("state");
        state.p[(1 * 4 + 1) * 4 + 1] = 10.0;
        step_fluid_fdtd_in_place(&cfg, &mut state).expect("step");

        let exported = export_fdtd_cell_records(&cfg, &state).expect("export");
        assert_eq!(exported.len(), 64);
        assert!(exported.iter().any(|r| r.pressure_pa.abs() > 0.0));

        let medium_model = FluidMediumModel::FluidLinear {
            density_kg_m3: cfg.medium.density_kg_m3,
            bulk_modulus_pa: cfg.medium.bulk_modulus_pa,
            temperature_k: None,
            ambient_pressure_pa: None,
            flow_m_s: None,
            matter_digest: None,
            material_table_digest: None,
            material_table_epoch: None,
            material_ids: Vec::new(),
        };
        let clk = ClockSpec {
            scale: 0,
            epoch_seconds: 1_700_000_000,
            epoch_nanoseconds: 0,
            ticks_per_second: RationalRate::new(48_000, 1).expect("hz"),
        };
        let prov = ProvenanceSpec {
            role: 0,
            source_digests: &[],
            algorithm: Some("urn:qac:test-fdtd-bridge"),
            algorithm_version: Some("1"),
            instrument_uri: None,
            authority_receipt_digest: None,
            lineage: None,
        };
        let qac = write_pressure_velocity_qac(
            &exported,
            &medium_model,
            &StaggeringOffsetsPV::collocated(),
            &clk,
            RationalRate::new(50_000, 1).expect("rate"),
            &FrameSpec::local_default(),
            &prov,
            None,
            ProfileLimits::default(),
        )
        .expect("write_pressure_velocity_qac");

        let env = AdmittedEnvelope::open_alloc(&qac, ProfileLimits::default()).expect("admit");
        let summary = env.inspect_entry_summary(1).expect("summary");
        assert_eq!(summary.kind, AdmittedEntryKind::PressureVelocity);
        assert_eq!(summary.kind.as_str(), "pressure-velocity");
        assert!(!summary.kind.is_signal_waveform_entry());

        let view = read_pressure_velocity_qac(&qac, 1, ProfileLimits::default()).expect("read");
        let back = chunk_to_numerical_pv_records(&view.records).expect("convert back");
        assert_eq!(back.len(), 64);
        assert_eq!(back[0].pressure_pa, exported[0].pressure_pa);
    }
}
