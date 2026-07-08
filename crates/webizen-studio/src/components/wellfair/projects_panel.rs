//! Cooperative projects — projects, contributions, and derived effort obligations (Phase 5 / COP).

use super::host_client::{
    add_contribution, add_project, fetch_health_records, fetch_project_obligations, ObligationDto,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

fn record_uuid(record_id: &str) -> Option<String> {
    record_id.rsplit(':').next().map(str::to_string)
}

#[derive(Clone, Debug)]
struct ProjectsUi {
    status: String,
    project_name: String,
    project_description: String,
    selected_project_id: String,
    contributor_did: String,
    contribution_description: String,
    effort_minutes: String,
    attached_asset_uri: String,
    records: Vec<HealthRecordDto>,
    obligations: Vec<ObligationDto>,
}

impl Default for ProjectsUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            project_name: String::new(),
            project_description: String::new(),
            selected_project_id: String::new(),
            contributor_did: "self".into(),
            contribution_description: String::new(),
            effort_minutes: String::new(),
            attached_asset_uri: String::new(),
            records: Vec::new(),
            obligations: Vec::new(),
        }
    }
}

#[component]
pub fn WellfairProjectsPanel() -> Element {
    let mut ui = use_signal(ProjectsUi::default);

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
        if loaded() { return; }
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
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Cooperative projects" }
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
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let name = ui().project_name.trim().to_string();
                    if name.is_empty() {
                        ui.write().status = "Project name required.".into();
                        return;
                    }
                    let description = ui().project_description.trim().to_string();
                    spawn(async move {
                        ui.write().status = "Saving project…".into();
                        match add_project(&name, &description).await {
                            Ok(_) => {
                                ui.write().status = "Project saved.".into();
                                ui.write().project_name = String::new();
                                ui.write().project_description = String::new();
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
                        placeholder: "Minutes",
                        value: "{ui().effort_minutes}",
                        oninput: move |e| ui.write().effort_minutes = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
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
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let mut project_id = ui().selected_project_id.trim().to_string();
                        if project_id.is_empty() {
                            if let Some((uuid, _)) = projects.first() {
                                project_id = uuid.clone();
                            }
                        }
                        let contributor = ui().contributor_did.trim().to_string();
                        let description = ui().contribution_description.trim().to_string();
                        let minutes: u32 = ui().effort_minutes.trim().parse().unwrap_or(0);
                        if project_id.is_empty() || description.is_empty() || minutes == 0 {
                            ui.write().status = "Select a project, describe the work, and enter minutes.".into();
                            return;
                        }
                        let contributor = if contributor.is_empty() { "self".to_string() } else { contributor };
                        let asset_uri = ui().attached_asset_uri.trim().to_string();
                        let attached_uri = if asset_uri.is_empty() { None } else { Some(asset_uri) };
                        spawn(async move {
                            ui.write().status = "Saving contribution…".into();
                            match add_contribution(&project_id, &contributor, &description, minutes, attached_uri.as_deref()).await {
                                Ok(_) => {
                                    ui.write().status = "Contribution saved.".into();
                                    ui.write().contribution_description = String::new();
                                    ui.write().effort_minutes = String::new();
                                    ui.write().attached_asset_uri = String::new();
                                    reload();
                                }
                                Err(e) => ui.write().status = format!("Failed: {e}"),
                            }
                        });
                    },
                    "Log contribution"
                }
            }

            if !ui().obligations.is_empty() {
                h3 { style: "margin:0.85rem 0 0.35rem;font-size:0.88rem;", "Derived effort obligations" }
                ul {
                    style: "margin:0 0 0.5rem;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.3rem;",
                    for ob in ui().obligations.clone() {
                        li {
                            key: "{ob.project_id}-{ob.contributor_did}",
                            style: "display:flex;justify-content:space-between;padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.76rem;",
                            span { "{ob.contributor_did}" }
                            span { style: "color:var(--qualia-text-muted,#666);",
                                "{ob.total_effort_minutes} min · {ob.contribution_count} contributions"
                            }
                        }
                    }
                }
            }
        }
    }
}
