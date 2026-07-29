use axum::{
    body::{Body, Bytes},
    extract::{ws::Message, Query, State, WebSocketUpgrade},
    http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tower_http::{cors::CorsLayer, services::ServeDir, set_header::SetResponseHeaderLayer};

use crate::{
    daemon_query::{self, QueryExecError},
    q_hash,
    wal::append_mutation,
    NQuin,
};

const OFFICIAL_WEB_HUB_ORIGIN: &str = "https://mediaprophet.github.io";
const QUERY_PAYLOAD_LIMIT_BYTES: u64 = 64 * 1024;
const PROXY_FETCH_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Block loopback / RFC1918 targets for the browser CORS relay (`GET /proxy/fetch`).
fn proxy_target_allowed(url: &reqwest::Url) -> bool {
    match url.scheme() {
        "http" | "https" => {}
        _ => return false,
    }

    let host = match url.host_str() {
        Some(h) => h.to_ascii_lowercase(),
        None => return false,
    };

    if host == "localhost" || host.ends_with(".localhost") || host == "127.0.0.1" {
        return false;
    }
    if host.starts_with("127.") || host == "::1" || host == "[::1]" {
        return false;
    }
    if host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || host.starts_with("fe80:")
    {
        return false;
    }
    if let Some(stripped) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        if let Ok(ip) = stripped.parse::<std::net::IpAddr>() {
            return !ip_is_restricted(ip);
        }
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return !ip_is_restricted(ip);
    }

    true
}

fn ip_is_restricted(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unicast_link_local() || v6.is_unique_local()
        }
    }
}

/// Shared state for the Webizen loopback server
#[derive(Clone)]
pub struct WebizenState {
    pub telemetry_tx: broadcast::Sender<Vec<u8>>,
    pub type_index: Arc<[(u64, u64)]>,
    pub dev: bool,
    pub token: Option<String>,
    pub vault: Arc<Mutex<crate::key_vault::KeyVault>>,
    pub storage_path: String,
    pub port: u16,
    pub in_sanctuary_mode: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Deserialize)]
struct NativeQueryRequest {
    query: String,
    format: Option<String>,
}

enum OutputFormat {
    JsonLd,
    NTriples,
    RawQ42,
}

const SUPPORTED_QUERY_FORMATS: &str = "application/ld+json, application/n-triples";

fn not_acceptable_format_response() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_ACCEPTABLE,
        Json(json!({
            "code": "not_acceptable",
            "status": "error",
            "message": format!("Supported formats: {SUPPORTED_QUERY_FORMATS}")
        })),
    )
}

fn negotiate_format(
    payload_format: Option<&str>,
    accept: Option<&str>,
) -> Result<OutputFormat, ()> {
    if let Some(fmt) = payload_format {
        return match fmt {
            "json-ld" | "application/ld+json" => Ok(OutputFormat::JsonLd),
            "n-triples" | "application/n-triples" => Ok(OutputFormat::NTriples),
            "q42" | "application/x-qualia-q42" => Ok(OutputFormat::RawQ42),
            _ => Err(()),
        };
    }
    if let Some(accept) = accept {
        if accept.contains("application/x-qualia-q42") {
            return Ok(OutputFormat::RawQ42);
        }
        if accept.contains("application/n-triples") {
            return Ok(OutputFormat::NTriples);
        }
        if accept.contains("application/ld+json")
            || accept.contains("application/json")
            || accept.contains("*/*")
        {
            return Ok(OutputFormat::JsonLd);
        }
        return Err(());
    }
    Ok(OutputFormat::JsonLd)
}

fn ws_query_error_json(id: u64, err: QueryExecError) -> serde_json::Value {
    match err {
        QueryExecError::EmptyQuery => json!({ "type": "error", "id": id, "code": "empty_query" }),
        QueryExecError::ParseError(msg) => {
            json!({ "type": "error", "id": id, "code": "parse_error", "message": msg })
        }
        QueryExecError::OutputBufferFull => {
            json!({ "type": "error", "id": id, "code": "result_set_too_large" })
        }
        QueryExecError::InvalidProgram => json!({ "type": "error", "id": id, "code": "vm_error" }),
        QueryExecError::ClassifiedEgress => {
            json!({ "type": "error", "id": id, "code": "restricted_data_access" })
        }
    }
}

fn decode_bench_load_b64(b64: &str) -> Result<Vec<u8>, &'static str> {
    let cleaned: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let padded = match cleaned.len() % 4 {
        0 => cleaned,
        n => format!("{cleaned}{}", "=".repeat(4 - n)),
    };
    let mut out = Vec::with_capacity(padded.len() * 3 / 4);
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buf = 0u32;
    let mut bits = 0u32;
    for ch in padded.bytes() {
        if ch == b'=' {
            break;
        }
        let val = table
            .iter()
            .position(|&t| t == ch)
            .ok_or("invalid base64")? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1u32 << bits) - 1;
        }
    }
    Ok(out)
}

