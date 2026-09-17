//! Biosignal differential-privacy filtering (T44).
//!
//! Host-side DP filtering for biosignal events (EEG, EMG, ECG, GSR).
//! Biosignal events are capability-leased and must be DP-filtered before
//! being exposed to VibeScript scripts.
//!
//! The filter applies Laplace noise to biosignal sample values, calibrated
//! to the L1 sensitivity of the signal and the configured epsilon budget.
//! The privacy budget is tracked across releases to prevent budget
//! exhaustion attacks.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.9 T44.

use crate::specialized_libs::linear_algebra::privacy::{
    CompositionMethod, DifferentialPrivacy, PrivacyError,
};
use vibe::{DiagCode, Diagnostic, Span, Value};

/// Configuration for biosignal DP filtering (T44).
#[derive(Debug, Clone)]
pub struct BiosignalDpConfig {
    /// Privacy budget (total epsilon across all releases).
    pub epsilon_budget: f64,
    /// Delta parameter for approximate DP (0 for pure DP).
    pub delta: f64,
    /// L1 sensitivity of a single biosignal sample.
    /// For normalized signals in [0, 1], this is 1.0.
    /// For raw EEG in microvolts, this is the expected range.
    pub l1_sensitivity: f64,
    /// Per-release epsilon fraction (0.01 = 1% of remaining budget).
    pub per_release_fraction: f64,
    /// Minimum remaining budget before refusing release.
    pub min_remaining: f64,
}

impl Default for BiosignalDpConfig {
    fn default() -> Self {
        Self {
            epsilon_budget: 1.0,
            delta: 1e-6,
            l1_sensitivity: 1.0,
            per_release_fraction: 0.01,
            min_remaining: 0.001,
        }
    }
}

impl BiosignalDpConfig {
    /// Create a strict DP config for clinical-grade biosignal filtering.
    pub fn clinical() -> Self {
        Self {
            epsilon_budget: 0.5,
            delta: 1e-6,
            l1_sensitivity: 1.0,
            per_release_fraction: 0.005,
            min_remaining: 0.0001,
        }
    }

    /// Create a relaxed DP config for non-sensitive biosignal research.
    pub fn research() -> Self {
        Self {
            epsilon_budget: 2.0,
            delta: 1e-4,
            l1_sensitivity: 1.0,
            per_release_fraction: 0.02,
            min_remaining: 0.01,
        }
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), PrivacyError> {
        if self.epsilon_budget <= 0.0 {
            return Err(PrivacyError::InvalidEpsilon);
        }
        if self.delta < 0.0 || self.delta >= 1.0 {
            return Err(PrivacyError::InvalidDelta);
        }
        if self.l1_sensitivity <= 0.0 {
            return Err(PrivacyError::InvalidSensitivity);
        }
        if self.per_release_fraction <= 0.0 || self.per_release_fraction > 1.0 {
            return Err(PrivacyError::InvalidEpsilon);
        }
        Ok(())
    }
}

/// Stateful biosignal DP filter (T44).
///
/// Tracks the privacy budget across releases. Each call to `filter_samples`
/// consumes a fraction of the remaining budget. When the budget is
/// exhausted, releases are refused (fail-closed).
pub struct BiosignalDpFilter {
    config: BiosignalDpConfig,
    dp: DifferentialPrivacy,
    releases: u64,
    total_samples: u64,
}

#[allow(dead_code)]
impl BiosignalDpFilter {
    /// Create a new biosignal DP filter with the given configuration.
    pub fn new(config: BiosignalDpConfig) -> Result<Self, PrivacyError> {
        config.validate()?;
        let dp = DifferentialPrivacy::with_budget(
            config.epsilon_budget,
            config.delta,
            CompositionMethod::BasicComposition,
        )?;
        Ok(Self {
            config,
            dp,
            releases: 0,
            total_samples: 0,
        })
    }

    /// Create a filter with default configuration.
    pub fn default_filter() -> Result<Self, PrivacyError> {
        Self::new(BiosignalDpConfig::default())
    }

    /// Create a clinical-grade filter.
    pub fn clinical() -> Result<Self, PrivacyError> {
        Self::new(BiosignalDpConfig::clinical())
    }

    /// Remaining privacy budget (epsilon).
    pub fn remaining_budget(&self) -> f64 {
        self.config.epsilon_budget - self.dp.privacy_accountant.total_epsilon_spent
    }

    /// Total epsilon consumed so far.
    pub fn consumed_budget(&self) -> f64 {
        self.dp.privacy_accountant.total_epsilon_spent
    }

    /// Number of releases performed.
    pub fn release_count(&self) -> u64 {
        self.releases
    }

    /// Total samples processed.
    pub fn total_samples_processed(&self) -> u64 {
        self.total_samples
    }

    /// Whether the filter has remaining budget.
    pub fn has_budget(&self) -> bool {
        self.remaining_budget() > self.config.min_remaining
    }

