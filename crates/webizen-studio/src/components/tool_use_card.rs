//! Principal-gated MCP tool propose card (U3-A).
//!
//! Manual dogfood path without model: pick tool → args → **Propose** →
//! **Permit** / **Deny**. Deny never invokes MCP. Permit calls
//! `mcp_call_tool_gated` with `principal_permitted = true` after allowlist check.

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

const CARD: &str = "background:#111827; border:1px solid #1f2937; border-radius:10px; padding:12px; margin-bottom:12px;";
const H3: &str = "margin:0 0 8px; color:#94a3b8; font-size:11px; text-transform:uppercase; letter-spacing:0.6px;";
const INPUT: &str = "width:100%; box-sizing:border-box; padding:8px 10px; margin-bottom:8px; background:#0b1220; color:#f3f4f6; border:1px solid #334155; border-radius:8px; font-family:inherit; font-size:12px;";
const BTN: &str = "background:#8b5cf6; color:white; padding:7px 12px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:12px; margin-right:6px;";
const BTN2: &str = "background:#334155; color:#e5e7eb; padding:7px 12px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:12px; margin-right:6px;";
const BTN_OK: &str = "background:#059669; color:white; padding:7px 12px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:12px; margin-right:6px;";
const BTN_DENY: &str = "background:#7f1d1d; color:#fecaca; padding:7px 12px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:12px;";

fn default_args_for(tool: &str) -> &'static str {
    if tool == "computer_vision" {
        r#"{"op":"list"}"#
    } else {
        "{}"
    }
}

fn tool_name_of(v: &serde_json::Value) -> String {
    v.get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string()
}

