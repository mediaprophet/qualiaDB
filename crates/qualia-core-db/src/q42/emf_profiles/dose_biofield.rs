//! Packet E06: Ionizing radiation dose/transport (mode 0x0106 / ionizing_radiation_v1)
//! and bioelectric/biofield source records (mode 0x0107 / bioelectric_dipole_v1), enforcing
//! strict transport species distinction, epistemic isolation and provenance/uncertainty rules.
//! Note: Storage/measurement records never emit universal physiological or clinical safety verdicts.

use super::schemas::{evaluate_calibration, i64_at, EpistemicClass};
use super::spectrum::ELECTRON_VOLT_JOULE;
use super::*;
use crate::container_10d::crc32c::crc32c;
use core::f64::consts::PI;

pub const DOSE_PROFILE: &str = "ionizing_radiation_v1";
pub const DOSE_MODEL_URI: &str = "urn:qualia:emf:ionizing_radiation_v1";
pub const BIOFIELD_PROFILE: &str = "bioelectric_dipole_v1";
pub const BIOFIELD_MODEL_URI: &str = "urn:qualia:emf:bioelectric_dipole_v1";

pub const DOSE_VALID_PHOTON_ENERGY: u32 = 1 << 0;
pub const DOSE_VALID_LINEAR_ATTEN: u32 = 1 << 1;
pub const DOSE_VALID_MASS_ATTEN: u32 = 1 << 2;
pub const DOSE_VALID_FLUENCE: u32 = 1 << 3;
pub const DOSE_VALID_ABSORBED_GY: u32 = 1 << 4;
pub const DOSE_VALID_EQUIVALENT_SV: u32 = 1 << 5;
pub const DOSE_VALID_LET: u32 = 1 << 6;
pub const DOSE_VALID_UNCERTAINTY: u32 = 1 << 7;
pub const DOSE_ALL_VALID_MASK: u32 = 0xFF;

pub const BIO_FLAG_POTENTIAL: u32 = 1 << 0;
pub const BIO_FLAG_CARRIER: u32 = 1 << 1;
pub const BIO_FLAG_UNCERTAINTY: u32 = 1 << 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportSpecies {
    Photon,
    Electron,
    Positron,
    Proton,
    Alpha,
    Neutron,
    HeavyIon,
}

