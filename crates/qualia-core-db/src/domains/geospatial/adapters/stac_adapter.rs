use crate::domains::geospatial::adapters::{AdapterHttpRequest, DataAdapter};
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for SpatioTemporal Asset Catalog (STAC) API endpoints.
/// Supports querying JSON-based metadata via 4D bounding boxes (x, y, z, t).
pub struct StacAdapter {
    pub id: &'static str,
    pub endpoint: String,
    pub collection: Option<String>,
}

/// Build a provenance NQuin from three pre-hashed 60-bit tokens.
/// Parity is the XOR fold of the three semantic vectors (matches the
/// convention used elsewhere for lightweight integrity checks).
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

impl StacAdapter {
    pub fn new(id: &'static str, endpoint: &str, collection: Option<&str>) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
            collection: collection.map(|s| s.to_string()),
        }
    }

    /// Parse a STAC item-search response (a GeoJSON `FeatureCollection`) into
    /// provenance `NQuin`s. IRIs are hashed with `generate_60bit_token` so the
    /// resulting quins are queryable by the SPARQL layer, which hashes the same
    /// way. Returns `Err` only when the body is not valid JSON; a response with
    /// no `features` array yields an empty `Vec`.
    pub fn parse_features(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        use crate::lexicon::generate_60bit_token;

        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

        // Predicate / kind hashes (stable, IRI-derived).
        let title_p = generate_60bit_token(b"http://purl.org/dc/terms/title");
        let type_p = generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let license_p = generate_60bit_token(b"http://purl.org/dc/terms/license");
        let created_p = generate_60bit_token(b"http://purl.org/dc/terms/created");
        let source_p = generate_60bit_token(b"http://purl.org/dc/terms/source");
        let lat_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#lat");
        let long_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#long");
        let kind_o = generate_60bit_token(b"https://stacspec.org/Item");

        let features = match json.get("features").and_then(|f| f.as_array()) {
            Some(f) => f,
            None => return Ok(Vec::new()),
        };

        let mut quins = Vec::new();
        for (i, feature) in features.iter().enumerate() {
            // Subject id: STAC feature "id", or a stable index fallback.
            let id = feature
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("stac:item:{}", i));
            let subject = generate_60bit_token(id.as_bytes());

            // Always: title (the item id) and rdf:type = STAC Item.
            quins.push(quin(subject, title_p, generate_60bit_token(id.as_bytes())));
            quins.push(quin(subject, type_p, kind_o));

            let props = feature.get("properties");

            // properties.datetime -> dc:created
            if let Some(dt) = props
                .and_then(|p| p.get("datetime"))
                .and_then(|v| v.as_str())
            {
                quins.push(quin(
                    subject,
                    created_p,
                    generate_60bit_token(dt.as_bytes()),
                ));
            }

            // properties.license -> dc:license
            if let Some(lic) = props
                .and_then(|p| p.get("license"))
                .and_then(|v| v.as_str())
            {
                quins.push(quin(
                    subject,
                    license_p,
                    generate_60bit_token(lic.as_bytes()),
                ));
            }

            // First asset href -> dc:source
            if let Some(href) = feature
                .get("assets")
                .and_then(|a| a.as_object())
                .and_then(|assets| assets.values().next())
                .and_then(|asset| asset.get("href"))
                .and_then(|v| v.as_str())
            {
                quins.push(quin(
                    subject,
                    source_p,
                    generate_60bit_token(href.as_bytes()),
                ));
            }

            // bbox (>=4 numbers) -> centre lat/long as raw f64 bits.
            if let Some(bbox) = feature.get("bbox").and_then(|v| v.as_array()) {
                if bbox.len() >= 4 {
                    let coords: Option<Vec<f64>> =
                        bbox.iter().take(4).map(|v| v.as_f64()).collect();
                    if let Some(c) = coords {
                        let long_c = (c[0] + c[2]) / 2.0;
                        let lat_c = (c[1] + c[3]) / 2.0;
                        quins.push(quin(subject, lat_p, lat_c.to_bits()));
                        quins.push(quin(subject, long_p, long_c.to_bits()));
                    }
                }
            }
        }

        Ok(quins)
    }
}

impl DataAdapter for StacAdapter {
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for STAC endpoint {}",
                self.endpoint
            ));
        }

        // Translate the bounding box and time_range into STAC API parameters
        let stac_bbox = format!("{},{},{},{}", bbox.0, bbox.1, bbox.2, bbox.3);
        let stac_datetime = format!("{}/{}", time_range.0, time_range.1); // Stub: properly format ISO8601 strings

        let mut url = format!(
            "{}/search?bbox={}&datetime={}",
            self.endpoint.trim_end_matches('/'),
            stac_bbox,
            stac_datetime
        );
        if let Some(c) = &self.collection {
            url.push_str(&format!("&collections={}", c));
        }

        Ok(AdapterHttpRequest::get(url, "STAC"))
    }

    fn parse_response(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        self.parse_features(body)
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        // STAC search is usually a single API query that returns multiple item records
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stac_adapter_consent_denied() {
        let registry = NetworkDisclosureRegistry::new();
        let adapter = StacAdapter::new(
            "stac_adapter",
            "https://planetarycomputer.microsoft.com/api/stac/v1",
            Some("landsat-c2-l2"),
        );

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }

    #[test]
    fn test_stac_parse_features() {
        use crate::lexicon::generate_60bit_token;

        let body = r#"{
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "id": "S2A_31UFU_20230501",
                    "bbox": [4.5, 51.0, 5.5, 52.0],
                    "properties": {
                        "datetime": "2023-05-01T10:00:00Z",
                        "license": "proprietary",
                        "platform": "sentinel-2a"
                    },
                    "assets": {
                        "visual": {"href": "https://example/S2A.tif", "type": "image/tiff"}
                    },
                    "collection": "sentinel-2-l2a"
                }
            ]
        }"#;

        let adapter = StacAdapter::new("stac_adapter", "https://example/stac/v1", None);
        let quins = adapter.parse_features(body).expect("valid STAC JSON");

        // title + type + created + license + source + lat + long = 7 quins.
        assert_eq!(quins.len(), 7, "unexpected quin count: {}", quins.len());

        // The CREATED quin must be present with the exact expected hashes.
        let subject = generate_60bit_token(b"S2A_31UFU_20230501");
        let created_p = generate_60bit_token(b"http://purl.org/dc/terms/created");
        let created_o = generate_60bit_token(b"2023-05-01T10:00:00Z");
        assert!(
            quins
                .iter()
                .any(|q| q.subject == subject && q.predicate == created_p && q.object == created_o),
            "expected CREATED quin not found"
        );
    }
}
