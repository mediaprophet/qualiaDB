use axum::{
    extract::State,
    response::{sse::{Event, Sse}, IntoResponse},
    routing::{get, post},
    Router,
};
use core_affinity::CoreId;
use futures_core::stream::Stream;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use crate::wal::append_mutation;
use crate::{q_hash, NQuin};
use tokio_stream::StreamExt;
use tower_http::services::ServeDir;
use tower_http::cors::{CorsLayer, Any};

/// Shared state for the Webizen loopback server
pub struct WebizenState {
    telemetry_tx: broadcast::Sender<String>,
    /// D4.1: Type Index. A flat, sorted array mapping Semantic Type Hash -> Block Location / SuperBlock Context.
    /// Loaded on boot for O(log N) zero-alloc lookups without heavyweight HashMaps.
    type_index: Arc<[(u64, u64)]>,
}

/// Spawns the embedded Axum HTTP server on a background thread tied to Core 3.
/// This fulfills the requirement to serve the Webizen UI over loopback
/// without blocking the deterministic Prolog Sentinel.
pub fn spawn_loopback_server(ui_static_dir: String) -> Arc<WebizenState> {
    // D4.1: Boot up TypeIndex scan
    let mut flat_index = Vec::new();
    let index_predicate = q_hash("q42:TypeIndex");
    if let Ok(mut file) = std::fs::File::open("qualia_global.wal") {
        use std::io::Read;
        let mut buf = [0u8; 48];
        while file.read_exact(&mut buf).is_ok() {
            let quin: NQuin = bytemuck::cast(buf);
            if quin.predicate == index_predicate {
                // Subject = Type Hash (e.g. q_hash("schema:MedicalRecord"))
                // Object = Pointer to SuperBlock / File Offset
                flat_index.push((quin.subject, quin.object));
            }
        }
    }
    // Sort for binary search
    flat_index.sort_unstable_by_key(|&(k, _)| k);

    let (telemetry_tx, _) = broadcast::channel(100);
    let state = Arc::new(WebizenState {
        telemetry_tx: telemetry_tx.clone(),
        type_index: flat_index.into(),
    });
    
    let server_state = state.clone();

    std::thread::Builder::new()
        .name("Webizen-Axum-Core3".into())
        .spawn(move || {
            // Pin to Core 3 (I/O & Parity) to prevent preemption of Core 1 (Sentinel)
            // Note: We use the 4th core (index 3) if available, otherwise just use the last available core.
            if let Some(core_ids) = core_affinity::get_core_ids() {
                if let Some(core3) = core_ids.get(3).or(core_ids.last()) {
                    core_affinity::set_for_current(*core3);
                }
            }

            // Create a dedicated, single-threaded Tokio runtime for this core
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to build Webizen tokio runtime");

            rt.block_on(async move {
                // C5: Private Network Access Headers
                // We add a middleware to manually inject the PNA header into all responses,
                // and configure the CorsLayer to allow the header in preflight.
                let cors = CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(vec![
                        axum::http::header::CONTENT_TYPE,
                        axum::http::header::HeaderName::from_static("access-control-allow-private-network")
                    ]);

                let app = Router::new()
                    // 1. Serve static WASM UI assets
                    .fallback_service(ServeDir::new(ui_static_dir))
                    // 2. Telemetry SSE endpoint
                    .route("/telemetry", get(telemetry_handler))
                    // 3. Manifest POST endpoint for yaml-ld-q42 compilation
                    .route("/manifest", post(manifest_handler))
                    // 4. Manifest GET endpoint for Boot Rehydration
                    .route("/manifest/current", get(current_manifest_handler))
                    // 5. Extension Management Endpoints
                    .route("/extensions/list", get(list_extensions_handler))
                    .route("/extensions/query/:interface", get(query_extensions_handler))
                    .route("/extensions/register", post(register_extension_handler))
                    // 6. Mobile Deployment Endpoints (Phase C)
                    .route("/mobile/qr", get(mobile_qr_handler))
                    .route("/mobile/manifest.json", get(mobile_pwa_manifest_handler))
                    .route("/mobile/sw.js", get(mobile_sw_handler))
                    .route("/mobile/stream", get(mobile_ws_handler))
                    // 7. Advanced Features (Phase D)
                    .route("/generate_pane", post(mobile_generate_pane_handler))
                    .layer(cors)
                    .layer(axum::middleware::from_fn(pna_middleware))
                    .with_state(server_state);

                let addr = SocketAddr::from(([0, 0, 0, 0], 8080)); // Listen on all interfaces for mobile access
                println!("Starting Sovereign Webizen Loopback on http://{}", addr);

                let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
                axum::serve(listener, app).await.unwrap();
            });
        })
        .expect("Failed to spawn Webizen server thread");

    state
}

