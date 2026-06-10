//! CLI harness for native GGUF lifecycle: discover → mmap → infer → evict.
//!
//! Uses the in-process stack (`gguf_bridge` / `wgpu` / Phase 8 bifurcated compute),
//! not Ollama or external daemons. Models are memory-mapped via `memmap2`; the UI
//! RAM ceiling is tracked separately from the 42 MB SlgArena Sentinel budget.

use std::path::{Path, PathBuf};
use std::sync::mpsc::sync_channel;
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use qualia_client_core::model_lifecycle::{
    self, lifecycle_label, wait_for_eviction_scrub, ActiveModelRecord, VaultGgufEntry,
};
use qualia_client_core::system_telemetry::SystemTelemetryEvent;
use qualia_core_db::llm_agent::{AgentBackend, AgentIntent, AgentRuntime, LocalLlmAgent};
use qualia_core_db::n3_compiler::N3OutputMode;
use qualia_core_db::orchestrator::OrchestrationResult;
use qualia_core_db::q_hash;

static CLI_SESSION: OnceLock<Mutex<Option<CliSession>>> = OnceLock::new();

struct CliSession {
    record: ActiveModelRecord,
    agent: LocalLlmAgent,
}

fn session_lock() -> &'static Mutex<Option<CliSession>> {
    CLI_SESSION.get_or_init(|| Mutex::new(None))
}

fn store_session(record: ActiveModelRecord, agent: LocalLlmAgent) {
    *session_lock().lock().expect("cli session lock") = Some(CliSession { record, agent });
}

fn with_session<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce(&CliSession) -> Result<T, String>,
{
    let guard = session_lock().lock().map_err(|e| e.to_string())?;
    let session = guard.as_ref().ok_or_else(|| {
        "No model loaded — run `qualia-cli llm load --vault-path <DIR> <MODEL>` first".to_string()
    })?;
    f(session)
}

fn clear_session() {
    if let Ok(mut guard) = session_lock().lock() {
        *guard = None;
    }
}

/// Attach structured logging and optional 100 ms telemetry samples to stdout.
pub fn init_log_stream(enable_telemetry: bool) {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            let target = record.target();
            if target.contains("qualia") || record.args().to_string().contains("LLM_LOAD") {
                writeln!(
                    buf,
                    "[{}] {} — {}",
                    record.level(),
                    target,
                    record.args()
                )
            } else {
                writeln!(buf, "[{}] {}", record.level(), record.args())
            }
        })
        .try_init();

    if !enable_telemetry {
        return;
    }

    let (tx, rx) = sync_channel::<SystemTelemetryEvent>(64);
    qualia_client_core::system_telemetry::subscribe_system_telemetry(tx);
    thread::Builder::new()
        .name("qualia-cli-telemetry".into())
        .spawn(move || {
            for event in rx {
                eprintln!(
                    "[TELEMETRY] RAM {}/{} MB | VRAM {}/{} MB | LLM {} MB | KV {} MB | lifecycle={} | {}",
                    event.ram_used_mb,
                    event.ram_total_mb,
                    event.vram_used_mb,
                    event.vram_total_mb,
                    event.llm_memory_mb,
                    event.kv_cache_mb,
                    event.lifecycle,
                    event.status,
                );
            }
        })
        .ok();
}

pub fn run_list(vault_path: &Path) -> Result<(), String> {
    let entries = model_lifecycle::scan_vault_gguf(vault_path).map_err(|e| e.to_string())?;
    if entries.is_empty() {
        println!("No `.gguf` files under {}", vault_path.display());
        return Ok(());
    }
    println!(
        "{:<32} {:>12}  {:>18}  {}",
        "NAME", "SIZE (MiB)", "PROFILE_ID", "PATH"
    );
    println!("{}", "-".repeat(96));
    for VaultGgufEntry {
        name,
        path,
        profile_id,
        size_bytes,
    } in entries
    {
        let mib = size_bytes as f64 / (1024.0 * 1024.0);
        println!(
            "{:<32} {:>12.1}  0x{profile_id:016x}  {path}",
            name,
            mib,
        );
    }
    Ok(())
}

pub fn run_load(vault_path: &Path, model_ref: &str) -> Result<(), String> {
    let gguf = model_lifecycle::resolve_vault_model(vault_path, model_ref)
        .map_err(|e| e.to_string())?;

    println!("Loading {} …", gguf.display());
    qualia_client_core::system_telemetry::start_activation_telemetry("CLI load");

    let record = model_lifecycle::activate_vault_gguf(&gguf).map_err(|e| {
        qualia_client_core::system_telemetry::stop_activation_telemetry();
        e.to_string()
    })?;

    qualia_client_core::system_telemetry::stop_activation_telemetry();
    qualia_client_core::system_telemetry::publish_idle_telemetry();

    let agent = LocalLlmAgent::with_local_backend(
        format!("did:qualia:cli-vault:{}", record.profile_id),
        AgentBackend::Local {
            model_path: record.gguf_path.clone(),
            context_window: record.context_window,
            quantization: record.quantization.clone(),
            vision_projector_path: record.mmproj_path.clone(),
            modality: record.modality.clone(),
            architecture: record.architecture.clone(),
        },
    );

    store_session(record.clone(), agent);

    let orch = model_lifecycle::task_orchestrator();
    println!("Model ready.");
    println!("  model_id   : {}", record.model_id);
    println!("  profile_id : 0x{:016x}", record.profile_id);
    println!("  path       : {}", record.gguf_path);
    println!("  lifecycle  : {}", record.lifecycle_state);
    println!(
        "  resident   : {} bytes mapped (+ KV cache tracked separately)",
        orch.resident_memory_bytes()
    );
    println!("  backend    : native GGUF → wgpu (DirectML when available)");
    Ok(())
}

