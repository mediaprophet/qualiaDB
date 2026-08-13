//! Agent roster + MCP tool allowlist editor (U3-B).
//!
//! Replaces the prior mock temperature/deontic sliders with a real path:
//! load roster → multi-select tools from local MCP catalogue → save via
//! `agent_set_allowed_mcp_tools` / roster upsert.

#![allow(non_snake_case)]
use dioxus::prelude::*;

use crate::components::honesty_chip::{HonestyChip, HonestyLevel};

#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
async fn invoke_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    if !crate::endpoints::is_native_host() {
        return Err("The desktop host is unavailable in this preview.".to_string());
    }
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

fn tool_name_of(v: &serde_json::Value) -> String {
    v.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string()
}

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}

fn comma_list(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|entries| entries.as_array())
        .map(|entries| entries.iter().filter_map(|entry| entry.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn semantic_draft_from_agent(agent: &serde_json::Value) -> String {
    agent
        .get("semantic_profile")
        .and_then(|profile| profile.get("tags"))
        .and_then(|tags| tags.as_array())
        .map(|tags| {
            tags.iter()
                .map(|tag| {
                    format!(
                        "{} | {} | {} | {}",
                        s(tag, "facet"),
                        s(tag, "label"),
                        s(tag, "iri"),
                        tag.get("broader_iri")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

#[cfg(target_arch = "wasm32")]
fn semantic_tags_from_draft(draft: &str) -> Result<Vec<serde_json::Value>, String> {
    let permitted = [
        "classification",
        "specialisation",
        "geography",
        "language",
        "method",
        "dataset",
        "tool",
        "constraint",
    ];
    let mut tags = Vec::new();
    for line in draft.lines().filter(|line| !line.trim().is_empty()) {
        let parts: Vec<_> = line.split('|').map(str::trim).collect();
        if parts.len() < 3 || parts.len() > 4 {
            return Err(
                "Each semantic tag must be: facet | label | IRI | optional broader IRI".into(),
            );
        }
        if !permitted.contains(&parts[0]) {
            return Err(format!("Unknown semantic facet `{}`", parts[0]));
        }
        if parts[1].is_empty() || parts[2].is_empty() {
            return Err("Semantic tags need a non-empty label and IRI.".into());
        }
        tags.push(json!({
            "facet": parts[0], "label": parts[1], "iri": parts[2],
            "broader_iri": if parts.get(3).is_some_and(|value| !value.is_empty()) { json!(parts[3]) } else { serde_json::Value::Null },
        }));
    }
    if tags.len() > 32 {
        return Err("An agent may have at most 32 semantic tags.".into());
    }
    Ok(tags)
}

#[component]
pub fn AgentConfig() -> Element {
    let agents = use_signal(Vec::<serde_json::Value>::new);
    let tools = use_signal(Vec::<serde_json::Value>::new);
    let selected_slug = use_signal(|| "local".to_string());
    let allowed = use_signal(Vec::<String>::new);
    let status = use_signal(String::new);
    let runtime_status = use_signal(String::new);
    let wildcard = use_signal(|| false);
    let new_name = use_signal(String::new);
    let new_slug = use_signal(String::new);
    let new_role = use_signal(String::new);
    let new_model = use_signal(String::new);
    let new_instructions = use_signal(String::new);
    let new_context_sources = use_signal(String::new);
    let new_context_recipients = use_signal(String::new);
    let new_context_confirmation = use_signal(|| false);
    let new_semantic_draft = use_signal(String::new);
    let remote_name = use_signal(String::new);
    let remote_slug = use_signal(String::new);
    let remote_endpoint = use_signal(String::new);
    let remote_model = use_signal(String::new);
    let remote_instructions = use_signal(String::new);
    let remote_bearer = use_signal(String::new);
    let edit_name = use_signal(String::new);
    let edit_role = use_signal(String::new);
    let edit_model = use_signal(String::new);
    let edit_instructions = use_signal(String::new);
    let edit_enabled = use_signal(|| true);
    let edit_semantic_draft = use_signal(String::new);
    let edit_allowed_ontologies = use_signal(String::new);

    #[cfg(target_arch = "wasm32")]
    use_hook(|| {
        let mut agents = agents;
        let mut tools = tools;
        let mut selected_slug = selected_slug;
        let mut allowed = allowed;
        let mut wildcard = wildcard;
        let mut status = status;
        let mut runtime_status = runtime_status;
        let mut edit_name = edit_name;
        let mut edit_role = edit_role;
        let mut edit_model = edit_model;
        let mut edit_instructions = edit_instructions;
        let mut edit_enabled = edit_enabled;
        let mut edit_semantic_draft = edit_semantic_draft;
        let mut edit_allowed_ontologies = edit_allowed_ontologies;
        spawn(async move {
            if let Ok(list) =
                invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({})).await
            {
                if selected_slug().is_empty() {
                    if let Some(first) = list.first() {
                        selected_slug.set(s(first, "slug"));
                    }
                }
                agents.set(list);
            }
            match invoke_json::<Vec<serde_json::Value>>("mcp_list_local_tools", json!({})).await {
                Ok(list) => tools.set(list),
                Err(e) => status.set(format!("tools/list: {e}")),
            }
            // Load allowlist for current agent
            let slug = selected_slug();
            if let Ok(agent) =
                invoke_json::<serde_json::Value>("agent_roster_get", json!({ "slug": slug })).await
            {
                edit_name.set(s(&agent, "display_name"));
                edit_role.set(s(&agent, "description"));
                edit_instructions.set(s(&agent, "system_prompt"));
                edit_enabled.set(
                    agent
                        .get("enabled")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(true),
                );
                edit_model.set(
                    agent
                        .get("backend")
                        .and_then(|backend| backend.get("local_engine"))
                        .and_then(|local| local.get("model_id"))
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                );
                edit_semantic_draft.set(semantic_draft_from_agent(&agent));
                edit_allowed_ontologies.set(comma_list(
                    agent.get("data_policy").unwrap_or(&serde_json::Value::Null),
                    "allowed_ontology_ids",
                ));
                if let Some(arr) = agent.get("allowed_mcp_tools").and_then(|a| a.as_array()) {
                    let list: Vec<String> = arr
                        .iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect();
                    wildcard.set(list.iter().any(|t| t == "*"));
                    allowed.set(list);
                }
            }
            if let Ok(runtime) = invoke_json::<serde_json::Value>(
                "agent_runtime_status",
                json!({ "slug": selected_slug() }),
            )
            .await
            {
                let backend = s(&runtime, "backend");
                let lifecycle = s(&runtime, "lifecycle_state");
                let resident = runtime
                    .get("resident")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                let speed = runtime
                    .get("last_decode_tokens_per_sec")
                    .and_then(|value| value.as_f64())
                    .map(|value| format!(" · last {:.2} tok/s", value))
                    .unwrap_or_default();
                runtime_status.set(format!(
                    "{} · {} · {}{}",
                    backend,
                    if resident { "resident" } else { "on demand" },
                    lifecycle,
                    speed
                ));
            }
        });
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &agents,
            &tools,
            &selected_slug,
            &allowed,
            &status,
            &runtime_status,
            &wildcard,
            &new_name,
            &new_slug,
            &new_role,
            &new_model,
            &new_instructions,
            &new_context_sources,
            &new_context_recipients,
            &new_context_confirmation,
            &new_semantic_draft,
            &remote_name,
            &remote_slug,
            &remote_endpoint,
            &remote_model,
            &remote_instructions,
            &remote_bearer,
            &edit_name,
            &edit_role,
            &edit_model,
            &edit_instructions,
            &edit_enabled,
            &edit_semantic_draft,
            &edit_allowed_ontologies,
        );
    }

    let is_checked = move |name: &str| -> bool {
        if wildcard() {
            return true;
        }
        allowed().iter().any(|t| t == name)
    };

    rsx! {
        div { style: "padding: 24px; background: #111827; color: #f3f4f6; height: 100%; box-sizing: border-box; overflow-y: auto;",
            div { style: "max-width: 860px; margin: 0 auto;",
                div { style: "display:flex; justify-content:space-between; align-items:center; gap:12px; margin-bottom:16px; border-bottom:1px solid #374151; padding-bottom:12px;",
                    h2 { style: "color: #a78bfa; margin:0; font-size: 24px;", "Agent · MCP allowlist" }
                    HonestyChip {
                        level: HonestyLevel::Partial,
                        detail: "Empty = deny-all · * = all tools".to_string(),
                    }
                }

                p { style: "color:#94a3b8; font-size:13px; line-height:1.5; margin:0 0 16px;",
                    "Create named agents with their own purpose, instructions, and local model. Tools the selected agent may use after an explicit principal "
                    strong { "Permit" }
                    " in Talk. Empty allowlist means no tools. Prefer explicit names over "
                    code { style: "color:#e9d5ff;", "*" }
                    "."
                }

                if !status().is_empty() {
                    div { style: "background:#0b3b2e; border:1px solid #10b981; color:#a7f3d0; padding:8px 12px; border-radius:8px; font-size:12px; margin-bottom:12px; white-space:pre-wrap;",
                        "{status}"
                    }
                }

                div { style: "background:#172033; padding:16px; border:1px solid #334155; border-radius:12px; margin-bottom:16px;",
                    h3 { style: "margin:0 0 6px; color:#e9d5ff; font-size:15px;", "Create local agent" }
                    p { style: "margin:0 0 12px; color:#94a3b8; font-size:12px; line-height:1.45;",
                        "An agent is a saved identity and policy. Its model is loaded on demand by the local runtime; creating it does not upload data or start inference."
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Name" }
                    input {
                        style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:10px;",
                        placeholder: "e.g. Researcher", value: "{new_name}",
                        oninput: move |e| { let mut v = new_name; v.set(e.value()); }
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Stable @ handle (lowercase letters, digits, hyphens)" }
                    input {
                        style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:10px;",
                        placeholder: "researcher", value: "{new_slug}",
                        oninput: move |e| { let mut v = new_slug; v.set(e.value()); }
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Role / purpose" }
                    input {
                        style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:10px;",
                        placeholder: "Evidence-oriented research assistant", value: "{new_role}",
                        oninput: move |e| { let mut v = new_role; v.set(e.value()); }
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Installed local model ID (blank = current default)" }
                    input {
                        style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:10px;",
                        placeholder: "smollm2-360m-instruct-q8_0", value: "{new_model}",
                        oninput: move |e| { let mut v = new_model; v.set(e.value()); }
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Instructions / characteristics" }
                    textarea {
                        style: "width:100%;min-height:90px;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:12px;font-family:inherit;",
                        placeholder: "How this agent should help, communicate, and handle uncertainty.", value: "{new_instructions}",
                        oninput: move |e| { let mut v = new_instructions; v.set(e.value()); }
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "May read summaries from (comma-separated @ handles; blank = none)" }
                    input {
                        style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:10px;",
                        placeholder: "@researcher, @reviewer", value: "{new_context_sources}",
                        oninput: move |e| { let mut v = new_context_sources; v.set(e.value()); }
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "May share answer summaries with (comma-separated @ handles; blank = owner only)" }
                    input {
                        style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:10px;",
                        placeholder: "@reviewer", value: "{new_context_recipients}",
                        oninput: move |e| { let mut v = new_context_recipients; v.set(e.value()); }
                    }
                    label { style: "display:flex;align-items:center;gap:8px;color:#cbd5e1;font-size:12px;margin:0 0 12px;",
                        input {
                            r#type: "checkbox", checked: new_context_confirmation(),
                            onchange: move |e| { let mut v = new_context_confirmation; v.set(e.checked()); }
                        }
                        "Ask me to confirm this agent's context before each turn"
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Pointed study graph (one tag per line)" }
                    textarea {
                        style: "width:100%;min-height:110px;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:5px;font-family:ui-monospace,monospace;font-size:12px;",
                        placeholder: "classification | Researcher | q42:Researcher\nspecialisation | History | q42:History | q42:Researcher\nspecialisation | Australian History | q42:AustralianHistory | q42:History\ngeography | Australia | geo:AU\nmethod | Source criticism | q42:SourceCriticism",
                        value: "{new_semantic_draft}",
                        oninput: move |e| { let mut value = new_semantic_draft; value.set(e.value()); }
                    }
                    p { style: "margin:0 0 12px;color:#94a3b8;font-size:11px;line-height:1.4;",
                        "Facets: classification, specialisation, geography, language, method, dataset, tool, constraint. Tags focus retrieval; they do not grant a dataset or tool permission."
                    }
                    button {
                        style: "background:#8b5cf6;color:white;padding:10px 18px;border:none;border-radius:8px;font-weight:600;cursor:pointer;",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (mut new_name, mut new_slug, mut new_role, mut new_model, mut new_instructions, mut new_context_sources, mut new_context_recipients, mut new_context_confirmation, mut new_semantic_draft, mut agents, mut selected_slug, mut status) =
                                    (new_name, new_slug, new_role, new_model, new_instructions, new_context_sources, new_context_recipients, new_context_confirmation, new_semantic_draft, agents, selected_slug, status);
                                spawn(async move {
                                    let name = new_name().trim().to_string();
                                    let slug = new_slug().trim().to_string();
                                    if name.is_empty() || slug.is_empty() {
                                        status.set("Name and stable @ handle are required.".into());
                                        return;
                                    }
                                    let model = new_model().trim().to_string();
                                    let handles = |value: String| -> Vec<String> {
                                        value.split(',')
                                            .map(|item| item.trim().trim_start_matches('@').to_string())
                                            .filter(|item| !item.is_empty())
                                            .collect()
                                    };
                                    let sources = handles(new_context_sources());
                                    let recipients = handles(new_context_recipients());
                                    let tags = match semantic_tags_from_draft(&new_semantic_draft()) {
                                        Ok(tags) => tags,
                                        Err(e) => { status.set(e); return; }
                                    };
                                    let agent = json!({
                                        "slug": slug,
                                        "display_name": name,
                                        "description": new_role(),
                                        "backend": { "local_engine": { "model_id": if model.is_empty() { serde_json::Value::Null } else { json!(model) } } },
                                        "system_prompt": new_instructions(),
                                        "allowed_mcp_tools": [],
                                        "max_sensitivity": 255,
                                        "outcome_sharing": { "visibility": "owner_only", "share_provenance": true, "share_model_attribution": false, "allow_peer_llm_context": false, "allowed_dids": [] },
                                        "context_policy": {
                                            "conversation": "addressed_message", "retrieval": "permitted_scopes", "attachments": "permitted_attachments",
                                            "allowed_source_agents": sources,
                                            "default_visibility": if recipients.is_empty() { "owner_only" } else { "named_agents" },
                                            "allowed_recipient_agents": recipients,
                                            "may_share_raw_prompt": false, "may_share_attachments": false,
                                            "may_share_graph_records": false, "may_share_provenance": true,
                                            "require_turn_confirmation": new_context_confirmation(),
                                        },
                                        "semantic_profile": { "tags": tags },
                                        "enabled": true
                                    });
                                    match invoke_json::<()>("agent_roster_upsert", json!({ "agentJson": agent.to_string() })).await {
                                        Ok(()) => {
                                            let selected = agent.get("slug").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                                            selected_slug.set(selected.clone());
                                            new_name.set(String::new());
                                            new_slug.set(String::new());
                                            new_role.set(String::new());
                                            new_model.set(String::new());
                                            new_instructions.set(String::new());
                                            new_context_sources.set(String::new());
                                            new_context_recipients.set(String::new());
                                            new_context_confirmation.set(false);
                                            new_semantic_draft.set(String::new());
                                            match invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({})).await {
                                                Ok(list) => { agents.set(list); status.set(format!("Created @{selected}. Select it below to set its tool allowlist.")); }
                                                Err(e) => status.set(format!("Agent saved, but refresh failed: {e}")),
                                            }
                                        }
                                        Err(e) => status.set(format!("Could not create agent: {e}")),
                                    }
                                });
                            }
                        },
                        "Create local agent"
                    }
                }

                div { style: "background:#172033; padding:16px; border:1px solid #334155; border-radius:12px; margin-bottom:16px;",
                    h3 { style: "margin:0 0 6px; color:#e9d5ff; font-size:15px;", "Add remote MCP agent" }
                    p { style: "margin:0 0 12px; color:#94a3b8; font-size:12px; line-height:1.45;",
                        "The endpoint is contacted only when you explicitly test it or invoke this agent. A bearer token, if required, is written to this device's OS keychain and is never shown again or saved in the roster."
                    }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Name" }
                    input { style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;", placeholder: "e.g. Hosted researcher", value: "{remote_name}", oninput: move |e| { let mut value = remote_name; value.set(e.value()); } }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Stable @ handle / keychain connection ID" }
                    input { style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;", placeholder: "hosted-researcher", value: "{remote_slug}", oninput: move |e| { let mut value = remote_slug; value.set(e.value()); } }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "MCP HTTP endpoint" }
                    input { style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;", placeholder: "https://example.net/mcp", value: "{remote_endpoint}", oninput: move |e| { let mut value = remote_endpoint; value.set(e.value()); } }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Provider model ID (optional)" }
                    input { style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;", placeholder: "provider-selected-model", value: "{remote_model}", oninput: move |e| { let mut value = remote_model; value.set(e.value()); } }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Bearer token (optional; stored locally in OS keychain)" }
                    input { r#type: "password", autocomplete: "off", style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;", placeholder: "Paste only if this MCP host requires it", value: "{remote_bearer}", oninput: move |e| { let mut value = remote_bearer; value.set(e.value()); } }
                    label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Instructions / characteristics" }
                    textarea { style: "width:100%;min-height:66px;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:12px;font-family:inherit;", placeholder: "What this external agent is allowed to do.", value: "{remote_instructions}", oninput: move |e| { let mut value = remote_instructions; value.set(e.value()); } }
                    button {
                        style: "background:#0ea5e9;color:white;padding:10px 18px;border:none;border-radius:8px;font-weight:600;cursor:pointer;",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (mut remote_name, mut remote_slug, mut remote_endpoint, mut remote_model, mut remote_instructions, mut remote_bearer, mut agents, mut selected_slug, mut status) =
                                    (remote_name, remote_slug, remote_endpoint, remote_model, remote_instructions, remote_bearer, agents, selected_slug, status);
                                spawn(async move {
                                    let name = remote_name().trim().to_string();
                                    let slug = remote_slug().trim().to_string();
                                    let endpoint = remote_endpoint().trim().to_string();
                                    if name.is_empty() || slug.is_empty() || endpoint.is_empty() {
                                        status.set("Remote name, @ handle, and MCP HTTP endpoint are required.".into());
                                        return;
                                    }
                                    if !endpoint.starts_with("https://") && !endpoint.starts_with("http://") {
                                        status.set("MCP HTTP endpoint must start with https:// or http://.".into());
                                        return;
                                    }
                                    let bearer = remote_bearer();
                                    let has_bearer = !bearer.trim().is_empty();
                                    if has_bearer {
                                        if let Err(error) = invoke_json::<()>("provider_credential_store", json!({ "connectionId": slug, "bearer": bearer })).await {
                                            status.set(format!("Could not save the key in the OS keychain: {error}"));
                                            return;
                                        }
                                    }
                                    let model = remote_model().trim().to_string();
                                    let agent = json!({
                                        "slug": slug,
                                        "display_name": name,
                                        "description": "Remote MCP agent configured by the principal.",
                                        "backend": { "remote_mcp": {
                                            "endpoint": endpoint,
                                            "transport": { "http": { "url": endpoint, "credential_id": if has_bearer { json!(remote_slug()) } else { serde_json::Value::Null } } },
                                            "infer_tool": "llm_infer",
                                            "model": if model.is_empty() { serde_json::Value::Null } else { json!(model) }
                                        } },
                                        "system_prompt": remote_instructions(),
                                        "allowed_mcp_tools": [], "max_sensitivity": 0,
                                        "outcome_sharing": { "visibility": "owner_only", "share_provenance": true, "share_model_attribution": true, "allow_peer_llm_context": false, "allowed_dids": [] },
                                        "context_policy": { "conversation": "addressed_message", "retrieval": "permitted_scopes", "attachments": "none", "allowed_source_agents": [], "default_visibility": "owner_only", "allowed_recipient_agents": [], "may_share_raw_prompt": false, "may_share_attachments": false, "may_share_graph_records": false, "may_share_provenance": true, "require_turn_confirmation": true },
                                        "execution_policy": { "residency": "on_demand", "priority": "interactive", "max_parallel_turns": 1, "remote_consent": "per_turn", "allow_scheduled_runs": false },
                                        "semantic_profile": { "tags": [] }, "enabled": true
                                    });
                                    match invoke_json::<()>("agent_roster_upsert", json!({ "agentJson": agent.to_string() })).await {
                                        Ok(()) => {
                                            selected_slug.set(agent.get("slug").and_then(|value| value.as_str()).unwrap_or_default().to_string());
                                            remote_name.set(String::new()); remote_slug.set(String::new()); remote_endpoint.set(String::new()); remote_model.set(String::new()); remote_instructions.set(String::new()); remote_bearer.set(String::new());
                                            match invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({})).await { Ok(list) => { agents.set(list); status.set("Remote MCP agent saved. Select it and use Test connection before your first turn.".into()); }, Err(error) => status.set(format!("Remote agent saved, but refresh failed: {error}")) }
                                        }
                                        Err(error) => status.set(format!("Could not save remote agent: {error}")),
                                    }
                                });
                            }
                        },
                        "Save remote MCP agent"
                    }
                }

                div { style: "background: #1f2937; padding: 16px; border-radius: 12px; margin-bottom: 16px;",
                    label { style: "display:block; margin-bottom:8px; color:#9ca3af; font-size:12px;", "Agent" }
                    select {
                        style: "width:100%; box-sizing:border-box; padding:8px 10px; background:#0b1220; color:#f3f4f6; border:1px solid #334155; border-radius:8px; margin-bottom:12px;",
                        value: "{selected_slug}",
                        onchange: move |e| {
                            let slug = e.value();
                            let mut selected_slug = selected_slug;
                            selected_slug.set(slug.clone());
                            #[cfg(target_arch = "wasm32")]
                            {
                                let mut allowed = allowed;
                                let mut wildcard = wildcard;
                                let mut runtime_status = runtime_status;
                                let mut edit_name = edit_name;
                                let mut edit_role = edit_role;
                                let mut edit_model = edit_model;
                                let mut edit_instructions = edit_instructions;
                                let mut edit_enabled = edit_enabled;
                                let mut edit_semantic_draft = edit_semantic_draft;
                                let mut edit_allowed_ontologies = edit_allowed_ontologies;
                                let mut status = status;
                                spawn(async move {
                                    match invoke_json::<serde_json::Value>(
                                        "agent_roster_get",
                                        json!({ "slug": slug }),
                                    )
                                    .await
                                    {
                                        Ok(agent) => {
                                            edit_name.set(s(&agent, "display_name"));
                                            edit_role.set(s(&agent, "description"));
                                            edit_instructions.set(s(&agent, "system_prompt"));
                                            edit_enabled.set(agent.get("enabled").and_then(|value| value.as_bool()).unwrap_or(true));
                                            edit_model.set(agent.get("backend").and_then(|backend| backend.get("local_engine")).and_then(|local| local.get("model_id")).and_then(|value| value.as_str()).unwrap_or_default().to_string());
                                            edit_semantic_draft.set(semantic_draft_from_agent(&agent));
                                            edit_allowed_ontologies.set(comma_list(agent.get("data_policy").unwrap_or(&serde_json::Value::Null), "allowed_ontology_ids"));
                                            if let Some(arr) = agent
                                                .get("allowed_mcp_tools")
                                                .and_then(|a| a.as_array())
                                            {
                                                let list: Vec<String> = arr
                                                    .iter()
                                                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                                    .collect();
                                                wildcard.set(list.iter().any(|t| t == "*"));
                                                allowed.set(list);
                                            } else {
                                                allowed.set(Vec::new());
                                                wildcard.set(false);
                                            }
                                            if let Ok(runtime) = invoke_json::<serde_json::Value>(
                                                "agent_runtime_status",
                                                json!({ "slug": slug }),
                                            ).await {
                                                let backend = s(&runtime, "backend");
                                                let lifecycle = s(&runtime, "lifecycle_state");
                                                let resident = runtime.get("resident").and_then(|v| v.as_bool()).unwrap_or(false);
                                                let speed = runtime.get("last_decode_tokens_per_sec").and_then(|v| v.as_f64()).map(|v| format!(" · last {:.2} tok/s", v)).unwrap_or_default();
                                                runtime_status.set(format!("{} · {} · {}{}", backend, if resident { "resident" } else { "on demand" }, lifecycle, speed));
                                            }
                                        }
                                        Err(e) => status.set(format!("Load agent: {e}")),
                                    }
                                });
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let _ = slug;
                            }
                        },
                        for a in agents() {
                            option { value: "{s(&a, \"slug\")}",
                                "{s(&a, \"display_name\")} ({s(&a, \"slug\")})"
                            }
                        }
                    }
                    if !runtime_status().is_empty() {
                        div { style: "margin:-2px 0 12px;padding:8px 10px;border-radius:8px;background:#0b1220;color:#a7f3d0;font-size:12px;",
                            "Runtime: {runtime_status}"
                        }
                    }
                    button {
                        style: "background:#0ea5e9;color:white;padding:7px 12px;border:none;border-radius:8px;font-weight:600;cursor:pointer;font-size:12px;margin:0 0 12px;",
                        title: "Runs MCP tools/list only; does not send a chat prompt.",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let slug = selected_slug();
                                let mut status = status;
                                spawn(async move {
                                    status.set(format!("Testing @{slug} connection…"));
                                    match invoke_json::<serde_json::Value>("agent_remote_connection_test", json!({ "slug": slug })).await {
                                        Ok(result) => status.set(format!("Connection verified: {} MCP tools available.", result.get("tool_count").and_then(|value| value.as_u64()).unwrap_or(0))),
                                        Err(error) => status.set(format!("Connection test failed: {error}")),
                                    }
                                });
                            }
                        },
                        "Test remote MCP connection"
                    }
                    div { style: "padding:12px;border:1px solid #334155;border-radius:8px;background:#172033;margin-bottom:12px;",
                        h3 { style: "margin:0 0 8px;color:#e9d5ff;font-size:14px;", "Selected agent profile" }
                        input {
                            style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;",
                            placeholder: "Name", value: "{edit_name}",
                            oninput: move |e| { let mut value = edit_name; value.set(e.value()); }
                        }
                        input {
                            style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;",
                            placeholder: "Role / purpose", value: "{edit_role}",
                            oninput: move |e| { let mut value = edit_role; value.set(e.value()); }
                        }
                        input {
                            style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;",
                            placeholder: "Local model ID (blank = default)", value: "{edit_model}",
                            oninput: move |e| { let mut value = edit_model; value.set(e.value()); }
                        }
                        textarea {
                            style: "width:100%;min-height:72px;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;font-family:inherit;",
                            placeholder: "Instructions / characteristics", value: "{edit_instructions}",
                            oninput: move |e| { let mut value = edit_instructions; value.set(e.value()); }
                        }
                        label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Allowed ontology/data source IDs (comma-separated; blank = session-permitted sources)" }
                        input {
                            style: "width:100%;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;",
                            placeholder: "e.g. wordnet, australian-history", value: "{edit_allowed_ontologies}",
                            oninput: move |e| { let mut value = edit_allowed_ontologies; value.set(e.value()); }
                        }
                        p { style: "margin:-3px 0 9px;color:#94a3b8;font-size:11px;line-height:1.4;", "This is an enforceable narrowing boundary. It intersects the chat's permitted scopes; it never grants an extra dataset." }
                        label { style: "display:block;margin:0 0 5px;color:#cbd5e1;font-size:12px;", "Semantic study graph" }
                        textarea {
                            style: "width:100%;min-height:110px;box-sizing:border-box;padding:8px 10px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;margin-bottom:8px;font-family:ui-monospace,monospace;font-size:12px;",
                            placeholder: "facet | label | IRI | optional broader IRI", value: "{edit_semantic_draft}",
                            oninput: move |e| { let mut value = edit_semantic_draft; value.set(e.value()); }
                        }
                        label { style: "display:flex;align-items:center;gap:8px;color:#cbd5e1;font-size:12px;margin:0 0 10px;",
                            input {
                                r#type: "checkbox", checked: edit_enabled(),
                                onchange: move |e| { let mut value = edit_enabled; value.set(e.checked()); }
                            }
                            "Enabled (disable to archive without deleting history)"
                        }
                        button {
                            style: "background:#8b5cf6;color:white;padding:8px 14px;border:none;border-radius:8px;font-weight:600;cursor:pointer;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (selected_slug, edit_name, edit_role, edit_model, edit_instructions, edit_enabled, edit_semantic_draft, edit_allowed_ontologies, mut agents, mut status) =
                                        (selected_slug, edit_name, edit_role, edit_model, edit_instructions, edit_enabled, edit_semantic_draft, edit_allowed_ontologies, agents, status);
                                    spawn(async move {
                                        let slug = selected_slug();
                                        match invoke_json::<serde_json::Value>("agent_roster_get", json!({ "slug": slug })).await {
                                            Ok(mut agent) if agent.is_object() => {
                                                let name = edit_name().trim().to_string();
                                                if name.is_empty() { status.set("Agent name is required.".into()); return; }
                                                let model = edit_model().trim().to_string();
                                                let tags = match semantic_tags_from_draft(&edit_semantic_draft()) {
                                                    Ok(tags) => tags,
                                                    Err(e) => { status.set(e); return; }
                                                };
                                                let ontology_ids: Vec<String> = edit_allowed_ontologies().split(',').map(str::trim).filter(|value| !value.is_empty()).map(str::to_string).collect();
                                                if let Some(object) = agent.as_object_mut() {
                                                    object.insert("display_name".into(), json!(name));
                                                    object.insert("description".into(), json!(edit_role()));
                                                    object.insert("system_prompt".into(), json!(edit_instructions()));
                                                    object.insert("enabled".into(), json!(edit_enabled()));
                                                    object.insert("semantic_profile".into(), json!({ "tags": tags }));
                                                    object.insert("data_policy".into(), json!({ "allowed_ontology_ids": ontology_ids }));
                                                    if let Some(local) = object.get_mut("backend").and_then(|backend| backend.get_mut("local_engine")).and_then(|local| local.as_object_mut()) {
                                                        local.insert("model_id".into(), if model.is_empty() { serde_json::Value::Null } else { json!(model) });
                                                    }
                                                }
                                                match invoke_json::<()>("agent_roster_upsert", json!({ "agentJson": agent.to_string() })).await {
                                                    Ok(()) => match invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({})).await {
                                                        Ok(list) => { agents.set(list); status.set("Agent profile saved.".into()); }
                                                        Err(e) => status.set(format!("Agent saved, but refresh failed: {e}")),
                                                    },
                                                    Err(e) => status.set(format!("Could not save agent: {e}")),
                                                }
                                            }
                                            Ok(_) => status.set("Selected agent no longer exists.".into()),
                                            Err(e) => status.set(format!("Could not load agent: {e}")),
                                        }
                                    });
                                }
                            },
                            "Save profile"
                        }
                    }

                    label { style: "display:flex; align-items:center; gap:10px; margin-bottom:12px; color:#e5e7eb; font-size:13px;",
                        input {
                            r#type: "checkbox",
                            checked: wildcard(),
                            onchange: move |e| {
                                let mut wildcard = wildcard;
                                let mut allowed = allowed;
                                let on = e.checked();
                                wildcard.set(on);
                                if on {
                                    allowed.set(vec!["*".to_string()]);
                                } else {
                                    allowed.set(Vec::new());
                                }
                            },
                        }
                        span { "Allow all tools (" }
                        code { "*" }
                        span { ") — use sparingly" }
                    }

                    h3 { style: "margin:0 0 10px; color:#e5e7eb; font-size:14px;", "Tools" }
                    if tools().is_empty() {
                        p { style: "color:#64748b; font-size:12px;",
                            "No tools loaded (host not ready or tools/list failed). Safe seeds: list_capabilities, computer_vision."
                        }
                    }
                    div { style: "display:flex; flex-direction:column; gap:6px; max-height:360px; overflow-y:auto;",
                        for t in tools() {
                            {
                                let name = tool_name_of(&t);
                                let desc = t
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .unwrap_or("")
                                    .chars()
                                    .take(120)
                                    .collect::<String>();
                                let checked = is_checked(&name);
                                let name_for_cb = name.clone();
                                rsx! {
                                    label {
                                        style: "display:flex; align-items:flex-start; gap:10px; padding:8px; background:#0b1220; border-radius:8px; cursor:pointer;",
                                        input {
                                            r#type: "checkbox",
                                            checked: checked,
                                            disabled: wildcard(),
                                            onchange: move |e| {
                                                if wildcard() {
                                                    return;
                                                }
                                                let on = e.checked();
                                                let mut allowed = allowed;
                                                let mut list = allowed();
                                                list.retain(|x| x != &name_for_cb && x != "*");
                                                if on {
                                                    list.push(name_for_cb.clone());
                                                }
                                                allowed.set(list);
                                            },
                                        }
                                        div {
                                            div { style: "font-weight:600; font-size:12px; color:#f3f4f6;", "{name}" }
                                            if !desc.is_empty() {
                                                div { style: "font-size:11px; color:#94a3b8; margin-top:2px;", "{desc}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div { style: "display:flex; flex-wrap:wrap; gap:8px; margin-top:16px;",
                        button {
                            style: "background:#8b5cf6; color:white; padding:10px 18px; border:none; border-radius:8px; font-weight:600; cursor:pointer;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let selected_slug = selected_slug;
                                    let allowed = allowed;
                                    let mut status = status;
                                    spawn(async move {
                                        let slug = selected_slug();
                                        let tools_list = allowed();
                                        match invoke_json::<()>(
                                            "agent_set_allowed_mcp_tools",
                                            json!({ "slug": slug, "tools": tools_list }),
                                        )
                                        .await
                                        {
                                            Ok(()) => status.set("Allowlist saved.".into()),
                                            Err(e) => status.set(format!("Save failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Save allowlist"
                        }
                        button {
                            style: "background:#334155; color:#e5e7eb; padding:10px 18px; border:none; border-radius:8px; font-weight:600; cursor:pointer;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let selected_slug = selected_slug;
                                    let mut allowed = allowed;
                                    let mut wildcard = wildcard;
                                    let mut status = status;
                                    spawn(async move {
                                        let slug = selected_slug();
                                        match invoke_json::<serde_json::Value>(
                                            "mcp_ensure_safe_tool_allowlist",
                                            json!({ "slug": slug }),
                                        )
                                        .await
                                        {
                                            Ok(v) => {
                                                if let Some(arr) = v.as_array() {
                                                    let list: Vec<String> = arr
                                                        .iter()
                                                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                                        .collect();
                                                    wildcard.set(list.iter().any(|t| t == "*"));
                                                    allowed.set(list);
                                                }
                                                status.set(
                                                    "Seeded safe tools (list_capabilities, computer_vision) if empty."
                                                        .into(),
                                                );
                                            }
                                            Err(e) => status.set(format!("Seed failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Seed safe tools if empty"
                        }
                    }
                }

                p { style: "color:#64748b; font-size:11px; line-height:1.4;",
                    "Relations → Chat has a Propose / Permit / Deny card that uses this allowlist. Runtime dogfood still required on desktop."
                }
            }
        }
    }
}
