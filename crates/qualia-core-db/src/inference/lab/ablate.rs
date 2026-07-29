//! Ablation matrix: toggle axes × decode-proxy tok/s → experiment CSV.

use std::path::Path;

use crate::hardware_passport::measure_decode_proxy_tok_s;
use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::lab::experiment_log::{append_run_csv, ExperimentRun};
use crate::llm_bench::{set_coop_gemv, set_ffn_fusion, set_kv_int8, set_resident_decode};

#[derive(Debug, Clone)]
pub struct AblationRow {
    pub label: String,
    pub tok_s: Option<f64>,
    pub toggles: String,
}

/// Default small ablation matrix (fast enough for lab sessions).
pub fn run_ablation_matrix(model: &Path, tokens: u32, csv_out: Option<&Path>) -> Vec<AblationRow> {
    let tokens = tokens.max(4).min(32);
    let model_id = model
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string();

    // (label, resident, coop, ffn_flag, kv_int8, mode)
    let cases: [(&str, bool, bool, bool, bool, InferenceMode); 6] = [
        (
            "baseline_resident",
            true,
            true,
            true,
            true,
            InferenceMode::Portable,
        ),
        ("no_coop", true, false, true, true, InferenceMode::Portable),
        (
            "no_resident",
            false,
            true,
            true,
            true,
            InferenceMode::Portable,
        ),
        ("kv_f32", true, true, true, false, InferenceMode::Portable),
        (
            "fast_verify",
            true,
            true,
            true,
            true,
            InferenceMode::FastVerify,
        ),
        ("cuda_mode", true, true, true, true, InferenceMode::CudaTc),
    ];

    let mut rows = Vec::new();
    let passport_key =
        crate::hardware_passport::read_passport(&crate::hardware_passport::default_cache_path())
            .map(|p| p.key)
            .unwrap_or_default();

    for (label, res, coop, ffn, kv, mode) in cases {
        set_resident_decode(res);
        set_coop_gemv(coop);
        set_ffn_fusion(ffn);
        set_kv_int8(kv);
        set_inference_mode(mode);
        std::env::set_var("QUALIA_INFERENCE_MODE", mode.as_str());

        let toggles = format!(
            "{{\"resident\":{res},\"coop\":{coop},\"ffn_fusion\":{ffn},\"kv_int8\":{kv},\"mode\":\"{}\"}}",
            mode.as_str()
        );
        let tok_s = measure_decode_proxy_tok_s(model, tokens);
        rows.push(AblationRow {
            label: label.into(),
            tok_s,
            toggles: toggles.clone(),
        });

        if let Some(path) = csv_out {
            let mut run = ExperimentRun {
                run_id: ExperimentRun::new_id(),
                utc: ExperimentRun::utc_now(),
                git_sha: String::new(),
                host_passport_key: passport_key.clone(),
                adapter: String::new(),
                backend: std::env::var("QUALIA_WGPU_BACKEND").unwrap_or_else(|_| "auto".into()),
                model_id: model_id.clone(),
                model_hash: String::new(),
                layout: guess_layout(&model_id),
                mode: mode.as_str().into(),
                profile: "lab-ablate".into(),
                toggles_json: toggles,
                qualia_decode_tok_s: tok_s,
                ollama_decode_tok_s: None,
                a_gap: None,
                prefill_tok_s: None,
                phase_ns_json: "{}".into(),
                n_ulp_max: None,
                c_score: None,
                notes: label.into(),
            };
            run.compute_a_gap();
            let _ = append_run_csv(path, &run);
        }
    }

    // Restore sane defaults
    set_resident_decode(true);
    set_coop_gemv(true);
    set_ffn_fusion(true);
    set_kv_int8(true);
    set_inference_mode(InferenceMode::Portable);
    std::env::remove_var("QUALIA_INFERENCE_MODE");

    // ── Forge backend profile rows ──────────────────────────────────────────
    // Test each forge shader backend with default toggles (resident+coop+ffn+kv_int8).
    // These run in-process like the toggle rows above; the forge backend is
    // selected via QUALIA_FORGE_BACKEND env var which is read at pipeline
    // compile time.
    let forge_backends: &[(&str, &str)] = &[
        ("forge_wgsl", "wgsl"),
        ("forge_hlsl", "hlsl"),
        ("forge_spirv", "spirv"),
        ("forge_ptx", "ptx"),
    ];

    for &(label, backend) in forge_backends {
        set_resident_decode(true);
        set_coop_gemv(true);
        set_ffn_fusion(true);
        set_kv_int8(true);
        set_inference_mode(InferenceMode::Portable);
        std::env::set_var("QUALIA_INFERENCE_MODE", "portable");
        std::env::set_var("QUALIA_FORGE_BACKEND", backend);

        let toggles = format!(
            "{{\"forge_backend\":\"{backend}\",\"resident\":true,\"coop\":true,\"ffn_fusion\":true,\"kv_int8\":true,\"mode\":\"portable\"}}"
        );
        let tok_s = measure_decode_proxy_tok_s(model, tokens);
        rows.push(AblationRow {
            label: label.into(),
            tok_s,
            toggles: toggles.clone(),
        });

        if let Some(path) = csv_out {
            let mut run = ExperimentRun {
                run_id: ExperimentRun::new_id(),
                utc: ExperimentRun::utc_now(),
                git_sha: String::new(),
                host_passport_key: passport_key.clone(),
                adapter: String::new(),
                backend: format!("forge-{backend}"),
                model_id: model_id.clone(),
                model_hash: String::new(),
                layout: guess_layout(&model_id),
                mode: "portable".into(),
                profile: "lab-ablate".into(),
                toggles_json: toggles,
                qualia_decode_tok_s: tok_s,
                ollama_decode_tok_s: None,
                a_gap: None,
                prefill_tok_s: None,
                phase_ns_json: "{}".into(),
                n_ulp_max: None,
                c_score: None,
                notes: label.into(),
            };
            run.compute_a_gap();
            let _ = append_run_csv(path, &run);
        }
    }

    // Clean up forge backend env
    std::env::remove_var("QUALIA_FORGE_BACKEND");

    rows
}

fn guess_layout(name: &str) -> String {
    let n = name.to_ascii_lowercase();
    if n.contains(".soa.") || n.contains("soa") {
        "soa".into()
    } else if n.contains("f16") {
        "f16".into()
    } else {
        "verbatim".into()
    }
}

pub fn format_ablation_report(rows: &[AblationRow]) -> String {
    let mut s = String::from("Ablation matrix (decode-proxy tok/s)\n");
    for r in rows {
        s.push_str(&format!(
            "  {:20}  {:>8}\n",
            r.label,
            r.tok_s
                .map(|t| format!("{t:.3}"))
                .unwrap_or_else(|| "fail".into())
        ));
    }
    s
}
