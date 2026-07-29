use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

use super::ExecutionReceipt;

pub const MANIFEST_SCHEMA_VERSION: u16 = 3;
pub const RAW_GREEDY_DECODE_POLICY: &str =
    "greedy-argmax;temperature=0;top_k=1;top_p=1;repeat_penalty=1;repeat_last_n=0;eos=ignored";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkManifest {
    pub schema_version: u16,
    pub benchmark_kind: String,
    pub executable_commit: String,
    pub dirty_diff_hash: String,
    pub executable_sha256: String,
    pub model_path: String,
    pub model_sha256: String,
    pub prompt_token_sha256: String,
    /// Number of tokens produced by the declared prompt before decode begins.
    #[serde(default)]
    pub prompt_tokens: u32,
    /// Prepared runtime context capacity used for this run.
    #[serde(default)]
    pub context_window: u32,
    /// Complete sampling/termination contract. Comparator runs must declare an identical policy.
    #[serde(default)]
    pub decode_policy: String,
    pub quantization: String,
    pub decode_steps_requested: u32,
    pub decode_steps_executed: u32,
    pub warmup_runs: u16,
    pub measured_runs: u16,
    pub median_tok_s: f64,
    pub p95_ms_per_token: f64,
    pub receipt: ExecutionReceipt,
}

impl BenchmarkManifest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != MANIFEST_SCHEMA_VERSION {
            return Err("unsupported benchmark manifest schema");
        }
        if self.benchmark_kind.is_empty() {
            return Err("benchmark kind is required");
        }
        if self.model_sha256.len() != 64
            || self.prompt_token_sha256.len() != 64
            || self.executable_sha256.len() != 64
        {
            return Err("SHA-256 fields must contain 64 hexadecimal characters");
        }
        if self.measured_runs == 0 {
            return Err("at least one measured run is required");
        }
        if self.decode_steps_executed == 0 {
            return Err("at least one decode step is required");
        }
        if self.prompt_tokens == 0 || self.context_window == 0 {
            return Err("prompt token count and context window are required");
        }
        if self.decode_policy.is_empty() {
            return Err("decode policy is required");
        }
        if self
            .prompt_tokens
            .saturating_add(self.decode_steps_executed)
            > self.context_window
        {
            return Err("prompt plus decode budget exceeds the prepared context window");
        }
        if !self.median_tok_s.is_finite() || !self.p95_ms_per_token.is_finite() {
            return Err("benchmark rates must be finite");
        }
        Ok(())
    }
}

pub fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn sha256_token_ids(token_ids: &[u32]) -> String {
    let mut hasher = Sha256::new();
    for token_id in token_ids {
        hasher.update(token_id.to_le_bytes());
    }
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference::runtime::receipt::BackendKind;

    #[test]
    fn manifest_round_trip_and_validation() {
        let manifest = BenchmarkManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            benchmark_kind: "raw-decode".into(),
            executable_commit: "abc".into(),
            dirty_diff_hash: "def".into(),
            executable_sha256: "c".repeat(64),
            model_path: "model.gguf".into(),
            model_sha256: "a".repeat(64),
            prompt_token_sha256: "b".repeat(64),
            prompt_tokens: 32,
            context_window: 1024,
            decode_policy: RAW_GREEDY_DECODE_POLICY.into(),
            quantization: "Q8_0".into(),
            decode_steps_requested: 256,
            decode_steps_executed: 256,
            warmup_runs: 1,
            measured_runs: 5,
            median_tok_s: 100.0,
            p95_ms_per_token: 11.0,
            receipt: ExecutionReceipt::new(BackendKind::Cuda, BackendKind::Cuda, "model", "plan"),
        };
        manifest.validate().unwrap();
        let json = serde_json::to_string(&manifest).unwrap();
        let decoded: BenchmarkManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, manifest);
    }

    #[test]
    fn manifest_rejects_undeclared_decode_policy() {
        let mut manifest = BenchmarkManifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            benchmark_kind: "raw-decode".into(),
            executable_commit: "abc".into(),
            dirty_diff_hash: "def".into(),
            executable_sha256: "c".repeat(64),
            model_path: "model.gguf".into(),
            model_sha256: "a".repeat(64),
            prompt_token_sha256: "b".repeat(64),
            prompt_tokens: 5,
            context_window: 1024,
            decode_policy: RAW_GREEDY_DECODE_POLICY.into(),
            quantization: "Q8_0".into(),
            decode_steps_requested: 256,
            decode_steps_executed: 256,
            warmup_runs: 1,
            measured_runs: 5,
            median_tok_s: 100.0,
            p95_ms_per_token: 11.0,
            receipt: ExecutionReceipt::new(BackendKind::Cuda, BackendKind::Cuda, "model", "plan"),
        };
        manifest.decode_policy.clear();
        assert_eq!(manifest.validate(), Err("decode policy is required"));
    }
}
