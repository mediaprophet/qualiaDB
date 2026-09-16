//! Benchmark manifest types and loader (NLP-004).
//!
//! Lives in the integration-test tree because Cargo.toml / src/nlp are frozen.

use super::json_lite::{parse_json, Json, JsonError};

pub const FROZEN_EVAL_SEED: u64 = 42;
pub const FROZEN_SEED_POLICY: &str = "frozen-u64-42";
pub const DUMMY_CLOCK: &str = "1970-01-01T00:00:00Z";
pub const SOURCE_REVISION_UNCOMMITTED: &str = "uncommitted";

pub const SMOKE_MANIFEST_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/nlp-004-smoke-tokenize-en.json"
));

pub const UNAVAILABLE_MANIFEST_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/nlp-004-ud-en-ewt-unavailable.json"
));

pub const SCHEMA_EXAMPLE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../benchmarks/nlp/manifests/schema.example.json"
));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateStatus {
    Unevaluated,
    Unavailable,
    Blocked,
    Fail,
    Pass,
}

impl GateStatus {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "UNEVALUATED" => Ok(Self::Unevaluated),
            "unavailable" => Ok(Self::Unavailable),
            "blocked" => Ok(Self::Blocked),
            "fail" => Ok(Self::Fail),
            "pass" => Ok(Self::Pass),
            other => Err(format!("unknown gate_status {other:?}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unevaluated => "UNEVALUATED",
            Self::Unavailable => "unavailable",
            Self::Blocked => "blocked",
            Self::Fail => "fail",
            Self::Pass => "pass",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEnvelope {
    pub max_source_bytes: Option<u64>,
    pub max_working_memory_bytes: Option<u64>,
    pub network: String,
    pub download_datasets: bool,
    pub download_models: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkManifest {
    pub id: String,
    pub task: String,
    pub language: String,
    pub dataset_id: String,
    pub dataset_revision: Option<String>,
    pub split: String,
    pub split_hash: Option<String>,
    pub domain: Option<String>,
    pub profile: Option<String>,
    pub label_mapping: Option<String>,
    pub scorer_revision: Option<String>,
    pub primary_metric: Option<String>,
    pub metric_names: Vec<String>,
    pub minimum_score: Option<f64>,
    pub achieved_score: Option<f64>,
    pub baseline_score: Option<f64>,
    pub noninferiority_margin: Option<f64>,
    pub minimum_effect: Option<f64>,
    pub confidence_method: Option<String>,
    pub coverage_floor: Option<f64>,
    pub latency_budget: Option<String>,
    pub memory_budget: Option<String>,
    pub release_required: bool,
    pub gate_status: GateStatus,
    pub unavailable_reason: Option<String>,
    pub seed: u64,
    pub seed_policy: String,
    pub resource_envelope: ResourceEnvelope,
    pub corpus: Vec<String>,
    pub notes: Option<String>,
}

pub fn load_manifest(json_text: &str) -> Result<BenchmarkManifest, String> {
    let root = parse_json(json_text).map_err(|JsonError(m)| m)?;
    let obj = root.as_object().ok_or("manifest must be a JSON object")?;

    let req_str = |key: &str| -> Result<String, String> {
        match obj.get(key) {
            Some(Json::String(s)) if !s.is_empty() => Ok(s.clone()),
            _ => Err(format!("missing or empty string field {key}")),
        }
    };
    let opt_str = |key: &str| -> Result<Option<String>, String> {
        match obj.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::String(s)) => Ok(Some(s.clone())),
            _ => Err(format!("{key} must be string or null")),
        }
    };
    let opt_num = |key: &str| -> Result<Option<f64>, String> {
        match obj.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::Int(n)) => Ok(Some(*n as f64)),
            Some(Json::Float(x)) => Ok(Some(*x)),
            _ => Err(format!("{key} must be number or null")),
        }
    };

    let gate_status = GateStatus::parse(&req_str("gate_status")?)?;
    let unavailable_reason = opt_str("unavailable_reason")?;
    if gate_status == GateStatus::Unavailable {
        match &unavailable_reason {
            Some(r) if r.len() >= 8 => {}
            _ => {
                return Err("unavailable manifests require unavailable_reason (≥8 chars)".into());
            }
        }
    }

    let metric_names = match obj.get("metric_names") {
        Some(Json::Array(items)) if !items.is_empty() => {
            let mut names = Vec::new();
            for item in items {
                let s = item
                    .as_str()
                    .ok_or("metric_names entries must be strings")?;
                if s.is_empty() {
                    return Err("empty metric name".into());
                }
                names.push(s.to_string());
            }
            names
        }
        _ => return Err("metric_names must be a non-empty array".into()),
    };

    let seed = obj
        .get("seed")
        .and_then(Json::as_i64)
        .ok_or("seed must be an integer")?;
    if seed != FROZEN_EVAL_SEED as i64 {
        return Err(format!(
            "seed must be the frozen value {FROZEN_EVAL_SEED}, got {seed}"
        ));
    }
    let seed_policy = req_str("seed_policy")?;
    if seed_policy != FROZEN_SEED_POLICY {
        return Err(format!(
            "seed_policy must be {FROZEN_SEED_POLICY:?}, got {seed_policy:?}"
        ));
    }

