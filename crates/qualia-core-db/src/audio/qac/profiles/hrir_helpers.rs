//! Auxiliary vector/scalar array entry pushers, legacy Q4AU v3 validation, and lossless migration.

use crate::audio::audio_spectral_sheet_v3::{
    parse_v3_header, Q4AuHeaderV3, SofaHrirEntry, Q4AU_V3_HEADER_BYTES, SOFA_DEFAULT_FIR_TAPS,
};

use super::*;

pub(crate) fn push_aux_f32_vec3_entry(
    meta: &mut MetadataBuilder,
    desc_id: u32,
    prov_id: u32,
    qty: &str,
    units: &str,
    vecs: &[[f32; 3]],
) -> Result<OwnedEntry, ProfileError> {
    let mut payload = vec![0u8; vecs.len() * 12];
    for (i, v) in vecs.iter().enumerate() {
        write_f32_slice(&mut payload, i * 12, v)?;
    }
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
            channels: Some(3),
            layout: Some("record-component"),
            scalar: Some(AuxScalar::F32),
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
        record_count: vecs.len() as u64,
        stride: 12,
        descriptor_id: desc_id,
        payload,
    })
}

pub(crate) fn push_aux_f64_scalar_entry(
    meta: &mut MetadataBuilder,
    desc_id: u32,
    prov_id: u32,
    qty: &str,
    units: &str,
    vals: &[f64],
) -> Result<OwnedEntry, ProfileError> {
    let mut payload = vec![0u8; vals.len() * 8];
    for (i, &v) in vals.iter().enumerate() {
        let f = require_finite_f64(v)?;
        payload[i * 8..(i + 1) * 8].copy_from_slice(&f.to_le_bytes());
    }
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
            channels: Some(1),
            layout: Some("record-component"),
            scalar: Some(AuxScalar::F64),
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
        record_count: vals.len() as u64,
        stride: 8,
        descriptor_id: desc_id,
        payload,
    })
}

// ---------------------------------------------------------------------------
// Legacy Q4AU v3 SOFA 64-tap validation & lossless migration
// ---------------------------------------------------------------------------

/// Strictly validated `Q4AU v3` legacy container parsed without assuming pointer alignment.
pub struct ValidatedLegacyQ4auV3 {
    pub header: Q4AuHeaderV3,
    pub sofa_entries: Vec<SofaHrirEntry>,
}

/// Parses and validates a legacy `Q4AU v3` file, rejecting truncation, overlap,
/// tap count mismatch, non-finite floats, and WASM32 multiplication overflow.
pub fn parse_legacy_q4au_v3(bytes: &[u8]) -> Result<ValidatedLegacyQ4auV3, ProfileError> {
    let header = parse_v3_header(bytes)
        .ok_or(ProfileError::LegacyFormat("invalid Q4AU v3 magic/version/header CRC"))?;
    if header.sample_rate == 0 {
        return Err(ProfileError::LegacyFormat("Q4AU v3 sample_rate must be > 0"));
    }
    let ch = header.hoa_channel_count();
    let spectral_bytes = (header.bin_count as usize)
        .checked_mul(header.frame_count as usize)
        .and_then(|v| v.checked_mul(ch))
        .and_then(|v| v.checked_mul(4))
        .ok_or(ProfileError::Overflow)?;
    let spectral_end = Q4AU_V3_HEADER_BYTES
        .checked_add(spectral_bytes)
        .ok_or(ProfileError::Overflow)?;
    if bytes.len() < spectral_end {
        return Err(ProfileError::LegacyFormat("Q4AU v3 truncated spectral payload"));
    }
    let dir_count = usize::try_from(header.sofa_direction_count).map_err(|_| ProfileError::Overflow)?;
    let mut sofa_entries = Vec::new();
    if dir_count > 0 {
        if header.sofa_fir_taps as usize != SOFA_DEFAULT_FIR_TAPS {
            return Err(ProfileError::LegacyFormat("Q4AU v3 sofa_fir_taps disagrees with 64-tap layout"));
        }
        let sofa_start = usize::try_from(header.sofa_block_offset).map_err(|_| ProfileError::Overflow)?;
        if sofa_start < spectral_end {
            return Err(ProfileError::LegacyFormat("Q4AU v3 SOFA block overlaps header/spectral payload"));
        }
        let entry_size = std::mem::size_of::<SofaHrirEntry>();
        let sofa_len = dir_count.checked_mul(entry_size).ok_or(ProfileError::Overflow)?;
        let sofa_end = sofa_start.checked_add(sofa_len).ok_or(ProfileError::Overflow)?;
        if bytes.len() < sofa_end {
            return Err(ProfileError::LegacyFormat("Q4AU v3 truncated SOFA table"));
        }
        sofa_entries.reserve(dir_count);
        for i in 0..dir_count {
            let off = sofa_start + i * entry_size;
            let e = bytemuck::pod_read_unaligned::<SofaHrirEntry>(&bytes[off..off + entry_size]);
            if !e.azimuth_deg.is_finite()
                || !e.elevation_deg.is_finite()
                || !e.itd_samples.is_finite()
                || e.left_fir.iter().any(|v| !v.is_finite())
                || e.right_fir.iter().any(|v| !v.is_finite())
            {
                return Err(ProfileError::NonFiniteFloat);
            }
            sofa_entries.push(e);
        }
    }
    Ok(ValidatedLegacyQ4auV3 {
        header,
        sofa_entries,
    })
}

