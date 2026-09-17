//! Host-injected Semantic Library boundary for the standalone POET browser.
//!
//! The loopback router stays independent of `qualia-client-core`; native hosts
//! inject the persistent HypermediaStore implementation during cold startup.

use std::sync::{Arc, OnceLock};

use axum::{
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

pub const LIBRARY_REQUEST_LIMIT_BYTES: usize = 2 * 1024 * 1024;
const LIBRARY_TEXT_LIMIT_BYTES: usize = 1024 * 1024;
const MAX_FACET_VALUES: usize = 32;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PoetLibraryQueryRequest {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub purposes: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub media_types: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoetLibraryIngestRequest {
    pub uri: String,
    #[serde(default = "default_media_type")]
    pub media_type: String,
    pub text: String,
    #[serde(default)]
    pub section: Option<String>,
    #[serde(default)]
    pub sensitivity: Option<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub purposes: Vec<String>,
    #[serde(default)]
    pub occurred_at: Option<i64>,
    #[serde(default)]
    pub place_label: Option<String>,
    #[serde(default)]
    pub lat: Option<f32>,
    #[serde(default)]
    pub lon: Option<f32>,
}

fn default_media_type() -> String {
    "text/markdown".into()
}

pub trait PoetLibraryProvider: Send + Sync + 'static {
    fn stats(&self) -> Result<serde_json::Value, String>;
    fn query(&self, request: &PoetLibraryQueryRequest) -> Result<serde_json::Value, String>;
    fn ingest(&self, request: &PoetLibraryIngestRequest) -> Result<serde_json::Value, String>;
}

static LIBRARY_PROVIDER: OnceLock<Arc<dyn PoetLibraryProvider>> = OnceLock::new();

pub fn register_poet_library_provider(
    provider: Arc<dyn PoetLibraryProvider>,
) -> Result<(), &'static str> {
    LIBRARY_PROVIDER
        .set(provider)
        .map_err(|_| "a POET Semantic Library provider is already registered")
}

fn diagnostic(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "honesty": "unavailable",
            "code": code,
            "diagnostic": message.into()
        })),
    )
        .into_response()
}

fn decode<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, Response> {
    if body.len() > LIBRARY_REQUEST_LIMIT_BYTES {
        return Err(diagnostic(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Semantic Library requests are limited to 2 MiB",
        ));
    }
    serde_json::from_slice(body)
        .map_err(|error| diagnostic(StatusCode::BAD_REQUEST, "invalid_json", error.to_string()))
}

fn validate_query(request: &mut PoetLibraryQueryRequest) -> Result<(), &'static str> {
    request.query = request.query.trim().to_string();
    if request.query.len() > 512 {
        return Err("library query cannot exceed 512 bytes");
    }
    if let Some(section) = request.section.as_mut() {
        *section = section.trim().to_ascii_lowercase();
        if section.len() > 32 {
            return Err("library section cannot exceed 32 bytes");
        }
    }
    for values in [
        &request.topics,
        &request.purposes,
        &request.projects,
        &request.media_types,
        &request.categories,
    ] {
        if values.len() > MAX_FACET_VALUES || values.iter().any(|value| value.len() > 128) {
            return Err("each facet supports at most 32 values of at most 128 bytes");
        }
    }
    Ok(())
}

fn validate_ingest(request: &mut PoetLibraryIngestRequest) -> Result<(), &'static str> {
    request.uri = request.uri.trim().to_string();
    request.media_type = request.media_type.trim().to_ascii_lowercase();
    if request.uri.is_empty() || request.uri.len() > 2048 {
        return Err("asset URI must contain between 1 and 2048 bytes");
    }
    if request.media_type.is_empty() || request.media_type.len() > 128 {
        return Err("media type must contain between 1 and 128 bytes");
    }
    if request.text.trim().is_empty() || request.text.len() > LIBRARY_TEXT_LIMIT_BYTES {
        return Err("document text must contain between 1 byte and 1 MiB");
    }
    if request.projects.len() > MAX_FACET_VALUES
        || request.purposes.len() > MAX_FACET_VALUES
        || request
            .projects
            .iter()
            .chain(&request.purposes)
            .any(|value| value.len() > 128)
    {
        return Err("ingest facets support at most 32 values of at most 128 bytes");
    }
    if let (Some(lat), Some(lon)) = (request.lat, request.lon) {
        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lon) {
            return Err("latitude or longitude is outside its valid range");
        }
    } else if request.lat.is_some() || request.lon.is_some() {
        return Err("latitude and longitude must be supplied together");
    }
    Ok(())
}