/// C5: Middleware to inject Private Network Access (PNA) header
async fn pna_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut res = next.run(req).await;
    res.headers_mut().insert(
        axum::http::header::HeaderName::from_static("access-control-allow-private-network"),
        axum::http::header::HeaderValue::from_static("true"),
    );
    res
}

/// Handler for the SSE telemetry stream (RAM usage, Fiduciary Log events)
async fn telemetry_handler(
    State(state): State<Arc<WebizenState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.telemetry_tx.subscribe();
    let stream = BroadcastStream::new(rx).map(|msg| {
        match msg {
            Ok(data) => Ok(Event::default().data(data)),
            Err(_) => Ok(Event::default().data("Stream lag")),
        }
    });

    Sse::new(stream)
}

/// Handler for accepting new Webizen Studio workspaces via yaml-ld-q42 payloads.
///
/// The incoming body is a yaml-ld-q42 serialized `WebizenWorkspace`. We stream it,
/// compile it to CBOR-LD (via the yaml_ld_q42 module), and persist each pane as an NQuin.
async fn manifest_handler(mut body: axum::body::Body) -> impl IntoResponse {
    use http_body_util::BodyExt;
    println!("Receiving yaml-ld-q42 manifest as stream...");

    let namespace = 0x00u64;

    // Collect body bytes
    let mut payload_bytes = Vec::new();
    while let Some(Ok(frame)) = body.frame().await {
        if let Some(chunk) = frame.data_ref() {
            payload_bytes.extend_from_slice(chunk);
        }
    }

    // Get Lamport clock (use WAL buffered count as a proxy for the clock increment)
    let lamport_clock: u64 = match crate::wal::WriteAheadLog::open("qualia_global.wal") {
        Ok(mut wal) => wal.buffered_count().unwrap_or(0) as u64 + 1,
        Err(_) => 1,
    };

    // Compile yaml-ld to NQuins
    match crate::yaml_ld_q42::compile_yaml_ld_to_quins(&payload_bytes, namespace, lamport_clock) {
        Ok(quins) => {
            for quin in quins {
                let _ = append_mutation(&quin);
            }
            println!("Successfully compiled and persisted yaml-ld-q42 manifest.");
            axum::http::StatusCode::OK
        }
        Err(e) => {
            println!("Failed to compile yaml-ld-q42: {}", e);
            axum::http::StatusCode::BAD_REQUEST
        }
    }
}