    /// Filter a batch of biosignal samples with DP noise (T44).
    ///
    /// Applies Laplace noise calibrated to the per-release epsilon
    /// (a fraction of the remaining budget). The filtered samples
    /// are written into the output buffer.
    ///
    /// Returns the number of samples filtered, or an error if the
    /// budget is exhausted or the buffers are mismatched.
    pub fn filter_samples(
        &mut self,
        samples: &[f64],
        out: &mut [f64],
    ) -> Result<usize, PrivacyError> {
        if samples.len() > out.len() {
            return Err(PrivacyError::OutputBufferTooSmall);
        }
        if samples.is_empty() {
            return Ok(0);
        }
        if !self.has_budget() {
            return Err(PrivacyError::BudgetExceeded);
        }

        // Calculate per-release epsilon: fraction of remaining budget
        let remaining = self.remaining_budget();
        let epsilon = remaining * self.config.per_release_fraction;
        if epsilon <= 0.0 || epsilon < 1e-15 {
            return Err(PrivacyError::BudgetExceeded);
        }

        // Release with Laplace noise
        let n = self
            .dp
            .release_laplace_into(samples, self.config.l1_sensitivity, epsilon, out)?;

        self.releases += 1;
        self.total_samples += n as u64;
        Ok(n)
    }

    /// Filter a single biosignal sample (convenience method).
    pub fn filter_sample(&mut self, sample: f64) -> Result<f64, PrivacyError> {
        let mut out = [0.0f64; 1];
        self.filter_samples(&[sample], &mut out)?;
        Ok(out[0])
    }

    /// Reset the filter (creates a fresh DP engine).
    /// This should only be called when starting a new session.
    pub fn reset(&mut self) -> Result<(), PrivacyError> {
        self.dp = DifferentialPrivacy::with_budget(
            self.config.epsilon_budget,
            self.config.delta,
            CompositionMethod::BasicComposition,
        )?;
        self.releases = 0;
        self.total_samples = 0;
        Ok(())
    }
}

/// VibeScript binding: biosignal.dp_filter(samples, config?) → filtered samples (T44).
pub fn dp_filter(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let args_list = match args {
        Value::List(l) => l,
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "biosignal.dp_filter expects a list argument",
            ));
        }
    };

    // First argument: list of samples
    let samples: Vec<f64> = match args_list.first() {
        Some(Value::List(l)) => l.iter().filter_map(|v| v.as_f64()).collect(),
        _ => {
            return Err(Diagnostic::new(
                DiagCode::E100,
                span,
                "biosignal.dp_filter first argument must be a list of numbers",
            ));
        }
    };

    // Optional second argument: config record with epsilon_budget, etc.
    let config = match args_list.get(1) {
        Some(Value::Record(r)) => {
            let mut cfg = BiosignalDpConfig::default();
            if let Some(Value::F64(e)) = r.get("epsilon_budget") {
                cfg.epsilon_budget = *e;
            }
            if let Some(Value::F64(d)) = r.get("delta") {
                cfg.delta = *d;
            }
            if let Some(Value::F64(s)) = r.get("l1_sensitivity") {
                cfg.l1_sensitivity = *s;
            }
            if let Some(Value::F64(f)) = r.get("per_release_fraction") {
                cfg.per_release_fraction = *f;
            }
            cfg
        }
        _ => BiosignalDpConfig::default(),
    };

    let mut filter = BiosignalDpFilter::new(config).map_err(|e| {
        Diagnostic::new(
            DiagCode::E100,
            span,
            format!("DP filter config error: {e:?}"),
        )
    })?;

    let mut out = vec![0.0f64; samples.len()];
    let n = filter
        .filter_samples(&samples, &mut out)
        .map_err(|e| Diagnostic::new(DiagCode::E100, span, format!("DP filter error: {e:?}")))?;

    out.truncate(n);
    Ok(Value::List(out.into_iter().map(Value::F64).collect()))
}

/// VibeScript binding: biosignal.dp_config() → default config record (T44).
pub fn dp_config(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let args_list = match args {
        Value::List(l) => l,
        _ => return Ok(default_config_value()),
    };

    let preset = match args_list.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Ok(default_config_value()),
    };

    let cfg = match preset {
        "clinical" => BiosignalDpConfig::clinical(),
        "research" => BiosignalDpConfig::research(),
        _ => BiosignalDpConfig::default(),
    };

    Ok(config_to_value(&cfg))
}

fn default_config_value() -> Value {
    config_to_value(&BiosignalDpConfig::default())
}

