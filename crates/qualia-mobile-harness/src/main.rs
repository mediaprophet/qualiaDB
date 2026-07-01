#![allow(non_snake_case)]

mod companion_device;

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{MessageEvent, WebSocket};
use wellfare_core::companion_pairing::MSG_CHALLENGE;
use wellfare_core::companion_sync::{CompanionCsvFile, CompanionHealthBundle};

fn main() {
    dioxus::launch(App);
}

const OUTBOX_KEY: &str = "qualia-wellfair-health-outbox";

#[wasm_bindgen(inline_js = r#"
    export async function startQrScanner(videoElementId, onScanSuccess, onScanError) {
        const video = document.getElementById(videoElementId);
        if (!video) {
            onScanError("Video element not found");
            return;
        }
        if (!('BarcodeDetector' in window)) {
            onScanError("Camera QR scan unavailable — enter the desktop WS URL manually.");
            return;
        }
        try {
            const stream = await navigator.mediaDevices.getUserMedia({
                video: { facingMode: { ideal: "environment" } },
                audio: false,
            });
            video.srcObject = stream;
            await video.play();
            const detector = new BarcodeDetector({ formats: ["qr_code"] });
            const scan = async () => {
                try {
                    const codes = await detector.detect(video);
                    if (codes.length > 0 && codes[0].rawValue) {
                        onScanSuccess(codes[0].rawValue);
                        stream.getTracks().forEach((t) => t.stop());
                        return;
                    }
                } catch (_) {}
                requestAnimationFrame(scan);
            };
            scan();
        } catch (err) {
            onScanError(String(err));
        }
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
    fn startQrScanner(
        video_id: &str,
        on_success: &Closure<dyn FnMut(String)>,
        on_error: &Closure<dyn FnMut(String)>,
    );
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
    ManualConnect,
    Connecting(String),
    Connected,
    BundleReady { json: String, file_count: usize },
    Error(String),
}

fn captured_at_unix() -> u32 {
    (js_sys::Date::new_0().get_time() / 1000.0) as u32
}

fn build_bundle_from_files(files: Vec<CompanionCsvFile>) -> Result<String, String> {
    let bundle = CompanionHealthBundle::new(companion_device::device_id(), captured_at_unix(), files);
    bundle.validate()?;
    serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())
}

fn App() -> Element {
    let mut state = use_signal(|| AppState::Initializing);
    let mut ws_target = use_signal(|| String::new());
    let mut ws_handle = use_signal(|| None::<WebSocket>);
    let mut status_msg = use_signal(|| String::new());
    let mut manual_ws_url = use_signal(|| String::new());
    let file_input_id = "samsung-csv-picker";

    use_effect(move || {
        if *state.read() == AppState::Initializing {
            let stored = loadOutbox(OUTBOX_KEY);
            if !stored.is_empty() {
                if let Ok(bundle) = serde_json::from_str::<CompanionHealthBundle>(&stored) {
                    let file_count = bundle.files.len();
                    state.set(AppState::BundleReady {
                        json: stored,
                        file_count,
                    });
                    status_msg.set("Restored health bundle from outbox.".into());
                    return;
                }
            }

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
            let on_success = Closure::wrap(Box::new(move |scanned_text: String| {
                web_sys::console::log_1(&format!("Scanned QR: {}", scanned_text).into());
                ws_target.set(scanned_text.clone());
                state.set(AppState::Connecting(scanned_text));
            }) as Box<dyn FnMut(String)>);
            let mut status_msg_scan = status_msg.clone();
            let on_error = Closure::wrap(Box::new(move |msg: String| {
                status_msg_scan.set(msg);
                state.set(AppState::ManualConnect);
            }) as Box<dyn FnMut(String)>);

            startQrScanner("qr-video-element", &on_success, &on_error);
            on_success.forget();
            on_error.forget();
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

                    let mut status_msg_ws = status_msg.clone();
                    let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
                        if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                            let text: String = txt.into();
                            web_sys::console::log_1(&format!("Received WS Message: {}", text).into());

                            if text.contains(MSG_CHALLENGE) {
                                web_sys::console::log_1(&"Signing Ed25519 pairing challenge...".into());
                                match companion_device::build_pairing_response(&text) {
                                    Ok(response_json) => {
                                        let _ = ws_clone.send_with_str(&response_json);
                                    }
                                    Err(e) => {
                                        status_msg_ws.set(format!("Pairing sign failed: {e}"));
                                        state.set(AppState::Error(e));
                                    }
                                }
                            } else if text.contains("AUTH_SUCCESS") {
                                state.set(AppState::Connected);
                                let pending = loadOutbox(OUTBOX_KEY);
                                if !pending.is_empty() {
                                    let payload = format!(
                                        r#"{{"type":"HEALTH_BUNDLE","bundle":{pending}}}"#
                                    );
                                    let _ = ws_clone.send_with_str(&payload);
                                    status_msg_ws.set("Sent pending health bundle to desktop.".into());
                                }
                            } else if text.contains("AUTH_DENIED") {
                                status_msg_ws.set("Desktop denied pairing.".into());
                                state.set(AppState::Error("Pairing denied by desktop".into()));
                            } else if text.contains("HEALTH_BUNDLE_ACK") {
                                status_msg_ws.set("Desktop acknowledged health bundle.".into());
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
                        style: "text-align: center; max-width: 420px;",
                        h3 { "Scan Desktop QR Code" }
                        video {
                            id: "qr-video-element",
                            width: "100%",
                            height: "auto",
                            autoplay: true,
                        }
                        p { "Point your camera at the QR shown in desktop WellFair → Tools." }
                        button {
                            style: "margin-top: 0.75rem; padding: 8px 14px; background: #444; color: #fff; border: none; border-radius: 4px;",
                            onclick: move |_| state.set(AppState::ManualConnect),
                            "Enter URL manually"
                        }
                    }
                },
                AppState::ManualConnect => rsx! {
                    div {
                        style: "max-width: 420px; width: 100%;",
                        h3 { "Connect manually" }
                        p {
                            style: "font-size: 0.85rem; color: #ccc;",
                            "Copy the WebSocket URL from desktop WellFair pairing panel."
                        }
                        input {
                            r#type: "text",
                            value: "{manual_ws_url.read()}",
                            placeholder: "ws://192.168.x.x:8080/mobile/stream",
                            style: "width: 100%; padding: 10px; margin: 0.75rem 0; border-radius: 4px; border: 1px solid #555; background: #222; color: #fff;",
                            oninput: move |e| manual_ws_url.set(e.value()),
                        }
                        button {
                            style: "padding: 10px 16px; background: #2a6f97; color: white; border: none; border-radius: 4px;",
                            onclick: move |_| {
                                let url = manual_ws_url.read().trim().to_string();
                                if url.starts_with("ws://") || url.starts_with("wss://") {
                                    ws_target.set(url.clone());
                                    state.set(AppState::Connecting(url));
                                } else {
                                    let mut msg_sig = status_msg.clone();
                                    msg_sig.set("Enter a ws:// or wss:// URL from your desktop.".into());
                                }
                            },
                            "Connect"
                        }
                        button {
                            style: "margin-left: 0.5rem; padding: 10px 16px; background: #444; color: white; border: none; border-radius: 4px;",
                            onclick: move |_| state.set(AppState::Scanning),
                            "Try camera again"
                        }
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
                                            let mut status_msg = status_msg.clone();
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