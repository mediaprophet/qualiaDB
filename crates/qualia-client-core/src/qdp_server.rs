//! **QDP profile renderer** — HTTP content-negotiation for the `/.well-known/QDP` profile.
//!
//! This module is **pure**: it turns a [`crate::front_door::FrontDoorRecord`] plus an HTTP `Accept`
//! header value into a [`QdpResponse`] (content-type + body bytes). It does **not** bind a socket,
//! spawn a listener, or perform any I/O — the transport layer (daemon, Worker, Solid POD) owns the
//! socket and calls [`render_profile`] to produce the payload for the matched route.
//!
//! Content negotiation follows QDP §3.1 (the rich profile is served in Turtle + JSON-LD + CBOR-LD):
//! - `application/ld+json` → JSON-LD (`to_json_ld`)
//! - `application/cbor`     → CBOR-LD (`to_cbor_ld`)
//! - anything else, incl. `text/turtle` and `*/*` → Turtle (`to_turtle`, the default)

use crate::front_door::FrontDoorRecord;

/// The well-known path the rich QDP profile is served at (QDP §3.1).
pub const WELL_KNOWN_QDP_PATH: &str = "/.well-known/QDP";

/// A rendered QDP profile response: the negotiated content type and the serialized body bytes.
#[derive(Debug, Clone)]
pub struct QdpResponse {
    /// The `Content-Type` header value for the negotiated representation.
    pub content_type: String,
    /// The serialized profile body.
    pub body: Vec<u8>,
}

/// Render a [`FrontDoorRecord`] as a QDP profile, negotiating the representation from an HTTP
/// `Accept` header value.
///
/// Matching is a simple substring test against `accept` (sufficient for the QDP media types):
/// - contains `"application/ld+json"` → JSON-LD, content-type `application/ld+json`
/// - else contains `"application/cbor"` → CBOR-LD, content-type `application/cbor`
/// - else (default, incl. `text/turtle` and `*/*`) → Turtle, content-type `text/turtle`
///
/// Returns `Err` only if serialization fails (JSON-LD or CBOR-LD encoding).
pub fn render_profile(rec: &FrontDoorRecord, accept: &str) -> Result<QdpResponse, String> {
    if accept.contains("application/ld+json") {
        let body = serde_json::to_vec(&rec.to_json_ld()).map_err(|e| e.to_string())?;
        Ok(QdpResponse { content_type: "application/ld+json".to_string(), body })
    } else if accept.contains("application/cbor") {
        let body = rec.to_cbor_ld()?;
        Ok(QdpResponse { content_type: "application/cbor".to_string(), body })
    } else {
        let body = rec.to_turtle().into_bytes();
        Ok(QdpResponse { content_type: "text/turtle".to_string(), body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::AgentType;
    use crate::front_door::FrontDoorRecord;

    /// A minimal record for the pure content-negotiation tests.
    fn minimal_record() -> FrontDoorRecord {
        FrontDoorRecord {
            domain: "a.example".to_string(),
            agent_type: AgentType::NaturalPerson,
            front_door_did: "did:qdp:a".to_string(),
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
    fn well_known_path_is_stable() {
        assert_eq!(WELL_KNOWN_QDP_PATH, "/.well-known/QDP");
    }

    #[test]
    fn turtle_is_the_default() {
        let rec = minimal_record();
        let resp = render_profile(&rec, "text/turtle").expect("turtle renders");
        assert_eq!(resp.content_type, "text/turtle");
        assert!(!resp.body.is_empty());

        // `*/*` and unknown accept values also fall through to Turtle.
        let any = render_profile(&rec, "*/*").expect("wildcard renders");
        assert_eq!(any.content_type, "text/turtle");
        assert!(!any.body.is_empty());
    }

    #[test]
    fn json_ld_negotiation_roundtrips() {
        let rec = minimal_record();
        let resp = render_profile(&rec, "application/ld+json").expect("json-ld renders");
        assert_eq!(resp.content_type, "application/ld+json");

        let parsed: serde_json::Value =
            serde_json::from_slice(&resp.body).expect("body is valid JSON");
        assert!(parsed.get("@type").is_some(), "JSON-LD node carries an @type");
    }

    #[test]
    fn cbor_negotiation_produces_bytes() {
        let rec = minimal_record();
        let resp = render_profile(&rec, "application/cbor").expect("cbor renders");
        assert_eq!(resp.content_type, "application/cbor");
        assert!(!resp.body.is_empty());
    }
}
