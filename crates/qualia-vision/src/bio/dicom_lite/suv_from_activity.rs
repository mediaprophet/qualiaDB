//! Standardised Uptake Value (SUV) from activity and body metrics.
//!
//! Pure formula helpers — no DICOM I/O. Callers supply decay-corrected
//! activity and patient mass from headers they have already parsed.

use super::parse_dicom_tags_basic::DicomLiteError;

/// Body-weight SUV (SUV_bw).
///
/// ```text
/// SUV = (C_img · W) / (A_inj · decay)
/// ```
///
/// - `image_activity_bq_per_ml` — reconstructed activity concentration (Bq/ml)
/// - `injected_activity_bq` — administered activity at reference time (Bq)
/// - `body_weight_g` — patient weight in **grams** (DICOM PatientWeight is kg → ×1000)
/// - `decay_factor` — residual fraction at imaging time (`exp(-λ·Δt)`), in (0, 1]
///
/// Returns dimensionless SUV. Errors if any factor is non-positive or non-finite,
/// or if `decay_factor` is outside (0, 1].
pub fn suv_from_activity(
    image_activity_bq_per_ml: f64,
    injected_activity_bq: f64,
    body_weight_g: f64,
    decay_factor: f64,
) -> Result<f64, DicomLiteError> {
    if !image_activity_bq_per_ml.is_finite()
        || !injected_activity_bq.is_finite()
        || !body_weight_g.is_finite()
        || !decay_factor.is_finite()
    {
        return Err(DicomLiteError::InvalidParameter);
    }
    if image_activity_bq_per_ml < 0.0
        || injected_activity_bq <= 0.0
        || body_weight_g <= 0.0
        || decay_factor <= 0.0
        || decay_factor > 1.0
    {
        return Err(DicomLiteError::InvalidParameter);
    }

    let denom = injected_activity_bq * decay_factor;
    Ok(image_activity_bq_per_ml * body_weight_g / denom)
}

/// Convenience: SUV_bw from activity concentration, injected activity (Bq),
/// body weight in **kg**, and decay factor.
///
/// Internally converts kg → g.
pub fn suv_bw(
    image_activity_bq_per_ml: f64,
    injected_activity_bq: f64,
    body_weight_kg: f64,
    decay_factor: f64,
) -> Result<f64, DicomLiteError> {
    suv_from_activity(
        image_activity_bq_per_ml,
        injected_activity_bq,
        body_weight_kg * 1000.0,
        decay_factor,
    )
}

/// Compute radioactive decay factor `exp(-λ Δt)` for half-life `half_life_sec`
/// and delay `delta_t_sec` (imaging time − injection time).
pub fn decay_factor_from_half_life(half_life_sec: f64, delta_t_sec: f64) -> Result<f64, DicomLiteError> {
    if !(half_life_sec > 0.0) || !half_life_sec.is_finite() || !delta_t_sec.is_finite() {
        return Err(DicomLiteError::InvalidParameter);
    }
    if delta_t_sec < 0.0 {
        return Err(DicomLiteError::InvalidParameter);
    }
    let lambda = core::f64::consts::LN_2 / half_life_sec;
    Ok((-lambda * delta_t_sec).exp())
}

/// F-18 half-life in seconds (~109.77 min).
pub const F18_HALF_LIFE_SEC: f64 = 6586.2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_suv_value() {
        // C=5000 Bq/ml, A=100e6 Bq, W=70_000 g, decay=1 → SUV = 5000*70000/1e8 = 3.5
        let s = suv_from_activity(5000.0, 100_000_000.0, 70_000.0, 1.0).unwrap();
        assert!((s - 3.5).abs() < 1e-9);
    }

    #[test]
    fn suv_bw_kg_path() {
        let s = suv_bw(5000.0, 100_000_000.0, 70.0, 1.0).unwrap();
        assert!((s - 3.5).abs() < 1e-9);
    }

    #[test]
    fn decay_halves_activity() {
        let d = decay_factor_from_half_life(100.0, 100.0).unwrap();
        assert!((d - 0.5).abs() < 1e-9);
        let s = suv_from_activity(5000.0, 100_000_000.0, 70_000.0, d).unwrap();
        // decay 0.5 → SUV doubles vs decay=1
        assert!((s - 7.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_bad_decay() {
        assert!(suv_from_activity(1.0, 1.0, 1.0, 0.0).is_err());
        assert!(suv_from_activity(1.0, 1.0, 1.0, 1.1).is_err());
        assert!(suv_from_activity(1.0, 0.0, 1.0, 1.0).is_err());
    }

    #[test]
    fn f18_half_life_positive() {
        assert!(F18_HALF_LIFE_SEC > 6000.0 && F18_HALF_LIFE_SEC < 7000.0);
    }
}
