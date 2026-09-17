//! Host-injected rendering boundary for the POET loopback API.
//!
//! `qualia-core-db` owns routing and scene authoring but deliberately does not
//! depend on `webizen-render`. Native hosts register an implementation at cold
//! startup, avoiding a dependency cycle while keeping browser responses honest.

use std::sync::{Arc, OnceLock};

use axum::{
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use super::poet_api::POET_PAYLOAD_LIMIT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoetRenderRequest {
    pub kind: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PoetRenderResponse {
    pub ok: bool,
    pub kind: String,
    pub honesty: String,
    pub width: u32,
    pub height: u32,
    pub node_count: usize,
    pub edge_count: usize,
    pub face_count: usize,
    pub data_uri: Option<String>,
    pub diagnostic: Option<String>,
    pub contract: String,
}

pub trait PoetRenderProvider: Send + Sync + 'static {
    fn render_preview(&self, request: &PoetRenderRequest) -> PoetRenderResponse;
}

static RENDER_PROVIDER: OnceLock<Arc<dyn PoetRenderProvider>> = OnceLock::new();

/// Install the process-wide renderer during native host startup.
pub fn register_poet_render_provider(
    provider: Arc<dyn PoetRenderProvider>,
) -> Result<(), &'static str> {
    RENDER_PROVIDER
        .set(provider)
        .map_err(|_| "a POET render provider is already registered")
}

fn normalise_request(mut request: PoetRenderRequest) -> Result<PoetRenderRequest, &'static str> {
    request.kind = request.kind.trim().to_string();
    if request.kind.is_empty() || request.kind.len() > 64 {
        return Err("render kind must contain between 1 and 64 bytes");
    }
    request.width = Some(request.width.unwrap_or(960).clamp(160, 1920));
    request.height = Some(request.height.unwrap_or(480).clamp(120, 1080));
    Ok(request)
}

fn unavailable(request: &PoetRenderRequest, diagnostic: impl Into<String>) -> PoetRenderResponse {
    PoetRenderResponse {
        ok: false,
        kind: request.kind.clone(),
        honesty: "unavailable".into(),
        width: request.width.unwrap_or(960),
        height: request.height.unwrap_or(480),
        node_count: 0,
        edge_count: 0,
        face_count: 0,
        data_uri: None,
        diagnostic: Some(diagnostic.into()),
        contract: "webizen_render::scene_contract::RenderScene".into(),
    }
}

pub async fn render_preview_handler(body: Bytes) -> Response {
    if body.len() > POET_PAYLOAD_LIMIT_BYTES {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({
                "ok": false,
                "diagnostic": "POET render requests are limited to 64 KiB"
            })),
        )
            .into_response();
    }

    let request = match serde_json::from_slice::<PoetRenderRequest>(&body)
        .map_err(|error| error.to_string())
        .and_then(|request| normalise_request(request).map_err(str::to_string))
    {
        Ok(request) => request,
        Err(diagnostic) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "ok": false, "diagnostic": diagnostic })),
            )
                .into_response();
        }
    };

    let Some(provider) = RENDER_PROVIDER.get().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(unavailable(
                &request,
                "This daemon host has no registered webizen-render provider",
            )),
        )
            .into_response();
    };

    let fallback = request.clone();
    match tokio::task::spawn_blocking(move || provider.render_preview(&request)).await {
        Ok(result) => Json(result).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(unavailable(
                &fallback,
                format!("Renderer worker failed: {error}"),
            )),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_dimensions_are_bounded() {
        let request = normalise_request(PoetRenderRequest {
            kind: " map ".into(),
            width: Some(1),
            height: Some(u32::MAX),
        })
        .unwrap();
        assert_eq!(request.kind, "map");
        assert_eq!(request.width, Some(160));
        assert_eq!(request.height, Some(1080));
    }

    #[tokio::test]
    async fn oversized_request_is_rejected_before_provider_lookup() {
        let response =
            render_preview_handler(Bytes::from(vec![b'x'; POET_PAYLOAD_LIMIT_BYTES + 1])).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn missing_provider_is_reported_as_unavailable() {
        let response = render_preview_handler(Bytes::from_static(
            br#"{"kind":"map","width":320,"height":180}"#,
        ))
        .await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
