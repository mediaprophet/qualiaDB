//! Companion gateway — phone WS + loopback POST ingest for Samsung health bundles.

use std::net::UdpSocket;
use std::sync::atomic::{AtomicU16, Ordering};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use qualia_client_core::wellfair::api::WebizenHostApi;
use rand::Rng;
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use wellfare_core::companion_pairing::{
    CompanionAuthResult, CompanionChallenge, CompanionPairingResponse, COMPANION_PAIRING_CONTEXT,
    MSG_PAIRING_RESPONSE,
};
use wellfare_core::live_share::{
    LiveSectionRequest, UsageAgreement, MSG_LIVE_SECTION_REQUEST, MSG_USAGE_AGREEMENT,
};

pub type HostApiHandle = Arc<Mutex<Option<WebizenHostApi>>>;

pub const DEFAULT_COMPANION_PORT: u16 = 8080;

static COMPANION_LISTEN_PORT: AtomicU16 = AtomicU16::new(DEFAULT_COMPANION_PORT);

pub fn set_companion_listen_port(port: u16) {
    COMPANION_LISTEN_PORT.store(port, Ordering::SeqCst);
}

pub fn companion_listen_port() -> u16 {
    COMPANION_LISTEN_PORT.load(Ordering::SeqCst)
}

pub fn guess_lan_ipv4() -> String {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return "127.0.0.1".into(),
    };
    if socket.connect("8.8.8.8:80").is_err() {
        return "127.0.0.1".into();
    }
    match socket.local_addr() {
        Ok(addr) if addr.ip().is_ipv4() => addr.ip().to_string(),
        _ => "127.0.0.1".into(),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CompanionPairingInfo {
    pub ws_url: String,
    pub lan_ip: String,
    pub port: u16,
    pub qr_path: String,
}

pub fn companion_pairing_info(port: u16) -> CompanionPairingInfo {
    let lan_ip = guess_lan_ipv4();
    let ws_url = format!("ws://{lan_ip}:{port}/mobile/stream");
    CompanionPairingInfo {
        ws_url,
        lan_ip,
        port,
        qr_path: "/mobile/qr".into(),
    }
}

pub fn companion_qr_svg(ws_url: &str) -> String {
    let qr = fast_qr::QRBuilder::new(ws_url)
        .build()
        .unwrap_or_else(|_| {
            fast_qr::QRBuilder::new("ws://127.0.0.1:8080/mobile/stream")
                .build()
                .unwrap()
        });
    fast_qr::convert::svg::SvgBuilder::default().to_str(&qr)
}

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

fn random_nonce_hex() -> String {
    let mut nonce = [0u8; 32];
    rand::rng().fill_bytes(&mut nonce);
    hex::encode(nonce)
}

fn verify_pairing_response(
    challenge: &CompanionChallenge,
    response: &CompanionPairingResponse,
) -> Result<(), String> {
    if response.msg_type != MSG_PAIRING_RESPONSE {
        return Err("expected PAIRING_RESPONSE".into());
    }
    if response.device_id.trim().is_empty() {
        return Err("device_id required".into());
    }
    if challenge.context != COMPANION_PAIRING_CONTEXT {
        return Err("invalid challenge context".into());
    }

    let nonce = hex::decode(&challenge.nonce_hex).map_err(|e| format!("bad nonce: {e}"))?;
    if nonce.len() != 32 {
        return Err("challenge nonce must be 32 bytes".into());
    }

    let pk_bytes: [u8; 32] = hex::decode(&response.public_key_hex)
        .map_err(|e| format!("bad public key: {e}"))?
        .try_into()
        .map_err(|_| "public key must be 32 bytes".to_string())?;
    let sig_bytes: [u8; 64] = hex::decode(&response.signature_hex)
        .map_err(|e| format!("bad signature: {e}"))?
        .try_into()
        .map_err(|_| "signature must be 64 bytes".to_string())?;

    let verifying_key =
        VerifyingKey::from_bytes(&pk_bytes).map_err(|_| "invalid Ed25519 public key".to_string())?;
    let signature = Signature::from_bytes(&sig_bytes);

    let mut payload = Vec::with_capacity(challenge.context.len() + 1 + nonce.len());
    payload.extend_from_slice(challenge.context.as_bytes());
    payload.push(0);
    payload.extend_from_slice(&nonce);

    verifying_key
        .verify(&payload, &signature)
        .map_err(|_| "signature verification failed".to_string())
}

fn register_usage_agreement_json(host_api: &HostApiHandle, agreement: &UsageAgreement) -> Result<(), String> {
    let guard = host_api.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    host.register_usage_agreement(agreement)
}

fn submit_live_share_request_json(
    host_api: &HostApiHandle,
    request: &LiveSectionRequest,
) -> Result<String, String> {
    let guard = host_api.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized — unlock vault first".to_string())?;
    let entry = host.submit_live_share_request(request)?;
    Ok(entry.id)
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
    ingest_bundle_json(&host_api, &body)
        .map(Json)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e))
}

