//! Normalized response outcomes and measurement coverage.
//! Unknown usage != zero usage.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MeasurementStatus {
    Measured,
    Estimated,
    Unknown,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedInferenceResult {
    pub text: String,
    pub committed: bool,
    pub model_id: Option<String>,
    pub tokens_generated: Option<u32>,
    pub token_measurement: MeasurementStatus,
    pub latency_ms: Option<u64>,
    pub latency_measurement: MeasurementStatus,
    pub citations: Vec<String>,
    pub requirement_receipts_passed: bool,
}

impl NormalizedInferenceResult {
    pub fn new_unmeasured(text: String, committed: bool) -> Self {
        Self {
            text,
            committed,
            model_id: None,
            tokens_generated: None,
            token_measurement: MeasurementStatus::Unknown,
            latency_ms: None,
            latency_measurement: MeasurementStatus::Unknown,
            citations: Vec::new(),
            requirement_receipts_passed: true,
        }
    }

    pub fn new_measured(
        text: String,
        committed: bool,
        tokens: u32,
        duration_ms: u64,
        citations: Vec<String>,
    ) -> Self {
        Self {
            text,
            committed,
            model_id: None,
            tokens_generated: Some(tokens),
            token_measurement: MeasurementStatus::Measured,
            latency_ms: Some(duration_ms),
            latency_measurement: MeasurementStatus::Measured,
            citations,
            requirement_receipts_passed: true,
        }
    }
}