/// Sidebar / thread card: propose local MCP tools under principal Permit/Deny.
#[component]
pub fn ToolUseCard(agent_slug: String) -> Element {
    let tools = use_signal(Vec::<serde_json::Value>::new);
    let allowlist = use_signal(Vec::<String>::new);
    let selected = use_signal(|| "list_capabilities".to_string());
    let args_json = use_signal(|| "{}".to_string());
    let proposed = use_signal(|| false);
    let result_text = use_signal(String::new);
    let status = use_signal(String::new);
    let busy = use_signal(|| false);
    let agent_slug_sig = use_signal(|| agent_slug.clone());

    // Keep slug in sync when parent changes.
    {
        let mut agent_slug_sig = agent_slug_sig;
        let slug = agent_slug.clone();
        use_effect(move || {
            agent_slug_sig.set(slug.clone());
        });
    }

    #[cfg(target_arch = "wasm32")]
    use_hook(|| {
        let mut tools = tools;
        let mut allowlist = allowlist;
        let mut selected = selected;
        let mut args_json = args_json;
        let mut status = status;
        let agent_slug_sig = agent_slug_sig;
        spawn(async move {
            // Seed empty allowlist with safe golden tools on first open (not a Permit).
            let slug = {
                let s = agent_slug_sig();
                if s.is_empty() {
                    "local".to_string()
                } else {
                    s
                }
            };
            match invoke_json::<serde_json::Value>(
                "mcp_ensure_safe_tool_allowlist",
                json!({ "slug": slug.clone() }),
            )
            .await
            {
                Ok(v) => {
                    if let Some(arr) = v.as_array() {
                        let list: Vec<String> = arr
                            .iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect();
                        allowlist.set(list);
                    }
                }
                Err(e) => status.set(format!("Allowlist seed: {e}")),
            }
            match invoke_json::<Vec<serde_json::Value>>("mcp_list_local_tools", json!({})).await {
                Ok(list) => {
                    if selected().is_empty() {
                        if let Some(first) = list.first() {
                            let n = tool_name_of(first);
                            selected.set(n.clone());
                            args_json.set(default_args_for(&n).to_string());
                        }
                    }
                    tools.set(list);
                }
                Err(e) => status.set(format!("tools/list failed: {e}")),
            }
            // Refresh allowlist from roster agent
            if let Ok(agent) =
                invoke_json::<serde_json::Value>("agent_roster_get", json!({ "slug": slug })).await
            {
                if let Some(arr) = agent.get("allowed_mcp_tools").and_then(|a| a.as_array()) {
                    let list: Vec<String> = arr
                        .iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect();
                    if !list.is_empty() {
                        allowlist.set(list);
                    }
                }
            }
        });
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &tools,
            &allowlist,
            &selected,
            &args_json,
            &proposed,
            &result_text,
            &status,
            &busy,
            &agent_slug_sig,
        );
    }

    let al = allowlist();
    let al_label = if al.is_empty() {
        "(empty — deny-all until seed/save)".to_string()
    } else {
        al.join(", ")
    };

    rsx! {
        div { style: "{CARD}",
            div { style: "display:flex; justify-content:space-between; align-items:center; gap:8px; margin-bottom:8px;",
                h3 { style: "{H3} margin:0;", "MCP tools (principal-gated)" }
                HonestyChip {
                    level: HonestyLevel::Partial,
                    detail: "Permit required · allowlist enforced".to_string(),
                }
            }
            p { style: "color:#94a3b8; font-size:11px; margin:0 0 8px; line-height:1.4;",
                "Propose a local tool → Permit or Deny. Deny never runs the tool. Agent: "
                span { style: "color:#e5e7eb; font-weight:600;",
                    if agent_slug_sig().is_empty() { "local" } else { "{agent_slug_sig}" }
                }
            }
            p { style: "color:#64748b; font-size:10px; margin:0 0 8px;", "Allowlist: {al_label}" }

            if !status().is_empty() {
                div { style: "font-size:11px; color:#a7f3d0; margin-bottom:8px; white-space:pre-wrap;", "{status}" }
            }

            label { style: "display:block; font-size:11px; color:#94a3b8; margin-bottom:4px;", "Tool" }
            select {
                style: "{INPUT}",
                value: "{selected}",
                onchange: move |e| {
                    let v = e.value();
                    let mut selected = selected;
                    let mut args_json = args_json;
                    let mut proposed = proposed;
                    selected.set(v.clone());
                    args_json.set(default_args_for(&v).to_string());
                    proposed.set(false);
                },
                if tools().is_empty() {
                    option { value: "list_capabilities", "list_capabilities" }
                    option { value: "computer_vision", "computer_vision" }
                }
                for t in tools() {
                    {
                        let n = tool_name_of(&t);
                        rsx! { option { value: "{n}", "{n}" } }
                    }
                }
            }

            label { style: "display:block; font-size:11px; color:#94a3b8; margin-bottom:4px;", "Arguments (JSON)" }
            textarea {
                style: "{INPUT} height:56px; resize:vertical; font-family:ui-monospace,Consolas,monospace;",
                value: "{args_json}",
                oninput: move |e| {
                    let mut args_json = args_json;
                    args_json.set(e.value());
                },
            }

            div { style: "display:flex; flex-wrap:wrap; gap:4px; margin-bottom:8px;",
                button {
                    style: "{BTN}",
                    disabled: busy(),
                    onclick: move |_| {
                        let mut proposed = proposed;
                        let mut result_text = result_text;
                        let mut status = status;
                        proposed.set(true);
                        result_text.set(String::new());
                        status.set(format!(
                            "Proposed {} — Permit to run, Deny to cancel.",
                            selected()
                        ));
                    },
                    "Propose tool"
                }
                button {
                    style: "{BTN2}",
                    onclick: move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let mut tools = tools;
                            let mut allowlist = allowlist;
                            let mut status = status;
                            let agent_slug_sig = agent_slug_sig;
                            spawn(async move {
                                let slug = {
                                    let s = agent_slug_sig();
                                    if s.is_empty() { "local".into() } else { s }
                                };
                                if let Ok(list) =
                                    invoke_json::<Vec<serde_json::Value>>("mcp_list_local_tools", json!({}))
                                        .await
                                {
                                    tools.set(list);
                                }
                                if let Ok(agent) = invoke_json::<serde_json::Value>(
                                    "agent_roster_get",
                                    json!({ "slug": slug }),
                                )
                                .await
                                {
                                    if let Some(arr) =
                                        agent.get("allowed_mcp_tools").and_then(|a| a.as_array())
                                    {
                                        allowlist.set(
                                            arr.iter()
                                                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                                                .collect(),
                                        );
                                    }
                                }
                                status.set("Tools / allowlist refreshed.".into());
                            });
                        }
                    },
                    "Refresh"
                }
            }

            if proposed() {
                div { style: "background:#0b1220; border:1px solid #4c1d95; border-radius:8px; padding:10px; margin-bottom:8px;",
                    div { style: "font-size:12px; color:#e9d5ff; font-weight:600; margin-bottom:4px;",
                        "Awaiting principal: {selected}"
                    }
                    pre { style: "margin:0 0 8px; font-size:11px; color:#94a3b8; white-space:pre-wrap; max-height:80px; overflow:auto;",
                        "{args_json}"
                    }
                    button {
                        style: "{BTN_OK}",
                        disabled: busy(),
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let selected = selected;
                                let args_json = args_json;
                                let agent_slug_sig = agent_slug_sig;
                                let mut result_text = result_text;
                                let mut status = status;
                                let mut busy = busy;
                                let mut proposed = proposed;
                                spawn(async move {
                                    busy.set(true);
                                    status.set("Permitted — calling MCP…".into());
                                    let slug = {
                                        let s = agent_slug_sig();
                                        if s.is_empty() { "local".into() } else { s }
                                    };
                                    let args = json!({
                                        "agentSlug": slug,
                                        "toolName": selected(),
                                        "argumentsJson": args_json(),
                                        "principalPermitted": true,
                                    });
                                    match invoke_json::<String>("mcp_call_tool_gated", args).await {
                                        Ok(text) => {
                                            result_text.set(text);
                                            status.set("Permit OK — result below.".into());
                                            proposed.set(false);
                                        }
                                        Err(e) => {
                                            result_text.set(String::new());
                                            status.set(format!("Permit blocked/failed: {e}"));
                                        }
                                    }
                                    busy.set(false);
                                });
                            }
                        },
                        "Permit"
                    }
                    button {
                        style: "{BTN_DENY}",
                        disabled: busy(),
                        onclick: move |_| {
                            // Deny: never invoke MCP (no gated call with false either —
                            // surface denial only).
                            let mut proposed = proposed;
                            let mut result_text = result_text;
                            let mut status = status;
                            proposed.set(false);
                            result_text.set(String::new());
                            status.set(format!(
                                "Denied by principal — {} was not invoked.",
                                selected()
                            ));
                        },
                        "Deny"
                    }
                }
            }

            if !result_text().is_empty() {
                div { style: "background:#064e3b; border:1px solid #10b981; border-radius:8px; padding:10px; margin-top:4px;",
                    div { style: "font-size:11px; color:#a7f3d0; font-weight:600; margin-bottom:4px;",
                        "Tool result"
                    }
                    pre { style: "margin:0; font-size:11px; color:#ecfdf5; white-space:pre-wrap; max-height:200px; overflow:auto;",
                        "{result_text}"
                    }
                }
            }
        }
    }
}
