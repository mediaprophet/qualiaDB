//! Zero-heap QAC v1 structural admission helpers, field extractors, and re-exported builder APIs.

use crate::q42::{
    field_envelope::{Block, Entry, Family, Field, StructuralView},
    field_semantics as sem,
};

pub use super::builder::*;
use super::registry::QacProfileRegistry;
use super::types::*;

// ---------------------------------------------------------------------------
// Zero-heap admitted structural view & field extractors
// ---------------------------------------------------------------------------

/// Verified structural QAC v1 reader anchored to caller or bounded scratch.
pub struct AdmittedEnvelope<'a> {
    view: StructuralView<'a>,
    limits: ProfileLimits,
}

impl<'a> AdmittedEnvelope<'a> {
    /// Zero-heap admission into caller-owned scratch (`metadata` >= blocks, `entries` >= entry_count).
    pub fn open<'s>(
        bytes: &'a [u8],
        scratch: &'s mut sem::Scratch<'_>,
        limits: ProfileLimits,
    ) -> Result<Self, ProfileError> {
        let view = StructuralView::open(bytes, Family::QacV1Candidate, limits.envelope)?;
        let _ = sem::admit_shared(&view, &QacProfileRegistry, scratch, limits.semantics)?;
        Ok(Self { view, limits })
    }

    /// Cold bounded helper allocating `Vec<sem::NodeState>` capped by `limits`.
    pub fn open_alloc(bytes: &'a [u8], limits: ProfileLimits) -> Result<Self, ProfileError> {
        let view = StructuralView::open(bytes, Family::QacV1Candidate, limits.envelope)?;
        let block_count = view.blocks().count();
        let entry_count = view.entry_count() as usize;
        let mut m = vec![sem::NodeState::default(); block_count];
        let mut e = vec![sem::NodeState::default(); entry_count];
        let mut scratch = sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        };
        let _ = sem::admit_shared(&view, &QacProfileRegistry, &mut scratch, limits.semantics)?;
        Ok(Self { view, limits })
    }

    pub fn view(&self) -> &StructuralView<'a> {
        &self.view
    }

    pub fn entry(&self, index: u32) -> Result<Entry<'a>, ProfileError> {
        Ok(self.view.entry(index)?)
    }

    pub fn verify_chunk(&self, index: u32) -> Result<&'a [u8], ProfileError> {
        let e = self.view.entry(index)?;
        if e.record_count() > self.limits.max_records_per_chunk {
            return Err(ProfileError::LimitExceeded);
        }
        Ok(self.view.verify_chunk(index)?)
    }

    pub fn block(&self, id: u32) -> Result<Block<'a>, ProfileError> {
        self.view
            .blocks()
            .find(|b| b.id() == id)
            .ok_or(ProfileError::Semantics(sem::Error::Reference))
    }

    pub fn descriptor_for_entry(&self, entry: Entry<'_>) -> Result<Block<'a>, ProfileError> {
        let desc = self.block(entry.descriptor_id())?;
        if desc.schema_type() != 3 {
            return Err(ProfileError::Semantics(sem::Error::ReferenceSchema));
        }
        Ok(desc)
    }

    pub fn extension_block(
        &self,
        descriptor: Block<'_>,
        schema_type: u16,
    ) -> Result<Option<Block<'a>>, ProfileError> {
        let Some(f) = descriptor.fields().find(|f| f.key() == 18) else {
            return Ok(None);
        };
        for chunk in f.value().chunks_exact(4) {
            let id = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let blk = self.block(id)?;
            if blk.schema_type() == schema_type {
                return Ok(Some(blk));
            }
        }
        Ok(None)
    }

    pub fn find_entry_indices_by_mode(&self, mode: u16) -> Vec<u32> {
        let n = self.view.entry_count();
        let mut out = Vec::new();
        for idx in 1..=n {
            if let Ok(e) = self.view.entry(idx) {
                if e.mode() == mode {
                    out.push(idx);
                }
            }
        }
        out
    }

    /// Reads optional validity mask (`key 16` -> `mode 0x0106`) and verifies whole-chunk digest.
    pub fn read_validity_mask(&self, descriptor: Block<'_>, record_count: u64) -> Result<Option<&'a [u8]>, ProfileError> {
        let Some(vm_field) = field_opt(descriptor, 16) else {
            return Ok(None);
        };
        let mask_idx = field_u32(vm_field)?;
        let mask_entry = self.entry(mask_idx)?;
        if mask_entry.mode() != MODE_VALIDITY_MASK || mask_entry.stride() != 1 {
            return Err(ProfileError::Semantics(sem::Error::Validity));
        }
        let bytes = self.verify_chunk(mask_idx)?;
        if bytes.len() as u64 != record_count {
            return Err(ProfileError::Semantics(sem::Error::Validity));
        }
        Ok(Some(bytes))
    }

    /// Validates descriptor tensor shape (`Schema 205` key 14), channels/scalar consistency,
    /// and verifies whole chunk payload digest.
    pub fn check_structured_entry(
        &self,
        entry_idx: u32,
        expected_mode: u16,
        expected_stride: u32,
        require_medium: bool,
        required_extension_schema: Option<u16>,
    ) -> Result<(Entry<'a>, Block<'a>, &'a [u8], Option<&'a [u8]>), ProfileError> {
        let e = self.entry(entry_idx)?;
        if e.mode() != expected_mode {
            return Err(ProfileError::InvalidMode(e.mode()));
        }
        if e.stride() != expected_stride {
            return Err(ProfileError::InvalidStride {
                expected: expected_stride,
                actual: e.stride(),
            });
        }
        let desc = self.descriptor_for_entry(e)?;
        if require_medium && field_opt(desc, 4).is_none() {
            return Err(ProfileError::MediumMissingOrUnknown("descriptor key 4 (medium) is required"));
        }
        // Mixed structured modes omit scalar (key 11) and channels (key 9).
        if field_opt(desc, 9).is_some() || field_opt(desc, 11).is_some() {
            return Err(ProfileError::InvalidMetadata("structured mode must omit scalar/channels"));
        }
        let grid_blk = self
            .extension_block(desc, 205)?
            .ok_or(ProfileError::MissingRequiredSchema(205))?;
        let shape = field_u64_vec(field_req(grid_blk, 14)?)?;
        let prod = checked_shape_product(&shape)?;
        if prod != e.record_count() {
            return Err(ProfileError::InvalidMetadata("tensor_shape product disagrees with record_count"));
        }
        if let Some(ext_schema) = required_extension_schema {
            if self.extension_block(desc, ext_schema)?.is_none() {
                return Err(ProfileError::MissingRequiredSchema(ext_schema));
            }
        }
        let payload = self.verify_chunk(entry_idx)?;
        let expected_bytes = e
            .record_count()
            .checked_mul(u64::from(expected_stride))
            .ok_or(ProfileError::Overflow)?;
        if payload.len() as u64 != expected_bytes {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let mask = self.read_validity_mask(desc, e.record_count())?;
        Ok((e, desc, payload, mask))
    }

    pub fn read_clock(&self, id: u32) -> Result<ClockSpec, ProfileError> {
        let b = self.block(id)?;
        if b.schema_type() != 1 {
            return Err(ProfileError::Semantics(sem::Error::ReferenceSchema));
        }
        let spec = ClockSpec {
            scale: field_u32(field_req(b, 1)?)?,
            epoch_seconds: field_i64(field_req(b, 2)?)?,
            epoch_nanoseconds: field_u32(field_req(b, 3)?)?,
            ticks_per_second: RationalRate::new(
                field_u64(field_req(b, 4)?)?,
                field_u64(field_req(b, 5)?)?,
            )?,
        };
        spec.validate()?;
        Ok(spec)
    }

    pub fn read_instant(&self, id: u32) -> Result<(InstantSpec, ClockSpec), ProfileError> {
        let b = self.block(id)?;
        if b.schema_type() != 5 {
            return Err(ProfileError::Semantics(sem::Error::ReferenceSchema));
        }
        let clk_id = field_u32(field_req(b, 1)?)?;
        let clk = self.read_clock(clk_id)?;
        let inst = InstantSpec::new(
            clk_id,
            field_i64(field_req(b, 2)?)?,
            field_u64(field_req(b, 3)?)?,
            field_u64(field_req(b, 4)?)?,
        )?;
        Ok((inst, clk))
    }

    pub fn read_frame_quaternion(&self, id: u32) -> Result<QuaternionXyzw, ProfileError> {
        let b = self.block(id)?;
        if b.schema_type() != 2 {
            return Err(ProfileError::Semantics(sem::Error::ReferenceSchema));
        }
        let f = field_req(b, 4)?;
        let vals = field_f64_vec(f)?;
        if vals.len() != 4 {
            return Err(ProfileError::InvalidQuaternion);
        }
        QuaternionXyzw::from_xyzw([vals[0], vals[1], vals[2], vals[3]])
    }
}

pub fn checked_shape_product(shape: &[u64]) -> Result<u64, ProfileError> {
    if shape.is_empty() {
        return Err(ProfileError::InvalidMetadata("empty tensor_shape"));
    }
    let mut prod = 1u64;
    for &d in shape {
        if d == 0 {
            return Err(ProfileError::InvalidMetadata("zero tensor_shape axis"));
        }
        prod = prod.checked_mul(d).ok_or(ProfileError::Overflow)?;
    }
    Ok(prod)
}

pub fn field_opt<'a>(block: Block<'a>, key: u16) -> Option<Field<'a>> {
    block.fields().find(|f| f.key() == key)
}

