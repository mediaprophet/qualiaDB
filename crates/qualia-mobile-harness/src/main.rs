#![allow(non_snake_case)]

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use wellfare_core::companion_sync::{CompanionCsvFile, CompanionHealthBundle};

fn main() {
    dioxus::launch(App);
}

const OUTBOX_KEY: &str = "qualia-wellfair-health-outbox";

#[wasm_bindgen(inline_js = r#"
    export function startQrScanner(videoElementId, onScanSuccess) {
        console.log("Starting QR scanner on element:", videoElementId);
        setTimeout(() => {
            const simulatedDesktopIP = "192.168.1.45:8080";
            onScanSuccess(`ws://${simulatedDesktopIP}/mobile/stream`);
        }, 2000);
    }

    export async function requestDirectoryAccess() {
        if (!('showDirectoryPicker' in window)) {
            throw new Error("File System Access API not supported");
        }
        const dirHandle = await window.showDirectoryPicker({ mode: 'readwrite' });
        console.log("Directory access granted:", dirHandle.name);
        return true;
    }

    export async function readCsvFilesFromInput(inputId) {
        const input = document.getElementById(inputId);
        if (!input || !input.files || input.files.length === 0) {
            return [];
        }
        const out = [];
        for (const file of input.files) {
            const text = await file.text();
            out.push({ filename: file.name, csv_content: text });
        }
        return out;
    }

    export function persistOutbox(key, json) {
        localStorage.setItem(key, json);
    }

    export function loadOutbox(key) {
        return localStorage.getItem(key) || "";
    }

    export async function copyToClipboard(text) {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            await navigator.clipboard.writeText(text);
            return true;
        }
        return false;
    }

    export async function shareBundle(title, text) {
        if (navigator.share) {
            await navigator.share({ title, text });
            return true;
        }
        return false;
    }
"#)]
extern "C" {
    fn startQrScanner(video_id: &str, callback: &Closure<dyn FnMut(String)>);
    #[wasm_bindgen(catch)]
    async fn requestDirectoryAccess() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn readCsvFilesFromInput(input_id: &str) -> Result<JsValue, JsValue>;
    fn persistOutbox(key: &str, json: &str);
    fn loadOutbox(key: &str) -> String;
    #[wasm_bindgen(catch)]
    async fn copyToClipboard(text: &str) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch)]
    async fn shareBundle(title: &str, text: &str) -> Result<JsValue, JsValue>;
}

#[derive(Clone, PartialEq)]
enum AppState {
    Initializing,
    VaultInit,
    Scanning,
    Connecting(String),
    Connected,
    BundleReady { json: String, file_count: usize },
    Error(String),
}

fn device_id() -> String {
    let stored = loadOutbox("qualia-device-id");
    if !stored.is_empty() {
        return stored;
    }
    let id = format!("phone-{}", js_sys::Math::random().to_string());
    persistOutbox("qualia-device-id", &id);
    id
}

fn captured_at_unix() -> u32 {
    (js_sys::Date::new_0().get_time() / 1000.0) as u32
}

fn build_bundle_from_files(files: Vec<CompanionCsvFile>) -> Result<String, String> {
    let bundle = CompanionHealthBundle::new(device_id(), captured_at_unix(), files);
    bundle.validate()?;
    serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())
}

