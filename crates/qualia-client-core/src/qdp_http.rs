//! **QDP-over-HTTP** — serve the rich `/.well-known/QDP` profile over a local HTTP listener.
//!
//! This is the transport that fronts the pure [`crate::qdp_server`] renderer: once a domain-agent is
//! reachable on the WireGuard mesh (or any bound address), it can *self-host* its own front-door
//! profile instead of relying on external hosting (Cloudflare Worker/R2, Solid POD). No hosting
//! provider, no lock-in — the agent answers `GET /.well-known/QDP` from its own process.
//!
//! The routing decision ([`route`]) is **pure** and testable without binding a socket: it maps
//! `(method, path, accept)` to an [`HttpReply`]. Only [`serve_blocking`] touches the network, and it
//! is a thin `tiny_http` accept loop that delegates every request to [`route`].
//!
//! Content negotiation itself is delegated to [`crate::qdp_server::render_profile`] (Turtle / JSON-LD /
//! CBOR-LD per the `Accept` header), so this module never duplicates the serialization logic.

use crate::front_door::FrontDoorRecord;

/// A fully-resolved HTTP reply: status code, `Content-Type`, and the body bytes.
///
/// This is the pure result of [`route`]; [`serve_blocking`] turns it into a `tiny_http::Response`.
#[derive(Debug, Clone)]
pub struct HttpReply {
    /// HTTP status code (200 on a rendered profile, 404 for an unmatched route, 500 on render error).
    pub status: u16,
    /// The `Content-Type` header value for [`Self::body`].
    pub content_type: String,
    /// The response body bytes.
    pub body: Vec<u8>,
}

/// Route one HTTP request against a [`FrontDoorRecord`] — **pure**, no I/O.
///
/// - `GET` on [`crate::qdp_server::WELL_KNOWN_QDP_PATH`] (query string stripped) →
///   [`render_profile`](crate::qdp_server::render_profile) with the `Accept` value.
///   - `Ok` → `200` with the negotiated content-type and body.
///   - `Err` → `500 text/plain` carrying the error message.
/// - anything else → `404 text/plain` `not found`.
///
/// The method match is ASCII-case-insensitive; the path is compared after stripping any `?query`.
pub fn route(record: &FrontDoorRecord, method: &str, path: &str, accept: &str) -> HttpReply {
    // Strip any query string before comparing the path.
    let path = path.split('?').next().unwrap_or(path);

    if method.eq_ignore_ascii_case("GET") && path == crate::qdp_server::WELL_KNOWN_QDP_PATH {
        match crate::qdp_server::render_profile(record, accept) {
            Ok(resp) => HttpReply {
                status: 200,
                content_type: resp.content_type,
                body: resp.body,
            },
            Err(err) => HttpReply {
                status: 500,
                content_type: "text/plain".to_string(),
                body: err.into_bytes(),
            },
        }
    } else {
        HttpReply {
            status: 404,
            content_type: "text/plain".to_string(),
            body: b"not found".to_vec(),
        }
    }
}

/// Serve the QDP profile for `record` on `bind_addr` (e.g. `"[fd00::1]:80"` on the mesh, or
/// `"127.0.0.1:8080"` locally). **Blocks** the calling thread on the accept loop — the caller is
/// expected to run this on a dedicated thread.
///
/// Native-only (`tiny_http`). Every request is dispatched through the pure [`route`], so the network
/// path and the tested path share one implementation.
#[cfg(not(target_arch = "wasm32"))]
pub fn serve_blocking(record: FrontDoorRecord, bind_addr: &str) -> Result<(), String> {
    let server = tiny_http::Server::http(bind_addr).map_err(|e| e.to_string())?;

    for req in server.incoming_requests() {
        let method = req.method().as_str().to_string();
        let url = req.url().to_string();
        // Extract the `Accept` header value (case-insensitive field match); default to `*/*`.
        let accept = req
            .headers()
            .iter()
            .find(|h| h.field.equiv("Accept"))
            .map(|h| h.value.as_str().to_string())
            .unwrap_or_else(|| "*/*".to_string());

        let reply = route(&record, &method, &url, &accept);

        let mut resp =
            tiny_http::Response::from_data(reply.body).with_status_code(reply.status);
        if let Ok(header) =
            tiny_http::Header::from_bytes(b"Content-Type", reply.content_type.as_bytes())
        {
            resp = resp.with_header(header);
        }
        let _ = req.respond(resp);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front_door::FrontDoorRecord;

    /// A minimal record for the pure routing tests (no socket is bound).
    fn minimal_record() -> FrontDoorRecord {
        FrontDoorRecord {
            domain: "a.example".into(),
            agent_type: crate::domains::AgentType::NaturalPerson,
            front_door_did: "did:qdp:a".into(),
            name: None,
            webid: None,
            services: vec![],
            identity_pubkey_hex: None,
            wireguard_pubkey_hex: None,
            overlay_addr: None,
            profile_url: None,
        }
    }

    #[test]
    fn get_well_known_qdp_turtle_is_200() {
        let rec = minimal_record();
        let reply = route(&rec, "GET", "/.well-known/QDP", "text/turtle");
        assert_eq!(reply.status, 200);
        assert!(
            reply.content_type.contains("turtle"),
            "content_type was {:?}",
            reply.content_type
        );
        assert!(!reply.body.is_empty());
    }

    #[test]
    fn query_string_is_stripped_before_matching() {
        let rec = minimal_record();
        let reply = route(&rec, "GET", "/.well-known/QDP?x=1", "application/ld+json");
        assert_eq!(reply.status, 200);
        assert_eq!(reply.content_type, "application/ld+json");
        assert!(!reply.body.is_empty());
    }

    #[test]
    fn method_match_is_case_insensitive() {
        let rec = minimal_record();
        let reply = route(&rec, "get", "/.well-known/QDP", "*/*");
        assert_eq!(reply.status, 200);
    }

    #[test]
    fn unknown_path_is_404() {
        let rec = minimal_record();
        let reply = route(&rec, "GET", "/other", "*/*");
        assert_eq!(reply.status, 404);
        assert_eq!(reply.content_type, "text/plain");
        assert_eq!(reply.body, b"not found");
    }

    #[test]
    fn non_get_method_is_404() {
        let rec = minimal_record();
        let reply = route(&rec, "POST", "/.well-known/QDP", "*/*");
        assert_eq!(reply.status, 404);
    }
}
