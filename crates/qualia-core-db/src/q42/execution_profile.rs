//! P64 / Q42 **execution profile** — attested how to run a **native package at excellence**.
//!
//! QualiaDB is a human-centric multi-capability system. Inference packages are one toolchain
//! lane. The profile exists so autonomous campaigns optimise **speed and sense** (and later
//! grounding/rights), not so agents ship half-working “skeleton” products.
//!
//! ## On disk
//! Sibling of a winner `.p64`: `{stem}.execution-profile.json`

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Schema version for campaign tooling (bump when fields change meaning).
pub const EXECUTION_PROFILE_VERSION: u32 = 1;

/// File suffix: `model.f16.p64` → `model.f16.execution-profile.json` (stem-preserving).
pub const EXECUTION_PROFILE_SUFFIX: &str = "execution-profile.json";

/// Attested run recipe for a converted native weight package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutionProfile {
    pub version: u32,
    /// ISO-ish stamp (unix ms) when this profile was written.
    pub written_unix_ms: u64,
    /// Source import path if known (GGUF/Safetensors); empty if measured from P64 only.
    pub source_import: String,
    /// Absolute or operator-local path to the winning `.p64`.
    pub p64_path: String,
    /// Optional `.q42` model-helper path.
    #[serde(default)]
    pub q42_helper_path: String,
    /// Convert layout: `verbatim` | `f16` | `soa` | …
    pub layout: String,
    /// `portable` | `cuda` | `quant-graph` | `fast-verify`
    pub inference_mode: String,
    /// wgpu backend hint (`auto` | `vulkan` | `dx12` | …).
    pub backend: String,
    /// Runtime toggles applied or recommended for this package.
    pub toggles: ExecutionToggles,
    /// Measured evidence (honest; may be partial).
    pub metrics: ExecutionMetrics,
    /// Representation levers available or used (not all active at once).
    pub representation: RepresentationNotes,
    /// Human/agent notes — failures, next steps, human decisions needed.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Multi-objective scores (optional; fill as gates land).
    #[serde(default)]
    pub objectives: ObjectiveScores,
}

/// Runtime knobs that compose with InferenceMode (env-applied at load).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExecutionToggles {
    /// W5a int8 KV cache (native; ~3.8× less KV BW when on).
    pub kv_int8: bool,
    /// Resident mega-pass decode when available.
    pub resident_decode: bool,
    /// Cooperative GEMV.
    pub coop_gemv: bool,
    /// FFN fusion in resident path.
    pub ffn_fusion: bool,
    /// Promote FFN weights to f16 in VRAM when measuring.
    pub ffn_f16: bool,
    /// Ternary (BitNet-class ~1.58b / ≈1.6 bit/weight) FFN path when container has it.
    pub ternary_ffn: bool,
}

/// Measured numbers — never invent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExecutionMetrics {
    /// Decode-proxy tokens/s when measured.
    pub decode_proxy_tok_s: Option<f64>,
    /// Token budget used for that measurement.
    pub decode_proxy_tokens: u32,
    /// Package size on disk (bytes).
    pub p64_bytes: u64,
    /// Optional coherence flag from a later gate (null until implemented).
    pub coherence_ok: Option<bool>,
    /// Optional ΔPPL vs reference (null until calibrated).
    pub delta_ppl: Option<f64>,
}

/// Documents which representation levers exist in the ecosystem (matrix, not a single format).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RepresentationNotes {
    /// Weight quant / layout family used for this package.
    pub weight_path: String,
    /// KV cache class recommended: `int8` | `f32` | `dict`.
    pub kv_class: String,
    /// BitNet ternary ~1.58b (code type 1158); FFN-only when present.
    pub ternary_158_available: bool,
    /// f16 expand layout was candidate or winner.
    pub f16_layout: bool,
    /// Q4_K SoA layout was candidate or winner.
    pub soa_layout: bool,
}

/// Multi-objective matrix (incremental fill). Missing = not yet measured.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ObjectiveScores {
    /// A — throughput (tok/s class).
    pub throughput: Option<f64>,
    /// B — format/layout fitness (operator 0..1 or free note in metrics).
    pub format_fitness: Option<f64>,
    /// C — text/coherence / ΔPPL gate.
    pub correctness: Option<f64>,
    /// D — grounding / tools / graph (later).
    pub grounding: Option<f64>,
    /// E — rights / governance path exercised.
    pub governance: Option<f64>,
    /// F — resource (inverse memory pressure; later).
    pub resource: Option<f64>,
}

impl ExecutionProfile {
    pub fn path_for_p64(p64_path: &Path) -> PathBuf {
        let parent = p64_path.parent().unwrap_or_else(|| Path::new("."));
        let stem = p64_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model");
        parent.join(format!("{stem}.{EXECUTION_PROFILE_SUFFIX}"))
    }

    pub fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Write JSON beside the P64 (atomic-ish: write temp then rename when possible).
    pub fn write_beside_p64(&self, p64_path: &Path) -> Result<PathBuf, String> {
        let path = Self::path_for_p64(p64_path);
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {e}"))?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json.as_bytes())
            .map_err(|e| format!("write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &path).or_else(|_| {
            std::fs::write(&path, json.as_bytes())
                .map_err(|e| format!("write {}: {e}", path.display()))
        })?;
        Ok(path)
    }

