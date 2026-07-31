//! Global background-work indicator and full job-centre page.

use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct JobQueueSnapshot {
    #[serde(default)]
    pub jobs: Vec<JobSnapshot>,
    #[serde(default)]
    pub queued: usize,
    #[serde(default)]
    pub running: usize,
    #[serde(default)]
    pub completed: usize,
    #[serde(default)]
    pub failed: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct JobSnapshot {
    pub id: String,
    #[serde(default)]
    pub kind: serde_json::Value,
    pub status: String,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub progress: f64,
    pub message: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

fn kind_name(kind: &serde_json::Value) -> &'static str {
    match kind.get("kind").and_then(serde_json::Value::as_str) {
        Some("model_download") => "Model download",
        Some("model_activation") => "Model activation",
        Some("anatomy_asset_acquire") => "Anatomy assets",
        Some("ontology_catalog_import") => "Ontology import",
        Some("ontology_uri_import") => "Ontology import",
        Some("bundled_ontology_seed") => "Knowledge preparation",
        Some("workbench_daemon_sync") => "Knowledge sync",
        Some("daemon_graph_reload") => "Graph reload",
        Some("agent_turn") => "Agent task",
        _ => "Background task",
    }
}

fn kind_detail(kind: &serde_json::Value) -> String {
    for key in [
        "filename",
        "model_name",
        "model",
        "ontology_id",
        "uri",
        "session_id",
    ] {
        if let Some(value) = kind.get(key).and_then(serde_json::Value::as_str) {
            if !value.trim().is_empty() {
                return value.to_string();
            }
        }
    }
    String::new()
}

fn state_color(status: &str) -> &'static str {
    match status {
        "running" => "#38bdf8",
        "queued" => "#fbbf24",
        "completed" => "#34d399",
        "failed" => "#fb7185",
        "cancelled" => "#94a3b8",
        _ => "#94a3b8",
    }
}

fn active(status: &str) -> bool {
    matches!(status, "queued" | "running")
}

async fn fetch_jobs() -> Result<JobQueueSnapshot, String> {
    let value =
        crate::components::qapp_engine::invoke_json("list_local_jobs", serde_json::json!({}))
            .await?;
    serde_json::from_value(value).map_err(|error| format!("decode job queue: {error}"))
}

fn refresh(mut snapshot: Signal<JobQueueSnapshot>, mut note: Signal<String>) {
    spawn(async move {
        match fetch_jobs().await {
            Ok(next) => {
                note.set(String::new());
                snapshot.set(next);
            }
            Err(error) => note.set(error),
        }
    });
}

fn cancel_job(id: String, snapshot: Signal<JobQueueSnapshot>, note: Signal<String>) {
    spawn(async move {
        match crate::components::qapp_engine::invoke_json(
            "cancel_local_job",
            serde_json::json!({ "id": id }),
        )
        .await
        {
            Ok(_) => refresh(snapshot, note),
            Err(error) => {
                let mut note = note;
                note.set(format!("Cancel failed: {error}"));
            }
        }
    });
}

fn retry_job(id: String, snapshot: Signal<JobQueueSnapshot>, note: Signal<String>) {
    spawn(async move {
        match crate::components::qapp_engine::invoke_json(
            "retry_local_job",
            serde_json::json!({ "id": id }),
        )
        .await
        {
            Ok(_) => refresh(snapshot, note),
            Err(error) => {
                let mut note = note;
                note.set(format!("Retry failed: {error}"));
            }
        }
    });
}

fn use_job_snapshot() -> (Signal<JobQueueSnapshot>, Signal<String>) {
    let snapshot = use_signal(JobQueueSnapshot::default);
    let note = use_signal(String::new);
    use_effect(move || {
        if !crate::endpoints::is_native_host() {
            return;
        }
        refresh(snapshot, note);
        #[cfg(target_arch = "wasm32")]
        spawn(async move {
            loop {
                gloo_timers::future::sleep(std::time::Duration::from_millis(1200)).await;
                if let Ok(next) = fetch_jobs().await {
                    let mut snapshot = snapshot;
                    snapshot.set(next);
                }
            }
        });
    });
    (snapshot, note)
}