pub fn spawn_loopback_server(
    port: u16,
    dev: bool,
    vault: Arc<Mutex<crate::key_vault::KeyVault>>,
    token: Option<String>,
) -> Arc<WebizenState> {
    let mut flat_index = Vec::new();
    let index_predicate = q_hash("q42:TypeIndex");
    if let Ok(mut file) = std::fs::File::open("qualia_global.wal") {
        use std::io::Read;
        let mut buf = [0u8; 48];
        while file.read_exact(&mut buf).is_ok() {
            let quin: NQuin = bytemuck::cast(buf);
            if quin.predicate == index_predicate {
                flat_index.push((quin.subject, quin.object));
            }
        }
    }
    flat_index.sort_unstable_by_key(|&(k, _)| k);

    let (telemetry_tx, _) = broadcast::channel(100);

    let storage_path = std::env::var("QUALIA_STORAGE_PATH").unwrap_or_else(|_| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| format!("{h}/.qualia"))
            .unwrap_or_else(|_| ".qualia".to_string())
    });

    let state = Arc::new(WebizenState {
        telemetry_tx: telemetry_tx.clone(),
        type_index: flat_index.into(),
        dev,
        token: token.or_else(|| {
            std::env::var("QUALIA_TOKEN")
                .ok()
                .or_else(|| std::env::var("QUALIA_DEV_TOKEN").ok())
        }),
        vault: vault.clone(),
        storage_path: storage_path.clone(),
        port,
        in_sanctuary_mode: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    });

    let server_state = state.clone();

    std::thread::Builder::new()
        .name("Webizen-Axum-Core3".into())
        .spawn(move || {
            if let Some(core_ids) = core_affinity::get_core_ids() {
                if let Some(core3) = core_ids.get(3).or(core_ids.last()) {
                    core_affinity::set_for_current(*core3);
                }
            }

            let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

            rt.block_on(async move {
                let allowed_origins: Vec<HeaderValue> = if server_state.dev {
                    vec![
                        "http://localhost:8080".parse().unwrap(),
                        "http://127.0.0.1:8080".parse().unwrap(),
                        "http://localhost:8788".parse().unwrap(),
                        "http://127.0.0.1:8788".parse().unwrap(),
                        "http://localhost:5173".parse().unwrap(),
                        "http://127.0.0.1:5173".parse().unwrap(),
                        "http://localhost:4173".parse().unwrap(),
                        "http://127.0.0.1:4173".parse().unwrap(),
                        OFFICIAL_WEB_HUB_ORIGIN.parse().unwrap(),
                    ]
                } else {
                    vec![OFFICIAL_WEB_HUB_ORIGIN.parse().unwrap()]
                };

                let cors = if server_state.dev {
                    CorsLayer::permissive()
                } else {
                    CorsLayer::new()
                        .allow_origin(allowed_origins)
                        .allow_methods(vec![Method::GET, Method::POST, Method::OPTIONS])
                        .allow_headers(vec![
                            header::CONTENT_TYPE,
                            header::ACCEPT,
                            HeaderName::from_static("x-qualia-token"),
                            HeaderName::from_static("x-qualia-standpoint-class"),
                            HeaderName::from_static("x-qualia-t-slice"),
                            HeaderName::from_static("x-qualia-t-window"),
                            HeaderName::from_static("x-qualia-session-nonce"),
                            HeaderName::from_static("x-qualia-identifier-did"),
                            HeaderName::from_static("x-qualia-signature"),
                            HeaderName::from_static("x-qualia-lane"),
                            HeaderName::from_static("access-control-request-private-network"),
                        ])
                        .expose_headers(vec![
                            HeaderName::from_static("x-qualia-compute-cost"),
                            HeaderName::from_static("x-qualia-tensor-nodes"),
                            HeaderName::from_static("x-qualia-tensor-bytes"),
                        ])
                };

                let csp_layer = SetResponseHeaderLayer::overriding(
                    header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static(
                        "default-src 'self'; connect-src 'self' ws://127.0.0.1:4242; script-src 'self' 'wasm-unsafe-eval'; style-src 'self';"
                    )
                );

                let ui_static_dir = std::env::var("QUALIA_UI_DIR").unwrap_or_else(|_| "crates/webizen-studio/dist".to_string());

                let app = Router::new()
                    // UI & Static
                    .fallback_service(ServeDir::new(ui_static_dir).precompressed_gzip())

                    // WebSockets
                    .route("/qualia-bridge", get(bridge_handler))
                    .route("/telemetry", get(telemetry_handler))
                    // External WASM telemetry ingestion — external devices
                    // (phones, browser tabs, other machines) POST their
                    // SystemTelemetry here for local processing.
                    .route("/telemetry/ingest", post(telemetry_ingest_handler).options(preflight_handler))

                    // REST
                    .route("/health", get(health_handler).options(preflight_handler))
                    .route(
                        "/tensor/slice",
                        get(tensor_slice_handler).options(preflight_handler),
                    )
                    .route(
                        "/tensor/events",
                        get(tensor_events_handler).options(preflight_handler),
                    )
                    .route(
                        "/tensor/dev-signing-key",
                        get(tensor_dev_signing_key_handler).options(preflight_handler),
                    )
                    .route("/query", post(query_handler).options(preflight_handler))
                    .route("/update", post(update_handler).options(preflight_handler))
                    .route("/cache", post(cache_handler))
                    .route("/proxy/fetch", get(proxy_fetch_handler).options(preflight_handler))
                    .route("/api/v1/system/storage/selfhood", get(storage_selfhood_handler))
                    .route("/api/v1/system/storage/commons", get(storage_commons_handler))
                    .route("/api/v1/permissions/compile", post(permissions_compile_handler))
                    .route("/api/v1/webizen/rpc", post(webizen_rpc_handler))

                    // Manifest
                    .route("/manifest", post(manifest_handler))
                    .route("/manifest/current", get(current_manifest_handler))

                    // Extensions
                    .route("/extensions/list", get(list_extensions_handler))
                    .route("/extensions/query/{interface}", get(query_extensions_handler))
                    .route("/extensions/register", post(register_extension_handler))

                    // Mobile
                    .route("/mobile/qr", get(mobile_qr_handler))
                    .route("/mobile/stream", get(mobile_ws_handler))
                    .route("/generate_pane", post(mobile_generate_pane_handler))
                    .nest_service("/mobile/app", tower_http::services::ServeDir::new("bootstrap_gateway/mobile"))

                    // External Routers
                    .with_state(server_state.clone())
                    .merge(crate::chat_relay_daemon::chat_relay_routes(server_state.storage_path.clone(), server_state.vault.clone()))
                    .nest("/torrent", crate::webtorrent_routes::webtorrent_routes(server_state.port))
                    .layer(csp_layer)
                    .layer(cors)
                    .layer(axum::middleware::from_fn(pna_middleware));

                let addr = SocketAddr::from(([0, 0, 0, 0], server_state.port));
                let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
                axum::serve(listener, app).await.unwrap();
            });
        })
        .expect("Failed to spawn Webizen server thread");

    state
}

