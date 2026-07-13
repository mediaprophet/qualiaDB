use crate::domains::geospatial::adapters::{AdapterHttpRequest, DataAdapter};
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for Semantic Web RDF / SPARQL endpoints (e.g. Wikidata).
pub struct SparqlAdapter {
    pub id: &'static str,
    pub endpoint: String,
}

/// Build a provenance quin with the crate's canonical parity checksum.
fn quin(s: u64, p: u64, o: u64) -> crate::NQuin {
    crate::NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: 0,
        metadata: 0,
        parity: s ^ p ^ o,
    }
}

impl SparqlAdapter {
    pub fn new(id: &'static str, endpoint: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
        }
    }

    /// Parse a standard SPARQL 1.1 Query Results JSON response (e.g. from Wikidata
    /// or any generic SPARQL endpoint) into provenance `NQuin`s.
    ///
    /// The mapping is generic and deterministic so the resulting quins are queryable
    /// by the SPARQL layer, which hashes IRIs with the same `generate_60bit_token`:
    ///
    /// For each `results.bindings` row `i` (iterating `head.vars` in declared order):
    /// * subject = the value of the FIRST variable whose binding is `type == "uri"`;
    ///   if the row binds no URI, subject = `generate_60bit_token("urn:sparql:row:{i}")`.
    /// * for every OTHER bound variable `v` in the row: predicate =
    ///   `generate_60bit_token("urn:sparql:var:{v}")`, object =
    ///   `generate_60bit_token(binding.value)`, emitting `quin(subject, predicate, object)`.
    ///
    /// If `results` or `results.bindings` are absent, an empty `Vec` is returned
    /// (not an error). `Err` is only returned when `body` is not valid JSON.
    pub fn parse_features(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        use crate::query::lexicon::generate_60bit_token;

        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

        // Ordered list of variable names from head.vars (deterministic subject selection).
        let vars: Vec<String> = json
            .get("head")
            .and_then(|h| h.get("vars"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Absent results / bindings => empty Vec, not an error.
        let bindings = match json
            .get("results")
            .and_then(|r| r.get("bindings"))
            .and_then(|b| b.as_array())
        {
            Some(b) => b,
            None => return Ok(Vec::new()),
        };

        let mut quins = Vec::new();

        for (i, row) in bindings.iter().enumerate() {
            // Pick subject: first head var bound in this row with type == "uri".
            let mut subject_var: Option<&str> = None;
            for v in &vars {
                if let Some(binding) = row.get(v.as_str()) {
                    if binding.get("type").and_then(|t| t.as_str()) == Some("uri") {
                        subject_var = Some(v.as_str());
                        break;
                    }
                }
            }

            let subject = match subject_var {
                Some(v) => {
                    let val = row
                        .get(v)
                        .and_then(|b| b.get("value"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    generate_60bit_token(val.as_bytes())
                }
                None => generate_60bit_token(format!("urn:sparql:row:{}", i).as_bytes()),
            };

            // Emit a quin for every OTHER bound variable in this row.
            for v in &vars {
                if Some(v.as_str()) == subject_var {
                    continue;
                }
                let value = match row
                    .get(v.as_str())
                    .and_then(|b| b.get("value"))
                    .and_then(|v| v.as_str())
                {
                    Some(val) => val,
                    None => continue, // variable not bound in this row
                };
                let predicate = generate_60bit_token(format!("urn:sparql:var:{}", v).as_bytes());
                let object = generate_60bit_token(value.as_bytes());
                quins.push(quin(subject, predicate, object));
            }
        }

        Ok(quins)
    }
}

impl DataAdapter for SparqlAdapter {
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for SPARQL endpoint {}",
                self.endpoint
            ));
        }

        // Construct geospatial SPARQL query. Using wdt:P625 for Earth-based coordinates.
        // Bounding box: west, south, east, north -> bbox.0, bbox.1, bbox.2, bbox.3
        let sparql_query = format!(
            "SELECT ?item ?itemLabel ?location WHERE {{
  ?item wdt:P625 ?location.
  SERVICE wikibase:box {{
    ?item wdt:P625 ?location.
    bd:serviceParam wikibase:cornerSouthWest \"Point({} {})\"^^geo:wktLiteral.
    bd:serviceParam wikibase:cornerNorthEast \"Point({} {})\"^^geo:wktLiteral.
  }}
  SERVICE wikibase:label {{ bd:serviceParam wikibase:language \"[AUTO_LANGUAGE],en\". }}
}}",
            bbox.0, bbox.1, bbox.2, bbox.3
        );

        let url = format!("{}?query={}&format=json", self.endpoint, urlencoding::encode(&sparql_query));

        Ok(AdapterHttpRequest::get(url, "SPARQL"))
    }

    fn parse_response(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        self.parse_features(body)
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        1 // A single complex SPARQL query.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparql_adapter_consent_denied() {
        let registry = NetworkDisclosureRegistry::new();
        let adapter = SparqlAdapter::new("sparql_adapter", "https://query.wikidata.org/sparql");

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }

    #[test]
    fn test_sparql_results_parse_features() {
        use crate::query::lexicon::generate_60bit_token;

        let body = r#"{"head":{"vars":["item","itemLabel","coord"]},
 "results":{"bindings":[
   {"item":{"type":"uri","value":"http://www.wikidata.org/entity/Q42"},
    "itemLabel":{"type":"literal","value":"Douglas Adams"},
    "coord":{"type":"literal","value":"Point(-0.13 51.5)"}},
   {"item":{"type":"uri","value":"http://www.wikidata.org/entity/Q64"},
    "itemLabel":{"type":"literal","value":"Berlin"}}
 ]}}"#;

        let adapter = SparqlAdapter::new("sparql_adapter", "https://query.wikidata.org/sparql");
        let quins = adapter.parse_features(body).expect("parse should succeed");

        // Row1 (Q42): itemLabel + coord => 2 quins. Row2 (Q64): itemLabel => 1 quin. Total 3.
        assert_eq!(quins.len(), 3, "expected 3 quins, got {}", quins.len());

        // The Q42 subject must have an `urn:sparql:var:itemLabel` quin with the Douglas Adams value.
        let q42_subject = generate_60bit_token(b"http://www.wikidata.org/entity/Q42");
        let label_pred = generate_60bit_token(b"urn:sparql:var:itemLabel");
        let douglas_obj = generate_60bit_token(b"Douglas Adams");

        let found = quins.iter().any(|q| {
            q.subject == q42_subject
                && q.predicate == label_pred
                && q.object == douglas_obj
                && q.parity == (q.subject ^ q.predicate ^ q.object)
        });
        assert!(found, "expected Q42 itemLabel=Douglas Adams quin not found");
    }
}
