use tokio::net::TcpListener;
use tokio::sync::broadcast;
use futures_util::sink::SinkExt;
use serde::Serialize;
use sysinfo::System;

#[derive(Serialize, Clone)]
pub struct TelemetryPayload {
    pub r#type: String, // "telemetry"
    pub rss_mb: f64,
    pub blocks_loaded: usize,
    pub hot_blocks: Vec<HotBlock>,
}

#[derive(Serialize, Clone)]
pub struct HotBlock {
    pub id: u64,
    pub source: String, // "local" or "remote"
}

pub async fn start_telemetry_server(rx: broadcast::Receiver<TelemetryPayload>) {
    let listener = TcpListener::bind("127.0.0.1:9090").await.expect("Failed to bind telemetry port");
    println!("📡 Telemetry WebSocket server running on ws://127.0.0.1:9090");

    while let Ok((stream, _)) = listener.accept().await {
        let mut rx_clone = rx.resubscribe();
        
        tokio::spawn(async move {
            let mut ws_stream = tokio_tungstenite::accept_async(stream).await.expect("Error during websocket handshake");
            
            while let Ok(payload) = rx_clone.recv().await {
                let json = serde_json::to_string(&payload).unwrap();
                if ws_stream.send(tokio_tungstenite::tungstenite::Message::Text(json)).await.is_err() {
                    break;
                }
            }
        });
    }
}

pub fn get_peak_rss(sys: &mut System) -> f64 {
    sys.refresh_all();
    let pid = sysinfo::get_current_pid().unwrap();
    if let Some(process) = sys.process(pid) {
        process.memory() as f64 / 1_048_576.0
    } else {
        0.0
    }
}
