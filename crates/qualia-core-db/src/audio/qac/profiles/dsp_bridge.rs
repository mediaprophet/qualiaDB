//! Bridges between QAC domain profile views (`0x0000..=0x0001`, `0x0003`, `0x0102..=0x0107`)
//! and `audio::qac::spatial_dsp` + `audio::qac::numerical` (`[GAP-026]`).

use crate::audio::qac::numerical::{
    compute_tis_soft_tissue, compute_utd_wedge_diffraction,
    compute_zwicker_specific_and_total_loudness_into, solve_biot_poroelastic_dispersion,
    solve_pennes_bioheat_steady_state, BiotPoroelasticParams, BiotWaveDispersion,
    PennesBioheatParams, PennesBioheatResult, ThermalIndexEstimate, ZwickerLoudnessReport,
    ZWICKER_BARK_BANDS,
};
use crate::audio::qac::spatial_dsp::{
    convolve_stereo_hrir_with_additional_delays_into, decode_hoa_stream_to_speakers_into,
    extract_mel_mfcc_frame_into, measure_bs1770_integrated_loudness, Bs1770ChannelRole,
    Bs1770LoudnessReading, HoaNormalization as DspHoaNormalization, HoaOrder, HoaWignerRotation,
};

use super::{
    synthesize_modal_sample, ElasticStressView, HapticDocumentView, HoaNormalization, HoaOrdering,
    HoaTemporalView, HrirDocumentView, ModalBankView, PerceptualDocumentView, PerceptualScale,
    PressureVelocityView, ProfileError, RayContributionView, UltrasoundBeamSummaryRecord,
    UltrasoundDocumentView,
};

fn map_hoa_order(order: u8) -> Result<HoaOrder, ProfileError> {
    match order {
        0 => Ok(HoaOrder::Order0),
        1 => Ok(HoaOrder::Order1),
        2 => Ok(HoaOrder::Order2),
        3 => Ok(HoaOrder::Order3),
        _ => Err(ProfileError::InvalidMetadata("unsupported HOA order")),
    }
}

/// Rotates and decodes a `0x0102` `HoaTemporalView` (`ACN`/`SN3D` or `N3D`) via `spatial_dsp`.
pub fn bridge_hoa_view_rotate_and_decode(
    view: &HoaTemporalView<'_>,
    quat_xyzw: [f64; 4],
    speaker_directions_xyz: &[[f32; 3]],
    apply_max_re: bool,
) -> Result<Vec<f32>, ProfileError> {
    if view.ordering != HoaOrdering::Acn {
        return Err(ProfileError::InvalidMetadata("spatial_dsp HOA bridge requires ACN ordering"));
    }
    let order = map_hoa_order(view.order)?;
    let norm = match view.normalization {
        HoaNormalization::Sn3d => DspHoaNormalization::Sn3d,
        HoaNormalization::N3d => DspHoaNormalization::N3d,
        HoaNormalization::MaxN => {
            return Err(ProfileError::InvalidMetadata("MaxN not supported in ACN stream bridge"));
        }
    };
    let wigner = HoaWignerRotation::from_quaternion_xyzw(quat_xyzw)
        .map_err(|_| ProfileError::InvalidQuaternion)?;
    let n_ch = view.chunk.channels();
    let n_frames = view.chunk.frame_count();
    let mut in_stream = vec![0.0f32; n_frames * n_ch];
    for f in 0..n_frames {
        view.chunk
            .read_frame(f, &mut in_stream[f * n_ch..(f + 1) * n_ch])?;
    }
    let mut rot_stream = vec![0.0f32; in_stream.len()];
    wigner
        .apply_stream_into_f32(order, &in_stream, &mut rot_stream)
        .map_err(|_| ProfileError::InvalidMetadata("Wigner HOA stream rotation failed"))?;
    let mut out_speakers = vec![0.0f32; n_frames * speaker_directions_xyz.len()];
    decode_hoa_stream_to_speakers_into(
        order,
        norm,
        &rot_stream,
        speaker_directions_xyz,
        apply_max_re,
        &mut out_speakers,
    )
    .map_err(|_| ProfileError::InvalidMetadata("HOA speaker decode failed"))?;
    Ok(out_speakers)
}

