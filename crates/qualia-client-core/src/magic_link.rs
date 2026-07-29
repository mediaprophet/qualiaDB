//! **Magic link** — encode/decode a [`ConnectionIdentifier`] as a shareable link for email/text/web
//! onboarding. A single connection payload can travel three interchangeable ways, all decoded by
//! [`from_link`]:
//!
//! 1. A custom-scheme **deep link** (`web+qualia://connect?p=<payload>`) that opens the app directly.
//! 2. A **domain-hosted HTTPS fallback** (`https://<domain>/w#<payload>`) — a normal link that works in
//!    any browser and, on a device with the app registered, hands off to it. The payload rides in the
//!    URL `#` fragment so it is never sent to the hosting server (fragments are client-only).
//! 3. A **bare** `qcx1_…` string (paste-anywhere).
//!
//! The payload in every form is the self-certifying `qcx1_<base64url>` from
//! [`ConnectionIdentifier::encode`]; recipients verify it locally, so the link carries no trust in the
//! transport. base64url is already URL-safe, but we percent-decode defensively on the way in.

use crate::connection_identifier::ConnectionIdentifier;

/// Custom URL scheme registered by the app for deep links.
pub const SCHEME: &str = "web+qualia";

/// Build a custom-scheme **deep link** that opens the app directly:
/// `web+qualia://connect?p=<id.encode()?>`.
///
/// The payload is base64url (URL-safe) so it is placed in the query verbatim.
pub fn to_deep_link(id: &ConnectionIdentifier) -> Result<String, String> {
    let payload = id.encode()?;
    Ok(format!("{SCHEME}://connect?p={payload}"))
}

/// Build a **domain-hosted HTTPS fallback** link: `https://<domain>/w#<id.encode()?>`.
///
/// This is an ordinary `https://` URL that works in any browser; on a device where the app has
/// claimed the domain it opens the app instead. The payload rides in the `#` fragment, which browsers
/// never transmit to the server — so the hosting domain sees only that `/w` was requested, not the
/// connection payload.
pub fn to_https_link(id: &ConnectionIdentifier, domain: &str) -> Result<String, String> {
    let payload = id.encode()?;
    Ok(format!("https://{domain}/w#{payload}"))
}

/// Decode a [`ConnectionIdentifier`] from ANY supported link form:
///
/// - `web+qualia://connect?p=<payload>` — the deep link (payload is the `p=` query value).
/// - `https://<domain>/w#<payload>` — the HTTPS fallback (payload is the `#` fragment).
/// - a bare `qcx1_…` string.
///
/// The extracted payload is percent-decoded defensively (base64url is URL-safe, but a `%`-escaped
/// payload is still accepted) before [`ConnectionIdentifier::decode`].
pub fn from_link(link: &str) -> Result<ConnectionIdentifier, String> {
    let link = link.trim();

    let payload = if let Some(rest) = link.strip_prefix(&format!("{SCHEME}://")) {
        // web+qualia://connect?p=<payload>[&...]  — extract the `p` query value.
        let query = rest.split_once('?').map(|(_, q)| q).unwrap_or("");
        query
            .split('&')
            .find_map(|kv| kv.strip_prefix("p="))
            .ok_or("deep link missing `p=` payload")?
            .to_string()
    } else if let Some((_, frag)) = link.split_once('#') {
        // https://<domain>/w#<payload>  — payload is the fragment.
        if frag.is_empty() {
            return Err("https link has an empty `#` fragment".into());
        }
        frag.to_string()
    } else if link.starts_with("qcx1_") {
        // Bare identifier string.
        link.to_string()
    } else {
        return Err("unrecognised link: expected a web+qualia:// deep link, an https://…/w# link, or a bare qcx1_ string".into());
    };

    let payload = pct_decode(&payload);
    ConnectionIdentifier::decode(&payload)
}