    pub fn load_beside_p64(p64_path: &Path) -> Result<Option<Self>, String> {
        let path = Self::path_for_p64(p64_path);
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let p: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {e}", path.display()))?;
        Ok(Some(p))
    }

    /// Build a profile from an explore winner (minimal attested fields).
    pub fn from_explore_winner(
        source_import: &str,
        p64_path: &Path,
        layout: &str,
        inference_mode: &str,
        backend: &str,
        tok_s: f64,
        tokens: u32,
        toggle_label: &str,
    ) -> Self {
        let p64_bytes = std::fs::metadata(p64_path).map(|m| m.len()).unwrap_or(0);
        let ffn_f16 = toggle_label.contains("ffn_f16=on");
        let layout_l = layout.to_ascii_lowercase();
        let helper = crate::q42::model_helper::helper_path_for_p64(p64_path);
        let q42_helper_path = if helper.is_file() {
            helper.display().to_string()
        } else {
            String::new()
        };

        // Snapshot ambient toggles (best-effort; campaign may pin env).
        let kv_int8 = crate::llm_bench::kv_int8_enabled();
        let resident_decode = crate::llm_bench::resident_decode_enabled();
        let coop_gemv = crate::llm_bench::coop_gemv_enabled();
        let ffn_fusion = crate::llm_bench::ffn_fusion_in_resident();
        let ternary_ffn = crate::llm_bench::ternary_ffn_enabled();

        Self {
            version: EXECUTION_PROFILE_VERSION,
            written_unix_ms: Self::now_ms(),
            source_import: source_import.to_string(),
            p64_path: p64_path.display().to_string(),
            q42_helper_path,
            layout: layout.to_string(),
            inference_mode: inference_mode.to_string(),
            backend: backend.to_string(),
            toggles: ExecutionToggles {
                kv_int8,
                resident_decode,
                coop_gemv,
                ffn_fusion,
                ffn_f16,
                ternary_ffn,
            },
            metrics: ExecutionMetrics {
                decode_proxy_tok_s: Some(tok_s),
                decode_proxy_tokens: tokens,
                p64_bytes,
                coherence_ok: None,
                delta_ppl: None,
            },
            representation: RepresentationNotes {
                weight_path: layout.to_string(),
                kv_class: if kv_int8 { "int8".into() } else { "f32".into() },
                ternary_158_available: false, // filled true when convert emits ternary FFN
                f16_layout: layout_l.contains("f16"),
                soa_layout: layout_l.contains("soa"),
            },
            notes: vec![
                "Target: competitive native package (layout×mode×toggles) with coherent decode."
                    .into(),
                "BitNet ternary ~1.58b (≈1.6 bits/weight, type 1158) is an FFN compression lever when present."
                    .into(),
                format!("explore toggle label: {toggle_label}"),
            ],
            objectives: ObjectiveScores {
                throughput: Some(tok_s),
                format_fitness: None,
                correctness: None,
                grounding: None,
                governance: None,
                resource: None,
            },
        }
    }

    /// Env lines a campaign / operator can apply before load (PowerShell-friendly comments separate).
    pub fn apply_env_script_ps1(&self) -> String {
        let mut s = String::new();
        s.push_str("# Qualia execution profile — apply before llm load / decode-proxy\n");
        s.push_str(&format!(
            "$env:QUALIA_INFERENCE_MODE='{}'\n",
            self.inference_mode
        ));
        if self.backend != "auto" && !self.backend.is_empty() {
            s.push_str(&format!("$env:QUALIA_WGPU_BACKEND='{}'\n", self.backend));
        }
        s.push_str(&format!(
            "$env:QUALIA_LLM_KV_INT8='{}'\n",
            if self.toggles.kv_int8 { "1" } else { "0" }
        ));
        s.push_str(&format!(
            "$env:QUALIA_LLM_FFN_F16='{}'\n",
            if self.toggles.ffn_f16 { "1" } else { "0" }
        ));
        s.push_str(&format!(
            "$env:QUALIA_LLM_FFN_FUSION='{}'\n",
            if self.toggles.ffn_fusion { "1" } else { "0" }
        ));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn round_trip_beside_p64() {
        let dir = std::env::temp_dir().join(format!("qualia-exec-profile-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let p64 = dir.join("toy.p64");
        {
            let mut f = std::fs::File::create(&p64).unwrap();
            f.write_all(b"p64\0toy").unwrap();
        }
        let mut prof = ExecutionProfile::from_explore_winner(
            "toy.gguf", &p64, "f16", "portable", "auto", 12.5, 16, "baseline",
        );
        prof.representation.ternary_158_available = true;
        let written = prof.write_beside_p64(&p64).expect("write");
        assert!(written.is_file());
        let loaded = ExecutionProfile::load_beside_p64(&p64)
            .expect("load")
            .expect("some");
        assert_eq!(loaded.layout, "f16");
        assert_eq!(loaded.metrics.decode_proxy_tok_s, Some(12.5));
        assert!(loaded.representation.ternary_158_available);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
