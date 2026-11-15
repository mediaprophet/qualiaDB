//! Helper types and migration loaders for Q06 Mode `0x0105` perceptual features.

use super::*;

/// Zero-heap borrowed view over verified `0x0105` perceptual feature vectors.
pub struct PerceptualChunkView<'a> {
    bytes: &'a [u8],
    feature_count: usize,
    mask: Option<&'a [u8]>,
}

impl<'a> PerceptualChunkView<'a> {
    pub fn frame_count(&self) -> usize {
        self.bytes.len() / (self.feature_count * 4)
    }

    pub fn feature_count(&self) -> usize {
        self.feature_count
    }

    pub fn read_frame(&self, index: usize, out: &mut [f32]) -> Result<(), ProfileError> {
        if out.len() != self.feature_count {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let stride = self.feature_count * 4;
        let start = index.checked_mul(stride).ok_or(ProfileError::Overflow)?;
        let end = start.checked_add(stride).ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("perceptual frame"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        for (i, dst) in out.iter_mut().enumerate() {
            let v = read_f32_finite(slice, i * 4)?;
            require_zero_when_masked(ext_valid, v)?;
            *dst = v;
        }
        Ok(())
    }
}

pub struct PerceptualDocumentView<'a> {
    pub chunk: PerceptualChunkView<'a>,
    pub scale: PerceptualScale,
    pub reference_pressure_pa: Option<f64>,
    pub gating_profile: Option<&'a str>,
    pub centers_hz: Option<Vec<f32>>,
    pub filter_coefficients: Option<Vec<f32>>,
    pub channel_weights: Option<Vec<f32>>,
    pub units: &'a str,
}

pub fn read_perceptual_features_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<PerceptualDocumentView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let e = env.entry(entry_index)?;
    if e.mode() != MODE_PERCEPTUAL_FEATURES {
        return Err(ProfileError::InvalidMode(e.mode()));
    }
    let desc = env.descriptor_for_entry(e)?;
    let s203 = env
        .extension_block(desc, 203)?
        .ok_or(ProfileError::MissingRequiredSchema(203))?;
    let bands = field_u32(field_req(s203, 3)?)?;
    let scale = PerceptualScale::parse(field_str(field_req(s203, 4)?)?)?;
    let units = field_str(field_req(desc, 8)?)?;
    if scale == PerceptualScale::DbSpl && units != "dB-SPL" {
        return Err(ProfileError::InvalidDomainConversion("dB-SPL scale must declare dB-SPL units"));
    }
    if scale == PerceptualScale::Lufs && units != "LUFS" {
        return Err(ProfileError::InvalidDomainConversion("LUFS scale must declare LUFS units"));
    }
    let reference_pressure_pa = field_opt(s203, 8).map(field_f64).transpose()?;
    if scale == PerceptualScale::DbSpl && reference_pressure_pa.is_none() {
        return Err(ProfileError::InvalidMetadata("dB-SPL requires reference_pressure_pa"));
    }
    let gating_profile = field_opt(s203, 10).map(field_str).transpose()?;
    if scale == PerceptualScale::Lufs && gating_profile.is_none() {
        return Err(ProfileError::InvalidMetadata("LUFS requires BS.1770 gating_profile"));
    }
    let expected_stride = bands.checked_mul(4).ok_or(ProfileError::Overflow)?;
    if e.stride() != expected_stride {
        return Err(ProfileError::InvalidStride {
            expected: expected_stride,
            actual: e.stride(),
        });
    }
    let s205 = env
        .extension_block(desc, 205)?
        .ok_or(ProfileError::MissingRequiredSchema(205))?;
    let shape = field_u64_vec(field_req(s205, 14)?)?;
    if shape.len() != 2 || shape[0] != e.record_count() || shape[1] != u64::from(bands) {
        return Err(ProfileError::InvalidMetadata("perceptual tensor_shape mismatch"));
    }
    let payload = env.verify_chunk(entry_index)?;
    let mask = env.read_validity_mask(desc, e.record_count())?;