pub async fn companion_ws_upgrade(
    State(host_api): State<HostApiHandle>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| companion_ws_session(socket, host_api))
}

async fn companion_ws_session(mut socket: WebSocket, host_api: HostApiHandle) {
    let challenge = CompanionChallenge::new(random_nonce_hex());
    let challenge_json = match serde_json::to_string(&challenge) {
        Ok(j) => j,
        Err(_) => return,
    };

    if socket
        .send(Message::Text(challenge_json.into()))
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
            match serde_json::from_str::<CompanionPairingResponse>(&text) {
                Ok(response) => match verify_pairing_response(&challenge, &response) {
                    Ok(()) => {
                        authenticated = true;
                        let ack = serde_json::to_string(&CompanionAuthResult::success())
                            .unwrap_or_else(|_| r#"{"type":"AUTH_SUCCESS"}"#.into());
                        if socket.send(Message::Text(ack.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(reason) => {
                        let deny = serde_json::to_string(&CompanionAuthResult::denied(reason))
                            .unwrap_or_else(|_| r#"{"type":"AUTH_DENIED"}"#.into());
                        let _ = socket.send(Message::Text(deny.into())).await;
                        break;
                    }
                },
                Err(_) => {
                    let deny = serde_json::to_string(&CompanionAuthResult::denied(
                        "expected PAIRING_RESPONSE JSON",
                    ))
                    .unwrap_or_else(|_| r#"{"type":"AUTH_DENIED"}"#.into());
                    let _ = socket.send(Message::Text(deny.into())).await;
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
                continue;
            }
        }

        if let Ok(agreement) = serde_json::from_str::<UsageAgreement>(&text) {
            if agreement.msg_type == MSG_USAGE_AGREEMENT {
                let ack = match register_usage_agreement_json(&host_api, &agreement) {
                    Ok(()) => serde_json::json!({
                        "type": "USAGE_AGREEMENT_ACK",
                        "ok": true,
                        "device_id": agreement.device_id,
                    }),
                    Err(e) => serde_json::json!({
                        "type": "USAGE_AGREEMENT_ACK",
                        "ok": false,
                        "errors": [e],
                    }),
                };
                if socket.send(Message::Text(ack.to_string().into())).await.is_err() {
                    break;
                }
                continue;
            }
        }

        if let Ok(request) = serde_json::from_str::<LiveSectionRequest>(&text) {
            if request.msg_type == MSG_LIVE_SECTION_REQUEST {
                let ack = match submit_live_share_request_json(&host_api, &request) {
                    Ok(journal_id) => serde_json::json!({
                        "type": "LIVE_SECTION_REQUEST_ACK",
                        "ok": true,
                        "request_id": request.id,
                        "journal_id": journal_id,
                        "status": "pending_owner_approval",
                    }),
                    Err(e) => serde_json::json!({
                        "type": "LIVE_SECTION_REQUEST_ACK",
                        "ok": false,
                        "request_id": request.id,
                        "errors": [e],
                    }),
                };
                if socket.send(Message::Text(ack.to_string().into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
    use ed25519_dalek::SigningKey;

    #[test]
    fn verify_valid_pairing_response() {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let nonce = [9u8; 32];
        let challenge = CompanionChallenge::new(hex::encode(nonce));

        let mut payload = Vec::new();
        payload.extend_from_slice(challenge.context.as_bytes());
        payload.push(0);
        payload.extend_from_slice(&nonce);
        let signature = signing_key.sign(&payload);

        let response = CompanionPairingResponse::new(
            "phone-test",
            hex::encode(signing_key.verifying_key().to_bytes()),
            hex::encode(signature.to_bytes()),
        );

        assert!(verify_pairing_response(&challenge, &response).is_ok());
    }

    #[test]
    fn pairing_info_includes_ws_url_and_port() {
        let info = companion_pairing_info(DEFAULT_COMPANION_PORT);
        assert!(info.ws_url.contains("/mobile/stream"));
        assert_eq!(info.port, DEFAULT_COMPANION_PORT);
        assert!(!info.lan_ip.is_empty());
    }

    #[test]
    fn qr_svg_is_non_empty() {
        let svg = companion_qr_svg("ws://192.168.1.10:8080/mobile/stream");
        assert!(svg.contains("<svg"));
    }
}