//! LDP HTTP routes backed by [`crate::pod_store::PodStore`].
//!
//! Default builds still expose a small in-memory simulation when no store is
//! attached (unit tests). Production/personal-pod serve attaches a real store.

use crate::pod_store::PodStore;
use qualia_core_db::modalities::logic::core::WebizenOpcode;
use qualia_core_db::{q_hash, NQuin};
use std::sync::Arc;
use warp::http::{HeaderMap, HeaderValue, StatusCode};
use warp::Filter;

/// Simulated routes used by unit tests / no-store fallback.
pub fn ldp_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let get_public =
        warp::path!("public" / String)
            .and(warp::get())
            .map(|_resource_name: String| {
                let turtle = "<urn:qualia:node:123> <urn:qualia:pred:456> <urn:qualia:node:789> .";
                warp::reply::with_header(turtle, "content-type", "text/turtle")
            });

    let post_private = warp::path!("private" / String)
        .and(warp::post())
        .and(warp::body::bytes())
        .map(|_folder: String, payload: bytes::Bytes| {
            let compressed_quins = ldp_to_quins(&payload);
            warp::reply::json(&serde_json::json!({
                "status": "Stored via Bilateral Micro-Commons",
                "quin_count": compressed_quins.len()
            }))
        });

    get_public.or(post_private)
}

/// Full personal-pod LDP surface: GET/HEAD/PUT/POST/DELETE under the pod root.
pub fn pod_ldp_routes(
    store: Arc<PodStore>,
    public_base: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let s_get = store.clone();
    let base_get = public_base.clone();
    let get = warp::path::full()
        .and(warp::get())
        .and(warp::header::optional::<String>("accept"))
        .map(move |full: warp::path::FullPath, _accept: Option<String>| {
            serve_get(&s_get, &base_get, full.as_str())
        });

    let s_put = store.clone();
    let put = warp::path::full()
        .and(warp::put())
        .and(warp::body::bytes())
        .map(move |full: warp::path::FullPath, body: bytes::Bytes| {
            let path = full.as_str();
            if path == "/" || path.is_empty() {
                return reply_status(StatusCode::METHOD_NOT_ALLOWED, "cannot PUT pod root");
            }
            match s_put.write_bytes(path, &body) {
                Ok(()) => {
                    let mut headers = HeaderMap::new();
                    headers.insert("content-type", HeaderValue::from_static("application/json"));
                    headers.insert(
                        "location",
                        HeaderValue::from_str(path).unwrap_or(HeaderValue::from_static("/")),
                    );
                    json_reply(
                        StatusCode::CREATED,
                        headers,
                        serde_json::json!({"ok": true, "path": path}),
                    )
                }
                Err(e) => reply_status(StatusCode::BAD_REQUEST, &e.to_string()),
            }
        });

    let s_post = store.clone();
    let post = warp::path::full()
        .and(warp::post())
        .and(warp::body::bytes())
        .and(warp::header::optional::<String>("slug"))
        .map(
            move |full: warp::path::FullPath, body: bytes::Bytes, slug: Option<String>| {
                let path = full.as_str().trim_end_matches('/');
                let name = slug.unwrap_or_else(|| {
                    format!(
                        "resource-{}.ttl",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_millis())
                            .unwrap_or(0)
                    )
                });
                let target = if path.is_empty() || path == "/" {
                    format!("/{}", name)
                } else {
                    format!("{}/{}", path, name)
                };
                // Also run allocation firewall on inbound
                let _quins = ldp_to_quins(&body);
                match s_post.write_bytes(&target, &body) {
                    Ok(()) => {
                        let mut headers = HeaderMap::new();
                        headers.insert(
                            "location",
                            HeaderValue::from_str(&target).unwrap_or(HeaderValue::from_static("/")),
                        );
                        headers
                            .insert("content-type", HeaderValue::from_static("application/json"));
                        json_reply(
                            StatusCode::CREATED,
                            headers,
                            serde_json::json!({
                                "ok": true,
                                "path": target,
                                "note": "stored; inbound payload hashed to Quins at firewall"
                            }),
                        )
                    }
                    Err(e) => reply_status(StatusCode::BAD_REQUEST, &e.to_string()),
                }
            },
        );

    let s_del = store.clone();
    let delete = warp::path::full()
        .and(warp::delete())
        .map(move |full: warp::path::FullPath| {
            let path = full.as_str();
            match s_del.delete(path) {
                Ok(()) => reply_status(StatusCode::NO_CONTENT, ""),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    reply_status(StatusCode::NOT_FOUND, "not found")
                }
                Err(e) => reply_status(StatusCode::BAD_REQUEST, &e.to_string()),
            }
        });

    // OPTIONS for CORS preflight is handled by warp::cors on the outer stack.
    get.or(put).or(post).or(delete)
}

