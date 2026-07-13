//! Autonomous multi-hour lab loop: measure → search configs → re-measure → lock-in.
//!
//! This is **not** an LLM rewriting kernels. It is a disciplined experimental
//! program that:
//! 1. Explores a discrete configuration space (resident, coop, kv, mode, backend)
//! 2. Optionally re-samples the best configs (self-improvement via evidence)
//! 3. Tracks plateau / wall-clock budget
//! 4. Emits a **lock-in package**: best config, full CSV, methodology text, apply script
//!
//! Plan: `docs/plans/inference-superiority-lab-and-toolset-plan.md` L4.

use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::hardware_passport::measure_decode_proxy_tok_s;
use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::lab::audit_path::audit_hot_path;
use crate::lab::device_roof::calibrate_device_roof;
use crate::lab::experiment_log::{append_run_csv, ExperimentRun};
use crate::lab::micro::run_q4k_soa_microbench;
use crate::llm_bench::{
    set_coop_gemv, set_ffn_fusion, set_kv_int8, set_resident_decode, set_resident_prefill,
    set_resident_weights,
};

/// One point in the searchable config space.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LabConfig {
    pub label: String,
    pub resident: bool,
    pub coop: bool,
    pub ffn_fusion: bool,
    pub kv_int8: bool,
    pub mode: InferenceMode,
    pub backend: Option<&'static str>, // None = leave env alone
}

impl LabConfig {
    pub fn toggles_json(&self) -> String {
        format!(
            "{{\"resident\":{},\"coop\":{},\"ffn_fusion\":{},\"kv_int8\":{},\"mode\":\"{}\",\"backend\":\"{}\"}}",
            self.resident,
            self.coop,
            self.ffn_fusion,
            self.kv_int8,
            self.mode.as_str(),
            self.backend.unwrap_or("auto")
        )
    }

    pub fn apply(&self) {
        set_resident_decode(self.resident);
        set_resident_prefill(self.resident);
        set_resident_weights(true);
        set_coop_gemv(self.coop);
        set_ffn_fusion(self.ffn_fusion);
        set_kv_int8(self.kv_int8);
        set_inference_mode(self.mode);
        std::env::set_var("QUALIA_INFERENCE_MODE", self.mode.as_str());
        match self.backend {
            Some(b) => std::env::set_var("QUALIA_WGPU_BACKEND", b),
            None => {
                // Leave passport/path_select free unless already set by operator
            }
        }
    }
}

