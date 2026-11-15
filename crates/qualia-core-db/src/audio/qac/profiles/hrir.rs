//! Q05 Mode `0x0103`: HRIR / RIR FIR taps (`taps * 4` B) & legacy Q4AU v3 SOFA preservation.
//!
//! Strictly separates impulse sample rate (`Schema 204` keys 17/18) from measurement acquisition
//! cadence (`Schema 3` keys 13/14), applies explicit per-receiver delays once relative to excitation
//! origin, and performs lossless Q4AU v3 -> QAC v1 migration retaining unparsed original bytes (`Schema 209`).

use super::envelope::*;
use super::types::*;

#[path = "hrir_helpers.rs"]
mod hrir_helpers;
pub use hrir_helpers::*;

/// Static or temporally indexed HRIR/RIR payload specification.
#[derive(Debug, Clone)]
pub struct HrirWriteSpec<'a> {
    pub representation: &'a str, // "HRIR" or "RIR"
    pub taps: u32,
    pub receivers: u32,
    pub measurements: u32,
    pub impulse_sample_rate: RationalRate,
    pub delay_reference: &'a str,
    pub sofa_convention: Option<&'a str>,
    pub units: &'a str,
    pub delays_samples: Option<&'a [f64]>,
    pub source_positions_xyz: Option<&'a [[f32; 3]]>,
    pub receiver_positions_xyz: Option<&'a [[f32; 3]]>,
    /// Present ONLY when measurements form a uniform time series (distinct from `impulse_sample_rate`).
    pub measurement_cadence: Option<(&'a ClockSpec, RationalRate)>,
}

/// Zero-heap borrowed view over verified `0x0103` FIR tap records (`[measurement][receiver][tap]`).
pub struct HrirChunkView<'a> {
    bytes: &'a [u8],
    taps: usize,
    receivers: usize,
    measurements: usize,
    mask: Option<&'a [u8]>,
}

impl<'a> HrirChunkView<'a> {
    pub fn taps(&self) -> usize {
        self.taps
    }

    pub fn receivers(&self) -> usize {
        self.receivers
    }

    pub fn measurements(&self) -> usize {
        self.measurements
    }

    pub fn record_count(&self) -> usize {
        self.measurements * self.receivers
    }

    /// Reads exact FIR taps for `(measurement, receiver)` into caller buffer `out_taps[..taps]`.
    pub fn read_impulse(
        &self,
        measurement: usize,
        receiver: usize,
        out_taps: &mut [f32],
    ) -> Result<(), ProfileError> {
        if measurement >= self.measurements || receiver >= self.receivers {
            return Err(ProfileError::OutOfBoundsIndex("hrir measurement/receiver"));
        }
        if out_taps.len() != self.taps {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let rec_idx = measurement * self.receivers + receiver;
        let stride = self.taps.checked_mul(4).ok_or(ProfileError::Overflow)?;
        let start = rec_idx.checked_mul(stride).ok_or(ProfileError::Overflow)?;
        let end = start.checked_add(stride).ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("hrir slice"))?;
        let ext_valid = self.mask.map_or(true, |m| m[rec_idx] != 0);
        for (i, dst) in out_taps.iter_mut().enumerate() {
            let v = read_f32_finite(slice, i * 4)?;
            require_zero_when_masked(ext_valid, v)?;
            *dst = v;
        }
        Ok(())
    }
}

/// Exact tap arrival time in seconds relative to excitation origin:
/// `(delay_samples + tap_index) * (impulse_rate_den / impulse_rate_num)`. Applied ONCE.
pub fn tap_time_seconds(
    delay_samples: f64,
    tap_index: u32,
    impulse_sample_rate: RationalRate,
) -> Result<f64, ProfileError> {
    if !delay_samples.is_finite() {
        return Err(ProfileError::NonFiniteFloat);
    }
    let dt = (impulse_sample_rate.den as f64) / (impulse_sample_rate.num as f64);
    Ok((delay_samples + f64::from(tap_index)) * dt)
}

