//! Source-bound quantitative & non-visual inference over calibrated medical values,
//! 1D/ND spectroscopy bins, and calibrated time-series waveforms with analytical
//! error propagation and strict BLAKE3 provenance stamping.
use super::spectral_fft::fft_1d;
use crate::medical::dicom_frames::{LinearTransform, ValueUnits};
use crate::medical::dicom_geometry::PatientImagePlane;
use crate::solvers::transforms::fourier::Cplx;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceError {
    UnspecifiedUnits,
    SourceIdentityMismatch,
    FrameNumberMismatch,
    EmptySeries,
    DimensionMismatch,
    NonFiniteValue,
    InvalidCalibration(&'static str),
    MissingSpatialFrameOfReference,
    OutOfStoredRange,
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "quantitative inference error: {self:?}")
    }
}
impl std::error::Error for InferenceError {}

#[derive(Clone, Copy, Debug)]
pub struct UncertaintyBudget {
    pub stored_measurement_std: f64,
    pub slope_std: f64,
    pub intercept_std: f64,
    pub epistemic_std: f64,
    pub aleatoric_std: f64,
}

impl Default for UncertaintyBudget {
    fn default() -> Self {
        Self {
            stored_measurement_std: 0.0,
            slope_std: 0.0,
            intercept_std: 0.0,
            epistemic_std: 0.0,
            aleatoric_std: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NonVisualDomain<'a> {
    ScalarSeries,
    Spectroscopy { axis_values: &'a [f64], axis_units: &'a str },
    Waveform { sample_rate_hz: f64 },
}

#[derive(Clone, Debug)]
pub struct InferenceProvenance {
    pub source_blake3: [u8; 32],
    pub derivation_blake3: [u8; 32],
    pub frame_number: u32,
    pub algorithm_id: &'static str,
    pub units_description: String,
    pub slope: f64,
    pub intercept: f64,
    pub sample_count: usize,
    pub mean_calibrated: f64,
    pub std_calibrated: f64,
    pub min_calibrated: f64,
    pub max_calibrated: f64,
    pub propagated_uncertainty_rms: f64,
    pub spectral_centroid: Option<f64>,
    pub integral_trapezoid: Option<f64>,
    pub dominant_frequency_hz: Option<f64>,
}

fn validate_units<'a>(units: ValueUnits<'a>) -> Result<String, InferenceError> {
    match units {
        ValueUnits::Unspecified => Err(InferenceError::UnspecifiedUnits),
        ValueUnits::RescaleType(s) => {
            let t = s.trim();
            if t.is_empty() || t.eq_ignore_ascii_case("US") {
                Err(InferenceError::UnspecifiedUnits)
            } else {
                Ok(t.to_string())
            }
        }
        ValueUnits::Code { scheme, value, meaning } => {
            if scheme.trim().is_empty() || value.trim().is_empty() {
                Err(InferenceError::UnspecifiedUnits)
            } else {
                Ok(match meaning {
                    Some(m) => format!("{scheme}:{value} ({m})"),
                    None => format!("{scheme}:{value}"),
                })
            }
        }
    }
}

fn validate_budget(b: &UncertaintyBudget) -> Result<(), InferenceError> {
    let fields = [
        b.stored_measurement_std,
        b.slope_std,
        b.intercept_std,
        b.epistemic_std,
        b.aleatoric_std,
    ];
    if fields.iter().any(|&v| !v.is_finite() || v < 0.0) {
        return Err(InferenceError::InvalidCalibration("uncertainty components must be finite non-negative"));
    }
    Ok(())
}

/// Calibrate raw stored DICOM values via source-bound `LinearTransform<'a>`, propagate
/// measurement + calibration + model uncertainty into `calibrated_out` / `uncertainty_out`,
/// and compute scalar / spectroscopy / waveform summaries with BLAKE3 provenance.
pub fn calibrate_and_infer_into<'a>(
    stored_samples: &[f64],
    transform: &LinearTransform<'a>,
    expected_source_blake3: [u8; 32],
    expected_frame_number: u32,
    domain: NonVisualDomain<'a>,
    optional_plane: Option<&PatientImagePlane<'a>>,
    uncertainty: UncertaintyBudget,
    algorithm_id: &'static str,
    calibrated_out: &mut [f64],
    uncertainty_out: &mut [f64],
) -> Result<InferenceProvenance, InferenceError> {
    if stored_samples.is_empty() {
        return Err(InferenceError::EmptySeries);
    }
    if calibrated_out.len() < stored_samples.len() || uncertainty_out.len() < stored_samples.len() {
        return Err(InferenceError::DimensionMismatch);
    }
    if transform.source_blake3() != expected_source_blake3 {
        return Err(InferenceError::SourceIdentityMismatch);
    }
    if transform.frame_number() != expected_frame_number {
        return Err(InferenceError::FrameNumberMismatch);
    }
    if !transform.slope().is_finite() || transform.slope() == 0.0 || !transform.intercept().is_finite() {
        return Err(InferenceError::InvalidCalibration("zero or non-finite slope/intercept"));
    }
    if let Some(plane) = optional_plane {
        if plane.frame_of_reference_uid().is_none() {
            return Err(InferenceError::MissingSpatialFrameOfReference);
        }
    }
    validate_budget(&uncertainty)?;
    let units_description = validate_units(transform.units())?;

    match domain {
        NonVisualDomain::ScalarSeries => {}
        NonVisualDomain::Spectroscopy { axis_values, axis_units } => {
            if axis_values.len() != stored_samples.len()
                || axis_units.trim().is_empty()
                || axis_values.iter().any(|v| !v.is_finite())
            {
                return Err(InferenceError::DimensionMismatch);
            }
        }
        NonVisualDomain::Waveform { sample_rate_hz } => {
            if !sample_rate_hz.is_finite() || sample_rate_hz <= 0.0 {
                return Err(InferenceError::InvalidCalibration("waveform sample_rate_hz must be positive finite"));
            }
        }
    }

    let n = stored_samples.len();
    let mut staged_val = Vec::with_capacity(n);
    let mut staged_unc = Vec::with_capacity(n);
    let slope = transform.slope();

    for &x in stored_samples {
        if !x.is_finite() {
            return Err(InferenceError::NonFiniteValue);
        }
        let y = transform.apply(x).map_err(|_| InferenceError::OutOfStoredRange)?;
        let var = (slope * uncertainty.stored_measurement_std).powi(2)
            + (x * uncertainty.slope_std).powi(2)
            + uncertainty.intercept_std.powi(2)
            + uncertainty.epistemic_std.powi(2)
            + uncertainty.aleatoric_std.powi(2);
        staged_val.push(y);
        staged_unc.push(var.sqrt());
    }

    let mean_calibrated = staged_val.iter().sum::<f64>() / n as f64;
    let var_cal = staged_val.iter().map(|v| (v - mean_calibrated).powi(2)).sum::<f64>() / n as f64;
    let std_calibrated = var_cal.sqrt();
    let min_calibrated = staged_val.iter().copied().fold(f64::INFINITY, f64::min);
    let max_calibrated = staged_val.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let rms_unc = (staged_unc.iter().map(|u| u * u).sum::<f64>() / n as f64).sqrt();

    let (spectral_centroid, integral_trapezoid, dominant_frequency_hz) = match domain {
        NonVisualDomain::ScalarSeries => (None, None, None),
        NonVisualDomain::Spectroscopy { axis_values, .. } => {
            let mut mass = 0.0;
            let mut w_axis = 0.0;
            let mut trap = 0.0;
            for i in 0..n {
                let w = staged_val[i].max(0.0);
                mass += w;
                w_axis += w * axis_values[i];
                if i + 1 < n {
                    let dx = (axis_values[i + 1] - axis_values[i]).abs();
                    trap += 0.5 * (staged_val[i] + staged_val[i + 1]) * dx;
                }
            }
            let centroid = if mass > 1e-18 { Some(w_axis / mass) } else { None };
            (centroid, Some(trap), None)
        }
        NonVisualDomain::Waveform { sample_rate_hz } => {
            let dt = 1.0 / sample_rate_hz;
            let mut trap = 0.0;
            for i in 0..n.saturating_sub(1) {
                trap += 0.5 * (staged_val[i] + staged_val[i + 1]) * dt;
            }
            let cx: Vec<Cplx> = staged_val.iter().map(|&v| (v - mean_calibrated, 0.0)).collect();
            let dom_hz = if n >= 2 {
                let spec = fft_1d(&cx).map_err(|_| InferenceError::NonFiniteValue)?;
                let mut best_k = 1usize;
                let mut best_p = -1.0;
                for (k, &(re, im)) in spec.iter().enumerate().take(n / 2 + 1).skip(1) {
                    let p = re * re + im * im;
                    if p > best_p {
                        best_p = p;
                        best_k = k;
                    }
                }
                if best_p > 1e-18 {
                    Some(best_k as f64 * sample_rate_hz / n as f64)
                } else {
                    Some(0.0)
                }
            } else {
                None
            };
            (None, Some(trap), dom_hz)
        }
    };

    let mut hasher = blake3::Hasher::new();
    hasher.update(&expected_source_blake3);
    hasher.update(algorithm_id.as_bytes());
    hasher.update(units_description.as_bytes());
    hasher.update(&transform.slope().to_le_bytes());
    hasher.update(&transform.intercept().to_le_bytes());
    for (&v, &u) in staged_val.iter().zip(staged_unc.iter()) {
        hasher.update(&v.to_le_bytes());
        hasher.update(&u.to_le_bytes());
    }
    calibrated_out[..n].copy_from_slice(&staged_val);
    uncertainty_out[..n].copy_from_slice(&staged_unc);

    Ok(InferenceProvenance {
        source_blake3: expected_source_blake3,
        derivation_blake3: *hasher.finalize().as_bytes(),
        frame_number: expected_frame_number,
        algorithm_id,
        units_description,
        slope,
        intercept: transform.intercept(),
        sample_count: n,
        mean_calibrated,
        std_calibrated,
        min_calibrated,
        max_calibrated,
        propagated_uncertainty_rms: rms_unc,
        spectral_centroid,
        integral_trapezoid,
        dominant_frequency_hz,
    })
}
