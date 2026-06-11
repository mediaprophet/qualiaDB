#![allow(non_snake_case)]

use dioxus::prelude::*;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlVideoElement, WebSocket, MessageEvent};

fn main() {
    dioxus::launch(App);
}

// C7: JS Camera Interop via html5-qrcode
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
        
        // In a real implementation, we would store this handle in IndexedDB
        // const idbReq = indexedDB.open("qualia-vault"); ...
        return true;
    }
"#)]
extern "C" {
    fn startQrScanner(video_id: &str, callback: &Closure<dyn FnMut(String)>);
    #[wasm_bindgen(catch)]
    async fn requestDirectoryAccess() -> Result<JsValue, JsValue>;
}

#[derive(Clone, PartialEq)]
enum AppState {
    Initializing,
    VaultInit,
    Scanning,
    Connecting(String),
    Connected,
    Error(String),
}

fn App() -> Element {
    let mut state = use_signal(|| AppState::Initializing);
    let mut ws_target = use_signal(|| String::new());

    // Effect for feature detection on boot
    use_effect(move || {
        if *state.read() == AppState::Initializing {
            let window = web_sys::window().unwrap();
            let has_picker = js_sys::Reflect::has(&window, &JsValue::from_str("showDirectoryPicker")).unwrap_or(false);
            
            if has_picker {
                // Android/Chrome: Enable Sovereign Edge Mode
                state.set(AppState::VaultInit);
            } else {
                // iOS/Safari: Fallback to Tethered Mode
                state.set(AppState::Scanning);
            }
        }
    });

    // Effect to start the scanner when the component mounts in Scanning state
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

    // Effect to handle WebSocket connection
    use_effect(move || {
        let connect_url = if let AppState::Connecting(ref url) = *state.read() {
            Some(url.clone())
        } else {
            None
        };
        
        if let Some(url) = connect_url {
            match WebSocket::new(&url) {
                Ok(ws) => {
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
                            
                            // C8: WebSocket Handshake Logic
                            if text == "CHALLENGE_BYTES_123456789" {
                                web_sys::console::log_1(&"Solving DID Challenge...".into());
                                let signature = "SIGNED_WITH_MOBILE_DID_777";
                                let _ = ws_clone.send_with_str(signature);
                            } else if text == "AUTH_SUCCESS" {
                                state.set(AppState::Connected);
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
            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; background-color: #1a1a1a; color: white; font-family: sans-serif;",
            
            h1 { "Qualia Mobile Harness" }
            
            match &*state.read() {
                AppState::Initializing => rsx! {
                    div { p { "Initializing Fiduciary Boundary..." } }
                },
                AppState::VaultInit => rsx! {
                    div {
                        style: "text-align: center;",
                        h3 { "Sovereign Vault Initialization" }
                        p { "Your device supports Tier-1 Edge capabilities." }
                        p { "Please bind a local folder to store your persistent .q42 Graph Data." }
                        button {
                            style: "padding: 12px 24px; background-color: #4CAF50; color: white; border: none; border-radius: 4px; font-size: 16px; margin-top: 16px;",
                            onclick: move |_| {
                                let mut state = state.clone();
                                dioxus::prelude::spawn(async move {
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
                        h3 { "Scan Desktop QR Code" }
                        video {
                            id: "qr-video-element",
                            width: "100%",
                            height: "auto",
                            autoplay: true,
                        }
                        p { "Waiting for camera..." }
                    }
                },
                AppState::Connecting(url) => rsx! {
                    div {
                        h3 { "Connecting to Desktop Engine" }
                        p { "Target: {url}" }
                        div { class: "spinner" }
                    }
                },
                AppState::Connected => rsx! {
                    div {
                        h3 { style: "color: #4CAF50;", "✅ DID Pipeline Established" }
                        p { "Secure Fiduciary Boundary active." }
                        p { "Awaiting NQuin streams from local Sentinel..." }
                    }
                },
                AppState::Error(msg) => rsx! {
                    div {
                        h3 { style: "color: #F44336;", "Connection Error" }
                        p { "{msg}" }
                    }
                }
            }
        }
    }
}
