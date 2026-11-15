//! Q06 Mode `0x0104`: Damped acoustic modal resonance bank (`32 B` per mode).
//!
//! Enforces positive frequency, signed finite amplitude preservation, nonnegative decay `d`
//! with exact amplitude `T60 = ln(1000)/d` (`d = 0 => None`, never `+inf`), explicit mode shape
//! and excitation coupling arrays (`0x0107`), explicit silence/unvoiced status, and clean
//! disambiguation from `0x0104` opaque byte preservation (`stride 1` vs `stride 32` + `Schema 206`).

use std::f64::consts::PI;

use super::envelope::*;
use super::ray_rir::push_aux_f32_entry;
use super::types::*;

pub const MODAL_RECORD_STRIDE: u32 = 32;

/// Exact 32-byte wire record for `mode 0x0104` (`modal_resonance`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModalRecord {
    pub frequency_hz: f32,
    pub amplitude_pa: f32,
    pub decay_rate_s1: f32,
    pub phase_rad: f32,
    pub mode_shape_index: u32,
    pub material_ref: u32,
    pub excitation_index: u32,
}

impl ModalRecord {
    pub fn validate(&self, externally_valid: bool) -> Result<(), ProfileError> {
        if !self.frequency_hz.is_finite()
            || !self.amplitude_pa.is_finite()
            || !self.decay_rate_s1.is_finite()
            || !self.phase_rad.is_finite()
        {
            return Err(ProfileError::NonFiniteFloat);
        }
        if !externally_valid {
            if self.frequency_hz.to_bits() != 0
                || self.amplitude_pa.to_bits() != 0
                || self.decay_rate_s1.to_bits() != 0
                || self.phase_rad.to_bits() != 0
            {
                return Err(ProfileError::MaskedLaneNonZero);
            }
            return Ok(());
        }
        if self.frequency_hz <= 0.0 {
            return Err(ProfileError::InvalidMetadata("modal frequency_hz must be > 0"));
        }
        if self.decay_rate_s1 < 0.0 {
            return Err(ProfileError::InvalidMetadata("modal decay_rate_s1 must be >= 0"));
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8], externally_valid: bool) -> Result<Self, ProfileError> {
        if bytes.len() != MODAL_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        if read_u32(bytes, 28)? != 0 {
            return Err(ProfileError::NonZeroPadding);
        }
        let rec = Self {
            frequency_hz: read_f32_finite(bytes, 0)?,
            amplitude_pa: read_f32_finite(bytes, 4)?,
            decay_rate_s1: read_f32_finite(bytes, 8)?,
            phase_rad: read_f32_finite(bytes, 12)?,
            mode_shape_index: read_u32(bytes, 16)?,
            material_ref: read_u32(bytes, 20)?,
            excitation_index: read_u32(bytes, 24)?,
        };
        rec.validate(externally_valid)?;
        Ok(rec)
    }

    pub fn encode_into(&self, out: &mut [u8], externally_valid: bool) -> Result<(), ProfileError> {
        if out.len() != MODAL_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        self.validate(externally_valid)?;
        out.fill(0);
        write_f32_finite(out, 0, self.frequency_hz)?;
        write_f32_finite(out, 4, self.amplitude_pa)?;
        write_f32_finite(out, 8, self.decay_rate_s1)?;
        write_f32_finite(out, 12, self.phase_rad)?;
        write_u32(out, 16, self.mode_shape_index);
        write_u32(out, 20, self.material_ref);
        write_u32(out, 24, self.excitation_index);
        write_u32(out, 28, 0);
        Ok(())
    }

    /// Critical damping ratio `zeta = d / (2 * pi * f)`.
    pub fn damping_ratio(&self) -> Result<f64, ProfileError> {
        self.validate(true)?;
        Ok(f64::from(self.decay_rate_s1) / (2.0 * PI * f64::from(self.frequency_hz)))
    }

