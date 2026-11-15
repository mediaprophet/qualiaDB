//! QAC v1 passive codec for Q07 Ultrasound (`mode 0x0107` beam summary, `mode 0x0105` raw RF/IQ,
//! array geometry, transmit delays, and context-qualified `TI` validation).

use super::envelope::*;
use super::hrir::{push_aux_f32_vec3_entry, push_aux_f64_scalar_entry};
use super::types::*;
use super::ultrasound::{
    RawUltrasoundPayload, UltrasoundAcquisitionSpec, UltrasoundBeamSummaryRecord,
    UltrasoundChunkView, UltrasoundRole, ULTRASOUND_SUMMARY_STRIDE, US_VALID_TI,
};

/// Writes a Q07 ultrasound QAC v1 container (`mode 0x0107` beam summaries plus optional
/// `mode 0x0105` raw RF/IQ, transducer element positions, channel map, transmit delays, and calibration).
pub fn write_ultrasound_qac(
    records: &[UltrasoundBeamSummaryRecord],
    spec: &UltrasoundAcquisitionSpec<'_>,
    raw_source: Option<&RawUltrasoundPayload<'_>>,
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
    if let Some(fc) = spec.center_frequency_hz {
        if !fc.is_finite() || fc <= 0.0 {
            return Err(ProfileError::InvalidMetadata("Schema 208 center_frequency_hz <= 0"));
        }
    }
    if let Some(phi0) = spec.demodulation_phase_rad {
        let _ = require_finite_f64(phi0)?;
    }

    let has_valid_ti = records.iter().enumerate().any(|(i, r)| {
        validity_mask.map_or(true, |m| m[i] != 0) && (r.validity_bits & US_VALID_TI != 0)
    });
    if has_valid_ti && (spec.thermal_model_uri.is_none() || spec.exposure_context.is_none()) {
        return Err(ProfileError::MissingThermalModel(
            "valid Thermal Index requires Schema 208 thermal_model_URI and exposure_context",
        ));
    }

    const ACQ_BLOCK_ID: u32 = 4;
    let mut main_payload = vec![0u8; records.len() * ULTRASOUND_SUMMARY_STRIDE as usize];
    for (idx, r) in records.iter().enumerate() {
        if r.acquisition_ref != ACQ_BLOCK_ID {
            return Err(ProfileError::InvalidMetadata("acquisition_ref must match Schema 208 ID 4"));
        }
        let ext_valid = validity_mask.map_or(true, |m| m[idx] != 0);
        let off = idx * ULTRASOUND_SUMMARY_STRIDE as usize;
        r.encode_into(&mut main_payload[off..off + ULTRASOUND_SUMMARY_STRIDE as usize], ext_valid)?;
    }

    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, clock)?;
    meta.push_frame(2, frame)?;

    let cal_id = if let Some(cal) = &spec.calibration {
        Some(meta.push_calibration(11, cal)?)
    } else {
        None
    };
    let target_frame_id = if let Some(tf) = &spec.target_frame {
        Some(meta.push_frame(12, tf)?)
    } else {
        None
    };

    // Stream 0 static auxiliary arrays precede stream 1 temporal entries (`0x0105` < `0x0106` < `0x0107`).
    let mut static_entries: Vec<OwnedEntry> = Vec::new();
    let mut s208_fields = vec![OwnedField::crit(
        1,
        OwnedValue::Utf8(spec.role.as_str().to_owned()),
    )];
    if let Some(fc) = spec.center_frequency_hz {
        s208_fields.push(OwnedField::opt(2, OwnedValue::F64(vec![fc])));
    }

    if let Some(pos) = spec.element_positions_m {
        if pos.is_empty() {
            return Err(ProfileError::InvalidMetadata("empty element_positions"));
        }
        let idx = (static_entries.len() as u32) + 1;
        s208_fields.push(OwnedField::opt(3, OwnedValue::EntryRefs(vec![idx])));
        static_entries.push(push_aux_f32_vec3_entry(
            &mut meta,
            20,
            3,
            "transducer-element-positions",
            "m",
            pos,
        )?);
    }
    if let Some(cmap) = spec.channel_map {
        if cmap.is_empty() {
            return Err(ProfileError::InvalidMetadata("empty channel_map"));
        }
        let idx = (static_entries.len() as u32) + 1;
        s208_fields.push(OwnedField::opt(4, OwnedValue::EntryRefs(vec![idx])));
        let mut cm_bytes = vec![0u8; cmap.len() * 4];
        for (i, &ch) in cmap.iter().enumerate() {
            write_u32(&mut cm_bytes, i * 4, ch);
        }
        meta.push_descriptor(
            21,
            &DescriptorSpec {
                clock_id: None,
                frame_id: None,
                provenance_id: 3,
                medium_id: None,
                transform_id: None,
                calibration_id: None,
                quantity: "ultrasound-channel-map",
                units: "1",
                channels: Some(1),
                layout: Some("record-component"),
                scalar: Some(AuxScalar::I32),
                model_uri: "urn:qac:mode:0x0107:v1",
                sample_rate: None,
                valid_record_count: None,
                validity_mask_entry: None,
                source_entries: &[],
                extension_metadata: &[],
            },
        )?;
        static_entries.push(OwnedEntry {
            mode: MODE_AUX_ARRAY,
            flags: 1,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: cmap.len() as u64,
            stride: 4,
            descriptor_id: 21,
            payload: cm_bytes,
        });
    }
    if let Some(delays) = spec.transmit_delays_s {
        if delays.is_empty() {
            return Err(ProfileError::InvalidMetadata("empty transmit_delays_s"));
        }
        let idx = (static_entries.len() as u32) + 1;
        s208_fields.push(OwnedField::opt(5, OwnedValue::EntryRefs(vec![idx])));
        static_entries.push(push_aux_f64_scalar_entry(
            &mut meta,
            22,
            3,
            "transmit-delays-s",
            "s",
            delays,
        )?);
    }

    if let Some(phi0) = spec.demodulation_phase_rad {
        s208_fields.push(OwnedField::opt(6, OwnedValue::F64(vec![phi0])));
    }
    if let Some(prf) = spec.prf {
        s208_fields.push(OwnedField::opt(7, OwnedValue::U64(vec![prf.num])));
        s208_fields.push(OwnedField::opt(8, OwnedValue::U64(vec![prf.den])));
    }
    if let Some(dm) = spec.derating_model_uri {
        validate_identifier_str(dm, "derating_model_URI")?;
        s208_fields.push(OwnedField::opt(10, OwnedValue::Utf8(dm.to_owned())));
    }
    if let Some(ctx) = spec.exposure_context {
        validate_identifier_str(ctx, "exposure_context")?;
        s208_fields.push(OwnedField::opt(11, OwnedValue::Utf8(ctx.to_owned())));
    }
    if let Some(tm) = spec.thermal_model_uri {
        validate_identifier_str(tm, "thermal_model_URI")?;
        s208_fields.push(OwnedField::opt(12, OwnedValue::Utf8(tm.to_owned())));
    }
    if let Some(idig) = spec.instrument_digest {
        s208_fields.push(OwnedField::opt(14, OwnedValue::Bytes(idig.to_vec())));
    }
    if let Some(adig) = spec.authority_receipt_digest {
        s208_fields.push(OwnedField::opt(15, OwnedValue::Bytes(adig.to_vec())));
    }
    if let Some(tf_id) = target_frame_id {
        s208_fields.push(OwnedField::opt(16, OwnedValue::MetadataRefs(vec![tf_id])));
    }

    let mut temporal_entries: Vec<OwnedEntry> = Vec::new();
    let mut raw_entry_idx: Option<u32> = None;
    let mut raw_digest: Option<[u8; 32]> = None;

    if let Some(raw) = raw_source {
        let (ch, rate, scalar, qty, payload, rec_cnt, stride) = match raw {
            RawUltrasoundPayload::RawRfF32 {
                channels,
                sample_rate: rr,
                samples_interleaved,
            } => {
                if *channels == 0
                    || samples_interleaved.is_empty()
                    || samples_interleaved.len() % (*channels as usize) != 0
                {
                    return Err(ProfileError::InvalidPayloadLength);
                }
                let mut bytes = vec![0u8; samples_interleaved.len() * 4];
                write_f32_slice(&mut bytes, 0, samples_interleaved)?;
                let frames = (samples_interleaved.len() / (*channels as usize)) as u64;
                (*channels, *rr, AuxScalar::F32, "ultrasound-raw-rf", bytes, frames, *channels * 4)
            }
            RawUltrasoundPayload::BasebandIqComplexF32 {
                channels,
                sample_rate: iq_rate,
                iq_interleaved,
            } => {
                if *channels == 0
                    || iq_interleaved.is_empty()
                    || iq_interleaved.len() % (*channels as usize) != 0
                {
                    return Err(ProfileError::InvalidPayloadLength);
                }
                let mut bytes = vec![0u8; iq_interleaved.len() * 8];
                for (i, &[re, im]) in iq_interleaved.iter().enumerate() {
                    write_f32_finite(&mut bytes, i * 8, re)?;
                    write_f32_finite(&mut bytes, i * 8 + 4, im)?;
                }
                let frames = (iq_interleaved.len() / (*channels as usize)) as u64;
                (*channels, *iq_rate, AuxScalar::ComplexF32, "ultrasound-baseband-iq", bytes, frames, *channels * 8)
            }
        };
        let digest = *blake3::hash(&payload).as_bytes();
        raw_digest = Some(digest);
        let idx = (static_entries.len() + temporal_entries.len() + 1) as u32;
        raw_entry_idx = Some(idx);
        s208_fields.push(OwnedField::opt(17, OwnedValue::EntryRefs(vec![idx])));
        let ext_raw = [ACQ_BLOCK_ID];
        meta.push_descriptor(
            18,
            &DescriptorSpec {
                clock_id: Some(1),
                frame_id: Some(2),
                provenance_id: 3,
                medium_id: None,
                transform_id: None,
                calibration_id: cal_id,
                quantity: qty,
                units: "Pa",
                channels: Some(ch),
                layout: Some("frame-channel"),
                scalar: Some(scalar),
                model_uri: "urn:qac:mode:0x0105:ultrasound-raw:v1",
                sample_rate: Some(rate),
                valid_record_count: None,
                validity_mask_entry: None,
                source_entries: &[],
                extension_metadata: &ext_raw,
            },
        )?;
        temporal_entries.push(OwnedEntry {
            mode: MODE_RAW_RF_IQ,
            flags: 3,
            stream_id: 1,
            frame_id: 2,
            start_tick: 0,
            time_count: rec_cnt,
            spatial_key: 0,
            record_count: rec_cnt,
            stride,
            descriptor_id: 18,
            payload,
        });
    }

    meta.push_provenance(3, provenance)?;
    let beam_prov_id = if let Some(rdig) = raw_digest {
        let digs = [rdig];
        meta.push_provenance(
            13,
            &ProvenanceSpec {
                role: provenance.role,
                source_digests: &digs,
                algorithm: provenance.algorithm,
                algorithm_version: provenance.algorithm_version,
                instrument_uri: provenance.instrument_uri,
                authority_receipt_digest: provenance.authority_receipt_digest,
                lineage: provenance.lineage,
            },
        )?
    } else {
        3
    };

    let mask_entry_idx = if let Some(mb) = validity_mask {
        let idx = (static_entries.len() + temporal_entries.len() + 1) as u32;
        temporal_entries.push(push_validity_mask_entry(&mut meta, 7, 3, 1, 0, 1, mb)?);
        Some(idx)
    } else {
        None
    };

    meta.push_block(OwnedBlock {
        id: ACQ_BLOCK_ID,
        schema_type: 208,
        revision: 1,
        critical: true,
        fields: s208_fields,
    })?;

    let shape = [records.len() as u64];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: "urn:qac:model:ultrasound-beam-summary:v1",
            model_version: "1",
            discretization: Some("beam-summary-focus"),
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: Some(sample_rate),
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &shape,
            component_names: "pos,fc,pr,ispta,mi,ti,iq,v_axial,harmonic",
        },
    )?;

    let valid_count = validity_mask.map(|m| m.iter().filter(|&&b| b == 1).count() as u64);
    let src_entries: Vec<u32> = raw_entry_idx.into_iter().collect();
    let ext = [ACQ_BLOCK_ID, 5u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: Some(1),
            frame_id: Some(2),
            provenance_id: beam_prov_id,
            medium_id: None,
            transform_id: None,
            calibration_id: cal_id,
            quantity: "ultrasound-beam-summary",
            units: "mixed:mode-registry",
            channels: None,
            layout: None,
            scalar: None,
            model_uri: "urn:qac:mode:0x0107:v1",
            sample_rate: Some(sample_rate),
            valid_record_count: valid_count,
            validity_mask_entry: mask_entry_idx,
            source_entries: &src_entries,
            extension_metadata: &ext,
        },
    )?;

    temporal_entries.push(OwnedEntry {
        mode: MODE_ULTRASOUND_SUMMARY,
        flags: 3,
        stream_id: 1,
        frame_id: 2,
        start_tick: 0,
        time_count: 1,
        spatial_key: 0,
        record_count: records.len() as u64,
        stride: ULTRASOUND_SUMMARY_STRIDE,
        descriptor_id: 6,
        payload: main_payload,
    });

    let mut all_entries = static_entries;
    all_entries.extend(temporal_entries);
    assemble_qac_v1(&meta, &all_entries, limits)
}