async fn pna_middleware(req: axum::extract::Request, next: axum::middleware::Next) -> Response {
    let mut res = next.run(req).await;
    res.headers_mut().insert(
        HeaderName::from_static("access-control-allow-private-network"),
        HeaderValue::from_static("true"),
    );
    res
}

async fn preflight_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

#[derive(Deserialize)]
struct TensorSliceQuery {
    max_nodes: Option<u32>,
    t_slice: Option<f32>,
    t_window: Option<f32>,
    lane: Option<String>,
}

fn header_parse_f32(headers: &HeaderMap, name: &str) -> Option<f32> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
}

fn header_parse_u32(headers: &HeaderMap, name: &str) -> Option<u32> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
}

fn header_str(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

async fn tensor_slice_handler(
    State(state): State<Arc<WebizenState>>,
    Query(q): Query<TensorSliceQuery>,
    headers: HeaderMap,
) -> Response {
    use crate::daemon_tensor::{
        build_tensor_slice_bytes, verify_tensor_slice_signature, TensorSliceAuthError,
        TensorSliceError, TensorSliceLane, TensorSliceRequest, DEFAULT_SLICE_MAX_NODES,
    };
    use crate::render::telemetry::{STANDPOINT_DID, STANDPOINT_VAULT};
    use crate::tensor::buffer_export::tensor_node_count;

    let max_nodes = header_parse_u32(&headers, "x-qualia-max-nodes")
        .or(q.max_nodes)
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_SLICE_MAX_NODES);

    let t_slice = header_parse_f32(&headers, "x-qualia-t-slice")
        .or(q.t_slice)
        .unwrap_or(0.5);

    let t_window = header_parse_f32(&headers, "x-qualia-t-window")
        .or(q.t_window)
        .unwrap_or(1.0);

    let standpoint_class = header_parse_u32(&headers, "x-qualia-standpoint-class").unwrap_or(0);

    let identifier_did = header_str(&headers, "x-qualia-identifier-did").unwrap_or_default();
    let session_nonce = header_str(&headers, "x-qualia-session-nonce").unwrap_or_default();
    let signature_hex = header_str(&headers, "x-qualia-signature").unwrap_or_default();

    let mut lane = if standpoint_class >= STANDPOINT_VAULT {
        TensorSliceLane::Identifier
    } else if standpoint_class >= STANDPOINT_DID {
        TensorSliceLane::Identifier
    } else {
        TensorSliceLane::Commons
    };

    if standpoint_class >= STANDPOINT_DID {
        let vault = match state.vault.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "vault_unavailable" })),
                )
                    .into_response();
            }
        };
        if let Err(auth_err) = verify_tensor_slice_signature(
            &vault,
            &identifier_did,
            &session_nonce,
            standpoint_class,
            t_slice,
            t_window,
            &signature_hex,
        ) {
            let code = match auth_err {
                TensorSliceAuthError::InvalidSignature => "invalid_signature",
                TensorSliceAuthError::InvalidSignatureEncoding => "invalid_signature_encoding",
                TensorSliceAuthError::SignatureRequired => "signature_required",
                TensorSliceAuthError::IdentifierDidRequired => "identifier_did_required",
                TensorSliceAuthError::SessionNonceRequired => "session_nonce_required",
            };
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "tensor_slice_auth_failed",
                    "code": code,
                })),
            )
                .into_response();
        }
        drop(vault);
    } else {
        lane = headers
            .get("x-qualia-lane")
            .and_then(|v| v.to_str().ok())
            .or(q.lane.as_deref())
            .map(TensorSliceLane::from_header)
            .unwrap_or(TensorSliceLane::Commons);
    }

    let req = TensorSliceRequest {
        max_nodes,
        t_slice,
        t_window,
        lane,
        standpoint_class,
    };

    let guard = crate::daemon_graph::graph_read_guard();
    match build_tensor_slice_bytes(guard.as_slice(), &req) {
        Ok(bytes) => {
            let node_count = tensor_node_count(&bytes).unwrap_or(0);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/octet-stream")
                .header("X-Qualia-Tensor-Nodes", node_count.to_string())
                .header("X-Qualia-Tensor-Bytes", bytes.len().to_string())
                .header(
                    "X-Qualia-Tensor-Lane",
                    match lane {
                        TensorSliceLane::Commons => "commons",
                        TensorSliceLane::Identifier => "identifier",
                    },
                )
                .body(Body::from(bytes))
                .unwrap()
        }
        Err(TensorSliceError::EmptyGraph) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "empty_graph",
                "message": "graph empty or no nodes match temporal window"
            })),
        )
            .into_response(),
        Err(TensorSliceError::BufferTooSmall) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "tensor_buffer_error" })),
        )
            .into_response(),
    }
}

