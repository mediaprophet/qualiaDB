use crate::net::disclosure::NetworkDisclosureRegistry;

/// Stub adapter for OpenStreetMap Overpass API and MVT vector tile endpoints.
pub struct OsmAdapter {
    pub overpass_endpoint: String,
    pub mvt_endpoint: String,
}

impl OsmAdapter {
    pub fn new(overpass_endpoint: &str, mvt_endpoint: &str) -> Self {
        Self {
            overpass_endpoint: overpass_endpoint.to_string(),
            mvt_endpoint: mvt_endpoint.to_string(),
        }
    }
}

impl OsmAdapter {
    pub fn adapter_id(&self) -> &'static str {
        "osm_adapter"
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

        let _ = (bbox, time_range, &self.mvt_endpoint);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osm_adapter_egress() {
        let adapter = OsmAdapter::new(
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
        assert!(res2.is_ok());
    }
}