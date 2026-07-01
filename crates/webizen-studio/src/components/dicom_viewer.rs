use dioxus::prelude::*;

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    pub async fn tauri_invoke(cmd: &str, args: wasm_bindgen::JsValue) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[component]
pub fn DicomViewer() -> Element {
    let mut slice_idx = use_signal(|| 45);
    let window_level = use_signal(|| 40);
    let window_width = use_signal(|| 400);
    let mut tool = use_signal(|| "pan".to_string());
    
    let mut image_data_b64 = use_signal(|| String::new());

    use_effect(move || {
        let current_slice = slice_idx();
        spawn(async move {
            // Using the slice index as a slot mock for diffusion_frame
            let args = serde_json::json!({ "slot": (current_slice % 8) as u8 });
            if let Ok(js_val) = serde_wasm_bindgen::to_value(&args) {
                if let Ok(res) = tauri_invoke("get_diffusion_frame_rgba", js_val).await {
                    if let Ok(data) = serde_wasm_bindgen::from_value::<Vec<u8>>(res) {
                        use base64::Engine;
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                        image_data_b64.set(format!("data:image/png;base64,{}", b64));
                    }
                }
            }
        });
    });

    let opacity = window_width() as f64 / 800.0;
    let brightness = window_level() as f64 / 50.0;

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100%; min-height: 500px; background: #000000; border-radius: 16px; overflow: hidden; color: #a1a1aa; font-family: 'Inter', sans-serif; box-shadow: 0 10px 30px rgba(0,0,0,0.8); border: 1px solid #27272a;",

            // Toolbar
            div {
                style: "display: flex; padding: 1rem; background: #18181b; border-bottom: 1px solid #27272a; gap: 1rem; align-items: center;",
                div { style: "font-weight: 600; color: #f4f4f5; margin-right: 2rem;", "DICOM Viewer" }

                button {
                    style: "padding: 0.5rem 1rem; border-radius: 6px; border: none; cursor: pointer; background: #3f3f46; color: white;",
                    onclick: move |_| tool.set("pan".to_string()),
                    "✋ Pan"
                }
                button {
                    style: "padding: 0.5rem 1rem; border-radius: 6px; border: none; cursor: pointer; background: transparent; color: #a1a1aa;",
                    onclick: move |_| tool.set("zoom".to_string()),
                    "🔍 Zoom"
                }
                button {
                    style: "padding: 0.5rem 1rem; border-radius: 6px; border: none; cursor: pointer; background: transparent; color: #a1a1aa;",
                    onclick: move |_| tool.set("wl".to_string()),
                    "🌓 W/L"
                }
                button {
                    style: "padding: 0.5rem 1rem; border-radius: 6px; border: none; cursor: pointer; background: transparent; color: #a1a1aa;",
                    onclick: move |_| tool.set("measure".to_string()),
                    "📏 Measure"
                }

                div { style: "flex-grow: 1;" }

                div { style: "font-size: 0.85rem;", "Series: AXIAL T1 SE" }
            }

            // Viewport
            div {
                style: "flex: 1; position: relative; background: radial-gradient(circle, #27272a, #000000); display: flex; align-items: center; justify-content: center;",

                if !image_data_b64.read().is_empty() {
                    img {
                        src: "{image_data_b64.read()}",
                        style: "max-width: 100%; max-height: 100%; opacity: {opacity}; filter: brightness({brightness});"
                    }
                } else {
                    div {
                        style: "width: 400px; height: 400px; border-radius: 50%; background: conic-gradient(from 0deg, #111, #444, #888, #333, #111); opacity: {opacity}; filter: brightness({brightness}); box-shadow: inset 0 0 50px black;"
                    }
                }

                // Overlays
                div { style: "position: absolute; top: 1rem; left: 1rem; font-size: 0.85rem; color: #f4f4f5; text-shadow: 1px 1px 2px black;",
                    div { "DOE, JOHN" }
                    div { "DOB: 1980-01-01" }
                    div { "ID: 987654321" }
                }

                div { style: "position: absolute; bottom: 1rem; left: 1rem; font-size: 0.85rem; color: #f4f4f5; text-shadow: 1px 1px 2px black;",
                    div { "W: {window_width} L: {window_level}" }
                    div { "Slice: {slice_idx} / 120" }
                }

                div { style: "position: absolute; top: 1rem; right: 1rem; font-size: 0.85rem; color: #f4f4f5; text-align: right; text-shadow: 1px 1px 2px black;",
                    div { "Hosp: GEN. HOSPITAL" }
                    div { "Date: 2026-06-14" }
                }

                div { style: "position: absolute; bottom: 1rem; right: 1rem; font-size: 0.85rem; color: #f4f4f5; text-align: right; text-shadow: 1px 1px 2px black;",
                    div { "Zoom: 1.5x" }
                    div { "Thickness: 5.0mm" }
                }
            }

            // Slice Slider
            div {
                style: "padding: 1rem; background: #18181b; border-top: 1px solid #27272a; display: flex; gap: 1rem; align-items: center;",
                span { style: "font-size: 0.85rem; min-width: 50px;", "Slice" }
                input {
                    type: "range",
                    min: "1",
                    max: "120",
                    value: "{slice_idx}",
                    oninput: move |e| { if let Ok(v) = e.value().parse() { slice_idx.set(v); } },
                    style: "flex: 1; accent-color: #3b82f6;"
                }
                span { style: "font-size: 0.85rem; min-width: 30px;", "{slice_idx}" }
            }
        }
    }
}