/// Exact fractional-delay impulse interpolation oracle (`y(t)` at fractional sample `idx_f`).
pub fn sample_delayed_impulse(taps: &[f32], delay_samples: f64, sample_idx: f64) -> f64 {
    let shifted = sample_idx - delay_samples;
    if shifted < 0.0 || shifted > (taps.len().saturating_sub(1)) as f64 {
        return 0.0;
    }
    let i0 = shifted.floor() as usize;
    let frac = shifted - (i0 as f64);
    if i0 + 1 < taps.len() {
        (1.0 - frac) * f64::from(taps[i0]) + frac * f64::from(taps[i0 + 1])
    } else {
        f64::from(taps[i0])
    }
}

/// Exact discrete FIR convolution for one receiver response (applying optional `delay_samples` once).
pub fn convolve_hrir(input: &[f32], taps: &[f32], out: &mut [f32]) -> Result<(), ProfileError> {
    if out.len() != input.len() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    for (n, slot) in out.iter_mut().enumerate() {
        let mut acc = 0.0f64;
        for (k, &h) in taps.iter().enumerate() {
            if n >= k {
                acc += f64::from(input[n - k]) * f64::from(h);
            }
        }
        *slot = acc as f32;
    }
    Ok(())
}

/// Writes canonical QAC v1 `mode 0x0103` HRIR/RIR container (plus optional delays & positions `0x0107`).
pub fn write_hrir_qac(
    impulses: &[f32],
    spec: &HrirWriteSpec<'_>,
    frame: &FrameSpec<'_>,
    provenance: &ProvenanceSpec<'_>,
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    if spec.representation != "HRIR" && spec.representation != "RIR" {
        return Err(ProfileError::InvalidMetadata("representation must be HRIR or RIR"));
    }
    if spec.delay_reference != BASE_DELAY_REFERENCE && spec.delay_reference != LEGACY_DELAY_REFERENCE {
        return Err(ProfileError::InvalidMetadata("invalid delay_reference"));
    }
    if spec.taps == 0 || spec.receivers == 0 || spec.measurements == 0 {
        return Err(ProfileError::InvalidMetadata("taps/receivers/measurements must be > 0"));
    }
    if spec.taps > limits.max_taps || spec.receivers > limits.max_channels {
        return Err(ProfileError::LimitExceeded);
    }
    let rec_count = u64::from(spec.measurements)
        .checked_mul(u64::from(spec.receivers))
        .ok_or(ProfileError::Overflow)?;
    let expected_floats = rec_count
        .checked_mul(u64::from(spec.taps))
        .ok_or(ProfileError::Overflow)?;
    if impulses.len() as u64 != expected_floats {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let stride = spec.taps.checked_mul(4).ok_or(ProfileError::Overflow)?;
    let mut main_payload = vec![0u8; impulses.len() * 4];
    write_f32_slice(&mut main_payload, 0, impulses)?;

    let mut meta = MetadataBuilder::new();
    let has_time = spec.measurement_cadence.is_some();
    if let Some((clk, _)) = spec.measurement_cadence {
        meta.push_clock(1, clk)?;
    }
    meta.push_frame(2, frame)?;
    meta.push_provenance(3, provenance)?;

    let mut entries = Vec::new();
    let stream_id = if has_time { 1 } else { 0 };
    let main_flags = if has_time { 3 } else { 1 };
    let main_desc_id = 6u32;

    // Canonical directory order: (stream_id, start_tick, spatial_key, mode, descriptor_id).
    // Main entry uses mode 0x0103; auxiliary geometry/delays use mode 0x0107 at stream_id=0 or stream_id=main
    // so 0x0103 < 0x0107 cleanly orders Main as Entry 1 and Aux arrays as Entry 2+.
    let mut s204_fields = vec![
        OwnedField::crit(1, OwnedValue::Utf8(spec.representation.to_owned())),
        OwnedField::crit(8, OwnedValue::U32(vec![spec.taps])),
        OwnedField::crit(9, OwnedValue::U32(vec![spec.receivers])),
        OwnedField::crit(10, OwnedValue::U32(vec![spec.measurements])),
        OwnedField::opt(13, OwnedValue::MetadataRefs(vec![2])),
        OwnedField::crit(17, OwnedValue::U64(vec![spec.impulse_sample_rate.num])),
        OwnedField::crit(18, OwnedValue::U64(vec![spec.impulse_sample_rate.den])),
        OwnedField::crit(19, OwnedValue::Utf8(spec.delay_reference.to_owned())),
    ];
    if let Some(conv) = spec.sofa_convention {
        validate_identifier_str(conv, "sofa_convention")?;
        s204_fields.push(OwnedField::opt(12, OwnedValue::Utf8(conv.to_owned())));
    }

    let mut aux_desc_id = 7u32;
    let mut next_entry_idx = 2u32;
    let mut aux_entries = Vec::new();

    if let Some(rx_pos) = spec.receiver_positions_xyz {
        if rx_pos.len() != spec.receivers as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let e = push_aux_f32_vec3_entry(&mut meta, aux_desc_id, 3, "receiver-positions", "m", rx_pos)?;
        s204_fields.push(OwnedField::opt(5, OwnedValue::EntryRefs(vec![next_entry_idx])));
        aux_entries.push(e);
        aux_desc_id += 1;
        next_entry_idx += 1;
    }
    if let Some(tx_pos) = spec.source_positions_xyz {
        if tx_pos.len() != spec.measurements as usize {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let e = push_aux_f32_vec3_entry(&mut meta, aux_desc_id, 3, "source-positions", "m", tx_pos)?;
        s204_fields.push(OwnedField::opt(6, OwnedValue::EntryRefs(vec![next_entry_idx])));
        aux_entries.push(e);
        aux_desc_id += 1;
        next_entry_idx += 1;
    }
    if let Some(delays) = spec.delays_samples {
        if delays.len() as u64 != rec_count {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let e = push_aux_f64_scalar_entry(&mut meta, aux_desc_id, 3, "delays-samples", "1", delays)?;
        s204_fields.push(OwnedField::opt(7, OwnedValue::EntryRefs(vec![next_entry_idx])));
        aux_entries.push(e);
    }

    meta.push_block(OwnedBlock {
        id: 4,
        schema_type: 204,
        revision: 1,
        critical: true,
        fields: s204_fields,
    })?;

    let shape = [
        u64::from(spec.measurements),
        u64::from(spec.receivers),
        u64::from(spec.taps),
    ];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: "urn:qac:model:hrir-fir:v1",
            model_version: "1",
            discretization: Some("discrete-fir-taps"),
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: None,
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &shape,
            component_names: "hrir_tap",
        },
    )?;

    let ext = [4u32, 5u32];
    let (clock_id, sample_rate) = if let Some((_, rate)) = spec.measurement_cadence {
        (Some(1), Some(rate))
    } else {
        (None, None)
    };
    meta.push_descriptor(
        main_desc_id,
        &DescriptorSpec {
            clock_id,
            frame_id: Some(2),
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "impulse-response",
            units: spec.units,
            channels: Some(spec.receivers),
            layout: Some("measurement-receiver-tap"),
            scalar: Some(AuxScalar::F32),
            model_uri: "urn:qac:mode:0x0103:v1",
            sample_rate,
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;

    entries.push(OwnedEntry {
        mode: MODE_HRIR_IMPULSE,
        flags: main_flags,
        stream_id,
        frame_id: 2,
        start_tick: 0,
        time_count: if has_time { u64::from(spec.measurements) } else { 0 },
        spatial_key: 0,
        record_count: rec_count,
        stride,
        descriptor_id: main_desc_id,
        payload: main_payload,
    });
    // Note: if stream_id == 1 on main entry, place aux at stream_id = 1 too? No: static aux at stream_id=0
    // sorts BEFORE stream_id=1! So when has_time==true, aux at stream_id=0 comes first!
    // Reorder cleanly if has_time==true by putting aux at stream_id=1 so main (0x0103) precedes aux (0x0107).
    for mut aux in aux_entries {
        aux.stream_id = stream_id;
        entries.push(aux);
    }
    assemble_qac_v1(&meta, &entries, limits)
}

pub struct HrirDocumentView<'a> {
    pub chunk: HrirChunkView<'a>,
    pub representation: &'a str,
    pub impulse_sample_rate: RationalRate,
    pub delay_reference: &'a str,
    pub delays_samples: Option<Vec<f64>>,
    pub legacy_original_bytes: Option<&'a [u8]>,
}

/// Opens and fully validates `mode 0x0103` HRIR/RIR entry + optional delays & legacy origin.
pub fn read_hrir_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<HrirDocumentView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let e = env.entry(entry_index)?;
    if e.mode() != MODE_HRIR_IMPULSE {
        return Err(ProfileError::InvalidMode(e.mode()));
    }
    let desc = env.descriptor_for_entry(e)?;
    let s204 = env
        .extension_block(desc, 204)?
        .ok_or(ProfileError::MissingRequiredSchema(204))?;
    let repr = field_str(field_req(s204, 1)?)?;
    if repr != "HRIR" && repr != "RIR" {
        return Err(ProfileError::InvalidMetadata("Schema 204 representation must be HRIR or RIR"));
    }
    let taps = field_u32(field_req(s204, 8)?)?;
    let receivers = field_u32(field_req(s204, 9)?)?;
    let measurements = field_u32(field_req(s204, 10)?)?;
    if taps == 0 || receivers == 0 || measurements == 0 || taps > limits.max_taps {
        return Err(ProfileError::InvalidMetadata("invalid HRIR dimensions"));
    }
    let impulse_sample_rate = RationalRate::new(
        field_u64(field_req(s204, 17)?)?,
        field_u64(field_req(s204, 18)?)?,
    )?;
    let delay_reference = field_str(field_req(s204, 19)?)?;
    if delay_reference == LEGACY_DELAY_REFERENCE && env.extension_block(desc, 209)?.is_none() {
        return Err(ProfileError::MissingRequiredSchema(209));
    }
    let expected_rec = u64::from(measurements)
        .checked_mul(u64::from(receivers))
        .ok_or(ProfileError::Overflow)?;
    let expected_stride = taps.checked_mul(4).ok_or(ProfileError::Overflow)?;
    if e.record_count() != expected_rec || e.stride() != expected_stride {
        return Err(ProfileError::InvalidStride {
            expected: expected_stride,
            actual: e.stride(),
        });
    }
    let payload = env.verify_chunk(entry_index)?;
    let mask = env.read_validity_mask(desc, expected_rec)?;

    let delays_samples = if let Some(df) = field_opt(s204, 7) {
        let d_idx = field_u32(df)?;
        let d_bytes = env.verify_chunk(d_idx)?;
        if d_bytes.len() as u64 != expected_rec * 8 {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let mut out = Vec::with_capacity(expected_rec as usize);
        for c in d_bytes.chunks_exact(8) {
            let v = f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]);
            out.push(require_finite_f64(v)?);
        }
        Some(out)
    } else {
        None
    };

    let legacy_original_bytes = if let Some(s209) = env.extension_block(desc, 209)? {
        let orig_idx = field_u32(field_req(s209, 3)?)?;
        let orig_digest = field_bytes32(field_req(s209, 4)?)?;
        let orig_bytes = env.verify_chunk(orig_idx)?;
        if blake3::hash(orig_bytes).as_bytes() != &orig_digest {
            return Err(ProfileError::LegacyFormat("Schema 209 original_digest mismatch"));
        }
        Some(orig_bytes)
    } else {
        None
    };

    let chunk = HrirChunkView {
        bytes: payload,
        taps: taps as usize,
        receivers: receivers as usize,
        measurements: measurements as usize,
        mask,
    };
    let mut tmp = vec![0.0f32; taps as usize];
    for m in 0..measurements as usize {
        for r in 0..receivers as usize {
            chunk.read_impulse(m, r, &mut tmp)?;
        }
    }
    Ok(HrirDocumentView {
        chunk,
        representation: repr,
        impulse_sample_rate,
        delay_reference,
        delays_samples,
        legacy_original_bytes,
    })
}
