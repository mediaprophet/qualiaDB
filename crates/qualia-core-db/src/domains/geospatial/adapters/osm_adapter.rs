use crate::domains::geospatial::adapters::AdapterHttpRequest;
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Stub adapter for OpenStreetMap Overpass API and MVT vector tile endpoints.
pub struct OsmAdapter {
    pub id: &'static str,
    pub overpass_endpoint: String,
    pub tile_endpoint: String,
}

impl OsmAdapter {
    pub fn new(id: &'static str, overpass_endpoint: &str, tile_endpoint: &str) -> Self {
        Self {
            id,
            overpass_endpoint: overpass_endpoint.to_string(),
            tile_endpoint: tile_endpoint.to_string(),
        }
    }
}

impl OsmAdapter {
    pub fn adapter_id(&self) -> &'static str {
        self.id
    }

    pub fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        let primary = &self.overpass_endpoint;
        if !registry.check_egress_consent(self.adapter_id(), primary) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                primary,
                self.adapter_id()
            ));
        }

        let date_filter = if time_range.1 > 0 {
            format!("[date:\"{}\"]", time_range.1) // Stub: proper ISO8601 formatting required
        } else {
            String::new()
        };

        // Construct Overpass QL bounding box: (south, west, north, east)
        let query = format!(
            "{date_filter}[out:json];(node({s},{w},{n},{e});way({s},{w},{n},{e});relation({s},{w},{n},{e}););out body;",
            date_filter = date_filter,
            s = bbox.1,
            w = bbox.0,
            n = bbox.3,
            e = bbox.2
        );

        Ok(AdapterHttpRequest::post_form(
            primary.clone(),
            query,
            "OSM Overpass",
        ))
    }

    pub fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        let request = self.build_fetch_request(bbox, time_range, registry)?;
        super::execute_http_request_status(&request)?;
        // Extruding building footprints and roads mapped to .10d Tensor models is deferred.
        Ok(())
    }

    /// Parse an OpenStreetMap Overpass API JSON response body into provenance `NQuin`s.
    ///
    /// Each element of the top-level `elements` array (a node, way, or relation) is
    /// mapped to a set of Dublin Core / RDF / WGS84 provenance quins, using
    /// `generate_60bit_token` for IRI/literal hashing so the emitted data is queryable
    /// by the SPARQL layer (which hashes IRIs the same way). If `elements` is absent an
    /// empty `Vec` is returned. Only invalid JSON yields an `Err`.
    pub fn parse_features(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        use crate::lexicon::generate_60bit_token;

        // Predicate hashes (constant per call).
        let title = generate_60bit_token(b"http://purl.org/dc/terms/title");
        let ty = generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let source = generate_60bit_token(b"http://purl.org/dc/terms/source");
        let lat_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#lat");
        let long_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#long");

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
        let elements = match json.get("elements").and_then(|e| e.as_array()) {
            Some(arr) => arr,
            None => return Ok(quins),
        };

        for el in elements {
            // `type` (node/way/relation) and `id` identify the element; skip if absent.
            let el_type = match el.get("type").and_then(|t| t.as_str()) {
                Some(t) => t,
                None => continue,
            };
            let id = match el.get("id").and_then(|i| i.as_u64()) {
                Some(i) => i,
                None => continue,
            };

            let el_iri = format!("https://www.openstreetmap.org/{el_type}/{id}");
            let subject = generate_60bit_token(el_iri.as_bytes());
            let kind =
                generate_60bit_token(format!("https://www.openstreetmap.org/{el_type}").as_bytes());

            // rdf:type -> osm element kind (always).
            quins.push(quin(subject, ty, kind));

            // dcterms:source -> the element IRI (always).
            quins.push(quin(
                subject,
                source,
                generate_60bit_token(el_iri.as_bytes()),
            ));

            // node lat/lon -> WGS84 geo (f64 bits stored as u64).
            if let Some(lat) = el.get("lat").and_then(|v| v.as_f64()) {
                quins.push(quin(subject, lat_p, lat.to_bits()));
            }
            if let Some(lon) = el.get("lon").and_then(|v| v.as_f64()) {
                quins.push(quin(subject, long_p, lon.to_bits()));
            }

            // Tags: `name` -> dcterms:title; every other tag -> OSM Key: predicate.
            if let Some(tags) = el.get("tags").and_then(|t| t.as_object()) {
                for (k, v) in tags {
                    let val = match v.as_str() {
                        Some(s) => s,
                        None => continue,
                    };
                    if k == "name" {
                        quins.push(quin(subject, title, generate_60bit_token(val.as_bytes())));
                    } else {
                        let pred = generate_60bit_token(
                            format!("https://wiki.openstreetmap.org/wiki/Key:{k}").as_bytes(),
                        );
                        quins.push(quin(subject, pred, generate_60bit_token(val.as_bytes())));
                    }
                }
            }
        }

        Ok(quins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osm_adapter_egress() {
        let adapter = OsmAdapter::new(
            "osm_adapter",
            "https://overpass-api.de/api/interpreter",
            "https://tiles.example.com/osm",
        );
        let registry = NetworkDisclosureRegistry::new();

        let res1 = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res1.is_err());

        let mut registry = registry;
        registry.register_egress(
            adapter.adapter_id(),
            "https://overpass-api.de/api/interpreter",
            "Fetch OSM features via Overpass",
            "User views map layer",
        );

        let res2 = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        if let Err(e) = res2 {
            assert!(
                !e.contains("Consent denied"),
                "Failed on consent when it should have been granted: {}",
                e
            );
        }
    }

    #[test]
    fn test_osm_parse_features() {
        use crate::lexicon::generate_60bit_token;

        let adapter = OsmAdapter::new(
            "osm_adapter",
            "https://overpass-api.de/api/interpreter",
            "https://tiles.example.com/osm",
        );

        // Realistic Overpass fixture: one node (name + lat + lon + amenity),
        // one way (name).
        let body = r#"{
            "version": 0.6,
            "generator": "Overpass API",
            "elements": [
                {
                    "type": "node",
                    "id": 1,
                    "lat": -37.8183,
                    "lon": 144.9671,
                    "tags": {
                        "name": "Flinders Street Station",
                        "amenity": "station"
                    }
                },
                {
                    "type": "way",
                    "id": 2,
                    "nodes": [1, 2, 3],
                    "tags": {
                        "name": "Yarra River",
                        "waterway": "river"
                    }
                }
            ]
        }"#;

        let quins = adapter
            .parse_features(body)
            .expect("valid JSON should parse");

        // Node emits: rdf:type, source, lat, long, title, amenity tag = 6.
        // Way emits: rdf:type, source, title, waterway tag = 4.
        assert_eq!(quins.len(), 10, "unexpected quin count: {}", quins.len());

        // Recompute the node's dcterms:title quin and assert it is present.
        let title = generate_60bit_token(b"http://purl.org/dc/terms/title");
        let node_subject = generate_60bit_token(b"https://www.openstreetmap.org/node/1");
        let node_name = generate_60bit_token(b"Flinders Street Station");

        assert!(
            quins.iter().any(|q| {
                q.subject == node_subject && q.predicate == title && q.object == node_name
            }),
            "node TITLE quin missing from parsed output"
        );

        // Sanity: an empty/absent elements array yields an empty Vec, not an error.
        let empty = adapter.parse_features(r#"{"version":0.6}"#).expect("valid");
        assert!(empty.is_empty());

        // Invalid JSON is the only error path.
        assert!(adapter.parse_features("not json").is_err());
    }
}