    /// Exact amplitude decay `T60 = ln(1000) / d = 3*ln(10)/d` (`-60 dB` amplitude and energy).
    /// Returns `Ok(None)` when `decay_rate_s1 == 0.0` (never `+inf`).
    pub fn t60_seconds(&self) -> Result<Option<f64>, ProfileError> {
        self.validate(true)?;
        if self.decay_rate_s1 == 0.0 {
            return Ok(None);
        }
        Ok(Some(1000.0f64.ln() / f64::from(self.decay_rate_s1)))
    }

    /// Evaluates damped sinusoidal response `A * exp(-d * t) * sin(2 * pi * f * t + phi)`.
    pub fn evaluate_at(&self, t_s: f64) -> Result<f64, ProfileError> {
        if !t_s.is_finite() || t_s < 0.0 {
            return Err(ProfileError::InvalidTimeSpec);
        }
        self.validate(true)?;
        let amp = f64::from(self.amplitude_pa);
        let d = f64::from(self.decay_rate_s1);
        let f = f64::from(self.frequency_hz);
        let phi = f64::from(self.phase_rad);
        Ok(amp * (-d * t_s).exp() * (2.0 * PI * f * t_s + phi).sin())
    }
}

/// Evaluates modal synthesis output at `t_s >= 0` for receiver `dof`, multiplying each valid mode
/// by its mode-shape coupling `mode_shapes[mode_shape_index * dofs + dof]` and excitation weight.
pub fn synthesize_modal_sample(
    modes: &[ModalRecord],
    statuses: Option<&[SampleStatus]>,
    mode_shapes: &[f32],
    dofs: usize,
    dof: usize,
    excitations: &[f32],
    t_s: f64,
) -> Result<f64, ProfileError> {
    if dofs == 0 || dof >= dofs || mode_shapes.len() % dofs != 0 {
        return Err(ProfileError::OutOfBoundsIndex("modal dof/shapes"));
    }
    if let Some(st) = statuses {
        if st.len() != modes.len() {
            return Err(ProfileError::InvalidPayloadLength);
        }
    }
    let shape_rows = mode_shapes.len() / dofs;
    let mut sum = 0.0f64;
    for (idx, m) in modes.iter().enumerate() {
        let is_valid = statuses.map_or(true, |s| s[idx] == SampleStatus::Valid);
        m.validate(is_valid)?;
        if !is_valid {
            continue;
        }
        if (m.mode_shape_index as usize) >= shape_rows {
            return Err(ProfileError::OutOfBoundsIndex("mode_shape_index"));
        }
        if (m.excitation_index as usize) >= excitations.len() {
            return Err(ProfileError::OutOfBoundsIndex("excitation_index"));
        }
        let shape_w = f64::from(mode_shapes[m.mode_shape_index as usize * dofs + dof]);
        let exc_w = f64::from(excitations[m.excitation_index as usize]);
        sum += shape_w * exc_w * m.evaluate_at(t_s)?;
    }
    Ok(sum)
}