    let read_opt_f32_aux = |key: u16| -> Result<Option<Vec<f32>>, ProfileError> {
        let Some(f) = field_opt(s203, key) else {
            return Ok(None);
        };
        let b = env.verify_chunk(field_u32(f)?)?;
        if b.len() % 4 != 0 {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let mut v = Vec::with_capacity(b.len() / 4);
        for i in 0..b.len() / 4 {
            v.push(read_f32_finite(b, i * 4)?);
        }
        Ok(Some(v))
    };

    let centers_hz = read_opt_f32_aux(5)?;
    let filter_coefficients = read_opt_f32_aux(6)?;
    let channel_weights = read_opt_f32_aux(9)?;

    let chunk = PerceptualChunkView {
        bytes: payload,
        feature_count: bands as usize,
        mask,
    };
    let mut scratch = vec![0.0f32; bands as usize];
    for i in 0..chunk.frame_count() {
        chunk.read_frame(i, &mut scratch)?;
    }
    Ok(PerceptualDocumentView {
        chunk,
        scale,
        reference_pressure_pa,
        gating_profile,
        centers_hz,
        filter_coefficients,
        channel_weights,
        units,
    })
}

/// Migrates a legacy `Q4AU v2` container preserving exact original bytes (`mode 0x0104`),
/// plane-0 spectra (`mode 0x0101` under `legacy-unknown`), mel & MFCC planes (`mode 0x0105`),
/// and `Schema 209 LegacyRetention` lineage without fabricating window/hop metadata.
pub fn migrate_legacy_q4au_v2_features(
    legacy_bytes: &[u8],
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    let header = parse_sidecar_header(legacy_bytes)
        .ok_or(ProfileError::LegacyFormat("invalid Q4AU v1/v2 header"))?;
    if header.version != AudioSpectralSidecarHeader::VERSION_V2 || header.sample_rate == 0 {
        return Err(ProfileError::LegacyFormat("expected valid Q4AU v2 with sample_rate > 0"));
    }
    let frames = usize::try_from(header.frame_count).map_err(|_| ProfileError::Overflow)?;
    let bins = usize::try_from(header.bin_count).map_err(|_| ProfileError::Overflow)?;
    let hdr_sz = std::mem::size_of::<AudioSpectralSidecarHeader>();
    let plane0_bytes = frames
        .checked_mul(bins)
        .and_then(|v| v.checked_mul(4))
        .ok_or(ProfileError::Overflow)?;
    let plane0_end = hdr_sz.checked_add(plane0_bytes).ok_or(ProfileError::Overflow)?;
    let sub_sz = std::mem::size_of::<SpectralV2SubHeader>();
    let sub_end = plane0_end.checked_add(sub_sz).ok_or(ProfileError::Overflow)?;
    let sub_slice = legacy_bytes
        .get(plane0_end..sub_end)
        .ok_or(ProfileError::LegacyFormat("truncated Q4AU v2 subheader"))?;
    let sub = bytemuck::pod_read_unaligned::<SpectralV2SubHeader>(sub_slice);
    if sub.magic != SPECTRAL_V2_SUBHEADER_MAGIC || sub.n_mel == 0 || sub.n_mfcc == 0 {
        return Err(ProfileError::LegacyFormat("invalid Q4M2 subheader"));
    }
    let n_mel = usize::try_from(sub.n_mel).map_err(|_| ProfileError::Overflow)?;
    let n_mfcc = usize::try_from(sub.n_mfcc).map_err(|_| ProfileError::Overflow)?;
    let mel_bytes = frames.checked_mul(n_mel).and_then(|v| v.checked_mul(4)).ok_or(ProfileError::Overflow)?;
    let mfcc_bytes = frames.checked_mul(n_mfcc).and_then(|v| v.checked_mul(4)).ok_or(ProfileError::Overflow)?;
    let mel_start = sub_end;
    let mel_end = mel_start.checked_add(mel_bytes).ok_or(ProfileError::Overflow)?;
    let mfcc_end = mel_end.checked_add(mfcc_bytes).ok_or(ProfileError::Overflow)?;
    let plane0 = legacy_bytes.get(hdr_sz..plane0_end).ok_or(ProfileError::LegacyFormat("truncated plane0"))?;
    let mel_slice = legacy_bytes.get(mel_start..mel_end).ok_or(ProfileError::LegacyFormat("truncated mel"))?;
    let mfcc_slice = legacy_bytes.get(mel_end..mfcc_end).ok_or(ProfileError::LegacyFormat("truncated mfcc"))?;
    for c in [plane0, mel_slice, mfcc_slice] {
        for i in 0..c.len() / 4 {
            let _ = read_f32_finite(c, i * 4)?;
        }
    }

    let orig_digest: [u8; 32] = *blake3::hash(legacy_bytes).as_bytes();
    let rate = RationalRate::new(u64::from(header.sample_rate), 1)?;
    assemble_legacy_v2_ordered(legacy_bytes, orig_digest, rate, header.bin_count, frames, bins, sub, plane0, mel_slice, mfcc_slice, limits)
}

#[allow(clippy::too_many_arguments)]
fn assemble_legacy_v2_ordered(
    legacy_bytes: &[u8],
    orig_digest: [u8; 32],
    rate: RationalRate,
    bin_count: u32,
    frames: usize,
    bins: usize,
    sub: SpectralV2SubHeader,
    plane0: &[u8],
    mel_slice: &[u8],
    mfcc_slice: &[u8],
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, &ClockSpec { scale: 0, epoch_seconds: 0, epoch_nanoseconds: 0, ticks_per_second: rate })?;
    let digests = [orig_digest];
    meta.push_provenance(2, &ProvenanceSpec {
        role: 2, source_digests: &digests, algorithm: Some("urn:qac:migration:q4au-v2"),
        algorithm_version: Some("1"), instrument_uri: None, authority_receipt_digest: None,
        lineage: Some("lossless Q4AU v2 mel/mfcc extraction with opaque original bytes"),
    })?;
    meta.push_provenance(3, &ProvenanceSpec {
        role: 4, source_digests: &[], algorithm: None, algorithm_version: None,
        instrument_uri: None, authority_receipt_digest: None, lineage: Some("unparsed original Q4AU v2"),
    })?;
    meta.push_block(OwnedBlock {
        id: 4, schema_type: 201, revision: 1, critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("legacy-unknown".to_owned())),
            OwnedField::crit(13, OwnedValue::U32(vec![bin_count])),
        ],
    })?;
    meta.push_block(OwnedBlock {
        id: 5, schema_type: 209, revision: 1, critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("Q4AU".to_owned())),
            OwnedField::crit(2, OwnedValue::U32(vec![2])),
            OwnedField::crit(3, OwnedValue::EntryRefs(vec![1])),
            OwnedField::crit(4, OwnedValue::Bytes(orig_digest.to_vec())),
            OwnedField::crit(5, OwnedValue::Utf8("urn:qac:migration:q4au-v2".to_owned())),
            OwnedField::crit(6, OwnedValue::Utf8("1".to_owned())),
            OwnedField::opt(7, OwnedValue::Utf8("plane0,n_mel,n_mfcc,sample_rate".to_owned())),
            OwnedField::opt(8, OwnedValue::Utf8("fft_size,hop_samples,window".to_owned())),
        ],
    })?;
    let stft_shape = [frames as u64, 1, bins as u64];
    meta.push_model_grid(6, &ModelGridSpec {
        model_uri: "urn:qac:model:legacy-unknown-spectrum:v1", model_version: "1", discretization: None,
        component_offsets_xyz: &[], component_time_offset_num: &[], component_time_offset_den: &[],
        timestep: None, tolerance_abs: None, tolerance_rel: None, tensor_shape: &stft_shape, component_names: "mag",
    })?;
    meta.push_descriptor(7, &DescriptorSpec {
        clock_id: Some(1), frame_id: None, provenance_id: 2, medium_id: None, transform_id: Some(4),
        calibration_id: None, quantity: "spectral-magnitude", units: "1", channels: Some(1),
        layout: Some("frame-channel-bin"), scalar: Some(AuxScalar::F32), model_uri: "urn:qac:mode:0x0101:legacy:v1",
        sample_rate: Some(rate), valid_record_count: None, validity_mask_entry: None, source_entries: &[1], extension_metadata: &[5, 6],
    })?;
    meta.push_descriptor(8, &DescriptorSpec {
        clock_id: None, frame_id: None, provenance_id: 3, medium_id: None, transform_id: None,
        calibration_id: None, quantity: "opaque-original", units: "1", channels: Some(1), layout: Some("byte"),
        scalar: Some(AuxScalar::U8), model_uri: "urn:qac:mode:0x0104:opaque-original:v1", sample_rate: None,
        valid_record_count: None, validity_mask_entry: None, source_entries: &[], extension_metadata: &[],
    })?;
    meta.push_block(OwnedBlock {
        id: 9, schema_type: 203, revision: 1, critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("urn:qac:legacy:q4au-v2-mel".to_owned())),
            OwnedField::crit(2, OwnedValue::Utf8("1".to_owned())),
            OwnedField::crit(3, OwnedValue::U32(vec![sub.n_mel])),
            OwnedField::crit(4, OwnedValue::Utf8("mel".to_owned())),
        ],
    })?;
    let mel_shape = [frames as u64, sub.n_mel as u64];
    meta.push_model_grid(10, &ModelGridSpec {
        model_uri: "urn:qac:legacy:q4au-v2-mel", model_version: "1", discretization: None,
        component_offsets_xyz: &[], component_time_offset_num: &[], component_time_offset_den: &[],
        timestep: None, tolerance_abs: None, tolerance_rel: None, tensor_shape: &mel_shape, component_names: "mel",
    })?;
    meta.push_descriptor(11, &DescriptorSpec {
        clock_id: Some(1), frame_id: None, provenance_id: 2, medium_id: None, transform_id: None,
        calibration_id: None, quantity: "perceptual-mel", units: "mel", channels: Some(1),
        layout: Some("frame-feature"), scalar: Some(AuxScalar::F32), model_uri: "urn:qac:mode:0x0105:v1",
        sample_rate: Some(rate), valid_record_count: None, validity_mask_entry: None, source_entries: &[1], extension_metadata: &[5, 9, 10],
    })?;
    meta.push_block(OwnedBlock {
        id: 12, schema_type: 203, revision: 1, critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("urn:qac:legacy:q4au-v2-mfcc".to_owned())),
            OwnedField::crit(2, OwnedValue::Utf8("1".to_owned())),
            OwnedField::crit(3, OwnedValue::U32(vec![sub.n_mfcc])),
            OwnedField::crit(4, OwnedValue::Utf8("MFCC".to_owned())),
        ],
    })?;
    let mfcc_shape = [frames as u64, sub.n_mfcc as u64];
    meta.push_model_grid(13, &ModelGridSpec {
        model_uri: "urn:qac:legacy:q4au-v2-mfcc", model_version: "1", discretization: None,
        component_offsets_xyz: &[], component_time_offset_num: &[], component_time_offset_den: &[],
        timestep: None, tolerance_abs: None, tolerance_rel: None, tensor_shape: &mfcc_shape, component_names: "mfcc",
    })?;
    meta.push_descriptor(14, &DescriptorSpec {
        clock_id: Some(1), frame_id: None, provenance_id: 2, medium_id: None, transform_id: None,
        calibration_id: None, quantity: "perceptual-mfcc", units: "1", channels: Some(1),
        layout: Some("frame-feature"), scalar: Some(AuxScalar::F32), model_uri: "urn:qac:mode:0x0105:v1",
        sample_rate: Some(rate), valid_record_count: None, validity_mask_entry: None, source_entries: &[1], extension_metadata: &[5, 12, 13],
    })?;

    let entries = vec![
        OwnedEntry {
            mode: MODE_OPAQUE_SOURCE, flags: 1, stream_id: 0, frame_id: 0, start_tick: 0,
            time_count: 0, spatial_key: 0, record_count: legacy_bytes.len() as u64, stride: 1,
            descriptor_id: 8, payload: legacy_bytes.to_vec(),
        },
        OwnedEntry {
            mode: MODE_STFT_PRESERVATION, flags: 3, stream_id: 1, frame_id: 0, start_tick: 0,
            time_count: frames as u64, spatial_key: 0, record_count: frames as u64, stride: (bins * 4) as u32,
            descriptor_id: 7, payload: plane0.to_vec(),
        },
        OwnedEntry {
            mode: MODE_PERCEPTUAL_FEATURES, flags: 3, stream_id: 1, frame_id: 0, start_tick: 0,
            time_count: frames as u64, spatial_key: 0, record_count: frames as u64, stride: sub.n_mel * 4,
            descriptor_id: 11, payload: mel_slice.to_vec(),
        },
        OwnedEntry {
            mode: MODE_PERCEPTUAL_FEATURES, flags: 3, stream_id: 1, frame_id: 0, start_tick: 0,
            time_count: frames as u64, spatial_key: 0, record_count: frames as u64, stride: sub.n_mfcc * 4,
            descriptor_id: 14, payload: mfcc_slice.to_vec(),
        },
    ];
    assemble_qac_v1(&meta, &entries, limits)
}