/// Search space for recursive improvement (expandable).
pub fn default_search_space() -> Vec<LabConfig> {
    let mut v = Vec::new();
    let modes = [
        InferenceMode::Portable,
        InferenceMode::FastVerify,
        InferenceMode::CudaTc,
    ];
    let backends: [Option<&'static str>; 3] = [None, Some("vulkan"), Some("dx12")];
    for &resident in &[true, false] {
        for &coop in &[true, false] {
            for &kv in &[true, false] {
                for &mode in &modes {
                    // Skip nonsense combos early
                    if !resident && mode == InferenceMode::CudaTc {
                        continue;
                    }
                    for &backend in &backends {
                        // Only vary backend for portable/fast-verify resident path
                        if backend.is_some() && (!resident || mode == InferenceMode::CudaTc) {
                            continue;
                        }
                        let label = format!(
                            "r{}_c{}_k{}_{}_{}",
                            resident as u8,
                            coop as u8,
                            kv as u8,
                            mode.as_str(),
                            backend.unwrap_or("auto")
                        );
                        v.push(LabConfig {
                            label,
                            resident,
                            coop,
                            ffn_fusion: true, // flag only; audit says not in resident yet
                            kv_int8: kv,
                            mode,
                            backend,
                        });
                    }
                }
            }
        }
    }
    v
}

#[derive(Debug, Clone)]
pub struct TrialResult {
    pub config: LabConfig,
    pub tok_s: Option<f64>,
    pub wall_ms: f64,
    pub generation: u32,
}

#[derive(Debug, Clone)]
pub struct AutoImproveConfig {
    pub model: PathBuf,
    pub tokens: u32,
    pub max_duration: Duration,
    pub out_dir: PathBuf,
    pub ollama_model: Option<String>,
    pub ollama_url: String,
    /// Re-sample top-k each generation (self-improve via variance reduction).
    pub elite_resample: usize,
    /// Stop if best has not improved by this relative fraction for `plateau_gens` gens.
    pub plateau_rel: f64,
    pub plateau_gens: u32,
    pub max_generations: u32,
}

impl Default for AutoImproveConfig {
    fn default() -> Self {
        Self {
            model: PathBuf::from("C:\\LLM_Models\\P64\\smollm2-360m-instruct-q8_0.p64"),
            tokens: 16,
            max_duration: Duration::from_secs(2 * 3600),
            out_dir: PathBuf::from("experiments/inference-lab/lockin"),
            ollama_model: Some("qualia-smol-q8:latest".into()),
            ollama_url: "http://127.0.0.1:11434".into(),
            elite_resample: 3,
            plateau_rel: 0.02,
            plateau_gens: 2,
            max_generations: 8,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LockInPackage {
    pub best: Option<TrialResult>,
    pub ollama_tok_s: Option<f64>,
    pub a_gap: Option<f64>,
    pub trials: usize,
    pub generations: u32,
    pub elapsed_secs: f64,
    pub out_dir: PathBuf,
    pub methodology: String,
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
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    num.parse().ok()
}

fn measure_config(model: &Path, tokens: u32, cfg: &LabConfig) -> TrialResult {
    cfg.apply();
    let t0 = Instant::now();
    let tok_s = measure_decode_proxy_tok_s(model, tokens);
    let wall_ms = t0.elapsed().as_secs_f64() * 1e3;
    TrialResult {
        config: cfg.clone(),
        tok_s,
        wall_ms,
        generation: 0,
    }
}

/// Run the recursive lab program; write lock-in package under `cfg.out_dir`.
pub fn run_auto_improve(cfg: &AutoImproveConfig) -> Result<LockInPackage, String> {
    if !cfg.model.is_file() {
        return Err(format!("model not found: {}", cfg.model.display()));
    }
    create_dir_all(&cfg.out_dir).map_err(|e| e.to_string())?;
    let csv_path = cfg.out_dir.join("runs.csv");
    let log_path = cfg.out_dir.join("auto_improve.log");
    let mut log = File::create(&log_path).map_err(|e| e.to_string())?;

    macro_rules! logln {
        ($($t:tt)*) => {{
            let line = format!($($t)*);
            let _ = writeln!(log, "{line}");
            let _ = log.flush();
            log::info!("lab_auto|{line}");
            eprintln!("lab_auto|{line}");
        }};
    }

    logln!("start model={} tokens={} budget_s={:.0}",
        cfg.model.display(),
        cfg.tokens,
        cfg.max_duration.as_secs_f64()
    );

    // Baseline instruments
    let audit = audit_hot_path();
    let _ = std::fs::write(cfg.out_dir.join("audit_path.txt"), audit.format_report());
    let roof = calibrate_device_roof(512);
    let _ = std::fs::write(cfg.out_dir.join("device_roof.txt"), roof.format_report());
    let micro = run_q4k_soa_microbench(256, 32);
    let _ = std::fs::write(cfg.out_dir.join("micro_q4k.txt"), micro.format_report());

    let ollama_tok_s = cfg.ollama_model.as_ref().and_then(|m| {
        logln!("ollama_probe model={m}");
        try_ollama_decode_tok_s(&cfg.ollama_url, m, cfg.tokens.max(16))
    });
    if let Some(o) = ollama_tok_s {
        logln!("ollama_decode_tok_s={o:.3}");
    } else {
        logln!("ollama_probe skipped or failed");
    }

    let space = default_search_space();
    logln!("search_space_size={}", space.len());

    let t_start = Instant::now();
    let mut all_trials: Vec<TrialResult> = Vec::new();
    let mut best: Option<TrialResult> = None;
    let mut gens_without_improve = 0u32;
    let mut generation = 0u32;

    let model_id = cfg
        .model
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string();
    let passport_key = crate::hardware_passport::read_passport(
        &crate::hardware_passport::default_cache_path(),
    )
    .map(|p| p.key)
    .unwrap_or_default();

    // Generation 0: full space (or truncated if huge — still fine)
    let mut queue: Vec<LabConfig> = space;
    while generation < cfg.max_generations && t_start.elapsed() < cfg.max_duration {
        generation += 1;
        logln!(
            "generation={generation} queue={} elapsed_s={:.0}",
            queue.len(),
            t_start.elapsed().as_secs_f64()
        );

        let gen_best_before = best.as_ref().and_then(|b| b.tok_s).unwrap_or(0.0);

        for cfg_point in queue.drain(..) {
            if t_start.elapsed() >= cfg.max_duration {
                logln!("budget_exhausted mid-generation");
                break;
            }
            let mut trial = measure_config(&cfg.model, cfg.tokens, &cfg_point);
            trial.generation = generation;
            logln!(
                "trial gen={generation} label={} tok_s={:?} wall_ms={:.0}",
                trial.config.label,
                trial.tok_s,
                trial.wall_ms
            );

            // CSV
            let mut run = ExperimentRun {
                run_id: ExperimentRun::new_id(),
                utc: ExperimentRun::utc_now(),
                git_sha: String::new(),
                host_passport_key: passport_key.clone(),
                adapter: roof.best_label.clone(),
                backend: trial
                    .config
                    .backend
                    .unwrap_or("auto")
                    .to_string(),
                model_id: model_id.clone(),
                model_hash: String::new(),
                layout: if model_id.contains("soa") {
                    "soa".into()
                } else if model_id.contains("f16") {
                    "f16".into()
                } else {
                    "verbatim".into()
                },
                mode: trial.config.mode.as_str().into(),
                profile: "lab-auto".into(),
                toggles_json: trial.config.toggles_json(),
                qualia_decode_tok_s: trial.tok_s,
                ollama_decode_tok_s: ollama_tok_s,
                a_gap: None,
                prefill_tok_s: None,
                phase_ns_json: "{}".into(),
                n_ulp_max: if micro.cuda_ok {
                    Some(micro.max_ulp)
                } else {
                    None
                },
                c_score: None,
                notes: format!("gen={generation}"),
            };
            run.compute_a_gap();
            let _ = append_run_csv(&csv_path, &run);

            let improve = match (&best, trial.tok_s) {
                (None, Some(t)) if t > 0.0 => true,
                (Some(b), Some(t)) => t > b.tok_s.unwrap_or(0.0) * (1.0 + 1e-6),
                _ => false,
            };
            if improve {
                logln!(
                    "NEW_BEST tok_s={:?} label={}",
                    trial.tok_s,
                    trial.config.label
                );
                best = Some(trial.clone());
            }
            all_trials.push(trial);
        }

        // Self-improvement step: re-sample elites + neighbors
        let mut ranked: Vec<_> = all_trials
            .iter()
            .filter(|t| t.tok_s.unwrap_or(0.0) > 0.0)
            .cloned()
            .collect();
        ranked.sort_by(|a, b| {
            b.tok_s
                .unwrap_or(0.0)
                .partial_cmp(&a.tok_s.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let gen_best = best.as_ref().and_then(|b| b.tok_s).unwrap_or(0.0);
        if gen_best > gen_best_before * (1.0 + cfg.plateau_rel) {
            gens_without_improve = 0;
        } else {
            gens_without_improve += 1;
        }
        logln!(
            "generation_end best={:.4} plateau_gens={gens_without_improve}",
            gen_best
        );
        if gens_without_improve >= cfg.plateau_gens && generation >= 2 {
            logln!("plateau_stop");
            break;
        }

        // Next queue: re-measure top elites (variance) + flip one bit neighbors of best
        queue.clear();
        for elite in ranked.iter().take(cfg.elite_resample) {
            queue.push(elite.config.clone());
        }
        if let Some(b) = best.as_ref() {
            for neighbor in neighbors(&b.config) {
                if !queue.iter().any(|c| c == &neighbor) {
                    queue.push(neighbor);
                }
            }
        }
        if queue.is_empty() {
            break;
        }
    }

    // Restore safe defaults
    set_resident_decode(true);
    set_coop_gemv(true);
    set_ffn_fusion(true);
    set_kv_int8(true);
    set_inference_mode(InferenceMode::Portable);

    let a_gap = match (best.as_ref().and_then(|b| b.tok_s), ollama_tok_s) {
        (Some(q), Some(o)) if q > 0.0 => Some(o / q),
        _ => None,
    };

    let methodology = build_methodology(&best, ollama_tok_s, a_gap, &audit, &all_trials, generation);
    let _ = std::fs::write(cfg.out_dir.join("METHODOLOGY.md"), &methodology);

    if let Some(ref b) = best {
        let _ = std::fs::write(
            cfg.out_dir.join("BEST_CONFIG.json"),
            format!(
                "{{\n  \"label\": \"{}\",\n  \"tok_s\": {},\n  \"toggles\": {},\n  \"mode\": \"{}\",\n  \"backend\": \"{}\",\n  \"resident\": {},\n  \"coop\": {},\n  \"kv_int8\": {}\n}}\n",
                b.config.label,
                b.tok_s.unwrap_or(0.0),
                b.config.toggles_json(),
                b.config.mode.as_str(),
                b.config.backend.unwrap_or("auto"),
                b.config.resident,
                b.config.coop,
                b.config.kv_int8
            ),
        );
        let backend_line = b
            .config
            .backend
            .map(|be| format!("$env:QUALIA_WGPU_BACKEND='{be}'\n"))
            .unwrap_or_default();
        let apply = format!(
            "# Auto-generated by lab auto-improve — apply winning config\n\
             $env:QUALIA_INFERENCE_MODE='{}'\n\
             {backend_line}\
             $env:QUALIA_LLM_RESIDENT_DECODE='{}'\n\
             $env:QUALIA_LLM_COOP_GEMV='{}'\n\
             $env:QUALIA_LLM_KV_INT8='{}'\n\
             # tok_s measured ≈ {:?}\n",
            b.config.mode.as_str(),
            if b.config.resident { "1" } else { "0" },
            if b.config.coop { "1" } else { "0" },
            if b.config.kv_int8 { "1" } else { "0" },
            b.tok_s
        );
        let _ = std::fs::write(cfg.out_dir.join("apply-best.ps1"), apply);
    }

    let pkg = LockInPackage {
        best,
        ollama_tok_s,
        a_gap,
        trials: all_trials.len(),
        generations: generation,
        elapsed_secs: t_start.elapsed().as_secs_f64(),
        out_dir: cfg.out_dir.clone(),
        methodology,
    };
    let summary = format_lockin_summary(&pkg);
    let _ = std::fs::write(cfg.out_dir.join("LOCKIN_SUMMARY.txt"), &summary);
    logln!("done\n{summary}");
    Ok(pkg)
}

fn neighbors(c: &LabConfig) -> Vec<LabConfig> {
    let mut out = Vec::new();
    let mut flip = |mut x: LabConfig, label_sfx: &str| {
        x.label = format!("{}_{label_sfx}", c.label);
        out.push(x);
    };
    let mut a = c.clone();
    a.coop = !a.coop;
    flip(a, "flip_coop");
    let mut b = c.clone();
    b.kv_int8 = !b.kv_int8;
    flip(b, "flip_kv");
    let mut d = c.clone();
    d.resident = !d.resident;
    flip(d, "flip_res");
    if c.mode == InferenceMode::Portable {
        let mut e = c.clone();
        e.mode = InferenceMode::FastVerify;
        flip(e, "to_fast_verify");
    }
    out
}

fn build_methodology(
    best: &Option<TrialResult>,
    ollama: Option<f64>,
    a_gap: Option<f64>,
    audit: &crate::lab::audit_path::HotPathAudit,
    trials: &[TrialResult],
    gens: u32,
) -> String {
    let mut s = String::from("# Lab auto-improve methodology lock-in\n\n");
    s.push_str("Generated by `qualia-cli llm lab auto` — **evidence-based**, not LLM kernel rewrite.\n\n");
    s.push_str("## Best config\n\n");
    if let Some(b) = best {
        s.push_str(&format!(
            "- label: `{}`\n- tok_s: {:?}\n- toggles: `{}`\n\n",
            b.config.label,
            b.tok_s,
            b.config.toggles_json()
        ));
    } else {
        s.push_str("- **none** (all trials failed)\n\n");
    }
    s.push_str("## Ollama yardstick\n\n");
    s.push_str(&format!("- ollama_tok_s: {:?}\n- a_gap: {:?}\n\n", ollama, a_gap));
    s.push_str("## Audit notes (unfinished integration)\n\n");
    for n in &audit.notes {
        s.push_str(&format!("- {n}\n"));
    }
    s.push_str(&format!(
        "\n## Search stats\n\n- trials: {}\n- generations: {gens}\n- ffn_fusion_in_resident: {}\n\n",
        trials.len(),
        audit.ffn_fusion_in_resident_decode
    ));
    s.push_str("## Next engineering (not search)\n\n");
    s.push_str("1. **T-A1** (done wiring) — re-optimise fused_ffn body (coop/shared act) if A-gap still open.\n");
    s.push_str("2. **T-A2** CUDA hidden-on-device full layer stack.\n");
    s.push_str("3. Re-run `lab auto` after each; only lock-in when A-gap phase targets improve.\n");
    s.push_str("\n## Apply\n\n```powershell\n. .\\experiments\\inference-lab\\lockin\\apply-best.ps1\n```\n");
    s
}

pub fn format_lockin_summary(pkg: &LockInPackage) -> String {
    format!(
        "LOCK-IN SUMMARY\n  out_dir:     {}\n  trials:      {}\n  generations: {}\n  elapsed_s:   {:.1}\n  best_tok_s:  {:?}\n  best_label:  {}\n  ollama_tok_s:{:?}\n  a_gap:       {:?}\n",
        pkg.out_dir.display(),
        pkg.trials,
        pkg.generations,
        pkg.elapsed_secs,
        pkg.best.as_ref().and_then(|b| b.tok_s),
        pkg.best
            .as_ref()
            .map(|b| b.config.label.as_str())
            .unwrap_or("none"),
        pkg.ollama_tok_s,
        pkg.a_gap
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_space_nonempty_and_dedup_labels() {
        let space = default_search_space();
        assert!(space.len() >= 8, "expected a real search grid, got {}", space.len());
        let mut labels: Vec<&str> = space.iter().map(|c| c.label.as_str()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), space.len(), "duplicate labels in search space");
    }

    #[test]
    fn neighbors_flip_bits() {
        let c = LabConfig {
            label: "base".into(),
            resident: true,
            coop: true,
            ffn_fusion: true,
            kv_int8: true,
            mode: InferenceMode::Portable,
            backend: None,
        };
        let n = neighbors(&c);
        assert!(n.len() >= 3);
        assert!(n.iter().any(|x| x.coop != c.coop));
        assert!(n.iter().any(|x| x.kv_int8 != c.kv_int8));
        assert!(n.iter().any(|x| x.mode == InferenceMode::FastVerify));
    }

    #[test]
    fn toggles_json_is_compact() {
        let c = LabConfig {
            label: "t".into(),
            resident: true,
            coop: false,
            ffn_fusion: true,
            kv_int8: true,
            mode: InferenceMode::FastVerify,
            backend: Some("vulkan"),
        };
        let j = c.toggles_json();
        assert!(j.contains("\"resident\":true"));
        assert!(j.contains("\"coop\":false"));
        assert!(j.contains("fast-verify"));
        assert!(j.contains("vulkan"));
    }

    #[test]
    fn lockin_summary_handles_empty_best() {
        let pkg = LockInPackage {
            best: None,
            ollama_tok_s: None,
            a_gap: None,
            trials: 0,
            generations: 0,
            elapsed_secs: 0.0,
            out_dir: PathBuf::from("experiments/inference-lab/lockin"),
            methodology: String::new(),
        };
        let s = format_lockin_summary(&pkg);
        assert!(s.contains("none"));
        assert!(s.contains("LOCK-IN SUMMARY"));
    }
}
