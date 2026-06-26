use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use crate::extension_manifest::{ExtensionManifest, ExtensionCapability};
use crate::{q_hash, NQuin};

/// Global registry of discovered and provisioned capability extensions.
pub struct ExtensionBus {
    /// Maps an extension_id to its parsed manifest
    registered_extensions: RwLock<HashMap<String, ExtensionManifest>>,
    /// Maps a semantic interface hash (e.g., q_hash("q42:VideoTranscode")) to a list of capable extension IDs
    capability_index: RwLock<HashMap<u64, Vec<String>>>,
    /// The local directory where extensions are stored
    extensions_dir: PathBuf,
}

impl ExtensionBus {
    pub fn new(extensions_dir: impl AsRef<Path>) -> Self {
        let bus = Self {
            registered_extensions: RwLock::new(HashMap::new()),
            capability_index: RwLock::new(HashMap::new()),
            extensions_dir: extensions_dir.as_ref().to_path_buf(),
        };
        bus.scan_and_load_extensions();
        bus
    }

    /// Scans the extensions directory for `manifest.json` files and loads them into memory.
    pub fn scan_and_load_extensions(&self) {
        if !self.extensions_dir.exists() {
            let _ = fs::create_dir_all(&self.extensions_dir);
            return;
        }

        let mut reg_lock = self.registered_extensions.write().unwrap();
        let mut idx_lock = self.capability_index.write().unwrap();

        if let Ok(entries) = fs::read_dir(&self.extensions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let manifest_path = path.join("manifest.json");
                    if manifest_path.exists() {
                        if let Ok(json_bytes) = fs::read(&manifest_path) {
                            if let Ok(manifest) = ExtensionManifest::from_json(&json_bytes) {
                                println!("Loaded extension: {} (v{})", manifest.display_name, manifest.version);
                                
                                // Index capabilities
                                for cap in &manifest.capabilities {
                                    let cap_hash = q_hash(&cap.interface);
                                    idx_lock.entry(cap_hash).or_default().push(manifest.extension_id.clone());
                                }
                                
                                reg_lock.insert(manifest.extension_id.clone(), manifest);
                            } else {
                                eprintln!("Failed to parse manifest at {:?}", manifest_path);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Register a new extension from an absolute path (CLI workflow)
    pub fn register_extension_from_path(&self, manifest_path: &Path) -> Result<(), String> {
        let json_bytes = fs::read(manifest_path).map_err(|e| format!("Failed to read manifest: {}", e))?;
        let manifest = ExtensionManifest::from_json(&json_bytes).map_err(|e| format!("Invalid manifest JSON: {}", e))?;
        
        // Copy to local extensions dir
        let target_dir = self.extensions_dir.join(&manifest.extension_id);
        fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create extension dir: {}", e))?;
        fs::write(target_dir.join("manifest.json"), &json_bytes).map_err(|e| format!("Failed to write manifest: {}", e))?;

        let mut reg_lock = self.registered_extensions.write().unwrap();
        let mut idx_lock = self.capability_index.write().unwrap();

        for cap in &manifest.capabilities {
            let cap_hash = q_hash(&cap.interface);
            idx_lock.entry(cap_hash).or_default().push(manifest.extension_id.clone());
        }
        
        reg_lock.insert(manifest.extension_id.clone(), manifest);
        Ok(())
    }

    /// Retrieve all registered manifests (used by the Studio UI)
    pub fn list_extensions(&self) -> Vec<ExtensionManifest> {
        let lock = self.registered_extensions.read().unwrap();
        lock.values().cloned().collect()
    }

    /// Query the bus for any extensions supporting a given semantic interface hash
    pub fn query_capability(&self, interface_hash: u64) -> Vec<ExtensionManifest> {
        let idx_lock = self.capability_index.read().unwrap();
        let reg_lock = self.registered_extensions.read().unwrap();
        
        let mut results = Vec::new();
        if let Some(ext_ids) = idx_lock.get(&interface_hash) {
            for id in ext_ids {
                if let Some(manifest) = reg_lock.get(id) {
                    results.push(manifest.clone());
                }
            }
        }
        results
    }

    /// Dispatches a task recipe to an extension over localhost RPC.
    /// Includes Sentinel Gatekeeping logic to enforce sensitivity classification.
    pub fn dispatch_task(
        &self,
        extension_id: &str,
        input_file_path: &str,
        pipeline_steps: Vec<serde_json::Value>,
        sensitivity_context: u64,
        guardianship_override: bool,
    ) -> Result<String, String> {
        // B6: Sentinel Gatekeeping
        let sensitivity = sensitivity_context >> 56;
        if sensitivity == 0x02 && !guardianship_override {
            // Log violation to WAL
            let violation_quin = NQuin {
                subject: q_hash(extension_id),
                predicate: q_hash("q42:GatekeeperViolation"),
                object: q_hash(input_file_path),
                context: sensitivity_context,
                metadata: 0,
                parity: 0, // XOR fold omitted for brevity
            };
            let _ = crate::wal::append_mutation(&violation_quin);
            
            return Err("GATEKEEPER_BLOCK: Cannot send Classified (0x02) data to an extension without Guardianship override.".to_string());
        }

        let reg_lock = self.registered_extensions.read().unwrap();
        let manifest = reg_lock.get(extension_id).ok_or("Extension not found")?;

        // B5: Task Recipe Payload
        let payload = serde_json::json!({
            "input_file_path": input_file_path, // Zero-copy pointer to local file
            "pipeline_steps": pipeline_steps,
            "output_routing": "http://127.0.0.1:8080/ingest" // Loopback ingest target
        });

        // Simulate Dispatch
        println!("Dispatching task to {} via {:?}: {}", manifest.display_name, manifest.transport, payload);

        // In a real system, we would open a Reqwest client (LocalHttp) or NamedPipe connection here
        Ok(format!("Task dispatched successfully to {}", extension_id))
    }

    /// B4: Provisioning Loop
    /// Downloads a required asset (e.g., ONNX model weights) for an extension
    /// into the extension's local data directory.
    pub async fn provision_asset(
        &self,
        extension_id: &str,
        asset_url: &str,
        filename: &str,
    ) -> Result<String, String> {
        let reg_lock = self.registered_extensions.read().unwrap();
        if !reg_lock.contains_key(extension_id) {
            return Err("Extension not found".to_string());
        }

        let target_dir = self.extensions_dir.join(extension_id).join("assets");
        std::fs::create_dir_all(&target_dir).map_err(|e| format!("Failed to create asset dir: {}", e))?;

        let target_file = target_dir.join(filename);
        
        println!("Provisioning asset from {} into {:?}", asset_url, target_file);
        
        // In production, this would use Reqwest to stream the download with a progress bar.
        // For the Phase B architecture proof, we create a placeholder file.
        std::fs::write(&target_file, b"MOCK_ASSET_DATA")
            .map_err(|e| format!("Failed to write asset: {}", e))?;

        Ok(format!("Asset provisioned successfully at {:?}", target_file))
    }
}

#[cfg(target_arch = "wasm32")]
pub mod wasm_bus {
    use std::cell::RefCell;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;
    use web_sys::{ErrorEvent, Event, MessageEvent, WebSocket};
    use serde::{Serialize, Deserialize};

    thread_local! {
        pub static EXTENSION_BUS: RefCell<Option<ExtensionBusState>> = RefCell::new(None);
    }

    pub struct ExtensionBusState {
        pub ws: WebSocket,
        pub on_open: Closure<dyn FnMut(Event)>,
        pub on_message: Closure<dyn FnMut(MessageEvent)>,
        pub on_error: Closure<dyn FnMut(ErrorEvent)>,
        pub on_close: Closure<dyn FnMut(Event)>,
        pub is_authenticated: bool,
        pub active_token_callback: Option<Box<dyn FnMut(String)>>,
    }

    #[derive(Serialize)]
    struct ChallengePayload {
        pub challenge: String,
        pub did: String,
    }

    #[derive(Serialize)]
    struct IntentPayload {
        pub rpc: String,
        pub prompt: String,
        pub graph_context: String,
        pub signature: String,
    }

    pub fn init_extension_bus(did: String) -> Result<(), JsValue> {
        let ws = WebSocket::new("ws://127.0.0.1:4242")?;
        
        let ws_clone = ws.clone();
        let did_clone = did.clone();
        
        let on_open = Closure::wrap(Box::new(move |_e: Event| {
            let payload = ChallengePayload {
                challenge: "did:q42".into(),
                did: did_clone.clone(),
            };
            if let Ok(json) = serde_json::to_string(&payload) {
                let _ = ws_clone.send_with_str(&json);
            }
        }) as Box<dyn FnMut(Event)>);
        ws.set_onopen(Some(on_open.as_ref().unchecked_ref()));

        let on_message = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let s: String = txt.into();
                // Parse the response
                if s.contains("\"authenticated\":true") {
                    EXTENSION_BUS.with(|bus| {
                        if let Some(state) = bus.borrow_mut().as_mut() {
                            state.is_authenticated = true;
                        }
                    });
                } else if s.contains("\"token\":") || s.contains("\"text\":") {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                        if let Some(text) = v.get("text").and_then(|t| t.as_str()) {
                            EXTENSION_BUS.with(|bus| {
                                if let Some(state) = bus.borrow_mut().as_mut() {
                                    if let Some(ref mut cb) = state.active_token_callback {
                                        cb(text.to_string());
                                    }
                                }
                            });
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));

        let on_error = Closure::wrap(Box::new(move |_e: ErrorEvent| {
            // Placeholder for error telemetry
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        
        let on_close = Closure::wrap(Box::new(move |_e: Event| {
            EXTENSION_BUS.with(|bus| {
                *bus.borrow_mut() = None;
            });
        }) as Box<dyn FnMut(Event)>);
        ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));

        EXTENSION_BUS.with(|bus| {
            *bus.borrow_mut() = Some(ExtensionBusState {
                ws,
                on_open,
                on_message,
                on_error,
                on_close,
                is_authenticated: false,
                active_token_callback: None,
            });
        });

        Ok(())
    }

    pub fn is_connected() -> bool {
        EXTENSION_BUS.with(|bus| {
            bus.borrow().as_ref().map(|s| s.is_authenticated).unwrap_or(false)
        })
    }

    pub fn send_intent<F: FnMut(String) + 'static>(prompt: &str, graph_context: &str, on_token: F) -> Result<(), String> {
        let payload = IntentPayload {
            rpc: "infer_local_model".into(),
            prompt: prompt.to_string(),
            graph_context: graph_context.to_string(),
            signature: "did:q42:active".into(), 
        };
        let intent_json = serde_json::to_string(&payload).unwrap_or_default();
        
        EXTENSION_BUS.with(|bus| {
            if let Some(state) = bus.borrow_mut().as_mut() {
                if state.is_authenticated {
                    state.active_token_callback = Some(Box::new(on_token));
                    state.ws.send_with_str(&intent_json).map_err(|e| format!("{:?}", e))?;
                    return Ok(());
                }
            }
            Err("Not connected or authenticated".into())
        })
    }
}