/// Build a `mailto:` URI that pre-fills an onboarding email: a short human line plus the deep link in
/// the body. Both subject and body are percent-encoded.
///
/// `mailto:?subject=<pct>&body=<pct>`
pub fn to_mailto(id: &ConnectionIdentifier, subject: &str) -> Result<String, String> {
    let deep = to_deep_link(id)?;
    let body =
        format!("You've been invited to connect. Open this link on your device:\n\n{deep}\n");
    Ok(format!(
        "mailto:?subject={}&body={}",
        pct_encode(subject),
        pct_encode(&body)
    ))
}

/// Percent-encode a string for use in a URL query/`mailto` component. Uses the `urlencoding` crate
/// (a declared dependency) so the escaping is RFC 3986 compliant.
fn pct_encode(s: &str) -> String {
    urlencoding::encode(s).into_owned()
}

/// Percent-decode a string. Invalid `%`-escapes are left as-is (lossy, defensive): base64url payloads
/// contain no `%`, so a payload with no escapes round-trips unchanged.
fn pct_decode(s: &str) -> String {
    match urlencoding::decode(s) {
        Ok(decoded) => decoded.into_owned(),
        Err(_) => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ConnectionIdentifier {
        ConnectionIdentifier {
            version: 1,
            front_door_did: "did:x".into(),
            identity_pubkey_hex: String::new(),
            wireguard_pubkey_hex: "aa".repeat(32),
            overlay_addr: "fd00::1".into(),
            rendezvous: vec![],
            relation_type: String::new(),
            display_name: "Alice".into(),
            created_at: 1,
            expires_at: 0,
            nonce: "n".into(),
            signature_hex: String::new(),
        }
    }

    #[test]
    fn deep_link_round_trips() {
        let id = sample();
        let link = to_deep_link(&id).expect("deep link");
        assert!(link.starts_with("web+qualia://connect?p="));
        let back = from_link(&link).expect("decode deep link");
        assert_eq!(back, id, "deep link round-trips to an equal identifier");
    }

    #[test]
    fn https_link_round_trips() {
        let id = sample();
        let link = to_https_link(&id, "alice.example").expect("https link");
        assert!(link.starts_with("https://alice.example/w#"));
        let back = from_link(&link).expect("decode https link");
        assert_eq!(back, id, "https link round-trips to an equal identifier");
    }

    #[test]
    fn bare_identifier_is_accepted() {
        let id = sample();
        let bare = id.encode().expect("encode");
        assert!(bare.starts_with("qcx1_"));
        let back = from_link(&bare).expect("decode bare identifier");
        assert_eq!(back, id, "a bare qcx1_ string is accepted");
    }

    #[test]
    fn mailto_contains_the_deep_link() {
        let id = sample();
        let deep = to_deep_link(&id).expect("deep link");
        let mailto = to_mailto(&id, "Connect with Alice").expect("mailto");
        assert!(mailto.starts_with("mailto:?subject="));
        assert!(mailto.contains("&body="));
        // The deep link survives percent-encoding into the body.
        let encoded_deep = pct_encode(&deep);
        assert!(
            mailto.contains(&encoded_deep),
            "mailto body contains the (percent-encoded) deep link"
        );
    }

    #[test]
    fn percent_escaped_payload_is_decoded_defensively() {
        // base64url is URL-safe, but a payload that arrives percent-escaped must still decode.
        let id = sample();
        let payload = id.encode().expect("encode");
        // Escape every 'a' as %61 to force the percent-decode path.
        let escaped = payload.replace('a', "%61");
        let link = format!("{SCHEME}://connect?p={escaped}");
        let back = from_link(&link).expect("decode percent-escaped payload");
        assert_eq!(back, id, "percent-escaped payload decodes");
    }

    #[test]
    fn unrecognised_link_is_rejected() {
        assert!(from_link("ftp://nope").is_err());
        assert!(from_link("just some text").is_err());
    }
}