    let envelope_obj = obj
        .get("resource_envelope")
        .and_then(Json::as_object)
        .ok_or("resource_envelope must be an object")?;
    let env_str = |key: &str| -> Result<String, String> {
        match envelope_obj.get(key) {
            Some(Json::String(s)) => Ok(s.clone()),
            _ => Err(format!("resource_envelope.{key} must be a string")),
        }
    };
    let env_bool = |key: &str| -> Result<bool, String> {
        match envelope_obj.get(key) {
            Some(Json::Bool(b)) => Ok(*b),
            _ => Err(format!("resource_envelope.{key} must be a boolean")),
        }
    };
    let env_u64 = |key: &str| -> Result<Option<u64>, String> {
        match envelope_obj.get(key) {
            None | Some(Json::Null) => Ok(None),
            Some(Json::Int(n)) if *n >= 0 => Ok(Some(*n as u64)),
            _ => Err(format!(
                "resource_envelope.{key} must be non-negative integer or null"
            )),
        }
    };
    let network = env_str("network")?;
    if network != "forbidden" {
        return Err("resource_envelope.network must be \"forbidden\" (no CI downloads)".into());
    }
    let download_datasets = env_bool("download_datasets")?;
    let download_models = env_bool("download_models")?;
    if download_datasets || download_models {
        return Err("download_datasets and download_models must be false".into());
    }

    let corpus = match obj.get("corpus") {
        None | Some(Json::Null) => Vec::new(),
        Some(Json::Array(items)) => {
            let mut lines = Vec::new();
            for item in items {
                lines.push(
                    item.as_str()
                        .ok_or("corpus entries must be strings")?
                        .to_string(),
                );
            }
            lines
        }
        _ => return Err("corpus must be an array of strings or null".into()),
    };

    let release_required = match obj.get("release_required") {
        Some(Json::Bool(b)) => *b,
        _ => return Err("release_required must be a boolean".into()),
    };

    Ok(BenchmarkManifest {
        id: req_str("id")?,
        task: req_str("task")?,
        language: req_str("language")?,
        dataset_id: req_str("dataset_id")?,
        dataset_revision: opt_str("dataset_revision")?,
        split: req_str("split")?,
        split_hash: opt_str("split_hash")?,
        domain: opt_str("domain")?,
        profile: opt_str("profile")?,
        label_mapping: opt_str("label_mapping")?,
        scorer_revision: opt_str("scorer_revision")?,
        primary_metric: opt_str("primary_metric")?,
        metric_names,
        minimum_score: opt_num("minimum_score")?,
        achieved_score: opt_num("achieved_score")?,
        baseline_score: opt_num("baseline_score")?,
        noninferiority_margin: opt_num("noninferiority_margin")?,
        minimum_effect: opt_num("minimum_effect")?,
        confidence_method: opt_str("confidence_method")?,
        coverage_floor: opt_num("coverage_floor")?,
        latency_budget: opt_str("latency_budget")?,
        memory_budget: opt_str("memory_budget")?,
        release_required,
        gate_status,
        unavailable_reason,
        seed: seed as u64,
        seed_policy,
        resource_envelope: ResourceEnvelope {
            max_source_bytes: env_u64("max_source_bytes")?,
            max_working_memory_bytes: env_u64("max_working_memory_bytes")?,
            network,
            download_datasets,
            download_models,
        },
        corpus,
        notes: opt_str("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_and_example_manifests_load() {
        let smoke = load_manifest(SMOKE_MANIFEST_JSON).expect("smoke manifest");
        assert_eq!(smoke.id, "nlp-004-smoke-tokenize-en");
        assert_eq!(smoke.gate_status, GateStatus::Unevaluated);
        assert!(!smoke.release_required);
        assert_eq!(smoke.seed, FROZEN_EVAL_SEED);
        assert_eq!(smoke.corpus.len(), 3);
        assert!(smoke.corpus[0].contains("North Spring"));

        let example = load_manifest(SCHEMA_EXAMPLE_JSON).expect("schema example");
        assert_eq!(example.gate_status, GateStatus::Unevaluated);
        assert!(example.minimum_score.is_none());
        assert!(example.achieved_score.is_none());
    }

    #[test]
    fn achieved_score_loads_from_json() {
        let json = r#"{
  "id": "nlp-006-achieved-score-json",
  "task": "tokenize",
  "language": "en",
  "dataset_id": "qualia-english-notes-v0",
  "split": "smoke",
  "metric_names": ["span_in_source"],
  "minimum_score": 0.99,
  "achieved_score": 0.5,
  "release_required": false,
  "gate_status": "fail",
  "unavailable_reason": null,
  "seed": 42,
  "seed_policy": "frozen-u64-42",
  "resource_envelope": {
    "network": "forbidden",
    "download_datasets": false,
    "download_models": false
  },
  "corpus": ["ok."]
}"#;
        let m = load_manifest(json).expect("achieved_score JSON");
        assert_eq!(m.minimum_score, Some(0.99));
        assert_eq!(m.achieved_score, Some(0.5));
        assert_eq!(m.gate_status, GateStatus::Fail);
    }

    #[test]
    fn unavailable_stub_loads_with_reason() {
        let m = load_manifest(UNAVAILABLE_MANIFEST_JSON).expect("unavailable stub");
        assert_eq!(m.gate_status, GateStatus::Unavailable);
        assert!(m.release_required);
        assert!(m.unavailable_reason.as_ref().unwrap().contains("licensed"));
        assert!(m.corpus.is_empty());
        assert!(!m.resource_envelope.download_datasets);
    }
}
