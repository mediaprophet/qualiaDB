//! Q07 Mode `0x0106`: Haptic mechanical force, displacement, contact geometry,
//! mechanoreceptor models, and separate actuation-command authority evaluation (`Schema 207`).
//!
//! Strictly separates passive QAC storage from hardware actuation: `read_haptic_qac`
//! validates structural and metadata contracts without acting as an actuation gate,
//! while `evaluate_actuator_command_at` enforces role, digest, and exact `i128` expiry `[< expiry]`.

use super::envelope::*;
use super::types::*;

/// Frozen wire stride in bytes for `mode 0x0106` haptic mechanical samples.
pub const HAPTIC_RECORD_STRIDE: u32 = 64;

/// Bitmask for `HapticRecord::validity_bits`: force vector (`16..28`).
pub const HAPTIC_VALID_FORCE: u32 = 1 << 0;
/// Bitmask for `HapticRecord::validity_bits`: normal indentation (`28..32`).
pub const HAPTIC_VALID_INDENTATION: u32 = 1 << 1;
/// Bitmask for `HapticRecord::validity_bits`: surface displacement (`32..44`).
pub const HAPTIC_VALID_DISPLACEMENT: u32 = 1 << 2;
/// Bitmask for `HapticRecord::validity_bits`: vibrotactile carrier frequency (`44..48`).
pub const HAPTIC_VALID_CARRIER_HZ: u32 = 1 << 3;
/// Bitmask covering all defined validity bits (`0..=3`).
pub const HAPTIC_VALID_ALL: u32 =
    HAPTIC_VALID_FORCE | HAPTIC_VALID_INDENTATION | HAPTIC_VALID_DISPLACEMENT | HAPTIC_VALID_CARRIER_HZ;

/// Explicit role of a haptic entry recorded in `Schema 207` key 1 (`role`).
#[derive(Debug, Clone, PartialEq)]
pub enum HapticRole {
    /// Physically observed contact force / indentation / displacement sensor measurement.
    Measurement {
        receptor_model_uri: Option<String>,
    },
    /// Modelled mechanoreceptor neural output (e.g. SA-I / RA-I / PC firing estimate).
    ReceptorModel {
        receptor_model_uri: String,
    },
    /// Authorized actuator drive command requiring profile digest, authority receipt, and expiry.
    ActuatorCommand {
        device_profile_digest: [u8; 32],
        authority_receipt_digest: [u8; 32],
        command_expiry: InstantSpec,
        receptor_model_uri: Option<String>,
    },
}

impl HapticRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Measurement { .. } => "measurement",
            Self::ReceptorModel { .. } => "receptor-model",
            Self::ActuatorCommand { .. } => "actuator-command",
        }
    }
}

/// Explicit action-reaction sign convention between contacting body and haptic device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceActionConvention {
    /// Stored vector is the force exerted ON the contacted body/tissue by the device.
    ForceOnBody,
    /// Stored vector is the reaction force exerted ON the haptic device by the body.
    ForceOnDevice,
}

impl ForceActionConvention {
    /// Converts stored force into `ForceOnBody` without ambiguous sign assumptions.
    pub const fn to_force_on_body(self, f: [f64; 3]) -> [f64; 3] {
        match self {
            Self::ForceOnBody => f,
            Self::ForceOnDevice => [-f[0], -f[1], -f[2]],
        }
    }
}

/// Full `Schema 207` haptic contact & authorization specification.
#[derive(Debug, Clone, PartialEq)]
pub struct HapticContactSpec {
    pub role: HapticRole,
    pub contact_frame: FrameSpec<'static>,
    pub contact_area_m2: Option<f64>,
}

/// Canonical 64-byte Q07 `mode 0x0106` haptic sample record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HapticRecord {
    pub contact_position_m: [f32; 3],
    pub contact_area_m2: f32,
    pub force_local_xyz_n: [f32; 3],
    pub normal_indentation_m: f32,
    pub displacement_local_xyz_m: [f32; 3],
    pub carrier_hz: f32,
    pub validity_bits: u32,
    pub contact_ref: u32,
}