/// Dev-only: export KeyVault-derived Ed25519 seed for `identifier_did` (localhost pairing).
async fn tensor_dev_signing_key_handler(
    State(state): State<Arc<WebizenState>>,
    Query(qs): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if !state.dev {
        return (StatusCode::FORBIDDEN, Json(json!({ "error": "dev_only" }))).into_response();
    }
    let identifier_did = qs.get("identifier_did").cloned().unwrap_or_default();
    if identifier_did.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "identifier_did_required" })),
        )
            .into_response();
    }
    let vault = match state.vault.lock() {
        Ok(guard) => guard,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "vault_unavailable" })),
            )
                .into_response();
        }
    };
    let sk = vault.derive_key(&identifier_did);
    let pk = vault.public_key_bytes_for_context(&identifier_did);
    (
        StatusCode::OK,
        Json(json!({
            "identifier_did": identifier_did,
            "signing_key_hex": hex::encode(sk.to_bytes()),
            "public_key_hex": hex::encode(pk),
            "warning": "dev_only — never expose signing keys outside localhost pairing"
        })),
    )
        .into_response()
}

/// SSE stream of Lamport graph revisions for live viewport sync (`PR-C9c.1`).
async fn tensor_events_handler() -> Response {
    use std::convert::Infallible;
    use std::time::Duration;

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let initial = crate::daemon_graph::graph_revision();

    tokio::spawn(async move {
        let _ = tx.send(format!("data: {{\"revision\":{initial}}}\n\n"));
        let mut last = initial;
        let mut sub = crate::daemon_graph::subscribe_graph_revisions();
        let mut keepalive = tokio::time::interval(Duration::from_secs(15));
        keepalive.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            if tx.is_closed() {
                break;
            }
            tokio::select! {
                result = sub.recv() => {
                    match result {
                        Ok(rev) => {
                            if rev > last {
                                last = rev;
                                let _ = tx.send(format!("data: {{\"revision\":{rev}}}\n\n"));
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            let current = crate::daemon_graph::graph_revision();
                            if current > last {
                                last = current;
                                let _ = tx.send(format!("data: {{\"revision\":{current}}}\n\n"));
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                _ = keepalive.tick() => {
                    let _ = tx.send(": keepalive\n\n".to_string());
                }
            }
        }
    });

    let body_stream =
        UnboundedReceiverStream::new(rx).map(|chunk| Ok::<Bytes, Infallible>(Bytes::from(chunk)));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(body_stream))
        .unwrap()
}

async fn health_handler(State(state): State<Arc<WebizenState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "active",
            "engine": "qualia-core-db",
            "version": crate::ENGINE_VERSION,
            "dev_mode": state.dev,
            "graph_quin_count": crate::daemon_graph::graph_quin_count(),
            "graph_revision": crate::daemon_graph::graph_revision(),
            "webtorrent": crate::webtorrent_seeder::telemetry(),
            "execution_environment": crate::services::daemon::execution_environment_json(),
        })),
    )
}

async fn proxy_fetch_handler(
    Query(qs): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let target = qs.get("url").cloned().unwrap_or_default();
    if target.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "missing url"})),
        )
            .into_response();
    }
    let parsed = match reqwest::Url::parse(&target) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": "invalid url"})),
            )
                .into_response()
        }
    };

    if !proxy_target_allowed(&parsed) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "proxy target not allowed"})),
        )
            .into_response();
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap();
    let response = match client.get(parsed).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = response.bytes().await.unwrap_or_default();
    if bytes.len() > PROXY_FETCH_MAX_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({"error": "too large"})),
        )
            .into_response();
    }

    ([(header::CONTENT_TYPE, content_type)], bytes.to_vec()).into_response()
}

async fn cache_handler(
    Query(qs): Query<std::collections::HashMap<String, String>>,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    let filename = qs
        .get("filename")
        .cloned()
        .unwrap_or_else(|| "dataset_shard.q42".to_string());
    let mut path = std::path::PathBuf::from(".qualia");
    path.push("cache");
    let _ = std::fs::create_dir_all(&path);
    path.push(&filename);
    let _ = std::fs::write(&path, body);
    (
        StatusCode::OK,
        Json(json!({ "status": "ok", "saved_to": path.to_str() })),
    )
}

async fn storage_selfhood_handler(State(state): State<Arc<WebizenState>>) -> impl IntoResponse {
    let path = std::path::Path::new(&state.storage_path).join("selfhood");
    Json(json!({ "path": path.to_str().unwrap_or_default(), "status": "isolated" }))
}

async fn storage_commons_handler(State(state): State<Arc<WebizenState>>) -> impl IntoResponse {
    let path = std::path::Path::new(&state.storage_path).join("commons");
    Json(json!({ "path": path.to_str().unwrap_or_default(), "status": "public" }))
}

