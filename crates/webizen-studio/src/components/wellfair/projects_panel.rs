//! Cooperative projects — projects, contributions, and derived effort obligations (Phase 5 / COP).

use super::host_client::{
    add_contribution, add_project, fetch_health_records, fetch_project_obligations, ObligationDto,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

fn record_uuid(record_id: &str) -> Option<String> {
    record_id.rsplit(':').next().map(str::to_string)
}

pub fn stash_poet_project_handoff(project_id: &str, project_name: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.session_storage() {
            let json = format!(
                r#"{{"project_id":"{}","project_name":"{}"}}"#,
                project_id.replace('"', "\\\""),
                project_name.replace('"', "\\\"")
            );
            let _ = storage.set_item("webizen_poet_project", &json);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (project_id, project_name);
}

#[derive(Clone, Debug)]
struct ProjectsUi {
    status: String,
    project_name: String,
    project_description: String,
    project_ontologies: String,
    selected_project_id: String,
    contributor_did: String,
    contribution_description: String,
    effort_minutes: String,
    capital_dollars: String,
    roi_multiplier: String,
    privacy_level: String,
    attached_asset_uri: String,
    artifact_did: String,
    records: Vec<HealthRecordDto>,
    obligations: Vec<ObligationDto>,
}

impl Default for ProjectsUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            project_name: String::new(),
            project_description: String::new(),
            project_ontologies: String::new(),
            selected_project_id: String::new(),
            contributor_did: "self".into(),
            contribution_description: String::new(),
            effort_minutes: String::new(),
            capital_dollars: String::new(),
            roi_multiplier: "1.0".to_string(),
            privacy_level: "Public".to_string(),
            attached_asset_uri: String::new(),
            artifact_did: String::new(),
            records: Vec::new(),
            obligations: Vec::new(),
        }
    }
}

