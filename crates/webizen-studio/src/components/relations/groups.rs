use crate::components::settings::host::invoke_json;
use dioxus::prelude::*;

fn value_text(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[component]
pub fn GroupsOverview() -> Element {
    let mut sessions = use_signal(Vec::<serde_json::Value>::new);
    let mut projects = use_signal(Vec::<serde_json::Value>::new);
    let mut status = use_signal(String::new);
    let mut project_name = use_signal(String::new);
    let mut project_description = use_signal(String::new);
    let mut creating = use_signal(|| false);

    let mut refresh = move || {
        status.set("Loading groups and cooperative spaces…".to_string());
        spawn(async move {
            if let Ok(value) =
                invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", serde_json::json!({}))
                    .await
            {
                sessions.set(
                    value
                        .into_iter()
                        .filter(|item| {
                            item.get("participant_count")
                                .and_then(serde_json::Value::as_u64)
                                .unwrap_or(0)
                                > 1
                                || value_text(item, "kind")
                                    .to_ascii_lowercase()
                                    .contains("group")
                        })
                        .collect(),
                );
            }
            if let Ok(value) =
                invoke_json::<serde_json::Value>("list_coop_projects", serde_json::json!({})).await
            {
                projects.set(
                    value
                        .as_array()
                        .cloned()
                        .or_else(|| {
                            value
                                .get("projects")
                                .and_then(serde_json::Value::as_array)
                                .cloned()
                        })
                        .unwrap_or_default(),
                );
            }
            status.set(format!(
                "{} group conversation(s) · {} cooperative project(s)",
                sessions().len(),
                projects().len()
            ));
        });
    };
    use_hook(move || refresh());

    rsx! {
        section { style: "height:100%;overflow-y:auto;padding:22px;display:grid;gap:15px;",
            div { style: "display:flex;justify-content:space-between;gap:14px;align-items:flex-start;flex-wrap:wrap;",
                div {
                    h2 { style: "margin:0;font-size:1.15rem;", "Groups & commons" }
                    p { style: "margin:5px 0 0;color:var(--qualia-text-muted);font-size:.76rem;line-height:1.5;", "Shared conversation, projects and deliberately common artefacts." }
                }
                button { style: "{crate::components::settings::SECONDARY_BUTTON}", onclick: move |_| refresh(), "Refresh" }
            }
            div { role: "status", style: "font-size:.7rem;color:var(--qualia-text-muted);", "{status}" }
            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(250px,1fr));gap:12px;",
                div { style: "{crate::components::settings::PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.9rem;", "Group conversations" }
                    for session in sessions() {
                        div { style: "{crate::components::settings::ACTION_ROW}",
                            strong { "{value_text(&session, \"title\")}" }
                            div { style: "margin-top:4px;font-size:.65rem;color:var(--qualia-text-muted);", "{session.get(\"message_count\").and_then(serde_json::Value::as_u64).unwrap_or(0)} messages" }
                        }
                    }
                    if sessions().is_empty() { div { style: "{crate::components::settings::EMPTY_CARD}", "No group conversations yet." } }
                }
                div { style: "{crate::components::settings::PANEL}",
                    h3 { style: "margin:0 0 10px;font-size:.9rem;", "Cooperative projects" }
                    for project in projects() {
                        div { style: "{crate::components::settings::ACTION_ROW}",
                            strong { "{value_text(&project, \"name\")}" }
                            div { style: "margin-top:4px;font-size:.65rem;color:var(--qualia-text-muted);", "{value_text(&project, \"description\")}" }
                        }
                    }
                    if projects().is_empty() { div { style: "{crate::components::settings::EMPTY_CARD}", "No cooperative projects yet." } }
                }
            }
            form {
                style: "{crate::components::settings::PANEL} display:grid;gap:10px;",
                onsubmit: move |event| {
                    event.prevent_default();
                    let name = project_name().trim().to_string();
                    let description = project_description().trim().to_string();
                    if name.is_empty() {
                        status.set("Give the cooperative project a name.".to_string());
                        return;
                    }
                    creating.set(true);
                    status.set("Creating cooperative project…".to_string());
                    spawn(async move {
                        match invoke_json::<serde_json::Value>(
                            "create_coop_project",
                            serde_json::json!({ "name": name, "description": description }),
                        ).await {
                            Ok(_) => {
                                project_name.set(String::new());
                                project_description.set(String::new());
                                match invoke_json::<serde_json::Value>("list_coop_projects", serde_json::json!({})).await {
                                    Ok(value) => {
                                        projects.set(
                                            value.as_array().cloned()
                                                .or_else(|| value.get("projects").and_then(serde_json::Value::as_array).cloned())
                                                .unwrap_or_default(),
                                        );
                                        status.set("Cooperative project created.".to_string());
                                    }
                                    Err(error) => status.set(format!("Project created, but refresh failed: {error}")),
                                }
                            }
                            Err(error) => status.set(format!("Could not create project: {error}")),
                        }
                        creating.set(false);
                    });
                },
                h3 { style: "margin:0;font-size:.9rem;", "Start a cooperative project" }
                p { style: "margin:0;color:var(--qualia-text-muted);font-size:.7rem;line-height:1.45;", "Create the shared space here; technical join packages and low-level controls remain available in Existing tools." }
                input {
                    required: true,
                    value: "{project_name}",
                    placeholder: "Project name",
                    aria_label: "Project name",
                    style: "{crate::components::settings::FIELD}",
                    oninput: move |event| project_name.set(event.value()),
                }
                textarea {
                    value: "{project_description}",
                    placeholder: "Purpose and expectations",
                    aria_label: "Project description",
                    style: "{crate::components::settings::FIELD} min-height:78px;resize:vertical;",
                    oninput: move |event| project_description.set(event.value()),
                }
                button { r#type: "submit", disabled: creating(), style: "{crate::components::settings::PRIMARY_BUTTON}", "Create project" }
            }
        }
    }
}
