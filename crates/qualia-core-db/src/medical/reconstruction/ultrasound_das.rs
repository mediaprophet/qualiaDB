//! Dynamic Receive Delay-and-Sum (DAS) ultrasound beamformer with plane-wave &
//! virtual-source acoustic focusing, dynamic f-number aperture apodization,
//! cubic RF interpolation, analytic Hilbert envelope detection, and dB compression.
use super::spectral_fft::{analytic_hilbert_envelope_1d, apodization_weight, WindowKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DasError {
    DimensionMismatch,
    InvalidConfig(&'static str),
    NonFiniteInput,
    OutputTooSmall,
    Overflow,
}

impl std::fmt::Display for DasError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ultrasound DAS beamforming error: {self:?}")
    }
}
impl std::error::Error for DasError {}

#[derive(Clone, Copy, Debug)]
pub enum TxFocusMode {
    PlaneWave { angle_rad: f64 },
    VirtualPoint { source_xz_mm: [f64; 2], t0_s: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct UltrasoundArray<'a> {
    pub elem_x_mm: &'a [f64],
    pub elem_z_mm: &'a [f64],
    pub tx_mode: TxFocusMode,
}

#[derive(Clone, Copy, Debug)]
pub struct DasConfig<'a> {
    pub sound_speed_mps: f64,
    pub sample_rate_hz: f64,
    pub f_number: f64,
    pub min_aperture_half_mm: f64,
    pub apodization: WindowKind,
    pub dynamic_range_db: f64,
    pub pixel_x_mm: &'a [f64],
    pub pixel_z_mm: &'a [f64],
}

#[derive(Clone, Debug)]
pub struct DasBeamformResult {
    pub source_blake3: [u8; 32],
    pub derivation_blake3: [u8; 32],
    pub width: usize,
    pub height: usize,
    pub rf_beamformed: Vec<f64>,
    pub envelope: Vec<f64>,
    pub peak_envelope: f64,
}

/// 4-tap Catmull-Rom cubic fractional sample interpolator for continuous-time RF delay lines.
fn sample_rf_cubic(channel: &[f64], frac_sample: f64) -> f64 {
    if frac_sample < 0.0 || frac_sample >= (channel.len() - 1) as f64 {
        return 0.0;
    }
    let i = frac_sample.floor() as isize;
    let t = frac_sample - i as f64;
    let get = |idx: isize| -> f64 {
        if idx >= 0 && (idx as usize) < channel.len() {
            channel[idx as usize]
        } else {
            0.0
        }
    };
    let p0 = get(i - 1);
    let p1 = get(i);
    let p2 = get(i + 1);
    let p3 = get(i + 2);
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
}

