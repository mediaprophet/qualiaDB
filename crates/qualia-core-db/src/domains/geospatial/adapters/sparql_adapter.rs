use crate::domains::geospatial::adapters::{AdapterHttpRequest, DataAdapter};
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for Semantic Web RDF / SPARQL endpoints (e.g. Wikidata).
pub struct SparqlAdapter {
    pub id: &'static str,
    pub endpoint: String,
}

impl SparqlAdapter {
    pub fn new(id: &'static str, endpoint: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
        }
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
}