impl HapticRecord {
    pub fn validate(&self, externally_valid: bool) -> Result<(), ProfileError> {
        for v in self.contact_position_m {
            if !v.is_finite() {
                return Err(ProfileError::NonFiniteFloat);
            }
            require_zero_when_masked(externally_valid, v)?;
        }
        if !self.contact_area_m2.is_finite() || self.contact_area_m2 < 0.0 {
            return Err(ProfileError::InvalidMetadata("haptic contact_area_m2 < 0"));
        }
        require_zero_when_masked(externally_valid, self.contact_area_m2)?;
        for v in self
            .force_local_xyz_n
            .iter()
            .chain(std::iter::once(&self.normal_indentation_m))
            .chain(self.displacement_local_xyz_m.iter())
            .chain(std::iter::once(&self.carrier_hz))
        {
            if !v.is_finite() {
                return Err(ProfileError::NonFiniteFloat);
            }
        }
        if self.validity_bits & !HAPTIC_VALID_ALL != 0 {
            return Err(ProfileError::InvalidValidityBits);
        }
        if self.contact_ref == 0 {
            return Err(ProfileError::InvalidMetadata("haptic contact_ref must be nonzero Schema 207 ID"));
        }
        if !externally_valid && self.validity_bits != 0 {
            return Err(ProfileError::MaskedLaneNonZero);
        }

        let force_valid = externally_valid && (self.validity_bits & HAPTIC_VALID_FORCE != 0);
        let ind_valid = externally_valid && (self.validity_bits & HAPTIC_VALID_INDENTATION != 0);
        let disp_valid = externally_valid && (self.validity_bits & HAPTIC_VALID_DISPLACEMENT != 0);
        let carrier_valid = externally_valid && (self.validity_bits & HAPTIC_VALID_CARRIER_HZ != 0);

        require_zero_vec3_when_masked(force_valid, self.force_local_xyz_n)?;
        require_zero_when_masked(ind_valid, self.normal_indentation_m)?;
        require_zero_vec3_when_masked(disp_valid, self.displacement_local_xyz_m)?;
        require_zero_when_masked(carrier_valid, self.carrier_hz)?;
        if carrier_valid && self.carrier_hz < 0.0 {
            return Err(ProfileError::InvalidMetadata("negative haptic carrier_hz"));
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8], externally_valid: bool) -> Result<Self, ProfileError> {
        if bytes.len() != HAPTIC_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        ensure_zero_bytes(&bytes[56..64])?;
        let rec = Self {
            contact_position_m: read_f32_3(bytes, 0)?,
            contact_area_m2: read_f32_finite(bytes, 12)?,
            force_local_xyz_n: read_f32_3(bytes, 16)?,
            normal_indentation_m: read_f32_finite(bytes, 28)?,
            displacement_local_xyz_m: read_f32_3(bytes, 32)?,
            carrier_hz: read_f32_finite(bytes, 44)?,
            validity_bits: read_u32(bytes, 48)?,
            contact_ref: read_u32(bytes, 52)?,
        };
        rec.validate(externally_valid)?;
        Ok(rec)
    }

    pub fn encode_into(&self, out: &mut [u8], externally_valid: bool) -> Result<(), ProfileError> {
        if out.len() != HAPTIC_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        self.validate(externally_valid)?;
        out.fill(0);
        write_f32_slice(out, 0, &self.contact_position_m)?;
        write_f32_finite(out, 12, self.contact_area_m2)?;
        write_f32_slice(out, 16, &self.force_local_xyz_n)?;
        write_f32_finite(out, 28, self.normal_indentation_m)?;
        write_f32_slice(out, 32, &self.displacement_local_xyz_m)?;
        write_f32_finite(out, 44, self.carrier_hz)?;
        write_u32(out, 48, self.validity_bits);
        write_u32(out, 52, self.contact_ref);
        Ok(())
    }
}

