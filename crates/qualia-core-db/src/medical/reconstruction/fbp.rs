//! Bounded source-bound CT 2D Filtered Back-Projection (FBP) over Parallel,
//! Equiangular Fan-Beam, and Equispaced Flat-Panel Fan-Beam geometries.
use super::spectral_fft::{fft_1d, ifft_1d};
use crate::medical::dicom_frames::ValueUnits;
use core::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FbpFilter {
    RamLak,
    SheppLogan,
    Hann,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProjectionGeometry {
    Parallel,
    FanEquiangular {
        source_to_isocenter_mm: f64,
        detector_arc_spacing_rad: f64,
    },
    FanEquispaced {
        source_to_isocenter_mm: f64,
        source_to_detector_mm: f64,
        detector_spacing_mm: f64,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct FbpConfig {
    pub image_width: usize,
    pub image_height: usize,
    pub pixel_spacing_mm: [f64; 2],
    pub filter: FbpFilter,
    pub cutoff_ratio: f64,
    pub geometry: ProjectionGeometry,
    pub apply_parker_short_scan: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct SinogramView<'a> {
    pub source_blake3: [u8; 32],
    pub n_angles: usize,
    pub n_detectors: usize,
    pub detector_spacing_mm: f64,
    pub center_detector_offset: f64,
    pub angles_rad: &'a [f64],
    pub projections: &'a [f64],
    pub units: ValueUnits<'a>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FbpProvenance<'a> {
    pub source_blake3: [u8; 32],
    pub derivation_blake3: [u8; 32],
    pub image_width: usize,
    pub image_height: usize,
    pub filter: FbpFilter,
    pub geometry: ProjectionGeometry,
    pub units: ValueUnits<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbpError {
    MissingSourceData,
    DimensionMismatch,
    OutputTooSmall,
    NonFiniteInput,
    InvalidGeometry(&'static str),
    InvalidFilterCutoff,
    Overflow,
}

impl std::fmt::Display for FbpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CT FBP reconstruction error: {self:?}")
    }
}
impl std::error::Error for FbpError {}

/// Construct Kak & Slaney DC-faithful Ram-Lak discrete impulse response modulated
/// in frequency domain by Shepp-Logan (`sinc`) or Hann rolloff window.
pub fn filter_projection_fft(
    proj: &[f64],
    spacing: f64,
    filter: FbpFilter,
    cutoff_ratio: f64,
) -> Result<Vec<f64>, FbpError> {
    let n = proj.len();
    let pad = (n * 2).max(8).next_power_of_two();
    let half = pad / 2;

    // Spatial Kak-Slaney kernel h_rl[k] for k in -half..half mapped cyclically
    let mut kernel = vec![(0.0, 0.0); pad];
    kernel[0] = (1.0 / (4.0 * spacing * spacing), 0.0);
    for k in 1..half {
        if k % 2 == 1 {
            let val = -1.0 / (PI * k as f64 * spacing).powi(2);
            kernel[k] = (val, 0.0);
            kernel[pad - k] = (val, 0.0);
        }
    }
    let mut h_freq = fft_1d(&kernel).map_err(|_| FbpError::NonFiniteInput)?;

    // Apply requested band-limit & window rolloff in frequency domain (bin k -> |freq| in [0, 1])
    for (k, bin) in h_freq.iter_mut().enumerate() {
        let bin_abs = if k <= half { k } else { pad - k };
        let norm_f = bin_abs as f64 / half as f64; // [0, 1] Nyquist
        let w = if norm_f > cutoff_ratio {
            0.0
        } else {
            let scaled = norm_f / cutoff_ratio.max(1e-9);
            match filter {
                FbpFilter::RamLak => 1.0,
                FbpFilter::SheppLogan => {
                    if scaled == 0.0 {
                        1.0
                    } else {
                        let a = 0.5 * PI * scaled;
                        a.sin() / a
                    }
                }
                FbpFilter::Hann => 0.5 * (1.0 + (PI * scaled).cos()),
            }
        };
        bin.0 *= w;
        bin.1 *= w;
    }

    let mut padded = vec![(0.0, 0.0); pad];
    for (i, &v) in proj.iter().enumerate() {
        padded[i] = (v, 0.0);
    }
    let mut p_freq = fft_1d(&padded).map_err(|_| FbpError::NonFiniteInput)?;
    for (pf, hf) in p_freq.iter_mut().zip(h_freq.iter()) {
        let re = pf.0 * hf.0 - pf.1 * hf.1;
        let im = pf.0 * hf.1 + pf.1 * hf.0;
        *pf = (re, im);
    }
    let filtered = ifft_1d(&p_freq).map_err(|_| FbpError::NonFiniteInput)?;
    Ok(filtered.into_iter().take(n).map(|(re, _)| re * spacing).collect())
}

/// Smooth Parker sinogram redundancy weights for short-scan fan-beam `[0, pi + 2*gamma_max)`.
fn parker_weight(beta: f64, gamma: f64, delta: f64) -> f64 {
    if delta <= 0.0 {
        return 1.0;
    }
    let beta_w = beta.rem_euclid(2.0 * PI);
    let cond1 = 2.0 * delta - 2.0 * gamma;
    let cond2 = PI - 2.0 * gamma;
    let cond3 = PI + 2.0 * delta + 2.0 * gamma;
    let cond4 = PI + 2.0 * delta;
    if beta_w >= 0.0 && beta_w < cond1 {
        let arg = (PI / 4.0) * (beta_w / (delta - gamma).max(1e-9));
        arg.sin().powi(2)
    } else if beta_w >= cond1 && beta_w < cond2 {
        1.0
    } else if beta_w >= cond2 && beta_w <= cond4.max(cond3) {
        let denom = (delta + gamma).max(1e-9);
        let arg = (PI / 4.0) * ((PI + 2.0 * delta - beta_w) / denom).clamp(0.0, 2.0);
        arg.sin().powi(2)
    } else {
        0.0
    }
}

fn bilinear_sample(row: &[f64], pos: f64) -> f64 {
    if pos < 0.0 || pos > (row.len() - 1) as f64 {
        return 0.0;
    }
    let idx = pos.floor() as usize;
    let frac = pos - idx as f64;
    if idx + 1 >= row.len() {
        row[idx]
    } else {
        row[idx] * (1.0 - frac) + row[idx + 1] * frac
    }
}

fn validate_sinogram_and_config(
    sino: &SinogramView<'_>,
    cfg: &FbpConfig,
    out_len: usize,
) -> Result<(usize, f64), FbpError> {
    if sino.n_angles == 0 || sino.n_detectors < 2 || cfg.image_width == 0 || cfg.image_height == 0 {
        return Err(FbpError::MissingSourceData);
    }
    let expected_sino = sino.n_angles.checked_mul(sino.n_detectors).ok_or(FbpError::Overflow)?;
    if sino.angles_rad.len() != sino.n_angles || sino.projections.len() != expected_sino {
        return Err(FbpError::DimensionMismatch);
    }
    let out_pixels = cfg.image_width.checked_mul(cfg.image_height).ok_or(FbpError::Overflow)?;
    if out_len < out_pixels {
        return Err(FbpError::OutputTooSmall);
    }
    if cfg.pixel_spacing_mm[0] <= 0.0
        || cfg.pixel_spacing_mm[1] <= 0.0
        || !cfg.pixel_spacing_mm[0].is_finite()
        || !cfg.pixel_spacing_mm[1].is_finite()
    {
        return Err(FbpError::InvalidGeometry("pixel spacing must be positive and finite"));
    }
    if sino.detector_spacing_mm <= 0.0 || !sino.detector_spacing_mm.is_finite() {
        return Err(FbpError::InvalidGeometry("detector spacing must be positive"));
    }
    if !(0.05..=1.0).contains(&cfg.cutoff_ratio) || !cfg.cutoff_ratio.is_finite() {
        return Err(FbpError::InvalidFilterCutoff);
    }
    if sino.angles_rad.iter().any(|v| !v.is_finite()) || sino.projections.iter().any(|v| !v.is_finite()) {
        return Err(FbpError::NonFiniteInput);
    }
    match cfg.geometry {
        ProjectionGeometry::Parallel => {}
        ProjectionGeometry::FanEquiangular { source_to_isocenter_mm, detector_arc_spacing_rad } => {
            if source_to_isocenter_mm <= 0.0 || detector_arc_spacing_rad <= 0.0 {
                return Err(FbpError::InvalidGeometry("fan equiangular parameters must be positive"));
            }
        }
        ProjectionGeometry::FanEquispaced {
            source_to_isocenter_mm,
            source_to_detector_mm,
            detector_spacing_mm,
        } => {
            if source_to_isocenter_mm <= 0.0 || source_to_detector_mm <= 0.0 || detector_spacing_mm <= 0.0 {
                return Err(FbpError::InvalidGeometry("fan equispaced parameters must be positive"));
            }
        }
    }
    let min_a = sino.angles_rad.iter().copied().fold(f64::INFINITY, f64::min);
    let max_a = sino.angles_rad.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max_a - min_a).abs();
    if span <= 1e-6 {
        return Err(FbpError::InvalidGeometry("zero angular span"));
    }
    let d_theta = if sino.n_angles > 1 { span / (sino.n_angles - 1) as f64 } else { span };
    Ok((out_pixels, d_theta))
}

/// Execute source-bound CT 2D Filtered Back-Projection into caller-owned `output`.
/// Leaves `output` completely untouched on any validation error.
pub fn reconstruct_ct_fbp_into<'a>(
    sino: &SinogramView<'a>,
    cfg: &FbpConfig,
    output: &mut [f64],
) -> Result<FbpProvenance<'a>, FbpError> {
    let (out_pixels, d_theta) = validate_sinogram_and_config(sino, cfg, output.len())?;

    let mut staged = vec![0.0f64; out_pixels];
    let det_mid = 0.5 * (sino.n_detectors - 1) as f64 + sino.center_detector_offset;
    let img_cx = 0.5 * (cfg.image_width - 1) as f64;
    let img_cy = 0.5 * (cfg.image_height - 1) as f64;

    let min_a = sino.angles_rad.iter().copied().fold(f64::INFINITY, f64::min);
    let max_a = sino.angles_rad.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = max_a - min_a;

    for (ai, &angle) in sino.angles_rad.iter().enumerate() {
        let raw_row = &sino.projections[ai * sino.n_detectors..(ai + 1) * sino.n_detectors];
        let mut weighted = vec![0.0; sino.n_detectors];
        let filt_spacing = match cfg.geometry {
            ProjectionGeometry::Parallel => {
                weighted.copy_from_slice(raw_row);
                sino.detector_spacing_mm
            }
            ProjectionGeometry::FanEquiangular { source_to_isocenter_mm, detector_arc_spacing_rad } => {
                let half_fan = 0.5 * (sino.n_detectors as f64) * detector_arc_spacing_rad;
                for (di, wv) in weighted.iter_mut().enumerate() {
                    let gamma = (di as f64 - det_mid) * detector_arc_spacing_rad;
                    let pw = if cfg.apply_parker_short_scan && span < 1.9 * PI {
                        parker_weight(angle - min_a, gamma, half_fan)
                    } else {
                        1.0
                    };
                    *wv = raw_row[di] * source_to_isocenter_mm * gamma.cos() * pw;
                }
                detector_arc_spacing_rad
            }
            ProjectionGeometry::FanEquispaced { source_to_isocenter_mm, source_to_detector_mm, detector_spacing_mm } => {
                let half_fan = (0.5 * sino.n_detectors as f64 * detector_spacing_mm).atan2(source_to_detector_mm);
                for (di, wv) in weighted.iter_mut().enumerate() {
                    let s = (di as f64 - det_mid) * detector_spacing_mm;
                    let cos_g = source_to_detector_mm / (source_to_detector_mm.powi(2) + s * s).sqrt();
                    let gamma = s.atan2(source_to_detector_mm);
                    let pw = if cfg.apply_parker_short_scan && span < 1.9 * PI {
                        parker_weight(angle - min_a, gamma, half_fan)
                    } else {
                        1.0
                    };
                    let mag = source_to_isocenter_mm / source_to_detector_mm;
                    *wv = raw_row[di] * mag * cos_g * pw;
                }
                detector_spacing_mm * (source_to_isocenter_mm / source_to_detector_mm)
            }
        };

        let filt_row = filter_projection_fft(&weighted, filt_spacing, cfg.filter, cfg.cutoff_ratio)?;
        let (sin_a, cos_a) = angle.sin_cos();

        for py in 0..cfg.image_height {
            let y = (py as f64 - img_cy) * cfg.pixel_spacing_mm[0];
            for px in 0..cfg.image_width {
                let x = (px as f64 - img_cx) * cfg.pixel_spacing_mm[1];
                let (det_idx, bp_weight) = match cfg.geometry {
                    ProjectionGeometry::Parallel => {
                        let t = x * cos_a + y * sin_a;
                        (det_mid + t / sino.detector_spacing_mm, 1.0)
                    }
                    ProjectionGeometry::FanEquiangular { source_to_isocenter_mm, detector_arc_spacing_rad } => {
                        let sx = -source_to_isocenter_mm * sin_a;
                        let sy = source_to_isocenter_mm * cos_a;
                        let rx = x - sx;
                        let ry = y - sy;
                        let l_sq = rx * rx + ry * ry;
                        let u = x * cos_a + y * sin_a;
                        let v_dist = source_to_isocenter_mm - (-x * sin_a + y * cos_a);
                        let gamma = u.atan2(v_dist.max(1e-6));
                        (det_mid + gamma / detector_arc_spacing_rad, 0.5 / l_sq.max(1e-6))
                    }
                    ProjectionGeometry::FanEquispaced {
                        source_to_isocenter_mm,
                        source_to_detector_mm,
                        detector_spacing_mm,
                    } => {
                        let u_vec = source_to_isocenter_mm + x * sin_a - y * cos_a;
                        let ratio = source_to_detector_mm / u_vec.max(1e-6);
                        let s = (x * cos_a + y * sin_a) * ratio;
                        let mag_u = (source_to_isocenter_mm / u_vec.max(1e-6)).powi(2);
                        (det_mid + s / detector_spacing_mm, 0.5 * mag_u)
                    }
                };
                let val = bilinear_sample(&filt_row, det_idx);
                staged[py * cfg.image_width + px] += val * bp_weight * d_theta;
            }
        }
    }

    let mut hasher = blake3::Hasher::new();
    hasher.update(&sino.source_blake3);
    hasher.update(&(cfg.image_width as u64).to_le_bytes());
    hasher.update(&(cfg.image_height as u64).to_le_bytes());
    for v in &staged {
        hasher.update(&v.to_le_bytes());
    }
    output[..out_pixels].copy_from_slice(&staged);
    Ok(FbpProvenance {
        source_blake3: sino.source_blake3,
        derivation_blake3: *hasher.finalize().as_bytes(),
        image_width: cfg.image_width,
        image_height: cfg.image_height,
        filter: cfg.filter,
        geometry: cfg.geometry,
        units: sino.units,
    })
}