pub fn run_status() -> Result<(), String> {
    let orch = model_lifecycle::task_orchestrator();
    let lifecycle = model_lifecycle::get_model_lifecycle_state();
    let resident_id = orch.resident_model_id();
    let llm_mb = model_lifecycle::get_llm_memory_bytes() / (1024 * 1024);
    let kv_mb = model_lifecycle::get_kv_cache_used_mb();

    println!("Lifecycle state : {}", lifecycle_label(lifecycle));
    println!("Resident id     : {:?}", resident_id.map(|id| format!("0x{id:016x}")));
    println!("Resident bytes  : {}", orch.resident_memory_bytes());
    println!("LLM memory      : {} MiB", llm_mb);
    println!("KV cache        : {} MiB", kv_mb);
    println!("Thermal         : {}", model_lifecycle::get_thermal_state_label());
    println!("Scrubbing       : {}", orch.scrubbing_lock.load(std::sync::atomic::Ordering::Acquire));

    if let Ok(guard) = session_lock().lock() {
        if let Some(session) = guard.as_ref() {
            println!("CLI session     : {} ({})", session.record.model_id, session.record.gguf_path);
        } else {
            println!("CLI session     : none");
        }
    }
    Ok(())
}

pub fn run_eval(prompt: &str, orchestrated: bool, stream: bool) -> Result<(), String> {
    with_session(|session| {
        if orchestrated {
            let intent = cli_read_intent();
            let orch = model_lifecycle::task_orchestrator();
            let started = std::time::Instant::now();
            match orch.orchestrate_inference(
                &session.agent,
                prompt,
                "ctx:qualia-cli-eval",
                intent,
                None,
            ) {
                OrchestrationResult::Committed { text, .. } => {
                    println!("\n--- output ({} ms) ---\n{text}\n", started.elapsed().as_millis());
                }
                OrchestrationResult::Blocked {
                    rule_violated,
                    reason,
                } => {
                    return Err(format!(
                        "Webizen blocked inference: {reason} (rule 0x{rule_violated:016x})"
                    ));
                }
                OrchestrationResult::Failed(msg) => return Err(format!("Inference failed: {msg}")),
            }
        } else if stream {
            let (text, _prov, tokens, _quin) = session.agent.infer_local_model_streaming(
                prompt,
                "ctx:qualia-cli-eval",
                Some(|delta: String| {
                    print!("{delta}");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }),
            );
            println!("\n--- tokens generated: {tokens} ---");
            if !text.is_empty() {
                println!("(final length {} chars)", text.len());
            }
        } else {
            let output = session
                .agent
                .infer(prompt, "ctx:qualia-cli-eval")
                .map_err(|e| format!("{e:?}"))?;
            println!("\n--- output ({} ms, {} tokens) ---\n{}\n", output.inference_duration_ms, output.tokens_generated, output.text);
        }
        Ok(())
    })
}

pub fn run_evict(model_id_ref: &str) -> Result<(), String> {
    let profile_id = parse_model_id_ref(model_id_ref)?;
    println!("Evicting model 0x{profile_id:016x} …");
    model_lifecycle::unload_active_model(Some(profile_id));
    if !wait_for_eviction_scrub(Duration::from_secs(10)) {
        eprintln!("Warning: scrub did not finish within 10 s");
    }
    clear_session();
    qualia_client_core::system_telemetry::publish_idle_telemetry();
    println!("Eviction complete. Lifecycle → Discovered.");
    run_status()
}

fn parse_model_id_ref(raw: &str) -> Result<u64, String> {
    let trimmed = raw.trim();
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).map_err(|e| format!("Invalid hex profile id: {e}"))
    } else if let Ok(id) = trimmed.parse::<u64>() {
        Ok(id)
    } else if let Ok(guard) = session_lock().lock() {
        if let Some(session) = guard.as_ref() {
            if session.record.model_id == trimmed {
                return Ok(session.record.profile_id);
            }
        }
        drop(guard);
        Err(format!(
            "Unknown model id `{trimmed}` — use 0x{{16 hex}} profile id or loaded model stem"
        ))
    } else {
        Err("Session lock poisoned".to_string())
    }
}

fn cli_read_intent() -> AgentIntent {
    AgentIntent {
        intent_predicate: q_hash("llm:ReadGraph"),
        requested_graph_scope: vec![q_hash("ctx:qualia-cli-eval")],
        context_namespaces: vec![],
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: q_hash("did:qualia:cli-operator"),
        mcp_intent_frame_hash: q_hash("purpose:CliEval"),
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    }
}

/// Default vault path when none is supplied (Windows-friendly).
pub fn default_vault_path() -> PathBuf {
    if let Ok(dir) = std::env::var("QUALIA_LLM_VAULT") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    PathBuf::from("C:/llmmodels")
}
