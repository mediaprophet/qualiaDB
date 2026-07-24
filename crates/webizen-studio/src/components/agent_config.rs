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

#[component]
pub fn AgentConfig() -> Element {
    let agents = use_signal(Vec::<serde_json::Value>::new);
    let tools = use_signal(Vec::<serde_json::Value>::new);
    let selected_slug = use_signal(|| "local".to_string());
    let allowed = use_signal(Vec::<String>::new);
    let status = use_signal(String::new);
    let wildcard = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    use_hook(|| {
        let mut agents = agents;
        let mut tools = tools;
        let mut selected_slug = selected_slug;
        let mut allowed = allowed;
        let mut wildcard = wildcard;
        let mut status = status;
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
                if let Some(arr) = agent.get("allowed_mcp_tools").and_then(|a| a.as_array()) {
                    let list: Vec<String> = arr
                        .iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect();
                    wildcard.set(list.iter().any(|t| t == "*"));
                    allowed.set(list);
                }
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
            &wildcard,
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
                    "Tools the selected agent may use after an explicit principal "
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
                                let mut status = status;
                                spawn(async move {
                                    match invoke_json::<serde_json::Value>(
                                        "agent_roster_get",
                                        json!({ "slug": slug }),
                                    )
                                    .await
                                    {
                                        Ok(agent) => {
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
                    "Talk → Chat has a Propose / Permit / Deny card that uses this allowlist. Runtime dogfood still required on desktop."
                }
            }
        }
    }
}