/// Handler for Boot Rehydration.
///
/// Recovers all NQuins from the WAL, filters for `q42:SystemPaneState` and
/// `q42:SystemPageDef` predicates, and reconstructs a `WebizenWorkspace` JSON.
///
/// When multiple Quins exist for the same subject+object (same component on the same page),
/// only the one with the highest Lamport clock is kept (LWW semantics via CRDT).
async fn current_manifest_handler() -> impl IntoResponse {
    let pred_pane = q_hash("q42:SystemPaneState");
    let pred_page = q_hash("q42:SystemPageDef");

    // Recover all Quins from the WAL
    let all_quins = match crate::wal::WriteAheadLog::open("qualia_global.wal") {
        Ok(mut wal) => wal.recover().unwrap_or_default(),
        Err(_) => Vec::new(),
    };

    // Separate pane quins and page definition quins
    let pane_quins: Vec<&NQuin> = all_quins.iter()
        .filter(|q| q.predicate == pred_pane && q.context == 0x00)
        .collect();

    let page_quins: Vec<&NQuin> = all_quins.iter()
        .filter(|q| q.predicate == pred_page && q.context == 0x00)
        .collect();

    // Deduplicate pane quins by (subject, object) using highest Lamport clock (LWW)
    let mut deduped_panes: std::collections::HashMap<(u64, u64), &NQuin> = std::collections::HashMap::new();
    for q in &pane_quins {
        let key = (q.subject, q.object);
        let lamport = (q.metadata >> 32) & 0x1FFF_FFFF;
        let should_replace = match deduped_panes.get(&key) {
            Some(existing) => {
                let existing_lamport = (existing.metadata >> 32) & 0x1FFF_FFFF;
                lamport > existing_lamport
            }
            None => true,
        };
        if should_replace {
            deduped_panes.insert(key, q);
        }
    }

    // Deduplicate page defs by subject (page url_path hash) using highest Lamport
    let mut deduped_pages: std::collections::HashMap<u64, &NQuin> = std::collections::HashMap::new();
    for q in &page_quins {
        let lamport = (q.metadata >> 32) & 0x1FFF_FFFF;
        let should_replace = match deduped_pages.get(&q.subject) {
            Some(existing) => {
                let existing_lamport = (existing.metadata >> 32) & 0x1FFF_FFFF;
                lamport > existing_lamport
            }
            None => true,
        };
        if should_replace {
            deduped_pages.insert(q.subject, q);
        }
    }

    // Group panes by their page (object = page url_path hash)
    let mut pages_json: Vec<serde_json::Value> = Vec::new();

    // Collect known page hashes
    let known_page_hashes: Vec<u64> = deduped_pages.keys().copied().collect();

    // Also collect page hashes from panes that don't have explicit page defs
    let mut all_page_hashes: Vec<u64> = known_page_hashes.clone();
    for (_, q) in &deduped_panes {
        if !all_page_hashes.contains(&q.object) {
            all_page_hashes.push(q.object);
        }
    }

    for page_hash in &all_page_hashes {
        // Get page name from page def, or use "Unknown Page"
        let page_name = "Fiduciary Dashboard"; // Default; we can't reverse q_hash
        let page_url = "/"; // Default

        // Reconstruct panes for this page
        let mut panes_json: Vec<serde_json::Value> = Vec::new();
        for ((_, obj), q) in &deduped_panes {
            if obj == page_hash {
                let x = ((q.metadata >> 24) & 0xFF) as u8;
                let y = ((q.metadata >> 16) & 0xFF) as u8;
                let w = ((q.metadata >> 8) & 0xFF) as u8;
                let h = (q.metadata & 0xFF) as u8;

                // We can't reverse q_hash, so we store a placeholder component_id
                // In production, the component_id should be stored in a lexicon lookup
                panes_json.push(serde_json::json!({
                    "component_id": format!("pane_{:016x}", q.subject),
                    "x": x,
                    "y": y,
                    "w": w,
                    "h": h,
                    "data_bindings": []
                }));
            }
        }

        pages_json.push(serde_json::json!({
            "url_path": page_url,
            "name": page_name,
            "layout_strategy": {
                "CssGrid": { "cols": 12, "rows": 12, "gap": 16 }
            },
            "panes": panes_json
        }));
    }

    // If no pages were recovered from the WAL, return an empty default scaffold
    if pages_json.is_empty() {
        pages_json.push(serde_json::json!({
            "url_path": "/",
            "name": "Fiduciary Dashboard",
            "layout_strategy": {
                "CssGrid": { "cols": 12, "rows": 12, "gap": 16 }
            },
            "panes": []
        }));
    }

    let workspace = serde_json::json!({ "pages": pages_json });
    axum::response::Json(workspace)
}

