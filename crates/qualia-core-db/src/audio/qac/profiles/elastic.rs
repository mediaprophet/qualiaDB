//! Q03 Mode `0x0001`: Continuum full symmetric 6-component Cauchy stress tensor & velocity (`64 B`).
//!
//! Enforces tensile-positive Cauchy stress `[xx, yy, zz, xy, yz, xz]`, `p = -(xx+yy+zz)/3`,
//! Hookean tensor strain vs Voigt engineering shear `gamma = 2*eps`, active `R*sigma*R^T`
//! rotations, fluid/solid & free-surface interface fixtures, and explicit VacuumMask vs UnknownMedium.

use super::envelope::*;
use super::types::*;

pub const ELASTIC_RECORD_STRIDE: u32 = 64;

pub const ELASTIC_VALID_VELOCITY: u32 = 1 << 0;
pub const ELASTIC_VALID_STRESS: u32 = 1 << 1;
pub const ELASTIC_VALID_DISPLACEMENT: u32 = 1 << 2;
pub const ELASTIC_VALID_ALL: u32 =
    ELASTIC_VALID_VELOCITY | ELASTIC_VALID_STRESS | ELASTIC_VALID_DISPLACEMENT;

/// Symmetric 6-component Cauchy stress tensor in Pa (tensile-positive).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CauchyStress6 {
    pub xx: f64,
    pub yy: f64,
    pub zz: f64,
    pub xy: f64,
    pub yz: f64,
    pub xz: f64,
}

impl CauchyStress6 {
    pub const ZERO: Self = Self {
        xx: 0.0,
        yy: 0.0,
        zz: 0.0,
        xy: 0.0,
        yz: 0.0,
        xz: 0.0,
    };

    pub fn from_wire_f32(s: [f32; 6]) -> Self {
        Self {
            xx: f64::from(s[0]),
            yy: f64::from(s[1]),
            zz: f64::from(s[2]),
            xy: f64::from(s[3]),
            yz: f64::from(s[4]),
            xz: f64::from(s[5]),
        }
    }

    pub fn to_wire_f32(self) -> [f32; 6] {
        [
            self.xx as f32,
            self.yy as f32,
            self.zz as f32,
            self.xy as f32,
            self.yz as f32,
            self.xz as f32,
        ]
    }

    /// Mechanical pressure `p = -trace(sigma) / 3` under tensile-positive sign convention.
    pub fn hydrostatic_pressure_pa(self) -> f64 {
        -(self.xx + self.yy + self.zz) / 3.0
    }

    pub fn deviatoric(self) -> Self {
        let p = self.hydrostatic_pressure_pa();
        Self {
            xx: self.xx + p,
            yy: self.yy + p,
            zz: self.zz + p,
            xy: self.xy,
            yz: self.yz,
            xz: self.xz,
        }
    }

    /// Active frame rotation `R * sigma * R^T`.
    pub fn rotate(self, q: &QuaternionXyzw) -> Self {
        let out = q.rotate_symmetric_tensor6([self.xx, self.yy, self.zz, self.xy, self.yz, self.xz]);
        Self {
            xx: out[0],
            yy: out[1],
            zz: out[2],
            xy: out[3],
            yz: out[4],
            xz: out[5],
        }
    }

    /// Traction vector `t = sigma * n` acting on surface with outward unit normal `n`.
    pub fn traction_pa(self, n: [f64; 3]) -> [f64; 3] {
        [
            self.xx * n[0] + self.xy * n[1] + self.xz * n[2],
            self.xy * n[0] + self.yy * n[1] + self.yz * n[2],
            self.xz * n[0] + self.yz * n[1] + self.zz * n[2],
        ]
    }

