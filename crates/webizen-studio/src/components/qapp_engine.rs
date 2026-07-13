use dioxus::prelude::*;
use serde::Deserialize;

// ── Dual-mode Tauri invoke ───────────────────────────────────────────────────
//
// On WASM (web build): calls `window.__TAURI__.core.invoke()` via wasm_bindgen.
// On native (desktop build): calls the local settings server's REST API on
//   http://127.0.0.1:8080/api/invoke/{cmd} — the settings server proxies all
//   Tauri commands through a single REST endpoint.
//
// This allows the same component code to work in both builds without change.

#[cfg(target_arch = "wasm32")]
mod imp {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(
            js_namespace = ["window", "__TAURI__", "core"],
            js_name = invoke,
            catch
        )]
        async fn tauri_invoke_raw(
            cmd: &str,
            args: wasm_bindgen::JsValue,
        ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
    }

    pub async fn tauri_invoke(
        cmd: &str,
        args: wasm_bindgen::JsValue,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
        let global = js_sys::global();
        let tauri = js_sys::Reflect::get(&global, &JsValue::from_str("__TAURI__"))?;
        let core = js_sys::Reflect::get(&tauri, &JsValue::from_str("core"))?;
        let invoke = js_sys::Reflect::get(&core, &JsValue::from_str("invoke"))?;
        if !invoke.is_function() {
            return Err(JsValue::from_str(
                "This action requires the Webizen desktop host",
            ));
        }

        tauri_invoke_raw(cmd, args).await
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use serde_json::Value;

    /// Native invoke — routes to specific typed REST portals on the local
    /// settings server. The generic `/api/invoke/{cmd}` proxy has been removed
    /// to lock down the control plane.
    pub async fn tauri_invoke(
        cmd: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (method, endpoint) = match cmd {
            "get_supervisor_state" => ("GET", "/api/status".to_string()),
            "get_config" => ("GET", "/api/config".to_string()),
            "save_config" => ("POST", "/api/config".to_string()),
            "list_jobs" => ("GET", "/api/jobs".to_string()),
            "enqueue_job" => ("POST", "/api/jobs".to_string()),
            "system_telemetry" => ("GET", "/api/telemetry".to_string()),
            "execute_sparql_query" => ("POST", "/api/sparql/query".to_string()),
            _ => return Err(format!("Command '{cmd}' is not exposed via typed REST portals")),
        };

        let url = format!("http://127.0.0.1:8080{endpoint}");
        let client = reqwest::Client::new();
        
        let req = if method == "GET" {
            client.get(&url)
        } else {
            client.post(&url).json(&args)
        };

        let resp = req
            .send()
            .await
            .map_err(|e| format!("invoke {cmd}: {e}"))?;

        if resp.status().is_success() {
            resp.json::<Value>()
                .await
                .map_err(|e| format!("invoke {cmd} parse: {e}"))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("invoke {cmd} failed: {status} — {body}"))
        }
    }
}

// Re-export the invoke function for both modes
#[cfg(target_arch = "wasm32")]
pub use imp::tauri_invoke;

#[cfg(not(target_arch = "wasm32"))]
pub use imp::tauri_invoke;

// ── Convenience wrapper for native mode ──────────────────────────────────────
//
// On native, tauri_invoke takes serde_json::Value and returns serde_json::Value.
// On wasm, it takes wasm_bindgen::JsValue and returns wasm_bindgen::JsValue.
// Components that use serde_json can use this helper on native:

#[cfg(not(target_arch = "wasm32"))]
pub async fn invoke_json(cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    tauri_invoke(cmd, args).await
}

#[cfg(target_arch = "wasm32")]
pub async fn invoke_json(cmd: &str, args: serde_json::Value) -> Result<serde_json::Value, String> {
    let js_args = serde_wasm_bindgen::to_value(&args)
        .map_err(|e| format!("serialize args: {e}"))?;
    let result = tauri_invoke(cmd, js_args).await
        .map_err(|e| format!("invoke {cmd}: {:?}", e))?;
    serde_wasm_bindgen::from_value::<serde_json::Value>(result)
        .map_err(|e| format!("deserialize result: {e}"))
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
            if let Ok(res) = invoke_json("fetch_domain_ontology", args).await {
                if let Ok(parsed) = serde_json::from_value::<OntologySchema>(res) {
                    schema.set(Some(parsed));
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
                div { class: "text-gray-400", "Loading domain ontology constraints..." }
            }
        }
    }
}
