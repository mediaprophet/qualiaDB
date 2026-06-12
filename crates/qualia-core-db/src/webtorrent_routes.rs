//! HTTP routes for the Qualia-native WebTorrent seeder.

#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;
use axum::{
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum::http::HeaderMap;
use axum::body::Body;
use serde::Serialize;
use serde_json::json;

use crate::webtorrent_seeder::{
    self, RegisterSeedRequest, SeederBandwidthPolicy, UnregisterSeedRequest,
};

#[derive(Clone)]
pub struct TorrentState {
    pub daemon_port: u16,
}

fn parse_range(range_header: &str, file_size: u64) -> Option<(u64, u64)> {
    let trimmed = range_header.trim();
    let rest = trimmed.strip_prefix("bytes=")?;
    let (start_s, end_s) = rest.split_once('-')?;
    let start: u64 = start_s.parse().ok()?;
    let end = if end_s.is_empty() {
        file_size.saturating_sub(1)
    } else {
        end_s.parse().ok()?
    };
    if start > end || end >= file_size {
        return None;
    }
    Some((start, end))
}

async fn telemetry_handler() -> impl IntoResponse {
    (StatusCode::OK, Json(webtorrent_seeder::telemetry()))
}

async fn register_handler(Json(req): Json<RegisterSeedRequest>) -> impl IntoResponse {
    match webtorrent_seeder::register_seed(req) {
        Ok(rec) => (
            StatusCode::OK,
            Json(json!({
                "status": "ok",
                "seed": rec,
                "seeder": "qualia-daemon",
            })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "error", "message": e })),
        ),
    }
}

async fn unseed_handler(Json(req): Json<UnregisterSeedRequest>) -> impl IntoResponse {
    let removed = webtorrent_seeder::unregister_seed(&req.info_hash);
    (
        StatusCode::OK,
        Json(json!({
            "status": if removed { "ok" } else { "not_found" },
            "info_hash": req.info_hash,
        })),
    )
}

async fn policy_get_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(webtorrent_seeder::get_bandwidth_policy()),
    )
}

async fn policy_set_handler(Json(policy): Json<SeederBandwidthPolicy>) -> impl IntoResponse {
    webtorrent_seeder::set_bandwidth_policy(policy.clone());
    (
        StatusCode::OK,
        Json(json!({ "status": "ok", "policy": policy })),
    )
}

async fn sync_handler(State(state): State<TorrentState>) -> impl IntoResponse {
    let storage = std::env::var("QUALIA_STORAGE_PATH").unwrap_or_else(|_| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| format!("{h}/.qualia"))
            .unwrap_or_else(|_| ".qualia".to_string())
    });
    webtorrent_seeder::sync_from_workbench(&storage, state.daemon_port);
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "active": webtorrent_seeder::list_active_seeds().len(),
        })),
    )
}

async fn webseed_handler(
    AxumPath(info_hash): AxumPath<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let Some(seed) = webtorrent_seeder::lookup_seed(&info_hash) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "seed not found" })),
        ).into_response();
    };

    let path = Path::new(&seed.file_path);
    let Ok(data) = std::fs::read(path) else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "file read failed" })),
        ).into_response();
    };

    let file_size = data.len() as u64;
    let range_hdr_val = headers.get("range").and_then(|v| v.to_str().ok());
    
    let (body_bytes, status, content_range) = if let Some(range_hdr) = range_hdr_val {
        if let Some((start, end)) = parse_range(range_hdr, file_size) {
            let slice = data[start as usize..=end as usize].to_vec();
            let len = slice.len() as u64;
            webtorrent_seeder::record_bytes_served(&info_hash, len);
            let cr = format!("bytes {start}-{end}/{file_size}");
            (slice, StatusCode::PARTIAL_CONTENT, Some(cr))
        } else {
            webtorrent_seeder::record_bytes_served(&info_hash, file_size);
            webtorrent_seeder::record_full_download(&info_hash);
            (data, StatusCode::OK, None)
        }
    } else {
        webtorrent_seeder::record_bytes_served(&info_hash, file_size);
        webtorrent_seeder::record_full_download(&info_hash);
        (data, StatusCode::OK, None)
    };

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/octet-stream"),
    );
    response_headers.insert(
        header::CONTENT_DISPOSITION,
        header::HeaderValue::from_str(&format!("attachment; filename=\"{}\"", seed.display_name))
            .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
    );
    response_headers.insert(
        header::ACCEPT_RANGES,
        header::HeaderValue::from_static("bytes"),
    );
    if let Some(cr) = content_range {
        if let Ok(v) = header::HeaderValue::from_str(&cr) {
            response_headers.insert(header::CONTENT_RANGE, v);
        }
    }

    let r: axum::response::Response = (status, response_headers, body_bytes).into_response(); r
}

pub fn webtorrent_routes(daemon_port: u16) -> Router {
    let state = TorrentState { daemon_port };
    Router::new()
        .route("/telemetry", get(telemetry_handler))
        .route("/seed", post(register_handler))
        .route("/unseed", post(unseed_handler))
        .route("/policy", get(policy_get_handler).post(policy_set_handler))
        .route("/sync", post(sync_handler))
        .route("/webseed/:info_hash", get(webseed_handler))
        .with_state(state)
}
