use crate::domains::geospatial::adapters::{AdapterHttpRequest, DataAdapter};
use crate::net::disclosure::NetworkDisclosureRegistry;

pub struct Ogc3dTilesAdapter {
    pub id: &'static str,
    pub endpoint: String,
}

impl Ogc3dTilesAdapter {
    pub fn new(id: &'static str, endpoint: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
        }
    }

    /// Parse an OGC 3D Tiles `tileset.json` document into provenance `NQuin`s.
    ///
    /// Walks the tile tree from `root` recursively (root plus every descendant
    /// reachable through `children`) and emits one subject per tile that carries
    /// a `content.uri`. Hashing follows the canonical SPARQL IRI convention
    /// (`crate::lexicon::generate_60bit_token`) so the emitted graph is queryable
    /// with the same predicate/subject hashes the query layer computes.
    ///
    /// Emitted per tile with content:
    /// - `(subject, rdf:type, 3d-tiles#Tile)`
    /// - `(subject, dcterms:source, hash(content.uri))`
    /// - `(subject, 3d-tiles#geometricError, err.to_bits())` when present
    /// - `(subject, geo:lat, lat_deg.to_bits())` and `(subject, geo:long, long_deg.to_bits())`
    ///   when the tile has a `region` bounding volume (centre of the region,
    ///   radians converted to degrees). Tiles with only a `box` volume skip lat/long.
    ///
    /// Returns `Err` only when `body` is not valid JSON.
    pub fn parse_features(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        use crate::lexicon::generate_60bit_token;

        let type_p = generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let source_p = generate_60bit_token(b"http://purl.org/dc/terms/source");
        let geomerr_p =
            generate_60bit_token(b"https://github.com/CesiumGS/3d-tiles#geometricError");
        let lat_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#lat");
        let long_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#long");
        let tile_kind = generate_60bit_token(b"https://github.com/CesiumGS/3d-tiles#Tile");

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

        // Recursively walk a tile node, emitting quins for tiles with content.
        #[allow(clippy::too_many_arguments)]
        fn walk(
            tile: &serde_json::Value,
            depth: usize,
            out: &mut Vec<crate::NQuin>,
            type_p: u64,
            source_p: u64,
            geomerr_p: u64,
            lat_p: u64,
            long_p: u64,
            tile_kind: u64,
        ) {
            // Bound recursion to guard against pathological nesting.
            if depth > 64 {
                return;
            }

            if let Some(uri) = tile
                .get("content")
                .and_then(|c| c.get("uri"))
                .and_then(|u| u.as_str())
            {
                let subject = generate_60bit_token(uri.as_bytes());

                out.push(quin(subject, type_p, tile_kind));
                out.push(quin(
                    subject,
                    source_p,
                    generate_60bit_token(uri.as_bytes()),
                ));

                if let Some(err) = tile.get("geometricError").and_then(|g| g.as_f64()) {
                    out.push(quin(subject, geomerr_p, err.to_bits()));
                }

                // Prefer a `region` bounding volume for a geographic centre.
                // A `box`-only volume has no lat/long to derive, so it is skipped.
                if let Some(region) = tile
                    .get("boundingVolume")
                    .and_then(|bv| bv.get("region"))
                    .and_then(|r| r.as_array())
                {
                    if region.len() >= 4 {
                        let west = region[0].as_f64().unwrap_or(0.0);
                        let south = region[1].as_f64().unwrap_or(0.0);
                        let east = region[2].as_f64().unwrap_or(0.0);
                        let north = region[3].as_f64().unwrap_or(0.0);
                        let rad_to_deg = 180.0 / std::f64::consts::PI;
                        let lat_deg = ((south + north) / 2.0) * rad_to_deg;
                        let long_deg = ((west + east) / 2.0) * rad_to_deg;
                        out.push(quin(subject, lat_p, lat_deg.to_bits()));
                        out.push(quin(subject, long_p, long_deg.to_bits()));
                    }
                }
            }

            if let Some(children) = tile.get("children").and_then(|c| c.as_array()) {
                for child in children {
                    walk(
                        child,
                        depth + 1,
                        out,
                        type_p,
                        source_p,
                        geomerr_p,
                        lat_p,
                        long_p,
                        tile_kind,
                    );
                }
            }
        }

        let doc: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

        let mut out = Vec::new();
        if let Some(root) = doc.get("root") {
            walk(
                root, 0, &mut out, type_p, source_p, geomerr_p, lat_p, long_p, tile_kind,
            );
        }

        Ok(out)
    }
}

