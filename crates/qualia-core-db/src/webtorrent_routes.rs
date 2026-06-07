//! HTTP routes for the Qualia-native WebTorrent seeder.

#![cfg(not(target_arch = "wasm32"))]

use std::path::Path;

use serde::Serialize;
use serde_json::json;
use warp::http::{header, StatusCode};
use warp::hyper::Body;
use warp::{Filter, Reply};

use crate::webtorrent_seeder::{
    self, RegisterSeedRequest, SeederBandwidthPolicy, UnregisterSeedRequest,
};

type HttpResponse = warp::http::Response<Body>;

fn json_response<T: Serialize>(status: StatusCode, body: &T) -> HttpResponse {
    warp::reply::with_status(warp::reply::json(body), status).into_response()
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

pub fn webtorrent_routes(
    daemon_port: u16,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let telemetry = warp::path!("torrent" / "telemetry")
        .and(warp::get())
        .map(|| json_response(StatusCode::OK, &webtorrent_seeder::telemetry()));

    let register = warp::path!("torrent" / "seed")
        .and(warp::post())
        .and(warp::body::json())
        .map(|req: RegisterSeedRequest| {
            match webtorrent_seeder::register_seed(req) {
                Ok(rec) => json_response(
                    StatusCode::OK,
                    &json!({
                        "status": "ok",
                        "seed": rec,
                        "seeder": "qualia-daemon",
                    }),
                ),
                Err(e) => json_response(
                    StatusCode::BAD_REQUEST,
                    &json!({ "status": "error", "message": e }),
                ),
            }
        });

    let unseed = warp::path!("torrent" / "unseed")
        .and(warp::post())
        .and(warp::body::json())
        .map(|req: UnregisterSeedRequest| {
            let removed = webtorrent_seeder::unregister_seed(&req.info_hash);
            json_response(
                StatusCode::OK,
                &json!({
                    "status": if removed { "ok" } else { "not_found" },
                    "info_hash": req.info_hash,
                }),
            )
        });

    let policy_get = warp::path!("torrent" / "policy")
        .and(warp::get())
        .map(|| json_response(StatusCode::OK, &webtorrent_seeder::get_bandwidth_policy()));

    let policy_set = warp::path!("torrent" / "policy")
        .and(warp::post())
        .and(warp::body::json())
        .map(|policy: SeederBandwidthPolicy| {
            webtorrent_seeder::set_bandwidth_policy(policy.clone());
            json_response(StatusCode::OK, &json!({ "status": "ok", "policy": policy }))
        });

    let sync = warp::path!("torrent" / "sync")
        .and(warp::post())
        .map(move || {
            let storage = std::env::var("QUALIA_STORAGE_PATH").unwrap_or_else(|_| {
                std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map(|h| format!("{h}/.qualia"))
                    .unwrap_or_else(|_| ".qualia".to_string())
            });
            webtorrent_seeder::sync_from_workbench(&storage, daemon_port);
            json_response(
                StatusCode::OK,
                &json!({
                    "status": "ok",
                    "active": webtorrent_seeder::list_active_seeds().len(),
                }),
            )
        });

    let webseed = warp::path!("torrent" / "webseed" / String)
        .and(warp::get())
        .and(warp::header::optional::<String>("range"))
        .map(|info_hash: String, range: Option<String>| {
            let Some(seed) = webtorrent_seeder::lookup_seed(&info_hash) else {
                return json_response(
                    StatusCode::NOT_FOUND,
                    &json!({ "error": "seed not found" }),
                );
            };

            let path = Path::new(&seed.file_path);
            let Ok(data) = std::fs::read(path) else {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &json!({ "error": "file read failed" }),
                );
            };

            let file_size = data.len() as u64;
            let (body, status, content_range) = if let Some(ref range_hdr) = range {
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

            let mut reply = HttpResponse::new(body.into());
            *reply.status_mut() = status;
            let headers = reply.headers_mut();
            headers.insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static("application/octet-stream"),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                header::HeaderValue::from_str(&format!(
                    "attachment; filename=\"{}\"",
                    seed.display_name
                ))
                .unwrap_or_else(|_| header::HeaderValue::from_static("attachment")),
            );
            headers.insert(
                header::ACCEPT_RANGES,
                header::HeaderValue::from_static("bytes"),
            );
            if let Some(cr) = content_range {
                if let Ok(v) = header::HeaderValue::from_str(&cr) {
                    headers.insert(header::CONTENT_RANGE, v);
                }
            }
            reply
        });

    telemetry
        .or(register)
        .or(unseed)
        .or(policy_get)
        .or(policy_set)
        .or(sync)
        .or(webseed)
}