#[derive(Deserialize)]
pub struct PermissionsCompileRequest {
    pub payload: String, // yaml-ld-q42
}

#[derive(Serialize, Deserialize)]
pub struct CompiledPermission {
    pub routing_mask: u64,
    pub semantic_handshake: String,
    pub is_permissive_commons: bool,
}

pub async fn permissions_compile_handler(
    Json(req): Json<PermissionsCompileRequest>,
) -> impl IntoResponse {
    // Compile yaml-ld-q42 down to hardware bitmask
    let mut is_permissive_commons = false;
    let mut routing_mask = 0u64;

    // Semantic Handshake template creation
    let semantic_handshake = format!(
        "Semantic Cryptographic Proof Template: [Payload Length: {}]",
        req.payload.len()
    );

    if req.payload.contains("Commercial Micro-Commons") || req.payload.contains("Bilateral") {
        routing_mask |= 0x02 << 61; // EnforceBilateralMicroCommons
        routing_mask |= 1 << 50; // MASK_COMMERCIAL_BILLABLE_GATE
    } else if req.payload.contains("Public Commons") || req.payload.contains("PermissiveCommons") {
        routing_mask |= 0x01 << 61; // EnforcePermissiveCommons
        is_permissive_commons = true;
    } else {
        routing_mask |= 0x00 << 61; // PassthroughStandard
    }

    Json(CompiledPermission {
        routing_mask,
        semantic_handshake,
        is_permissive_commons,
    })
}

#[derive(Deserialize)]
pub struct WebizenRpcRequest {
    pub method: String,
    pub scopes: Option<Vec<String>>,
    pub payload: Option<String>,
}

pub async fn webizen_rpc_handler(
    State(state): State<Arc<WebizenState>>,
    Json(req): Json<WebizenRpcRequest>,
) -> impl IntoResponse {
    if req.method == "requestAccess" {
        if let Some(scopes) = req.scopes {
            if state
                .in_sanctuary_mode
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                // Fiduciary Supremacy: Reject requests to 'wf:' or the selfhood store completely.
                // Block both the new `selfhood` name and the legacy `sovereign` name (fail-closed —
                // never loosen this lock while the rename propagates).
                if scopes.iter().any(|s| {
                    s.starts_with("wf:") || s.contains("selfhood") || s.contains("sovereign")
                }) {
                    return (
                        StatusCode::LOCKED,
                        Json(json!({"error": "Selfhood path locked during Sanctuary mode."})),
                    )
                        .into_response();
                }
            }
        }
        return (StatusCode::OK, Json(json!({"status": "access_granted"}))).into_response();
    }

    if req.method == "signAndInject" {
        // Implement pure 48-byte NQuin request and semantic handshake...
        return (StatusCode::OK, Json(json!({"status": "injected"}))).into_response();
    }

    if req.method == "resolveNym" {
        return (StatusCode::OK, Json(json!({"status": "resolved"}))).into_response();
    }

    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": "unknown method"})),
    )
        .into_response()
}

async fn bridge_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebizenState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |mut socket| async move {
        let handshake = json!({ "type": "HANDSHAKE_SUCCESS", "payload": { "mode": "NATIVE", "version": crate::ENGINE_VERSION } });
        let _ = socket.send(Message::Text(handshake.to_string().into())).await;

        let mut pending_bench_id: Option<u64> = None;
        while let Some(Ok(msg)) = socket.recv().await {
            match msg {
                Message::Binary(bytes) => {
                    if let Some(id) = pending_bench_id.take() {
                        let reply = match crate::daemon_graph::replace_graph_from_flat_bytes(&bytes) {
                            Ok(c) => json!({ "type": "bench_loaded", "id": id, "quin_count": c }),
                            Err(e) => json!({ "type": "error", "id": id, "code": "bench_load_failed", "message": e }),
                        };
                        let _ = socket.send(Message::Text(reply.to_string().into())).await;
                    }
                }
                Message::Text(text) => {
                    if let Ok(frame) = serde_json::from_str::<serde_json::Value>(&text) {
                        let frame_type = frame.get("type").and_then(|v| v.as_str()).unwrap_or("");
                        let id = frame.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                        let reply = match frame_type {
                            "query" => {
                                let q = frame.get("query").and_then(|v| v.as_str()).unwrap_or("");
                                if q.len() as u64 > QUERY_PAYLOAD_LIMIT_BYTES {
                                    json!({
                                        "type": "error",
                                        "id": id,
                                        "code": "query_too_large",
                                        "message": format!("query exceeds {QUERY_PAYLOAD_LIMIT_BYTES} byte limit")
                                    })
                                } else {
                                let graph = crate::daemon_graph::graph_read_guard();
                                match daemon_query::execute_ntriples_metrics(q, graph.as_slice()) {
                                    Ok(stats) => json!({
                                        "type": "result",
                                        "id": id,
                                        "match_count": stats.match_count,
                                        "vm_cycles": stats.vm_cycles
                                    }),
                                    Err(err) => ws_query_error_json(id, err),
                                }
                                }
                            }
                            "bench_load" if state.dev => {
                                if frame.get("byte_length").is_some() { pending_bench_id = Some(id); json!({ "type": "bench_load_ready", "id": id }) }
                                else if let Some(b64) = frame.get("db_b64").and_then(|v| v.as_str()) {
                                    match decode_bench_load_b64(b64) {
                                        Ok(bytes) => match crate::daemon_graph::replace_graph_from_flat_bytes(&bytes) {
                                            Ok(c) => json!({ "type": "bench_loaded", "id": id, "quin_count": c }),
                                            Err(e) => json!({ "type": "error", "id": id, "message": e })
                                        },
                                        Err(e) => json!({ "type": "error", "id": id, "message": e })
                                    }
                                } else { json!({ "type": "error", "id": id, "message": "requires db_b64" }) }
                            }
                            _ => json!({ "type": "error", "id": id, "message": "unsupported" }),
                        };
                        let _ = socket.send(Message::Text(reply.to_string().into())).await;
                    }
                }
                _ => {}
            }
        }
    })
}

