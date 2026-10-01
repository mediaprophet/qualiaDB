//! UE-014 — SPARQL / Node / WASM adapters for `application/ld+json`.
//!
//! Two distinct JSON-LD-labelled surfaces exist; Node and browser hosts must
//! pick the right one:
//!
//! | Surface | Content-Type | Shape | Use |
//! |---|---|---|---|
//! | Daemon `POST /query` (Accept: `application/ld+json`) | `application/ld+json` | `@graph` of **quin field decimals** (`subject`/`predicate`/… as strings of `u64`) | Fast wire for LocalHost / benchmarks — **not** RDF-shaped JSON-LD 1.1 |
//! | `serialize_to_jsonld` / `serialize_rdf_wasm(format:"jsonld")` | `application/ld+json` | Expanded node objects with `@id` / `@value`/`@type` | RDF interchange; round-trips with `parse_jsonld_wasm` |
//! | Vendor CBOR `serialize_to_cborld` | `application/vnd.qualia.nquin-cbor` | JSON-LD-shaped maps, **lossless** inline literals | Compact quin transport |
//!
//! CONSTRUCT results are `NQuin` slices on both paths. Prefer the serializer
//! for any egress that must preserve typed literals bit-exactly.

use crate::NQuin;
use crate::sparql_library::serialisers::rdf_serializers::serialize_to_jsonld;

/// Build the RDF-shaped JSON-LD document a Node/WASM adapter should emit for
/// CONSTRUCT / graph results (not the daemon quin-decimal `@graph`).
pub fn construct_results_as_ld_json(quins: &[NQuin]) -> Result<String, String> {
    let mut buf = Vec::with_capacity(quins.len().saturating_mul(96).max(16));
    serialize_to_jsonld(&mut buf, quins)?;
    String::from_utf8(buf).map_err(|e| format!("utf8: {e}"))
}

/// Daemon-style quin-decimal `@graph` body (documentary parity with
/// `webizen_server` JsonLd arm). Marked so adapters do not confuse it with
/// RDF JSON-LD 1.1.
pub fn daemon_quin_graph_ld_json(quins: &[NQuin]) -> String {
    use serde_json::json;
    let graph: Vec<serde_json::Value> = quins
        .iter()
        .map(|q| {
            json!({
                "subject": q.subject.to_string(),
                "predicate": q.predicate.to_string(),
                "object": q.object.to_string(),
                "context": q.context.to_string(),
                "metadata": q.metadata.to_string(),
                "parity": q.parity.to_string(),
            })
        })
        .collect();
    json!({
        "@context": { "@vocab": "https://webizen.org/vocab#" },
        "@graph": graph,
        "match_count": quins.len(),
        "profile": "qualia:daemon-quin-graph-v1",
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::resolver::{INLINE_TAG_INTEGER, MSB_FLAG};

    fn quin(s: u64, p: u64, o: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn construct_adapter_emits_rdf_shaped_value_objects() {
        let quins = [quin(
            MSB_FLAG | 1,
            MSB_FLAG | 2,
            INLINE_TAG_INTEGER | 42,
        )];
        let out = construct_results_as_ld_json(&quins).unwrap();
        assert!(out.contains("\"@value\""), "{out}");
        assert!(out.contains("\"@type\""), "{out}");
        assert!(!out.contains("match_count"), "{out}");
    }

    #[test]
    fn daemon_graph_profile_is_labelled_not_rdf_shaped() {
        let quins = [quin(MSB_FLAG | 1, MSB_FLAG | 2, INLINE_TAG_INTEGER | 7)];
        let out = daemon_quin_graph_ld_json(&quins);
        assert!(out.contains("qualia:daemon-quin-graph-v1"), "{out}");
        assert!(out.contains("match_count"), "{out}");
        assert!(!out.contains("\"@value\""), "{out}");
    }
}