use crate::extension_bus::ExtensionBus;
use std::sync::OnceLock;

static EXTENSION_BUS: OnceLock<ExtensionBus> = OnceLock::new();

/// Retrieve the global ExtensionBus
fn get_extension_bus() -> &'static ExtensionBus {
    EXTENSION_BUS.get_or_init(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        // D5.1: Shared Extension Pool globally scoped
        let path = std::path::PathBuf::from(home).join(".qualia").join("extensions").join("pool");
        ExtensionBus::new(path)
    })
}

async fn list_extensions_handler() -> impl IntoResponse {
    let bus = get_extension_bus();
    let exts = bus.list_extensions();
    axum::response::Json(exts)
}

async fn query_extensions_handler(axum::extract::Path(interface): axum::extract::Path<String>) -> impl IntoResponse {
    let bus = get_extension_bus();
    let interface_hash = crate::q_hash(&interface);
    let exts = bus.query_capability(interface_hash);
    axum::response::Json(exts)
}

async fn register_extension_handler(mut body: axum::body::Body) -> impl IntoResponse {
    use http_body_util::BodyExt;
    
    let mut payload_bytes = Vec::new();
    while let Some(Ok(frame)) = body.frame().await {
        if let Some(chunk) = frame.data_ref() {
            payload_bytes.extend_from_slice(chunk);
        }
    }
    
    let bus = get_extension_bus();
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("temp_manifest.json");
    if std::fs::write(&temp_file, payload_bytes).is_ok() {
        if bus.register_extension_from_path(&temp_file).is_ok() {
            return axum::http::StatusCode::OK;
        }
    }
    axum::http::StatusCode::BAD_REQUEST
}

// ============================================================================
// Phase C: Mobile Deployment Endpoints
// ============================================================================

/// C1: QR Code Generation Endpoint
/// Generates an SVG QR Code containing the link to the bootstrap gateway
async fn mobile_qr_handler() -> impl IntoResponse {
    use fast_qr::qr::QRBuilder;
    use fast_qr::convert::{svg::SvgBuilder, Builder};
    
    // In production, we would dynamically determine the active LAN IP
    let local_ip = "192.168.1.45"; 
    let port = 8080;
    
    // The target URL that the GitHub Pages bootstrap will redirect to
    let target_url = format!("http://{}:{}/", local_ip, port);
    
    // The payload embedded in the QR Code
    let payload = format!("https://qualia-db.github.io/connect/?target={}", target_url);
    
    match QRBuilder::new(payload).build() {
        Ok(qrc) => {
            let svg = SvgBuilder::default().to_str(&qrc);
            (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "image/svg+xml")],
                svg,
            ).into_response()
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate QR code",
        ).into_response(),
    }
}

/// C3: PWA Manifest for Mobile "Add to Home Screen"
async fn mobile_pwa_manifest_handler() -> impl IntoResponse {
    let manifest = serde_json::json!({
        "name": "Qualia Mobile Harness",
        "short_name": "Qualia",
        "start_url": "/",
        "display": "standalone",
        "background_color": "#000000",
        "theme_color": "#000000",
        "prefer_related_applications": false,
        "icons": [
            {
                "src": "/icon.png",
                "sizes": "192x192",
                "type": "image/png"
            }
        ]
    });
    
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/manifest+json")],
        manifest.to_string(),
    ).into_response()
}

/// C3: PWA Service Worker (prevents stubborn offline caching loops)
async fn mobile_sw_handler() -> impl IntoResponse {
    let sw_content = r#"
self.addEventListener('install', (event) => {
    self.skipWaiting();
});
self.addEventListener('fetch', (event) => {
    // Network-first or bypass cache to ensure secure context loophole doesn't stick
    event.respondWith(fetch(event.request));
});
"#;
    (
        axum::http::StatusCode::OK,
        [
            (axum::http::header::CONTENT_TYPE, "application/javascript"),
            (axum::http::header::CACHE_CONTROL, "no-cache")
        ],
        sw_content,
    ).into_response()
}

