use crate::domains::geospatial::adapters::AdapterHttpRequest;
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Stub adapter for GBIF occurrence API bbox queries.
pub struct GbifAdapter {
    pub id: &'static str,
    pub occurrence_endpoint: String,
}

impl GbifAdapter {
    pub fn new(id: &'static str, occurrence_endpoint: &str) -> Self {
        Self {
            id,
            occurrence_endpoint: occurrence_endpoint.to_string(),
        }
    }
}

impl GbifAdapter {
    pub fn adapter_id(&self) -> &'static str {
        self.id
    }

    pub fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.occurrence_endpoint) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                self.occurrence_endpoint,
                self.adapter_id()
            ));
        }

        let gbif_query = format!(
            "{}?decimalLatitude={},{}&decimalLongitude={},{}&year={},{}",
            self.occurrence_endpoint, bbox.1, bbox.3, bbox.0, bbox.2, time_range.0, time_range.1
        );

        Ok(AdapterHttpRequest::get(gbif_query, "GBIF"))
    }

    /// Parse a GBIF occurrence-search JSON response body into provenance `NQuin`s.
    ///
    /// The `results` array (each element an occurrence record) is mapped to a set of
    /// Dublin Core / WGS84 provenance quins per record, using `generate_60bit_token`
    /// for IRI/literal hashing so the emitted data is queryable by the SPARQL layer
    /// (which hashes IRIs the same way). If `results` is absent an empty `Vec` is
    /// returned. Only invalid JSON yields an `Err`.
    pub fn parse_features(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        use crate::lexicon::generate_60bit_token;

        // Predicate / type-object hashes (constant per call).
        let title = generate_60bit_token(b"http://purl.org/dc/terms/title");
        let ty = generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let license = generate_60bit_token(b"http://purl.org/dc/terms/license");
        let created = generate_60bit_token(b"http://purl.org/dc/terms/created");
        let source = generate_60bit_token(b"http://purl.org/dc/terms/source");
        let lat_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#lat");
        let long_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#long");
        let kind = generate_60bit_token(b"https://www.gbif.org/occurrence");

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

        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

        let mut quins = Vec::new();
        let results = match json.get("results").and_then(|r| r.as_array()) {
            Some(arr) => arr,
            None => return Ok(quins),
        };

        for rec in results {
            // `key` identifies the occurrence; skip records without one.
            let key = match rec.get("key").and_then(|k| k.as_u64()) {
                Some(k) => k,
                None => continue,
            };
            let occ_iri = format!("https://www.gbif.org/occurrence/{key}");
            let subject = generate_60bit_token(occ_iri.as_bytes());

            // rdf:type -> gbif occurrence (always).
            quins.push(quin(subject, ty, kind));

            if let Some(name) = rec.get("scientificName").and_then(|v| v.as_str()) {
                quins.push(quin(subject, title, generate_60bit_token(name.as_bytes())));
            }
            if let Some(lic) = rec.get("license").and_then(|v| v.as_str()) {
                quins.push(quin(subject, license, generate_60bit_token(lic.as_bytes())));
            }
            if let Some(date) = rec.get("eventDate").and_then(|v| v.as_str()) {
                quins.push(quin(
                    subject,
                    created,
                    generate_60bit_token(date.as_bytes()),
                ));
            }

            // dc:source -> occurrence IRI (always).
            quins.push(quin(
                subject,
                source,
                generate_60bit_token(occ_iri.as_bytes()),
            ));

            if let Some(lat) = rec.get("decimalLatitude").and_then(|v| v.as_f64()) {
                quins.push(quin(subject, lat_p, lat.to_bits()));
            }
            if let Some(lon) = rec.get("decimalLongitude").and_then(|v| v.as_f64()) {
                quins.push(quin(subject, long_p, lon.to_bits()));
            }
        }

        Ok(quins)
    }

    pub fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        let request = self.build_fetch_request(bbox, time_range, registry)?;
        super::execute_http_request_status(&request)?;
        // Parsing JSON and emitting 4D semantic points is deferred.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbif_adapter_egress() {
        let adapter = GbifAdapter::new("gbif_adapter", "https://api.gbif.org/v1/occurrence/search");
        let registry = NetworkDisclosureRegistry::new();

        let res1 = adapter.fetch_region((10.0, 45.0, 11.0, 46.0), (2020, 2024), &registry);
        assert!(res1.is_err());

        let mut registry = registry;
        registry.register_egress(
            adapter.adapter_id(),
            "https://api.gbif.org/v1/occurrence/search",
            "Fetch species occurrence records in bbox",
            "User enables biodiversity layer",
        );

        let res2 = adapter.fetch_region((10.0, 45.0, 11.0, 46.0), (2020, 2024), &registry);
        assert!(res2.is_ok());
    }

    #[test]
    fn test_gbif_parse_features() {
        use crate::lexicon::generate_60bit_token;

        let adapter = GbifAdapter::new("gbif_adapter", "https://api.gbif.org/v1/occurrence/search");

        // Realistic 2-occurrence fixture.
        // Record 1: full record -> type, title, license, created, source, lat, long = 7 quins.
        // Record 2: minimal (no license/eventDate/coords) -> type, title, source = 3 quins.
        let body = r#"{
            "offset": 0,
            "limit": 20,
            "endOfRecords": false,
            "count": 1234,
            "results": [
                {
                    "key": 123456,
                    "scientificName": "Panthera leo",
                    "decimalLatitude": -1.2,
                    "decimalLongitude": 36.8,
                    "eventDate": "2019-05-01T00:00:00",
                    "datasetKey": "abc-def",
                    "license": "http://creativecommons.org/licenses/by/4.0/legalcode"
                },
                {
                    "key": 654321,
                    "scientificName": "Loxodonta africana"
                }
            ]
        }"#;

        let quins = adapter.parse_features(body).expect("valid JSON parses");
        // 7 from record 1 + 3 from record 2 = 10.
        assert_eq!(quins.len(), 10, "expected 10 quins, got {}", quins.len());

        // Recompute expected hashes the same way the parser does.
        let title = generate_60bit_token(b"http://purl.org/dc/terms/title");
        let subj1 = generate_60bit_token(b"https://www.gbif.org/occurrence/123456");
        let title_obj1 = generate_60bit_token(b"Panthera leo");

        assert!(
            quins
                .iter()
                .any(|q| q.subject == subj1 && q.predicate == title && q.object == title_obj1),
            "expected TITLE quin for Panthera leo not found"
        );

        // Absent-results object -> empty Vec, not an error.
        let empty = adapter
            .parse_features(r#"{"offset":0,"count":0}"#)
            .expect("no results field is not an error");
        assert!(empty.is_empty());

        // Invalid JSON -> Err.
        assert!(adapter.parse_features("{not json").is_err());
    }
}
