//! **QDP front-door resolver over DNS-over-HTTPS (DoH)** — QDP §3.6.
//!
//! The front-door anchor for a domain is a DNS `TXT` record at `_qdp.<domain>` (see
//! [`crate::front_door`]). Rather than depend on the host's stub resolver (which often cannot
//! return `TXT`, and leaks the lookup to the local network), we resolve it over **DoH** against a
//! public resolver that speaks the JSON DoH format (Cloudflare / Google, RFC 8484 + the JSON
//! profile). The wire format is a small JSON document with an `"Answer"` array; each answer's
//! `"data"` field is the (possibly quote-wrapped, possibly split) TXT string.
//!
//! Two pieces:
//! - [`parse_doh_txt_answers`] — **pure**: turns a DoH JSON body into one concatenated TXT string.
//!   A TXT record longer than 255 bytes is transmitted as multiple `"..." "..."` character-strings;
//!   this joins them (and strips the surrounding / boundary `"` quotes) into a single string.
//! - [`resolve_front_door`] — **host-only** (`not(wasm32)`): performs the blocking HTTPS GET and
//!   feeds the joined text to [`crate::front_door::FrontDoorRecord::from_dns_txt`].
//!
//! No private keys are ever fetched (QDP §5) — the front-door TXT carries only the Front Door DID
//! and public peering material.

use crate::front_door::{dns_record_name, FrontDoorRecord};

/// PURE. From a Cloudflare/Google DoH JSON response, read the `"Answer"` array and concatenate the
/// `"data"` field of every answer into one string.
///
/// DNS `TXT` values are transmitted as one or more length-prefixed character-strings (≤255 bytes
/// each); DoH resolvers surface a multi-part TXT as `"part-a" "part-b"` inside a single `"data"`
/// field, and split records may also appear as multiple `Answer` elements. In both cases the
/// logical value is the pieces concatenated with no separator, so we:
/// 1. take each answer's `"data"` string,
/// 2. strip the surrounding double-quotes and any internal `" "` string-boundary quotes, and
/// 3. concatenate everything into one string (which is then a valid `_qdp.<domain>` TXT value).
///
/// Missing / malformed answers are skipped; a body with no usable answers yields `""`.
pub fn parse_doh_txt_answers(json: &serde_json::Value) -> String {
    let Some(answers) = json.get("Answer").and_then(|a| a.as_array()) else {
        return String::new();
    };
    let mut joined = String::new();
    for ans in answers {
        let Some(data) = ans.get("data").and_then(|d| d.as_str()) else {
            continue;
        };
        // Strip the surrounding double-quotes the resolver wraps the record in, then remove any
        // internal `"` string-boundary quotes that separate the multi-part `"a" "b"` pieces.
        let trimmed = data.trim().trim_matches('"');
        for piece in trimmed.split('"') {
            joined.push_str(piece);
        }
    }
    joined
}

/// Resolve a domain's QDP front-door record via DoH (Cloudflare), host targets only.
///
/// Builds the query name with [`dns_record_name`] (`_qdp.<domain>`), performs a blocking HTTPS
/// `GET https://cloudflare-dns.com/dns-query?name=<name>&type=TXT` with the
/// `accept: application/dns-json` header (8 s timeout), parses the JSON body, joins the TXT answers
/// via [`parse_doh_txt_answers`], and decodes them with
/// [`FrontDoorRecord::from_dns_txt`]. All errors are mapped to `String`.
#[cfg(not(target_arch = "wasm32"))]
pub fn resolve_front_door(domain: &str) -> Result<FrontDoorRecord, String> {
    let name = dns_record_name(domain);
    let url = format!("https://cloudflare-dns.com/dns-query?name={name}&type=TXT");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| format!("failed to build DoH client: {e}"))?;

    let resp = client
        .get(&url)
        .header("accept", "application/dns-json")
        .send()
        .map_err(|e| format!("DoH request to {url} failed: {e}"))?;

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| format!("DoH response for {domain} was not valid JSON: {e}"))?;

    let joined = parse_doh_txt_answers(&json);
    if joined.is_empty() {
        return Err(format!("no _qdp TXT answer for {domain}"));
    }
    FrontDoorRecord::from_dns_txt(domain, &joined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_doh_txt_answers_joins_and_strips_quotes() {
        // A Cloudflare/Google DoH body: the TXT record is wrapped in `"..."` and its internal
        // string-boundary quotes are escaped in the JSON `data` field.
        let body = json!({
            "Answer": [
                { "data": "\"qdp:signer <did:qdp:x> ; qdp:agentType \\\"person\\\"\"" }
            ]
        });
        let joined = parse_doh_txt_answers(&body);
        assert!(
            joined.contains("qdp:signer <did:qdp:x>"),
            "expected the signer clause in the joined TXT, got: {joined:?}"
        );

        // And the joined text must decode into a real front-door record.
        let rec = FrontDoorRecord::from_dns_txt("x.example", &joined)
            .expect("joined TXT should parse into a FrontDoorRecord");
        assert_eq!(rec.front_door_did, "did:qdp:x");
        assert_eq!(rec.domain, "x.example");
    }

    #[test]
    fn parse_doh_txt_answers_concatenates_split_records() {
        // A single TXT value delivered as two `Answer` elements (a long record split across
        // multiple character-strings) — quote-stripped, the pieces concatenate with no separator.
        let body = json!({
            "Answer": [
                { "data": "\"qdp:signer <did:qdp:sp\"" },
                { "data": "\"lit> ; qdp:agentType \\\"org\\\"\"" }
            ]
        });
        let joined = parse_doh_txt_answers(&body);
        assert!(
            joined.contains("qdp:signer <did:qdp:split>"),
            "split TXT pieces should concatenate, got: {joined:?}"
        );
        let rec = FrontDoorRecord::from_dns_txt("split.example", &joined).unwrap();
        assert_eq!(rec.front_door_did, "did:qdp:split");
    }

    #[test]
    fn parse_doh_txt_answers_empty_on_missing_answer() {
        assert_eq!(parse_doh_txt_answers(&json!({ "Status": 0 })), "");
        assert_eq!(parse_doh_txt_answers(&json!({ "Answer": [] })), "");
    }
}