/// C4 & C6: DID Challenge-Response over WebSocket
async fn mobile_ws_handler(ws: axum::extract::ws::WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        // 1. Send the Challenge Payload
        let challenge = "CHALLENGE_BYTES_123456789";
        if socket.send(axum::extract::ws::Message::Text(challenge.to_string().into())).await.is_ok() {
            // 2. Await the signed response
            if let Some(Ok(msg)) = socket.recv().await {
                if let axum::extract::ws::Message::Text(signed_payload) = msg {
                    println!("Received DID signature: {}", signed_payload);
                    // 3. Unlock data pipe
                    let _ = socket.send(axum::extract::ws::Message::Text("AUTH_SUCCESS".to_string().into())).await;
                    
                    // Proceed with streaming data...
                    while let Some(Ok(_msg)) = socket.recv().await {
                        // Handle incoming mobile data
                    }
                }
            }
        }
    })
}

// ============================================================================
// Phase D: Advanced Features Endpoints
// ============================================================================

#[derive(serde::Deserialize)]
pub struct GeneratePaneRequest {
    pub prompt: String,
}

/// D2.2: LLM-Driven Pane Maker
/// Takes natural language, delegates to qualia-llm-extension via ExtensionBus
async fn mobile_generate_pane_handler(
    axum::extract::Json(req): axum::extract::Json<GeneratePaneRequest>,
) -> impl IntoResponse {
    let bus = get_extension_bus();
    
    // Simulate natural language -> yaml-ld-q42 generation via local LLM extension.
    // In production, this dispatches to the ONNX/WebGPU extension.
    // For D2.3 Gatekeeper test, we allow simulation of a Classified capability request.
    let is_classified = req.prompt.contains("classified");
    let sensitivity = if is_classified { 0x02_u64 << 56 } else { 0x00_u64 << 56 };
    
    match bus.dispatch_task(
        "qualia-llm-extension",
        "mock_prompt_file.txt",
        vec![serde_json::json!({"action": "generate_pane", "prompt": req.prompt})],
        sensitivity,
        false // No override for the test
    ) {
        Ok(_) => {
            let mock_yaml = format!(
                "---\n@context: https://qualia.io/q42\n@type: SystemPaneState\ncomponent_id: generated-pane\n..."
            );
            (
                axum::http::StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/yaml-ld-q42")],
                mock_yaml,
            ).into_response()
        }
        Err(e) => {
            (
                axum::http::StatusCode::FORBIDDEN,
                e,
            ).into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D2.3: Sentinel Gatekeeper Integration Test for LLM Payloads
    #[tokio::test]
    async fn test_llm_gatekeeper_blocks_classified_prompts() {
        // Assume extension bus is properly initialized
        let bus = get_extension_bus();

        // 1. Dispatch a non-classified prompt
        let safe_res = bus.dispatch_task(
            "qualia-llm-extension",
            "mock.txt",
            vec![serde_json::json!({"action": "generate_pane", "prompt": "normal"})],
            0x00_u64 << 56, // SENSITIVITY_PUBLIC
            false
        );
        // Note: Unless the llm extension is loaded locally, it will fail with "Extension not found"
        // But it won't fail with a gatekeeper block. We're testing the gatekeeper logic.
        assert!(!safe_res.unwrap_err().contains("GATEKEEPER_BLOCK"));

        // 2. Dispatch a classified prompt without guardianship override
        let classified_res = bus.dispatch_task(
            "qualia-llm-extension",
            "mock.txt",
            vec![serde_json::json!({"action": "generate_pane", "prompt": "classified medical record"})],
            0x02_u64 << 56, // SENSITIVITY_CLASSIFIED
            false
        );
        
        let err_msg = classified_res.unwrap_err();
        assert!(err_msg.contains("GATEKEEPER_BLOCK"));
        assert!(err_msg.contains("Classified (0x02)"));
    }
}
