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

    pub fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
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

        let client = reqwest::blocking::Client::new();
        let resp = client.post(primary)
            .body(query)
            .send()
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("OSM Overpass API returned error: {}", resp.status()));
        }

        // Extruding building footprints and roads mapped to .10d Tensor models is deferred.
        Ok(())
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
            assert!(!e.contains("Consent denied"), "Failed on consent when it should have been granted: {}", e);
        }
    }
}