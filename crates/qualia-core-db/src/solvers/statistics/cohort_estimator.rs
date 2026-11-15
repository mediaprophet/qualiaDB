//! User-Selectable & User-Defined Cohort Graph Estimation Engine (C2–C3).
//! Tier-1 100% Zero-Heap kernel estimating invalid numeric observations over comparable cohorts.

use crate::frame_layout::pack_float_object;
use crate::q_hash;
use crate::NQuin;
use super::descriptive;

pub const P_ESTIMATED_VALUE: u64 = q_hash("urn:qualia:cohort:estimatedValue");
pub const P_ESTIMATE_LINEAGE: u64 = q_hash("urn:qualia:cohort:lineage");

/// 48-byte aligned observation record for cohort matching and estimation.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct CohortObservation {
    pub subject_hash: u64,
    pub predicate_hash: u64,
    pub taxon_or_domain_hash: u64,
    pub organ_or_feature_hash: u64,
    pub stage_or_season_hash: u64,
    pub unit_hash: u64,
    pub crs_or_frame_hash: u64,
    pub epoch_bucket: u64,
    pub context: u64, // bits 56..63 store sensitivity class (0=Public, 1=Restricted, 2=Classified)
    pub value: f64,
    pub is_valid: bool,
    pub raw_lexical_hash: u64,
}

impl CohortObservation {
    #[inline(always)]
    pub fn sensitivity_class(&self) -> u8 { (self.context >> 56) as u8 }

    #[inline(always)]
    pub fn with_sensitivity_class(mut self, sensitivity: u8) -> Self {
        self.context = (self.context & 0x00FF_FFFF_FFFF_FFFF) | ((sensitivity as u64) << 56);
        self
    }
}

/// Declared or user-defined statistical method for estimating missing values.
#[derive(Clone, Copy, Debug)]
pub enum CohortEstimationMethod {
    Median,
    TrimmedMean { trim_fraction: f64 },
    WinsorizedMean { limit_fraction: f64 },
    Mean,
    UserDefinedWeighted { weights_hash: u64 },
    CustomFn { method_hash: u64, eval: fn(&mut [f64]) -> f64 },
}

impl PartialEq for CohortEstimationMethod {
    fn eq(&self, other: &Self) -> bool {
        self.method_hash() == other.method_hash()
    }
}

impl CohortEstimationMethod {
    pub fn method_hash(&self) -> u64 {
        match *self {
            Self::Median => q_hash("urn:qualia:estimator:median"),
            Self::TrimmedMean { .. } => q_hash("urn:qualia:estimator:trimmed_mean"),
            Self::WinsorizedMean { .. } => q_hash("urn:qualia:estimator:winsorized_mean"),
            Self::Mean => q_hash("urn:qualia:estimator:mean"),
            Self::UserDefinedWeighted { weights_hash } => weights_hash,
            Self::CustomFn { method_hash, .. } => method_hash,
        }
    }
}

/// Query specification for filtering comparable cohort observations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CohortQuerySpec {
    pub min_cohort_size: usize,
    pub max_sensitivity_class: u8,
    pub match_taxon_or_domain: bool,
    pub match_organ_or_feature: bool,
    pub match_stage_or_season: bool,
    pub match_epoch_bucket: bool,
    pub strict_units: bool,
    pub strict_frames: bool,
}

impl Default for CohortQuerySpec {
    fn default() -> Self {
        Self {
            min_cohort_size: 3,
            max_sensitivity_class: 0,
            match_taxon_or_domain: true,
            match_organ_or_feature: true,
            match_stage_or_season: true,
            match_epoch_bucket: false,
            strict_units: true,
            strict_frames: true,
        }
    }
}

/// Output summary produced by cohort estimation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CohortEstimateResult {
    pub estimate: f64,
    pub uncertainty_mad: f64,
    pub uncertainty_iqr: f64,
    pub cohort_size: usize,
    pub method_hash: u64,
    pub inherited_sensitivity: u8,
}

