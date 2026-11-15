//! Q03 Mode `0x0000`: Continuum acoustic pressure perturbation and particle velocity (`64 B`).
//!
//! Preserves instantaneous `p`, `v`, energy density `e`, acoustic intensity `I`, explicit
//! `VacuumMask` vs `UnknownMedium` semantics, spatial/temporal staggering descriptors,
//! viscosity/dispersion/anisotropy/poroelastic extensions, and fail-closed noncollocated guards.

use super::envelope::*;
use super::types::*;

pub const PV_RECORD_STRIDE: u32 = 64;

pub const PV_VALID_PRESSURE: u32 = 1 << 0;
pub const PV_VALID_VELOCITY: u32 = 1 << 1;
pub const PV_VALID_ENERGY: u32 = 1 << 2;
pub const PV_VALID_INTENSITY: u32 = 1 << 3;
pub const PV_VALID_ALL: u32 =
    PV_VALID_PRESSURE | PV_VALID_VELOCITY | PV_VALID_ENERGY | PV_VALID_INTENSITY;

/// Exact 64-byte wire record for `mode 0x0000` (`pressure_velocity_continuum`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressureVelocityRecord {
    pub position_m: [f32; 3],
    pub pressure_pa: f32,
    pub particle_velocity_m_s: [f32; 3],
    pub energy_density_j_m3: f32,
    pub intensity_w_m2: [f32; 3],
    pub material_ref: u32,
    pub validity_bits: u32,
}

impl PressureVelocityRecord {
    pub fn validate(&self, externally_valid: bool) -> Result<(), ProfileError> {
        for c in self.position_m {
            if !c.is_finite() {
                return Err(ProfileError::NonFiniteFloat);
            }
        }
        if !self.pressure_pa.is_finite()
            || !self.energy_density_j_m3.is_finite()
            || self.particle_velocity_m_s.iter().any(|v| !v.is_finite())
            || self.intensity_w_m2.iter().any(|v| !v.is_finite())
        {
            return Err(ProfileError::NonFiniteFloat);
        }
        if self.validity_bits & !PV_VALID_ALL != 0 {
            return Err(ProfileError::InvalidValidityBits);
        }
        if !externally_valid {
            if self.validity_bits != 0
                || self.position_m.iter().any(|v| v.to_bits() != 0)
                || self.pressure_pa.to_bits() != 0
                || self.particle_velocity_m_s.iter().any(|v| v.to_bits() != 0)
                || self.energy_density_j_m3.to_bits() != 0
                || self.intensity_w_m2.iter().any(|v| v.to_bits() != 0)
            {
                return Err(ProfileError::MaskedLaneNonZero);
            }
            return Ok(());
        }
        require_zero_when_masked((self.validity_bits & PV_VALID_PRESSURE) != 0, self.pressure_pa)?;
        require_zero_vec3_when_masked(
            (self.validity_bits & PV_VALID_VELOCITY) != 0,
            self.particle_velocity_m_s,
        )?;
        require_zero_when_masked(
            (self.validity_bits & PV_VALID_ENERGY) != 0,
            self.energy_density_j_m3,
        )?;
        require_zero_vec3_when_masked(
            (self.validity_bits & PV_VALID_INTENSITY) != 0,
            self.intensity_w_m2,
        )?;
        if (self.validity_bits & PV_VALID_ENERGY) != 0 && self.energy_density_j_m3 < 0.0 {
            return Err(ProfileError::InvalidMetadata("energy_density_j_m3 must be >= 0"));
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8], externally_valid: bool) -> Result<Self, ProfileError> {
        if bytes.len() != PV_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        ensure_zero_bytes(&bytes[52..64])?;
        let rec = Self {
            position_m: read_f32_3(bytes, 0)?,
            pressure_pa: read_f32_finite(bytes, 12)?,
            particle_velocity_m_s: read_f32_3(bytes, 16)?,
            energy_density_j_m3: read_f32_finite(bytes, 28)?,
            intensity_w_m2: read_f32_3(bytes, 32)?,
            material_ref: read_u32(bytes, 44)?,
            validity_bits: read_u32(bytes, 48)?,
        };
        rec.validate(externally_valid)?;
        Ok(rec)
    }

