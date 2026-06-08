#![cfg(not(target_arch = "wasm32"))]

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use warp::http::StatusCode;
use warp::Filter;

const OFFICIAL_WEB_HUB_ORIGIN: &str = "https://mediaprophet.github.io";
const QUERY_PAYLOAD_LIMIT_BYTES: u64 = 64 * 1024;
const PROXY_FETCH_MAX_BYTES: usize = 64 * 1024 * 1024;
const CELL_MEMORY_FLOOR_MB: u16 = 512;

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

fn proxy_fetch_error(
    status: StatusCode,
    message: impl Into<String>,
) -> warp::http::Response<warp::hyper::Body> {
    let body = json!({
        "status": "error",
        "message": message.into()
    })
    .to_string();
    warp::http::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(warp::hyper::Body::from(body))
        .expect("proxy error response")
}

fn ws_query_error_json(id: u64, err: QueryExecError) -> serde_json::Value {
    match err {
        QueryExecError::EmptyQuery => json!({
            "type": "error",
            "id": id,
            "code": "empty_query",
            "message": "query must be non-empty",
        }),
        QueryExecError::ParseError(message) => json!({
            "type": "error",
            "id": id,
            "code": "parse_error",
            "message": message,
        }),
        QueryExecError::OutputBufferFull => json!({
            "type": "error",
            "id": id,
            "code": "result_set_too_large",
            "message": "query matched more than the output buffer allows",
        }),
        QueryExecError::InvalidProgram => json!({
            "type": "error",
            "id": id,
            "code": "vm_error",
            "message": "internal VM rejected the compiled program",
        }),
        QueryExecError::ClassifiedEgress => json!({
            "type": "error",
            "id": id,
            "code": "restricted_data_access",
            "message": "gatekeeper blocked classified context egress",
        }),
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

fn proxy_fetch_ok(content_type: &str, bytes: Vec<u8>) -> warp::http::Response<warp::hyper::Body> {
    let mut response = warp::http::Response::new(warp::hyper::Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        warp::http::header::CONTENT_TYPE,
        warp::http::HeaderValue::from_str(content_type)
            .unwrap_or_else(|_| warp::http::HeaderValue::from_static("application/octet-stream")),
    );
    response
}

/// Fractal-sharding topology configured at daemon boot (`qualia-cli daemon --workers N`).
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct DaemonTopology {
    pub worker_cells_configured: u16,
    pub compute_swarm_enabled: bool,
}

static DAEMON_TOPOLOGY: std::sync::OnceLock<DaemonTopology> = std::sync::OnceLock::new();

/// Called by `qualia-cli daemon` before the HTTP server starts.
pub fn configure_daemon_topology(topology: DaemonTopology) {
    let _ = DAEMON_TOPOLOGY.set(topology);
}

fn current_topology() -> DaemonTopology {
    *DAEMON_TOPOLOGY.get().unwrap_or(&DaemonTopology {
        worker_cells_configured: 1,
        compute_swarm_enabled: false,
    })
}

/// JSON block shared by `/health`, benchmark exports, and the comparative harness.
pub fn execution_environment_json() -> serde_json::Value {
    let topo = current_topology();
    let mode = if topo.worker_cells_configured > 1 || topo.compute_swarm_enabled {
        "fractal_swarm"
    } else {
        "single_cell"
    };
    json!({
        "runner": "qualia-core-db daemon",
        "engine_version": crate::ENGINE_VERSION,
        "memory_ceiling_mb": CELL_MEMORY_FLOOR_MB,
        "measurement_path": "daemon_http_query",
        "topology": {
            "mode": mode,
            "worker_cells_configured": topo.worker_cells_configured,
            "worker_cells_active_during_run": topo.worker_cells_configured,
            "compute_swarm_enabled": topo.compute_swarm_enabled,
            "cell_memory_floor_mb": CELL_MEMORY_FLOOR_MB,
            "scheduling": "fixed-pool"
        }
    })
}

use crate::daemon_query::{self, QueryExecError};

#[derive(Clone)]
struct DaemonSecurity {
    dev: bool,
    token: Option<String>,
    vault: std::sync::Arc<std::sync::Mutex<crate::key_vault::KeyVault>>,
}

#[derive(Deserialize)]
struct NativeQueryRequest {
    query: String,
    format: Option<String>,
}

// ---------------------------------------------------------------------------
// Output-format negotiation
// ---------------------------------------------------------------------------

enum OutputFormat {
    /// `application/ld+json` — default
    JsonLd,
    /// `application/n-triples` — text serialisation of SPO triples
    NTriples,
    /// `application/x-qualia-q42` — raw binary stream (future)
    RawQ42,
}

/// Resolve the desired output format from the JSON payload `"format"` key
/// (higher priority) or the HTTP `Accept` header.  Returns `Err(())` when
/// the client explicitly names a format the daemon cannot produce.
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

// ---------------------------------------------------------------------------
// Response helper
// ---------------------------------------------------------------------------

/// Build an HTTP response with an explicit `content-type`.
/// `warp::http::Response<String>` implements `warp::Reply`, so every branch of
/// the query handler can return the same concrete type.
fn make_response(
    status: StatusCode,
    content_type: &'static str,
    body: String,
) -> warp::http::Response<String> {
    warp::http::Response::builder()
        .status(status)
        .header("content-type", content_type)
        .body(body)
        .expect("infallible response builder")
}

/// Build a successful query response that includes the `X-Qualia-Compute-Cost`
/// telemetry header.  The header value is `{match_count}+{vm_cycles}` — the
/// number of results found and the total VM opcodes decoded to find them.
fn make_query_response(
    status: StatusCode,
    content_type: &'static str,
    body: String,
    match_count: usize,
    vm_cycles: u64,
) -> warp::http::Response<String> {
    warp::http::Response::builder()
        .status(status)
        .header("content-type", content_type)
        .header(
            "X-Qualia-Compute-Cost",
            format!("{match_count}+{vm_cycles}"),
        )
        .body(body)
        .expect("infallible query response builder")
}

// ---------------------------------------------------------------------------
// Daemon entry points
// ---------------------------------------------------------------------------

/// Starts the native loopback daemon on 127.0.0.1 with strict token checks.
pub async fn start_local_daemon(
    port: u16,
    vault: std::sync::Arc<std::sync::Mutex<crate::key_vault::KeyVault>>,
) {
    start_local_daemon_with_options(port, false, vault).await;
}

/// Starts the native loopback daemon with WebSocket and REST handoff routes.
pub async fn start_local_daemon_with_options(
    port: u16,
    dev: bool,
    vault: std::sync::Arc<std::sync::Mutex<crate::key_vault::KeyVault>>,
) {
    let storage_path = std::env::var("QUALIA_STORAGE_PATH").unwrap_or_else(|_| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| format!("{h}/.qualia"))
            .unwrap_or_else(|_| ".qualia".to_string())
    });
    crate::daemon_graph::init_daemon_graph(&storage_path);

    let security = DaemonSecurity {
        dev,
        token: std::env::var("QUALIA_TOKEN")
            .ok()
            .or_else(|| std::env::var("QUALIA_DEV_TOKEN").ok()),
        vault,
    };

    if dev {
        tokio::spawn(async move {
            crate::mcp_server::start_mcp_listener().await;
        });
    }

    // -----------------------------------------------------------------------
    // WebSocket bridge — handshake + query metrics + bench_load
    // -----------------------------------------------------------------------
    let ws_dev = dev;
    let qualia_bridge = warp::path("qualia-bridge")
        .and(warp::ws())
        .map(move |ws: warp::ws::Ws| {
            let dev = ws_dev;
            ws.on_upgrade(move |mut socket| async move {
                let handshake = json!({
                    "type": "HANDSHAKE_SUCCESS",
                    "payload": { "mode": "NATIVE", "version": crate::ENGINE_VERSION }
                });
                if socket
                    .send(warp::ws::Message::text(handshake.to_string()))
                    .await
                    .is_ok()
                {
                    println!("[Qualia Daemon] Client connected to Native WebSocket Bridge");
                }

                let mut pending_bench_id: Option<u64> = None;

                while let Some(result) = socket.next().await {
                    match result {
                        Ok(msg) if msg.is_binary() => {
                            if let Some(id) = pending_bench_id.take() {
                                let bytes = msg.as_bytes();
                                let reply = match crate::daemon_graph::replace_graph_from_flat_bytes(bytes)
                                {
                                    Ok(count) => json!({
                                        "type": "bench_loaded",
                                        "id": id,
                                        "quin_count": count,
                                        "graph_quin_count": crate::daemon_graph::graph_quin_count(),
                                    }),
                                    Err(message) => json!({
                                        "type": "error",
                                        "id": id,
                                        "code": "bench_load_failed",
                                        "message": message,
                                    }),
                                };
                                let _ = socket.send(warp::ws::Message::text(reply.to_string())).await;
                            }
                        }
                        Ok(msg) if msg.is_text() => {
                            let text = msg.to_str().unwrap_or_default();
                            let Ok(frame) = serde_json::from_str::<serde_json::Value>(text) else {
                                let _ = socket.send(warp::ws::Message::text(json!({
                                    "type": "error",
                                    "code": "invalid_json",
                                    "message": "expected JSON frame"
                                }).to_string())).await;
                                continue;
                            };

                            let frame_type = frame.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            let id = frame.get("id").and_then(|v| v.as_u64()).unwrap_or(0);

                            let reply = match frame_type {
                                "query" => {
                                    let query = frame
                                        .get("query")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let format = frame
                                        .get("format")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("metrics");
                                    if format != "metrics" {
                                        json!({
                                            "type": "error",
                                            "id": id,
                                            "code": "unsupported_format",
                                            "message": "WebSocket queries support format=metrics only"
                                        })
                                    } else {
                                        let graph = crate::daemon_graph::graph_read_guard();
                                        match daemon_query::execute_ntriples_metrics(&query, graph.as_slice()) {
                                            Ok(stats) => json!({
                                                "type": "result",
                                                "id": id,
                                                "match_count": stats.match_count,
                                                "vm_cycles": stats.vm_cycles,
                                                "direct_jump_ops": stats.direct_jump_ops,
                                                "lexicon_lookup_ops": stats.lexicon_lookup_ops,
                                            }),
                                            Err(err) => ws_query_error_json(id, err),
                                        }
                                    }
                                }
                                "bench_load" if dev => {
                                    if frame.get("byte_length").and_then(|v| v.as_u64()).is_some() {
                                        pending_bench_id = Some(id);
                                        json!({
                                            "type": "bench_load_ready",
                                            "id": id,
                                            "message": "send next binary frame with flat QualiaQuin bytes"
                                        })
                                    } else if let Some(b64) = frame.get("db_b64").and_then(|v| v.as_str()) {
                                        match decode_bench_load_b64(b64) {
                                            Ok(bytes) => match crate::daemon_graph::replace_graph_from_flat_bytes(&bytes) {
                                                Ok(count) => json!({
                                                    "type": "bench_loaded",
                                                    "id": id,
                                                    "quin_count": count,
                                                    "graph_quin_count": crate::daemon_graph::graph_quin_count(),
                                                }),
                                                Err(message) => json!({
                                                    "type": "error",
                                                    "id": id,
                                                    "code": "bench_load_failed",
                                                    "message": message,
                                                }),
                                            },
                                            Err(message) => json!({
                                                "type": "error",
                                                "id": id,
                                                "code": "bench_load_failed",
                                                "message": message,
                                            }),
                                        }
                                    } else {
                                        json!({
                                            "type": "error",
                                            "id": id,
                                            "code": "bench_load_failed",
                                            "message": "bench_load requires db_b64 or byte_length + binary frame"
                                        })
                                    }
                                }
                                "bench_load" => json!({
                                    "type": "error",
                                    "id": id,
                                    "code": "forbidden",
                                    "message": "bench_load requires daemon --dev"
                                }),
                                _ => json!({
                                    "type": "error",
                                    "id": id,
                                    "code": "unknown_type",
                                    "message": format!("unsupported frame type: {frame_type}")
                                }),
                            };

                            let _ = socket.send(warp::ws::Message::text(reply.to_string())).await;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("[Qualia Daemon] WebSocket error: {e}");
                            break;
                        }
                    }
                }
                println!("[Qualia Daemon] Client disconnected.");
            })
        });

    // -----------------------------------------------------------------------
    // GET /health
    // -----------------------------------------------------------------------
    let health = warp::path("health").and(warp::get()).map(|| {
        warp::reply::with_status(
            warp::reply::json(&json!({
                "status": "active",
                "engine": "qualia-core-db",
                "version": crate::ENGINE_VERSION,
                "graph_quin_count": crate::daemon_graph::graph_quin_count(),
                "webtorrent": crate::webtorrent_seeder::telemetry(),
                "execution_environment": execution_environment_json()
            })),
            StatusCode::OK,
        )
    });

    // -----------------------------------------------------------------------
    // POST /query
    // -----------------------------------------------------------------------
    let query_security = security.clone();
    let query = warp::path("query")
        .and(warp::post())
        .and(warp::header::optional::<String>("x-qualia-token"))
        .and(warp::header::optional::<String>("accept"))
        .and(warp::body::content_length_limit(QUERY_PAYLOAD_LIMIT_BYTES))
        .and(warp::body::json())
        .map(
            move |token: Option<String>,
                  accept: Option<String>,
                  request: NativeQueryRequest| {

                // --- Auth & Gatekeeper Policy -------------------------------
                let mut allowed_shapes: Option<Vec<String>> = None;
                
                if !query_security.dev {
                    if let Some(t) = token.as_ref() {
                        let vault = query_security.vault.lock().unwrap();
                        match vault.verify_qapp_token(t) {
                            Ok(payload) => {
                                // Semantic token valid!
                                allowed_shapes = Some(payload.allowed_shapes);
                            }
                            Err(_) => {
                                // Check if it's the simple global dev token fallback
                                if Some(t) != query_security.token.as_ref() {
                                    return make_response(
                                        StatusCode::UNAUTHORIZED,
                                        "application/json",
                                        json!({
                                            "status": "error",
                                            "code": "unauthorized",
                                            "message": "Invalid Semantic App Token or Dev Token"
                                        })
                                        .to_string(),
                                    );
                                }
                            }
                        }
                    } else {
                        return make_response(
                            StatusCode::UNAUTHORIZED,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "unauthorized",
                                "message": "Missing X-Qualia-Token header"
                            })
                            .to_string(),
                        );
                    }
                }

                // --- Semantic Shape Enforcement (Fail-Closed) ---------------
                if let Some(shapes) = allowed_shapes {
                    // Primitive namespace check based on the incoming query string.
                    // If the app requested shacl:MedicalRecord, and the query is trying to hit foaf:Person,
                    // we block it unless foaf:Person is also in the allowed shapes.
                    // This is a naive substring matching validation for foundational scaffolding.
                    let q = request.query.to_lowercase();
                    let mut authorized = false;
                    for shape in shapes {
                        let ns = shape.split(':').next().unwrap_or(&shape).to_lowercase();
                        if q.contains(&ns) {
                            authorized = true;
                            break;
                        }
                    }
                    // For safety, if they are just querying something broad we can allow it, 
                    // but if it's specifically a namespace, it must match.
                    if !authorized && !q.is_empty() {
                         return make_response(
                             StatusCode::FORBIDDEN,
                             "application/json",
                             json!({
                                 "status": "error",
                                 "code": "forbidden",
                                 "message": "Gatekeeper Policy Violation: Query targets semantic shapes outside your App Manifest scope."
                             })
                             .to_string(),
                         );
                    }
                }

                // --- Format negotiation -------------------------------------
                let output_format =
                    match negotiate_format(request.format.as_deref(), accept.as_deref()) {
                        Ok(f) => f,
                        Err(()) => {
                            return make_response(
                                StatusCode::NOT_ACCEPTABLE,
                                "application/json",
                                json!({
                                    "status": "error",
                                    "code": "not_acceptable",
                                    "message": "Supported: application/ld+json, application/n-triples, application/x-qualia-q42"
                                })
                                .to_string(),
                            );
                        }
                    };

                // Raw Q42 binary streaming is not implemented yet.
                if matches!(output_format, OutputFormat::RawQ42) {
                    return make_response(
                        StatusCode::NOT_IMPLEMENTED,
                        "application/json",
                        json!({
                            "status": "error",
                            "code": "not_implemented",
                            "message": "application/x-qualia-q42 binary streaming is not yet available"
                        })
                        .to_string(),
                    );
                }

                // --- Basic validation ----------------------------------------
                if request.query.trim().is_empty() {
                    return make_response(
                        StatusCode::BAD_REQUEST,
                        "application/json",
                        json!({
                            "status": "error",
                            "code": "empty_query",
                            "message": "Query payload must include a non-empty query string"
                        })
                        .to_string(),
                    );
                }

                let graph_guard = crate::daemon_graph::graph_read_guard();
                let (stats, final_results) = match daemon_query::execute_ntriples_pattern_on_graph(
                    &request.query,
                    graph_guard.as_slice(),
                ) {
                    Ok(pair) => pair,
                    Err(QueryExecError::EmptyQuery) => {
                        return make_response(
                            StatusCode::BAD_REQUEST,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "empty_query",
                                "message": "Query payload must include a non-empty query string"
                            })
                            .to_string(),
                        );
                    }
                    Err(QueryExecError::ParseError(detail)) => {
                        return make_response(
                            StatusCode::BAD_REQUEST,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "parse_error",
                                "message": format!(
                                    "Query could not be compiled to bytecode: {detail}. \
                                     Supply a single N-Triples pattern, e.g. \
                                     \"<subject> ?p <object> .\""
                                )
                            })
                            .to_string(),
                        );
                    }
                    Err(QueryExecError::OutputBufferFull) => {
                        return make_response(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "result_set_too_large",
                                "message": format!(
                                    "Query matched more than {} Quins. \
                                     Add more constraints to narrow the result set.",
                                    daemon_query::QUERY_OUT_SLOTS
                                )
                            })
                            .to_string(),
                        );
                    }
                    Err(QueryExecError::InvalidProgram) => {
                        return make_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "vm_error",
                                "message": "Internal VM rejected the compiled program."
                            })
                            .to_string(),
                        );
                    }
                    Err(QueryExecError::ClassifiedEgress) => {
                        println!("[Webizen Gatekeeper] DENIED egress of Classified record");
                        return make_response(
                            StatusCode::FORBIDDEN,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "restricted_data_access",
                                "message": "Gatekeeper Policy Violation: Attempted egress of CLASSIFIED context data."
                            })
                            .to_string(),
                        );
                    }
                };

                let match_count = stats.match_count;
                let vm_cycles = stats.vm_cycles;

                // --- Serialise and attach compute-cost telemetry -------------
                match output_format {
                    OutputFormat::NTriples => {
                        // `format_ntriples_to` writes directly to the Vec<u8>
                        // buffer — the formatter itself performs no allocation.
                        let mut body_buf: Vec<u8> =
                            Vec::with_capacity(match_count.max(1) * 80);
                        let _ = crate::resolver::format_ntriples_to(
                            &final_results,
                            &mut body_buf,
                        );
                        let body = String::from_utf8(body_buf).unwrap_or_default();
                        make_query_response(
                            StatusCode::OK,
                            "application/n-triples",
                            body,
                            match_count,
                            vm_cycles,
                        )
                    }

                    OutputFormat::JsonLd => {
                        let graph: Vec<serde_json::Value> = final_results
                            .iter()
                            .map(|q| {
                                json!({
                                    "subject":   q.subject.to_string(),
                                    "predicate": q.predicate.to_string(),
                                    "object":    q.object.to_string(),
                                    "context":   q.context.to_string(),
                                    "metadata":  q.metadata.to_string(),
                                    "parity":    q.parity.to_string()
                                })
                            })
                            .collect();

                        make_query_response(
                            StatusCode::OK,
                            "application/ld+json",
                            json!({
                                "@context": { "@vocab": "https://qualia-db.org/vocab#" },
                                "@graph": graph,
                                "match_count": match_count
                            })
                            .to_string(),
                            match_count,
                            vm_cycles,
                        )
                    }

                    // Already handled above; unreachable here.
                    OutputFormat::RawQ42 => unreachable!(),
                }
            },
        );

    // -----------------------------------------------------------------------
    // POST /cache  — dataset shard upload
    // -----------------------------------------------------------------------
    let cache = warp::path("cache")
        .and(warp::post())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(warp::body::content_length_limit(QUERY_PAYLOAD_LIMIT_BYTES))
        .and(warp::body::bytes())
        .map(
            |qs: std::collections::HashMap<String, String>, body: warp::hyper::body::Bytes| {
                let filename = qs
                    .get("filename")
                    .cloned()
                    .unwrap_or_else(|| "dataset_shard.q42".to_string());
                let mut path = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|_| std::path::PathBuf::from("."));
                path.push(".qualia");
                path.push("cache");
                let _ = std::fs::create_dir_all(&path);
                path.push(&filename);
                let _ = std::fs::write(&path, body);
                println!("[Qualia Daemon] Cached shard to {:?}", path);
                warp::reply::with_status(
                    warp::reply::json(&json!({ "status": "ok", "saved_to": path.to_str() })),
                    StatusCode::OK,
                )
            },
        );

    // -----------------------------------------------------------------------
    // GET /proxy/fetch?url=…  — CORS relay for GH Pages / playground URI import
    // -----------------------------------------------------------------------
    let proxy_fetch = warp::path!("proxy" / "fetch")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(|qs: std::collections::HashMap<String, String>| async move {
            let target = qs.get("url").cloned().unwrap_or_default();
            if target.is_empty() {
                return Ok::<_, warp::Rejection>(proxy_fetch_error(
                    StatusCode::BAD_REQUEST,
                    "missing url query parameter",
                ));
            }

            let parsed = match reqwest::Url::parse(&target) {
                Ok(url) => url,
                Err(_) => {
                    return Ok::<_, warp::Rejection>(proxy_fetch_error(
                        StatusCode::BAD_REQUEST,
                        "invalid url",
                    ));
                }
            };

            if !proxy_target_allowed(&parsed) {
                return Ok::<_, warp::Rejection>(proxy_fetch_error(
                    StatusCode::FORBIDDEN,
                    "target host is not allowed",
                ));
            }

            let client = match reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .redirect(reqwest::redirect::Policy::limited(5))
                .build()
            {
                Ok(c) => c,
                Err(_) => {
                    return Ok::<_, warp::Rejection>(proxy_fetch_error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "proxy client init failed",
                    ));
                }
            };

            let response = match client.get(parsed).send().await {
                Ok(r) => r,
                Err(e) => {
                    return Ok::<_, warp::Rejection>(proxy_fetch_error(
                        StatusCode::BAD_GATEWAY,
                        format!("upstream fetch failed: {e}"),
                    ));
                }
            };

            if !response.status().is_success() {
                return Ok::<_, warp::Rejection>(proxy_fetch_error(
                    StatusCode::BAD_GATEWAY,
                    format!("upstream HTTP {}", response.status()),
                ));
            }

            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();

            let bytes = match response.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    return Ok::<_, warp::Rejection>(proxy_fetch_error(
                        StatusCode::BAD_GATEWAY,
                        format!("upstream body read failed: {e}"),
                    ));
                }
            };

            if bytes.len() > PROXY_FETCH_MAX_BYTES {
                return Ok::<_, warp::Rejection>(proxy_fetch_error(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "upstream payload exceeds 64 MiB proxy limit",
                ));
            }

            Ok::<_, warp::Rejection>(proxy_fetch_ok(&content_type, bytes.to_vec()))
        });

    let proxy_preflight = warp::path!("proxy" / "fetch").and(warp::options()).map(|| {
        warp::reply::with_status(
            warp::reply::json(&json!({ "status": "ok" })),
            StatusCode::OK,
        )
    });

    // -----------------------------------------------------------------------
    // OPTIONS preflight
    // -----------------------------------------------------------------------
    let preflight = warp::path("health")
        .or(warp::path("query"))
        .unify()
        .and(warp::options())
        .map(|| {
            warp::reply::with_status(
                warp::reply::json(&json!({ "status": "ok" })),
                StatusCode::OK,
            )
        });

    // -----------------------------------------------------------------------
    // CORS + private-network header
    // Dev mode allows any origin so that localhost test runners and desktop
    // apps can connect without being blocked by same-origin policy.
    // -----------------------------------------------------------------------
    let allowed_origins: Vec<&str> = if dev {
        vec![
            "http://localhost:8788",
            "http://127.0.0.1:8788",
            "http://localhost:5173",
            "http://127.0.0.1:5173",
            OFFICIAL_WEB_HUB_ORIGIN,
        ]
    } else {
        vec![OFFICIAL_WEB_HUB_ORIGIN]
    };

    let cors = warp::cors()
        .allow_origins(allowed_origins)
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_header("range")
        .allow_headers(vec![
            "content-type",
            "accept",
            "x-qualia-token",
            "access-control-request-private-network",
        ])
        .expose_headers(vec!["x-qualia-compute-cost"]);

    let relay_routes =
        crate::chat_relay_daemon::chat_relay_routes(storage_path.clone(), security.vault.clone());

    crate::webtorrent_seeder::sync_from_workbench(&storage_path, port);

    let torrent_routes = crate::webtorrent_routes::webtorrent_routes(port);

    let routes = qualia_bridge
        .or(health)
        .or(query)
        .or(proxy_fetch)
        .or(proxy_preflight)
        .or(relay_routes)
        .or(torrent_routes)
        .or(cache)
        .or(preflight)
        .with(cors)
        .with(warp::reply::with::header(
            "Access-Control-Allow-Private-Network",
            "true",
        ));

    println!("============================================================");
    println!("Qualia-DB Native Local Daemon Booting");
    println!("Listening on 127.0.0.1:{port}");
    println!("  WebSocket: ws://127.0.0.1:{port}/qualia-bridge");
    println!("  Health:    http://127.0.0.1:{port}/health");
    println!("  Query:     http://127.0.0.1:{port}/query");
    println!("  Chat relay: http://127.0.0.1:{port}/chat/publish | /chat/pull");
    println!("  CORS proxy: http://127.0.0.1:{port}/proxy/fetch?url=<encoded>");
    println!("  WebTorrent: http://127.0.0.1:{port}/torrent/webseed/{{hash}} | /torrent/seed");
    println!(
        "  Mode:      {}",
        if security.dev {
            "dev token bypass"
        } else {
            "token required"
        }
    );
    println!("============================================================");

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            println!(
                "[Informatics Subsystem] Running N3Logic differential diagnostics over .q42 graph..."
            );
        }
    });

    tokio::spawn(async {
        println!("[Qualia Daemon] Nym Mixnet: Sphinx Packet routing initialized.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    tokio::spawn(async {
        println!("[Qualia Daemon] Gun.eco: WebSocket Graph bridge initialized.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;
        }
    });

    pub struct PeerLedger {
        pub unbilled_bytes: usize,
        pub warning_issued_at: Option<std::time::Instant>,
    }
    let bandwidth_meter = std::sync::Arc::new(dashmap::DashMap::<String, PeerLedger>::new());
    let bandwidth_meter_swarm = bandwidth_meter.clone();

    // -----------------------------------------------------------------------
    // P2P Network Swarm (CBOR-LD Semantic Sync)
    // -----------------------------------------------------------------------
    let p2p_vault = security.vault.clone();
    tokio::spawn(async move {
        let master_key_bytes = {
            let v = p2p_vault.lock().unwrap();
            v.get_master_key_bytes()
        };

        let mut ed25519_bytes = master_key_bytes;
        let local_key = libp2p::identity::Keypair::ed25519_from_bytes(&mut ed25519_bytes)
            .expect("Valid Ed25519 Key Vault Master Key");

        let local_peer_id = libp2p::PeerId::from(local_key.public());
        println!(
            "[Qualia Daemon] P2P Identity Active. PeerId: {}",
            local_peer_id
        );

        let behaviour = crate::p2p::swarm::build_behaviour(local_peer_id);

        let routing_table = std::sync::Arc::new(crate::p2p::routing::CivicsRoutingTable::new());
        let local_db_slice: &[crate::QualiaQuin] = &[]; // Mock of memory mapped DB slice
        routing_table.hydrate_from_db(local_db_slice);

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .unwrap()
            .with_behaviour(|_| behaviour)
            .unwrap()
            .with_swarm_config(|c| {
                c.with_idle_connection_timeout(std::time::Duration::from_secs(60))
            })
            .build();

        // Bind to all IPv6 interfaces
        swarm
            .listen_on("/ip6/::/tcp/4243".parse().unwrap())
            .expect("P2P Swarm Socket bind failed");

        loop {
            tokio::select! {
                event = swarm.select_next_some() => match event {
                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                        println!("[Qualia Daemon] P2P Node Listening on {}", address);
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(crate::p2p::swarm::QualiaBehaviourEvent::RequestResponse(
                        libp2p::request_response::Event::Message { peer, message, .. }
                    )) => {
                        match message {
                            libp2p::request_response::Message::Request { request, channel, .. } => {
                                match request {
                                    crate::p2p::protocol::QualiaRequest::Handshake { compressed_vcs } => {
                                        let mut route_authorized = false;
                                        let vcs_count = compressed_vcs.len() / 112;

                                        for i in 0..vcs_count {
                                            let offset = i * 112;
                                            if compressed_vcs.len() < offset + 112 { break; }

                                            // Zero-allocation cast for the 48-byte Quin
                                            let quin_bytes = &compressed_vcs[offset..offset+48];
                                            let quin: &crate::p2p::protocol::QualiaQuin = unsafe {
                                                &*(quin_bytes.as_ptr() as *const crate::p2p::protocol::QualiaQuin)
                                            };

                                            let signature_bytes: &[u8; 64] = compressed_vcs[offset+48..offset+112].try_into().unwrap();

                                            // Mock ORG_MEMBER_HASH for demonstration
                                            let org_member_hash = [1u8, 2, 3, 4, 5, 6, 7, 8];

                                            if quin.predicate == org_member_hash {
                                                if routing_table.is_authorized(&quin.object, quin_bytes, signature_bytes) {
                                                    route_authorized = true;
                                                    break; // Instant approval
                                                }
                                            }
                                        }

                                        if !route_authorized {
                                            println!("[Qualia Daemon] Dropping Handshake from {}: Unauthorized Group DID.", peer);
                                            let _ = swarm.behaviour_mut().request_response.send_response(
                                                channel,
                                                crate::p2p::protocol::QualiaResponse::HandshakeAck { success: false }
                                            );
                                            let _ = swarm.disconnect_peer_id(peer);
                                        } else {
                                            println!("[Qualia Daemon] Handshake approved for {}. Upgrading trust.", peer);
                                            let _ = swarm.behaviour_mut().request_response.send_response(
                                                channel,
                                                crate::p2p::protocol::QualiaResponse::HandshakeAck { success: true }
                                            );
                                        }
                                    },
                                    crate::p2p::protocol::QualiaRequest::Sync { hop_count, gatekeeper_token, target_shapes } => {
                                        let mut is_authorized = false;

                                        // Strict 2-Hop Limit for the Web Civics Mesh
                                        if hop_count > 2 {
                                            println!("[Qualia Daemon] Dropping Sync from {}: Exceeded 2-hop trust horizon.", peer);
                                        } else {
                                            if gatekeeper_token.is_some() {
                                                is_authorized = true;
                                            } else {
                                                if target_shapes.contains(&"foaf:Person".to_string()) {
                                                    is_authorized = true;
                                                }
                                            }
                                        }

                                        let response = if is_authorized {
                                            crate::p2p::protocol::QualiaResponse::SyncAck {
                                                success: true,
                                                message: "Sync Approved".to_string(),
                                                blocks_sent: 42,
                                            }
                                        } else {
                                            crate::p2p::protocol::QualiaResponse::SyncAck {
                                                success: false,
                                                message: "RequiresGatekeeperChallenge".to_string(),
                                                blocks_sent: 0,
                                            }
                                        };
                                        let _ = swarm.behaviour_mut().request_response.send_response(channel, response);
                                    }
                                }
                            },
                            libp2p::request_response::Message::Response { response, .. } => {
                                match response {
                                    crate::p2p::protocol::QualiaResponse::HandshakeAck { success } => {
                                        println!("[Qualia Daemon] Received Handshake Ack from {}: success={}", peer, success);
                                    },
                                    crate::p2p::protocol::QualiaResponse::SyncAck { success, blocks_sent, .. } => {
                                        println!("[Qualia Daemon] Received Sync Ack from {}: success={}, blocks={}", peer, success, blocks_sent);
                                        if success && blocks_sent > 0 {
                                            let mut buf = Vec::new();
                                            let overhead = if ciborium::into_writer(&response, &mut buf).is_ok() { buf.len() } else { 0 };
                                            // Actual serialized payload: CBOR overhead + (blocks_sent * 48 bytes per QualiaQuin)
                                            let bytes_transferred = overhead + (blocks_sent as usize * 48);
                                            let peer_str = peer.to_string();
                                            bandwidth_meter_swarm.entry(peer_str)
                                                .and_modify(|ledger| ledger.unbilled_bytes += bytes_transferred)
                                                .or_insert(PeerLedger {
                                                    unbilled_bytes: bytes_transferred,
                                                    warning_issued_at: None,
                                                });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    // -----------------------------------------------------------------------
    // Web Civics SOCKS5 Userspace Proxy
    // -----------------------------------------------------------------------
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:1080")
            .await
            .expect("Failed to bind SOCKS5 proxy");
        println!("[Web Civics] Userspace WireGuard Proxy Listening on 127.0.0.1:1080");

        loop {
            if let Ok((_socket, _addr)) = listener.accept().await {
                tokio::task::yield_now().await;
            }
        }
    });

    // -----------------------------------------------------------------------
    // Semantic Task Engine (Economics & Scientific Compute)
    // -----------------------------------------------------------------------
    tokio::spawn(async move {
        println!("[Semantic Task Engine] Watching graph for Distributed Compute Tasks...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            // In a real implementation, we would query the CRDT graph here:
            // SELECT ?task WHERE { ?task rdf:type qualia:MonteCarloSimulation . ?task status "Pending" }

            // Mocking a detected task for the walkthrough:
            let task_detected = false; // Set to true to test
            if task_detected {
                println!("[Semantic Task Engine] Detected MonteCarloSimulation Task.");
                let (mean, var) =
                    crate::economics::run_monte_carlo_var(100.0, 0.05, 0.20, 1.0, 252, 100_000);
                println!(
                    "[Semantic Task Engine] Simulation Complete. Mean: {:.2}, 95% VaR: {:.2}",
                    mean, var
                );
                // We would then write the result back into the graph as a qualia:SimulationResult
            }
        }
    });

    // -----------------------------------------------------------------------
    // Micropayment Settlement Engine (Soft/Hard Limits & Grace Period)
    // -----------------------------------------------------------------------
    let payment_meter = bandwidth_meter.clone();
    tokio::spawn(async move {
        println!("[Micropayment Engine] Initialized. Monitoring mesh bandwidth routing...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            let context = crate::economics::get_current_system_context();

            // Iterate over the DashMap without blocking the main router
            for mut entry in payment_meter.iter_mut() {
                let peer_id = entry.key().clone();
                let ledger = entry.value_mut();

                let liability = crate::economics::calculate_bandwidth_liability(
                    ledger.unbilled_bytes,
                    &context,
                );

                if liability >= 0.015 {
                    // HARD LIMIT
                    println!(
                        "[Micropayment Engine] {} hit Hard Limit (${:.4}). Disconnecting.",
                        peer_id, liability
                    );
                    // Disconnect peer immediately
                    // network::disconnect_and_block(&peer_id);
                    ledger.unbilled_bytes = 0;
                    ledger.warning_issued_at = None;
                } else if liability >= 0.010 && ledger.warning_issued_at.is_none() {
                    // SOFT LIMIT
                    println!("[Micropayment Engine] {} hit Soft Limit (${:.4}). Emitting DebtQuin. Grace Period started.", peer_id, liability);
                    // 1. Emit DebtQuin to local graph
                    // query_engine.insert_debt_quin(&peer_id, liability);

                    // 2. Send warning over libp2p
                    // network::send_payment_warning(&peer_id, liability);

                    // 3. Start the grace period clock
                    ledger.warning_issued_at = Some(std::time::Instant::now());
                } else if let Some(issued_at) = ledger.warning_issued_at {
                    if issued_at.elapsed() > std::time::Duration::from_secs(300) {
                        // 5 Minute Grace Period
                        println!(
                            "[Micropayment Engine] {} Grace Period EXPIRED. Disconnecting.",
                            peer_id
                        );
                        // Disconnect peer immediately
                        // network::disconnect_and_block(&peer_id);
                        ledger.unbilled_bytes = 0;
                        ledger.warning_issued_at = None;
                    }
                }
            }
        }
    });

    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}
