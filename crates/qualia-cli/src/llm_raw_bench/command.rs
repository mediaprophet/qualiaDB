use std::path::Path;
use std::time::{Duration, SystemTime};

use qualia_core_db::inference::inference_bench::raw_decode::{
    run_raw_decode_blocking, RawDecodeConfig,
};
use qualia_core_db::inference::runtime::{
    cleanup_stale_runs, ArtifactCleanupCounters, ArtifactRetention, BackendKind, RunArtifactDir,
};

pub struct CommandConfig<'a> {
    pub model: &'a Path,
    pub steps: u32,
    pub warmups: u16,
    pub runs: u16,
    pub quantization: &'a str,
    pub prompt: &'a str,
    pub target_prompt_tokens: Option<u32>,
    pub retain_artifacts: Option<&'a Path>,
}

pub fn run(command: CommandConfig<'_>) -> Result<(), String> {
    let mut config = RawDecodeConfig::new(
        command
            .model
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("model"),
        command.model.to_string_lossy(),
        command.quantization,
        command.prompt,
    );
    config.decode_steps = command.steps;
    config.warmup_runs = command.warmups;
    config.measured_runs = command.runs;
    config.target_prompt_tokens = command.target_prompt_tokens;
    config.requested_backend = BackendKind::Unknown;

    let mut result = run_raw_decode_blocking(&config)?;
    let retained = if let Some(target) = command.retain_artifacts {
        let scratch_parent = target
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if scratch_parent.is_dir() {
            let cutoff = SystemTime::now()
                .checked_sub(Duration::from_secs(24 * 60 * 60))
                .unwrap_or(SystemTime::UNIX_EPOCH);
            match cleanup_stale_runs(scratch_parent, cutoff) {
                Ok(cleanup) => {
                    result.manifest.receipt.artifacts.temp_removed_bytes = result
                        .manifest
                        .receipt
                        .artifacts
                        .temp_removed_bytes
                        .saturating_add(cleanup.removed_bytes);
                    result.manifest.receipt.artifacts.temp_cleanup_failures = result
                        .manifest
                        .receipt
                        .artifacts
                        .temp_cleanup_failures
                        .saturating_add(cleanup.failures);
                }
                Err(_) => {
                    result.manifest.receipt.artifacts.temp_cleanup_failures = result
                        .manifest
                        .receipt
                        .artifacts
                        .temp_cleanup_failures
                        .saturating_add(1);
                }
            }
        }
        let mut artifacts = RunArtifactDir::new_in(
            scratch_parent,
            "raw-decode",
            8 * 1024 * 1024,
            ArtifactRetention::RetainTo(target.to_path_buf()),
        )
        .map_err(|e| e.to_string())?;

        let manifest_json = manifest_json_with_predicted_stats(&mut result)?;
        artifacts
            .write_bounded("manifest.json", manifest_json.as_bytes())
            .map_err(|e| e.to_string())?;
        let tokens_json =
            serde_json::to_vec_pretty(&result.generated_token_ids).map_err(|e| e.to_string())?;
        artifacts
            .write_bounded("generated-token-ids.json", &tokens_json)
            .map_err(|e| e.to_string())?;
        let token_bytes_json =
            serde_json::to_vec_pretty(&result.generated_token_bytes).map_err(|e| e.to_string())?;
        artifacts
            .write_bounded("generated-token-bytes.json", &token_bytes_json)
            .map_err(|e| e.to_string())?;
        artifacts
            .write_bounded("generated-text.txt", result.generated_text.as_bytes())
            .map_err(|e| e.to_string())?;
        let finish = artifacts.finish().map_err(|e| e.to_string())?;
        finish.retained_path
    } else {
        None
    };

    println!(
        "RAW_DECODE median_tok_s={:.4} p95_ms_per_token={:.4} backend={:?} steps={} warmups={} runs={} dispatches={} fences={} d2h_bytes={} fallback_count={}",
        result.manifest.median_tok_s,
        result.manifest.p95_ms_per_token,
        result.manifest.receipt.executed_backend,
        result.manifest.decode_steps_executed,
        result.manifest.warmup_runs,
        result.manifest.measured_runs,
        result.manifest.receipt.counters.compute_dispatches,
        result.manifest.receipt.counters.device_fences,
        result.manifest.receipt.counters.device_to_host_bytes,
        result.manifest.receipt.counters.fallback_count,
    );
    println!(
        "RAW_DECODE_TEXT {}",
        serde_json::to_string(&result.generated_text).map_err(|e| e.to_string())?
    );
    println!(
        "{}",
        serde_json::to_string_pretty(&result.manifest).map_err(|e| e.to_string())?
    );
    if let Some(path) = retained {
        eprintln!("RAW_DECODE_ARTIFACTS {}", path.display());
    }
    Ok(())
}

fn manifest_json_with_predicted_stats(
    result: &mut qualia_core_db::inference::inference_bench::raw_decode::RawDecodeResult,
) -> Result<String, String> {
    let token_bytes = serde_json::to_vec_pretty(&result.generated_token_ids)
        .map_err(|e| e.to_string())?
        .len() as u64;
    let token_piece_bytes = serde_json::to_vec_pretty(&result.generated_token_bytes)
        .map_err(|e| e.to_string())?
        .len() as u64;
    let text_bytes = result.generated_text.len() as u64;
    let mut previous_len = 0u64;
    for _ in 0..4 {
        let json = serde_json::to_string_pretty(&result.manifest).map_err(|e| e.to_string())?;
        let total = json.len() as u64 + token_bytes + token_piece_bytes + text_bytes;
        result.manifest.receipt.artifacts = ArtifactCleanupCounters {
            temp_created_bytes: total,
            temp_removed_bytes: 0,
            temp_retained_bytes: total,
            temp_cleanup_failures: 0,
        };
        if total == previous_len {
            return serde_json::to_string_pretty(&result.manifest).map_err(|e| e.to_string());
        }
        previous_len = total;
    }
    serde_json::to_string_pretty(&result.manifest).map_err(|e| e.to_string())
}
