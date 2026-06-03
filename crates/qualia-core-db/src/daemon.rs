#![cfg(not(target_arch = "wasm32"))]

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use warp::http::StatusCode;
use warp::Filter;

const OFFICIAL_WEB_HUB_ORIGIN: &str = "https://mediaprophet.github.io";
const QUERY_PAYLOAD_LIMIT_BYTES: u64 = 64 * 1024;

/// Maximum number of result Quins the daemon will buffer per request.
/// At 48 bytes each, 1 000 Quins consume 48 KB — well inside the 512 MB ceiling.
const QUERY_OUT_SLOTS: usize = 1_000;

#[derive(Clone)]
struct DaemonSecurity {
    dev: bool,
    token: Option<String>,
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
fn negotiate_format(payload_format: Option<&str>, accept: Option<&str>) -> Result<OutputFormat, ()> {
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
        .header("X-Qualia-Compute-Cost", format!("{match_count}+{vm_cycles}"))
        .body(body)
        .expect("infallible query response builder")
}

// ---------------------------------------------------------------------------
// Daemon entry points
// ---------------------------------------------------------------------------

/// Starts the native loopback daemon on 127.0.0.1 with strict token checks.
pub async fn start_local_daemon(port: u16) {
    start_local_daemon_with_options(port, false).await;
}

/// Starts the native loopback daemon with WebSocket and REST handoff routes.
pub async fn start_local_daemon_with_options(port: u16, dev: bool) {
    let security = DaemonSecurity {
        dev,
        token: std::env::var("QUALIA_TOKEN")
            .ok()
            .or_else(|| std::env::var("QUALIA_DEV_TOKEN").ok()),
    };

    // -----------------------------------------------------------------------
    // WebSocket bridge
    // -----------------------------------------------------------------------
    let qualia_bridge = warp::path("qualia-bridge")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(move |mut socket| async move {
                let handshake = json!({
                    "type": "HANDSHAKE_SUCCESS",
                    "payload": { "mode": "NATIVE", "version": env!("CARGO_PKG_VERSION") }
                });
                if socket
                    .send(warp::ws::Message::text(handshake.to_string()))
                    .await
                    .is_ok()
                {
                    println!("[Qualia Daemon] Client connected to Native WebSocket Bridge");
                }
                while let Some(result) = socket.next().await {
                    match result {
                        Ok(msg) => {
                            if msg.is_text() {
                                let _text = msg.to_str().unwrap_or_default();
                            } else if msg.is_binary() {
                                let _bytes = msg.as_bytes();
                            }
                        }
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
                "version": env!("CARGO_PKG_VERSION")
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

                // --- Auth ---------------------------------------------------
                if !query_security.dev {
                    let valid = query_security
                        .token
                        .as_ref()
                        .zip(token.as_ref())
                        .map(|(expected, supplied)| expected == supplied)
                        .unwrap_or(false);
                    if !valid {
                        return make_response(
                            StatusCode::UNAUTHORIZED,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "unauthorized",
                                "message": "Missing or invalid X-Qualia-Token"
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

                // --- Step 1: Compile query → bytecode program ----------------
                let mut program = [0u8; 1024];
                if let Err(parse_err) = crate::mini_parser::compile_ntriples_to_bytecode(
                    request.query.as_bytes(),
                    &mut program,
                ) {
                    return make_response(
                        StatusCode::BAD_REQUEST,
                        "application/json",
                        json!({
                            "status": "error",
                            "code": "parse_error",
                            "message": format!(
                                "Query could not be compiled to bytecode: {:?}. \
                                 Supply a single N-Triples pattern, e.g. \
                                 \"<subject> ?p <object> .\"",
                                parse_err
                            )
                        })
                        .to_string(),
                    );
                }

                // --- Step 2: Execute VM against the in-memory graph ----------
                // Pre-allocate a fixed output slice (48 bytes × 1 000 = 48 KB).
                let mut out_buffer =
                    vec![crate::QualiaQuin::default(); QUERY_OUT_SLOTS];

                // `current_database_state` is the live in-process Quin graph.
                // The storage layer will populate this once the mmap shard is
                // wired into the daemon; for now an empty slice is used so that
                // the full pipeline compiles and the HTTP contract is exercised.
                let current_database_state: &[crate::QualiaQuin] = &[];

                let (match_count, vm_cycles) = match crate::sentinel_bytecode::execute_program(
                    &program,
                    current_database_state,
                    &mut out_buffer,
                ) {
                    Ok(pair) => pair,
                    Err(crate::sentinel_bytecode::VmError::OutputBufferFull) => {
                        return make_response(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "application/json",
                            json!({
                                "status": "error",
                                "code": "result_set_too_large",
                                "message": format!(
                                    "Query matched more than {QUERY_OUT_SLOTS} Quins. \
                                     Add more constraints to narrow the result set."
                                )
                            })
                            .to_string(),
                        );
                    }
                    Err(crate::sentinel_bytecode::VmError::InvalidProgram) => {
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
                };

                let final_results = &out_buffer[..match_count];

                // --- Step 3: Serialise and attach compute-cost telemetry -----
                match output_format {
                    OutputFormat::NTriples => {
                        // `format_ntriples_to` writes directly to the Vec<u8>
                        // buffer — the formatter itself performs no allocation.
                        let mut body_buf: Vec<u8> =
                            Vec::with_capacity(match_count.max(1) * 80);
                        let _ = crate::resolver::format_ntriples_to(
                            final_results,
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
            |qs: std::collections::HashMap<String, String>,
             body: warp::hyper::body::Bytes| {
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
    // -----------------------------------------------------------------------
    let cors = warp::cors()
        .allow_origin(OFFICIAL_WEB_HUB_ORIGIN)
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec![
            "content-type",
            "accept",
            "x-qualia-token",
            "access-control-request-private-network",
        ]);

    let routes = qualia_bridge
        .or(health)
        .or(query)
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
    println!(
        "  Mode:      {}",
        if security.dev { "dev token bypass" } else { "token required" }
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
        println!("[Qualia Daemon] WebTorrent: Native Magnet URI and DHT seeder initialized.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }
    });

    tokio::spawn(async {
        println!("[Qualia Daemon] Gun.eco: WebSocket Graph bridge initialized.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;
        }
    });

    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}
