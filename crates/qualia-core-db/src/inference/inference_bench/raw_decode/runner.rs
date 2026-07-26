use std::path::Path;
use std::time::Instant;

use crate::inference::runtime::receipt::{
    COUNTER_COMPUTE_DISPATCHES, COUNTER_DECODE_STEPS, COUNTER_DEVICE_FENCES,
    COUNTER_DEVICE_TO_HOST_BYTES, COUNTER_FALLBACKS, COUNTER_GRAPH_LAUNCHES,
    COUNTER_HOST_TO_DEVICE_BYTES,
};
use crate::inference::runtime::{
    capture_source_provenance, sha256_file, sha256_token_ids, BackendKind, BenchmarkManifest,
    ExecutionReceipt, MANIFEST_SCHEMA_VERSION, RAW_GREEDY_DECODE_POLICY,
};

use super::config::{RawDecodeConfig, RawDecodeResult};
use super::model::RawModel;
use super::stats::{median_f64, percentile_nearest_rank};

fn selected_wgpu_backend() -> BackendKind {
    match crate::gpu_context::shared_gpu()
        .adapter_caps
        .backend_label()
        .to_ascii_lowercase()
        .as_str()
    {
        "dx12" => BackendKind::WgpuDx12,
        "vulkan" => BackendKind::WgpuVulkan,
        "metal" => BackendKind::WgpuMetal,
        _ => BackendKind::Unknown,
    }
}

fn validate_config(config: &RawDecodeConfig) -> Result<(), String> {
    if !Path::new(&config.model_path).is_file() {
        return Err(format!("model not found: {}", config.model_path));
    }
    if config.decode_steps == 0 {
        return Err("raw decode requires at least one fixed step".into());
    }
    if config.measured_runs == 0 {
        return Err("raw decode requires at least one measured run".into());
    }
    Ok(())
}

pub fn run_raw_decode_blocking(config: &RawDecodeConfig) -> Result<RawDecodeResult, String> {
    validate_config(config)?;
    let config = config.clone();
    std::thread::Builder::new()
        .name("qualia-raw-decode".into())
        // The existing transformer call chain contains large fixed stack workspaces. Keep the
        // benchmark worker aligned with the production sticky-infer worker until R1 moves every
        // cold workspace into the prepared plan.
        .stack_size(64 * 1024 * 1024)
        .spawn(move || run_on_worker(config))
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "raw decode worker panicked".to_string())?
}

