//! Bounded HTTP adapters for POET's native evaluation capabilities.
//!
//! These handlers expose the same `PoetSnapshot` and NLP implementations used
//! by the desktop host. They deliberately return execution failures as typed
//! JSON responses so browser clients can render honest diagnostics.

use std::collections::BTreeMap;

use axum::{
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::poet_host::{format_value, PoetSnapshot};

pub const POET_PAYLOAD_LIMIT_BYTES: usize = 64 * 1024;
const MAX_JSON_DEPTH: usize = 32;
const MAX_JSON_NODES: usize = 4096;

#[derive(Debug, Deserialize)]
pub struct NativeEvalRequest {
    pub source: String,
    #[serde(default)]
    pub as_cell: bool,
    #[serde(default)]
    pub function: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NativeEvalResponse {
    pub ok: bool,
    pub value: String,
    pub diagnostic: Option<String>,
    pub revision: u64,
    pub committed: usize,
    pub honesty: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct NativeInvokeRequest {
    pub id: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct NativeGazetteerRequest {
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct GazetteerHitDto {
    pub surface: String,
    pub iri: String,
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct NativeGazetteerResponse {
    pub ok: bool,
    pub token_count: usize,
    pub sentence_count: usize,
    pub sealed: usize,
    pub hits: Vec<GazetteerHitDto>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IntentTarget {
    value: String,
}

#[derive(Debug, Deserialize)]
struct NativeIntentEnvelope {
    action_type: String,
    target_identifier: IntentTarget,
    #[serde(default)]
    parameters: serde_json::Value,
}

fn decode_json<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, Response> {
    if body.len() > POET_PAYLOAD_LIMIT_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "ok": false,
                "code": "payload_too_large",
                "diagnostic": format!("POET requests are limited to {POET_PAYLOAD_LIMIT_BYTES} bytes")
            })),
        )
            .into_response());
    }
    serde_json::from_slice(body).map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "code": "invalid_json",
                "diagnostic": error.to_string()
            })),
        )
            .into_response()
    })
}

fn eval_response(
    snapshot: &PoetSnapshot,
    result: Result<vibe::Value, vibe::Diagnostic>,
) -> NativeEvalResponse {
    match result {
        Ok(value) => NativeEvalResponse {
            ok: true,
            value: format_value(&value),
            diagnostic: None,
            revision: snapshot.revision,
            committed: snapshot.committed.len(),
            honesty: "live",
        },
        Err(error) => NativeEvalResponse {
            ok: false,
            value: String::new(),
            diagnostic: Some(error.to_string()),
            revision: snapshot.revision,
            committed: snapshot.committed.len(),
            honesty: "live",
        },
    }
}

pub async fn eval_handler(body: Bytes) -> Response {
    let request: NativeEvalRequest = match decode_json(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };

    let mut snapshot = PoetSnapshot::from_daemon();
    let result = if let Some(function) = request.function.as_deref() {
        snapshot.eval_fn(&request.source, function, Vec::new())
    } else if request.as_cell {
        snapshot.eval_cell_src(&request.source)
    } else {
        snapshot.eval_program_src(&request.source)
    };
    Json(eval_response(&snapshot, result)).into_response()
}

fn json_to_vibe(
    value: serde_json::Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<vibe::Value, &'static str> {
    if depth > MAX_JSON_DEPTH {
        return Err("invoke arguments exceed the maximum nesting depth");
    }
    *nodes += 1;
    if *nodes > MAX_JSON_NODES {
        return Err("invoke arguments contain too many values");
    }

    Ok(match value {
        serde_json::Value::Null => vibe::Value::Null,
        serde_json::Value::Bool(value) => vibe::Value::Bool(value),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                vibe::Value::I64(value)
            } else if let Some(value) = value.as_u64() {
                vibe::Value::U64(value)
            } else if let Some(value) = value.as_f64() {
                vibe::Value::F64(value)
            } else {
                return Err("invoke argument contains an unsupported number");
            }
        }
        serde_json::Value::String(value) => vibe::Value::String(value),
        serde_json::Value::Array(values) => vibe::Value::List(
            values
                .into_iter()
                .map(|value| json_to_vibe(value, depth + 1, nodes))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        serde_json::Value::Object(values) => {
            let mut record = BTreeMap::new();
            for (key, value) in values {
                record.insert(key, json_to_vibe(value, depth + 1, nodes)?);
            }
            vibe::Value::Record(record)
        }
    })
}

pub async fn invoke_handler(body: Bytes) -> Response {
    let request: NativeInvokeRequest = match decode_json(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if request.id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "ok": false,
                "code": "missing_capability_id",
                "diagnostic": "The invoke capability id cannot be empty"
            })),
        )
            .into_response();
    }

    let mut nodes = 0;
    let args = match json_to_vibe(request.args, 0, &mut nodes) {
        Ok(args) => args,
        Err(diagnostic) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "ok": false,
                    "code": "invalid_arguments",
                    "diagnostic": diagnostic
                })),
            )
                .into_response();
        }
    };

    let mut snapshot = PoetSnapshot::from_daemon();
    let result = snapshot.invoke_id(request.id.trim(), args);
    Json(eval_response(&snapshot, result)).into_response()
}