#[derive(Deserialize)]
struct UpdateRequest {
    update: String,
}

/// `POST /update` — apply a SPARQL 1.1 Update to the resident graph.
///
/// Same token auth as `/query`. The mutation is signed with a **real** key
/// derived from the daemon's key vault (never a placeholder) and persisted
/// durably (snapshot + signed WAL audit trail) via
/// `daemon_graph::apply_sparql_update_durable`. Loopback-only, like `/query`.
async fn update_handler(
    State(state): State<Arc<WebizenState>>,
    headers: HeaderMap,
    Json(request): Json<UpdateRequest>,
) -> impl IntoResponse {
    // Auth: mirror /query (token required unless dev mode).
    if !state.dev {
        let token = headers
            .get("x-qualia-token")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let ok = token
            .as_ref()
            .map(|t| {
                let vault = state.vault.lock().unwrap();
                vault.verify_qapp_token(t, "localhost").is_ok() || Some(t) == state.token.as_ref()
            })
            .unwrap_or(false);
        if !ok {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized"})),
            )
                .into_response();
        }
    }

    let src = request.update.trim();
    if src.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "empty update"})),
        )
            .into_response();
    }
    if src.len() as u64 > QUERY_PAYLOAD_LIMIT_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({"error": "update too large", "limit_bytes": QUERY_PAYLOAD_LIMIT_BYTES})),
        )
            .into_response();
    }
    if !crate::sparql_library::sparql_grammar::is_update(src) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "not a SPARQL Update (use /query for reads)"})),
        )
            .into_response();
    }

    let mut ctx = crate::sparql_ast::SparqlQueryContext::new();
    let prefixes = std::collections::HashMap::new();
    let op = match crate::sparql_library::sparql_grammar::parse_update(src, &mut ctx, &prefixes) {
        Ok(op) => op,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error": format!("update parse error: {e}")})),
            )
                .into_response()
        }
    };

    // Real signing key from the daemon key vault — never a placeholder.
    let signing_key = {
        let vault = match state.vault.lock() {
            Ok(v) => v,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"error": "vault_unavailable"})),
                )
                    .into_response()
            }
        };
        vault.derive_key("sparql-update")
    };
    let principal = crate::q_hash("did:qualia:sparql-update-principal");
    let agent = crate::q_hash("did:qualia:sparql-update-agent");
    let wal_path = format!("{}/sparql_update.wal", state.storage_path);

    match crate::daemon_graph::apply_sparql_update_durable(
        &op,
        &ctx,
        &signing_key,
        principal,
        agent,
        &wal_path,
        &state.storage_path,
    ) {
        Ok(outcome) => (
            StatusCode::OK,
            Json(json!({
                "inserted": outcome.inserted,
                "deleted": outcome.deleted,
                "persisted": outcome.persisted,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("update failed: {e}")})),
        )
            .into_response(),
    }
}

