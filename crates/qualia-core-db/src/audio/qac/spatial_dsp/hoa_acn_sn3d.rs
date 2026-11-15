//! Higher-Order Ambisonics (HOA) ACN / SN3D & N3D Encoding, Normalization, and Decoding
//! (`hoa_acn_sn3d.rs`).
//!
//! Adapts `etalon_signal::hoa` into QAC Mode `0x0102` (`hoa_spatial_audio`) and Schema 204
//! (`SpatialHrir`) container record and stream interfaces, plus optional GPU compute dispatch
//! via `etalon_wgsl_forge::numerical::audio::hoa_binaural`.

pub use etalon_signal::hoa::{
    hoa_decode_matrix_apply_into, HoaDecodeShape, HoaNormalization, HoaOrder, MAX_HOA_CHANNELS,
    MAX_HOA_ORDER,
};

#[cfg(feature = "gpu-runtime")]
pub use etalon_wgsl_forge::numerical::audio::hoa_binaural::{
    hoa_decode_cpu, hoa_decode_gpu_capacity, hoa_decode_with_context, HOA_DECODE_ENTRY,
    HOA_DECODE_WGSL,
};

/// Errors returned by QAC spatial DSP routines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialDspError {
    /// Input or output slice is empty when non-empty data is required.
    EmptyInput,
    /// Slice length does not match required channel, frame, or tap count.
    LengthMismatch,
    /// Input contains `NaN` or `±Infinity`.
    NonFiniteInput,
    /// Parameter is out of domain (e.g. negative sample rate, zero norm direction, invalid order).
    InvalidParameter,
    /// Quaternion squared norm deviates from `1.0` by more than `1e-12` (C02-C04 admission rule).
    InvalidQuaternionNorm,
    /// Numeric overflow or non-finite intermediate result.
    NumericRange,
    /// Underlying `etalon_signal` primitive failure.
    Signal(etalon_signal::SignalError),
}

impl From<etalon_signal::SignalError> for SpatialDspError {
    #[inline]
    fn from(err: etalon_signal::SignalError) -> Self {
        match err {
            etalon_signal::SignalError::EmptyInput => Self::EmptyInput,
            etalon_signal::SignalError::LengthMismatch => Self::LengthMismatch,
            etalon_signal::SignalError::NonFiniteInput => Self::NonFiniteInput,
            etalon_signal::SignalError::InvalidParameter => Self::InvalidParameter,
            etalon_signal::SignalError::NumericRange => Self::NumericRange,
        }
    }
}

/// Compute the Ambisonic Channel Number $\text{ACN} = l(l + 1) + m$ for degree $l \in [0, 3]$
/// and order $m \in [-l, l]$.
#[inline]
pub fn acn_index(l: usize, m: isize) -> Result<usize, SpatialDspError> {
    Ok(etalon_signal::hoa::acn_index(l, m)?)
}

/// Recover $(l, m)$ from an Ambisonic Channel Number $\text{ACN} \in [0, 15]$.
#[inline]
pub fn acn_degree_order(acn: usize) -> Result<(usize, isize), SpatialDspError> {
    Ok(etalon_signal::hoa::acn_degree_order(acn)?)
}

/// Exact `N3D / SN3D` scaling factor $\sqrt{2l + 1}$ for degree $l \in [0, 3]$ in `f64`.
#[inline]
pub fn n3d_over_sn3d_factor_f64(l: usize) -> Result<f64, SpatialDspError> {
    Ok(etalon_signal::hoa::n3d_over_sn3d_factor_f64(l)?)
}

/// Exact `SN3D / N3D` scaling factor $1 / \sqrt{2l + 1}$ for degree $l \in [0, 3]$ in `f64`.
#[inline]
pub fn sn3d_over_n3d_factor_f64(l: usize) -> Result<f64, SpatialDspError> {
    Ok(etalon_signal::hoa::sn3d_over_n3d_factor_f64(l)?)
}

/// Convert spherical listener-to-source angles `(azimuth_rad, elevation_rad)` to a unit
/// right-handed Cartesian direction vector `[x, y, z]` (`+X` forward, `+Y` left, `+Z` up).
#[inline]
pub fn spherical_to_unit_cartesian_f64(
    azimuth_rad: f64,
    elevation_rad: f64,
) -> Result<[f64; 3], SpatialDspError> {
    Ok(etalon_signal::hoa::spherical_to_unit_cartesian_f64(
        azimuth_rad,
        elevation_rad,
    )?)
}

/// Evaluate all real spherical harmonics (`ACN 0..(L+1)^2`) at a right-handed Cartesian
/// listener-to-source direction `direction_xyz = [x, y, z]` in `f64`.
#[inline]
pub fn evaluate_hoa_basis_f64(
    order: HoaOrder,
    normalization: HoaNormalization,
    direction_xyz: [f64; 3],
    out_coeffs: &mut [f64],
) -> Result<usize, SpatialDspError> {
    Ok(etalon_signal::hoa::evaluate_hoa_basis_f64(
        order,
        normalization,
        direction_xyz,
        out_coeffs,
    )?)
}

/// Evaluate real spherical harmonics (`ACN 0..(L+1)^2`) at `direction_xyz` into `f32` output.
#[inline]
pub fn evaluate_hoa_basis_f32(
    order: HoaOrder,
    normalization: HoaNormalization,
    direction_xyz: [f32; 3],
    out_coeffs: &mut [f32],
) -> Result<usize, SpatialDspError> {
    Ok(etalon_signal::hoa::evaluate_hoa_basis_f32(
        order,
        normalization,
        direction_xyz,
        out_coeffs,
    )?)
}