/// Convolves a `0x0103` `HrirDocumentView` stereo measurement with a mono `f32` signal via `spatial_dsp`.
pub fn bridge_hrir_view_binaural_convolve(
    doc: &HrirDocumentView<'_>,
    measurement_idx: u32,
    mono_signal: &[f32],
) -> Result<(Vec<f32>, Vec<f32>), ProfileError> {
    let m_idx = measurement_idx as usize;
    if doc.chunk.receivers() < 2 || m_idx >= doc.chunk.measurements() {
        return Err(ProfileError::OutOfBoundsIndex("HRIR measurement/receiver index"));
    }
    let taps = doc.chunk.taps();
    let mut left_ir = vec![0.0f32; taps];
    let mut right_ir = vec![0.0f32; taps];
    doc.chunk.read_impulse(m_idx, 0, &mut left_ir)?;
    doc.chunk.read_impulse(m_idx, 1, &mut right_ir)?;
    let delays_lr = match &doc.delays_samples {
        Some(d) if d.len() >= (m_idx + 1) * 2 => [
            d[m_idx * 2].max(0.0).round() as usize,
            d[m_idx * 2 + 1].max(0.0).round() as usize,
        ],
        _ => [0, 0],
    };
    let out_len = mono_signal.len() + taps + delays_lr[0].max(delays_lr[1]);
    let mut out_l = vec![0.0f32; out_len];
    let mut out_r = vec![0.0f32; out_len];
    let (nl, nr) = convolve_stereo_hrir_with_additional_delays_into(
        mono_signal,
        &left_ir,
        &right_ir,
        delays_lr,
        &mut out_l,
        &mut out_r,
    )
    .map_err(|_| ProfileError::InvalidMetadata("stereo HRIR convolution failed"))?;
    out_l.truncate(nl);
    out_r.truncate(nr);
    Ok((out_l, out_r))
}

/// Evaluates ISO 532-1 Zwicker 24-Bark loudness (`ZwickerLoudnessReport`) from a `0x0105` `PerceptualDocumentView`.
pub fn bridge_perceptual_view_zwicker_sones(
    doc: &PerceptualDocumentView<'_>,
    frame_idx: usize,
) -> Result<(ZwickerLoudnessReport, [f64; ZWICKER_BARK_BANDS]), ProfileError> {
    if doc.chunk.feature_count() != ZWICKER_BARK_BANDS || frame_idx >= doc.chunk.frame_count() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let mut frame = [0.0f32; ZWICKER_BARK_BANDS];
    doc.chunk.read_frame(frame_idx, &mut frame)?;
    let mut levels_db = [0.0f64; ZWICKER_BARK_BANDS];
    for (i, &val) in frame.iter().enumerate() {
        let v = f64::from(val);
        levels_db[i] = match doc.scale {
            PerceptualScale::DbSpl => v,
            _ => 10.0 * v.max(1e-12).log10(),
        };
    }
    let mut specific = [0.0f64; ZWICKER_BARK_BANDS];
    let report = compute_zwicker_specific_and_total_loudness_into(&levels_db, &mut specific)
        .map_err(|_| ProfileError::InvalidMetadata("Zwicker loudness failed"))?;
    Ok((report, specific))
}

/// Synthesizes a `0x0104` `ModalBankView` into PCM and extracts BS.1770 loudness and Mel/MFCC features.
pub fn bridge_modal_view_to_bs1770_and_mfcc(
    view: &ModalBankView<'_>,
    dof_index: usize,
    sample_rate_hz: u32,
    num_samples: usize,
) -> Result<(Bs1770LoudnessReading, Vec<f32>, Vec<f32>), ProfileError> {
    let mut modes = Vec::with_capacity(view.records.len());
    for i in 0..view.records.len() {
        modes.push(view.records.get(i)?);
    }
    let sr = sample_rate_hz as f64;
    let mut pcm = vec![0.0f32; num_samples];
    for (n, s) in pcm.iter_mut().enumerate() {
        *s = synthesize_modal_sample(
            &modes,
            None,
            &view.mode_shapes,
            view.shape_dofs,
            dof_index,
            &view.excitations,
            (n as f64) / sr,
        )? as f32;
    }
    let mut blk_scratch = vec![0.0f64; 64];
    let loudness = measure_bs1770_integrated_loudness(
        &pcm,
        &[Bs1770ChannelRole::FrontOrCenter],
        sample_rate_hz,
        &mut blk_scratch,
    )
    .map_err(|_| ProfileError::InvalidMetadata("BS.1770 loudness failed"))?;
    let n_fft = 64usize;
    let n_bins = n_fft / 2 + 1;
    let n_mel = 8usize;
    let mut fb = vec![0.0f32; n_mel * n_bins];
    for m in 0..n_mel {
        fb[m * n_bins + (m + 1).min(n_bins - 1)] = 1.0;
    }
    let mut stft_scratch = vec![0.0f32; n_fft * 2];
    let mut pwr_scratch = vec![0.0f32; n_bins];
    let mut mel_out = vec![0.0f32; n_mel];
    let mut mfcc_out = vec![0.0f32; n_mel];
    extract_mel_mfcc_frame_into(
        &pcm[..n_fft.min(pcm.len())],
        n_mel,
        &fb,
        &mut stft_scratch,
        &mut pwr_scratch,
        &mut mel_out,
        &mut mfcc_out,
    )
    .map_err(|_| ProfileError::InvalidMetadata("Mel/MFCC extraction failed"))?;
    Ok((loudness, mel_out, mfcc_out))
}