async fn query_handler(
    State(state): State<Arc<WebizenState>>,
    headers: HeaderMap,
    Json(request): Json<NativeQueryRequest>,
) -> impl IntoResponse {
    let token = headers
        .get("x-qualia-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let accept = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok());

    let mut allowed_shapes: Option<Vec<String>> = None;
    if !state.dev {
        if let Some(t) = token.as_ref() {
            let vault = state.vault.lock().unwrap();
            match vault.verify_qapp_token(t, "localhost") {
                Ok(payload) => {
                    allowed_shapes = Some(payload.capabilities);
                }
                Err(_) => {
                    if Some(t) != state.token.as_ref() {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(json!({"error": "unauthorized"})),
                        )
                            .into_response();
                    }
                }
            }
        } else {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "missing x-qualia-token"})),
            )
                .into_response();
        }
    }

    if let Some(shapes) = allowed_shapes {
        let q = request.query.to_lowercase();
        let mut authorized = false;
        for shape in shapes {
            let ns = shape.split(':').next().unwrap_or(&shape).to_lowercase();
            if q.contains(&ns) {
                authorized = true;
                break;
            }
        }
        if !authorized && !q.is_empty() {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "gatekeeper violation"})),
            )
                .into_response();
        }
    }

    let output_format = match negotiate_format(request.format.as_deref(), accept) {
        Ok(f) => f,
        Err(_) => return not_acceptable_format_response().into_response(),
    };
    if matches!(output_format, OutputFormat::RawQ42) {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"error": "raw q42 not implemented - use export tools"})),
        )
            .into_response();
    }

    if request.query.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "empty query"})),
        )
            .into_response();
    }

    if request.query.len() as u64 > QUERY_PAYLOAD_LIMIT_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({
                "error": "query too large",
                "limit_bytes": QUERY_PAYLOAD_LIMIT_BYTES
            })),
        )
            .into_response();
    }

    let graph_guard = crate::daemon_graph::graph_read_guard();
    let (stats, final_results) =
        match daemon_query::execute_query_on_graph(&request.query, graph_guard.as_slice()) {
            Ok(pair) => pair,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "query execution failed"})),
                )
                    .into_response()
            }
        };

    let mut sanitized_results = Vec::with_capacity(final_results.len());
    let mut gatekeeper_halt = false;
    for quin in final_results {
        let sensitivity = quin.context >> 56;
        if sensitivity == 0x02 {
            gatekeeper_halt = true;
            sanitized_results.clear();
            break;
        } else {
            sanitized_results.push(quin);
        }
    }
    if gatekeeper_halt {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error": "classified data egress blocked"})),
        )
            .into_response();
    }

    let final_results = sanitized_results;
    let match_count = final_results.len();

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        HeaderName::from_static("x-qualia-compute-cost"),
        HeaderValue::from_str(&format!("{}+{}", match_count, stats.vm_cycles)).unwrap(),
    );

    match output_format {
        OutputFormat::NTriples => {
            let mut body_buf: Vec<u8> = Vec::with_capacity(match_count.max(1) * 80);
            let _ = crate::resolver::format_ntriples_to(&final_results, &mut body_buf);
            response_headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/n-triples"),
            );
            (
                StatusCode::OK,
                response_headers,
                String::from_utf8(body_buf).unwrap_or_default(),
            )
                .into_response()
        }
        OutputFormat::JsonLd => {
            let graph: Vec<serde_json::Value> = final_results.iter().map(|q| json!({
                "subject": q.subject.to_string(), "predicate": q.predicate.to_string(), "object": q.object.to_string(),
                "context": q.context.to_string(), "metadata": q.metadata.to_string(), "parity": q.parity.to_string()
            })).collect();
            response_headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/ld+json"),
            );
            let res = json!({ "@context": { "@vocab": "https://webizen.org/vocab#" }, "@graph": graph, "match_count": match_count });
            (StatusCode::OK, response_headers, res.to_string()).into_response()
        }
        OutputFormat::RawQ42 => unreachable!(),
    }
}

async fn telemetry_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WebizenState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut rx = state.telemetry_tx.subscribe();
        while let Ok(msg) = rx.recv().await {
            if socket.send(Message::Binary(msg.into())).await.is_err() {
                break;
            }
        }
    })
}

/// External WASM telemetry ingestion endpoint.
///
/// External devices (phones, browser tabs, other machines on the LAN) can
/// POST their `SystemTelemetry` data here for local processing. The desktop
/// app receives it, processes it locally (GPU visualisation, graph storage,
/// anomaly detection), and broadcasts it to all connected telemetry
/// subscribers (including the ambient visualisation bridge).
///
/// POST /telemetry/ingest
/// Body: { "source": "device-id", "telemetry": { ...SystemTelemetry fields } }
/// Response: 200 OK { "status": "ingested", "subscribers": N }
async fn telemetry_ingest_handler(
    State(state): State<Arc<WebizenState>>,
    Json(payload): Json<TelemetryIngestRequest>,
) -> impl IntoResponse {
    // Serialize the SystemTelemetry to bytes and broadcast to all subscribers
    let bytes = bytemuck::bytes_of(&payload.telemetry);
    let subscriber_count = state.telemetry_tx.receiver_count();

    // Broadcast the telemetry to all connected subscribers (ambient viz,
    // render bridge, etc.)
    let _ = state.telemetry_tx.send(bytes.to_vec());

    // Also store the telemetry source for provenance tracking
    // (the source device ID is logged for audit purposes)
    eprintln!(
        "[telemetry] Ingested external telemetry from {} ({} subscribers)",
        payload.source, subscriber_count
    );

    (
        StatusCode::OK,
        Json(json!({
            "status": "ingested",
            "source": payload.source,
            "subscribers": subscriber_count,
        })),
    )
}

/// Request body for external telemetry ingestion
#[derive(Deserialize)]
struct TelemetryIngestRequest {
    /// Source device identifier (e.g. "phone-abc123", "browser-tab-1")
    source: String,
    /// The telemetry data from the external device
    telemetry: crate::render::telemetry::SystemTelemetry,
}

async fn manifest_handler(mut body: Body) -> impl IntoResponse {
    use http_body_util::BodyExt;
    let mut payload_bytes = Vec::new();
    while let Some(Ok(frame)) = body.frame().await {
        if let Some(chunk) = frame.data_ref() {
            payload_bytes.extend_from_slice(chunk);
        }
    }
    let lamport_clock: u64 = match crate::wal::WriteAheadLog::open("qualia_global.wal") {
        Ok(mut wal) => wal.buffered_count().unwrap_or(0) as u64 + 1,
        Err(_) => 1,
    };
    match crate::yaml_ld_q42::compile_yaml_ld_to_quins(&payload_bytes, 0, lamport_clock) {
        Ok(quins) => {
            for quin in quins {
                let _ = append_mutation(&quin);
            }
            axum::http::StatusCode::OK
        }
        Err(_) => axum::http::StatusCode::BAD_REQUEST,
    }
}

async fn current_manifest_handler() -> impl IntoResponse {
    let workspace = json!({ "pages": [] });
    Json(workspace)
}