/// Losslessly migrates a `Q4AU v3` 64-tap SOFA HRIR table to QAC v1 (`mode 0x0103`),
/// preserving bit-exact original taps, per-ear ITD offsets, spherical directions,
/// and exact unparsed original bytes (`mode 0x0104` + `Schema 209`).
pub fn migrate_legacy_q4au_v3_hrir(
    legacy_bytes: &[u8],
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    let parsed = parse_legacy_q4au_v3(legacy_bytes)?;
    if parsed.sofa_entries.is_empty() {
        return Err(ProfileError::LegacyFormat("Q4AU v3 has no SOFA HRIR entries"));
    }
    let orig_digest: [u8; 32] = *blake3::hash(legacy_bytes).as_bytes();
    let n = parsed.sofa_entries.len();
    let mut impulses = Vec::with_capacity(n * 2 * SOFA_DEFAULT_FIR_TAPS);
    let mut source_pos = Vec::with_capacity(n);
    let mut delays = Vec::with_capacity(n * 2);

    for e in &parsed.sofa_entries {
        impulses.extend_from_slice(&e.left_fir);
        impulses.extend_from_slice(&e.right_fir);
        let az = f64::from(e.azimuth_deg).to_radians();
        let el = f64::from(e.elevation_deg).to_radians();
        source_pos.push([
            (el.cos() * az.cos()) as f32,
            (el.cos() * az.sin()) as f32,
            el.sin() as f32,
        ]);
        delays.push(f64::from(e.itd_samples));
        delays.push(0.0);
    }

    let mut payload = vec![0u8; impulses.len() * 4];
    write_f32_slice(&mut payload, 0, &impulses)?;

    let mut meta = MetadataBuilder::new();
    meta.push_frame(2, &FrameSpec::local_default())?;
    let digests = [orig_digest];
    meta.push_provenance(
        3,
        &ProvenanceSpec {
            role: 2, // derived
            source_digests: &digests,
            algorithm: Some("urn:qac:migration:q4au-v3-hrir"),
            algorithm_version: Some("1"),
            instrument_uri: None,
            authority_receipt_digest: None,
            lineage: Some("migrated from Q4AU v3 preserving exact 64-tap FIR and unparsed original"),
        },
    )?;
    meta.push_provenance(
        8,
        &ProvenanceSpec {
            role: 4, // opaque-preserved
            source_digests: &[],
            algorithm: None,
            algorithm_version: None,
            instrument_uri: None,
            authority_receipt_digest: None,
            lineage: Some("unparsed original Q4AU v3 container"),
        },
    )?;

    let impulse_rate = RationalRate::new(u64::from(parsed.header.sample_rate), 1)?;
    let mut s204_fields = vec![
        OwnedField::crit(1, OwnedValue::Utf8("HRIR".to_owned())),
        OwnedField::opt(6, OwnedValue::EntryRefs(vec![3])),
        OwnedField::opt(7, OwnedValue::EntryRefs(vec![4])),
        OwnedField::crit(8, OwnedValue::U32(vec![SOFA_DEFAULT_FIR_TAPS as u32])),
        OwnedField::crit(9, OwnedValue::U32(vec![2])),
        OwnedField::crit(10, OwnedValue::U32(vec![n as u32])),
        OwnedField::opt(11, OwnedValue::EntryRefs(vec![2])),
        OwnedField::opt(12, OwnedValue::Utf8("SimpleFreeFieldHRIR".to_owned())),
        OwnedField::opt(13, OwnedValue::MetadataRefs(vec![2])),
        OwnedField::crit(17, OwnedValue::U64(vec![impulse_rate.num])),
        OwnedField::crit(18, OwnedValue::U64(vec![impulse_rate.den])),
        OwnedField::crit(19, OwnedValue::Utf8(LEGACY_DELAY_REFERENCE.to_owned())),
    ];
    let _ = &mut s204_fields;
    meta.push_block(OwnedBlock {
        id: 4,
        schema_type: 204,
        revision: 1,
        critical: true,
        fields: s204_fields,
    })?;

    let shape = [n as u64, 2, SOFA_DEFAULT_FIR_TAPS as u64];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: "urn:qac:model:q4au-v3-hrir:v1",
            model_version: "1",
            discretization: Some("discrete-fir-taps"),
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: None,
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &shape,
            component_names: "left_fir,right_fir",
        },
    )?;

    // Schema 209 LegacyRetention linking Entry 2 (0x0104 opaque original bytes).
    meta.push_block(OwnedBlock {
        id: 7,
        schema_type: 209,
        revision: 1,
        critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("Q4AU".to_owned())),
            OwnedField::crit(2, OwnedValue::U32(vec![3])),
            OwnedField::crit(3, OwnedValue::EntryRefs(vec![2])),
            OwnedField::crit(4, OwnedValue::Bytes(orig_digest.to_vec())),
            OwnedField::crit(5, OwnedValue::Utf8("urn:qac:migration:q4au-v3-hrir".to_owned())),
            OwnedField::crit(6, OwnedValue::Utf8("1".to_owned())),
            OwnedField::opt(
                7,
                OwnedValue::Utf8("sample_rate,sofa_direction_count,azimuth_deg,elevation_deg,itd_samples,left_fir,right_fir".to_owned()),
            ),
            OwnedField::opt(
                8,
                OwnedValue::Utf8("unknown_excitation_origin_delay_ref".to_owned()),
            ),
        ],
    })?;

    let ext = [4u32, 5u32, 7u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: None,
            frame_id: Some(2),
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "hrir-impulse-response",
            units: "1",
            channels: Some(2),
            layout: Some("measurement-receiver-tap"),
            scalar: Some(AuxScalar::F32),
            model_uri: "urn:qac:mode:0x0103:v1",
            sample_rate: None,
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[2],
            extension_metadata: &ext,
        },
    )?;

    meta.push_descriptor(
        9,
        &DescriptorSpec {
            clock_id: None,
            frame_id: None,
            provenance_id: 8,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "opaque-original",
            units: "1",
            channels: Some(1),
            layout: Some("byte"),
            scalar: Some(AuxScalar::U8),
            model_uri: "urn:qac:mode:0x0104:opaque-original:v1",
            sample_rate: None,
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[],
            extension_metadata: &[],
        },
    )?;

    let e_src_pos = push_aux_f32_vec3_entry(&mut meta, 10, 8, "source-positions", "1", &source_pos)?;
    let e_delays = push_aux_f64_scalar_entry(&mut meta, 11, 8, "delays-samples", "1", &delays)?;

    // Canonical directory order: stream_id=0, modes 0x0103 < 0x0104 < 0x0107 (desc 10 < desc 11).
    let entries = vec![
        OwnedEntry {
            mode: MODE_HRIR_IMPULSE,
            flags: 1,
            stream_id: 0,
            frame_id: 2,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: (n * 2) as u64,
            stride: (SOFA_DEFAULT_FIR_TAPS as u32) * 4,
            descriptor_id: 6,
            payload,
        },
        OwnedEntry {
            mode: MODE_OPAQUE_SOURCE,
            flags: 1,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: legacy_bytes.len() as u64,
            stride: 1,
            descriptor_id: 9,
            payload: legacy_bytes.to_vec(),
        },
        e_src_pos,
        e_delays,
    ];
    assemble_qac_v1(&meta, &entries, limits)
}