fn config_to_value(cfg: &BiosignalDpConfig) -> Value {
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("epsilon_budget".into(), Value::F64(cfg.epsilon_budget));
    rec.insert("delta".into(), Value::F64(cfg.delta));
    rec.insert("l1_sensitivity".into(), Value::F64(cfg.l1_sensitivity));
    rec.insert(
        "per_release_fraction".into(),
        Value::F64(cfg.per_release_fraction),
    );
    rec.insert("min_remaining".into(), Value::F64(cfg.min_remaining));
    Value::Record(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t44_filter_default() {
        let mut filter = BiosignalDpFilter::default_filter().unwrap();
        let samples = vec![0.5, 0.6, 0.7, 0.8, 0.9];
        let mut out = vec![0.0; 5];
        let n = filter.filter_samples(&samples, &mut out).unwrap();
        assert_eq!(n, 5);
        // Output should be finite (noise added, but values should be real numbers)
        for i in 0..5 {
            assert!(out[i].is_finite(), "output {} = {} not finite", i, out[i]);
        }
        assert_eq!(filter.release_count(), 1);
        assert_eq!(filter.total_samples_processed(), 5);
    }

    #[test]
    fn t44_filter_single_sample() {
        let mut filter = BiosignalDpFilter::default_filter().unwrap();
        let result = filter.filter_sample(0.5).unwrap();
        assert!(result.is_finite());
        assert_eq!(filter.release_count(), 1);
    }

    #[test]
    fn t44_budget_tracking() {
        let mut filter = BiosignalDpFilter::default_filter().unwrap();
        let initial = filter.remaining_budget();
        assert!(initial > 0.0);

        let samples = vec![0.5; 10];
        let mut out = vec![0.0; 10];
        filter.filter_samples(&samples, &mut out).unwrap();

        let after = filter.remaining_budget();
        assert!(
            after < initial,
            "budget should decrease: {} -> {}",
            initial,
            after
        );
        assert!(filter.consumed_budget() > 0.0);
    }

    #[test]
    fn t44_budget_exhaustion() {
        let config = BiosignalDpConfig {
            epsilon_budget: 0.01,
            delta: 1e-6,
            l1_sensitivity: 1.0,
            per_release_fraction: 0.5, // Fast consumption
            min_remaining: 0.001,
        };
        let mut filter = BiosignalDpFilter::new(config).unwrap();

        let samples = vec![0.5; 5];
        let mut out = vec![0.0; 5];

        // Eventually budget should be exhausted
        for _ in 0..20 {
            if filter.has_budget() {
                let _ = filter.filter_samples(&samples, &mut out);
            } else {
                break;
            }
        }
        assert!(!filter.has_budget(), "budget should be exhausted");

        // Further releases should fail (fail-closed)
        let result = filter.filter_samples(&samples, &mut out);
        assert!(result.is_err(), "should fail when budget exhausted");
    }

    #[test]
    fn t44_clinical_config() {
        let filter = BiosignalDpFilter::clinical().unwrap();
        assert!(filter.remaining_budget() > 0.0);
        assert!(filter.remaining_budget() <= 0.5);
    }

    #[test]
    fn t44_research_config() {
        let filter = BiosignalDpFilter::new(BiosignalDpConfig::research()).unwrap();
        assert!(filter.remaining_budget() > 0.5);
    }

    #[test]
    fn t44_reset() {
        let mut filter = BiosignalDpFilter::default_filter().unwrap();
        let samples = vec![0.5; 5];
        let mut out = vec![0.0; 5];
        filter.filter_samples(&samples, &mut out).unwrap();
        assert!(filter.consumed_budget() > 0.0);

        filter.reset().unwrap();
        assert_eq!(filter.consumed_budget(), 0.0);
        assert_eq!(filter.release_count(), 0);
    }

    #[test]
    fn t44_empty_samples() {
        let mut filter = BiosignalDpFilter::default_filter().unwrap();
        let n = filter.filter_samples(&[], &mut []).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn t44_config_validation() {
        let bad = BiosignalDpConfig {
            epsilon_budget: -1.0, // Invalid
            ..Default::default()
        };
        assert!(BiosignalDpFilter::new(bad).is_err());
    }

    #[test]
    fn t44_vibescript_dp_filter() {
        let args = Value::List(vec![Value::List(vec![
            Value::F64(0.5),
            Value::F64(0.6),
            Value::F64(0.7),
        ])]);
        let result = dp_filter(&args, Span::point(0)).unwrap();
        match result {
            Value::List(l) => assert_eq!(l.len(), 3),
            other => panic!("expected list, got {other:?}"),
        }
    }

    #[test]
    fn t44_vibescript_dp_config_default() {
        let args = Value::List(vec![]);
        let result = dp_config(&args, Span::point(0)).unwrap();
        match result {
            Value::Record(r) => {
                assert!(r.contains_key("epsilon_budget"));
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn t44_vibescript_dp_config_clinical() {
        let args = Value::List(vec![Value::String("clinical".into())]);
        let result = dp_config(&args, Span::point(0)).unwrap();
        match result {
            Value::Record(r) => {
                if let Some(Value::F64(e)) = r.get("epsilon_budget") {
                    assert!(*e <= 0.5);
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }
}