    /// Isotropic Hooke constitutive law from symmetric tensor strain `[xx, yy, zz, xy, yz, xz]`.
    pub fn from_isotropic_tensor_strain(lambda: f64, mu: f64, eps: [f64; 6]) -> Self {
        let trace = eps[0] + eps[1] + eps[2];
        Self {
            xx: lambda * trace + 2.0 * mu * eps[0],
            yy: lambda * trace + 2.0 * mu * eps[1],
            zz: lambda * trace + 2.0 * mu * eps[2],
            xy: 2.0 * mu * eps[3],
            yz: 2.0 * mu * eps[4],
            xz: 2.0 * mu * eps[5],
        }
    }

    /// Isotropic Hooke constitutive law from Voigt engineering strain `[exx, eyy, ezz, gamma_yz, gamma_xz, gamma_xy]`.
    pub fn from_voigt_engineering_strain(lambda: f64, mu: f64, voigt: [f64; 6]) -> Self {
        let eps = [
            voigt[0],
            voigt[1],
            voigt[2],
            0.5 * voigt[5],
            0.5 * voigt[3],
            0.5 * voigt[4],
        ];
        Self::from_isotropic_tensor_strain(lambda, mu, eps)
    }
}

/// Exact 64-byte wire record for `mode 0x0001` (`elastic_stress_velocity_continuum`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElasticStressVelocityRecord {
    pub velocity_m_s: [f32; 3],
    pub material_ref: u32,
    /// Full symmetric Cauchy stress tensor `[xx, yy, zz, xy, yz, xz]` in Pa (tensile-positive).
    pub stress_xx_yy_zz_xy_yz_xz_pa: [f32; 6],
    pub displacement_m: [f32; 3],
    pub validity_bits: u32,
}

impl ElasticStressVelocityRecord {
    pub fn stress(&self) -> CauchyStress6 {
        CauchyStress6::from_wire_f32(self.stress_xx_yy_zz_xy_yz_xz_pa)
    }

    pub fn validate(&self, externally_valid: bool) -> Result<(), ProfileError> {
        if self.velocity_m_s.iter().any(|v| !v.is_finite())
            || self.stress_xx_yy_zz_xy_yz_xz_pa.iter().any(|v| !v.is_finite())
            || self.displacement_m.iter().any(|v| !v.is_finite())
        {
            return Err(ProfileError::NonFiniteFloat);
        }
        if self.validity_bits & !ELASTIC_VALID_ALL != 0 {
            return Err(ProfileError::InvalidValidityBits);
        }
        if !externally_valid {
            if self.validity_bits != 0
                || self.velocity_m_s.iter().any(|v| v.to_bits() != 0)
                || self.stress_xx_yy_zz_xy_yz_xz_pa.iter().any(|v| v.to_bits() != 0)
                || self.displacement_m.iter().any(|v| v.to_bits() != 0)
            {
                return Err(ProfileError::MaskedLaneNonZero);
            }
            return Ok(());
        }
        require_zero_vec3_when_masked((self.validity_bits & ELASTIC_VALID_VELOCITY) != 0, self.velocity_m_s)?;
        let s_valid = (self.validity_bits & ELASTIC_VALID_STRESS) != 0;
        for c in self.stress_xx_yy_zz_xy_yz_xz_pa {
            require_zero_when_masked(s_valid, c)?;
        }
        require_zero_vec3_when_masked(
            (self.validity_bits & ELASTIC_VALID_DISPLACEMENT) != 0,
            self.displacement_m,
        )?;
        Ok(())
    }

    pub fn decode(bytes: &[u8], externally_valid: bool) -> Result<Self, ProfileError> {
        if bytes.len() != ELASTIC_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        ensure_zero_bytes(&bytes[56..64])?;
        let rec = Self {
            velocity_m_s: read_f32_3(bytes, 0)?,
            material_ref: read_u32(bytes, 12)?,
            stress_xx_yy_zz_xy_yz_xz_pa: read_f32_6(bytes, 16)?,
            displacement_m: read_f32_3(bytes, 40)?,
            validity_bits: read_u32(bytes, 52)?,
        };
        rec.validate(externally_valid)?;
        Ok(rec)
    }