/// Bridges `0x0000` `PressureVelocityView` and `0x0001` `ElasticStressView` with Biot poroelasticity.
pub fn bridge_continuum_views_biot_poroelastic(
    pv_view: &PressureVelocityView<'_>,
    el_view: &ElasticStressView<'_>,
    params: &BiotPoroelasticParams,
    omega_rad_s: f64,
) -> Result<(BiotWaveDispersion, f64), ProfileError> {
    if pv_view.records.is_empty() || el_view.records.is_empty() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let disp = solve_biot_poroelastic_dispersion(params, omega_rad_s)
        .map_err(|_| ProfileError::InvalidMetadata("Biot dispersion failed"))?;
    let p_fluid = f64::from(pv_view.records.get(0)?.pressure_pa);
    let p_solid = el_view.records.get(0)?.stress().hydrostatic_pressure_pa();
    Ok((disp, p_fluid + params.biot_willis_alpha * p_solid))
}

/// Bridges `0x0003` `RayContributionView`, `0x0106` `HapticDocumentView`, and `0x0107` `UltrasoundDocumentView`
/// with UTD wedge diffraction, Pennes bioheat steady-state temperature rise, and AIUM/NEMA UD 3 TIS.
pub fn bridge_ray_haptic_ultrasound_numerical(
    ray_view: &RayContributionView<'_>,
    haptic_view: &HapticDocumentView<'_>,
    us_view: &UltrasoundDocumentView<'_>,
    sound_speed_m_s: f64,
) -> Result<(f64, f64, PennesBioheatResult, ThermalIndexEstimate), ProfileError> {
    if ray_view.records.is_empty() || haptic_view.records.is_empty() || us_view.records.is_empty() {
        return Err(ProfileError::InvalidPayloadLength);
    }
    let r0 = ray_view.records.get(0)?;
    let f0 = f64::from(*ray_view.band_centers_hz.first().unwrap_or(&1000.0));
    let k = 2.0 * core::f64::consts::PI * f0 / sound_speed_m_s;
    let r_dist = f64::from(r0.distance_m).max(0.5);
    let d_utd = compute_utd_wedge_diffraction(1.5, k, r_dist, r_dist, 0.4, 2.1, 1.0, 1.0)
        .map_err(|_| ProfileError::InvalidMetadata("UTD diffraction failed"))?;
    let h0 = haptic_view.records.get(0)?;
    let normal_stress_pa = f64::from(h0.force_local_xyz_n[2].abs()) / f64::from(h0.contact_area_m2);
    let u0: UltrasoundBeamSummaryRecord = us_view.records.get(0)?;
    let f_mhz = (f64::from(u0.center_frequency_hz) * 1.0e-6).max(0.1);
    let bio = solve_pennes_bioheat_steady_state(&PennesBioheatParams {
        absorption_np_per_m_mhz: 5.0,
        frequency_mhz: f_mhz,
        i_spta_w_per_m2: f64::from(u0.intensity_ispta_w_m2),
        metabolic_heat_w_per_m3: 400.0,
        blood_perfusion_rate_s1: 0.008,
        blood_density_kg_m3: 1060.0,
        blood_specific_heat_j_kg_k: 3600.0,
        tissue_thermal_conductivity_w_m_k: 0.5,
    })
    .map_err(|_| ProfileError::InvalidMetadata("Pennes bioheat failed"))?;
    let tis = compute_tis_soft_tissue(105.0, f_mhz)
        .map_err(|_| ProfileError::InvalidMetadata("TIS failed"))?;
    Ok((d_utd.abs(), normal_stress_pa, bio, tis))
}
