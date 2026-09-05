//! WasmIntentBus — concrete IntentBus implementation for WASM.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Routes VibeScript payloads to the desktop daemon via fetch/WebSocket
//! when running inside the Tauri webview, or fails closed on public web.

use std::sync::atomic::{AtomicU64, Ordering};
use wasm_bindgen::JsCast;

use crate::tool_chest::core::intent_bus::{
    ActionType, IntentBus, IntentReceipt, IntentStatus, Provenance, VibeScriptPayload,
};

/// Errors produced by the WASM intent bus.
#[derive(Debug)]
pub enum WasmBusError {
    /// Not running inside the desktop host — engine unreachable.
    NotNativeHost,
    /// CBOR-LD serialisation failed.
    CborError(String),
    /// Network fetch failed.
    FetchError(String),
    /// Capability gate rejected the intent.
    Rejected(String),
}

impl std::fmt::Display for WasmBusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotNativeHost => write!(f, "not native host — engine unreachable"),
            Self::CborError(s) => write!(f, "CBOR-LD serialisation error: {}", s),
            Self::FetchError(s) => write!(f, "fetch error: {}", s),
            Self::Rejected(s) => write!(f, "capability rejected: {}", s),
        }
    }
}

impl std::error::Error for WasmBusError {}

/// Concrete IntentBus for the WASM HyperCanvas.
///
/// Stamps provenance, serialises to CBOR-LD, and routes to the
/// desktop daemon via `fetch`. On public web, returns `Rejected`.
pub struct WasmIntentBus {
    /// Monotonic dispatch counter.
    counter: AtomicU64,
    /// Emitter DID for provenance stamping.
    emitter_did: String,
}

impl WasmIntentBus {
    /// Create a new bus with the given emitter DID.
    pub fn new(emitter_did: impl Into<String>) -> Self {
        Self {
            counter: AtomicU64::new(1),
            emitter_did: emitter_did.into(),
        }
    }

    /// Create a bus with the default demo identity.
    pub fn with_default_identity() -> Self {
        Self::new("did:qualia:timothy_charles_holborn")
    }

    /// Stamp provenance on a payload.
    fn stamp_provenance(&self, payload: &mut VibeScriptPayload<impl serde::Serialize>) {
        let id = self.counter.fetch_add(1, Ordering::SeqCst);
        payload.provenance = Some(Provenance {
            emitter_did: self.emitter_did.clone(),
            component_label: "qualia-ui::browser".into(),
            intent_counter: id,
            capability_scope: payload
                .provenance
                .as_ref()
                .and_then(|p| p.capability_scope.clone())
                .or_else(|| capability_scope_for(payload.action_type)),
        });
    }

    /// Serialise a payload to CBOR-LD bytes.
    fn encode_cbor_ld<P: serde::Serialize>(
        &self,
        payload: &VibeScriptPayload<P>,
    ) -> Result<Vec<u8>, WasmBusError> {
        let mut bytes = Vec::new();
        ciborium::into_writer(payload, &mut bytes)
            .map_err(|e| WasmBusError::CborError(e.to_string()))?;
        Ok(bytes)
    }
}

/// Map an ActionType to a live ALL_BOUND id when the tool did not name one.
fn capability_scope_for(action: ActionType) -> Option<String> {
    match action {
        ActionType::Query => Some("GraphDatabase.sparql".into()),
        ActionType::Mutate => Some("GraphAuthoring.process".into()),
        ActionType::Publish => Some("Pulse.publish".into()),
        ActionType::Validate => Some("SHACL.validate".into()),
        ActionType::Invoke | ActionType::Navigate | ActionType::Annotate | ActionType::Cancel => {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// IntentBus impl
// ---------------------------------------------------------------------------

impl IntentBus for WasmIntentBus {
    type Error = WasmBusError;

    async fn dispatch<P>(
        &self,
        mut payload: VibeScriptPayload<P>,
    ) -> Result<IntentReceipt, Self::Error>
    where
        P: serde::Serialize + Send + Sync,
    {
        // Stamp provenance
        self.stamp_provenance(&mut payload);
        let dispatch_id = payload.provenance.as_ref().unwrap().intent_counter;

        // Capability gate: fail closed unless the loopback daemon was probed.
        let Some(base) = super::native_daemon::get_connected_daemon_url() else {
            return Ok(IntentReceipt {
                dispatch_id,
                status: IntentStatus::Rejected("local QualiaDB daemon is not connected".into()),
                provenance: payload.provenance,
            });
        };

        // Encode to CBOR-LD
        let cbor_bytes = self.encode_cbor_ld(&payload)?;

        // Route to daemon
        let url = format!("{base}/intent");

        // Use fetch API
        let window = web_sys::window().unwrap();
        let opts = web_sys::RequestInit::new();
        opts.set_method("POST");
        opts.set_mode(web_sys::RequestMode::Cors);

        // Body: CBOR-LD bytes as ArrayBuffer
        let array = js_sys::Uint8Array::from(&cbor_bytes[..]);
        let body: &wasm_bindgen::JsValue = array.as_ref();
        opts.set_body(body);

        let request = web_sys::Request::new_with_str_and_init(&url, &opts)
            .map_err(|e| WasmBusError::FetchError(format!("{:?}", e)))?;

        let headers = request.headers();
        headers
            .set("Content-Type", "application/cbor-ld")
            .map_err(|e| WasmBusError::FetchError(format!("{:?}", e)))?;

        let promise = window.fetch_with_request(&request);
        let response_value = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| WasmBusError::FetchError(format!("{:?}", e)))?;
        let response: web_sys::Response = response_value
            .dyn_into()
            .map_err(|_| WasmBusError::FetchError("invalid daemon response".into()))?;
        if !response.ok() {
            let diagnostic = match response.text() {
                Ok(text) => wasm_bindgen_futures::JsFuture::from(text)
                    .await
                    .ok()
                    .and_then(|value| value.as_string())
                    .unwrap_or_else(|| format!("daemon returned HTTP {}", response.status())),
                Err(_) => format!("daemon returned HTTP {}", response.status()),
            };
            return Ok(IntentReceipt {
                dispatch_id,
                status: IntentStatus::Rejected(diagnostic),
                provenance: payload.provenance,
            });
        }

        Ok(IntentReceipt {
            dispatch_id,
            status: IntentStatus::Accepted,
            provenance: payload.provenance,
        })
    }

    async fn cancel(&self, dispatch_id: u64) -> Result<IntentReceipt, Self::Error> {
        Ok(IntentReceipt {
            dispatch_id,
            status: IntentStatus::Rejected(
                "No cancellable native job contract is registered for this dispatch".into(),
            ),
            provenance: None,
        })
    }
}

// Safety: WasmIntentBus uses AtomicU64 which is Send+Sync.
// The bus is single-threaded in WASM but the trait requires Send+Sync.
unsafe impl Send for WasmIntentBus {}
unsafe impl Sync for WasmIntentBus {}