pub fn field_req<'a>(block: Block<'a>, key: u16) -> Result<Field<'a>, ProfileError> {
    field_opt(block, key).ok_or(ProfileError::Semantics(sem::Error::MissingField))
}

pub fn field_str<'a>(f: Field<'a>) -> Result<&'a str, ProfileError> {
    let s = std::str::from_utf8(f.value()).map_err(|_| ProfileError::InvalidMetadata("utf8"))?;
    if has_control_chars(s) {
        return Err(ProfileError::InvalidMetadata("control-char-in-identifier"));
    }
    Ok(s)
}

pub fn field_u32(f: Field<'_>) -> Result<u32, ProfileError> {
    read_u32(f.value(), 0).map_err(|_| ProfileError::InvalidMetadata("u32"))
}

pub fn field_u64(f: Field<'_>) -> Result<u64, ProfileError> {
    read_u64(f.value(), 0).map_err(|_| ProfileError::InvalidMetadata("u64"))
}

pub fn field_i64(f: Field<'_>) -> Result<i64, ProfileError> {
    let b = f.value();
    if b.len() != 8 {
        return Err(ProfileError::InvalidMetadata("i64"));
    }
    Ok(i64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

pub fn field_f64(f: Field<'_>) -> Result<f64, ProfileError> {
    let b = f.value();
    if b.len() != 8 {
        return Err(ProfileError::InvalidMetadata("f64"));
    }
    let v = f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    require_finite_f64(v)
}

pub fn field_f64_vec(f: Field<'_>) -> Result<Vec<f64>, ProfileError> {
    if f.value().len() % 8 != 0 {
        return Err(ProfileError::InvalidMetadata("f64[]"));
    }
    let mut out = Vec::with_capacity(f.value().len() / 8);
    for c in f.value().chunks_exact(8) {
        let v = f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
        out.push(require_finite_f64(v)?);
    }
    Ok(out)
}

pub fn field_u64_vec(f: Field<'_>) -> Result<Vec<u64>, ProfileError> {
    if f.value().len() % 8 != 0 {
        return Err(ProfileError::InvalidMetadata("u64[]"));
    }
    Ok(f.value()
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect())
}

pub fn field_i64_vec(f: Field<'_>) -> Result<Vec<i64>, ProfileError> {
    if f.value().len() % 8 != 0 {
        return Err(ProfileError::InvalidMetadata("i64[]"));
    }
    Ok(f.value()
        .chunks_exact(8)
        .map(|c| i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]))
        .collect())
}