impl TransportSpecies {
    pub fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "photon" | "gamma" | "x-ray" | "xray" => Ok(Self::Photon),
            "electron" | "e-" => Ok(Self::Electron),
            "positron" | "e+" => Ok(Self::Positron),
            "proton" | "p+" | "H1" => Ok(Self::Proton),
            "alpha" | "He4" => Ok(Self::Alpha),
            "neutron" | "n0" => Ok(Self::Neutron),
            "heavy_ion" => Ok(Self::HeavyIon),
            _ => Err(Error::UnsupportedProfile),
        }
    }

    pub const fn is_uncharged_photon(self) -> bool {
        matches!(self, Self::Photon)
    }

    pub const fn is_charged_particle(self) -> bool {
        matches!(
            self,
            Self::Electron | Self::Positron | Self::Proton | Self::Alpha | Self::HeavyIon
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DoseTransportRecord {
    pub position_m: [f64; 3],
    pub tick: i64,
    pub photon_energy_j: Option<f64>,
    pub linear_attenuation_per_m: Option<f64>,
    pub mass_attenuation_m2_per_kg: Option<f64>,
    pub photon_fluence_per_m2: Option<f64>,
    pub absorbed_dose_gy: Option<f64>,
    pub equivalent_dose_sv: Option<f64>,
    pub linear_energy_transfer_j_per_m: Option<f64>,
    pub uncertainty: Option<f64>,
    pub material_entry: u32,
    pub calibration_ref: u32,
    pub model_ref: u32,
    pub validity_flags: u32,
}

fn opt_lane(bytes: &[u8], off: usize, present: bool, positive_only: bool) -> Result<Option<f64>, Error> {
    if !present {
        zeros(&bytes[off..off + 8])?;
        return Ok(None);
    }
    let val = f64_at(bytes, off);
    if !val.is_finite() {
        return Err(Error::NonFinite);
    }
    if val < 0.0 || (positive_only && val <= 0.0) {
        return Err(Error::Value);
    }
    Ok(Some(val))
}

impl DoseTransportRecord {
    pub fn parse(bytes: &[u8], valid: bool) -> Result<Option<Self>, Error> {
        if bytes.len() != 128 {
            return Err(Error::Shape);
        }
        zeros(&bytes[112..124])?;
        if crc32c(&bytes[..124]) != u32_at(bytes, 124) {
            return Err(Error::Reserved);
        }
        if !valid {
            zeros(&bytes[..24])?;
            zeros(&bytes[32..112])?;
            return Ok(None);
        }
        finite(bytes, 0, 24)?;
        finite(bytes, 32, 96)?;
        let vflags = u32_at(bytes, 108);
        if vflags & !DOSE_ALL_VALID_MASK != 0 {
            return Err(Error::Flags);
        }
        let photon_energy_j = opt_lane(bytes, 32, vflags & DOSE_VALID_PHOTON_ENERGY != 0, true)?;
        let linear_attenuation_per_m =
            opt_lane(bytes, 40, vflags & DOSE_VALID_LINEAR_ATTEN != 0, false)?;
        let mass_attenuation_m2_per_kg =
            opt_lane(bytes, 48, vflags & DOSE_VALID_MASS_ATTEN != 0, false)?;
        let photon_fluence_per_m2 = opt_lane(bytes, 56, vflags & DOSE_VALID_FLUENCE != 0, false)?;
        let absorbed_dose_gy = opt_lane(bytes, 64, vflags & DOSE_VALID_ABSORBED_GY != 0, false)?;
        let equivalent_dose_sv =
            opt_lane(bytes, 72, vflags & DOSE_VALID_EQUIVALENT_SV != 0, false)?;
        let linear_energy_transfer_j_per_m =
            opt_lane(bytes, 80, vflags & DOSE_VALID_LET != 0, false)?;
        let uncertainty = opt_lane(bytes, 88, vflags & DOSE_VALID_UNCERTAINTY != 0, false)?;

        Ok(Some(Self {
            position_m: triple(bytes, 0),
            tick: i64_at(bytes, 24),
            photon_energy_j,
            linear_attenuation_per_m,
            mass_attenuation_m2_per_kg,
            photon_fluence_per_m2,
            absorbed_dose_gy,
            equivalent_dose_sv,
            linear_energy_transfer_j_per_m,
            uncertainty,
            material_entry: u32_at(bytes, 96),
            calibration_ref: u32_at(bytes, 100),
            model_ref: u32_at(bytes, 104),
            validity_flags: vflags,
        }))
    }

    pub fn encode(&self, valid: bool, out: &mut [u8; 128]) -> Result<(), Error> {
        *out = [0u8; 128];
        if valid {
            if self.validity_flags & !DOSE_ALL_VALID_MASK != 0 {
                return Err(Error::Flags);
            }
            if self.position_m.iter().any(|v| !v.is_finite()) {
                return Err(Error::NonFinite);
            }
            for (i, v) in self.position_m.iter().enumerate() {
                out[i * 8..i * 8 + 8].copy_from_slice(&v.to_le_bytes());
            }
            out[24..32].copy_from_slice(&self.tick.to_le_bytes());

            let lanes: [(Option<f64>, u32, usize, bool); 8] = [
                (self.photon_energy_j, DOSE_VALID_PHOTON_ENERGY, 32, true),
                (self.linear_attenuation_per_m, DOSE_VALID_LINEAR_ATTEN, 40, false),
                (self.mass_attenuation_m2_per_kg, DOSE_VALID_MASS_ATTEN, 48, false),
                (self.photon_fluence_per_m2, DOSE_VALID_FLUENCE, 56, false),
                (self.absorbed_dose_gy, DOSE_VALID_ABSORBED_GY, 64, false),
                (self.equivalent_dose_sv, DOSE_VALID_EQUIVALENT_SV, 72, false),
                (self.linear_energy_transfer_j_per_m, DOSE_VALID_LET, 80, false),
                (self.uncertainty, DOSE_VALID_UNCERTAINTY, 88, false),
            ];
            let mut computed_flags = 0u32;
            for (opt, bit, off, pos_only) in lanes {
                if let Some(v) = opt {
                    if !v.is_finite() {
                        return Err(Error::NonFinite);
                    }
                    if v < 0.0 || (pos_only && v <= 0.0) {
                        return Err(Error::Value);
                    }
                    if self.validity_flags & bit == 0 {
                        return Err(Error::Flags);
                    }
                    computed_flags |= bit;
                    out[off..off + 8].copy_from_slice(&v.to_le_bytes());
                } else if self.validity_flags & bit != 0 {
                    return Err(Error::Flags);
                }
            }
            out[96..100].copy_from_slice(&self.material_entry.to_le_bytes());
            out[100..104].copy_from_slice(&self.calibration_ref.to_le_bytes());
            out[104..108].copy_from_slice(&self.model_ref.to_le_bytes());
            out[108..112].copy_from_slice(&computed_flags.to_le_bytes());
        } else {
            out[24..32].copy_from_slice(&self.tick.to_le_bytes());
        }
        let c = crc32c(&out[..124]);
        out[124..128].copy_from_slice(&c.to_le_bytes());
        Ok(())
    }

    /// Validates dose record bindings against schema 107 Calibration and schema 134 ModelEvidence.
    pub fn validate_against_blocks(
        &self,
        cal: Block<'_>,
        model: Block<'_>,
    ) -> Result<(TransportSpecies, EpistemicClass), Error> {
        if cal.schema_type() != 107 || model.schema_type() != 134 {
            return Err(Error::Shared(semantics::Error::ReferenceSchema));
        }
        if self.calibration_ref != cal.id() || self.model_ref != model.id() {
            return Err(Error::Descriptor);
        }
        let species = TransportSpecies::parse(text(required(model, 9)?)?)?;
        let epistemic = EpistemicClass::parse(text(required(model, 8)?)?)?;
        // Ensure tick falls inside model integration interval and calibration half-open validity.
        let int_ticks = required(model, 10)?.value();
        let t_start = i64_at(int_ticks, 0);
        let t_end = i64_at(int_ticks, 8);
        if self.tick < t_start || self.tick > t_end {
            return Err(Error::Time);
        }
        let _ = evaluate_calibration(cal, 1.0, Some(self.tick))?;
        if self.equivalent_dose_sv.is_some() && field(model, 11).is_none() {
            // Equivalent dose (Sv) requires explicit radiation/tissue weighting model; never alias Gy.
            return Err(Error::Descriptor);
        }
        if self.uncertainty.is_some() && field(model, 5).is_none() {
            return Err(Error::Descriptor);
        }
        if species.is_charged_particle()
            && (self.linear_attenuation_per_m.is_some() || self.mass_attenuation_m2_per_kg.is_some())
        {
            // Beer-Lambert photon mass/linear attenuation does not apply directly to charged particles.
            return Err(Error::UnsupportedProfile);
        }
        Ok((species, epistemic))
    }
}

pub fn encode_dose_records(
    records: &[DoseTransportRecord],
    valid: Option<&[bool]>,
    out: &mut [u8],
) -> Result<usize, Error> {
    if let Some(v) = valid {
        if v.len() != records.len() {
            return Err(Error::Shape);
        }
    }
    let needed = records.len().checked_mul(128).ok_or(Error::Overflow)?;
    if out.len() < needed {
        return Err(Error::Shape);
    }
    for (idx, rec) in records.iter().enumerate() {
        let is_valid = valid.is_none_or(|v| v[idx]);
        let dst: &mut [u8; 128] = (&mut out[idx * 128..idx * 128 + 128])
            .try_into()
            .map_err(|_| Error::Shape)?;
        rec.encode(is_valid, dst)?;
    }
    Ok(needed)
}

#[inline]
pub fn kev_to_joules(energy_kev: f64) -> Result<f64, Error> {
    if !energy_kev.is_finite() {
        return Err(Error::NonFinite);
    }
    if energy_kev <= 0.0 {
        return Err(Error::Value);
    }
    Ok(energy_kev * 1.0e3 * ELECTRON_VOLT_JOULE)
}

#[inline]
pub fn linear_from_mass_attenuation(
    mass_attenuation_m2_per_kg: f64,
    density_kg_per_m3: f64,
) -> Result<f64, Error> {
    if !mass_attenuation_m2_per_kg.is_finite() || !density_kg_per_m3.is_finite() {
        return Err(Error::NonFinite);
    }
    if mass_attenuation_m2_per_kg < 0.0 || density_kg_per_m3 <= 0.0 {
        return Err(Error::Value);
    }
    Ok(mass_attenuation_m2_per_kg * density_kg_per_m3)
}

/// Uncollided Beer-Lambert photon attenuation `Phi_out = Phi_in * exp(-mu * x)`.
/// Rejects charged particles where stopping-power / CSDA transport applies instead.
pub fn photon_uncollided_fluence(
    species: TransportSpecies,
    incident_fluence_per_m2: f64,
    linear_attenuation_per_m: f64,
    thickness_m: f64,
) -> Result<f64, Error> {
    if !species.is_uncharged_photon() {
        return Err(Error::UnsupportedProfile);
    }
    if !incident_fluence_per_m2.is_finite()
        || !linear_attenuation_per_m.is_finite()
        || !thickness_m.is_finite()
    {
        return Err(Error::NonFinite);
    }
    if incident_fluence_per_m2 < 0.0 || linear_attenuation_per_m < 0.0 || thickness_m < 0.0 {
        return Err(Error::Value);
    }
    Ok(incident_fluence_per_m2 * (-linear_attenuation_per_m * thickness_m).exp())
}

#[inline]
pub fn absorbed_dose_gy(deposited_energy_j: f64, target_mass_kg: f64) -> Result<f64, Error> {
    if !deposited_energy_j.is_finite() || !target_mass_kg.is_finite() {
        return Err(Error::NonFinite);
    }
    if deposited_energy_j < 0.0 || target_mass_kg <= 0.0 {
        return Err(Error::Value);
    }
    Ok(deposited_energy_j / target_mass_kg)
}

#[inline]
pub fn equivalent_dose_sv(absorbed_gy: f64, radiation_weighting_factor: f64) -> Result<f64, Error> {
    if !absorbed_gy.is_finite() || !radiation_weighting_factor.is_finite() {
        return Err(Error::NonFinite);
    }
    if absorbed_gy < 0.0 || radiation_weighting_factor <= 0.0 {
        return Err(Error::Value);
    }
    Ok(absorbed_gy * radiation_weighting_factor)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiofieldRecord {
    pub position_m: [f64; 3],
    pub current_dipole_moment_a_m: [f64; 3],
    pub extracellular_potential_v: Option<f64>,
    pub tick: i64,
    pub carrier_hz: Option<f64>,
    pub uncertainty: Option<f64>,
    pub anatomy_metadata_ref: u32,
    pub model_ref: u32,
    pub calibration_ref: u32,
    pub consent_provenance_ref: u32,
    pub flags: u32,
}

impl BiofieldRecord {
    pub fn parse(bytes: &[u8], valid: bool) -> Result<Option<Self>, Error> {
        if bytes.len() != 128 {
            return Err(Error::Shape);
        }
        zeros(&bytes[100..124])?;
        if crc32c(&bytes[..124]) != u32_at(bytes, 124) {
            return Err(Error::Reserved);
        }
        if !valid {
            zeros(&bytes[..56])?;
            zeros(&bytes[64..100])?;
            return Ok(None);
        }
        finite(bytes, 0, 56)?;
        finite(bytes, 64, 80)?;
        let flags = u32_at(bytes, 96);
        if flags & !(BIO_FLAG_POTENTIAL | BIO_FLAG_CARRIER | BIO_FLAG_UNCERTAINTY) != 0 {
            return Err(Error::Flags);
        }
        let extracellular_potential_v = if flags & BIO_FLAG_POTENTIAL != 0 {
            Some(f64_at(bytes, 48))
        } else {
            zeros(&bytes[48..56])?;
            None
        };
        let carrier_hz = opt_lane(bytes, 64, flags & BIO_FLAG_CARRIER != 0, true)?;
        let uncertainty = opt_lane(bytes, 72, flags & BIO_FLAG_UNCERTAINTY != 0, false)?;

        Ok(Some(Self {
            position_m: triple(bytes, 0),
            current_dipole_moment_a_m: triple(bytes, 24),
            extracellular_potential_v,
            tick: i64_at(bytes, 56),
            carrier_hz,
            uncertainty,
            anatomy_metadata_ref: u32_at(bytes, 80),
            model_ref: u32_at(bytes, 84),
            calibration_ref: u32_at(bytes, 88),
            consent_provenance_ref: u32_at(bytes, 92),
            flags,
        }))
    }

    pub fn encode(&self, valid: bool, out: &mut [u8; 128]) -> Result<(), Error> {
        *out = [0u8; 128];
        if valid {
            if self.flags & !(BIO_FLAG_POTENTIAL | BIO_FLAG_CARRIER | BIO_FLAG_UNCERTAINTY) != 0 {
                return Err(Error::Flags);
            }
            if self.position_m.iter().any(|v| !v.is_finite())
                || self.current_dipole_moment_a_m.iter().any(|v| !v.is_finite())
            {
                return Err(Error::NonFinite);
            }
            for i in 0..3 {
                out[i * 8..i * 8 + 8].copy_from_slice(&self.position_m[i].to_le_bytes());
                out[24 + i * 8..32 + i * 8]
                    .copy_from_slice(&self.current_dipole_moment_a_m[i].to_le_bytes());
            }
            if let Some(phi) = self.extracellular_potential_v {
                if !phi.is_finite() {
                    return Err(Error::NonFinite);
                }
                if self.flags & BIO_FLAG_POTENTIAL == 0 {
                    return Err(Error::Flags);
                }
                out[48..56].copy_from_slice(&phi.to_le_bytes());
            } else if self.flags & BIO_FLAG_POTENTIAL != 0 {
                return Err(Error::Flags);
            }
            out[56..64].copy_from_slice(&self.tick.to_le_bytes());
            if let Some(f) = self.carrier_hz {
                if !f.is_finite() {
                    return Err(Error::NonFinite);
                }
                if f <= 0.0 {
                    return Err(Error::Value);
                }
                if self.flags & BIO_FLAG_CARRIER == 0 {
                    return Err(Error::Flags);
                }
                out[64..72].copy_from_slice(&f.to_le_bytes());
            } else if self.flags & BIO_FLAG_CARRIER != 0 {
                return Err(Error::Flags);
            }
            if let Some(u) = self.uncertainty {
                if !u.is_finite() {
                    return Err(Error::NonFinite);
                }
                if u < 0.0 {
                    return Err(Error::Value);
                }
                if self.flags & BIO_FLAG_UNCERTAINTY == 0 {
                    return Err(Error::Flags);
                }
                out[72..80].copy_from_slice(&u.to_le_bytes());
            } else if self.flags & BIO_FLAG_UNCERTAINTY != 0 {
                return Err(Error::Flags);
            }
            out[80..84].copy_from_slice(&self.anatomy_metadata_ref.to_le_bytes());
            out[84..88].copy_from_slice(&self.model_ref.to_le_bytes());
            out[88..92].copy_from_slice(&self.calibration_ref.to_le_bytes());
            out[92..96].copy_from_slice(&self.consent_provenance_ref.to_le_bytes());
            out[96..100].copy_from_slice(&self.flags.to_le_bytes());
        } else {
            out[56..64].copy_from_slice(&self.tick.to_le_bytes());
        }
        let c = crc32c(&out[..124]);
        out[124..128].copy_from_slice(&c.to_le_bytes());
        Ok(())
    }

    pub fn validate_against_blocks(
        &self,
        anatomy: Block<'_>,
        model: Block<'_>,
        cal: Block<'_>,
        consent_prov: Block<'_>,
        require_consent_receipt: bool,
    ) -> Result<EpistemicClass, Error> {
        if anatomy.schema_type() != 137
            || model.schema_type() != 134
            || cal.schema_type() != 107
            || consent_prov.schema_type() != 4
        {
            return Err(Error::Shared(semantics::Error::ReferenceSchema));
        }
        if self.anatomy_metadata_ref != anatomy.id()
            || self.model_ref != model.id()
            || self.calibration_ref != cal.id()
            || self.consent_provenance_ref != consent_prov.id()
        {
            return Err(Error::Descriptor);
        }
        if self.uncertainty.is_some() && field(model, 5).is_none() {
            return Err(Error::Descriptor);
        }
        let _ = evaluate_calibration(cal, 1.0, Some(self.tick))?;
        if require_consent_receipt {
            let receipt = field(consent_prov, 6).ok_or(Error::Descriptor)?;
            if receipt.value().len() != 32 || receipt.value().iter().all(|&b| b == 0) {
                return Err(Error::Descriptor);
            }
        }
        EpistemicClass::parse(text(required(model, 8)?)?)
    }
}

pub fn encode_biofield_records(
    records: &[BiofieldRecord],
    valid: Option<&[bool]>,
    out: &mut [u8],
) -> Result<usize, Error> {
    if let Some(v) = valid {
        if v.len() != records.len() {
            return Err(Error::Shape);
        }
    }
    let needed = records.len().checked_mul(128).ok_or(Error::Overflow)?;
    if out.len() < needed {
        return Err(Error::Shape);
    }
    for (idx, rec) in records.iter().enumerate() {
        let is_valid = valid.is_none_or(|v| v[idx]);
        let dst: &mut [u8; 128] = (&mut out[idx * 128..idx * 128 + 128])
            .try_into()
            .map_err(|_| Error::Shape)?;
        rec.encode(is_valid, dst)?;
    }
    Ok(needed)
}

/// Infinite homogeneous quasi-static point current dipole potential:
/// `phi(r_obs) = (q . (r_obs - r_src)) / (4*pi * sigma * |r_obs - r_src|^3)`.
/// Strictly fails closed at the singularity (`|r_obs - r_src| <= 1e-12`).
pub fn dipole_potential_infinite_homogeneous(
    source_pos_m: [f64; 3],
    moment_a_m: [f64; 3],
    obs_pos_m: [f64; 3],
    conductivity_s_per_m: f64,
) -> Result<f64, Error> {
    if source_pos_m.iter().any(|v| !v.is_finite())
        || moment_a_m.iter().any(|v| !v.is_finite())
        || obs_pos_m.iter().any(|v| !v.is_finite())
        || !conductivity_s_per_m.is_finite()
    {
        return Err(Error::NonFinite);
    }
    if conductivity_s_per_m <= 0.0 {
        return Err(Error::Value);
    }
    let dr = [
        obs_pos_m[0] - source_pos_m[0],
        obs_pos_m[1] - source_pos_m[1],
        obs_pos_m[2] - source_pos_m[2],
    ];
    let r = ((dr[0] * dr[0] + dr[1] * dr[1]) + dr[2] * dr[2]).sqrt();
    if r <= 1.0e-12 {
        return Err(Error::Value);
    }
    let q_dot_r = moment_a_m[0] * dr[0] + moment_a_m[1] * dr[1] + moment_a_m[2] * dr[2];
    Ok(q_dot_r / (4.0 * PI * conductivity_s_per_m * r * r * r))
}

/// Prevents speculative models (`hypothesis`, `counterfactual`, `fictional`) from silently
/// joining or contaminating `empirical` biofield/radiation evaluations.
pub fn verify_empirical_join_allowed(model_block: Block<'_>) -> Result<EpistemicClass, Error> {
    if model_block.schema_type() != 134 || model_block.revision() != 1 {
        return Err(Error::Shared(semantics::Error::ReferenceSchema));
    }
    let class = EpistemicClass::parse(text(required(model_block, 8)?)?)?;
    if class.is_speculative() {
        return Err(Error::UnsupportedProfile);
    }
    Ok(class)
}