fn run_on_worker(config: RawDecodeConfig) -> Result<RawDecodeResult, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let _runtime_guard = runtime.enter();

    // Cold control-plane boundary: parse any operator environment override once and publish it
    // to the atomic mode read by the zero-allocation token path.
    let _configured_mode = crate::inference_modes::active_inference_mode();
    let mut model = RawModel::load(&config.model_path)?;
    let mut prompt_tokens = model.tokenizer.encode(&config.prompt);
    if let Some(target) = config.target_prompt_tokens {
        if target == 0 || target > crate::gguf_bridge::MAX_CUDA_CONTEXT_WINDOW {
            return Err("target prompt tokens must be in 1..=4096".into());
        }
        if prompt_tokens.is_empty() {
            return Err("cannot expand an empty encoded prompt".into());
        }
        let base_len = prompt_tokens.len();
        prompt_tokens.reserve(target as usize);
        for index in base_len..target as usize {
            prompt_tokens.push(prompt_tokens[index % base_len]);
        }
        prompt_tokens.truncate(target as usize);
    }
    if prompt_tokens.is_empty() {
        return Err("raw decode prompt produced zero tokens".into());
    }

    let cuda_prepared = crate::inference_modes::prefer_tensor_core_gemm();
    let executed_backend = if cuda_prepared {
        BackendKind::Cuda
    } else {
        selected_wgpu_backend()
    };
    let requested_backend = if config.requested_backend == BackendKind::Unknown {
        executed_backend
    } else {
        config.requested_backend
    };
    if requested_backend != executed_backend {
        return Err(format!(
            "raw resident runner requested {requested_backend:?}, selected {executed_backend:?}; \
             refusing to relabel or silently fall back"
        ));
    }

    let total_runs = config.warmup_runs as usize + config.measured_runs as usize;
    let mut run_tok_s = Vec::with_capacity(config.measured_runs as usize);
    let mut step_latency_ms =
        Vec::with_capacity(config.measured_runs as usize * config.decode_steps as usize);
    let mut generated_token_ids = Vec::with_capacity(config.decode_steps as usize);

    for run_index in 0..total_runs {
        let (mut token_id, mut position) = model.prepare_prompt(&prompt_tokens, cuda_prepared)?;
        let mut run_tokens = vec![0u32; config.decode_steps as usize];
        let mut run_step_ms = vec![0.0f64; config.decode_steps as usize];
        let run_start = Instant::now();
        for step in 0..config.decode_steps as usize {
            let step_start = Instant::now();
            let output_token = if cuda_prepared {
                let emb_dim = model.index.emb_dim();
                let token = model
                    .engine
                    .try_cuda_mega_pass_decode_token(
                        &model.index,
                        token_id,
                        &mut model.emb[..emb_dim],
                        emb_dim,
                        position,
                    )
                    .ok_or_else(|| {
                        "prepared CUDA raw decode became ineligible; no fallback is allowed"
                            .to_string()
                    })?;
                if token == u32::MAX {
                    return Err("prepared CUDA raw decode did not own the output projection".into());
                }
                token
            } else {
                model.load_embedding(token_id)?;
                model
                    .engine
                    .dispatch_token_forward_resident(
                        &model.index,
                        &model.emb[..model.index.emb_dim()],
                        position,
                    )
                    .ok_or_else(|| {
                        "resident raw decode became ineligible; no fallback is allowed".to_string()
                    })?
                    .best_token_id
            };
            run_step_ms[step] = step_start.elapsed().as_secs_f64() * 1000.0;
            token_id = output_token;
            run_tokens[step] = token_id;
            position = position.saturating_add(1);
        }
        let elapsed = run_start.elapsed().as_secs_f64();
        if run_index >= config.warmup_runs as usize {
            run_tok_s.push(config.decode_steps as f64 / elapsed);
            step_latency_ms.extend_from_slice(&run_step_ms);
            generated_token_ids.clear();
            generated_token_ids.extend_from_slice(&run_tokens);
        }
    }

    let (
        dispatches_per_token,
        h2d_bytes_per_token,
        readback_bytes_per_token,
        graph_key,
        tuning_profile,
        context_window,
    ) = if cuda_prepared {
        let telemetry = model
            .engine
            .cuda_prepared_telemetry()
            .ok_or_else(|| "prepared CUDA plan did not expose telemetry".to_string())?;
        (
            telemetry.device_dispatches_per_token,
            telemetry.host_to_device_bytes_per_token,
            telemetry.readback_bytes_per_token,
            telemetry.graph_key,
            telemetry.tuning_profile,
            telemetry.context_window,
        )
    } else {
        (
            model
                .engine
                .resident_dispatches_per_token()
                .ok_or_else(|| "resident plan did not expose a dispatch count".to_string())?
                as u64,
            0,
            model
                .engine
                .resident_readback_bytes_per_token()
                .ok_or_else(|| "resident plan did not expose readback bytes".to_string())?
                as u64,
            None,
            "",
            crate::gguf_bridge::MAX_CONTEXT_WINDOW,
        )
    };
    let executed_steps = config.decode_steps as u64 * config.measured_runs as u64;

    let model_sha256 =
        sha256_file(Path::new(&config.model_path)).map_err(|e| format!("model SHA-256: {e}"))?;
    let source = capture_source_provenance();
    let mut receipt = ExecutionReceipt::new(
        requested_backend,
        executed_backend,
        model_sha256.clone(),
        format!(
            "{}:{model_sha256}:{executed_backend:?}",
            if cuda_prepared {
                if graph_key.is_some() {
                    "cuda-graph-v1"
                } else {
                    "cuda-prepared-v1"
                }
            } else {
                "resident-v1"
            }
        ),
    );
    if let Some(key) = graph_key {
        receipt.graph_hash = format!("cuda-graph:{key:016x}");
    }
    receipt.tuning_profile = tuning_profile.to_string();
    receipt.stop_reason = "fixed-step-budget".into();
    receipt.counter_coverage = COUNTER_DECODE_STEPS
        | COUNTER_GRAPH_LAUNCHES
        | COUNTER_COMPUTE_DISPATCHES
        | COUNTER_DEVICE_FENCES
        | COUNTER_HOST_TO_DEVICE_BYTES
        | COUNTER_DEVICE_TO_HOST_BYTES
        | COUNTER_FALLBACKS;
    receipt.counters.decode_steps = executed_steps;
    receipt.counters.graph_launches = if graph_key.is_some() {
        executed_steps
    } else {
        0
    };
    receipt.counters.compute_dispatches = executed_steps.saturating_mul(dispatches_per_token);
    receipt.counters.device_fences = executed_steps;
    receipt.counters.host_to_device_bytes = executed_steps.saturating_mul(h2d_bytes_per_token);
    receipt.counters.device_to_host_bytes = executed_steps.saturating_mul(readback_bytes_per_token);
    receipt.counters.fallback_count = 0;

    let manifest = BenchmarkManifest {
        schema_version: MANIFEST_SCHEMA_VERSION,
        benchmark_kind: "raw-decode-resident".into(),
        executable_commit: source.commit,
        dirty_diff_hash: source.dirty_source_hash,
        executable_sha256: source.executable_sha256,
        model_path: config.model_path,
        model_sha256,
        prompt_token_sha256: sha256_token_ids(&prompt_tokens),
        prompt_tokens: prompt_tokens.len() as u32,
        context_window,
        decode_policy: RAW_GREEDY_DECODE_POLICY.into(),
        quantization: config.quantization,
        decode_steps_requested: config.decode_steps,
        decode_steps_executed: config.decode_steps,
        warmup_runs: config.warmup_runs,
        measured_runs: config.measured_runs,
        median_tok_s: median_f64(&run_tok_s),
        p95_ms_per_token: percentile_nearest_rank(&step_latency_ms, 0.95),
        receipt,
    };
    manifest.validate().map_err(str::to_string)?;

    let generated_token_bytes = generated_token_ids
        .iter()
        .map(|token_id| model.tokenizer.decode_token_bytes_cold(*token_id))
        .collect();
    let generated_text = model.tokenizer.decode(&generated_token_ids);
    Ok(RawDecodeResult {
        manifest,
        run_tok_s,
        step_latency_ms,
        generated_token_ids,
        generated_token_bytes,
        generated_text,
    })
}