    pub fn encode_into(&self, out: &mut [u8], externally_valid: bool) -> Result<(), ProfileError> {
        if out.len() != PV_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        self.validate(externally_valid)?;
        out.fill(0);
        write_f32_slice(out, 0, &self.position_m)?;
        write_f32_finite(out, 12, self.pressure_pa)?;
        write_f32_slice(out, 16, &self.particle_velocity_m_s)?;
        write_f32_finite(out, 28, self.energy_density_j_m3)?;
        write_f32_slice(out, 32, &self.intensity_w_m2)?;
        write_u32(out, 44, self.material_ref);
        write_u32(out, 48, self.validity_bits);
        Ok(())
    }
}

/// Zero-heap iterator view over verified `0x0000` records.
pub struct PressureVelocityChunkView<'a> {
    bytes: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> PressureVelocityChunkView<'a> {
    pub fn len(&self) -> usize {
        self.bytes.len() / PV_RECORD_STRIDE as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<PressureVelocityRecord, ProfileError> {
        let start = index
            .checked_mul(PV_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let end = start
            .checked_add(PV_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("pv record index"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        PressureVelocityRecord::decode(slice, ext_valid)
    }
}

/// Explicit fluid medium vs vacuum mask vs unknown medium (`Schema 200`).
#[derive(Debug, Clone, PartialEq)]
pub enum FluidMediumModel {
    /// Linear nondispersive acoustic fluid with strictly positive `rho` and `K`.
    FluidLinear {
        density_kg_m3: f64,
        bulk_modulus_pa: f64,
        temperature_k: Option<f64>,
        ambient_pressure_pa: Option<f64>,
        flow_m_s: Option<[f64; 3]>,
        matter_digest: Option<[u8; 32]>,
        material_table_digest: Option<[u8; 32]>,
        material_table_epoch: Option<u64>,
        material_ids: Vec<u64>,
    },
    /// Explicit vacuum domain occupancy mask (`0x0106`); never synthesized by `rho == 0`.
    VacuumMask { mask_entry_index: u32 },
    /// Explicitly unknown medium; never treated as vacuum and never used to compute `c` or `e`.
    UnknownMedium,
    /// Storage-admissible extended rheology/dispersion/anisotropy/poroelastic profile.
    ExtendedConstitutive {
        model_uri: String,
        viscosity_pa_s: Option<f64>,
        dispersion_model_uri: Option<String>,
        anisotropy_uri: Option<String>,
        poroelastic_uri: Option<String>,
        material_ids: Vec<u64>,
    },
}

impl FluidMediumModel {
    pub fn air_20c() -> Self {
        Self::FluidLinear {
            density_kg_m3: 1.204,
            bulk_modulus_pa: 142_000.0,
            temperature_k: Some(293.15),
            ambient_pressure_pa: Some(101_325.0),
            flow_m_s: None,
            matter_digest: None,
            material_table_digest: None,
            material_table_epoch: None,
            material_ids: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        match self {
            Self::FluidLinear {
                density_kg_m3,
                bulk_modulus_pa,
                temperature_k,
                ambient_pressure_pa,
                flow_m_s,
                material_table_digest,
                material_table_epoch,
                ..
            } => {
                if !density_kg_m3.is_finite() || *density_kg_m3 <= 0.0 {
                    return Err(ProfileError::MediumMissingOrUnknown(
                        "fluid density must be finite > 0 (use VacuumMask for vacuum)",
                    ));
                }
                if !bulk_modulus_pa.is_finite() || *bulk_modulus_pa <= 0.0 {
                    return Err(ProfileError::MediumMissingOrUnknown(
                        "fluid bulk modulus must be finite > 0",
                    ));
                }
                if let Some(t) = temperature_k {
                    if !t.is_finite() || *t <= 0.0 {
                        return Err(ProfileError::MediumMissingOrUnknown("temperature must be > 0 K"));
                    }
                }
                if let Some(p0) = ambient_pressure_pa {
                    if !p0.is_finite() || *p0 <= 0.0 {
                        return Err(ProfileError::MediumMissingOrUnknown("ambient pressure must be > 0 Pa"));
                    }
                }
                if let Some(flow) = flow_m_s {
                    if flow.iter().any(|v| !v.is_finite()) {
                        return Err(ProfileError::NonFiniteFloat);
                    }
                }
                if material_table_digest.is_some() ^ material_table_epoch.is_some() {
                    return Err(ProfileError::InvalidMetadata("table_digest and epoch must pair"));
                }
                Ok(())
            }
            Self::VacuumMask { mask_entry_index } => {
                if *mask_entry_index == 0 {
                    return Err(ProfileError::MediumMissingOrUnknown("vacuum mask entry must be >= 1"));
                }
                Ok(())
            }
            Self::UnknownMedium => Ok(()),
            Self::ExtendedConstitutive {
                model_uri,
                viscosity_pa_s,
                ..
            } => {
                validate_identifier_str(model_uri, "medium.model_uri")?;
                if let Some(mu) = viscosity_pa_s {
                    if !mu.is_finite() || *mu < 0.0 {
                        return Err(ProfileError::InvalidMetadata("viscosity_pa_s"));
                    }
                }
                Ok(())
            }
        }
    }

    pub fn speed_of_sound_m_s(&self) -> Result<f64, ProfileError> {
        match self {
            Self::FluidLinear {
                density_kg_m3,
                bulk_modulus_pa,
                ..
            } => {
                self.validate()?;
                Ok((bulk_modulus_pa / density_kg_m3).sqrt())
            }
            Self::VacuumMask { .. } => Err(ProfileError::MediumMissingOrUnknown(
                "vacuum cannot propagate linear acoustic pressure waves",
            )),
            Self::UnknownMedium => Err(ProfileError::MediumMissingOrUnknown(
                "unknown medium cannot supply speed of sound",
            )),
            Self::ExtendedConstitutive { .. } => Err(ProfileError::UnsupportedConstitutiveModel(
                "extended viscous/dispersive/anisotropic/poroelastic medium requires dedicated solver profile",
            )),
        }
    }

    pub fn impedance_rayl(&self) -> Result<f64, ProfileError> {
        match self {
            Self::FluidLinear {
                density_kg_m3,
                bulk_modulus_pa,
                ..
            } => {
                self.validate()?;
                Ok((density_kg_m3 * bulk_modulus_pa).sqrt())
            }
            _ => Err(ProfileError::MediumMissingOrUnknown(
                "impedance undefined outside FluidLinear",
            )),
        }
    }

    fn push_schema_200(&self, meta: &mut MetadataBuilder, id: u32) -> Result<u32, ProfileError> {
        self.validate()?;
        let mut fields = Vec::new();
        match self {
            Self::FluidLinear {
                density_kg_m3,
                bulk_modulus_pa,
                temperature_k,
                ambient_pressure_pa,
                flow_m_s,
                matter_digest,
                material_table_digest,
                material_table_epoch,
                material_ids,
            } => {
                fields.push(OwnedField::crit(
                    1,
                    OwnedValue::Utf8("fluid-linear".to_owned()),
                ));
                fields.push(OwnedField::crit(2, OwnedValue::F64(vec![*density_kg_m3])));
                fields.push(OwnedField::crit(3, OwnedValue::F64(vec![*bulk_modulus_pa])));
                if let Some(t) = temperature_k {
                    fields.push(OwnedField::opt(5, OwnedValue::F64(vec![*t])));
                }
                if let Some(p) = ambient_pressure_pa {
                    fields.push(OwnedField::opt(6, OwnedValue::F64(vec![*p])));
                }
                if let Some(flow) = flow_m_s {
                    fields.push(OwnedField::opt(7, OwnedValue::F64(flow.to_vec())));
                }
                if let Some(d) = matter_digest {
                    fields.push(OwnedField::opt(8, OwnedValue::Bytes(d.to_vec())));
                }
                if let Some(td) = material_table_digest {
                    fields.push(OwnedField::opt(9, OwnedValue::Bytes(td.to_vec())));
                }
                if let Some(te) = material_table_epoch {
                    fields.push(OwnedField::opt(10, OwnedValue::U64(vec![*te])));
                }
                if !material_ids.is_empty() {
                    fields.push(OwnedField::opt(11, OwnedValue::U64(material_ids.clone())));
                }
            }
            Self::VacuumMask { mask_entry_index } => {
                fields.push(OwnedField::crit(
                    1,
                    OwnedValue::Utf8("vacuum-mask".to_owned()),
                ));
                fields.push(OwnedField::crit(
                    13,
                    OwnedValue::EntryRefs(vec![*mask_entry_index]),
                ));
            }
            Self::UnknownMedium => {
                fields.push(OwnedField::crit(
                    1,
                    OwnedValue::Utf8("unknown-medium".to_owned()),
                ));
            }
            Self::ExtendedConstitutive {
                model_uri,
                material_ids,
                ..
            } => {
                fields.push(OwnedField::crit(1, OwnedValue::Utf8(model_uri.clone())));
                if !material_ids.is_empty() {
                    fields.push(OwnedField::opt(11, OwnedValue::U64(material_ids.clone())));
                }
            }
        }
        meta.push_block(OwnedBlock {
            id,
            schema_type: 200,
            revision: 1,
            critical: true,
            fields,
        })
    }
}

/// Staggered spatial/temporal component offset descriptor (`Schema 205` keys 4, 5, 16).
#[derive(Debug, Clone, PartialEq)]
pub struct StaggeringOffsetsPV {
    /// `[p, vx, vy, vz]` XYZ spatial offsets in grid cells / metres.
    pub component_offsets_xyz: [f64; 12],
    /// `[p, vx, vy, vz]` rational time offset numerators (ticks).
    pub time_offset_num: [i64; 4],
    /// `[p, vx, vy, vz]` rational time offset denominators (`> 0`).
    pub time_offset_den: [u64; 4],
}

impl Default for StaggeringOffsetsPV {
    fn default() -> Self {
        Self::collocated()
    }
}

impl StaggeringOffsetsPV {
    pub fn collocated() -> Self {
        Self {
            component_offsets_xyz: [0.0; 12],
            time_offset_num: [0; 4],
            time_offset_den: [1; 4],
        }
    }

    pub fn yee_staggered(dx: f64, dy: f64, dz: f64) -> Self {
        Self {
            component_offsets_xyz: [
                0.0, 0.0, 0.0,
                0.5 * dx, 0.0, 0.0,
                0.0, 0.5 * dy, 0.0,
                0.0, 0.0, 0.5 * dz,
            ],
            time_offset_num: [0, 1, 1, 1],
            time_offset_den: [1, 2, 2, 2],
        }
    }

    pub fn is_collocated(&self) -> bool {
        let p_xyz = &self.component_offsets_xyz[0..3];
        let same_space = self.component_offsets_xyz.chunks_exact(3).all(|c| c == p_xyz);
        let same_time = self
            .time_offset_num
            .iter()
            .zip(self.time_offset_den)
            .all(|(&n, d)| n * (self.time_offset_den[0] as i64) == self.time_offset_num[0] * (d as i64));
        same_space && same_time
    }
}

/// Computes instantaneous acoustic intensity `I = p * v` ONLY when medium & collocation permit.
pub fn derive_collocated_intensity(
    rec: &PressureVelocityRecord,
    medium: &FluidMediumModel,
    staggering: &StaggeringOffsetsPV,
    collocated_after_interpolation: bool,
) -> Result<[f64; 3], ProfileError> {
    rec.validate(true)?;
    let _ = medium.speed_of_sound_m_s()?;
    if (rec.validity_bits & (PV_VALID_PRESSURE | PV_VALID_VELOCITY)) != (PV_VALID_PRESSURE | PV_VALID_VELOCITY) {
        return Err(ProfileError::InvalidValidityBits);
    }
    if !staggering.is_collocated() && !collocated_after_interpolation {
        return Err(ProfileError::NonCollocatedProduct(
            "cannot form instantaneous p * v across staggered components without explicit interpolation",
        ));
    }
    let p = f64::from(rec.pressure_pa);
    Ok([
        p * f64::from(rec.particle_velocity_m_s[0]),
        p * f64::from(rec.particle_velocity_m_s[1]),
        p * f64::from(rec.particle_velocity_m_s[2]),
    ])
}

/// Computes instantaneous energy density `rho*|v|^2/2 + p^2/(2*K)` only for valid collocated fluid fields.
pub fn derive_collocated_energy_density(
    rec: &PressureVelocityRecord,
    medium: &FluidMediumModel,
    staggering: &StaggeringOffsetsPV,
    collocated_after_interpolation: bool,
) -> Result<f64, ProfileError> {
    rec.validate(true)?;
    let (rho, bulk) = match medium {
        FluidMediumModel::FluidLinear {
            density_kg_m3,
            bulk_modulus_pa,
            ..
        } => {
            medium.validate()?;
            (*density_kg_m3, *bulk_modulus_pa)
        }
        _ => {
            let _ = medium.speed_of_sound_m_s()?;
            unreachable!()
        }
    };
    if (rec.validity_bits & (PV_VALID_PRESSURE | PV_VALID_VELOCITY)) != (PV_VALID_PRESSURE | PV_VALID_VELOCITY) {
        return Err(ProfileError::InvalidValidityBits);
    }
    if !staggering.is_collocated() && !collocated_after_interpolation {
        return Err(ProfileError::NonCollocatedProduct(
            "cannot form energy density across staggered p and v without explicit interpolation",
        ));
    }
    let p = f64::from(rec.pressure_pa);
    let vx = f64::from(rec.particle_velocity_m_s[0]);
    let vy = f64::from(rec.particle_velocity_m_s[1]);
    let vz = f64::from(rec.particle_velocity_m_s[2]);
    let v2 = vx * vx + vy * vy + vz * vz;
    Ok(0.5 * rho * v2 + (p * p) / (2.0 * bulk))
}

/// Analytic plane traveling wave record generator for Q03 verification fixtures.
pub fn plane_wave_record(
    pos_m: [f32; 3],
    t_s: f64,
    amp_pa: f64,
    dir: UnitDirection3,
    freq_hz: f64,
    rho: f64,
    bulk_modulus_pa: f64,
) -> PressureVelocityRecord {
    let c = (bulk_modulus_pa / rho).sqrt();
    let z = rho * c;
    let omega = 2.0 * std::f64::consts::PI * freq_hz;
    let k = omega / c;
    let dot = f64::from(pos_m[0]) * f64::from(dir.0[0])
        + f64::from(pos_m[1]) * f64::from(dir.0[1])
        + f64::from(pos_m[2]) * f64::from(dir.0[2]);
    let p = amp_pa * (k * dot - omega * t_s).cos();
    let v_mag = p / z;
    let vx = v_mag * f64::from(dir.0[0]);
    let vy = v_mag * f64::from(dir.0[1]);
    let vz = v_mag * f64::from(dir.0[2]);
    let e = 0.5 * rho * (vx * vx + vy * vy + vz * vz) + (p * p) / (2.0 * bulk_modulus_pa);
    PressureVelocityRecord {
        position_m: pos_m,
        pressure_pa: p as f32,
        particle_velocity_m_s: [vx as f32, vy as f32, vz as f32],
        energy_density_j_m3: e as f32,
        intensity_w_m2: [(p * vx) as f32, (p * vy) as f32, (p * vz) as f32],
        material_ref: 0,
        validity_bits: PV_VALID_ALL,
    }
}

/// Writes a canonical QAC v1 `mode 0x0000` container with optional record validity mask.
pub fn write_pressure_velocity_qac(
    records: &[PressureVelocityRecord],
    medium: &FluidMediumModel,
    staggering: &StaggeringOffsetsPV,
    clock: &ClockSpec,
    sample_rate: RationalRate,
    frame: &FrameSpec<'_>,
    provenance: &ProvenanceSpec<'_>,
    validity_mask: Option<&[u8]>,
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    if records.is_empty() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if matches!(medium, FluidMediumModel::VacuumMask { .. }) && validity_mask.is_none() {
        return Err(ProfileError::MediumMissingOrUnknown(
            "VacuumMask requires an accompanying validity_mask entry",
        ));
    }
    if let Some(m) = validity_mask {
        if m.len() != records.len() || m.iter().any(|&b| b > 1) {
            return Err(ProfileError::InvalidValidityBits);
        }
    }
    let mut payload = vec![0u8; records.len() * PV_RECORD_STRIDE as usize];
    for (idx, rec) in records.iter().enumerate() {
        let ext_valid = validity_mask.map_or(true, |m| m[idx] != 0);
        let off = idx * PV_RECORD_STRIDE as usize;
        rec.encode_into(&mut payload[off..off + PV_RECORD_STRIDE as usize], ext_valid)?;
    }

    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, clock)?;
    meta.push_frame(2, frame)?;
    meta.push_provenance(3, provenance)?;
    medium.push_schema_200(&mut meta, 4)?;
    let shape = [records.len() as u64];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: "urn:qac:model:pressure-velocity-continuum:v1",
            model_version: "1",
            discretization: Some("fdtd-continuum"),
            component_offsets_xyz: &staggering.component_offsets_xyz,
            component_time_offset_num: &staggering.time_offset_num,
            component_time_offset_den: &staggering.time_offset_den,
            timestep: Some(sample_rate),
            tolerance_abs: Some(1e-6),
            tolerance_rel: Some(1e-5),
            tensor_shape: &shape,
            component_names: "p,vx,vy,vz",
        },
    )?;

    let valid_count = validity_mask.map(|m| m.iter().filter(|&&b| b == 1).count() as u64);
    let mask_entry_idx = if validity_mask.is_some() { Some(2) } else { None };
    let ext = [5u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: Some(1),
            frame_id: Some(2),
            provenance_id: 3,
            medium_id: Some(4),
            transform_id: None,
            calibration_id: None,
            quantity: "acoustic-pressure-velocity",
            units: "mixed:mode-registry",
            channels: None,
            layout: None,
            scalar: None,
            model_uri: "urn:qac:mode:0x0000:v1",
            sample_rate: Some(sample_rate),
            valid_record_count: valid_count,
            validity_mask_entry: mask_entry_idx,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;

    let mut entries = vec![OwnedEntry {
        mode: MODE_PRESSURE_VELOCITY,
        flags: 3, // critical + temporal
        stream_id: 1,
        frame_id: 2,
        start_tick: 0,
        time_count: 1,
        spatial_key: 0,
        record_count: records.len() as u64,
        stride: PV_RECORD_STRIDE,
        descriptor_id: 6,
        payload,
    }];
    if let Some(mask) = validity_mask {
        let mask_entry = push_validity_mask_entry(&mut meta, 7, 3, 1, 0, 1, mask)?;
        entries.push(mask_entry);
    }
    assemble_qac_v1(&meta, &entries, limits)
}

/// Decoded Q03 `mode 0x0000` summary + zero-heap chunk view.
pub struct PressureVelocityView<'a> {
    pub records: PressureVelocityChunkView<'a>,
    pub medium: FluidMediumModel,
    pub staggering: StaggeringOffsetsPV,
}

pub fn read_pressure_velocity_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<PressureVelocityView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let (_entry, desc, payload, mask) = env.check_structured_entry(
        entry_index,
        MODE_PRESSURE_VELOCITY,
        PV_RECORD_STRIDE,
        true,
        Some(205),
    )?;
    let med_id = field_u32(field_req(desc, 4)?)?;
    let med_blk = env.block(med_id)?;
    let model = field_str(field_req(med_blk, 1)?)?;
    let medium = match model {
        "fluid-linear" => FluidMediumModel::FluidLinear {
            density_kg_m3: field_f64(field_req(med_blk, 2)?)?,
            bulk_modulus_pa: field_f64(field_req(med_blk, 3)?)?,
            temperature_k: field_opt(med_blk, 5).map(field_f64).transpose()?,
            ambient_pressure_pa: field_opt(med_blk, 6).map(field_f64).transpose()?,
            flow_m_s: match field_opt(med_blk, 7) {
                Some(f) => {
                    let v = field_f64_vec(f)?;
                    if v.len() != 3 {
                        return Err(ProfileError::InvalidMetadata("flow_m_s must have 3 components"));
                    }
                    Some([v[0], v[1], v[2]])
                }
                None => None,
            },
            matter_digest: field_opt(med_blk, 8).map(field_bytes32).transpose()?,
            material_table_digest: field_opt(med_blk, 9).map(field_bytes32).transpose()?,
            material_table_epoch: field_opt(med_blk, 10).map(field_u64).transpose()?,
            material_ids: field_opt(med_blk, 11).map(field_u64_vec).transpose()?.unwrap_or_default(),
        },
        "vacuum-mask" => FluidMediumModel::VacuumMask {
            mask_entry_index: field_u32(field_req(med_blk, 13)?)?,
        },
        "unknown-medium" => FluidMediumModel::UnknownMedium,
        other => FluidMediumModel::ExtendedConstitutive {
            model_uri: other.to_owned(),
            viscosity_pa_s: None,
            dispersion_model_uri: None,
            anisotropy_uri: None,
            poroelastic_uri: None,
            material_ids: field_opt(med_blk, 11).map(field_u64_vec).transpose()?.unwrap_or_default(),
        },
    };
    let grid = env
        .extension_block(desc, 205)?
        .ok_or(ProfileError::MissingRequiredSchema(205))?;
    let mut staggering = StaggeringOffsetsPV::collocated();
    if let Some(f) = field_opt(grid, 4) {
        let vals = field_f64_vec(f)?;
        if vals.len() == 12 {
            staggering.component_offsets_xyz.copy_from_slice(&vals);
        }
    }
    if let (Some(fn_), Some(fd)) = (field_opt(grid, 5), field_opt(grid, 16)) {
        let nums = field_i64_vec(fn_)?;
        let dens = field_u64_vec(fd)?;
        if nums.len() == 4 && dens.len() == 4 {
            staggering.time_offset_num.copy_from_slice(&nums);
            staggering.time_offset_den.copy_from_slice(&dens);
        }
    }
    let chunk = PressureVelocityChunkView {
        bytes: payload,
        mask,
    };
    // Validate every record in chunk up front so corrupt floats / masked bytes fail closed.
    for i in 0..chunk.len() {
        let _ = chunk.get(i)?;
    }
    Ok(PressureVelocityView {
        records: chunk,
        medium,
        staggering,
    })
}