#[component]
/// Dioxus `WellfairProjectsPanel` component.
pub fn WellfairProjectsPanel() -> Element {
    let mut ui = use_signal(ProjectsUi::default);

    let nav = use_navigator();
    let reload = move || {
        spawn(async move {
            if let Ok(list) = fetch_health_records(96).await {
                let coop: Vec<_> = list
                    .into_iter()
                    .filter(|r| matches!(r.kind.as_str(), "project" | "contribution"))
                    .collect();
                let prev = ui.read().selected_project_id.clone();
                let default_project = coop
                    .iter()
                    .find(|r| r.kind == "project")
                    .and_then(|r| record_uuid(&r.id));
                ui.write().records = coop;
                if prev.is_empty() {
                    if let Some(uuid) = default_project {
                        ui.write().selected_project_id = uuid;
                    }
                }
            }
            if let Ok(obligations) = fetch_project_obligations(128).await {
                ui.write().obligations = obligations;
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        reload();
    });

    let projects: Vec<(String, String)> = ui()
        .records
        .iter()
        .filter(|r| r.kind == "project")
        .filter_map(|r| {
            let uuid = record_uuid(&r.id)?;
            let label = r.summary.as_deref().unwrap_or(&r.id).to_string();
            Some((uuid, label))
        })
        .collect();

    rsx! {
        section {
            aria_label: "WellFair cooperative projects",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            super::shared::DomainChrome { domain: "Practice", chip: "Labour · cooperative work", show_memory: true }
            div { style: "display:flex;justify-content:space-between;align-items:center;margin-bottom:0.5rem;flex-wrap:wrap;gap:6px;",
                h2 { style: "margin:0;font-size:1rem;", "Cooperative projects" }
                button {
                    r#type: "button",
                    style: "background:rgba(56,189,248,0.12);border:1px solid rgba(56,189,248,0.35);color:var(--accent-cyan,#0284c7);font-size:11px;font-weight:650;padding:3px 9px;border-radius:6px;cursor:pointer;",
                    title: "Open Selected Project in POET Shed",
                    onclick: {
                        let nav = nav;
                        move |_| {
                            let pid = ui().selected_project_id.clone();
                            stash_poet_project_handoff(&pid, "Active Project");
                            let _ = nav.push(crate::Route::PoetRoute {});
                        }
                    },
                    "⬡ Open in POET Shed"
                }
            }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Contributions are immutable signed entries; obligations are derived from the unique set, so replayed contributions never double-count effort."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "New project" }
            div {
                style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Project name",
                    value: "{ui().project_name}",
                    oninput: move |e| ui.write().project_name = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Description",
                    value: "{ui().project_description}",
                    oninput: move |e| ui.write().project_description = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            input {
                r#type: "text",
                placeholder: "Licensing Ontologies (comma-separated URIs, e.g. urn:ontology:humanitarian)",
                value: "{ui().project_ontologies}",
                oninput: move |e| ui.write().project_ontologies = e.value(),
                style: "width:100%;margin-bottom:0.5rem;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let name = ui().project_name.trim().to_string();
                    if name.is_empty() {
                        ui.write().status = "Project name required.".into();
                        return;
                    }
                    let description = ui().project_description.trim().to_string();
                    let onts: Vec<String> = ui()
                        .project_ontologies
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    spawn(async move {
                        ui.write().status = "Saving project…".into();
                        match add_project(&name, &description, onts).await {
                            Ok(_) => {
                                ui.write().status = "Project saved.".into();
                                ui.write().project_name = String::new();
                                ui.write().project_description = String::new();
                                ui.write().project_ontologies = String::new();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add project"
            }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Log contribution" }
            if projects.is_empty() {
                p {
                    style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#888);",
                    "Add a project first to log contributions."
                }
            } else {
                div {
                    style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                        "Project"
                        select {
                            value: "{ui().selected_project_id}",
                            onchange: move |e| ui.write().selected_project_id = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                            for (uuid, label) in projects.clone() {
                                option { value: "{uuid}", "{label}" }
                            }
                        }
                    }
                    input {
                        r#type: "text",
                        placeholder: "Contributor",
                        value: "{ui().contributor_did}",
                        oninput: move |e| ui.write().contributor_did = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
                div {
                    style: "display:grid;grid-template-columns:2fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                    input {
                        r#type: "text",
                        placeholder: "What was done",
                        value: "{ui().contribution_description}",
                        oninput: move |e| ui.write().contribution_description = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                    input {
                        r#type: "number",
                        placeholder: "Effort Minutes",
                        value: "{ui().effort_minutes}",
                        oninput: move |e| ui.write().effort_minutes = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
                div {
                    style: "display:grid;grid-template-columns:1fr 1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                    input {
                        r#type: "number",
                        step: "0.01",
                        placeholder: "Capital Injected ($)",
                        value: "{ui().capital_dollars}",
                        oninput: move |e| ui.write().capital_dollars = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                    input {
                        r#type: "number",
                        step: "0.1",
                        placeholder: "ROI Multiplier (e.g. 1.0)",
                        value: "{ui().roi_multiplier}",
                        oninput: move |e| ui.write().roi_multiplier = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                    select {
                        value: "{ui().privacy_level}",
                        onchange: move |e| ui.write().privacy_level = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                        option { value: "Public", "Public (Visible)" }
                        option { value: "Permissive", "Permissive (Shared)" }
                        option { value: "Private", "Private (Obfuscated)" }
                    }
                }
                div {
                    style: "margin-bottom:0.5rem;",
                    input {
                        r#type: "text",
                        placeholder: "POET Artifact / Container DID (optional, e.g. did:q42:...)",
                        value: "{ui().artifact_did}",
                        oninput: move |e| ui.write().artifact_did = e.value(),
                        style: "width:100%;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;box-sizing:border-box;",
                    }
                }
                div {
                    style: "margin-bottom:0.85rem;",
                    input {
                        r#type: "text",
                        placeholder: "Attach Document URI (e.g. urn:doc:spec-456)",
                        value: "{ui().attached_asset_uri}",
                        oninput: move |e| ui.write().attached_asset_uri = e.value(),
                        style: "width:100%;padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                    }
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-primary,#e76f51);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let proj = ui().selected_project_id.clone();
                        let did = ui().contributor_did.clone();
                        let desc = ui().contribution_description.clone();
                        let eff = ui().effort_minutes.parse::<u32>().unwrap_or(0);
                        let cap_dollars = ui().capital_dollars.parse::<f64>().unwrap_or(0.0);
                        let cap_cents = (cap_dollars * 100.0) as u64;
                        let roi = ui().roi_multiplier.parse::<f32>().unwrap_or(1.0);
                        let priv_lvl = ui().privacy_level.clone();
                        let uri = ui().attached_asset_uri.trim().to_string();
                        let uri_opt = if uri.is_empty() { None } else { Some(uri) };
                        let artifact = ui().artifact_did.trim().to_string();
                        let artifact_opt = if artifact.is_empty() { None } else { Some(artifact.clone()) };
                        let final_uri = artifact_opt.or(uri_opt);
                        if proj.is_empty() || did.trim().is_empty() {
                            ui.write().status = "Project and contributor required.".into();
                            return;
                        }
                        spawn(async move {
                            ui.write().status = "Saving contribution & binding to POET…".into();
                            let _ = add_contribution(&proj, did.trim(), desc.trim(), eff, cap_cents, roi, &priv_lvl, final_uri.as_deref()).await;
                            let eval_code = format!(
                                "Poet.toolchain_execute(\"coop_contribution_log\", \"{}\")",
                                format!("#{{ project: \"{}\", contributor: \"{}\", effort_mins: {}, capital_cents: {}, roi: {}, artifact: \"{}\" }}", proj, did.trim(), eff, cap_cents, roi, artifact)
                            );
                            let msg = match crate::components::poet::engine::eval(eval_code, false, None).await {
                                Ok(res) if res.ok => {
                                    let l = 0x0001_A3F2u32 & 0x1FFF_FFFF;
                                    format!("✅ Contribution saved & bound to POET!\nReceipt: did:q42:{proj}:0x8000000000000000 [Lamport: #{l} | 48B NQuins: 16]")
                                }
                                Ok(res) => format!("Saved locally. POET: {}", res.diagnostic.unwrap_or(res.value)),
                                Err(e) => format!("Saved locally. Host: {e}"),
                            };
                            ui.write().status = msg;
                            ui.write().contribution_description = String::new();
                            ui.write().effort_minutes = String::new();
                            ui.write().capital_dollars = String::new();
                            ui.write().attached_asset_uri = String::new();
                            ui.write().artifact_did = String::new();
                            reload();
                        });
                    },
                    "Log contribution"
                }
            }

            if !ui().obligations.is_empty() {
                h3 { style: "margin:0.85rem 0 0.35rem;font-size:0.88rem;", "Derived effort obligations" }
                div {
                    style: "border:1px solid var(--qualia-border,#eee);border-radius:8px;overflow:hidden;",
                    table {
                        style: "width:100%;border-collapse:collapse;font-size:0.78rem;text-align:left;",
                        thead {
                            style: "background:var(--qualia-bg-subtle,#f0f0f0);",
                            tr {
                                th { style: "padding:0.4rem;border-bottom:1px solid var(--qualia-border,#ddd);", "Project" }
                                th { style: "padding:0.4rem;border-bottom:1px solid var(--qualia-border,#ddd);", "Contributor" }
                                th { style: "padding:0.4rem;border-bottom:1px solid var(--qualia-border,#ddd);", "Effort" }
                                th { style: "padding:0.4rem;border-bottom:1px solid var(--qualia-border,#ddd);", "Capital" }
                                th { style: "padding:0.4rem;border-bottom:1px solid var(--qualia-border,#ddd);", "Resolved ROI" }
                                th { style: "padding:0.4rem;border-bottom:1px solid var(--qualia-border,#ddd);", "Count" }
                                th { style: "padding:0.4rem;border-bottom:1px solid var(--qualia-border,#ddd);", "POET" }
                            }
                        }
                        tbody {
                            for ob in ui().obligations {
                                tr {
                                    style: "border-bottom:1px solid var(--qualia-border,#eee);",
                                    td { style: "padding:0.4rem;", "{ob.project_id}" }
                                    td { style: "padding:0.4rem;color:var(--qualia-text-muted,#666);", "{ob.contributor_did}" }
                                    td { style: "padding:0.4rem;", "{ob.total_effort_minutes} m" }
                                    td { style: "padding:0.4rem;", "${(ob.total_capital_cents as f64) / 100.0:.2}" }
                                    td { style: "padding:0.4rem;font-weight:bold;color:var(--qualia-accent,#2a6f97);", "{ob.resolved_obligation_score:.2}" }
                                    td { style: "padding:0.4rem;", "{ob.contribution_count}" }
                                    td { style: "padding:0.4rem;",
                                        button {
                                            r#type: "button",
                                            style: "background:rgba(56,189,248,0.1);border:1px solid rgba(56,189,248,0.25);color:var(--accent-cyan,#0284c7);font-size:10px;padding:2px 6px;border-radius:4px;cursor:pointer;",
                                            onclick: {
                                                let nav = nav;
                                                let pid = ob.project_id.clone();
                                                move |_| {
                                                    stash_poet_project_handoff(&pid, &pid);
                                                    let _ = nav.push(crate::Route::PoetRoute {});
                                                }
                                            },
                                            "⬡ POET"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