/// Zero-heap borrowed view over verified `0x0104` modal oscillator records.
pub struct ModalChunkView<'a> {
    bytes: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> ModalChunkView<'a> {
    pub fn len(&self) -> usize {
        self.bytes.len() / MODAL_RECORD_STRIDE as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<ModalRecord, ProfileError> {
        let start = index
            .checked_mul(MODAL_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let end = start
            .checked_add(MODAL_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("modal record"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        ModalRecord::decode(slice, ext_valid)
    }
}

/// Writes canonical QAC v1 `mode 0x0104` modal bank (`Schema 206`), required `mode_shapes` &
/// `excitation` (`0x0107`) arrays, optional `0x0106` validity mask, and optional per-mode status codes (`0x0107`).
pub fn write_modal_bank_qac(
    modes: &[ModalRecord],
    mode_shapes: &[f32],
    dofs: u32,
    excitations: &[f32],
    statuses: Option<&[SampleStatus]>,
    clock: &ClockSpec,
    sample_rate: RationalRate,
    provenance: &ProvenanceSpec<'_>,
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    if modes.is_empty()
        || dofs == 0
        || mode_shapes.is_empty()
        || mode_shapes.len() % (dofs as usize) != 0
        || excitations.is_empty()
    {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let shape_rows = mode_shapes.len() / (dofs as usize);
    if let Some(st) = statuses {
        if st.len() != modes.len() {
            return Err(ProfileError::InvalidPayloadLength);
        }
    }
    let mask_bytes: Option<Vec<u8>> = statuses.map(|st| {
        st.iter()
            .map(|&s| if s == SampleStatus::Valid { 1 } else { 0 })
            .collect()
    });
    let mut main_payload = vec![0u8; modes.len() * MODAL_RECORD_STRIDE as usize];
    for (idx, m) in modes.iter().enumerate() {
        let valid = mask_bytes.as_ref().map_or(true, |mb| mb[idx] != 0);
        if (m.mode_shape_index as usize) >= shape_rows {
            return Err(ProfileError::OutOfBoundsIndex("mode_shape_index"));
        }
        if (m.excitation_index as usize) >= excitations.len() {
            return Err(ProfileError::OutOfBoundsIndex("excitation_index"));
        }
        let off = idx * MODAL_RECORD_STRIDE as usize;
        m.encode_into(&mut main_payload[off..off + MODAL_RECORD_STRIDE as usize], valid)?;
    }

    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, clock)?;
    meta.push_provenance(3, provenance)?;

    // Canonical directory order with stream_id=1:
    // Entry 1: mode 0x0104 (descriptor 6)
    // Entry 2 (optional mask): mode 0x0106 (descriptor 7)
    // Aux entries (desc 8 excitation, desc 9 mode_shapes, desc 10 optional status): mode 0x0107
    // Note: 0x0104 < 0x0106 < 0x0107! Putting all in stream_id=1 ensures Entry 1 is index 1,
    // mask (if any) is index 2, excitation is index 2 or 3, mode_shapes is index 3 or 4!
    let mut next_idx = 2u32;
    let mask_idx = if mask_bytes.is_some() {
        let idx = next_idx;
        next_idx += 1;
        Some(idx)
    } else {
        None
    };
    let exc_idx = next_idx;
    next_idx += 1;
    let shapes_idx = next_idx;
    next_idx += 1;
    let status_idx = if statuses.is_some() {
        Some(next_idx)
    } else {
        None
    };

    meta.push_block(OwnedBlock {
        id: 4,
        schema_type: 206,
        revision: 1,
        critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("modal_bank".to_owned())),
            OwnedField::crit(8, OwnedValue::EntryRefs(vec![exc_idx])),
            OwnedField::crit(9, OwnedValue::EntryRefs(vec![shapes_idx])),
        ],
    })?;
    let tensor_shape = [modes.len() as u64];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: "urn:qac:model:modal-resonance-bank:v1",
            model_version: "1",
            discretization: Some("damped-harmonic-modes"),
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: Some(sample_rate),
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &tensor_shape,
            component_names: "frequency_hz,amplitude_pa,decay_rate_s1,phase_rad",
        },
    )?;

    let valid_count = mask_bytes
        .as_ref()
        .map(|mb| mb.iter().filter(|&&b| b == 1).count() as u64);
    let ext = [4u32, 5u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: Some(1),
            frame_id: None,
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "modal-resonance",
            units: "Pa",
            channels: None,
            layout: None,
            scalar: None,
            model_uri: "urn:qac:mode:0x0104:modal-resonance:v1",
            sample_rate: Some(sample_rate),
            valid_record_count: valid_count,
            validity_mask_entry: mask_idx,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;

    let mut entries = vec![OwnedEntry {
        mode: MODE_MODAL_RESONANCE,
        flags: 3,
        stream_id: 1,
        frame_id: 0,
        start_tick: 0,
        time_count: 1,
        spatial_key: 0,
        record_count: modes.len() as u64,
        stride: MODAL_RECORD_STRIDE,
        descriptor_id: 6,
        payload: main_payload,
    }];
    if let Some(mb) = &mask_bytes {
        entries.push(push_validity_mask_entry(&mut meta, 7, 3, 1, 0, 1, mb)?);
    }
    let mut e_exc = push_aux_f32_entry(&mut meta, 8, 3, "modal-excitation", "1", 1, AuxScalar::F32, excitations)?;
    e_exc.stream_id = 1;
    let mut e_shapes = push_aux_f32_entry(&mut meta, 9, 3, "modal-mode-shapes", "1", dofs, AuxScalar::F32, mode_shapes)?;
    e_shapes.stream_id = 1;
    entries.push(e_exc);
    entries.push(e_shapes);

    if let (Some(st), Some(_)) = (statuses, status_idx) {
        let status_u8: Vec<u8> = st.iter().map(|&s| s as u8).collect();
        meta.push_descriptor(
            10,
            &DescriptorSpec {
                clock_id: None,
                frame_id: None,
                provenance_id: 3,
                medium_id: None,
                transform_id: None,
                calibration_id: None,
                quantity: "modal-sample-status",
                units: "1",
                channels: Some(1),
                layout: Some("record-component"),
                scalar: Some(AuxScalar::U8),
                model_uri: "urn:qac:mode:0x0107:sample-status:v1",
                sample_rate: None,
                valid_record_count: None,
                validity_mask_entry: None,
                source_entries: &[],
                extension_metadata: &[],
            },
        )?;
        entries.push(OwnedEntry {
            mode: MODE_AUX_ARRAY,
            flags: 1,
            stream_id: 1,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: status_u8.len() as u64,
            stride: 1,
            descriptor_id: 10,
            payload: status_u8,
        });
    }
    assemble_qac_v1(&meta, &entries, limits)
}