#[component]
pub fn JobIndicator() -> Element {
    let (snapshot, note) = use_job_snapshot();
    let mut expanded = use_signal(|| false);
    let current = snapshot();
    let active_count = current.running + current.queued;
    let badge = if active_count > 0 {
        active_count
    } else {
        current.failed
    };
    let badge_color = if active_count > 0 {
        "#38bdf8"
    } else {
        "#fb7185"
    };
    let recent: Vec<JobSnapshot> = current.jobs.iter().rev().take(5).cloned().collect();

    rsx! {
        div {
            style: "position:relative;",
            button {
                r#type: "button",
                title: "Background work and failure events",
                aria_label: "Open background jobs",
                onclick: move |_| expanded.toggle(),
                style: "position:relative;display:inline-flex;align-items:center;gap:.42rem;border:1px solid var(--qualia-border);background:rgba(15,23,42,.72);color:var(--qualia-text);border-radius:999px;padding:.33rem .62rem;cursor:pointer;font-size:.72rem;font-weight:750;",
                sl-icon { "name": if active_count > 0 { "arrow-repeat" } else { "activity" } }
                if active_count > 0 {
                    span { "Working" }
                } else {
                    span { "Jobs" }
                }
                if badge > 0 {
                    span {
                        style: "min-width:18px;height:18px;padding:0 5px;border-radius:999px;display:inline-flex;align-items:center;justify-content:center;background:{badge_color};color:#06111d;font-size:.63rem;font-weight:900;",
                        "{badge}"
                    }
                }
            }
            if expanded() {
                div {
                    role: "dialog",
                    aria_label: "Background jobs",
                    style: "position:absolute;right:0;top:calc(100% + 10px);z-index:1000;width:min(420px,86vw);max-height:560px;overflow:auto;padding:12px;border:1px solid var(--qualia-border);border-radius:14px;background:color-mix(in srgb,var(--qualia-surface) 96%,#020617);box-shadow:0 22px 60px rgba(0,0,0,.46);",
                    div {
                        style: "display:flex;align-items:flex-start;justify-content:space-between;gap:12px;margin-bottom:10px;",
                        div {
                            strong { style: "display:block;font-size:.9rem;", "Background work" }
                            span { style: "font-size:.68rem;color:var(--qualia-text-muted);", "You can keep using Webizen while these run." }
                        }
                        Link {
                            to: crate::Route::JobsRoute {},
                            onclick: move |_| expanded.set(false),
                            style: "font-size:.7rem;color:var(--qualia-accent);font-weight:800;text-decoration:none;",
                            "Open job centre →"
                        }
                    }
                    if !note().is_empty() {
                        div { style: "padding:8px;color:#fb7185;font-size:.7rem;", "{note}" }
                    }
                    if recent.is_empty() {
                        div { style: "padding:20px;text-align:center;color:var(--qualia-text-muted);font-size:.75rem;", "No background work yet." }
                    } else {
                        div { style: "display:grid;gap:8px;",
                            for job in recent {
                                JobCompactRow { job, snapshot, note }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn JobCompactRow(
    job: JobSnapshot,
    snapshot: Signal<JobQueueSnapshot>,
    note: Signal<String>,
) -> Element {
    let color = state_color(&job.status);
    let progress = (job.progress * 100.0).clamp(0.0, 100.0);
    let id_for_cancel = job.id.clone();
    rsx! {
        div {
            style: "padding:10px;border:1px solid color-mix(in srgb,{color} 26%,var(--qualia-border));border-radius:10px;background:rgba(2,6,23,.32);",
            div { style: "display:flex;align-items:center;gap:8px;",
                span { style: "width:8px;height:8px;border-radius:50%;background:{color};box-shadow:0 0 8px {color};" }
                strong { style: "font-size:.74rem;", "{kind_name(&job.kind)}" }
                span { style: "margin-left:auto;font-size:.62rem;text-transform:uppercase;color:{color};font-weight:850;", "{job.status}" }
            }
            if !kind_detail(&job.kind).is_empty() {
                div { style: "margin-top:4px;font-size:.65rem;color:var(--qualia-text-muted);overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", "{kind_detail(&job.kind)}" }
            }
            div { style: "margin-top:6px;font-size:.68rem;line-height:1.35;", "{job.message}" }
            if active(&job.status) {
                div { style: "height:5px;margin-top:8px;border-radius:999px;background:rgba(148,163,184,.15);overflow:hidden;",
                    div { style: "height:100%;width:{progress}%;background:{color};transition:width .25s ease;" }
                }
                button {
                    r#type: "button",
                    onclick: move |_| cancel_job(id_for_cancel.clone(), snapshot, note),
                    style: "margin-top:7px;border:0;background:transparent;color:#fda4af;font-size:.65rem;font-weight:750;cursor:pointer;padding:0;",
                    "Cancel"
                }
            }
            if let Some(error) = &job.error {
                div { style: "margin-top:6px;color:#fda4af;font-size:.66rem;line-height:1.35;overflow-wrap:anywhere;", "{error}" }
            }
        }
    }
}

#[component]
pub fn JobCenterPage() -> Element {
    let (snapshot, note) = use_job_snapshot();
    let current = snapshot();
    let mut filter = use_signal(|| "active".to_string());
    let visible: Vec<JobSnapshot> = current
        .jobs
        .iter()
        .rev()
        .filter(|job| match filter().as_str() {
            "active" => active(&job.status),
            "failed" => matches!(job.status.as_str(), "failed" | "cancelled"),
            "finished" => !active(&job.status),
            _ => true,
        })
        .cloned()
        .collect();

    rsx! {
        div {
            style: "width:100%;height:100%;overflow:auto;padding:1.4rem;color:var(--qualia-text);",
            div {
                style: "max-width:1050px;margin:0 auto;display:grid;gap:1rem;",
                header {
                    style: "display:flex;align-items:flex-end;justify-content:space-between;gap:1rem;flex-wrap:wrap;",
                    div {
                        div { style: "font-size:.66rem;text-transform:uppercase;letter-spacing:.09em;color:#7dd3fc;font-weight:850;", "System · background work" }
                        h1 { style: "margin:.2rem 0;font-size:1.55rem;", "Job centre" }
                        p { style: "margin:0;color:var(--qualia-text-muted);font-size:.82rem;line-height:1.45;", "Downloads, model preparation, anatomy conversion, ontology work and agent tasks continue here without freezing their originating screen." }
                    }
                    div { style: "display:flex;gap:8px;",
                        Link { to: crate::Route::LogsRoute {}, style: "padding:.52rem .75rem;border:1px solid var(--qualia-border);border-radius:9px;color:var(--qualia-text);text-decoration:none;font-size:.72rem;font-weight:750;", "Failure logs" }
                        button {
                            r#type: "button",
                            onclick: move |_| {
                                spawn(async move {
                                    match crate::components::qapp_engine::invoke_json(
                                        "clear_finished_local_jobs",
                                        serde_json::json!({}),
                                    ).await {
                                        Ok(_) => refresh(snapshot, note),
                                        Err(error) => {
                                            let mut note = note;
                                            note.set(format!("Clear failed: {error}"));
                                        }
                                    }
                                });
                            },
                            style: "padding:.52rem .75rem;border:1px solid var(--qualia-border);border-radius:9px;background:transparent;color:var(--qualia-text);font-size:.72rem;font-weight:750;cursor:pointer;",
                            "Clear finished"
                        }
                    }
                }
                div { style: "display:grid;grid-template-columns:repeat(4,minmax(110px,1fr));gap:9px;",
                    QueueMetric { label: "Running", value: current.running, color: "#38bdf8" }
                    QueueMetric { label: "Queued", value: current.queued, color: "#fbbf24" }
                    QueueMetric { label: "Completed", value: current.completed, color: "#34d399" }
                    QueueMetric { label: "Failed", value: current.failed, color: "#fb7185" }
                }
                div { style: "display:flex;gap:6px;flex-wrap:wrap;",
                    for key in ["active", "all", "failed", "finished"] {
                        button {
                            r#type: "button",
                            onclick: move |_| filter.set(key.to_string()),
                            style: if filter() == key {
                                "border:1px solid var(--qualia-accent);background:var(--qualia-accent-glow);color:var(--qualia-text);border-radius:999px;padding:.38rem .7rem;font-size:.68rem;font-weight:800;cursor:pointer;"
                            } else {
                                "border:1px solid var(--qualia-border);background:transparent;color:var(--qualia-text-muted);border-radius:999px;padding:.38rem .7rem;font-size:.68rem;font-weight:750;cursor:pointer;"
                            },
                            "{key}"
                        }
                    }
                }
                if !note().is_empty() {
                    div { style: "padding:10px;border:1px solid rgba(251,113,133,.35);border-radius:9px;color:#fda4af;font-size:.72rem;", "{note}" }
                }
                if visible.is_empty() {
                    div { style: "padding:3rem;border:1px dashed var(--qualia-border);border-radius:14px;text-align:center;color:var(--qualia-text-muted);font-size:.82rem;", "No jobs in this view." }
                } else {
                    div { style: "display:grid;gap:10px;",
                        for job in visible {
                            JobDetailedRow { job, snapshot, note }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn QueueMetric(label: &'static str, value: usize, color: &'static str) -> Element {
    rsx! {
        div { style: "padding:12px;border:1px solid var(--qualia-border);border-radius:11px;background:rgba(2,6,23,.25);",
            div { style: "font-size:1.25rem;font-weight:850;color:{color};", "{value}" }
            div { style: "font-size:.65rem;color:var(--qualia-text-muted);text-transform:uppercase;letter-spacing:.07em;", "{label}" }
        }
    }
}

#[component]
fn JobDetailedRow(
    job: JobSnapshot,
    snapshot: Signal<JobQueueSnapshot>,
    note: Signal<String>,
) -> Element {
    let color = state_color(&job.status);
    let progress = (job.progress * 100.0).clamp(0.0, 100.0);
    let id_for_cancel = job.id.clone();
    let id_for_retry = job.id.clone();
    rsx! {
        article {
            style: "padding:14px;border:1px solid color-mix(in srgb,{color} 23%,var(--qualia-border));border-radius:12px;background:rgba(2,6,23,.3);",
            div { style: "display:flex;align-items:flex-start;gap:10px;",
                span { style: "width:9px;height:9px;margin-top:4px;border-radius:50%;background:{color};box-shadow:0 0 9px {color};flex:none;" }
                div { style: "min-width:0;flex:1;",
                    div { style: "display:flex;align-items:center;gap:8px;flex-wrap:wrap;",
                        strong { style: "font-size:.84rem;", "{kind_name(&job.kind)}" }
                        span { style: "font-size:.62rem;color:{color};font-weight:850;text-transform:uppercase;", "{job.status}" }
                        span { style: "margin-left:auto;font-family:ui-monospace,monospace;font-size:.6rem;color:var(--qualia-text-muted);", "{job.id}" }
                    }
                    if !kind_detail(&job.kind).is_empty() {
                        div { style: "margin-top:4px;font-size:.7rem;color:var(--qualia-text-muted);overflow-wrap:anywhere;", "{kind_detail(&job.kind)}" }
                    }
                    div { style: "margin-top:8px;font-size:.76rem;line-height:1.4;", "{job.message}" }
                    div { style: "height:7px;margin-top:10px;border-radius:999px;background:rgba(148,163,184,.14);overflow:hidden;",
                        div { style: "height:100%;width:{progress}%;background:{color};transition:width .25s ease;" }
                    }
                    div { style: "display:flex;align-items:center;gap:12px;margin-top:8px;",
                        span { style: "font-size:.65rem;color:var(--qualia-text-muted);", "{progress:.0}%" }
                        if active(&job.status) {
                            button {
                                r#type: "button",
                                onclick: move |_| cancel_job(id_for_cancel.clone(), snapshot, note),
                                style: "border:0;background:transparent;color:#fda4af;font-size:.68rem;font-weight:800;cursor:pointer;padding:0;",
                                "Cancel"
                            }
                        } else {
                            button {
                                r#type: "button",
                                onclick: move |_| retry_job(id_for_retry.clone(), snapshot, note),
                                style: "border:0;background:transparent;color:var(--qualia-accent);font-size:.68rem;font-weight:800;cursor:pointer;padding:0;",
                                "Run again"
                            }
                        }
                    }
                    if let Some(error) = &job.error {
                        pre { style: "margin:9px 0 0;padding:9px;border-radius:8px;background:rgba(127,29,29,.2);color:#fecdd3;white-space:pre-wrap;overflow-wrap:anywhere;font-size:.67rem;", "{error}" }
                    }
                }
            }
        }
    }
}