fn provider() -> Result<Arc<dyn PoetLibraryProvider>, Response> {
    LIBRARY_PROVIDER.get().cloned().ok_or_else(|| {
        diagnostic(
            StatusCode::SERVICE_UNAVAILABLE,
            "provider_unavailable",
            "This daemon host has no registered Semantic Library provider",
        )
    })
}

fn provider_result(result: Result<serde_json::Value, String>) -> Response {
    match result {
        Ok(data) => Json(serde_json::json!({
            "ok": true,
            "honesty": "live",
            "data": data
        }))
        .into_response(),
        Err(error) => diagnostic(
            StatusCode::UNPROCESSABLE_ENTITY,
            "library_operation_failed",
            error,
        ),
    }
}

pub async fn stats_handler() -> Response {
    let provider = match provider() {
        Ok(provider) => provider,
        Err(response) => return response,
    };
    match tokio::task::spawn_blocking(move || provider.stats()).await {
        Ok(result) => provider_result(result),
        Err(error) => diagnostic(
            StatusCode::INTERNAL_SERVER_ERROR,
            "library_worker_failed",
            error.to_string(),
        ),
    }
}

pub async fn query_handler(body: Bytes) -> Response {
    let mut request: PoetLibraryQueryRequest = match decode(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if let Err(error) = validate_query(&mut request) {
        return diagnostic(StatusCode::BAD_REQUEST, "invalid_query", error);
    }
    let provider = match provider() {
        Ok(provider) => provider,
        Err(response) => return response,
    };
    match tokio::task::spawn_blocking(move || provider.query(&request)).await {
        Ok(result) => provider_result(result),
        Err(error) => diagnostic(
            StatusCode::INTERNAL_SERVER_ERROR,
            "library_worker_failed",
            error.to_string(),
        ),
    }
}

pub async fn ingest_handler(body: Bytes) -> Response {
    let mut request: PoetLibraryIngestRequest = match decode(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    if let Err(error) = validate_ingest(&mut request) {
        return diagnostic(StatusCode::BAD_REQUEST, "invalid_ingest", error);
    }
    let provider = match provider() {
        Ok(provider) => provider,
        Err(response) => return response,
    };
    match tokio::task::spawn_blocking(move || provider.ingest(&request)).await {
        Ok(result) => provider_result(result),
        Err(error) => diagnostic(
            StatusCode::INTERNAL_SERVER_ERROR,
            "library_worker_failed",
            error.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_facet_count_is_bounded() {
        let mut request = PoetLibraryQueryRequest {
            topics: vec!["x".into(); MAX_FACET_VALUES + 1],
            ..Default::default()
        };
        assert!(validate_query(&mut request).is_err());
    }

    #[test]
    fn ingest_requires_paired_valid_coordinates() {
        let mut request = PoetLibraryIngestRequest {
            uri: "urn:test".into(),
            media_type: "text/plain".into(),
            text: "hello".into(),
            section: None,
            sensitivity: None,
            projects: Vec::new(),
            purposes: Vec::new(),
            occurred_at: None,
            place_label: None,
            lat: Some(91.0),
            lon: Some(0.0),
        };
        assert!(validate_ingest(&mut request).is_err());
        request.lat = Some(45.0);
        request.lon = None;
        assert!(validate_ingest(&mut request).is_err());
    }

    #[tokio::test]
    async fn missing_provider_is_explicit() {
        assert_eq!(
            stats_handler().await.status(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    #[tokio::test]
    async fn oversized_request_is_rejected_before_provider_lookup() {
        let response =
            query_handler(Bytes::from(vec![b'x'; LIBRARY_REQUEST_LIMIT_BYTES + 1])).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