/// Decoded passive Q07 ultrasound view retaining raw RF links, geometry, and derived metrics.
pub struct UltrasoundDocumentView<'a> {
    pub records: UltrasoundChunkView<'a>,
    pub role: UltrasoundRole,
    pub center_frequency_hz: Option<f64>,
    pub demodulation_phase_rad: Option<f64>,
    pub exposure_context: Option<&'a str>,
    pub thermal_model_uri: Option<&'a str>,
    pub element_positions_m: Option<Vec<[f32; 3]>>,
    pub channel_map: Option<Vec<u32>>,
    pub transmit_delays_s: Option<Vec<f64>>,
    pub raw_source_bytes: Option<&'a [u8]>,
}

/// Admits and passively decodes a `mode 0x0107` ultrasound beam summary entry.
/// Preserves valid high `MI`/`ISPTA` physical measurements without clinical veto,
/// while rejecting missing `thermal_model_URI` / `exposure_context` whenever `TI` valid bit is set.
pub fn read_ultrasound_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<UltrasoundDocumentView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let (_e, desc, payload, mask) = env.check_structured_entry(
        entry_index,
        MODE_ULTRASOUND_SUMMARY,
        ULTRASOUND_SUMMARY_STRIDE,
        false,
        Some(205),
    )?;
    let s208 = env
        .extension_block(desc, 208)?
        .ok_or(ProfileError::MissingRequiredSchema(208))?;
    let role = UltrasoundRole::parse(field_str(field_req(s208, 1)?)?)?;
    let center_frequency_hz = field_opt(s208, 2).map(field_f64).transpose()?;
    let demodulation_phase_rad = field_opt(s208, 6).map(field_f64).transpose()?;
    let exposure_context = field_opt(s208, 11).map(field_str).transpose()?;
    let thermal_model_uri = field_opt(s208, 12).map(field_str).transpose()?;

    let element_positions_m = if let Some(f) = field_opt(s208, 3) {
        let b = env.verify_chunk(field_u32(f)?)?;
        if b.len() % 12 != 0 {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let mut pts = Vec::with_capacity(b.len() / 12);
        for i in 0..b.len() / 12 {
            pts.push(read_f32_3(b, i * 12)?);
        }
        Some(pts)
    } else {
        None
    };

    let channel_map = if let Some(f) = field_opt(s208, 4) {
        let b = env.verify_chunk(field_u32(f)?)?;
        if b.len() % 4 != 0 {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let mut ch = Vec::with_capacity(b.len() / 4);
        for i in 0..b.len() / 4 {
            ch.push(read_u32(b, i * 4)?);
        }
        Some(ch)
    } else {
        None
    };

    let transmit_delays_s = if let Some(f) = field_opt(s208, 5) {
        let b = env.verify_chunk(field_u32(f)?)?;
        if b.len() % 8 != 0 {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let mut d = Vec::with_capacity(b.len() / 8);
        for i in 0..b.len() / 8 {
            let v = f64::from_le_bytes([
                b[i * 8],
                b[i * 8 + 1],
                b[i * 8 + 2],
                b[i * 8 + 3],
                b[i * 8 + 4],
                b[i * 8 + 5],
                b[i * 8 + 6],
                b[i * 8 + 7],
            ]);
            d.push(require_finite_f64(v)?);
        }
        Some(d)
    } else {
        None
    };

    let raw_source_bytes = if let Some(f) = field_opt(s208, 17) {
        let raw_idx = field_u32(f)?;
        let raw_entry = env.entry(raw_idx)?;
        if raw_entry.mode() != MODE_RAW_RF_IQ
            && raw_entry.mode() != MODE_PCM_PRESERVATION
            && raw_entry.mode() != MODE_OPAQUE_SOURCE
        {
            return Err(ProfileError::InvalidMode(raw_entry.mode()));
        }
        Some(env.verify_chunk(raw_idx)?)
    } else {
        None
    };

    let chunk = UltrasoundChunkView::new(payload, mask);
    let expected_ref = s208.id();
    for i in 0..chunk.len() {
        let rec = chunk.get(i)?;
        if rec.acquisition_ref != expected_ref {
            return Err(ProfileError::InvalidMetadata("acquisition_ref disagrees with Schema 208 ID"));
        }
        let ext_valid = mask.map_or(true, |m| m[i] != 0);
        if ext_valid
            && (rec.validity_bits & US_VALID_TI != 0)
            && (thermal_model_uri.is_none() || exposure_context.is_none())
        {
            return Err(ProfileError::MissingThermalModel(
                "TI bit set without thermal_model_URI and exposure_context",
            ));
        }
    }

    Ok(UltrasoundDocumentView {
        records: chunk,
        role,
        center_frequency_hz,
        demodulation_phase_rad,
        exposure_context,
        thermal_model_uri,
        element_positions_m,
        channel_map,
        transmit_delays_s,
        raw_source_bytes,
    })
}

/// Finds the 1-based directory index of the first `mode 0x0107` ultrasound beam summary
/// entry (64-byte stride + `Schema 208`), skipping `mode 0x0107` auxiliary array entries.
pub fn find_ultrasound_summary_entry_index(
    bytes: &[u8],
    limits: ProfileLimits,
) -> Result<u32, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    for idx in 1..=env.view().entry_count() {
        let e = env.entry(idx)?;
        if e.mode() == MODE_ULTRASOUND_SUMMARY && e.stride() == ULTRASOUND_SUMMARY_STRIDE {
            let desc = env.descriptor_for_entry(e)?;
            if env.extension_block(desc, 208)?.is_some() {
                return Ok(idx);
            }
        }
    }
    Err(ProfileError::MissingRequiredSchema(208))
}