/// Zero-heap borrowed view over verified `mode 0x0106` haptic sample records.
pub struct HapticChunkView<'a> {
    bytes: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> HapticChunkView<'a> {
    pub fn len(&self) -> usize {
        self.bytes.len() / HAPTIC_RECORD_STRIDE as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<HapticRecord, ProfileError> {
        let start = index
            .checked_mul(HAPTIC_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let end = start
            .checked_add(HAPTIC_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("haptic record index"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        HapticRecord::decode(slice, ext_valid)
    }
}

// ---------------------------------------------------------------------------
// Mechanics helpers: frame rotation, torque, pressure, Hooke stiffness
// ---------------------------------------------------------------------------

/// Actively rotates a local haptic force vector into its parent frame via Hamilton quaternion `q`.
pub fn rotate_force_to_parent(
    force_local_xyz_n: [f32; 3],
    contact_rotation: QuaternionXyzw,
    convention: ForceActionConvention,
) -> Result<[f64; 3], ProfileError> {
    for c in force_local_xyz_n {
        if !c.is_finite() {
            return Err(ProfileError::NonFiniteFloat);
        }
    }
    let body_local = convention.to_force_on_body([
        f64::from(force_local_xyz_n[0]),
        f64::from(force_local_xyz_n[1]),
        f64::from(force_local_xyz_n[2]),
    ]);
    Ok(contact_rotation.rotate_vec3(body_local))
}

/// Computes torque vector `tau = r x F` (`N*m`) about contact frame origin.
pub fn torque_about_origin_n_m(position_m: [f64; 3], force_n: [f64; 3]) -> Result<[f64; 3], ProfileError> {
    if position_m.iter().chain(force_n.iter()).any(|v| !v.is_finite()) {
        return Err(ProfileError::NonFiniteFloat);
    }
    Ok([
        position_m[1] * force_n[2] - position_m[2] * force_n[1],
        position_m[2] * force_n[0] - position_m[0] * force_n[2],
        position_m[0] * force_n[1] - position_m[1] * force_n[0],
    ])
}

/// Derives average normal contact pressure `P = F_n / A` (`Pa`), requiring strictly positive area.
pub fn contact_pressure_pa(normal_force_n: f64, contact_area_m2: f64) -> Result<f64, ProfileError> {
    if !normal_force_n.is_finite() || !contact_area_m2.is_finite() {
        return Err(ProfileError::NonFiniteFloat);
    }
    if contact_area_m2 <= 0.0 {
        return Err(ProfileError::InvalidMetadata("contact_area_m2 must be > 0 to derive pressure"));
    }
    Ok(normal_force_n / contact_area_m2)
}

/// Hooke spring oracle `F = k * x + F0` (`N`) along contact normal or tangential DOF.
pub fn hooke_spring_force_n(
    stiffness_n_per_m: f64,
    displacement_m: f64,
    preload_n: f64,
) -> Result<f64, ProfileError> {
    if !stiffness_n_per_m.is_finite() || !displacement_m.is_finite() || !preload_n.is_finite() {
        return Err(ProfileError::NonFiniteFloat);
    }
    if stiffness_n_per_m < 0.0 {
        return Err(ProfileError::InvalidMetadata("spring stiffness < 0"));
    }
    Ok(stiffness_n_per_m * displacement_m + preload_n)
}

// ---------------------------------------------------------------------------
// QAC v1 passive codec + separate actuation command gate
// ---------------------------------------------------------------------------

/// Writes a Q07 `mode 0x0106` haptic QAC v1 container.
///
/// NOTE: Writing `"actuator-command"` into QAC v1 only records its profile, authority
/// receipt, and expiry deadline; container assembly does NOT grant physical actuation.
pub fn write_haptic_qac(
    records: &[HapticRecord],
    contact_spec: &HapticContactSpec,
    receptor_output: Option<&[f32]>,
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
    if let Some(m) = validity_mask {
        if m.len() != records.len() || m.iter().any(|&b| b > 1) {
            return Err(ProfileError::InvalidValidityBits);
        }
    }
    if let Some(ro) = receptor_output {
        if ro.len() != records.len() {
            return Err(ProfileError::InvalidPayloadLength);
        }
    }
    if let Some(area) = contact_spec.contact_area_m2 {
        if !area.is_finite() || area <= 0.0 {
            return Err(ProfileError::InvalidMetadata("Schema 207 contact_area_m2 must be > 0"));
        }
    }

    // Contact block ID is 4; force all caller records to reference 4 consistently or validate 4.
    const CONTACT_BLOCK_ID: u32 = 4;
    let mut payload = vec![0u8; records.len() * HAPTIC_RECORD_STRIDE as usize];
    for (idx, r) in records.iter().enumerate() {
        if r.contact_ref != CONTACT_BLOCK_ID {
            return Err(ProfileError::InvalidMetadata("HapticRecord.contact_ref must match Schema 207 ID 4"));
        }
        let ext_valid = validity_mask.map_or(true, |m| m[idx] != 0);
        let off = idx * HAPTIC_RECORD_STRIDE as usize;
        r.encode_into(&mut payload[off..off + HAPTIC_RECORD_STRIDE as usize], ext_valid)?;
    }

    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, clock)?;
    meta.push_frame(2, frame)?;
    meta.push_provenance(3, provenance)?;
    // Block 12 is dedicated contact_frame so Schema 207 key 2 always points to a distinct Schema 2 frame.
    meta.push_frame(12, &contact_spec.contact_frame)?;

    let mut s207_fields = vec![
        OwnedField::crit(1, OwnedValue::Utf8(contact_spec.role.as_str().to_owned())),
        OwnedField::crit(2, OwnedValue::MetadataRefs(vec![12])),
    ];
    if let Some(area) = contact_spec.contact_area_m2 {
        s207_fields.push(OwnedField::opt(3, OwnedValue::F64(vec![area])));
    }
    match &contact_spec.role {
        HapticRole::Measurement { receptor_model_uri } => {
            if let Some(uri) = receptor_model_uri {
                validate_identifier_str(uri, "receptor_model_URI")?;
                s207_fields.push(OwnedField::opt(4, OwnedValue::Utf8(uri.clone())));
            }
        }
        HapticRole::ReceptorModel { receptor_model_uri } => {
            validate_identifier_str(receptor_model_uri, "receptor_model_URI")?;
            s207_fields.push(OwnedField::crit(4, OwnedValue::Utf8(receptor_model_uri.clone())));
        }
        HapticRole::ActuatorCommand {
            device_profile_digest,
            authority_receipt_digest,
            command_expiry,
            receptor_model_uri,
        } => {
            if command_expiry.clock_block_id != 1 {
                return Err(ProfileError::InvalidMetadata("command_expiry must reference clock block 1"));
            }
            if let Some(uri) = receptor_model_uri {
                validate_identifier_str(uri, "receptor_model_URI")?;
                s207_fields.push(OwnedField::opt(4, OwnedValue::Utf8(uri.clone())));
            }
            meta.push_instant(8, command_expiry)?;
            s207_fields.push(OwnedField::crit(5, OwnedValue::Bytes(device_profile_digest.to_vec())));
            s207_fields.push(OwnedField::crit(6, OwnedValue::Bytes(authority_receipt_digest.to_vec())));
            s207_fields.push(OwnedField::crit(7, OwnedValue::MetadataRefs(vec![8])));
        }
    }

    let receptor_entry_idx = if receptor_output.is_some() {
        Some(if validity_mask.is_some() { 3u32 } else { 2u32 })
    } else {
        None
    };
    if let Some(ei) = receptor_entry_idx {
        s207_fields.push(OwnedField::opt(9, OwnedValue::EntryRefs(vec![ei])));
    }
    meta.push_block(OwnedBlock {
        id: CONTACT_BLOCK_ID,
        schema_type: 207,
        revision: 1,
        critical: true,
        fields: s207_fields,
    })?;

    let shape = [records.len() as u64];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: "urn:qac:model:haptic-mechanical-sample:v1",
            model_version: "1",
            discretization: None,
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: Some(sample_rate),
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &shape,
            component_names: "contact_xyz,area,force_xyz,indentation,disp_xyz,carrier_hz",
        },
    )?;

    let valid_count = validity_mask.map(|m| m.iter().filter(|&&b| b == 1).count() as u64);
    let mask_entry_idx = if validity_mask.is_some() { Some(2u32) } else { None };
    let ext = [CONTACT_BLOCK_ID, 5u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: Some(1),
            frame_id: Some(2),
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "haptic-mechanical-sample",
            units: "mixed:mode-registry",
            channels: None,
            layout: None,
            scalar: None,
            model_uri: "urn:qac:mode:0x0106:v1",
            sample_rate: Some(sample_rate),
            valid_record_count: valid_count,
            validity_mask_entry: mask_entry_idx,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;

    let mut entries = vec![OwnedEntry {
        mode: MODE_HAPTIC_SAMPLE,
        flags: 3,
        stream_id: 1,
        frame_id: 2,
        start_tick: 0,
        time_count: 1,
        spatial_key: 0,
        record_count: records.len() as u64,
        stride: HAPTIC_RECORD_STRIDE,
        descriptor_id: 6,
        payload,
    }];
    // Both target (desc 6) and mask (desc 7) use mode 0x0106 on stream_id=1 -> desc 6 < desc 7 sorts cleanly.
    if let Some(mask) = validity_mask {
        entries.push(push_validity_mask_entry(&mut meta, 7, 3, 1, 0, 1, mask)?);
    }
    if let Some(ro) = receptor_output {
        let mut ro_payload = vec![0u8; ro.len() * 4];
        write_f32_slice(&mut ro_payload, 0, ro)?;
        meta.push_descriptor(
            9,
            &DescriptorSpec {
                clock_id: Some(1),
                frame_id: None,
                provenance_id: 3,
                medium_id: None,
                transform_id: None,
                calibration_id: None,
                quantity: "estimated-receptor-output",
                units: "1",
                channels: Some(1),
                layout: Some("record-component"),
                scalar: Some(AuxScalar::F32),
                model_uri: "urn:qac:mode:0x0107:v1",
                sample_rate: Some(sample_rate),
                valid_record_count: None,
                validity_mask_entry: None,
                source_entries: &[],
                extension_metadata: &[],
            },
        )?;
        entries.push(OwnedEntry {
            mode: MODE_AUX_ARRAY,
            flags: 3,
            stream_id: 1,
            frame_id: 0,
            start_tick: 0,
            time_count: 1,
            spatial_key: 0,
            record_count: ro.len() as u64,
            stride: 4,
            descriptor_id: 9,
            payload: ro_payload,
        });
    }
    assemble_qac_v1(&meta, &entries, limits)
}

/// Decoded passive Q07 haptic view. Does NOT authorize hardware actuation.
pub struct HapticDocumentView<'a> {
    pub records: HapticChunkView<'a>,
    pub role: HapticRole,
    pub clock: ClockSpec,
    pub contact_frame_rotation: QuaternionXyzw,
    pub contact_area_m2: Option<f64>,
    pub expiry_clock: Option<ClockSpec>,
}