pub fn field_bytes32(f: Field<'_>) -> Result<[u8; 32], ProfileError> {
    let b = f.value();
    if b.len() != 32 {
        return Err(ProfileError::InvalidMetadata("bytes[32]"));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(b);
    Ok(out)
}

// ---------------------------------------------------------------------------
// Unified QAC v1 entry router (signal preservation + domain profiles)
// ---------------------------------------------------------------------------

/// Disambiguated QAC v1 entry kind across Schema 210 signal preservation (`0x0100`/`0x0101`/`0x0104`)
/// and Q03-Q07 scientific domain profiles (`0x0000`/`0x0001`/`0x0003`/`0x0102`..=`0x0107`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmittedEntryKind {
    PcmPreservation,
    StftPreservation,
    OpaqueSource,
    PressureVelocity,
    ElasticStressVelocity,
    HoaSpatial,
    HrirImpulse,
    RayContribution,
    ModalResonance,
    PerceptualFeatures,
    HapticSample,
    UltrasoundSummary,
    RawRfIq,
    ValidityMask,
    AuxiliaryArray,
}

impl AdmittedEntryKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PcmPreservation => "pcm-preservation",
            Self::StftPreservation => "stft-preservation",
            Self::OpaqueSource => "opaque-source",
            Self::PressureVelocity => "pressure-velocity",
            Self::ElasticStressVelocity => "elastic-stress-velocity",
            Self::HoaSpatial => "hoa-spatial",
            Self::HrirImpulse => "hrir-impulse",
            Self::RayContribution => "ray-contribution",
            Self::ModalResonance => "modal-resonance",
            Self::PerceptualFeatures => "perceptual-features",
            Self::HapticSample => "haptic-sample",
            Self::UltrasoundSummary => "ultrasound-summary",
            Self::RawRfIq => "raw-rf-iq",
            Self::ValidityMask => "validity-mask",
            Self::AuxiliaryArray => "auxiliary-array",
        }
    }

    /// Only Schema 210 signal preservation entries (`0x0100`/`0x0101`) expose direct PCM/STFT
    /// sample windows; domain profiles require explicit scientific synthesis outside storage.
    pub const fn is_signal_waveform_entry(self) -> bool {
        matches!(self, Self::PcmPreservation | Self::StftPreservation)
    }
}