    pub fn encode_into(&self, out: &mut [u8], externally_valid: bool) -> Result<(), ProfileError> {
        if out.len() != ELASTIC_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        self.validate(externally_valid)?;
        out.fill(0);
        write_f32_slice(out, 0, &self.velocity_m_s)?;
        write_u32(out, 12, self.material_ref);
        write_f32_slice(out, 16, &self.stress_xx_yy_zz_xy_yz_xz_pa)?;
        write_f32_slice(out, 40, &self.displacement_m)?;
        write_u32(out, 52, self.validity_bits);
        Ok(())
    }
}

/// Zero-heap borrowed view over verified `0x0001` elastic stress/velocity records.
pub struct ElasticStressChunkView<'a> {
    bytes: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> ElasticStressChunkView<'a> {
    pub fn len(&self) -> usize {
        self.bytes.len() / ELASTIC_RECORD_STRIDE as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<ElasticStressVelocityRecord, ProfileError> {
        let start = index
            .checked_mul(ELASTIC_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let end = start
            .checked_add(ELASTIC_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("elastic record index"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        ElasticStressVelocityRecord::decode(slice, ext_valid)
    }
}

/// Elastic medium constitutive specification (`Schema 200`).
#[derive(Debug, Clone, PartialEq)]
pub enum ElasticMediumSpec {
    Isotropic {
        density_kg_m3: f64,
        bulk_modulus_pa: f64,
        shear_modulus_pa: f64,
        matter_digest: Option<[u8; 32]>,
        material_ids: Vec<u64>,
    },
    Anisotropic {
        model_uri: String,
        material_ids: Vec<u64>,
    },
    Poroelastic {
        model_uri: String,
        material_ids: Vec<u64>,
    },
    VacuumMask {
        mask_entry_index: u32,
    },
    UnknownMedium,
}

impl ElasticMediumSpec {
    pub fn steel() -> Self {
        Self::Isotropic {
            density_kg_m3: 7850.0,
            bulk_modulus_pa: 160.0e9,
            shear_modulus_pa: 80.0e9,
            matter_digest: None,
            material_ids: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), ProfileError> {
        match self {
            Self::Isotropic {
                density_kg_m3,
                bulk_modulus_pa,
                shear_modulus_pa,
                ..
            } => {
                if !density_kg_m3.is_finite() || *density_kg_m3 <= 0.0 {
                    return Err(ProfileError::MediumMissingOrUnknown("density must be > 0"));
                }
                if !bulk_modulus_pa.is_finite() || *bulk_modulus_pa <= 0.0 {
                    return Err(ProfileError::MediumMissingOrUnknown("bulk_modulus must be > 0"));
                }
                if !shear_modulus_pa.is_finite() || *shear_modulus_pa < 0.0 {
                    return Err(ProfileError::MediumMissingOrUnknown("shear_modulus must be >= 0"));
                }
                Ok(())
            }
            Self::Anisotropic { model_uri, .. } | Self::Poroelastic { model_uri, .. } => {
                validate_identifier_str(model_uri, "elastic.model_uri")
            }
            Self::VacuumMask { mask_entry_index } => {
                if *mask_entry_index == 0 {
                    return Err(ProfileError::MediumMissingOrUnknown("vacuum mask entry must be >= 1"));
                }
                Ok(())
            }
            Self::UnknownMedium => Ok(()),
        }
    }

    /// Longitudinal P-wave and transverse S-wave speeds `(c_p, c_s)` in m/s.
    pub fn wave_speeds_m_s(&self) -> Result<(f64, f64), ProfileError> {
        match self {
            Self::Isotropic {
                density_kg_m3,
                bulk_modulus_pa,
                shear_modulus_pa,
                ..
            } => {
                self.validate()?;
                let p_mod = bulk_modulus_pa + (4.0 / 3.0) * shear_modulus_pa;
                Ok(((p_mod / density_kg_m3).sqrt(), (shear_modulus_pa / density_kg_m3).sqrt()))
            }
            Self::Anisotropic { .. } | Self::Poroelastic { .. } => {
                Err(ProfileError::UnsupportedConstitutiveModel(
                    "isotropic (c_p, c_s) undefined for anisotropic/poroelastic medium without directional tensor profile",
                ))
            }
            _ => Err(ProfileError::MediumMissingOrUnknown(
                "wave speeds undefined in VacuumMask or UnknownMedium",
            )),
        }
    }

    pub fn lame_lambda_mu(&self) -> Result<(f64, f64), ProfileError> {
        match self {
            Self::Isotropic {
                bulk_modulus_pa,
                shear_modulus_pa,
                ..
            } => {
                self.validate()?;
                Ok((bulk_modulus_pa - (2.0 / 3.0) * shear_modulus_pa, *shear_modulus_pa))
            }
            _ => Err(ProfileError::UnsupportedConstitutiveModel("requires Isotropic elastic medium")),
        }
    }

    fn push_schema_200(&self, meta: &mut MetadataBuilder, id: u32) -> Result<u32, ProfileError> {
        self.validate()?;
        let mut fields = Vec::new();
        match self {
            Self::Isotropic {
                density_kg_m3,
                bulk_modulus_pa,
                shear_modulus_pa,
                matter_digest,
                material_ids,
            } => {
                fields.push(OwnedField::crit(
                    1,
                    OwnedValue::Utf8("elastic-isotropic".to_owned()),
                ));
                fields.push(OwnedField::crit(2, OwnedValue::F64(vec![*density_kg_m3])));
                fields.push(OwnedField::crit(3, OwnedValue::F64(vec![*bulk_modulus_pa])));
                fields.push(OwnedField::crit(4, OwnedValue::F64(vec![*shear_modulus_pa])));
                if let Some(d) = matter_digest {
                    fields.push(OwnedField::opt(8, OwnedValue::Bytes(d.to_vec())));
                }
                if !material_ids.is_empty() {
                    fields.push(OwnedField::opt(11, OwnedValue::U64(material_ids.clone())));
                }
            }
            Self::Anisotropic {
                model_uri,
                material_ids,
            }
            | Self::Poroelastic {
                model_uri,
                material_ids,
            } => {
                fields.push(OwnedField::crit(1, OwnedValue::Utf8(model_uri.clone())));
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

/// Normal-incidence planar P-wave pressure/stress reflection & transmission `(R, T)` between
/// medium 1 (`z1 = rho1 * c_p1`) and medium 2 (`z2 = rho2 * c_p2`), satisfying energy balance.
pub fn normal_incidence_p_reflection_transmission(z1: f64, z2: f64) -> Result<(f64, f64), ProfileError> {
    if !z1.is_finite() || !z2.is_finite() || z1 <= 0.0 || z2 <= 0.0 {
        return Err(ProfileError::MediumMissingOrUnknown("impedances must be finite > 0"));
    }
    let r = (z2 - z1) / (z2 + z1);
    let t = (2.0 * z2) / (z1 + z2);
    Ok((r, t))
}

/// Writes canonical QAC v1 `mode 0x0001` elastic continuum container.
pub fn write_elastic_stress_qac(
    records: &[ElasticStressVelocityRecord],
    medium: &ElasticMediumSpec,
    component_offsets_xyz: Option<&[f64]>,
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
    if matches!(medium, ElasticMediumSpec::VacuumMask { .. }) && validity_mask.is_none() {
        return Err(ProfileError::MediumMissingOrUnknown(
            "VacuumMask requires an accompanying validity_mask entry",
        ));
    }
    if let Some(m) = validity_mask {
        if m.len() != records.len() || m.iter().any(|&b| b > 1) {
            return Err(ProfileError::InvalidValidityBits);
        }
    }
    let mut payload = vec![0u8; records.len() * ELASTIC_RECORD_STRIDE as usize];
    for (idx, rec) in records.iter().enumerate() {
        let ext_valid = validity_mask.map_or(true, |m| m[idx] != 0);
        let off = idx * ELASTIC_RECORD_STRIDE as usize;
        rec.encode_into(&mut payload[off..off + ELASTIC_RECORD_STRIDE as usize], ext_valid)?;
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
            model_uri: "urn:qac:model:elastic-stress-velocity:v1",
            model_version: "1",
            discretization: Some("virieux-staggered-grid"),
            component_offsets_xyz: component_offsets_xyz.unwrap_or(&[]),
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: Some(sample_rate),
            tolerance_abs: Some(1e-5),
            tolerance_rel: Some(1e-4),
            tensor_shape: &shape,
            component_names: "vx,vy,vz,sxx,syy,szz,sxy,syz,sxz",
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
            quantity: "elastic-stress-velocity",
            units: "mixed:mode-registry",
            channels: None,
            layout: None,
            scalar: None,
            model_uri: "urn:qac:mode:0x0001:v1",
            sample_rate: Some(sample_rate),
            valid_record_count: valid_count,
            validity_mask_entry: mask_entry_idx,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;

    let mut entries = vec![OwnedEntry {
        mode: MODE_ELASTIC_STRESS_VELOCITY,
        flags: 3,
        stream_id: 1,
        frame_id: 2,
        start_tick: 0,
        time_count: 1,
        spatial_key: 0,
        record_count: records.len() as u64,
        stride: ELASTIC_RECORD_STRIDE,
        descriptor_id: 6,
        payload,
    }];
    if let Some(mask) = validity_mask {
        entries.push(push_validity_mask_entry(&mut meta, 7, 3, 1, 0, 1, mask)?);
    }
    assemble_qac_v1(&meta, &entries, limits)
}

pub struct ElasticStressView<'a> {
    pub records: ElasticStressChunkView<'a>,
    pub medium: ElasticMediumSpec,
}

pub fn read_elastic_stress_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<ElasticStressView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let (_e, desc, payload, mask) = env.check_structured_entry(
        entry_index,
        MODE_ELASTIC_STRESS_VELOCITY,
        ELASTIC_RECORD_STRIDE,
        true,
        Some(205),
    )?;
    let med_id = field_u32(field_req(desc, 4)?)?;
    let med_blk = env.block(med_id)?;
    let model = field_str(field_req(med_blk, 1)?)?;
    let medium = match model {
        "elastic-isotropic" => ElasticMediumSpec::Isotropic {
            density_kg_m3: field_f64(field_req(med_blk, 2)?)?,
            bulk_modulus_pa: field_f64(field_req(med_blk, 3)?)?,
            shear_modulus_pa: field_f64(field_req(med_blk, 4)?)?,
            matter_digest: field_opt(med_blk, 8).map(field_bytes32).transpose()?,
            material_ids: field_opt(med_blk, 11).map(field_u64_vec).transpose()?.unwrap_or_default(),
        },
        "vacuum-mask" => ElasticMediumSpec::VacuumMask {
            mask_entry_index: field_u32(field_req(med_blk, 13)?)?,
        },
        "unknown-medium" => ElasticMediumSpec::UnknownMedium,
        other if other.contains("poro") => ElasticMediumSpec::Poroelastic {
            model_uri: other.to_owned(),
            material_ids: field_opt(med_blk, 11).map(field_u64_vec).transpose()?.unwrap_or_default(),
        },
        other => ElasticMediumSpec::Anisotropic {
            model_uri: other.to_owned(),
            material_ids: field_opt(med_blk, 11).map(field_u64_vec).transpose()?.unwrap_or_default(),
        },
    };
    let chunk = ElasticStressChunkView {
        bytes: payload,
        mask,
    };
    for i in 0..chunk.len() {
        let _ = chunk.get(i)?;
    }
    Ok(ElasticStressView {
        records: chunk,
        medium,
    })
}
