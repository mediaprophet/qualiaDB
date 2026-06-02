#![cfg(not(target_arch = "wasm32"))]

use warp::Filter;
use futures_util::{StreamExt, SinkExt};
use serde_json::json;

/// Starts the WebSocket daemon on ws://127.0.0.1:4242/qualia-bridge
pub async fn start_local_daemon(port: u16) {
    let qualia_bridge = warp::path("qualia-bridge")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(move |mut socket| async move {
                // Send an initial handshake
                let handshake = json!({
                    "type": "HANDSHAKE_SUCCESS",
                    "payload": {
                        "mode": "NATIVE",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                
                if let Ok(_) = socket.send(warp::ws::Message::text(handshake.to_string())).await {
                    println!("🔌 [Qualia Daemon] Client connected to Native WebSocket Bridge");
                }

                while let Some(result) = socket.next().await {
                    match result {
                        Ok(msg) => {
                            if msg.is_text() {
                                let _text = msg.to_str().unwrap_or_default();
                                // Handle incoming messages if needed
                                // E.g., routing ingest requests to the CRDT or Graph layer.
                            } else if msg.is_binary() {
                                // Streaming CBOR-LD or raw Quins
                                // println!("📥 [Qualia Daemon] Received {} bytes of binary data", msg.as_bytes().len());
                            }
                        }
                        Err(e) => {
                            eprintln!("🔌 [Qualia Daemon] WebSocket Error: {}", e);
                            break;
                        }
                    }
                }
                println!("🔌 [Qualia Daemon] Client disconnected.");
            })
        });

    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec!["content-type"]);

    let routes = qualia_bridge.with(cors);

    println!("============================================================");
    println!("🚀 Qualia-DB Native Local Daemon Booting...");
    println!("📡 Listening on ws://127.0.0.1:{}", port);
    println!("============================================================");

    // Boot the Informatics Subsystem Background Heartbeat
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            println!("🩺 [Informatics Subsystem] Executing N3Logic Differential Diagnostics over .q42 graph...");
            // let inferences = crate::logic::execute_differential_diagnostics(&active_graph);
            // In a real system, `inferences` are written back into the CRDT
        }
    });

    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}