fn serve_get(store: &PodStore, public_base: &str, path: &str) -> warp::reply::Response {
    let (status, headers, body) = serve_get_parts(store, public_base, path);
    let mut resp = warp::http::Response::new(body.into());
    *resp.status_mut() = status;
    for (k, v) in headers.iter() {
        resp.headers_mut().insert(k, v.clone());
    }
    resp
}

fn serve_get_parts(
    store: &PodStore,
    public_base: &str,
    path: &str,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let mut headers = HeaderMap::new();
    // Root → redirect attention to profile + containers
    if path == "/" || path.is_empty() {
        let index = format!(
            r#"@prefix ldp: <http://www.w3.org/ns/ldp#> .
@prefix solid: <http://www.w3.org/ns/solid/terms#> .

<> a ldp:BasicContainer ;
   ldp:contains <profile/>, <public/>, <private/>, <inbox/> .
"#
        );
        headers.insert("content-type", HeaderValue::from_static("text/turtle"));
        headers.insert(
            "link",
            HeaderValue::from_static(
                r#"<http://www.w3.org/ns/ldp#BasicContainer>; rel="type", <http://www.w3.org/ns/ldp#Resource>; rel="type""#,
            ),
        );
        let _ = public_base;
        return (StatusCode::OK, headers, index.into_bytes());
    }

    match store.read_bytes(path) {
        Ok(bytes) => {
            let ct = if store.is_container(path) {
                "text/turtle"
            } else {
                PodStore::content_type_for(path)
            };
            headers.insert(
                "content-type",
                HeaderValue::from_str(ct).unwrap_or(HeaderValue::from_static("text/turtle")),
            );
            // LDP types from bundled ldp.ttl (W3C ns archive)
            let link = if store.is_container(path) {
                format!(
                    "<{}>; rel=\"type\", <{}>; rel=\"type\"",
                    crate::vocab::LDP_BASIC_CONTAINER,
                    crate::vocab::LDP_RESOURCE
                )
            } else {
                format!("<{}>; rel=\"type\"", crate::vocab::LDP_RESOURCE)
            };
            if let Ok(v) = HeaderValue::from_str(&link) {
                headers.insert("link", v);
            }
            // WAC discovery stub
            let acl = format!("<{}/.acl>; rel=\"acl\"", path.trim_end_matches('/'));
            if let Ok(v) = HeaderValue::from_str(&acl) {
                headers.append("link", v);
            }
            (StatusCode::OK, headers, bytes)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            headers.insert("content-type", HeaderValue::from_static("text/plain"));
            (StatusCode::NOT_FOUND, headers, b"not found".to_vec())
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            headers.insert("content-type", HeaderValue::from_static("text/plain"));
            (StatusCode::FORBIDDEN, headers, b"forbidden".to_vec())
        }
        Err(e) => {
            headers.insert("content-type", HeaderValue::from_static("text/plain"));
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                e.to_string().into_bytes(),
            )
        }
    }
}

fn reply_status(status: StatusCode, body: &str) -> warp::reply::Response {
    let mut resp = warp::http::Response::new(body.to_string().into());
    *resp.status_mut() = status;
    resp.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    resp
}