fn App() -> Element {
    let mut state = use_signal(|| AppState::Initializing);
    let mut ws_target = use_signal(|| String::new());
    let mut ws_handle = use_signal(|| None::<WebSocket>);
    let mut status_msg = use_signal(|| String::new());
    let file_input_id = "samsung-csv-picker";

    use_effect(move || {
        if *state.read() == AppState::Initializing {
            let window = web_sys::window().unwrap();
            let has_picker = js_sys::Reflect::has(
                &window,
                &JsValue::from_str("showDirectoryPicker"),
            )
            .unwrap_or(false);

            if has_picker {
                state.set(AppState::VaultInit);
            } else {
                state.set(AppState::Scanning);
            }
        }
    });

    use_effect(move || {
        if *state.read() == AppState::Scanning {
            let callback = Closure::wrap(Box::new(move |scanned_text: String| {
                web_sys::console::log_1(&format!("Scanned QR: {}", scanned_text).into());
                ws_target.set(scanned_text.clone());
                state.set(AppState::Connecting(scanned_text));
            }) as Box<dyn FnMut(String)>);

            startQrScanner("qr-video-element", &callback);
            callback.forget();
        }
    });

    use_effect(move || {
        let connect_url = if let AppState::Connecting(ref url) = *state.read() {
            Some(url.clone())
        } else {
            None
        };

        if let Some(url) = connect_url {
            match WebSocket::new(&url) {
                Ok(ws) => {
                    ws_handle.set(Some(ws.clone()));
                    let ws_clone = ws.clone();
                    let on_open = Closure::wrap(Box::new(move |_| {
                        web_sys::console::log_1(&"WebSocket Connected".into());
                    }) as Box<dyn FnMut(JsValue)>);
                    ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));
                    on_open.forget();

                    let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
                        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            let text: String = txt.into();
                            web_sys::console::log_1(&format!("Received WS Message: {}", text).into());

                            if text == "CHALLENGE_BYTES_123456789" {
                                web_sys::console::log_1(&"Solving DID Challenge...".into());
                                let signature = "SIGNED_WITH_MOBILE_DID_777";
                                let _ = ws_clone.send_with_str(signature);
                            } else if text == "AUTH_SUCCESS" {
                                state.set(AppState::Connected);
                                let pending = loadOutbox(OUTBOX_KEY);
                                if !pending.is_empty() {
                                    let payload = format!(
                                        r#"{{"type":"HEALTH_BUNDLE","bundle":{pending}}}"#
                                    );
                                    let _ = ws_clone.send_with_str(&payload);
                                    status_msg.set("Sent pending health bundle to desktop.".into());
                                }
                            }
                        }
                    }) as Box<dyn FnMut(MessageEvent)>);
                    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
                    on_message.forget();
                }
                Err(e) => {
                    state.set(AppState::Error(format!("WS Error: {:?}", e)));
                }
            }
        }
    });

    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background-color: #1a1a1a; color: white; font-family: sans-serif; padding: 1rem;",

            h1 { "Qualia Mobile Companion" }

            match &*state.read() {
                AppState::Initializing => rsx! {
                    div { p { "Initializing Fiduciary Boundary..." } }
                },
                AppState::VaultInit => rsx! {
                    div {
                        style: "text-align: center; max-width: 420px;",
                        h3 { "Sovereign Vault Initialization" }
                        p { "Your device supports Tier-1 Edge capabilities." }
                        button {
                            style: "padding: 12px 24px; background-color: #4CAF50; color: white; border: none; border-radius: 4px; font-size: 16px; margin-top: 16px;",
                            onclick: move |_| {
                                let mut state = state.clone();
                                spawn(async move {
                                    match requestDirectoryAccess().await {
                                        Ok(_) => state.set(AppState::Scanning),
                                        Err(e) => web_sys::console::error_1(&e)
                                    }
                                });
                            },
                            "Initialize Local Vault"
                        }
                    }
                },
                AppState::Scanning => rsx! {
                    div {
                        style: "text-align: center;",
                        h3 { "Scan Desktop QR Code" }
                        video {
                            id: "qr-video-element",
                            width: "100%",
                            height: "auto",
                            autoplay: true,
                        }
                        p { "Pair with your authoritative desktop, then export Samsung Health CSVs here." }
                    }
                },
                AppState::Connecting(url) => rsx! {
                    div {
                        h3 { "Connecting to Desktop Engine" }
                        p { "Target: {url}" }
                    }
                },
                AppState::Connected | AppState::BundleReady { .. } => rsx! {
                    div {
                        style: "max-width: 480px; width: 100%;",
                        h3 { style: "color: #4CAF50;", "Companion linked" }
                        p {
                            style: "font-size: 0.9rem; color: #ccc;",
                            "Samsung Health exports are only on your phone. Select CSV files from your Samsung export, build a bundle, then share it to your desktop."
                        }
                        input {
                            id: "{file_input_id}",
                            r#type: "file",
                            accept: ".csv,text/csv",
                            multiple: true,
                            style: "margin: 1rem 0; color: #eee;",
                        }
                        div {
                            style: "display: flex; flex-wrap: wrap; gap: 0.5rem;",
                            button {
                                style: "padding: 10px 16px; background: #2a6f97; color: white; border: none; border-radius: 4px;",
                                onclick: move |_| {
                                    let mut state = state.clone();
                                    let mut status_msg = status_msg.clone();
                                    spawn(async move {
                                        match readCsvFilesFromInput(file_input_id).await {
                                            Ok(js_files) => {
                                                let files_val: js_sys::Array = js_files.dyn_into().unwrap_or_default();
                                                let mut companion_files = Vec::new();
                                                for item in files_val.iter() {
                                                    let obj = js_sys::Object::from(item);
                                                    let name = js_sys::Reflect::get(&obj, &JsValue::from_str("filename"))
                                                        .ok()
                                                        .and_then(|v| v.as_string())
                                                        .unwrap_or_default();
                                                    let content = js_sys::Reflect::get(&obj, &JsValue::from_str("csv_content"))
                                                        .ok()
                                                        .and_then(|v| v.as_string())
                                                        .unwrap_or_default();
                                                    companion_files.push(CompanionCsvFile {
                                                        filename: name,
                                                        csv_content: content,
                                                    });
                                                }
                                                match build_bundle_from_files(companion_files.clone()) {
                                                    Ok(json) => {
                                                        persistOutbox(OUTBOX_KEY, &json);
                                                        status_msg.set(format!(
                                                            "Bundle ready ({} file(s)). Saved to outbox.",
                                                            companion_files.len()
                                                        ));
                                                        state.set(AppState::BundleReady {
                                                            json: json.clone(),
                                                            file_count: companion_files.len(),
                                                        });
                                                    }
                                                    Err(e) => status_msg.set(format!("Bundle error: {e}")),
                                                }
                                            }
                                            Err(e) => status_msg.set(format!("File read error: {e:?}")),
                                        }
                                    });
                                },
                                "Build health bundle"
                            }
                            if let AppState::BundleReady { json, .. } = &*state.read() {
                                button {
                                    style: "padding: 10px 16px; background: #555; color: white; border: none; border-radius: 4px;",
                                    onclick: {
                                        let json = json.clone();
                                        let status_msg = status_msg.clone();
                                        let ws_handle = ws_handle.clone();
                                        move |_| {
                                            let json = json.clone();
                                            let status_msg = status_msg.clone();
                                            let ws_handle = ws_handle.clone();
                                            spawn(async move {
                                                if copyToClipboard(&json).await.is_ok() {
                                                    status_msg.set("Bundle copied — paste into desktop WellFair Tools.".into());
                                                } else if shareBundle("WellFair Health Bundle", &json).await.is_ok() {
                                                    status_msg.set("Share sheet opened.".into());
                                                } else {
                                                    status_msg.set("Copy/share unavailable — use desktop paste field.".into());
                                                }
                                                if let Some(ws) = ws_handle.read().clone() {
                                                    let payload = format!(
                                                        r#"{{"type":"HEALTH_BUNDLE","bundle":{json}}}"#
                                                    );
                                                    if ws.send_with_str(&payload).is_ok() {
                                                        status_msg.set("Bundle sent over companion link.".into());
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    "Share to desktop"
                                }
                            }
                        }
                        if !status_msg.read().is_empty() {
                            p { style: "margin-top: 0.75rem; font-size: 0.85rem; color: #aaa;", "{status_msg.read()}" }
                        }
                    }
                },
                AppState::Error(msg) => rsx! {
                    div {
                        h3 { style: "color: #F44336;", "Connection Error" }
                        p { "{msg}" }
                    }
                },
            }
        }
    }
}