/// Error conditions produced during cohort matching and estimation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CohortEstimationError {
    InsufficientCohort,
    IncompatibleUnits,
    IncompatibleFrames,
    SensitivityDenied,
    ScratchBufferTooSmall,
    InvalidParameter,
    InvalidEstimateValue,
    OutputBufferTooSmall,
}

/// Estimates an invalid observation over comparable cohort candidates (Tier-1 Zero-Heap).
pub fn estimate_invalid_observation_from_cohort(
    target: &CohortObservation,
    candidates: &[CohortObservation],
    spec: &CohortQuerySpec,
    method: CohortEstimationMethod,
    scratch_values: &mut [f64],
    out_quins: &mut [NQuin],
) -> Result<CohortEstimateResult, CohortEstimationError> {
    if out_quins.len() < 3 {
        return Err(CohortEstimationError::OutputBufferTooSmall);
    }
    let target_sens = target.sensitivity_class();
    if target_sens > spec.max_sensitivity_class {
        return Err(CohortEstimationError::SensitivityDenied);
    }

    let mut n_valid: usize = 0;
    let mut max_cohort_sensitivity = target_sens;

    for cand in candidates {
        if (spec.match_taxon_or_domain && cand.taxon_or_domain_hash != target.taxon_or_domain_hash)
            || (spec.match_organ_or_feature && cand.organ_or_feature_hash != target.organ_or_feature_hash)
            || (spec.match_stage_or_season && cand.stage_or_season_hash != target.stage_or_season_hash)
            || (spec.match_epoch_bucket && cand.epoch_bucket != target.epoch_bucket)
        {
            continue;
        }

        let cand_sens = cand.sensitivity_class();
        if cand_sens > spec.max_sensitivity_class {
            return Err(CohortEstimationError::SensitivityDenied);
        }
        if spec.strict_units && cand.unit_hash != target.unit_hash {
            return Err(CohortEstimationError::IncompatibleUnits);
        }
        if spec.strict_frames && cand.crs_or_frame_hash != target.crs_or_frame_hash {
            return Err(CohortEstimationError::IncompatibleFrames);
        }
        if !cand.is_valid || !cand.value.is_finite() {
            continue;
        }

        if cand_sens > max_cohort_sensitivity { max_cohort_sensitivity = cand_sens; }
        if scratch_values.len() < n_valid + 1 { return Err(CohortEstimationError::ScratchBufferTooSmall); }
        scratch_values[n_valid] = cand.value;
        n_valid += 1;
    }

    if n_valid < spec.min_cohort_size { return Err(CohortEstimationError::InsufficientCohort); }
    if scratch_values.len() < 2 * n_valid { return Err(CohortEstimationError::ScratchBufferTooSmall); }

    scratch_values[0..n_valid].sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let med = descriptive::median_sorted(&scratch_values[0..n_valid])
        .ok_or(CohortEstimationError::InsufficientCohort)?;

    for i in 0..n_valid { scratch_values[n_valid + i] = (scratch_values[i] - med).abs(); }
    scratch_values[n_valid..2 * n_valid].sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let raw_mad = descriptive::median_sorted(&scratch_values[n_valid..2 * n_valid]).unwrap_or(0.0);
    let uncertainty_mad = 1.482_602_218_505_602 * raw_mad;
    let q75 = descriptive::quantile_sorted(&scratch_values[0..n_valid], 0.75).unwrap_or(0.0);
    let q25 = descriptive::quantile_sorted(&scratch_values[0..n_valid], 0.25).unwrap_or(0.0);
    let uncertainty_iqr = q75 - q25;

    let estimate = match method {
        CohortEstimationMethod::Median => med,
        CohortEstimationMethod::Mean => descriptive::mean(&scratch_values[0..n_valid])
            .ok_or(CohortEstimationError::InsufficientCohort)?,
        CohortEstimationMethod::TrimmedMean { trim_fraction } => {
            if !(0.0..0.5).contains(&trim_fraction) || trim_fraction.is_nan() {
                return Err(CohortEstimationError::InvalidParameter);
            }
            let cut = (n_valid as f64 * trim_fraction).floor() as usize;
            if 2 * cut >= n_valid { med } else {
                descriptive::mean(&scratch_values[cut..n_valid - cut])
                    .ok_or(CohortEstimationError::InsufficientCohort)?
            }
        }
        CohortEstimationMethod::WinsorizedMean { limit_fraction } => {
            if !(0.0..0.5).contains(&limit_fraction) || limit_fraction.is_nan() {
                return Err(CohortEstimationError::InvalidParameter);
            }
            let cut = (n_valid as f64 * limit_fraction).floor() as usize;
            if 2 * cut >= n_valid { med } else {
                let (lo, hi) = (scratch_values[cut], scratch_values[n_valid - 1 - cut]);
                let sum: f64 = scratch_values[0..n_valid].iter()
                    .map(|&v| if v < lo { lo } else if v > hi { hi } else { v }).sum();
                sum / (n_valid as f64)
            }
        }
        CohortEstimationMethod::UserDefinedWeighted { weights_hash } => {
            if weights_hash == 0 { return Err(CohortEstimationError::InvalidParameter); }
            let (mut w_sum, mut val_sum) = (0.0, 0.0);
            for (i, &v) in scratch_values[0..n_valid].iter().enumerate() {
                let w = (i + 1) as f64;
                val_sum += w * v;
                w_sum += w;
            }
            if w_sum <= 0.0 { return Err(CohortEstimationError::InvalidEstimateValue); }
            val_sum / w_sum
        }
        CohortEstimationMethod::CustomFn { eval, .. } => {
            let res = (eval)(&mut scratch_values[0..n_valid]);
            if !res.is_finite() { return Err(CohortEstimationError::InvalidEstimateValue); }
            res
        }
    };
    if !estimate.is_finite() { return Err(CohortEstimationError::InvalidEstimateValue); }

    let inherited_sensitivity = max_cohort_sensitivity;
    let inherited_ctx = (target.context & 0x00FF_FFFF_FFFF_FFFF) | ((inherited_sensitivity as u64) << 56);

    let mut q0 = NQuin::default();
    q0.subject = target.subject_hash;
    q0.predicate = target.predicate_hash;
    q0.object = if target.raw_lexical_hash != 0 { target.raw_lexical_hash } else { pack_float_object(target.value as f32) };
    q0.context = target.context;
    q0.recalculate_parity();
    out_quins[0] = q0;

    let mut q1 = NQuin::default();
    q1.subject = target.subject_hash;
    q1.predicate = P_ESTIMATED_VALUE;
    q1.object = pack_float_object(estimate as f32);
    q1.context = inherited_ctx;
    q1.metadata = (method.method_hash() & 0xFFFF_FFFF) | ((inherited_sensitivity as u64) << 56);
    q1.recalculate_parity();
    out_quins[1] = q1;

    let mut q2 = NQuin::default();
    q2.subject = target.subject_hash;
    q2.predicate = P_ESTIMATE_LINEAGE;
    q2.object = pack_float_object(uncertainty_mad as f32);
    q2.context = inherited_ctx;
    q2.metadata = (n_valid as u64 & 0xFFFF_FFFF) | ((method.method_hash() & 0x00FF_FFFF) << 32) | ((inherited_sensitivity as u64) << 56);
    q2.recalculate_parity();
    out_quins[2] = q2;

    Ok(CohortEstimateResult {
        estimate,
        uncertainty_mad,
        uncertainty_iqr,
        cohort_size: n_valid,
        method_hash: method.method_hash(),
        inherited_sensitivity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    fn fixture(sub: u64, val: f64, valid: bool, unit: u64, sens: u8) -> CohortObservation {
        CohortObservation {
            subject_hash: sub,
            predicate_hash: q_hash("urn:qualia:trait:leaf_width"),
            taxon_or_domain_hash: q_hash("urn:lsid:biodiversity:taxon:42"),
            organ_or_feature_hash: q_hash("urn:qualia:organ:leaf"),
            stage_or_season_hash: q_hash("urn:qualia:stage:mature"),
            unit_hash: unit,
            crs_or_frame_hash: q_hash("urn:ogc:def:crs:EPSG::4326"),
            epoch_bucket: 2026,
            context: (sens as u64) << 56 | 0x1234,
            value: val,
            is_valid: valid,
            raw_lexical_hash: if valid { 0 } else { q_hash("raw_bad_input") },
        }
    }

    #[test]
    fn test_median_estimation_and_quin_lineage() {
        let u_mm = q_hash("urn:qualia:unit:millimetre");
        let target = fixture(1, 0.0, false, u_mm, 0);
        let cands = [
            fixture(2, 10.0, true, u_mm, 0), fixture(3, 12.0, true, u_mm, 0),
            fixture(4, 14.0, true, u_mm, 0), fixture(5, 16.0, true, u_mm, 0), fixture(6, 18.0, true, u_mm, 0),
        ];
        let mut scratch = [0.0; 32];
        let mut quins = [NQuin::default(); 3];

        let res = estimate_invalid_observation_from_cohort(
            &target, &cands, &CohortQuerySpec::default(),
            CohortEstimationMethod::Median, &mut scratch, &mut quins,
        ).expect("estimation should succeed");

        assert_eq!(res.cohort_size, 5);
        assert!((res.estimate - 14.0).abs() < 1e-6);
        assert_eq!(quins[0].object, target.raw_lexical_hash);
        assert_eq!(quins[1].predicate, P_ESTIMATED_VALUE);
        assert_eq!(quins[2].predicate, P_ESTIMATE_LINEAGE);
        assert!(quins[0].verify_ecc_parity() && quins[1].verify_ecc_parity() && quins[2].verify_ecc_parity());
    }

    #[test]
    fn test_zero_alloc_guarantee() {
        let u_mm = q_hash("urn:qualia:unit:millimetre");
        let target = fixture(1, 0.0, false, u_mm, 0);
        let cands = [fixture(2, 2.0, true, u_mm, 0), fixture(3, 4.0, true, u_mm, 0), fixture(4, 6.0, true, u_mm, 0)];
        let mut scratch = [0.0; 16];
        let mut quins = [NQuin::default(); 3];

        assert_zero_alloc("cohort_estimate_hot_path", || {
            let res = estimate_invalid_observation_from_cohort(
                &target, &cands, &CohortQuerySpec::default(),
                CohortEstimationMethod::Median, &mut scratch, &mut quins,
            );
            assert!(res.is_ok());
        });
    }

    #[test]
    fn test_insufficient_cohort_fails_closed() {
        let u_mm = q_hash("urn:qualia:unit:millimetre");
        let target = fixture(1, 0.0, false, u_mm, 0);
        let cands = [fixture(2, 10.0, true, u_mm, 0), fixture(3, 12.0, true, u_mm, 0)];
        let mut spec = CohortQuerySpec::default();
        spec.min_cohort_size = 3;
        let mut scratch = [0.0; 16];
        let mut quins = [NQuin::default(); 3];

        let err = estimate_invalid_observation_from_cohort(
            &target, &cands, &spec, CohortEstimationMethod::Median, &mut scratch, &mut quins,
        ).unwrap_err();
        assert_eq!(err, CohortEstimationError::InsufficientCohort);
    }

    #[test]
    fn test_incompatible_units_and_frames_fail_closed() {
        let (u_mm, u_m) = (q_hash("urn:qualia:unit:millimetre"), q_hash("urn:qualia:unit:metre"));
        let target = fixture(1, 0.0, false, u_mm, 0);
        let cands_unit = [
            fixture(2, 10.0, true, u_mm, 0), fixture(3, 12.0, true, u_mm, 0), fixture(4, 0.014, true, u_m, 0),
        ];
        let mut scratch = [0.0; 16];
        let mut quins = [NQuin::default(); 3];

        let err = estimate_invalid_observation_from_cohort(
            &target, &cands_unit, &CohortQuerySpec::default(),
            CohortEstimationMethod::Median, &mut scratch, &mut quins,
        ).unwrap_err();
        assert_eq!(err, CohortEstimationError::IncompatibleUnits);

        let mut diff_frame = fixture(4, 14.0, true, u_mm, 0);
        diff_frame.crs_or_frame_hash = q_hash("urn:ogc:def:crs:EPSG::3857");
        let cands_frame = [fixture(2, 10.0, true, u_mm, 0), fixture(3, 12.0, true, u_mm, 0), diff_frame];
        let err2 = estimate_invalid_observation_from_cohort(
            &target, &cands_frame, &CohortQuerySpec::default(),
            CohortEstimationMethod::Median, &mut scratch, &mut quins,
        ).unwrap_err();
        assert_eq!(err2, CohortEstimationError::IncompatibleFrames);
    }

    #[test]
    fn test_sensitivity_inheritance_and_denial() {
        let u_mm = q_hash("urn:qualia:unit:millimetre");
        let target = fixture(1, 0.0, false, u_mm, 0);
        let cands = [
            fixture(2, 10.0, true, u_mm, 0), fixture(3, 12.0, true, u_mm, 0), fixture(4, 14.0, true, u_mm, 1),
        ];
        let mut spec = CohortQuerySpec::default();
        spec.max_sensitivity_class = 1;
        let mut scratch = [0.0; 16];
        let mut quins = [NQuin::default(); 3];

        let res = estimate_invalid_observation_from_cohort(
            &target, &cands, &spec, CohortEstimationMethod::Mean, &mut scratch, &mut quins,
        ).expect("should succeed with clearance 1");

        assert_eq!(res.inherited_sensitivity, 1);
        assert_eq!(quins[1].get_sensitivity_byte(), 1);

        spec.max_sensitivity_class = 0;
        let err = estimate_invalid_observation_from_cohort(
            &target, &cands, &spec, CohortEstimationMethod::Mean, &mut scratch, &mut quins,
        ).unwrap_err();
        assert_eq!(err, CohortEstimationError::SensitivityDenied);
    }

    #[test]
    fn test_custom_function_pointer_and_robust_methods() {
        fn midrange(buf: &mut [f64]) -> f64 { (buf[0] + buf[buf.len() - 1]) / 2.0 }
        let u_mm = q_hash("urn:qualia:unit:millimetre");
        let target = fixture(1, 0.0, false, u_mm, 0);
        let cands = [
            fixture(2, 10.0, true, u_mm, 0), fixture(3, 20.0, true, u_mm, 0),
            fixture(4, 30.0, true, u_mm, 0), fixture(5, 1000.0, true, u_mm, 0),
        ];
        let mut scratch = [0.0; 16];
        let mut quins = [NQuin::default(); 3];

        let res = estimate_invalid_observation_from_cohort(
            &target, &cands, &CohortQuerySpec::default(),
            CohortEstimationMethod::CustomFn { method_hash: 0xC0FFEE, eval: midrange },
            &mut scratch, &mut quins,
        ).expect("custom fn eval");
        assert_eq!(res.method_hash, 0xC0FFEE);

        let res_tm = estimate_invalid_observation_from_cohort(
            &target, &cands, &CohortQuerySpec::default(),
            CohortEstimationMethod::TrimmedMean { trim_fraction: 0.25 },
            &mut scratch, &mut quins,
        ).expect("trimmed mean");
        assert!(res_tm.estimate < 50.0);
    }
}
