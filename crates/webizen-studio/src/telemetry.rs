use dioxus::prelude::*;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use web_sys::{window, WebSocket};

pub fn use_telemetry() {
    use_effect(move || {
        let window = window().expect("should have a window in this context");
        let storage = window.local_storage().expect("should have local storage").unwrap();
        
        let mut device_id = storage.get_item("qualia_device_id").unwrap();
        if device_id.is_none() {
            let new_id = Uuid::new_v4().to_string();
            storage.set_item("qualia_device_id", &new_id).unwrap();
            device_id = Some(new_id);
        }
        let device_id = device_id.unwrap();

        let host = window.location().host().unwrap_or_else(|_| "127.0.0.1:4567".to_string());
        let ws_url = format!("ws://{}/telemetry", host);
        
        match WebSocket::new(&ws_url) {
            Ok(ws) => {
                ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

                // Scrub the qualia_token from the URL bar to prevent history leaks
                if let Ok(history) = window.history() {
                    let location = window.location();
                    let mut clean_url = location.pathname().unwrap_or_else(|_| "/".to_string());
                    if let Ok(hash) = location.hash() {
                        clean_url.push_str(&hash);
                    }
                    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&clean_url));
                }

                let ws_clone = ws.clone();
                let device_id_clone = device_id.clone();
                
                let onopen_callback = Closure::<dyn FnMut()>::new(move || {
                    let msg = format!("INIT:{}", device_id_clone);
                    let _ = ws_clone.send_with_str(&msg);
                });
                ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
                onopen_callback.forget();

                let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: web_sys::MessageEvent| {
                    if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                        let text: String = txt.into();
                        if text == "REVOKE" {
                            let window = web_sys::window().unwrap();
                            let storage = window.local_storage().unwrap().unwrap();
                            let _ = storage.remove_item("qualia_device_id");
                            let _ = window.location().reload();
                        }
                    } else if let Ok(buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                        let uint8_array = js_sys::Uint8Array::new(&buffer);
                        let mut bytes = vec![0; uint8_array.length() as usize];
                        uint8_array.copy_to(&mut bytes);
                        
                        if let Ok(text) = String::from_utf8(bytes.clone()) {
                            if text == "REVOKE" {
                                let window = web_sys::window().unwrap();
                                let storage = window.local_storage().unwrap().unwrap();
                                let _ = storage.remove_item("qualia_device_id");
                                let _ = window.location().reload();
                                return;
                            }
                        }
                        
                        // TODO: Handle raw binary CBOR-LD graph updates using zero-copy deserialization
                        // let _byte_len = buffer.byte_length();
                        // println!("Received binary payload of size: {}", byte_len);
                    }
                });
                ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
                onmessage_callback.forget();

                // Heartbeat interval
                let ws_for_interval = ws.clone();
                let interval_callback = Closure::<dyn FnMut()>::new(move || {
                    if ws_for_interval.ready_state() == WebSocket::OPEN {
                        let _ = ws_for_interval.send_with_str("HEARTBEAT");
                    }
                });
                let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
                    interval_callback.as_ref().unchecked_ref(),
                    10000,
                );
                interval_callback.forget();
            }
            Err(e) => {
                eprintln!("WebSocket connection error: {:?}", e);
            }
        }
    });
}
