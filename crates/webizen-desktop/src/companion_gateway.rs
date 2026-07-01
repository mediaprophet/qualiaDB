//! Companion gateway — phone WS + loopback POST ingest for Samsung health bundles.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use qualia_client_core::wellfair::api::WebizenHostApi;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub type HostApiHandle = Arc<Mutex<Option<WebizenHostApi>>>;

const CHALLENGE: &str = "CHALLENGE_BYTES_123456789";
const AUTH_OK: &str = "AUTH_SUCCESS";

#[derive(Debug, Deserialize)]
struct HealthBundleWire {
    #[serde(rename = "type")]
    msg_type: String,
    bundle: Value,
}

#[derive(Debug, serde::Serialize)]
pub struct IngestAck {
    pub ok: bool,
    pub records_committed: usize,
    pub records_skipped: usize,
    pub errors: Vec<String>,
}

fn ingest_bundle_json(host_api: &HostApiHandle, bundle_json: &str) -> Result<IngestAck, String> {
    let bundle: wellfare_core::companion_sync::CompanionHealthBundle =
        serde_json::from_str(bundle_json).map_err(|e| format!("invalid bundle JSON: {e}"))?;
    let mut guard = host_api.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_mut()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let report = host.ingest_companion_health_bundle(&bundle);
    Ok(IngestAck {
        ok: report.errors.is_empty(),
        records_committed: report.records_committed,
        records_skipped: report.records_skipped,
        errors: report.errors,
    })
}

pub async fn companion_ingest_post(
    State(host_api): State<HostApiHandle>,
    body: String,
) -> Result<Json<IngestAck>, (axum::http::StatusCode, String)> {
    ingest_bundle_json(&host_api, &body).map(Json).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            e,
        )
    })
}

pub async fn companion_ws_upgrade(
    State(host_api): State<HostApiHandle>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| companion_ws_session(socket, host_api))
}

async fn companion_ws_session(mut socket: WebSocket, host_api: HostApiHandle) {
    if socket
        .send(Message::Text(CHALLENGE.into()))
        .await
        .is_err()
    {
        return;
    }

    let mut authenticated = false;

    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
            Message::Close(_) => break,
            _ => continue,
        };

        if !authenticated {
            if !text.is_empty() {
                authenticated = true;
                if socket
                    .send(Message::Text(AUTH_OK.into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
            continue;
        }

        if let Ok(wire) = serde_json::from_str::<HealthBundleWire>(&text) {
            if wire.msg_type == "HEALTH_BUNDLE" {
                let bundle_json = wire.bundle.to_string();
                let ack = match ingest_bundle_json(&host_api, &bundle_json) {
                    Ok(ack) => serde_json::json!({
                        "type": "HEALTH_BUNDLE_ACK",
                        "ok": ack.ok,
                        "records_committed": ack.records_committed,
                        "records_skipped": ack.records_skipped,
                        "errors": ack.errors,
                    }),
                    Err(e) => serde_json::json!({
                        "type": "HEALTH_BUNDLE_ACK",
                        "ok": false,
                        "errors": [e],
                    }),
                };
                if socket
                    .send(Message::Text(ack.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    }
}