pub async fn gazetteer_handler(body: Bytes) -> Response {
    let request: NativeGazetteerRequest = match decode_json(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let analysis = crate::nlp::analyze_document(&request.source);
    let hits = analysis
        .hits
        .iter()
        .map(|hit| GazetteerHitDto {
            surface: hit
                .span
                .slice(&request.source)
                .unwrap_or(hit.surface)
                .to_string(),
            iri: hit.iri.to_string(),
            kind: "gazetteer".to_string(),
        })
        .collect();

    Json(NativeGazetteerResponse {
        ok: true,
        token_count: analysis.token_count,
        sentence_count: analysis.sentence_count,
        sealed: analysis.plans.len(),
        hits,
        diagnostic: None,
    })
    .into_response()
}

fn intent_error(code: &'static str, diagnostic: impl Into<String>) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(serde_json::json!({
            "ok": false,
            "status": "rejected",
            "code": code,
            "diagnostic": diagnostic.into()
        })),
    )
        .into_response()
}

/// Dispatch a CBOR-LD Tool Chest envelope through supported native contracts.
pub async fn intent_handler(body: Bytes) -> Response {
    if body.len() > POET_PAYLOAD_LIMIT_BYTES {
        return intent_error(
            "payload_too_large",
            "Intent exceeds the 64 KiB request limit",
        );
    }
    let envelope: NativeIntentEnvelope = match ciborium::de::from_reader(body.as_ref()) {
        Ok(envelope) => envelope,
        Err(error) => return intent_error("invalid_cbor_ld", error.to_string()),
    };

    match envelope.action_type.as_str() {
        "query" => {
            let Some(query) = envelope
                .parameters
                .get("query")
                .and_then(|value| value.as_str())
            else {
                return intent_error(
                    "missing_query",
                    "Query intent parameters need a `query` string",
                );
            };
            let graph = crate::daemon_graph::graph_read_guard();
            match crate::daemon_query::execute_query_on_graph(query, graph.as_slice()) {
                Ok((stats, _)) => Json(serde_json::json!({
                    "ok": true,
                    "status": "accepted",
                    "match_count": stats.match_count
                }))
                .into_response(),
                Err(error) => intent_error("query_failed", format!("{error:?}")),
            }
        }
        "invoke" | "validate" => {
            let capability = if envelope.action_type == "validate" {
                "SHACL.validate".to_string()
            } else {
                envelope
                    .parameters
                    .get("id")
                    .and_then(|value| value.as_str())
                    .unwrap_or(&envelope.target_identifier.value)
                    .to_string()
            };
            let json_args = envelope
                .parameters
                .get("args")
                .cloned()
                .unwrap_or(envelope.parameters);
            let mut nodes = 0;
            let args = match json_to_vibe(json_args, 0, &mut nodes) {
                Ok(args) => args,
                Err(error) => return intent_error("invalid_arguments", error),
            };
            let mut snapshot = PoetSnapshot::from_daemon();
            match snapshot.invoke_id(&capability, args) {
                Ok(value) => Json(serde_json::json!({
                    "ok": true,
                    "status": "accepted",
                    "value": format_value(&value),
                    "revision": snapshot.revision
                }))
                .into_response(),
                Err(error) => intent_error("invoke_failed", error.to_string()),
            }
        }
        "mutate" | "publish" | "annotate" => intent_error(
            "typed_contract_required",
            "This state-changing intent needs a dedicated typed and authorised contract",
        ),
        "navigate" => intent_error(
            "local_action",
            "Navigation is a local UI action and must not be sent to the daemon",
        ),
        "cancel" => intent_error(
            "no_cancellable_job",
            "No native cancellable job identifier was supplied",
        ),
        other => intent_error("unknown_action", format!("Unknown intent action `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_conversion_preserves_records_and_lists() {
        let mut nodes = 0;
        let converted = json_to_vibe(
            serde_json::json!({"name": "POET", "values": [1, true]}),
            0,
            &mut nodes,
        )
        .unwrap();
        let vibe::Value::Record(record) = converted else {
            panic!("expected record");
        };
        assert_eq!(
            record.get("name"),
            Some(&vibe::Value::String("POET".into()))
        );
        assert!(
            matches!(record.get("values"), Some(vibe::Value::List(values)) if values.len() == 2)
        );
    }

    #[tokio::test]
    async fn gazetteer_uses_real_document_analyzer() {
        let body =
            Bytes::from_static(br#"{"source":"North Spring recorded 12.5 mm on 2026-08-15."}"#);
        let response = gazetteer_handler(body).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn oversized_payload_is_rejected() {
        let response = eval_handler(Bytes::from(vec![b'x'; POET_PAYLOAD_LIMIT_BYTES + 1])).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn unsupported_state_change_intent_fails_closed() {
        let envelope = serde_json::json!({
            "@context": "https://qualiadb.org/schema/vibe#",
            "action_type": "publish",
            "target_identifier": { "kind": "iri", "value": "pulse:test" },
            "parameters": { "message": "not authorised" }
        });
        let mut body = Vec::new();
        ciborium::ser::into_writer(&envelope, &mut body).unwrap();
        let response = intent_handler(Bytes::from(body)).await;
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