pub struct ModalBankView<'a> {
    pub records: ModalChunkView<'a>,
    pub mode_shapes: Vec<f32>,
    pub shape_dofs: usize,
    pub excitations: Vec<f32>,
}

/// Reads and validates a `mode 0x0104` modal resonance bank.
/// Rejects `opaque_source_preservation` (`stride == 1` / missing Schema 206) without conflation.
pub fn read_modal_bank_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<ModalBankView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let (_e, desc, payload, mask) = env.check_structured_entry(
        entry_index,
        MODE_MODAL_RESONANCE,
        MODAL_RECORD_STRIDE,
        false,
        Some(206),
    )?;
    let s206 = env
        .extension_block(desc, 206)?
        .ok_or(ProfileError::MissingRequiredSchema(206))?;
    if field_str(field_req(s206, 1)?)? != "modal_bank" {
        return Err(ProfileError::InvalidMetadata("Schema 206 kind must be modal_bank"));
    }
    let exc_idx = field_u32(field_req(s206, 8)?)?;
    let exc_bytes = env.verify_chunk(exc_idx)?;
    if exc_bytes.len() % 4 != 0 || exc_bytes.is_empty() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let mut excitations = Vec::with_capacity(exc_bytes.len() / 4);
    for i in 0..exc_bytes.len() / 4 {
        excitations.push(read_f32_finite(exc_bytes, i * 4)?);
    }

    let shp_idx = field_u32(field_req(s206, 9)?)?;
    let shp_entry = env.entry(shp_idx)?;
    let shp_desc = env.descriptor_for_entry(shp_entry)?;
    let shape_dofs = field_u32(field_req(shp_desc, 9)?)? as usize;
    if shape_dofs == 0 {
        return Err(ProfileError::InvalidMetadata("mode_shapes dofs == 0"));
    }
    let shp_bytes = env.verify_chunk(shp_idx)?;
    if shp_bytes.len() % (shape_dofs * 4) != 0 || shp_bytes.is_empty() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let mut mode_shapes = Vec::with_capacity(shp_bytes.len() / 4);
    for i in 0..shp_bytes.len() / 4 {
        mode_shapes.push(read_f32_finite(shp_bytes, i * 4)?);
    }
    let shape_rows = mode_shapes.len() / shape_dofs;

    let chunk = ModalChunkView {
        bytes: payload,
        mask,
    };
    for i in 0..chunk.len() {
        let rec = chunk.get(i)?;
        if (rec.mode_shape_index as usize) >= shape_rows {
            return Err(ProfileError::OutOfBoundsIndex("mode_shape_index"));
        }
        if (rec.excitation_index as usize) >= excitations.len() {
            return Err(ProfileError::OutOfBoundsIndex("excitation_index"));
        }
    }
    Ok(ModalBankView {
        records: chunk,
        mode_shapes,
        shape_dofs,
        excitations,
    })
}