impl DataAdapter for Ogc3dTilesAdapter {
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    fn build_fetch_request(
        &self,
        _bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err("Consent denied for OGC 3D Tiles fetch".into());
        }

        let url = if self.endpoint.ends_with("tileset.json") {
            self.endpoint.clone()
        } else {
            format!("{}/tileset.json", self.endpoint.trim_end_matches('/'))
        };

        Ok(AdapterHttpRequest::get(url, "OGC 3D Tiles"))
    }

    fn parse_response(&self, body: &str) -> Result<Vec<crate::NQuin>, String> {
        self.parse_features(body)
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        10 // Honest estimate of HLOD nodes based on bounding box
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ogc_3d_tiles_consent_denied() {
        let registry = NetworkDisclosureRegistry::new();
        let adapter = Ogc3dTilesAdapter::new(
            "ogc_3d_tiles",
            "https://assets.ion.cesium.com/1/tileset.json",
        );

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }

    #[test]
    fn test_3dtiles_parse_features() {
        use crate::lexicon::generate_60bit_token;

        let tileset = r#"{"asset":{"version":"1.1"},"geometricError":500.0,
 "root":{"boundingVolume":{"region":[-1.3,0.6,-1.2,0.7,0,200]},"geometricError":100.0,"refine":"ADD",
   "content":{"uri":"root.b3dm"},
   "children":[
     {"boundingVolume":{"box":[0,0,0, 100,0,0, 0,100,0, 0,0,50]},"geometricError":10.0,"content":{"uri":"0/0.b3dm"}},
     {"boundingVolume":{"region":[-1.29,0.61,-1.21,0.69,0,150]},"geometricError":10.0,"content":{"uri":"0/1.b3dm"}}
   ]}}"#;

        let adapter = Ogc3dTilesAdapter::new("ogc_3d_tiles", "https://example.com/tileset.json");
        let quins = adapter
            .parse_features(tileset)
            .expect("valid tileset should parse");

        let type_p = generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let source_p = generate_60bit_token(b"http://purl.org/dc/terms/source");
        let tile_kind = generate_60bit_token(b"https://github.com/CesiumGS/3d-tiles#Tile");

        // 3 tiles have content -> 3 distinct subjects.
        let subjects: std::collections::HashSet<u64> = quins
            .iter()
            .filter(|q| q.predicate == type_p && q.object == tile_kind)
            .map(|q| q.subject)
            .collect();
        assert_eq!(
            subjects.len(),
            3,
            "expected 3 tile subjects (root + 2 children)"
        );

        // Quin accounting:
        //   root:  type + source + geomErr + lat + long        = 5 (region)
        //   0/0:   type + source + geomErr                      = 3 (box only, no lat/long)
        //   0/1:   type + source + geomErr + lat + long         = 5 (region)
        // total = 13
        assert_eq!(
            quins.len(),
            13,
            "expected 13 quins total, got {}",
            quins.len()
        );

        // A known tile's SOURCE quin is present (recompute hashes here).
        let child_subject = generate_60bit_token(b"0/1.b3dm");
        let child_source_obj = generate_60bit_token(b"0/1.b3dm");
        assert!(
            quins.iter().any(|q| q.subject == child_subject
                && q.predicate == source_p
                && q.object == child_source_obj),
            "expected SOURCE quin for tile 0/1.b3dm"
        );

        // The box-only child must NOT emit lat/long.
        let box_subject = generate_60bit_token(b"0/0.b3dm");
        let lat_p = generate_60bit_token(b"http://www.w3.org/2003/01/geo/wgs84_pos#lat");
        assert!(
            !quins
                .iter()
                .any(|q| q.subject == box_subject && q.predicate == lat_p),
            "box-only tile must not emit a lat quin"
        );

        // Invalid JSON returns Err.
        assert!(adapter.parse_features("{not json").is_err());
    }
}
