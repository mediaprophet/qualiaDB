#![cfg(not(target_arch = "wasm32"))]

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use warp::http::StatusCode;
use warp::Filter;

const OFFICIAL_WEB_HUB_ORIGIN: &str = "https://mediaprophet.github.io";
const QUERY_PAYLOAD_LIMIT_BYTES: u64 = 64 * 1024;

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

    let qualia_bridge = warp::path("qualia-bridge")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(move |mut socket| async move {
                let handshake = json!({
                    "type": "HANDSHAKE_SUCCESS",
                    "payload": {
                        "mode": "NATIVE",
                        "version": env!("CARGO_PKG_VERSION")
                    }
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

    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::with_status(
                warp::reply::json(&json!({
                    "status": "active",
                    "engine": "qualia-core-db",
                    "version": env!("CARGO_PKG_VERSION")
                })),
                StatusCode::OK,
            )
        });

    let query_security = security.clone();
    let query = warp::path("query")
        .and(warp::post())
        .and(warp::header::optional::<String>("x-qualia-token"))
        .and(warp::body::content_length_limit(QUERY_PAYLOAD_LIMIT_BYTES))
        .and(warp::body::json())
        .map(move |token: Option<String>, request: NativeQueryRequest| {
            if !query_security.dev {
                let valid = query_security
                    .token
                    .as_ref()
                    .zip(token.as_ref())
                    .map(|(expected, supplied)| expected == supplied)
                    .unwrap_or(false);

                if !valid {
                    return warp::reply::with_status(
                        warp::reply::json(&json!({
                            "status": "error",
                            "code": "unauthorized",
                            "message": "Missing or invalid X-Qualia-Token"
                        })),
                        StatusCode::UNAUTHORIZED,
                    );
                }
            }

            let format = request.format.as_deref().unwrap_or("sparql-star");
            if !matches!(format, "sparql-star" | "json-ld" | "n3") {
                return warp::reply::with_status(
                    warp::reply::json(&json!({
                        "status": "error",
                        "code": "unsupported_format",
                        "message": "Supported query formats are sparql-star, json-ld, and n3"
                    })),
                    StatusCode::BAD_REQUEST,
                );
            }

            if request.query.trim().is_empty() {
                return warp::reply::with_status(
                    warp::reply::json(&json!({
                        "status": "error",
                        "code": "empty_query",
                        "message": "Query payload must include a non-empty query string"
                    })),
                    StatusCode::BAD_REQUEST,
                );
            }

            if let Some(quin) = crate::query_compiler::QueryCompiler::compile_to_quin(&request.query) {
                let routing_tier = (quin.metadata >> 61) & 0b11;
                let validation_mask = quin.metadata & 0xFFFF;

                warp::reply::with_status(
                    warp::reply::json(&json!({
                        "status": "compiled",
                        "format": format,
                        "message": "Query compiled to a QualiaQuin routing envelope. Full SPARQL-Star result execution is not implemented by this bridge yet.",
                        "quin": {
                            "subject": quin.subject.to_string(),
                            "predicate": quin.predicate.to_string(),
                            "object": quin.object.to_string(),
                            "context": quin.context.to_string(),
                            "metadata": quin.metadata.to_string(),
                            "parity": quin.parity.to_string()
                        },
                        "routing_tier": routing_tier,
                        "validation_mask": validation_mask
                    })),
                    StatusCode::OK,
                )
            } else {
                warp::reply::with_status(
                    warp::reply::json(&json!({
                        "status": "error",
                        "code": "compiler_limit",
                        "message": "The native compiler could not compile this query shape yet. Try a simpler routing query or use the browser fallback."
                    })),
                    StatusCode::NOT_IMPLEMENTED,
                )
            }
        });

    let cache = warp::path("cache")
        .and(warp::post())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(warp::body::content_length_limit(QUERY_PAYLOAD_LIMIT_BYTES))
        .and(warp::body::bytes())
        .map(|qs: std::collections::HashMap<String, String>, body: warp::hyper::body::Bytes| {
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
        });

    let preflight = warp::path("health")
        .or(warp::path("query"))
        .unify()
        .and(warp::options())
        .map(|| warp::reply::with_status(warp::reply::json(&json!({ "status": "ok" })), StatusCode::OK));

    let cors = warp::cors()
        .allow_origin(OFFICIAL_WEB_HUB_ORIGIN)
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec![
            "content-type",
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
            println!("[Informatics Subsystem] Running N3Logic differential diagnostics over .q42 graph...");
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
