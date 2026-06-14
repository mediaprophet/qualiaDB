use serde::{Serialize, Deserialize};
use chrono::{DateTime, FixedOffset};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightRecord {
    pub uuid: String,
    pub weight: f64,
    pub body_fat: Option<f64>,
    pub muscle_mass: Option<f64>,
    pub body_water: Option<f64>,
    pub skeletal_muscle: Option<f64>,
    pub bmi: f64,
    pub start_datetime: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SleepRecord {
    pub uuid: String,
    pub sleep_duration: f64, // typically in minutes
    pub efficiency: f64,     // percent
    pub deep_sleep: Option<f64>,
    pub rem_sleep: Option<f64>,
    pub light_sleep: Option<f64>,
    pub wake_up_count: Option<u32>,
    pub start_datetime: DateTime<FixedOffset>,
    pub end_datetime: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartRateRecord {
    pub uuid: String,
    pub heart_rate: u32,
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub heart_rate_zone: Option<String>,
    pub binning_data: Option<String>,
    pub start_datetime: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepRecord {
    pub uuid: String,
    pub count: u32,
    pub calorie: f64,
    pub distance: f64,
    pub start_datetime: DateTime<FixedOffset>,
    pub end_datetime: DateTime<FixedOffset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacyMode {
    #[serde(rename = "MODE_A_RESTRICTED")]
    ModeARestricted,
    #[serde(rename = "MODE_B_PRIVILEGED")]
    ModeBPrivileged,
    #[serde(rename = "MODE_C_PUBLIC")]
    ModeCPublic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DiagnosticReportStatus {
    #[serde(rename = "preliminary")]
    Preliminary,
    #[serde(rename = "final")]
    Final,
    #[serde(rename = "amended")]
    Amended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathologyObservation {
    pub id: String,
    pub test_name: String,
    pub value: f64,
    pub unit: String,
    pub reference_range_low: Option<f64>,
    pub reference_range_high: Option<f64>,
    pub privacy_mode: PrivacyMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub id: String,
    pub patient_id: String,
    pub date_issued: DateTime<FixedOffset>,
    pub status: DiagnosticReportStatus,
    pub pdf_attachment_uri: String,
    pub observations: Vec<PathologyObservation>,
    pub privacy_mode: PrivacyMode,
    pub recorded_by_proxy_id: Option<String>,
}