/// Passive reader validating structure and Schema 207 semantics without gating actuation.
pub fn read_haptic_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<HapticDocumentView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let (_entry, desc, payload, mask) = env.check_structured_entry(
        entry_index,
        MODE_HAPTIC_SAMPLE,
        HAPTIC_RECORD_STRIDE,
        false,
        Some(205),
    )?;
    let clock_id = field_u32(field_req(desc, 1)?)?;
    let clock = env.read_clock(clock_id)?;
    let s207 = env
        .extension_block(desc, 207)?
        .ok_or(ProfileError::MissingRequiredSchema(207))?;
    let role_str = field_str(field_req(s207, 1)?)?;
    let cframe_id = field_u32(field_req(s207, 2)?)?;
    let contact_frame_rotation = env.read_frame_quaternion(cframe_id)?;
    let contact_area_m2 = field_opt(s207, 3).map(field_f64).transpose()?;
    let receptor_uri = field_opt(s207, 4)
        .map(|f| Ok::<String, ProfileError>(field_str(f)?.to_owned()))
        .transpose()?;

    let mut expiry_clock = None;
    let role = match role_str {
        "measurement" => HapticRole::Measurement {
            receptor_model_uri: receptor_uri,
        },
        "receptor-model" => {
            let uri = receptor_uri.ok_or(ProfileError::MissingRequiredSchema(207))?;
            HapticRole::ReceptorModel {
                receptor_model_uri: uri,
            }
        }
        "actuator-command" => {
            let dev_digest = field_bytes32(field_req(s207, 5)?)?;
            let auth_digest = field_bytes32(field_req(s207, 6)?)?;
            let exp_id = field_u32(field_req(s207, 7)?)?;
            let (command_expiry, exp_clk) = env.read_instant(exp_id)?;
            expiry_clock = Some(exp_clk);
            HapticRole::ActuatorCommand {
                device_profile_digest: dev_digest,
                authority_receipt_digest: auth_digest,
                command_expiry,
                receptor_model_uri: receptor_uri,
            }
        }
        _ => return Err(ProfileError::InvalidMetadata("unknown Schema 207 haptic role")),
    };

    let chunk = HapticChunkView {
        bytes: payload,
        mask,
    };
    let expected_ref = s207.id();
    for i in 0..chunk.len() {
        let rec = chunk.get(i)?;
        if rec.contact_ref != expected_ref {
            return Err(ProfileError::InvalidMetadata("HapticRecord.contact_ref disagrees with Schema 207 ID"));
        }
    }

    Ok(HapticDocumentView {
        records: chunk,
        role,
        clock,
        contact_frame_rotation,
        contact_area_m2,
        expiry_clock,
    })
}