fn json_reply(
    status: StatusCode,
    headers: HeaderMap,
    value: serde_json::Value,
) -> warp::reply::Response {
    let body = serde_json::to_vec(&value).unwrap_or_default();
    let mut resp = warp::http::Response::new(body.into());
    *resp.status_mut() = status;
    for (k, v) in headers.iter() {
        resp.headers_mut().insert(k, v.clone());
    }
    if !resp.headers().contains_key("content-type") {
        resp.headers_mut()
            .insert("content-type", HeaderValue::from_static("application/json"));
    }
    resp
}

/// Translates a Solid payload into Super-Quins (allocation firewall).
pub fn ldp_to_quins(payload: &[u8]) -> Vec<NQuin> {
    // Prefer Turtle triple parse when payload looks like text RDF.
    if let Ok(text) = std::str::from_utf8(payload) {
        if text.contains('<') && text.contains('>') {
            return crate::consumer::turtle_to_quins(text);
        }
    }
    let mut quins = Vec::new();
    let chunks = payload.chunks(128);
    for chunk in chunks {
        let subject = fast_hash_bytes(chunk);
        let mut quin = NQuin {
            subject,
            predicate: q_hash("solid:contains"),
            object: subject.wrapping_add(1),
            context: q_hash("local:inbox"),
            metadata: 0x4000_0000_0000_0000,
            parity: 0,
        };
        quin.recalculate_parity();
        quins.push(quin);
    }
    quins
}

fn fast_hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut i = 0;
    while i < bytes.len() {
        hash = hash ^ (bytes[i] as u64);
        hash = hash.wrapping_mul(0x100000001b3);
        i += 1;
    }
    hash & 0x0FFF_FFFF_FFFF_FFFF
}

/// Compiles a standard WAC .acl file down to Webizen Bytecode (N3Logic)
pub fn compile_wac_to_bytecode(acl_body: &str) -> Vec<WebizenOpcode> {
    let mut bytecode = Vec::new();
    if acl_body.contains("acl:Read") {
        bytecode.push(WebizenOpcode::EvalMetadataMask(0x01));
    }
    if acl_body.contains("acl:Write") {
        bytecode.push(WebizenOpcode::MatchSubject(0));
    }
    bytecode.push(WebizenOpcode::HaltIfFalse);
    bytecode
}

/// Translates a native Super-Quin vector back to JSON-LD (cold path).
pub fn quins_to_ldp(quins: &[NQuin]) -> serde_json::Value {
    let mut triples = Vec::new();
    for q in quins {
        triples.push(serde_json::json!({
            "@id": format!("urn:hash:{}", q.subject),
            "http://schema.org/predicate": q.predicate.to_string(),
            "http://schema.org/object": q.object.to_string(),
            "cml:context": q.context.to_string()
        }));
    }
    serde_json::json!(triples)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_firewall() {
        #[cfg(feature = "dhat-heap")]
        let _profiler = dhat::Profiler::new_heap();

        let payload = vec![0x41; 5 * 1024 * 1024];

        #[cfg(feature = "dhat-heap")]
        let stats_before = dhat::HeapStats::get();

        let quins = ldp_to_quins(&payload);

        #[cfg(feature = "dhat-heap")]
        let stats_after = dhat::HeapStats::get();

        assert_eq!(quins.len(), 40960);
        assert_eq!(quins[0].metadata, 0x4000_0000_0000_0000);

        #[cfg(feature = "dhat-heap")]
        {
            let current_diff = stats_after.curr_bytes - stats_before.curr_bytes;
            assert!(
                current_diff < 3 * 1024 * 1024,
                "Heap allocation firewall failed: {:?} bytes allocated",
                current_diff
            );
        }
    }

    #[test]
    fn test_acl_compilation() {
        let acl = "
        <#rule> a acl:Authorization;
            acl:mode acl:Read, acl:Write .
        ";
        let bytecode = compile_wac_to_bytecode(acl);
        assert!(bytecode.contains(&WebizenOpcode::MatchSubject(0)));
        assert!(bytecode.contains(&WebizenOpcode::HaltIfFalse));
    }

    #[test]
    fn turtle_inbound_uses_consumer_parser() {
        let body = b"<urn:a> <urn:b> <urn:c> .\n";
        let quins = ldp_to_quins(body);
        assert_eq!(quins.len(), 1);
        assert!(quins[0].verify_ecc_parity());
    }
}
