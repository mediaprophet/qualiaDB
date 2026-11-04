use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::llm_bench::{
    set_coop_gemv, set_ffn_fusion, set_kv_int8, set_resident_decode, set_resident_prefill,
    set_resident_weights,
};
pub use engine::{format_lockin_summary, AutoImproveConfig};
use etalon_inference_lab::auto_improve::{self as engine, AutoImproveHost, InstrumentReports};
use std::path::Path;
use std::time::Instant;
pub type LabConfig = engine::LabConfig<InferenceMode>;
pub type TrialResult = engine::TrialResult<InferenceMode>;
pub type LockInPackage = engine::LockInPackage<InferenceMode>;
impl engine::LabMode for InferenceMode {
    const PORTABLE: Self = Self::Portable;
    const FAST_VERIFY: Self = Self::FastVerify;
    const CUDA_TC: Self = Self::CudaTc;
    fn as_str(self) -> &'static str {
        InferenceMode::as_str(self)
    }
}
pub fn default_search_space() -> Vec<LabConfig> {
    engine::default_search_space()
}
impl engine::ConfigApplier for InferenceMode {
    fn apply_config(config: &LabConfig) {
        set_resident_decode(config.resident);
        set_resident_prefill(config.resident);
        set_resident_weights(true);
        set_coop_gemv(config.coop);
        set_ffn_fusion(config.ffn_fusion);
        set_kv_int8(config.kv_int8);
        set_inference_mode(config.mode);
        std::env::set_var("QUALIA_INFERENCE_MODE", config.mode.as_str());
        if let Some(b) = config.backend {
            std::env::set_var("QUALIA_WGPU_BACKEND", b);
        }
    }
}
struct QualiaAutoHost;
impl AutoImproveHost<InferenceMode> for QualiaAutoHost {
    fn instruments(&mut self) -> Result<InstrumentReports, String> {
        let a = super::audit_path::audit_hot_path();
        let r = super::device_roof::calibrate_device_roof(512);
        let m = super::micro::run_q4k_soa_microbench(256, 32);
        Ok(InstrumentReports {
            audit: a.format_report(),
            roof: r.format_report(),
            micro: m.format_report(),
            best_label: r.best_label,
            micro_cuda_ok: m.cuda_ok,
            micro_max_ulp: m.max_ulp,
            notes: a.notes,
            ffn_fusion_in_resident_decode: a.ffn_fusion_in_resident_decode,
            passport_key: crate::hardware_passport::read_passport(
                &crate::hardware_passport::default_cache_path(),
            )
            .map(|p| p.key)
            .unwrap_or_default(),
        })
    }
    fn measure(&mut self, model: &Path, tokens: u32, config: &LabConfig) -> TrialResult {
        let _state = super::state::TrialState::capture();
        config.apply();
        let started = Instant::now();
        let tok_s = crate::hardware_passport::measure_decode_proxy_tok_s(model, tokens);
        TrialResult {
            config: config.clone(),
            tok_s,
            wall_ms: started.elapsed().as_secs_f64() * 1000.0,
            generation: 0,
        }
    }
    fn external_comparison(&mut self, url: &str, model: &str, tokens: u32) -> Option<f64> {
        try_ollama_decode_tok_s(url, model, tokens)
    }
}
pub fn run_auto_improve(cfg: &AutoImproveConfig) -> Result<LockInPackage, String> {
    engine::run_auto_improve(cfg, &mut QualiaAutoHost)
}
/// Probe Ollama decode tok/s if server is up (optional; not a product dependency).
pub fn try_ollama_decode_tok_s(url: &str, model: &str, tokens: u32) -> Option<f64> {
    // Use std only — no reqwest in core-db hot path. Best-effort via blocking HTTP if available.
    // Core-db may not have reqwest; use a tiny TCP+manual approach is heavy.
    // Prefer env-injected pre-measure; try `std::process` curl on Windows.
    let body = format!(
        r#"{{"model":"{model}","prompt":"Write a short paragraph about rivers.","stream":false,"options":{{"num_predict":{tokens},"temperature":0}}}}"#
    );
    let out = std::process::Command::new("curl")
        .args([
            "-sS",
            "-X",
            "POST",
            &format!("{url}/api/generate"),
            "-H",
            "Content-Type: application/json",
            "-d",
            &body,
            "--max-time",
            "120",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    // Crude parse: "eval_count":N ... "eval_duration":N
    let eval_count = parse_json_u64(&text, "eval_count")?;
    let eval_duration = parse_json_u64(&text, "eval_duration")?;
    if eval_duration == 0 {
        return None;
    }
    Some((eval_count as f64) / (eval_duration as f64 / 1e9))
}

fn parse_json_u64(s: &str, key: &str) -> Option<u64> {
    let pat = format!("\"{key}\":");
    let i = s.find(&pat)?;
    let rest = s[i + pat.len()..].trim_start();
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

#[cfg(test)]
#[path = "auto_improve_tests.rs"]
mod tests;