/// Convert a single HOA coefficient frame in-place between `SN3D` and `N3D` in `f64`.
#[inline]
pub fn convert_hoa_normalization_in_place_f64(
    order: HoaOrder,
    from: HoaNormalization,
    to: HoaNormalization,
    frame: &mut [f64],
) -> Result<(), SpatialDspError> {
    Ok(etalon_signal::hoa::convert_hoa_normalization_in_place_f64(
        order, from, to, frame,
    )?)
}

/// Convert interleaved `time-channel` HOA frames in-place between `SN3D` and `N3D` in `f32`.
#[inline]
pub fn convert_hoa_stream_normalization_in_place_f32(
    order: HoaOrder,
    from: HoaNormalization,
    to: HoaNormalization,
    interleaved_frames: &mut [f32],
) -> Result<usize, SpatialDspError> {
    Ok(
        etalon_signal::hoa::convert_hoa_stream_normalization_in_place_f32(
            order,
            from,
            to,
            interleaved_frames,
        )?,
    )
}

/// Encode a mono time-domain signal into an interleaved `time-channel` (`[frame][acn]`)
/// HOA buffer by accumulating (`+=`) into `out_hoa_interleaved`.
#[inline]
pub fn encode_mono_to_hoa_accum_into(
    mono_samples: &[f32],
    order: HoaOrder,
    normalization: HoaNormalization,
    direction_xyz: [f32; 3],
    gain: f32,
    out_hoa_interleaved: &mut [f32],
) -> Result<usize, SpatialDspError> {
    Ok(etalon_signal::hoa::encode_mono_to_hoa_accum_into(
        mono_samples,
        order,
        normalization,
        direction_xyz,
        gain,
        out_hoa_interleaved,
    )?)
}

/// Evaluate max-$r_E$ high-frequency energy-vector optimization weight $a_l$ for degree $l \le L$.
#[inline]
pub fn max_re_degree_weight_f64(order: HoaOrder, l: usize) -> Result<f64, SpatialDspError> {
    Ok(etalon_signal::hoa::max_re_degree_weight_f64(order, l)?)
}

/// Decode a single HOA frame (`frame_hoa[..ch]`) to a virtual microphone or loudspeaker
/// pointing toward `look_direction_xyz`.
#[inline]
pub fn decode_hoa_frame_to_direction_f64(
    order: HoaOrder,
    normalization: HoaNormalization,
    frame_hoa: &[f64],
    look_direction_xyz: [f64; 3],
    apply_max_re: bool,
) -> Result<f64, SpatialDspError> {
    Ok(etalon_signal::hoa::decode_hoa_frame_to_direction_f64(
        order,
        normalization,
        frame_hoa,
        look_direction_xyz,
        apply_max_re,
    )?)
}

/// Decode an interleaved `time-channel` HOA stream (`hoa_interleaved[frame * ch + acn]`)
/// onto `speaker_directions_xyz` into `out_speaker_interleaved[frame * n_speakers + spk]`.
#[inline]
pub fn decode_hoa_stream_to_speakers_into(
    order: HoaOrder,
    normalization: HoaNormalization,
    hoa_interleaved: &[f32],
    speaker_directions_xyz: &[[f32; 3]],
    apply_max_re: bool,
    out_speaker_interleaved: &mut [f32],
) -> Result<usize, SpatialDspError> {
    Ok(etalon_signal::hoa::decode_hoa_stream_to_speakers_into(
        order,
        normalization,
        hoa_interleaved,
        speaker_directions_xyz,
        apply_max_re,
        out_speaker_interleaved,
    )?)
}

/// Encode a single QAC Mode `0x0102` (`hoa_spatial_audio`) record into little-endian bytes
/// (`4 * (L + 1)^2` bytes).
pub fn encode_qac_mode0102_hoa_record_le(
    order: HoaOrder,
    frame_coeffs: &[f32],
    out_bytes: &mut [u8],
) -> Result<usize, SpatialDspError> {
    let ch = order.channel_count();
    let stride = order.record_stride_bytes();
    if frame_coeffs.len() < ch || out_bytes.len() < stride {
        return Err(SpatialDspError::LengthMismatch);
    }
    if frame_coeffs[..ch].iter().any(|c| !c.is_finite()) {
        return Err(SpatialDspError::NonFiniteInput);
    }
    for (i, &c) in frame_coeffs[..ch].iter().enumerate() {
        out_bytes[i * 4..(i + 1) * 4].copy_from_slice(&c.to_le_bytes());
    }
    Ok(stride)
}

/// Decode a single QAC Mode `0x0102` (`hoa_spatial_audio`) record from little-endian bytes.
pub fn decode_qac_mode0102_hoa_record_le(
    order: HoaOrder,
    in_bytes: &[u8],
    out_coeffs: &mut [f32],
) -> Result<usize, SpatialDspError> {
    let ch = order.channel_count();
    let stride = order.record_stride_bytes();
    if in_bytes.len() < stride || out_coeffs.len() < ch {
        return Err(SpatialDspError::LengthMismatch);
    }
    for i in 0..ch {
        let b = &in_bytes[i * 4..(i + 1) * 4];
        let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        if !v.is_finite() {
            return Err(SpatialDspError::NonFiniteInput);
        }
        out_coeffs[i] = v;
    }
    Ok(ch)
}