/// Dynamic receive DAS beamforming (`bmode_db_out` in `[-dynamic_range_db, 0.0]`, `[nz * nx]`).
pub fn beamform_ultrasound_das_into(
    rf_channels: &[f64], // row-major `[n_elem][n_samples]`
    n_samples: usize,
    array: &UltrasoundArray<'_>,
    cfg: &DasConfig<'_>,
    source_blake3: [u8; 32],
    bmode_db_out: &mut [f64],
) -> Result<DasBeamformResult, DasError> {
    let n_elem = array.elem_x_mm.len();
    let nx = cfg.pixel_x_mm.len();
    let nz = cfg.pixel_z_mm.len();
    if n_elem == 0 || n_samples < 4 || nx == 0 || nz == 0 || array.elem_z_mm.len() != n_elem {
        return Err(DasError::DimensionMismatch);
    }
    let expect_rf = n_elem.checked_mul(n_samples).ok_or(DasError::Overflow)?;
    let expect_out = nz.checked_mul(nx).ok_or(DasError::Overflow)?;
    if rf_channels.len() != expect_rf {
        return Err(DasError::DimensionMismatch);
    }
    if bmode_db_out.len() < expect_out {
        return Err(DasError::OutputTooSmall);
    }
    if cfg.sound_speed_mps <= 0.0
        || cfg.sample_rate_hz <= 0.0
        || cfg.f_number <= 0.0
        || cfg.dynamic_range_db <= 0.0
        || cfg.min_aperture_half_mm <= 0.0
        || !cfg.sound_speed_mps.is_finite()
        || !cfg.sample_rate_hz.is_finite()
        || !cfg.f_number.is_finite()
        || !cfg.dynamic_range_db.is_finite()
    {
        return Err(DasError::InvalidConfig("acoustic speed, sample rate, f-number, and dynamic range must be positive"));
    }
    if rf_channels.iter().any(|v| !v.is_finite())
        || array.elem_x_mm.iter().any(|v| !v.is_finite())
        || array.elem_z_mm.iter().any(|v| !v.is_finite())
        || cfg.pixel_x_mm.iter().any(|v| !v.is_finite())
        || cfg.pixel_z_mm.iter().any(|v| !v.is_finite())
    {
        return Err(DasError::NonFiniteInput);
    }

    let c_mm_per_s = cfg.sound_speed_mps * 1000.0;
    let x_ref_min = array.elem_x_mm.iter().copied().fold(f64::INFINITY, f64::min);
    let x_ref_max = array.elem_x_mm.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    let mut rf_beamformed = vec![0.0f64; expect_out];
    for iz in 0..nz {
        let pz = cfg.pixel_z_mm[iz];
        for ix in 0..nx {
            let px = cfg.pixel_x_mm[ix];
            let t_tx = match array.tx_mode {
                TxFocusMode::PlaneWave { angle_rad } => {
                    let x_orig = if angle_rad >= 0.0 { x_ref_min } else { x_ref_max };
                    ((px - x_orig) * angle_rad.sin() + pz * angle_rad.cos()) / c_mm_per_s
                }
                TxFocusMode::VirtualPoint { source_xz_mm, t0_s } => {
                    let sign = if pz >= source_xz_mm[1] { 1.0 } else { -1.0 };
                    let dist = (px - source_xz_mm[0]).hypot(pz - source_xz_mm[1]);
                    t0_s + sign * dist / c_mm_per_s
                }
            };

            let half_ap = ((pz.abs()) / (2.0 * cfg.f_number)).max(cfg.min_aperture_half_mm);
            let mut sum = 0.0;
            let mut w_sum = 0.0;
            for e in 0..n_elem {
                let ex = array.elem_x_mm[e];
                let ez = array.elem_z_mm[e];
                let dx_el = (ex - px).abs();
                let u = dx_el / half_ap;
                let w = apodization_weight(cfg.apodization, u);
                if w <= 0.0 {
                    continue;
                }
                let t_rx = (px - ex).hypot(pz - ez) / c_mm_per_s;
                let sample_idx = (t_tx + t_rx) * cfg.sample_rate_hz;
                let ch = &rf_channels[e * n_samples..(e + 1) * n_samples];
                sum += w * sample_rf_cubic(ch, sample_idx);
                w_sum += w;
            }
            rf_beamformed[iz * nx + ix] = if w_sum > 1e-12 { sum / w_sum } else { 0.0 };
        }
    }

    // Axial analytic Hilbert envelope per lateral column `[0..nx]`
    let mut envelope = vec![0.0f64; expect_out];
    let mut col_buf = vec![0.0f64; nz];
    for ix in 0..nx {
        for iz in 0..nz {
            col_buf[iz] = rf_beamformed[iz * nx + ix];
        }
        let env_col = analytic_hilbert_envelope_1d(&col_buf).map_err(|_| DasError::NonFiniteInput)?;
        for iz in 0..nz {
            envelope[iz * nx + ix] = env_col[iz];
        }
    }

    let peak_envelope = envelope.iter().copied().fold(0.0f64, f64::max);
    let ref_peak = peak_envelope.max(1e-15);
    let mut staged_db = vec![0.0f64; expect_out];
    for (i, &env) in envelope.iter().enumerate() {
        let ratio = (env / ref_peak).max(1e-12);
        staged_db[i] = (20.0 * ratio.log10()).clamp(-cfg.dynamic_range_db, 0.0);
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(&source_blake3);
    for v in &staged_db {
        hasher.update(&v.to_le_bytes());
    }
    bmode_db_out[..expect_out].copy_from_slice(&staged_db);

    Ok(DasBeamformResult {
        source_blake3,
        derivation_blake3: *hasher.finalize().as_bytes(),
        width: nx,
        height: nz,
        rf_beamformed,
        envelope,
        peak_envelope,
    })
}