/// Verified QAC v1 entry metadata summary from `AdmittedEnvelope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmittedEntrySummary<'a> {
    pub kind: AdmittedEntryKind,
    pub mode: u16,
    pub quantity: &'a str,
    pub units: &'a str,
    pub purpose: &'a str,
    pub model_uri: Option<&'a str>,
    pub channels: Option<u32>,
    pub has_validity_mask: bool,
}

impl<'a> AdmittedEnvelope<'a> {
    /// Classifies and verifies an entry's descriptor + required extension block markers across
    /// all QAC v1 modes (`0x0000`, `0x0001`, `0x0003`, `0x0100`..=`0x0107`) without touching raw sample playback.
    pub fn inspect_entry_summary(
        &self,
        index: u32,
    ) -> Result<AdmittedEntrySummary<'a>, ProfileError> {
        let e = self.entry(index)?;
        let desc = self.descriptor_for_entry(e)?;
        let quantity = field_str(field_req(desc, 7)?)?;
        let units = field_str(field_req(desc, 8)?)?;
        let purpose = field_str(field_req(desc, 12)?)?;
        let channels = field_opt(desc, 9).map(field_u32).transpose()?;
        let has_validity_mask = field_opt(desc, 16).is_some();
        let model_uri = self
            .extension_block(desc, 205)?
            .and_then(|b| field_opt(b, 1))
            .map(field_str)
            .transpose()?;

        let kind = match e.mode() {
            MODE_PRESSURE_VELOCITY => AdmittedEntryKind::PressureVelocity,
            MODE_ELASTIC_STRESS_VELOCITY => AdmittedEntryKind::ElasticStressVelocity,
            MODE_RAY_CONTRIBUTION => AdmittedEntryKind::RayContribution,
            MODE_PCM_PRESERVATION => AdmittedEntryKind::PcmPreservation,
            MODE_STFT_PRESERVATION => AdmittedEntryKind::StftPreservation,
            MODE_HOA_SPATIAL => AdmittedEntryKind::HoaSpatial,
            MODE_HRIR_IMPULSE => AdmittedEntryKind::HrirImpulse,
            0x0104 => {
                if self.extension_block(desc, 206)?.is_some() {
                    AdmittedEntryKind::ModalResonance
                } else if model_uri == Some("urn:qualia:qac:opaque-original:1") {
                    AdmittedEntryKind::OpaqueSource
                } else {
                    return Err(ProfileError::InvalidMode(e.mode()));
                }
            }
            0x0105 => {
                if self.extension_block(desc, 203)?.is_some() {
                    AdmittedEntryKind::PerceptualFeatures
                } else if self.extension_block(desc, 208)?.is_some()
                    || model_uri == Some("urn:qac:mode:0x0105:ultrasound-raw:v1")
                {
                    AdmittedEntryKind::RawRfIq
                } else {
                    return Err(ProfileError::InvalidMode(e.mode()));
                }
            }
            0x0106 => {
                if self.extension_block(desc, 207)?.is_some() {
                    AdmittedEntryKind::HapticSample
                } else if e.stride() == 1 {
                    AdmittedEntryKind::ValidityMask
                } else {
                    return Err(ProfileError::InvalidMode(e.mode()));
                }
            }
            0x0107 => {
                if self.extension_block(desc, 208)?.is_some() {
                    AdmittedEntryKind::UltrasoundSummary
                } else {
                    AdmittedEntryKind::AuxiliaryArray
                }
            }
            other => return Err(ProfileError::InvalidMode(other)),
        };

        Ok(AdmittedEntrySummary {
            kind,
            mode: e.mode(),
            quantity,
            units,
            purpose,
            model_uri,
            channels,
            has_validity_mask,
        })
    }
}

