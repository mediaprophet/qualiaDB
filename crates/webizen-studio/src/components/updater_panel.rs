use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const PANEL_STYLE: &str = "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 18px; padding: 1.5rem; backdrop-filter: blur(22px); box-shadow: 0 10px 32px rgba(0,0,0,0.08); margin-bottom: 1.5rem; display: flex; flex-direction: column; gap: 1rem;";
const TITLE_STYLE: &str = "font-size: 1.1rem; font-weight: 700; color: var(--qualia-text); margin: 0;";
const DESC_STYLE: &str = "font-size: 0.85rem; color: var(--qualia-text-muted); line-height: 1.5; margin: 0;";
const PROGRESS_CONTAINER: &str = "width: 100%; height: 6px; background: rgba(128,128,128,0.2); border-radius: 3px; overflow: hidden; margin-top: 0.5rem;";
const PROGRESS_BAR: &str = "height: 100%; background: var(--qualia-brand); transition: width 0.2s ease;";

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(
        event: &str,
        handler: &Closure<dyn FnMut(js_sys::Object)>,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_tauri_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
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

#[derive(Clone, Debug, PartialEq)]
enum UpdaterState {
    Idle,
    Checking,
    Available(String),
    Downloading { downloaded: u64, total: u64 },
    ReadyToRestart,
    Error(String),
}

#[derive(Deserialize, Debug)]
struct ProgressPayload {
    downloaded: u64,
    total: u64,
}

#[derive(Deserialize)]
struct TauriEventPayload {
    payload: ProgressPayload,
}

#[component]
pub fn UpdaterPanel() -> Element {
    let mut state = use_signal(|| UpdaterState::Idle);

    #[cfg(target_arch = "wasm32")]
    let check_update = move |_| {
        state.set(UpdaterState::Checking);
        spawn(async move {
            let res = invoke_tauri_json::<Option<String>>("updater_check", serde_json::json!({})).await;
            match res {
                Ok(Some(version)) => state.set(UpdaterState::Available(version)),
                Ok(None) => state.set(UpdaterState::Idle),
                Err(e) => state.set(UpdaterState::Error(e)),
            }
        });
    };

    #[cfg(target_arch = "wasm32")]
    let download_update = move |_| {
        state.set(UpdaterState::Downloading { downloaded: 0, total: 100 });
        
        spawn(async move {
            // Setup listener
            let cb = Closure::wrap(Box::new(move |event_val: js_sys::Object| {
                if let Ok(event) = serde_wasm_bindgen::from_value::<TauriEventPayload>(event_val.into()) {
                    state.set(UpdaterState::Downloading {
                        downloaded: event.payload.downloaded,
                        total: event.payload.total,
                    });
                }
            }) as Box<dyn FnMut(js_sys::Object)>);
            
            let _ = tauri_listen("updater-progress", &cb).await;
            cb.forget(); // Leak closure so it stays alive during download

            let res = invoke_tauri_json::<()>("updater_download_and_install", serde_json::json!({})).await;
            if let Err(e) = res {
                state.set(UpdaterState::Error(e));
            } else {
                state.set(UpdaterState::ReadyToRestart);
            }
        });
    };

    #[cfg(target_arch = "wasm32")]
    let restart_app = move |_| {
        spawn(async move {
            let _ = invoke_tauri_json::<()>("updater_restart", serde_json::json!({})).await;
        });
    };

    #[cfg(not(target_arch = "wasm32"))]
    let check_update = move |_| {};
    #[cfg(not(target_arch = "wasm32"))]
    let download_update = move |_| {};
    #[cfg(not(target_arch = "wasm32"))]
    let restart_app = move |_| {};

    let current_state = state();

    rsx! {
        div {
            style: "{PANEL_STYLE}",
            div {
                h3 { style: "{TITLE_STYLE}", "Software Update" }
                p { style: "{DESC_STYLE}", "Keep Qualia Webizen up to date with the latest features and security improvements." }
            }
            
            match current_state {
                UpdaterState::Idle => rsx! {
                    sl-button {
                        variant: "primary",
                        onclick: check_update,
                        "Check for Updates"
                    }
                },
                UpdaterState::Checking => rsx! {
                    div {
                        display: "flex",
                        align_items: "center",
                        gap: "0.5rem",
                        sl-spinner {}
                        span { style: "color: var(--qualia-text-muted); font-size: 0.85rem;", "Checking for updates..." }
                    }
                },
                UpdaterState::Available(version) => rsx! {
                    div {
                        p { style: "color: var(--qualia-text); font-weight: 500;", "Version {version} is available." }
                        sl-button {
                            variant: "success",
                            onclick: download_update,
                            "Download & Install"
                        }
                    }
                },
                UpdaterState::Downloading { downloaded, total } => {
                    let percent = if total > 0 { (downloaded as f64 / total as f64) * 100.0 } else { 0.0 };
                    rsx! {
                        div {
                            div {
                                display: "flex",
                                justify_content: "space-between",
                                span { style: "color: var(--qualia-text); font-size: 0.85rem;", "Downloading update..." }
                                span { style: "color: var(--qualia-text-muted); font-size: 0.85rem;", "{percent:.1}%" }
                            }
                            div {
                                style: "{PROGRESS_CONTAINER}",
                                div {
                                    style: "{PROGRESS_BAR} width: {percent}%;"
                                }
                            }
                        }
                    }
                },
                UpdaterState::ReadyToRestart => rsx! {
                    div {
                        sl-alert {
                            variant: "success",
                            open: true,
                            sl-icon { slot: "icon", name: "check2-circle" }
                            "Update ready to install."
                        }
                        div { margin_top: "1rem", }
                        sl-button {
                            variant: "warning",
                            onclick: restart_app,
                            "Restart to Apply Update"
                        }
                    }
                },
                UpdaterState::Error(err) => rsx! {
                    div {
                        sl-alert {
                            variant: "danger",
                            open: true,
                            sl-icon { slot: "icon", name: "exclamation-triangle" }
                            "Update error: {err}"
                        }
                        div { margin_top: "1rem", }
                        sl-button {
                            variant: "default",
                            onclick: check_update,
                            "Try Again"
                        }
                    }
                }
            }
        }
    }
}
