//! Q05 Mode `0x0003`: Ray / RIR path contributions (`64 B`) with band transfer, phase, and diffraction metadata.
//!
//! Strictly enforces unit direction norm (`<= 1e-5` in `f64`), nonnegative distance/delay,
//! energy-ratio vs signed/complex pressure amplitude distinction (never inventing phase from energy),
//! explicit diffraction models (`none`, `UTD-wedge`, `BTM-aperture`), and bounds-checked `band_row`.

use super::envelope::*;
use super::types::*;

pub const RAY_RECORD_STRIDE: u32 = 64;
pub const RAY_VALID_DELAY: u32 = 1 << 0;
pub const RAY_VALID_DOPPLER: u32 = 1 << 1;
pub const RAY_VALID_ALL: u32 = RAY_VALID_DELAY | RAY_VALID_DOPPLER;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GainQuantity {
    EnergyRatio,
    PressureAmplitudeRatio,
}

impl GainQuantity {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EnergyRatio => "energy-ratio",
            Self::PressureAmplitudeRatio => "pressure-amplitude-ratio",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ProfileError> {
        match s {
            "energy-ratio" => Ok(Self::EnergyRatio),
            "pressure-amplitude-ratio" => Ok(Self::PressureAmplitudeRatio),
            _ => Err(ProfileError::InvalidMetadata("gain_quantity")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffractionModel {
    None,
    UtdWedge,
    BtmAperture,
}

impl DiffractionModel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::UtdWedge => "UTD-wedge",
            Self::BtmAperture => "BTM-aperture",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ProfileError> {
        match s {
            "none" => Ok(Self::None),
            "UTD-wedge" => Ok(Self::UtdWedge),
            "BTM-aperture" => Ok(Self::BtmAperture),
            _ => Err(ProfileError::InvalidMetadata("diffraction_model")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandPhaseModel {
    /// Non-negative broadband energy scalars (`EnergyRatio` only); no phase information present.
    EnergyOnlyNoPhase,
    /// Real signed pressure amplitude ratios preserving 0 / pi phase inversions.
    SignedAmplitudeReal,
    /// Complex `[re, im]` transfer values per `[ray, band]`.
    ComplexPerBandTransfer,
}

/// Exact 64-byte wire record for `mode 0x0003` (`RayContribution`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayContributionRecord {
    pub origin_m: [f32; 3],
    pub distance_m: f32,
    pub direction: [f32; 3],
    pub delay_s: f32,
    pub ray_id: u64,
    pub band_row: u32,
    pub material_ref: u32,
    pub specular_count: u16,
    pub diffraction_count: u16,
    pub scattering_coefficient: f32,
    pub doppler_ratio: f32,
    pub validity_bits: u32,
}

impl RayContributionRecord {
    pub fn validate(&self, externally_valid: bool) -> Result<(), ProfileError> {
        if self.origin_m.iter().any(|v| !v.is_finite())
            || !self.distance_m.is_finite()
            || !self.delay_s.is_finite()
            || !self.scattering_coefficient.is_finite()
            || !self.doppler_ratio.is_finite()
        {
            return Err(ProfileError::NonFiniteFloat);
        }
        if self.validity_bits & !RAY_VALID_ALL != 0 {
            return Err(ProfileError::InvalidValidityBits);
        }
        if !externally_valid {
            if self.validity_bits != 0
                || self.origin_m.iter().any(|v| v.to_bits() != 0)
                || self.distance_m.to_bits() != 0
                || self.direction.iter().any(|v| v.to_bits() != 0)
                || self.delay_s.to_bits() != 0
                || self.scattering_coefficient.to_bits() != 0
                || self.doppler_ratio.to_bits() != 0
            {
                return Err(ProfileError::MaskedLaneNonZero);
            }
            return Ok(());
        }
        let _ = UnitDirection3::new(self.direction)?;
        if self.distance_m < 0.0 || !(0.0..=1.0).contains(&self.scattering_coefficient) {
            return Err(ProfileError::InvalidMetadata("ray distance or scattering range"));
        }
        let has_delay = (self.validity_bits & RAY_VALID_DELAY) != 0;
        require_zero_when_masked(has_delay, self.delay_s)?;
        if has_delay && self.delay_s < 0.0 {
            return Err(ProfileError::InvalidMetadata("ray delay_s < 0"));
        }
        let has_doppler = (self.validity_bits & RAY_VALID_DOPPLER) != 0;
        require_zero_when_masked(has_doppler, self.doppler_ratio)?;
        if has_doppler && self.doppler_ratio <= 0.0 {
            return Err(ProfileError::InvalidMetadata("ray doppler_ratio <= 0"));
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8], externally_valid: bool) -> Result<Self, ProfileError> {
        if bytes.len() != RAY_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let rec = Self {
            origin_m: read_f32_3(bytes, 0)?,
            distance_m: read_f32_finite(bytes, 12)?,
            direction: read_f32_3(bytes, 16)?,
            delay_s: read_f32_finite(bytes, 28)?,
            ray_id: read_u64(bytes, 32)?,
            band_row: read_u32(bytes, 40)?,
            material_ref: read_u32(bytes, 44)?,
            specular_count: read_u16(bytes, 48)?,
            diffraction_count: read_u16(bytes, 50)?,
            scattering_coefficient: read_f32_finite(bytes, 52)?,
            doppler_ratio: read_f32_finite(bytes, 56)?,
            validity_bits: read_u32(bytes, 60)?,
        };
        rec.validate(externally_valid)?;
        Ok(rec)
    }

    pub fn encode_into(&self, out: &mut [u8], externally_valid: bool) -> Result<(), ProfileError> {
        if out.len() != RAY_RECORD_STRIDE as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        self.validate(externally_valid)?;
        out.fill(0);
        write_f32_slice(out, 0, &self.origin_m)?;
        write_f32_finite(out, 12, self.distance_m)?;
        write_f32_slice(out, 16, &self.direction)?;
        write_f32_finite(out, 28, self.delay_s)?;
        write_u64(out, 32, self.ray_id);
        write_u32(out, 40, self.band_row);
        write_u32(out, 44, self.material_ref);
        write_u16(out, 48, self.specular_count);
        write_u16(out, 50, self.diffraction_count);
        write_f32_finite(out, 52, self.scattering_coefficient)?;
        write_f32_finite(out, 56, self.doppler_ratio)?;
        write_u32(out, 60, self.validity_bits);
        Ok(())
    }
}

/// Zero-heap borrowed view over verified `0x0003` ray records.
pub struct RayContributionChunkView<'a> {
    bytes: &'a [u8],
    mask: Option<&'a [u8]>,
}

impl<'a> RayContributionChunkView<'a> {
    pub fn len(&self) -> usize {
        self.bytes.len() / RAY_RECORD_STRIDE as usize
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn get(&self, index: usize) -> Result<RayContributionRecord, ProfileError> {
        let start = index
            .checked_mul(RAY_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let end = start
            .checked_add(RAY_RECORD_STRIDE as usize)
            .ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("ray record"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        RayContributionRecord::decode(slice, ext_valid)
    }
}

/// Decoded band transfer values preserving energy vs signed vs complex phase distinctions.
#[derive(Debug, Clone, PartialEq)]
pub enum RayBandValues {
    EnergyRatio { bands: usize, values: Vec<f32> },
    SignedPressureRatio { bands: usize, values: Vec<f32> },
    ComplexTransfer { bands: usize, re_im: Vec<[f32; 2]> },
}

impl RayBandValues {
    pub fn phase_model(&self) -> BandPhaseModel {
        match self {
            Self::EnergyRatio { .. } => BandPhaseModel::EnergyOnlyNoPhase,
            Self::SignedPressureRatio { .. } => BandPhaseModel::SignedAmplitudeReal,
            Self::ComplexTransfer { .. } => BandPhaseModel::ComplexPerBandTransfer,
        }
    }

    pub fn gain_quantity(&self) -> GainQuantity {
        match self {
            Self::EnergyRatio { .. } => GainQuantity::EnergyRatio,
            Self::SignedPressureRatio { .. } | Self::ComplexTransfer { .. } => {
                GainQuantity::PressureAmplitudeRatio
            }
        }
    }

    pub fn bands(&self) -> usize {
        match self {
            Self::EnergyRatio { bands, .. }
            | Self::SignedPressureRatio { bands, .. }
            | Self::ComplexTransfer { bands, .. } => *bands,
        }
    }

    pub fn row_count(&self) -> usize {
        let b = self.bands().max(1);
        match self {
            Self::EnergyRatio { values, .. } | Self::SignedPressureRatio { values, .. } => {
                values.len() / b
            }
            Self::ComplexTransfer { re_im, .. } => re_im.len() / b,
        }
    }

    /// Synthesizes signed broadband impulse contributions per tap from amplitude rays.
    /// Rejects `EnergyRatio` because energy carries no phase / polarity!
    pub fn synthesize_signed_rir_impulse(
        &self,
        rays: &[RayContributionRecord],
        band_index: usize,
        sample_rate_hz: f64,
        taps: usize,
    ) -> Result<Vec<f32>, ProfileError> {
        if matches!(self, Self::EnergyRatio { .. }) {
            return Err(ProfileError::InvalidDomainConversion(
                "cannot synthesize phase-bearing signed RIR impulse from energy-only ray gains",
            ));
        }
        if band_index >= self.bands() || !sample_rate_hz.is_finite() || sample_rate_hz <= 0.0 {
            return Err(ProfileError::OutOfBoundsIndex("band_index"));
        }
        let mut ir = vec![0.0f64; taps];
        let b = self.bands();
        for r in rays {
            if (r.validity_bits & RAY_VALID_DELAY) == 0 {
                continue;
            }
            let row = r.band_row as usize;
            if row >= self.row_count() {
                return Err(ProfileError::OutOfBoundsIndex("ray band_row"));
            }
            let amp = match self {
                Self::SignedPressureRatio { values, .. } => f64::from(values[row * b + band_index]),
                Self::ComplexTransfer { re_im, .. } => f64::from(re_im[row * b + band_index][0]),
                Self::EnergyRatio { .. } => unreachable!(),
            };
            let idx = (f64::from(r.delay_s) * sample_rate_hz).round() as usize;
            if idx < taps {
                ir[idx] += amp;
            }
        }
        Ok(ir.into_iter().map(|v| v as f32).collect())
    }
}

/// Normal-incidence reflection & transmission energy conservation helper:
/// `R_p = (Z2 - Z1) / (Z2 + Z1)`, `E_reflected = R_p^2`, `E_transmitted = 1 - R_p^2`.
pub fn planar_impedance_reflection_energy(z1: f64, z2: f64) -> Result<(f64, f64, f64), ProfileError> {
    if !z1.is_finite() || !z2.is_finite() || z1 <= 0.0 || z2 <= 0.0 {
        return Err(ProfileError::MediumMissingOrUnknown("positive acoustic impedances required"));
    }
    let r_p = (z2 - z1) / (z2 + z1);
    let e_refl = r_p * r_p;
    let e_trans = 1.0 - e_refl;
    Ok((r_p, e_refl, e_trans))
}

/// Writes canonical QAC v1 `mode 0x0003` rays + auxiliary band centers/values/weights (`0x0107`).
pub fn write_ray_contributions_qac(
    rays: &[RayContributionRecord],
    band_centers_hz: &[f32],
    band_values: &RayBandValues,
    reference_distance_m: f64,
    diffraction: DiffractionModel,
    ray_weights: Option<&[f32]>,
    launch_time: Option<&InstantSpec>,
    clock: &ClockSpec,
    frame: &FrameSpec<'_>,
    provenance: &ProvenanceSpec<'_>,
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    if rays.is_empty() || band_centers_hz.is_empty() || band_centers_hz.len() != band_values.bands() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if !reference_distance_m.is_finite() || reference_distance_m <= 0.0 {
        return Err(ProfileError::InvalidMetadata("reference_distance_m <= 0"));
    }
    for w in band_centers_hz.windows(2) {
        if !w[0].is_finite() || !w[1].is_finite() || w[0] <= 0.0 || w[0] >= w[1] {
            return Err(ProfileError::InvalidMetadata("band_centers_hz must be strictly increasing > 0"));
        }
    }
    let rows = band_values.row_count();
    if rows == 0 {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if let Some(rw) = ray_weights {
        if rw.len() != rays.len() || rw.iter().any(|&w| !w.is_finite() || w < 0.0) {
            return Err(ProfileError::InvalidMetadata("ray_weights"));
        }
    }
    let mut main_payload = vec![0u8; rays.len() * RAY_RECORD_STRIDE as usize];
    for (i, r) in rays.iter().enumerate() {
        if r.band_row as usize >= rows {
            return Err(ProfileError::OutOfBoundsIndex("ray band_row >= rows"));
        }
        if diffraction == DiffractionModel::None && r.diffraction_count > 0 {
            return Err(ProfileError::InvalidMetadata("diffraction_count > 0 under DiffractionModel::None"));
        }
        let off = i * RAY_RECORD_STRIDE as usize;
        r.encode_into(&mut main_payload[off..off + RAY_RECORD_STRIDE as usize], true)?;
    }

    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, clock)?;
    meta.push_frame(2, frame)?;
    meta.push_provenance(3, provenance)?;
    let launch_id = if let Some(lt) = launch_time {
        Some(meta.push_instant(8, lt)?)
    } else {
        None
    };

    // Entries: 1 = 0x0003 rays, 2 = 0x0107 band_centers, 3 = 0x0107 band_values, 4 = optional ray_weights.
    // All stream_id=0 static or stream_id=1? Use stream_id=0 with 0x0003 < 0x0107 and desc_id 7 < 8 < 9.
    let e_centers = push_aux_f32_entry(&mut meta, 7, 3, "band-centers-hz", "Hz", 1, AuxScalar::F32, band_centers_hz)?;
    let e_values = match band_values {
        RayBandValues::EnergyRatio { values, .. } => {
            if values.iter().any(|&v| !v.is_finite() || v < 0.0) {
                return Err(ProfileError::InvalidMetadata("negative energy-ratio band value"));
            }
            push_aux_f32_entry(&mut meta, 8, 3, "ray-band-energy", "1", band_values.bands() as u32, AuxScalar::F32, values)?
        }
        RayBandValues::SignedPressureRatio { values, .. } => {
            push_aux_f32_entry(&mut meta, 8, 3, "ray-band-amplitude", "Pa/Pa", band_values.bands() as u32, AuxScalar::F32, values)?
        }
        RayBandValues::ComplexTransfer { re_im, .. } => {
            let mut flat = Vec::with_capacity(re_im.len() * 2);
            for &[re, im] in re_im {
                flat.push(re);
                flat.push(im);
            }
            push_aux_f32_entry(&mut meta, 8, 3, "ray-band-complex-transfer", "Pa/Pa", band_values.bands() as u32, AuxScalar::ComplexF32, &flat)?
        }
    };

    let mut s206_fields = vec![
        OwnedField::crit(1, OwnedValue::Utf8("geometric_rays".to_owned())),
        OwnedField::opt(2, OwnedValue::EntryRefs(vec![2])),
        OwnedField::crit(3, OwnedValue::Utf8(band_values.gain_quantity().as_str().to_owned())),
        OwnedField::crit(4, OwnedValue::F64(vec![reference_distance_m])),
        OwnedField::opt(10, OwnedValue::Utf8(diffraction.as_str().to_owned())),
        OwnedField::opt(12, OwnedValue::EntryRefs(vec![3])),
    ];
    if let Some(lid) = launch_id {
        s206_fields.push(OwnedField::opt(5, OwnedValue::MetadataRefs(vec![lid])));
    }

    let mut entries = vec![
        OwnedEntry {
            mode: MODE_RAY_CONTRIBUTION,
            flags: 1,
            stream_id: 0,
            frame_id: 2,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: rays.len() as u64,
            stride: RAY_RECORD_STRIDE,
            descriptor_id: 6,
            payload: main_payload,
        },
        e_centers,
        e_values,
    ];
    if let Some(rw) = ray_weights {
        let e_w = push_aux_f32_entry(&mut meta, 9, 3, "ray-weights", "1", 1, AuxScalar::F32, rw)?;
        s206_fields.push(OwnedField::opt(11, OwnedValue::EntryRefs(vec![4])));
        entries.push(e_w);
    }

    meta.push_block(OwnedBlock {
        id: 4,
        schema_type: 206,
        revision: 1,
        critical: true,
        fields: s206_fields,
    })?;
    let shape = [rays.len() as u64];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: "urn:qac:model:geometric-ray-paths:v1",
            model_version: "1",
            discretization: Some("ray-path-contributions"),
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: None,
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &shape,
            component_names: "ray_contribution",
        },
    )?;
    let ext = [4u32, 5u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: None,
            frame_id: Some(2),
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "ray-path-contributions",
            units: "mixed:mode-registry",
            channels: None,
            layout: None,
            scalar: None,
            model_uri: "urn:qac:mode:0x0003:v1",
            sample_rate: None,
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;

    assemble_qac_v1(&meta, &entries, limits)
}

pub(crate) fn push_aux_f32_entry(
    meta: &mut MetadataBuilder,
    desc_id: u32,
    prov_id: u32,
    qty: &str,
    units: &str,
    channels: u32,
    scalar: AuxScalar,
    vals: &[f32],
) -> Result<OwnedEntry, ProfileError> {
    let mut payload = vec![0u8; vals.len() * 4];
    write_f32_slice(&mut payload, 0, vals)?;
    let stride = channels * scalar.byte_size();
    let rec_count = (payload.len() as u64) / u64::from(stride);
    meta.push_descriptor(
        desc_id,
        &DescriptorSpec {
            clock_id: None,
            frame_id: None,
            provenance_id: prov_id,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: qty,
            units,
            channels: Some(channels),
            layout: Some("record-component"),
            scalar: Some(scalar),
            model_uri: "urn:qac:mode:0x0107:v1",
            sample_rate: None,
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[],
            extension_metadata: &[],
        },
    )?;
    Ok(OwnedEntry {
        mode: MODE_AUX_ARRAY,
        flags: 1,
        stream_id: 0,
        frame_id: 0,
        start_tick: 0,
        time_count: 0,
        spatial_key: 0,
        record_count: rec_count,
        stride,
        descriptor_id: desc_id,
        payload,
    })
}

pub struct RayContributionView<'a> {
    pub records: RayContributionChunkView<'a>,
    pub gain_quantity: GainQuantity,
    pub diffraction_model: DiffractionModel,
    pub band_centers_hz: Vec<f32>,
    pub band_values: RayBandValues,
}

pub fn read_ray_contributions_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<RayContributionView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let (_e, desc, payload, mask) = env.check_structured_entry(
        entry_index,
        MODE_RAY_CONTRIBUTION,
        RAY_RECORD_STRIDE,
        false,
        Some(206),
    )?;
    let s206 = env
        .extension_block(desc, 206)?
        .ok_or(ProfileError::MissingRequiredSchema(206))?;
    let gain_quantity = GainQuantity::parse(field_str(field_req(s206, 3)?)?)?;
    let diffraction_model = field_opt(s206, 10)
        .map(field_str)
        .transpose()?
        .map(DiffractionModel::parse)
        .transpose()?
        .unwrap_or(DiffractionModel::None);

    let centers_idx = field_u32(field_req(s206, 2)?)?;
    let c_bytes = env.verify_chunk(centers_idx)?;
    if c_bytes.len() % 4 != 0 || c_bytes.is_empty() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let mut band_centers_hz = Vec::with_capacity(c_bytes.len() / 4);
    for i in 0..c_bytes.len() / 4 {
        band_centers_hz.push(read_f32_finite(c_bytes, i * 4)?);
    }
    let bands = band_centers_hz.len();

    let vals_idx = field_u32(field_req(s206, 12)?)?;
    let v_entry = env.entry(vals_idx)?;
    let v_desc = env.descriptor_for_entry(v_entry)?;
    let v_scalar = AuxScalar::from_u32(field_u32(field_req(v_desc, 11)?)?)
        .ok_or(ProfileError::InvalidMetadata("band_values scalar"))?;
    let v_bytes = env.verify_chunk(vals_idx)?;

    let band_values = match (gain_quantity, v_scalar) {
        (GainQuantity::EnergyRatio, AuxScalar::F32) => {
            let mut vals = Vec::with_capacity(v_bytes.len() / 4);
            for i in 0..v_bytes.len() / 4 {
                let v = read_f32_finite(v_bytes, i * 4)?;
                if v < 0.0 {
                    return Err(ProfileError::InvalidMetadata("negative energy-ratio"));
                }
                vals.push(v);
            }
            RayBandValues::EnergyRatio { bands, values: vals }
        }
        (GainQuantity::PressureAmplitudeRatio, AuxScalar::F32) => {
            let mut vals = Vec::with_capacity(v_bytes.len() / 4);
            for i in 0..v_bytes.len() / 4 {
                vals.push(read_f32_finite(v_bytes, i * 4)?);
            }
            RayBandValues::SignedPressureRatio { bands, values: vals }
        }
        (GainQuantity::PressureAmplitudeRatio, AuxScalar::ComplexF32) => {
            if v_bytes.len() % 8 != 0 {
                return Err(ProfileError::InvalidPayloadLength);
            }
            let mut pairs = Vec::with_capacity(v_bytes.len() / 8);
            for i in 0..v_bytes.len() / 8 {
                pairs.push(read_f32_2(v_bytes, i * 8)?);
            }
            RayBandValues::ComplexTransfer { bands, re_im: pairs }
        }
        _ => {
            return Err(ProfileError::InvalidDomainConversion(
                "energy-ratio cannot store complex phase transfer scalar",
            ));
        }
    };
    let rows = band_values.row_count();
    let chunk = RayContributionChunkView {
        bytes: payload,
        mask,
    };
    for i in 0..chunk.len() {
        let r = chunk.get(i)?;
        if (r.band_row as usize) >= rows {
            return Err(ProfileError::OutOfBoundsIndex("ray band_row out of bounds"));
        }
        if diffraction_model == DiffractionModel::None && r.diffraction_count > 0 {
            return Err(ProfileError::InvalidMetadata("diffraction_count under DiffractionModel::None"));
        }
    }
    Ok(RayContributionView {
        records: chunk,
        gain_quantity,
        diffraction_model,
        band_centers_hz,
        band_values,
    })
}
