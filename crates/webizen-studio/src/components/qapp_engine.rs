use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    pub async fn tauri_invoke(cmd: &str, args: wasm_bindgen::JsValue) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[derive(Props, Clone, PartialEq)]
pub struct QAppEngineProps {
    pub ontology_id: String,
    pub title: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ShaclProperty {
    path: String,
    datatype: Option<String>,
    name: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct ShaclShape {
    #[serde(rename = "targetClass")]
    target_class: String,
    properties: Vec<ShaclProperty>,
}

#[derive(Deserialize, Debug, Clone)]
struct OntologySchema {
    domain: String,
    shapes: Vec<ShaclShape>,
}

#[component]
pub fn QAppEngine(props: QAppEngineProps) -> Element {
    let mut schema = use_signal(|| None::<OntologySchema>);
    let ontology_id = props.ontology_id.clone();

    use_effect(move || {
        let domain_id = ontology_id.clone();
        spawn(async move {
            let args = serde_json::json!({ "domainId": domain_id });
            if let Ok(js_val) = serde_wasm_bindgen::to_value(&args) {
                if let Ok(res) = tauri_invoke("fetch_domain_ontology", js_val).await {
                    if let Some(json_str) = res.as_string() {
                        if let Ok(parsed) = serde_json::from_str::<OntologySchema>(&json_str) {
                            schema.set(Some(parsed));
                        }
                    }
                }
            }
        });
    });

    rsx! {
        div { class: "qapp-engine w-full h-full p-6 text-qualia-fg bg-qualia-bg",
            h2 { class: "text-2xl font-bold mb-4", "{props.title}" }
            
            if let Some(s) = schema.read().as_ref() {
                div { class: "ontology-viewer",
                    div { class: "text-sm text-gray-400 mb-6", "Domain: {s.domain}" }
                    
                    for shape in s.shapes.iter() {
                        div { class: "shape-form mb-8 p-4 border border-gray-700 rounded-lg",
                            h3 { class: "text-xl font-semibold mb-4", "{shape.target_class}" }
                            
                            form { class: "flex flex-col gap-4",
                                for prop in shape.properties.iter() {
                                    div { class: "form-group flex flex-col",
                                        label { class: "text-sm font-medium mb-1", "{prop.name.as_deref().unwrap_or(&prop.path)}" }
                                        input { 
                                            class: "px-3 py-2 bg-gray-800 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500",
                                            placeholder: "Enter {prop.datatype.as_deref().unwrap_or(\"value\")}"
                                        }
                                        div { class: "text-xs text-gray-500 mt-1", "Path: {prop.path}" }
                                    }
                                }
                                button { 
                                    class: "mt-4 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-semibold rounded",
                                    type: "submit", 
                                    "Save Node" 
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "text-gray-400", "Loading domain ontology constraints via Tauri..." }
            }
        }
    }
}
