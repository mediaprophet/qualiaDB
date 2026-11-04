//! Trusted product setup for the existing authenticated Webizen HTTP transport.
//! Request DID fields are labels; they cannot select the authenticated actor.
use super::webizen_server::WebizenState;
use crate::llm_agent::{native_request::NativeProductSession, LocalLlmAgent};
use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::sync::{Arc, Mutex, OnceLock};

static SESSION: OnceLock<Mutex<Option<Arc<NativeProductSession>>>> = OnceLock::new();

/// Privileged LOCAL setup only. Do not register as an HTTP/MCP signing endpoint.
/// The session has already proved possession under signed native admission and
/// retains the configured host's shared revocation state and file leases.
pub fn configure(session: Arc<NativeProductSession>) {
    super::poet_llm_jobs::cancel_all();
    *SESSION
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(session);
}

pub fn disconnect() {
    super::poet_llm_jobs::cancel_all();
    if let Some(slot) = SESSION.get() {
        *slot.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

/// No development bypass: a local server token is transport authentication,
/// separate from the independently installed actor/policy authorization.
pub(super) fn authenticate(
    state: &WebizenState,
    headers: &HeaderMap,
) -> Result<Arc<NativeProductSession>, Response> {
    let token = headers.get("x-qualia-token").and_then(|v| v.to_str().ok());
    if !state
        .token
        .as_deref()
        .is_some_and(|expected| !expected.is_empty() && Some(expected) == token)
    {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "ok": false, "code": "inference_transport_authentication",
                "diagnostic": "Authenticated native Webizen transport is required"
            })),
        )
            .into_response());
    }
    let session = SESSION.get().and_then(|slot| slot.lock().ok().and_then(|s| s.clone()))
        .ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({
            "ok": false, "code": "inference_host_not_configured",
            "diagnostic": "Native host must bind an authenticated actor, installed policy and model preparation budgets"
        }))).into_response())?;
    // Recheck the actual session for history/cancellation as well as decode.
    // Strings in requests cannot restore an expired or disconnected owner.
    session.bind().map_err(|_| {
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "ok": false, "code": "inference_session_expired",
                "diagnostic": "Native inference session expired or was disconnected"
            })),
        )
            .into_response()
    })?;
    Ok(session)
}

pub(super) fn bind(agent: &LocalLlmAgent, session: &NativeProductSession) -> Result<(), String> {
    agent
        .bind_native_governed_request(session.bind().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
