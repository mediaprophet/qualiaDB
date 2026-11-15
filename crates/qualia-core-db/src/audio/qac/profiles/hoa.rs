//! Q05 Mode `0x0102`: Higher-Order Ambisonics temporal samples (`4 * (L + 1)^2` B, `L = 0..=3`).
//!
//! Strictly distinguishes temporal HOA frames (`mode 0x0102`) from spectral HOA bins (`mode 0x0101`),
//! supports `ACN` vs explicit `FuMa` ordering and `SN3D`/`N3D`/`maxN` normalization without silent relabeling,
//! uses `+X` forward / `+Y` left / `+Z` up Cartesian conventions, and implements exact `f64` order 0..=3 rotations.

use std::f64::consts::PI;

use super::envelope::*;
use super::types::*;

pub const HOA_MAX_ORDER: u8 = 3;
pub const HOA_MAX_CHANNELS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoaOrdering {
    Acn,
    FuMa,
}

impl HoaOrdering {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Acn => "ACN",
            Self::FuMa => "FuMa",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ProfileError> {
        match s {
            "ACN" => Ok(Self::Acn),
            "FuMa" => Ok(Self::FuMa),
            _ => Err(ProfileError::InvalidMetadata("hoa_ordering")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoaNormalization {
    Sn3d,
    N3d,
    MaxN,
}

impl HoaNormalization {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sn3d => "SN3D",
            Self::N3d => "N3D",
            Self::MaxN => "maxN",
        }
    }

    pub fn parse(s: &str) -> Result<Self, ProfileError> {
        match s {
            "SN3D" => Ok(Self::Sn3d),
            "N3D" => Ok(Self::N3d),
            "maxN" => Ok(Self::MaxN),
            _ => Err(ProfileError::InvalidMetadata("hoa_normalization")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoaDomain {
    /// Mode `0x0102` temporal sample frames (`[frames, channels]`).
    TimeSample,
    /// Mode `0x0101` spectral STFT frames (`[frames, channels, bins]`); disallowed on `0x0102`.
    Spectral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoaDirectionConvention {
    /// Vector from listener origin pointing toward sound source (canonical IEM/SOFA).
    ListenerToSource,
    /// Wave propagation direction vector (`k_hat`).
    PlaneWavePropagation,
}

impl HoaDirectionConvention {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListenerToSource => "listener-to-source",
            Self::PlaneWavePropagation => "plane-wave-propagation",
        }
    }
}

/// Validates order (`0..=3`) and returns `(order + 1)^2` channel count and byte stride.
#[inline]
pub fn hoa_channel_count(order: u8) -> Result<u32, ProfileError> {
    if order > HOA_MAX_ORDER {
        return Err(ProfileError::InvalidMetadata("hoa_order > 3 not supported in v1"));
    }
    let n = u32::from(order) + 1;
    Ok(n * n)
}

#[inline]
pub fn hoa_stride_bytes(order: u8) -> Result<u32, ProfileError> {
    Ok(hoa_channel_count(order)? * 4)
}

/// Real Spherical Harmonic basis values for `ACN / SN3D` (`L = 0..=3`) using
/// `+X` forward, `+Y` left, `+Z` up (`dir` must be finite unit vector in Cartesian XYZ).
pub fn evaluate_acn_sn3d_dir(order: u8, dir_xyz: [f64; 3]) -> Result<[f64; HOA_MAX_CHANNELS], ProfileError> {
    if order > HOA_MAX_ORDER {
        return Err(ProfileError::InvalidMetadata("hoa_order > 3"));
    }
    let norm = (dir_xyz[0] * dir_xyz[0] + dir_xyz[1] * dir_xyz[1] + dir_xyz[2] * dir_xyz[2]).sqrt();
    if !norm.is_finite() || norm <= 0.0 {
        return Err(ProfileError::InvalidUnitVector);
    }
    let x = dir_xyz[0] / norm;
    let y = dir_xyz[1] / norm;
    let z = (dir_xyz[2] / norm).clamp(-1.0, 1.0);
    let az = y.atan2(x);
    let el = z.asin();

    let cos_el = el.cos();
    let sin_el = el.sin();
    let cos_az = az.cos();
    let sin_az = az.sin();
    let cos_2az = (2.0 * az).cos();
    let sin_2az = (2.0 * az).sin();
    let cos_3az = (3.0 * az).cos();
    let sin_3az = (3.0 * az).sin();
    let cos_el2 = cos_el * cos_el;
    let cos_el3 = cos_el2 * cos_el;
    let sin_el2 = sin_el * sin_el;

    let mut out = [0.0f64; HOA_MAX_CHANNELS];
    // Order 0: ACN 0 = W
    out[0] = 1.0;
    if order >= 1 {
        // Order 1: ACN 1 = Y, 2 = Z, 3 = X
        out[1] = sin_az * cos_el;
        out[2] = sin_el;
        out[3] = cos_az * cos_el;
    }
    if order >= 2 {
        let s3_2 = 0.75f64.sqrt(); // sqrt(3)/2
        let s3 = 3.0f64.sqrt();
        out[4] = s3_2 * sin_2az * cos_el2;
        out[5] = s3 * sin_az * sin_el * cos_el;
        out[6] = 0.5 * (3.0 * sin_el2 - 1.0);
        out[7] = s3 * cos_az * sin_el * cos_el;
        out[8] = s3_2 * cos_2az * cos_el2;
    }
    if order >= 3 {
        let s5_8 = (5.0 / 8.0f64).sqrt();
        let s15_2 = (15.0 / 2.0f64).sqrt();
        let s3_8 = (3.0 / 8.0f64).sqrt();
        out[9] = s5_8 * sin_3az * cos_el3;
        out[10] = s15_2 * sin_2az * sin_el * cos_el2;
        out[11] = s3_8 * sin_az * cos_el * (5.0 * sin_el2 - 1.0);
        out[12] = 0.5 * sin_el * (5.0 * sin_el2 - 3.0);
        out[13] = s3_8 * cos_az * cos_el * (5.0 * sin_el2 - 1.0);
        out[14] = s15_2 * cos_2az * sin_el * cos_el2;
        out[15] = s5_8 * cos_3az * cos_el3;
    }
    Ok(out)
}

/// Converts ACN channel coefficients between SN3D and N3D normalization: `N3D(l,m) = sqrt(2l+1) * SN3D(l,m)`.
pub fn convert_sn3d_n3d(
    order: u8,
    src: &[f32],
    from: HoaNormalization,
    to: HoaNormalization,
    out: &mut [f32],
) -> Result<(), ProfileError> {
    let ch = hoa_channel_count(order)? as usize;
    if src.len() != ch || out.len() != ch {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if src.iter().any(|v| !v.is_finite()) {
        return Err(ProfileError::NonFiniteFloat);
    }
    match (from, to) {
        (HoaNormalization::Sn3d, HoaNormalization::Sn3d)
        | (HoaNormalization::N3d, HoaNormalization::N3d) => {
            out.copy_from_slice(src);
            Ok(())
        }
        (HoaNormalization::Sn3d, HoaNormalization::N3d) => {
            for (acn, (&v, dst)) in src.iter().zip(out.iter_mut()).enumerate() {
                let l = (acn as f64).sqrt().floor();
                let scale = (2.0 * l + 1.0).sqrt();
                *dst = (f64::from(v) * scale) as f32;
            }
            Ok(())
        }
        (HoaNormalization::N3d, HoaNormalization::Sn3d) => {
            for (acn, (&v, dst)) in src.iter().zip(out.iter_mut()).enumerate() {
                let l = (acn as f64).sqrt().floor();
                let scale = 1.0 / (2.0 * l + 1.0).sqrt();
                *dst = (f64::from(v) * scale) as f32;
            }
            Ok(())
        }
        _ => Err(ProfileError::InvalidDomainConversion(
            "use explicit FuMa<->ACN helpers for MaxN/FuMa",
        )),
    }
}

/// Exact ACN/SN3D -> FuMa (MaxN) channel reorder & normalization (`order <= 3`).
pub fn acn_sn3d_to_fuma_maxn(order: u8, acn_sn3d: &[f32], fuma_out: &mut [f32]) -> Result<(), ProfileError> {
    let ch = hoa_channel_count(order)? as usize;
    if acn_sn3d.len() != ch || fuma_out.len() != ch {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if acn_sn3d.iter().any(|v| !v.is_finite()) {
        return Err(ProfileError::NonFiniteFloat);
    }
    let inv_sqrt2 = 0.5f64.sqrt();
    // Order 0: W
    fuma_out[0] = (f64::from(acn_sn3d[0]) * inv_sqrt2) as f32;
    if order >= 1 {
        // FuMa [W, X, Y, Z] <- ACN [0=W, 3=X, 1=Y, 2=Z]
        fuma_out[1] = acn_sn3d[3];
        fuma_out[2] = acn_sn3d[1];
        fuma_out[3] = acn_sn3d[2];
    }
    if order >= 2 {
        // FuMa [R, S, T, U, V] <- ACN [6, 7, 5, 8, 4]
        let s2_3 = (4.0 / 3.0f64).sqrt();
        fuma_out[4] = acn_sn3d[6];
        fuma_out[5] = (f64::from(acn_sn3d[7]) * s2_3) as f32;
        fuma_out[6] = (f64::from(acn_sn3d[5]) * s2_3) as f32;
        fuma_out[7] = (f64::from(acn_sn3d[8]) * s2_3) as f32;
        fuma_out[8] = (f64::from(acn_sn3d[4]) * s2_3) as f32;
    }
    if order >= 3 {
        // FuMa [K, L, M, N, O, P, Q] <- ACN [12, 13, 11, 14, 10, 15, 9]
        let f1 = (45.0 / 32.0f64).sqrt();
        let f2 = (9.0 / 5.0f64).sqrt();
        let f3 = (8.0 / 5.0f64).sqrt();
        fuma_out[9] = acn_sn3d[12];
        fuma_out[10] = (f64::from(acn_sn3d[13]) * f1) as f32;
        fuma_out[11] = (f64::from(acn_sn3d[11]) * f1) as f32;
        fuma_out[12] = (f64::from(acn_sn3d[14]) * f2) as f32;
        fuma_out[13] = (f64::from(acn_sn3d[10]) * f2) as f32;
        fuma_out[14] = (f64::from(acn_sn3d[15]) * f3) as f32;
        fuma_out[15] = (f64::from(acn_sn3d[9]) * f3) as f32;
    }
    Ok(())
}

/// Computes exact `(L+1)^2 x (L+1)^2` active rotation operator in `ACN / SN3D` for `q`.
///
/// Uses exact 4-point Gauss-Legendre in `sin(el)` x 8-point uniform azimuth (32 spherical nodes)
/// which integrates every polynomial of degree `<= 6` on `S^2` to machine precision (`~1e-15`).
pub fn rotate_acn_sn3d_frame(
    order: u8,
    q: &QuaternionXyzw,
    src: &[f32],
    out: &mut [f32],
) -> Result<(), ProfileError> {
    let ch = hoa_channel_count(order)? as usize;
    if src.len() != ch || out.len() != ch {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if src.iter().any(|v| !v.is_finite()) {
        return Err(ProfileError::NonFiniteFloat);
    }
    let sqrt30 = 30.0f64.sqrt();
    let x1 = ((15.0 - 2.0 * sqrt30) / 35.0).sqrt();
    let x2 = ((15.0 + 2.0 * sqrt30) / 35.0).sqrt();
    let w1 = (18.0 + sqrt30) / 36.0;
    let w2 = (18.0 - sqrt30) / 36.0;
    let gl_nodes = [(-x2, w2), (-x1, w1), (x1, w1), (x2, w2)];

    let mut d_mat = [0.0f64; HOA_MAX_CHANNELS * HOA_MAX_CHANNELS];
    let d_phi = 2.0 * PI / 8.0;

    for &(mu, w_mu) in &gl_nodes {
        let r_xy = (1.0 - mu * mu).max(0.0).sqrt();
        for k in 0..8 {
            let phi = (k as f64) * d_phi;
            let n_dir = [r_xy * phi.cos(), r_xy * phi.sin(), mu];
            let rot_dir = q.rotate_vec3(n_dir);
            let y_n = evaluate_acn_sn3d_dir(order, n_dir)?;
            let y_rot = evaluate_acn_sn3d_dir(order, rot_dir)?;
            let w_omega = w_mu * d_phi;

            for l in 0..=usize::from(order) {
                let start = l * l;
                let end = (l + 1) * (l + 1);
                let factor = ((2 * l + 1) as f64 / (4.0 * PI)) * w_omega;
                for mp in start..end {
                    for m in start..end {
                        d_mat[mp * HOA_MAX_CHANNELS + m] += factor * y_rot[mp] * y_n[m];
                    }
                }
            }
        }
    }

    for (mp, slot) in out.iter_mut().enumerate().take(ch) {
        let mut acc = 0.0f64;
        for (m, &v) in src.iter().enumerate() {
            acc += d_mat[mp * HOA_MAX_CHANNELS + m] * f64::from(v);
        }
        *slot = acc as f32;
    }
    Ok(())
}

/// Zero-heap borrowed view over verified `0x0102` temporal HOA sample frames.
pub struct HoaChunkView<'a> {
    bytes: &'a [u8],
    order: u8,
    channels: usize,
    stride: usize,
    mask: Option<&'a [u8]>,
}

impl<'a> HoaChunkView<'a> {
    pub fn frame_count(&self) -> usize {
        self.bytes.len() / self.stride
    }

    pub fn order(&self) -> u8 {
        self.order
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn read_frame(&self, index: usize, out: &mut [f32]) -> Result<(), ProfileError> {
        if out.len() != self.channels {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let start = index.checked_mul(self.stride).ok_or(ProfileError::Overflow)?;
        let end = start.checked_add(self.stride).ok_or(ProfileError::Overflow)?;
        let slice = self
            .bytes
            .get(start..end)
            .ok_or(ProfileError::OutOfBoundsIndex("hoa frame"))?;
        let ext_valid = self.mask.map_or(true, |m| m[index] != 0);
        for (i, dst) in out.iter_mut().enumerate() {
            let v = read_f32_finite(slice, i * 4)?;
            require_zero_when_masked(ext_valid, v)?;
            *dst = v;
        }
        Ok(())
    }
}

/// HOA stream descriptor metadata (`Schema 204` + frame/rate bindings).
#[derive(Debug, Clone)]
pub struct HoaProfileSpec<'a> {
    pub order: u8,
    pub ordering: HoaOrdering,
    pub normalization: HoaNormalization,
    pub direction_convention: HoaDirectionConvention,
    pub units: &'a str,
    pub sample_rate: RationalRate,
}

/// Writes canonical QAC v1 `mode 0x0102` temporal HOA stream.
pub fn write_hoa_temporal_qac(
    frames: &[&[f32]],
    spec: &HoaProfileSpec<'_>,
    clock: &ClockSpec,
    frame: &FrameSpec<'_>,
    provenance: &ProvenanceSpec<'_>,
    validity_mask: Option<&[u8]>,
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    if frames.is_empty() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    if spec.ordering == HoaOrdering::FuMa && spec.normalization == HoaNormalization::Sn3d {
        return Err(ProfileError::InvalidMetadata("FuMa cannot claim SN3D normalization"));
    }
    let ch = hoa_channel_count(spec.order)? as usize;
    let stride = ch * 4;
    if let Some(m) = validity_mask {
        if m.len() != frames.len() || m.iter().any(|&b| b > 1) {
            return Err(ProfileError::InvalidValidityBits);
        }
    }
    let mut payload = vec![0u8; frames.len() * stride];
    for (idx, row) in frames.iter().enumerate() {
        if row.len() != ch {
            return Err(ProfileError::InvalidPayloadLength);
        }
        let ext_valid = validity_mask.map_or(true, |m| m[idx] != 0);
        for (c, &v) in row.iter().enumerate() {
            if !v.is_finite() {
                return Err(ProfileError::NonFiniteFloat);
            }
            require_zero_when_masked(ext_valid, v)?;
            write_f32_finite(&mut payload, idx * stride + c * 4, v)?;
        }
    }

    let mut meta = MetadataBuilder::new();
    meta.push_clock(1, clock)?;
    meta.push_frame(2, frame)?;
    meta.push_provenance(3, provenance)?;
    meta.push_block(OwnedBlock {
        id: 4,
        schema_type: 204,
        revision: 1,
        critical: true,
        fields: vec![
            OwnedField::crit(1, OwnedValue::Utf8("HOA".to_owned())),
            OwnedField::crit(2, OwnedValue::U32(vec![u32::from(spec.order)])),
            OwnedField::crit(3, OwnedValue::Utf8(spec.ordering.as_str().to_owned())),
            OwnedField::crit(4, OwnedValue::Utf8(spec.normalization.as_str().to_owned())),
            OwnedField::opt(13, OwnedValue::MetadataRefs(vec![2])),
        ],
    })?;
    let shape = [frames.len() as u64, ch as u64];
    meta.push_model_grid(
        5,
        &ModelGridSpec {
            model_uri: spec.direction_convention.as_str(),
            model_version: "1",
            discretization: Some("hoa-time-domain"),
            component_offsets_xyz: &[],
            component_time_offset_num: &[],
            component_time_offset_den: &[],
            timestep: Some(spec.sample_rate),
            tolerance_abs: None,
            tolerance_rel: None,
            tensor_shape: &shape,
            component_names: spec.ordering.as_str(),
        },
    )?;

    let valid_count = validity_mask.map(|m| m.iter().filter(|&&b| b == 1).count() as u64);
    let mask_entry_idx = if validity_mask.is_some() { Some(2) } else { None };
    let ext = [4u32, 5u32];
    meta.push_descriptor(
        6,
        &DescriptorSpec {
            clock_id: Some(1),
            frame_id: Some(2),
            provenance_id: 3,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "hoa-temporal-coefficients",
            units: spec.units,
            channels: Some(ch as u32),
            layout: Some("time-channel"),
            scalar: Some(AuxScalar::F32),
            model_uri: "urn:qac:mode:0x0102:v1",
            sample_rate: Some(spec.sample_rate),
            valid_record_count: valid_count,
            validity_mask_entry: mask_entry_idx,
            source_entries: &[],
            extension_metadata: &ext,
        },
    )?;

    let mut entries = vec![OwnedEntry {
        mode: MODE_HOA_SPATIAL,
        flags: 3,
        stream_id: 1,
        frame_id: 2,
        start_tick: 0,
        time_count: frames.len() as u64,
        spatial_key: 0,
        record_count: frames.len() as u64,
        stride: stride as u32,
        descriptor_id: 6,
        payload,
    }];
    if let Some(mask) = validity_mask {
        entries.push(push_validity_mask_entry(
            &mut meta,
            7,
            3,
            1,
            0,
            frames.len() as u64,
            mask,
        )?);
    }
    assemble_qac_v1(&meta, &entries, limits)
}

pub struct HoaTemporalView<'a> {
    pub chunk: HoaChunkView<'a>,
    pub order: u8,
    pub ordering: HoaOrdering,
    pub normalization: HoaNormalization,
    pub domain: HoaDomain,
}

/// Admits and decodes `mode 0x0102` temporal HOA stream; rejects spectral `0x0101` conflation.
pub fn read_hoa_temporal_qac<'a>(
    bytes: &'a [u8],
    entry_index: u32,
    limits: ProfileLimits,
) -> Result<HoaTemporalView<'a>, ProfileError> {
    let env = AdmittedEnvelope::open_alloc(bytes, limits)?;
    let e = env.entry(entry_index)?;
    if e.mode() == MODE_STFT_PRESERVATION {
        return Err(ProfileError::InvalidDomainConversion(
            "mode 0x0101 is spectral HOA/STFT, not temporal HOA mode 0x0102",
        ));
    }
    if e.mode() != MODE_HOA_SPATIAL {
        return Err(ProfileError::InvalidMode(e.mode()));
    }
    let desc = env.descriptor_for_entry(e)?;
    if field_opt(desc, 5).is_some() {
        return Err(ProfileError::InvalidDomainConversion(
            "temporal HOA mode 0x0102 must not attach spectral Schema 201 transform",
        ));
    }
    let s204 = env
        .extension_block(desc, 204)?
        .ok_or(ProfileError::MissingRequiredSchema(204))?;
    if field_str(field_req(s204, 1)?)? != "HOA" {
        return Err(ProfileError::InvalidMetadata("Schema 204 representation must be HOA"));
    }
    let order_u32 = field_u32(field_req(s204, 2)?)?;
    let order = u8::try_from(order_u32).map_err(|_| ProfileError::InvalidMetadata("hoa_order"))?;
    let ordering = HoaOrdering::parse(field_str(field_req(s204, 3)?)?)?;
    let normalization = HoaNormalization::parse(field_str(field_req(s204, 4)?)?)?;
    let expected_ch = hoa_channel_count(order)?;
    let expected_stride = hoa_stride_bytes(order)?;
    if field_u32(field_req(desc, 9)?)? != expected_ch
        || field_u32(field_req(desc, 11)?)? != AuxScalar::F32 as u32
        || e.stride() != expected_stride
    {
        return Err(ProfileError::InvalidStride {
            expected: expected_stride,
            actual: e.stride(),
        });
    }
    let s205 = env
        .extension_block(desc, 205)?
        .ok_or(ProfileError::MissingRequiredSchema(205))?;
    let shape = field_u64_vec(field_req(s205, 14)?)?;
    if shape.len() != 2 || shape[0] != e.record_count() || shape[1] != u64::from(expected_ch) {
        return Err(ProfileError::InvalidMetadata("HOA tensor_shape mismatch"));
    }
    let payload = env.verify_chunk(entry_index)?;
    let mask = env.read_validity_mask(desc, e.record_count())?;
    let chunk = HoaChunkView {
        bytes: payload,
        order,
        channels: expected_ch as usize,
        stride: expected_stride as usize,
        mask,
    };
    let mut scratch_frame = vec![0.0f32; expected_ch as usize];
    for i in 0..chunk.frame_count() {
        chunk.read_frame(i, &mut scratch_frame)?;
    }
    Ok(HoaTemporalView {
        chunk,
        order,
        ordering,
        normalization,
        domain: HoaDomain::TimeSample,
    })
}