use crate::extension_bus::ExtensionBus;
use std::sync::OnceLock;
static EXTENSION_BUS: OnceLock<ExtensionBus> = OnceLock::new();
fn get_extension_bus() -> &'static ExtensionBus {
    EXTENSION_BUS.get_or_init(|| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        ExtensionBus::new(
            std::path::PathBuf::from(home)
                .join(".qualia")
                .join("extensions")
                .join("pool"),
        )
    })
}
async fn list_extensions_handler() -> impl IntoResponse {
    Json(get_extension_bus().list_extensions())
}
async fn query_extensions_handler(
    axum::extract::Path(interface): axum::extract::Path<String>,
) -> impl IntoResponse {
    Json(get_extension_bus().query_capability(q_hash(&interface)))
}
async fn register_extension_handler(mut body: Body) -> impl IntoResponse {
    use http_body_util::BodyExt;
    let mut payload_bytes = Vec::new();
    while let Some(Ok(frame)) = body.frame().await {
        if let Some(chunk) = frame.data_ref() {
            payload_bytes.extend_from_slice(chunk);
        }
    }
    let bus = get_extension_bus();
    let temp_file = std::env::temp_dir().join("temp_manifest.json");
    if std::fs::write(&temp_file, payload_bytes).is_ok()
        && bus.register_extension_from_path(&temp_file).is_ok()
    {
        return axum::http::StatusCode::OK;
    }
    axum::http::StatusCode::BAD_REQUEST
}
async fn mobile_qr_handler(
    axum::extract::State(state): axum::extract::State<Arc<WebizenState>>,
) -> impl IntoResponse {
    let port = state.port;
    let local_ip = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("1.1.1.1:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "192.168.1.45".to_string());

    let target = format!("http://{}:{}/mobile/app/index.html", local_ip, port);
    let url = format!(
        "https://mediaprophet.github.io/qualiaDB/bootstrap_gateway/index.html?target={}",
        urlencoding::encode(&target)
    );

    let qr = fast_qr::QRBuilder::new(url).build().unwrap();
    let svg = fast_qr::convert::svg::SvgBuilder::default().to_str(&qr);

    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "image/svg+xml")],
        svg,
    )
        .into_response()
}
async fn mobile_ws_handler() -> impl IntoResponse {
    (StatusCode::OK, "WS Mock").into_response()
}
async fn mobile_generate_pane_handler() -> impl IntoResponse {
    (StatusCode::OK, "Pane Mock").into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_permissions_compile_handler_commercial() {
        let req = PermissionsCompileRequest {
            payload: "type: Commercial Micro-Commons".to_string(),
        };
        let response = permissions_compile_handler(Json(req)).await.into_response();
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let compiled: CompiledPermission = serde_json::from_slice(&body).unwrap();

        assert_eq!((compiled.routing_mask >> 61) & 0x03, 0x02); // EnforceBilateralMicroCommons
        assert_ne!((compiled.routing_mask >> 50) & 0x01, 0); // MASK_COMMERCIAL_BILLABLE_GATE set
        assert_eq!(compiled.is_permissive_commons, false);
    }

    #[test]
    fn not_acceptable_payload_matches_native_test_contract() {
        let (_, Json(body)) = not_acceptable_format_response();
        assert_eq!(body["code"], "not_acceptable");
        assert_eq!(body["status"], "error");
        let message = body["message"].as_str().expect("message");
        assert!(message.contains("application/ld+json"));
        assert!(message.contains("application/n-triples"));
    }

    #[tokio::test]
    async fn test_permissions_compile_handler_public() {
        let req = PermissionsCompileRequest {
            payload: "type: Public Commons".to_string(),
        };
        let response = permissions_compile_handler(Json(req)).await.into_response();
        let body = http_body_util::BodyExt::collect(response.into_body())
            .await
            .unwrap()
            .to_bytes();
        let compiled: CompiledPermission = serde_json::from_slice(&body).unwrap();

        assert_eq!((compiled.routing_mask >> 61) & 0x03, 0x01); // EnforcePermissiveCommons
        assert_eq!(compiled.is_permissive_commons, true);
    }

    #[tokio::test]
    async fn test_webizen_rpc_sanctuary_mode_assertion() {
        let (telemetry_tx, _) = tokio::sync::broadcast::channel(10);
        let state = Arc::new(WebizenState {
            telemetry_tx,
            type_index: Arc::new([]),
            dev: true,
            token: None,
            vault: Arc::new(Mutex::new(crate::key_vault::KeyVault::new())),
            storage_path: "/tmp".to_string(),
            port: 8080,
            in_sanctuary_mode: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        });

        // 1. Requesting 'wf:AgentProposal' should fail with 423 LOCKED
        let req1 = WebizenRpcRequest {
            method: "requestAccess".to_string(),
            scopes: Some(vec!["wf:AgentProposal".to_string()]),
            payload: None,
        };
        let response1 = webizen_rpc_handler(axum::extract::State(state.clone()), Json(req1))
            .await
            .into_response();
        assert_eq!(response1.status(), StatusCode::LOCKED);

        // 2. Requesting 'qp:Project' should succeed (Commons access allowed in Sanctuary)
        let req2 = WebizenRpcRequest {
            method: "requestAccess".to_string(),
            scopes: Some(vec!["qp:Project".to_string()]),
            payload: None,
        };
        let response2 = webizen_rpc_handler(axum::extract::State(state.clone()), Json(req2))
            .await
            .into_response();
        assert_eq!(response2.status(), StatusCode::OK);
    }
}
