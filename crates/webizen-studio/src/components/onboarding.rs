//! Persisted first-run gate for the desktop application.

#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use futures_util::future::{select, Either};
#[cfg(target_arch = "wasm32")]
use futures_util::pin_mut;
#[cfg(target_arch = "wasm32")]
use gloo_timers::future::TimeoutFuture;
#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::*, JsCast};

#[cfg(target_arch = "wasm32")]
async fn invoke_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let request = crate::components::qapp_engine::invoke_json(cmd, args);
    let timeout = TimeoutFuture::new(8_000);
    pin_mut!(request);
    pin_mut!(timeout);
    match select(request, timeout).await {
        Either::Left((result, _)) => result.and_then(|value| {
            serde_json::from_value(value).map_err(|error| format!("decode {cmd} response: {error}"))
        }),
        Either::Right((_, _)) => Err(format!("desktop command '{cmd}' timed out after 8 seconds")),
    }
}

#[cfg(target_arch = "wasm32")]
fn has_tauri_bridge() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let Ok(tauri) = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("__TAURI__")) else {
        return false;
    };
    let Ok(core) = js_sys::Reflect::get(&tauri, &JsValue::from_str("core")) else {
        return false;
    };
    let Ok(invoke) = js_sys::Reflect::get(&core, &JsValue::from_str("invoke")) else {
        return false;
    };
    invoke.dyn_ref::<js_sys::Function>().is_some()
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SetupStateSnapshot {
    completed: bool,
    current_step: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AgentConfigSnapshot {
    storage_path: String,
    storage_quota_gb: u64,
    base_connectivity_cost_ilp: u64,
    daemon_host: String,
    daemon_port: u16,
    inference_backend: String,
    settings_port: u16,
}

impl Default for AgentConfigSnapshot {
    fn default() -> Self {
        Self {
            storage_path: String::new(),
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5_000,
            daemon_host: "127.0.0.1".to_string(),
            daemon_port: 4_242,
            inference_backend: "local".to_string(),
            settings_port: 8_080,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn step_index(step: &str) -> u8 {
    match step {
        "storage" => 1,
        "inference" => 2,
        "ready" => 3,
        _ => 0,
    }
}

#[component]
pub fn OnboardingGate() -> Element {
    let mut loading = use_signal(|| true);
    let mut complete = use_signal(|| false);
    // Mutated only on the wasm/Tauri host path (setup steps).
    #[cfg_attr(not(target_arch = "wasm32"), allow(unused_mut))]
    let mut step = use_signal(|| 0_u8);
    let mut config = use_signal(AgentConfigSnapshot::default);
    let mut status = use_signal(String::new);
    let mut saving = use_signal(|| false);

    #[cfg(target_arch = "wasm32")]
    use_hook(move || {
        if !has_tauri_bridge() {
            complete.set(true);
            loading.set(false);
            return;
        }
        spawn(async move {
            match invoke_json::<SetupStateSnapshot>("get_setup_state", json!({})).await {
                Ok(setup) => {
                    complete.set(setup.completed);
                    step.set(step_index(&setup.current_step));
                    if !setup.completed {
                        match invoke_json::<AgentConfigSnapshot>("get_config", json!({})).await {
                            Ok(value) => config.set(value),
                            Err(error) => {
                                status.set(format!("Could not load local settings: {error}"))
                            }
                        }
                    }
                }
                Err(_) => {
                    // The hosted web preview has no Tauri bridge and should remain browsable.
                    complete.set(true);
                }
            }
            loading.set(false);
        });
    });

    #[cfg(not(target_arch = "wasm32"))]
    use_hook(move || {
        complete.set(true);
        loading.set(false);
    });

    if loading() {
        return rsx! {
            div {
                style: "width:100%;height:100%;display:grid;place-items:center;background:#07101f;color:#e5edf8;",
                div { style: "display:grid;gap:12px;text-align:center;",
                    div { style: "font-size:2rem;font-weight:800;letter-spacing:-0.04em;", "Webizen" }
                    div { style: "color:#94a3b8;font-size:0.8rem;", "Preparing your local workspace…" }
                }
            }
        };
    }

    if complete() {
        return rsx! { Router::<crate::Route> {} };
    }

    let progress = ((step() as f32 + 1.0) / 4.0) * 100.0;
    let cfg = config();

    rsx! {
        div {
            style: "width:100%;height:100%;overflow:auto;background:radial-gradient(circle at 20% 10%,rgba(56,189,248,.13),transparent 38%),radial-gradient(circle at 90% 80%,rgba(167,139,250,.12),transparent 35%),#07101f;color:#e5edf8;padding:32px;",
            div {
                style: "width:min(760px,100%);margin:0 auto;min-height:calc(100vh - 64px);display:flex;flex-direction:column;justify-content:center;",
                div { style: "display:flex;align-items:center;justify-content:space-between;gap:16px;margin-bottom:16px;",
                    div {
                        div { style: "font-size:1.55rem;font-weight:820;letter-spacing:-.035em;", "Welcome to Webizen" }
                        div { style: "color:#94a3b8;font-size:.78rem;margin-top:4px;", "Private by default · local-first · you remain in control" }
                    }
                    div { style: "color:#7dd3fc;font-size:.72rem;font-weight:700;", "STEP {step() + 1} OF 4" }
                }
                div { style: "height:5px;background:rgba(148,163,184,.14);border-radius:999px;overflow:hidden;margin-bottom:22px;",
                    div { style: "height:100%;width:{progress}%;background:linear-gradient(90deg,#38bdf8,#a78bfa);transition:width .25s ease;" }
                }

                div {
                    style: "background:rgba(15,23,42,.84);border:1px solid rgba(148,163,184,.18);border-radius:22px;padding:clamp(22px,5vw,40px);box-shadow:0 28px 80px rgba(0,0,0,.34);backdrop-filter:blur(22px);",
                    if step() == 0 {
                        div {
                            div { style: "width:46px;height:46px;border-radius:14px;display:grid;place-items:center;background:rgba(56,189,248,.14);color:#7dd3fc;font-size:1.4rem;margin-bottom:18px;", "◈" }
                            h1 { style: "font-size:1.7rem;margin:0 0 10px;letter-spacing:-.03em;", "Your computer, your data" }
                            p { style: "color:#b6c2d3;line-height:1.65;margin:0 0 20px;",
                                "Webizen brings chat, local AI, browsing, people, projects and semantic mail into one desktop. Your local workspace stays on this device unless you deliberately connect or share it."
                            }
                            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(170px,1fr));gap:10px;margin-bottom:26px;",
                                for (title, body) in [
                                    ("Local-first", "Chats and indexes use your chosen storage."),
                                    ("Explicit sharing", "Network and social actions remain visible."),
                                    ("Changeable", "Every choice can be revised in Settings."),
                                ] {
                                    div { style: "padding:14px;border:1px solid rgba(148,163,184,.16);border-radius:13px;background:rgba(255,255,255,.025);",
                                        div { style: "font-weight:700;font-size:.8rem;margin-bottom:5px;", "{title}" }
                                        div { style: "color:#94a3b8;font-size:.72rem;line-height:1.45;", "{body}" }
                                    }
                                }
                            }
                            button {
                                style: "width:100%;border:0;border-radius:12px;padding:12px 16px;background:linear-gradient(135deg,#38bdf8,#818cf8);color:#06111f;font-weight:800;cursor:pointer;",
                                onclick: move |_| {
                                    saving.set(true);
                                    status.set(String::new());
                                    #[cfg(target_arch = "wasm32")]
                                    spawn(async move {
                                        match invoke_json::<SetupStateSnapshot>("complete_setup_step", json!({"step":"welcome"})).await {
                                            Ok(_) => step.set(1),
                                            Err(error) => status.set(error),
                                        }
                                        saving.set(false);
                                    });
                                },
                                disabled: saving(),
                                "Continue"
                            }
                        }
                    } else if step() == 1 {
                        div {
                            h1 { style: "font-size:1.55rem;margin:0 0 8px;letter-spacing:-.025em;", "Choose local storage" }
                            p { style: "color:#94a3b8;line-height:1.55;margin:0 0 22px;font-size:.82rem;",
                                "Models, chats, indexes and QApps will live here. Webizen preserves a 15 GB operating-system safety margin."
                            }
                            label { style: "display:block;font-size:.76rem;font-weight:700;margin-bottom:7px;", "Workspace folder" }
                            input {
                                value: "{cfg.storage_path}",
                                placeholder: "C:\\Users\\you\\AppData\\Local\\QualiaData",
                                style: "width:100%;padding:12px;border-radius:11px;border:1px solid rgba(148,163,184,.25);background:#091323;color:#e5edf8;margin-bottom:16px;",
                                oninput: move |event| {
                                    let mut next = config();
                                    next.storage_path = event.value();
                                    config.set(next);
                                }
                            }
                            label { style: "display:block;font-size:.76rem;font-weight:700;margin-bottom:7px;", "Storage allowance (GB)" }
                            input {
                                r#type: "number",
                                min: "1",
                                max: "4096",
                                value: "{cfg.storage_quota_gb}",
                                style: "width:100%;padding:12px;border-radius:11px;border:1px solid rgba(148,163,184,.25);background:#091323;color:#e5edf8;margin-bottom:22px;",
                                oninput: move |event| {
                                    if let Ok(value) = event.value().parse::<u64>() {
                                        let mut next = config();
                                        next.storage_quota_gb = value.max(1);
                                        config.set(next);
                                    }
                                }
                            }
                            button {
                                style: "width:100%;border:0;border-radius:12px;padding:12px 16px;background:linear-gradient(135deg,#38bdf8,#818cf8);color:#06111f;font-weight:800;cursor:pointer;",
                                disabled: saving() || cfg.storage_path.trim().is_empty(),
                                onclick: move |_| {
                                    saving.set(true);
                                    status.set(String::new());
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let snapshot = config();
                                        spawn(async move {
                                            let result = match invoke_json::<()>(
                                                "save_config",
                                                json!({"newConfig":snapshot}),
                                            )
                                            .await
                                            {
                                                Ok(_) => invoke_json::<SetupStateSnapshot>(
                                                    "complete_setup_step",
                                                    json!({"step":"storage"}),
                                                )
                                                .await,
                                                Err(error) => Err(error),
                                            };
                                            match result {
                                                Ok(_) => step.set(2),
                                                Err(error) => status.set(error),
                                            }
                                            saving.set(false);
                                        });
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        saving.set(false);
                                    }
                                },
                                if saving() { "Saving…" } else { "Save storage choice" }
                            }
                        }
                    } else if step() == 2 {
                        div {
                            h1 { style: "font-size:1.55rem;margin:0 0 8px;letter-spacing:-.025em;", "Choose how AI runs" }
                            p { style: "color:#94a3b8;line-height:1.55;margin:0 0 20px;font-size:.82rem;",
                                "Local is the private default. Ollama uses an Ollama service on this computer. Hybrid keeps local as primary and allows configured remote providers later."
                            }
                            div { style: "display:grid;gap:10px;margin-bottom:22px;",
                                for (value, title, body) in [
                                    ("local", "Local model", "GGUF models run through Webizen’s local inference path."),
                                    ("ollama", "Ollama", "Use an existing local Ollama installation."),
                                    ("hybrid", "Hybrid", "Prefer local and enable explicitly configured providers."),
                                ] {
                                    label {
                                        style: if cfg.inference_backend == value { "display:flex;gap:12px;padding:14px;border:1px solid #38bdf8;border-radius:13px;background:rgba(56,189,248,.08);cursor:pointer;" } else { "display:flex;gap:12px;padding:14px;border:1px solid rgba(148,163,184,.17);border-radius:13px;background:rgba(255,255,255,.02);cursor:pointer;" },
                                        input {
                                            r#type: "radio",
                                            name: "inference-backend",
                                            value,
                                            checked: cfg.inference_backend == value,
                                            onchange: move |_| {
                                                let mut next = config();
                                                next.inference_backend = value.to_string();
                                                config.set(next);
                                            }
                                        }
                                        div {
                                            div { style: "font-size:.82rem;font-weight:750;", "{title}" }
                                            div { style: "font-size:.71rem;color:#94a3b8;margin-top:3px;line-height:1.4;", "{body}" }
                                        }
                                    }
                                }
                            }
                            button {
                                style: "width:100%;border:0;border-radius:12px;padding:12px 16px;background:linear-gradient(135deg,#38bdf8,#818cf8);color:#06111f;font-weight:800;cursor:pointer;",
                                disabled: saving(),
                                onclick: move |_| {
                                    saving.set(true);
                                    status.set(String::new());
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let snapshot = config();
                                        spawn(async move {
                                            let result = match invoke_json::<()>(
                                                "save_config",
                                                json!({"newConfig":snapshot}),
                                            )
                                            .await
                                            {
                                                Ok(_) => invoke_json::<SetupStateSnapshot>(
                                                    "complete_setup_step",
                                                    json!({"step":"inference"}),
                                                )
                                                .await,
                                                Err(error) => Err(error),
                                            };
                                            match result {
                                                Ok(_) => step.set(3),
                                                Err(error) => status.set(error),
                                            }
                                            saving.set(false);
                                        });
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        saving.set(false);
                                    }
                                },
                                if saving() { "Saving…" } else { "Save AI choice" }
                            }
                        }
                    } else {
                        div { style: "text-align:center;",
                            div { style: "width:58px;height:58px;margin:0 auto 18px;border-radius:18px;display:grid;place-items:center;background:rgba(52,211,153,.14);color:#6ee7b7;font-size:1.7rem;", "✓" }
                            h1 { style: "font-size:1.65rem;margin:0 0 10px;letter-spacing:-.03em;", "Your workspace is ready" }
                            p { style: "color:#aebccd;line-height:1.6;margin:0 auto 22px;max-width:520px;",
                                "Relations opens first — Chat, People, Reception, Mail and Projects together. Memory is home for lived records. Add an identity or mail domain when you’re ready; neither is required to start."
                            }
                            div { style: "display:flex;flex-wrap:wrap;gap:8px;justify-content:center;margin-bottom:24px;",
                                for feature in ["Chat + local LLM", "Desktop browser", "Social directory", "Semantic mail", "QApp Studio"] {
                                    span { style: "padding:7px 10px;border-radius:999px;background:rgba(148,163,184,.1);color:#cbd5e1;font-size:.7rem;", "{feature}" }
                                }
                            }
                            button {
                                style: "width:100%;border:0;border-radius:12px;padding:12px 16px;background:linear-gradient(135deg,#34d399,#38bdf8);color:#06111f;font-weight:850;cursor:pointer;",
                                disabled: saving(),
                                onclick: move |_| {
                                    saving.set(true);
                                    status.set(String::new());
                                    #[cfg(target_arch = "wasm32")]
                                    spawn(async move {
                                        match invoke_json::<SetupStateSnapshot>("finish_setup", json!({})).await {
                                            Ok(_) => complete.set(true),
                                            Err(error) => status.set(error),
                                        }
                                        saving.set(false);
                                    });
                                },
                                if saving() { "Finishing…" } else { "Open Webizen" }
                            }
                        }
                    }

                    if !status().is_empty() {
                        div { style: "margin-top:14px;padding:11px 13px;border-radius:10px;border:1px solid rgba(248,113,113,.35);background:rgba(127,29,29,.2);color:#fecaca;font-size:.72rem;line-height:1.45;",
                            "{status}"
                        }
                    }
                }
                div { style: "text-align:center;color:#64748b;font-size:.67rem;margin-top:14px;",
                    "No cloud account is required. Settings remain editable after setup."
                }
            }
        }
    }
}
