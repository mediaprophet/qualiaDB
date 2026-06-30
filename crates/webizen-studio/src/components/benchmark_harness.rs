use dioxus::prelude::*;
use serde::Deserialize;

#[cfg(target_arch = "wasm32")]
use serde::de::DeserializeOwned;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct RuntimeSnapshotRecord {
    epoch: u64,
    dimensions: (u32, u32),
    frame_slot: u8,
    state_hash: [u8; 32],
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
struct RuntimeLedgerHealth {
    persisted_epoch: u64,
    dropped_events: u64,
    gap_events: u64,
    recovery_events: u64,
    write_failures: u64,
    last_gap_from_epoch: Option<u64>,
    last_gap_to_epoch: Option<u64>,
    degraded: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct LocalPreviewProbe {
    target_url: String,
    reachable: bool,
    status_code: Option<u16>,
    detail: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ForgeKernelProbe {
    kernel: String,
    shape: String,
    output_elements: usize,
    elapsed_ms: f64,
    max_abs_error: f32,
    certified: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ForgeComputeProbe {
    engine_version: String,
    forge_schema_version: u32,
    backend: String,
    initialization_ms: f64,
    total_kernel_ms: f64,
    all_certified: bool,
    q42_provenance: String,
    kernels: Vec<ForgeKernelProbe>,
    note: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct BenchmarkReport {
    duration_ms: f64,
    avg_invoke_latency_ms: f64,
    p95_invoke_latency_ms: f64,
    epoch_rate: f64,
    target_tick_ratio: f64,
    epoch_delta: u64,
    dimensions_before: (u32, u32),
    dimensions_after: (u32, u32),
    reconfigure_ack_ms: Option<f64>,
    restore_ack_ms: Option<f64>,
    preview_probe: Option<LocalPreviewProbe>,
    health_before: RuntimeLedgerHealth,
    health_after: RuntimeLedgerHealth,
    notes: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_tauri_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: i32) -> Result<(), String> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(window) = web_sys::window() {
            let callback = Closure::once(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                ms,
            );
            callback.forget();
        } else {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });

    JsFuture::from(promise)
        .await
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
}

#[cfg(target_arch = "wasm32")]
fn is_tauri() -> bool {
    js_sys::Reflect::get(
        &js_sys::global(),
        &wasm_bindgen::JsValue::from_str("__TAURI__"),
    )
    .map(|v| !v.is_undefined() && !v.is_null())
    .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
async fn measure_snapshot_call() -> Result<(Option<RuntimeSnapshotRecord>, f64), String> {
    let started = js_sys::Date::now();
    let snapshot = invoke_tauri_json::<Option<RuntimeSnapshotRecord>>(
        "get_latest_diffusion_snapshot",
        json!({}),
    )
    .await?;
    Ok((snapshot, js_sys::Date::now() - started))
}

#[cfg(target_arch = "wasm32")]
async fn measure_reconfigure_ack(
    current_epoch: u64,
    target_dimensions: (u32, u32),
) -> Result<f64, String> {
    let started = js_sys::Date::now();
    while js_sys::Date::now() - started < 2_500.0 {
        let snapshot = invoke_tauri_json::<Option<RuntimeSnapshotRecord>>(
            "get_latest_diffusion_snapshot",
            json!({}),
        )
        .await?;
        if let Some(snapshot) = snapshot {
            if snapshot.epoch > current_epoch && snapshot.dimensions == target_dimensions {
                return Ok(js_sys::Date::now() - started);
            }
        }
        sleep_ms(120).await?;
    }
    Err(format!(
        "timed out waiting for runtime to acknowledge {}x{}",
        target_dimensions.0, target_dimensions.1
    ))
}

#[cfg(target_arch = "wasm32")]
async fn run_benchmark_sweep() -> Result<BenchmarkReport, String> {
    if !is_tauri() {
        return Err("Benchmark requires the Webizen desktop app (Tauri runtime). \
                    Running in a plain browser is not supported — launch the desktop build to use this tool.".to_string());
    }
    let preview_probe =
        invoke_tauri_json::<LocalPreviewProbe>("probe_localhost_preview", json!({}))
            .await
            .ok();
    let health_before =
        invoke_tauri_json::<RuntimeLedgerHealth>("get_diffusion_ledger_health", json!({})).await?;
    let initial_snapshot = invoke_tauri_json::<Option<RuntimeSnapshotRecord>>(
        "get_latest_diffusion_snapshot",
        json!({}),
    )
    .await?
    .ok_or_else(|| "runtime snapshot unavailable".to_string())?;

    let original_dimensions = initial_snapshot.dimensions;
    let expanded_dimensions = (
        (original_dimensions.0 + 64).min(512),
        (original_dimensions.1 + 64).min(512),
    );
    let mut notes = vec![format!(
        "Sweep starts at epoch {} on {}x{}.",
        initial_snapshot.epoch, original_dimensions.0, original_dimensions.1
    )];

    let sweep_started = js_sys::Date::now();
    let mut latencies = Vec::new();
    let mut last_snapshot = initial_snapshot.clone();

    for _ in 0..5 {
        let (snapshot, latency_ms) = measure_snapshot_call().await?;
        latencies.push(latency_ms);
        if let Some(snapshot) = snapshot {
            last_snapshot = snapshot;
        }
        sleep_ms(140).await?;
    }

    invoke_tauri_json::<()>(
        "reconfigure_diffusion",
        json!({
            "config": {
                "width": expanded_dimensions.0,
                "height": expanded_dimensions.1,
                "diffusion_rate": 0.18
            }
        }),
    )
    .await?;
    let reconfigure_ack_ms =
        match measure_reconfigure_ack(last_snapshot.epoch, expanded_dimensions).await {
            Ok(latency) => {
                notes.push(format!(
                    "Resize to {}x{} acknowledged in {:.1} ms.",
                    expanded_dimensions.0, expanded_dimensions.1, latency
                ));
                Some(latency)
            }
            Err(err) => {
                notes.push(err);
                None
            }
        };

    for _ in 0..6 {
        let (snapshot, latency_ms) = measure_snapshot_call().await?;
        latencies.push(latency_ms);
        if let Some(snapshot) = snapshot {
            last_snapshot = snapshot;
        }
        sleep_ms(140).await?;
    }

    invoke_tauri_json::<()>(
        "reconfigure_diffusion",
        json!({
            "config": {
                "width": original_dimensions.0,
                "height": original_dimensions.1,
                "diffusion_rate": 0.18
            }
        }),
    )
    .await?;
    let restore_ack_ms =
        match measure_reconfigure_ack(last_snapshot.epoch, original_dimensions).await {
            Ok(latency) => {
                notes.push(format!(
                    "Restore to {}x{} acknowledged in {:.1} ms.",
                    original_dimensions.0, original_dimensions.1, latency
                ));
                Some(latency)
            }
            Err(err) => {
                notes.push(err);
                None
            }
        };

    for _ in 0..5 {
        let (snapshot, latency_ms) = measure_snapshot_call().await?;
        latencies.push(latency_ms);
        if let Some(snapshot) = snapshot {
            last_snapshot = snapshot;
        }
        sleep_ms(140).await?;
    }

    latencies.sort_by(|a, b| a.total_cmp(b));
    let health_after =
        invoke_tauri_json::<RuntimeLedgerHealth>("get_diffusion_ledger_health", json!({})).await?;
    let duration_ms = (js_sys::Date::now() - sweep_started).max(1.0);
    let epoch_delta = last_snapshot.epoch.saturating_sub(initial_snapshot.epoch);
    let epoch_rate = epoch_delta as f64 / (duration_ms / 1000.0);
    let target_tick_ratio = (epoch_rate / 60.0).clamp(0.0, 2.0);
    let p95_index = ((latencies.len() as f64) * 0.95).floor() as usize;
    let p95_latency = latencies
        .get(p95_index.min(latencies.len().saturating_sub(1)))
        .copied()
        .unwrap_or_default();
    let avg_latency = if latencies.is_empty() {
        0.0
    } else {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    };

    if let Some(probe) = &preview_probe {
        if probe.reachable {
            notes.push(format!(
                "Preview endpoint answered on {} with status {}.",
                probe.target_url,
                probe.status_code.unwrap_or_default()
            ));
        } else {
            notes.push(format!(
                "Preview endpoint unavailable at {}: {}",
                probe.target_url, probe.detail
            ));
        }
    }

    Ok(BenchmarkReport {
        duration_ms,
        avg_invoke_latency_ms: avg_latency,
        p95_invoke_latency_ms: p95_latency,
        epoch_rate,
        target_tick_ratio,
        epoch_delta,
        dimensions_before: original_dimensions,
        dimensions_after: last_snapshot.dimensions,
        reconfigure_ack_ms,
        restore_ack_ms,
        preview_probe,
        health_before,
        health_after,
        notes,
    })
}

#[component]
pub fn BenchmarkHarness() -> Element {
    let mut running = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    let mut status = use_signal(|| {
        if is_tauri() {
            "Ready to benchmark the local runtime path.".to_string()
        } else {
            "Browser mode: benchmark requires the Webizen desktop app (Tauri). Click Run to see the full error.".to_string()
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    let mut status =
        use_signal(|| "Benchmark harness is only active in the webview runtime.".to_string());
    let report = use_signal(|| None::<BenchmarkReport>);
    let mut forge_running = use_signal(|| false);
    let mut forge_status =
        use_signal(|| "Ready to run bounded real-data Forge kernels.".to_string());
    let forge_report = use_signal(|| None::<ForgeComputeProbe>);

    let run_benchmark = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if running() {
                return;
            }

            running.set(true);
            status.set("Running local benchmark sweep...".to_string());
            let mut running = running;
            let mut status = status;
            let mut report = report;

            wasm_bindgen_futures::spawn_local(async move {
                match run_benchmark_sweep().await {
                    Ok(result) => {
                        status.set("Benchmark sweep completed.".to_string());
                        report.set(Some(result));
                    }
                    Err(err) => {
                        status.set(format!("Benchmark failed: {err}"));
                        report.set(None);
                    }
                }
                running.set(false);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = &running;
            status.set("Benchmark harness is only active in the webview runtime.".to_string());
        }
    };

    let run_forge_probe = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if forge_running() {
                return;
            }
            if !is_tauri() {
                forge_status.set(
                    "Forge compute probe requires the Webizen desktop app (Tauri).".to_string(),
                );
                return;
            }

            forge_running.set(true);
            forge_status.set(
                "Initializing ForgeRuntime and dispatching Top-K, GEMM, and FFT...".to_string(),
            );
            let mut forge_running = forge_running;
            let mut forge_status = forge_status;
            let mut forge_report = forge_report;

            wasm_bindgen_futures::spawn_local(async move {
                match invoke_tauri_json::<ForgeComputeProbe>("run_forge_compute_probe", json!({}))
                    .await
                {
                    Ok(result) => {
                        forge_status.set(if result.all_certified {
                            "All real-data Forge kernels matched their CPU oracles.".to_string()
                        } else {
                            "One or more Forge kernels exceeded certification tolerance."
                                .to_string()
                        });
                        forge_report.set(Some(result));
                    }
                    Err(err) => {
                        forge_status.set(format!("Forge compute probe failed: {err}"));
                        forge_report.set(None);
                    }
                }
                forge_running.set(false);
            });
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = &forge_running;
            forge_status.set("Forge compute probe is invoked by the desktop webview.".to_string());
        }
    };

    let run_status = status();
    let latest_report = report();
    let forge_status_text = forge_status();
    let latest_forge_report = forge_report();
    let forge_badge = match latest_forge_report.as_ref() {
        Some(probe) if probe.all_certified => "Certified",
        Some(_) => "Needs review",
        None => "Not run",
    };
    let health_delta = latest_report.as_ref().map(|result| {
        (
            result
                .health_after
                .dropped_events
                .saturating_sub(result.health_before.dropped_events),
            result
                .health_after
                .gap_events
                .saturating_sub(result.health_before.gap_events),
            result
                .health_after
                .recovery_events
                .saturating_sub(result.health_before.recovery_events),
        )
    });

    rsx! {
        div {
            style: "padding: 1.2rem; background: radial-gradient(circle at top left, rgba(34,197,94,0.18), transparent 32%), linear-gradient(180deg, #0f172a, #020617); height: 100%; color: #e2e8f0; overflow: auto;",
            div {
                style: "max-width: 1180px; margin: 0 auto;",
                div {
                    style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; margin-bottom: 1rem;",
                    div {
                        h1 { style: "margin: 0 0 0.35rem 0; font-size: 1.5rem; font-weight: 800; letter-spacing: 0.03em;", "Benchmark Harness" }
                        p { style: "margin: 0; color: rgba(226,232,240,0.74); line-height: 1.5; max-width: 760px;", "This sweep benchmarks the repaired native path: local Tauri command latency, runtime epoch cadence, resize acknowledgement, and whether the optional localhost preview server is really there." }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 0.55rem; min-width: 220px;",
                        button {
                            style: if running() {
                                "padding: 0.78rem 1rem; border-radius: 14px; border: 1px solid rgba(148,163,184,0.25); background: rgba(71,85,105,0.5); color: #cbd5e1; font-weight: 700; cursor: wait;"
                            } else {
                                "padding: 0.78rem 1rem; border-radius: 14px; border: 1px solid rgba(34,197,94,0.28); background: linear-gradient(135deg, rgba(22,163,74,0.95), rgba(5,150,105,0.95)); color: white; font-weight: 700; cursor: pointer; box-shadow: 0 16px 40px rgba(5,150,105,0.22);"
                            },
                            disabled: running(),
                            onclick: run_benchmark,
                            if running() { "Running sweep..." } else { "Run Local Runtime Sweep" }
                        }
                        button {
                            style: if forge_running() {
                                "padding: 0.78rem 1rem; border-radius: 14px; border: 1px solid rgba(148,163,184,0.25); background: rgba(71,85,105,0.5); color: #cbd5e1; font-weight: 700; cursor: wait;"
                            } else {
                                "padding: 0.78rem 1rem; border-radius: 14px; border: 1px solid rgba(56,189,248,0.3); background: linear-gradient(135deg, rgba(3,105,161,0.92), rgba(79,70,229,0.92)); color: white; font-weight: 700; cursor: pointer;"
                            },
                            disabled: forge_running(),
                            onclick: run_forge_probe,
                            if forge_running() { "Running Forge..." } else { "Run Forge Compute Probe" }
                        }
                    }
                }

                div {
                    style: "padding: 0.85rem 1rem; border-radius: 16px; border: 1px solid rgba(148,163,184,0.18); background: rgba(15,23,42,0.86); margin-bottom: 1rem;",
                    span { style: "font-size: 0.78rem; color: #93c5fd; text-transform: uppercase; letter-spacing: 0.08em;", "Status" }
                    div { style: "margin-top: 0.3rem; font-size: 0.95rem; color: #e2e8f0;", "{run_status}" }
                }

                div {
                    style: "padding: 1rem; border-radius: 18px; border: 1px solid rgba(56,189,248,0.18); background: radial-gradient(circle at top right, rgba(59,130,246,0.12), transparent 38%), rgba(15,23,42,0.92); margin-bottom: 1rem;",
                    div {
                        style: "display: flex; justify-content: space-between; gap: 1rem; align-items: flex-start; margin-bottom: 0.75rem;",
                        div {
                            h2 { style: "margin: 0 0 0.25rem 0; font-size: 0.95rem; text-transform: uppercase; letter-spacing: 0.08em; color: #7dd3fc;", "QualiaDB ForgeRuntime" }
                            p { style: "margin: 0; color: rgba(226,232,240,0.72); font-size: 0.8rem; line-height: 1.45;", "A separate real-data correctness probe. Initialization time is reported apart from kernel calls and is not presented as application or LLM throughput." }
                        }
                        span {
                            style: "padding: 0.28rem 0.55rem; border-radius: 999px; border: 1px solid rgba(56,189,248,0.2); background: rgba(2,132,199,0.12); color: #bae6fd; font-size: 0.68rem; white-space: nowrap;",
                            "{forge_badge}"
                        }
                    }
                    div { style: "font-size: 0.78rem; color: rgba(226,232,240,0.8); margin-bottom: 0.75rem;", "{forge_status_text}" }

                    if let Some(probe) = latest_forge_report.as_ref() {
                        div {
                            style: "display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 0.65rem; margin-bottom: 0.75rem;",
                            for kernel in probe.kernels.iter() {
                                div {
                                    style: "padding: 0.78rem; border-radius: 14px; border: 1px solid rgba(148,163,184,0.13); background: rgba(2,6,23,0.58);",
                                    div { style: "display: flex; justify-content: space-between; gap: 0.5rem; margin-bottom: 0.45rem;",
                                        strong { style: "color: #e0f2fe; text-transform: uppercase; font-size: 0.76rem;", "{kernel.kernel}" }
                                        span { style: if kernel.certified { "color: #4ade80; font-size: 0.68rem;" } else { "color: #f87171; font-size: 0.68rem;" }, if kernel.certified { "pass" } else { "fail" } }
                                    }
                                    {detail_row("Shape", kernel.shape.clone())}
                                    {detail_row("Kernel call", format!("{:.2} ms", kernel.elapsed_ms))}
                                    {detail_row("Max |error|", format!("{:.3e}", kernel.max_abs_error))}
                                    {detail_row("Output", format!("{} elements", kernel.output_elements))}
                                }
                            }
                        }
                        {detail_row("Context initialization", format!("{:.2} ms", probe.initialization_ms))}
                        {detail_row("Kernel call total", format!("{:.2} ms", probe.total_kernel_ms))}
                        {detail_row("Runtime", format!("{} · QualiaDB {} · schema {}", probe.backend, probe.engine_version, probe.forge_schema_version))}
                        div {
                            style: "margin-top: 0.65rem; padding: 0.6rem 0.7rem; border-radius: 11px; background: rgba(2,6,23,0.6); border: 1px solid rgba(148,163,184,0.1);",
                            code { style: "display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 0.68rem; color: #93c5fd;", "{probe.q42_provenance}" }
                            p { style: "margin: 0.35rem 0 0 0; font-size: 0.72rem; color: rgba(226,232,240,0.62); line-height: 1.4;", "{probe.note}" }
                        }
                    }
                }

                if let Some(result) = latest_report.as_ref() {
                    div {
                        style: "display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 0.8rem; margin-bottom: 1rem;",
                        {metric_card("Epoch Rate", format!("{:.1} epochs/s", result.epoch_rate), "#22c55e")}
                        {metric_card("Invoke Latency", format!("{:.2} ms avg", result.avg_invoke_latency_ms), "#38bdf8")}
                        {metric_card("P95 Latency", format!("{:.2} ms", result.p95_invoke_latency_ms), "#f59e0b")}
                        {metric_card("Target Tick", format!("{:.0}%", result.target_tick_ratio * 100.0), "#f472b6")}
                    }

                    div {
                        style: "display: grid; grid-template-columns: minmax(0, 1.25fr) minmax(320px, 0.9fr); gap: 1rem;",
                        div {
                            style: "padding: 1rem; border-radius: 18px; border: 1px solid rgba(148,163,184,0.16); background: rgba(15,23,42,0.92);",
                            h2 { style: "margin: 0 0 0.8rem 0; font-size: 0.95rem; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(148,163,184,0.9);", "Sweep Result" }
                            {detail_row("Duration", format!("{:.0} ms", result.duration_ms))}
                            {detail_row("Epoch delta", format!("{}", result.epoch_delta))}
                            {detail_row("Dimensions", format!("{}x{} -> {}x{}", result.dimensions_before.0, result.dimensions_before.1, result.dimensions_after.0, result.dimensions_after.1))}
                            {detail_row("Resize ack", format_option_ms(result.reconfigure_ack_ms))}
                            {detail_row("Restore ack", format_option_ms(result.restore_ack_ms))}
                            if let Some((dropped, gaps, recoveries)) = health_delta {
                                {detail_row("Ledger delta", format!("drops {} | gaps {} | recoveries {}", dropped, gaps, recoveries))}
                            }
                            {detail_row(
                                "Ledger status",
                                if result.health_after.degraded {
                                    "Degraded".to_string()
                                } else {
                                    "Nominal".to_string()
                                },
                            )}
                            if let Some(preview) = result.preview_probe.as_ref() {
                                {detail_row(
                                    "Preview probe",
                                    if preview.reachable {
                                        format!(
                                            "{} responded with {}",
                                            preview.target_url,
                                            preview.status_code.unwrap_or_default()
                                        )
                                    } else {
                                        format!("{} unavailable", preview.target_url)
                                    },
                                )}
                            }
                        }

                        div {
                            style: "padding: 1rem; border-radius: 18px; border: 1px solid rgba(148,163,184,0.16); background: rgba(15,23,42,0.92);",
                            h2 { style: "margin: 0 0 0.8rem 0; font-size: 0.95rem; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(148,163,184,0.9);", "Notes" }
                            div {
                                style: "display: grid; gap: 0.55rem;",
                                for note in result.notes.iter() {
                                    div {
                                        style: "padding: 0.7rem 0.8rem; border-radius: 12px; background: rgba(30,41,59,0.8); border: 1px solid rgba(148,163,184,0.1); color: rgba(226,232,240,0.84); font-size: 0.8rem; line-height: 1.5;",
                                        "{note}"
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div {
                        style: "padding: 1rem; border-radius: 18px; border: 1px dashed rgba(148,163,184,0.2); background: rgba(15,23,42,0.45); color: rgba(226,232,240,0.68);",
                        "No sweep has been run yet. The button above will execute a live runtime benchmark and surface whether the local app path and optional localhost preview path are both healthy."
                    }
                }
            }
        }
    }
}

fn metric_card(title: &'static str, value: String, accent: &'static str) -> Element {
    rsx! {
        div {
            style: "padding: 0.95rem 1rem; border-radius: 16px; border: 1px solid rgba(148,163,184,0.15); background: rgba(15,23,42,0.92);",
            div { style: "font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; color: rgba(148,163,184,0.82); margin-bottom: 0.35rem;", "{title}" }
            div { style: "font-size: 1.08rem; font-weight: 700; color: {accent};", "{value}" }
        }
    }
}

fn detail_row(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            style: "display: flex; justify-content: space-between; gap: 1rem; padding: 0.55rem 0; border-bottom: 1px solid rgba(148,163,184,0.08); font-size: 0.82rem;",
            span { style: "color: rgba(148,163,184,0.88);", "{label}" }
            span { style: "color: #e2e8f0; text-align: right;", "{value}" }
        }
    }
}

fn format_option_ms(value: Option<f64>) -> String {
    value
        .map(|ms| format!("{ms:.1} ms"))
        .unwrap_or_else(|| "No acknowledgement captured".to_string())
}