/// Separate explicit host command gate required before dispatching any actuator frame to hardware.
///
/// Rejects non-command roles (`measurement`, `receptor-model`), failed authority checks,
/// mismatched clock scales, and expired commands where `now >= command_expiry` using exact
/// checked `i128` rational comparisons.
pub fn evaluate_actuator_command_at(
    doc: &HapticDocumentView<'_>,
    now: &InstantSpec,
    now_clock: &ClockSpec,
    verify_authority: &dyn Fn(&[u8; 32], &[u8; 32]) -> bool,
) -> Result<(), ProfileError> {
    let (dev_digest, auth_digest, expiry) = match &doc.role {
        HapticRole::ActuatorCommand {
            device_profile_digest,
            authority_receipt_digest,
            command_expiry,
            ..
        } => (device_profile_digest, authority_receipt_digest, command_expiry),
        HapticRole::Measurement { .. } | HapticRole::ReceptorModel { .. } => {
            return Err(ProfileError::ActuationAuthority(
                "stored haptic role is not actuator-command",
            ));
        }
    };
    let exp_clk = doc
        .expiry_clock
        .as_ref()
        .ok_or(ProfileError::ActuationAuthority("missing expiry clock"))?;
    if !verify_authority(dev_digest, auth_digest) {
        return Err(ProfileError::ActuationAuthority("authority receipt verification failed"));
    }
    match now.cmp_exact(now_clock, expiry, exp_clk)? {
        std::cmp::Ordering::Less => Ok(()),
        std::cmp::Ordering::Equal | std::cmp::Ordering::Greater => Err(
            ProfileError::ActuationAuthority("actuator command expired (now >= command_expiry)"),
        ),
    }
}
