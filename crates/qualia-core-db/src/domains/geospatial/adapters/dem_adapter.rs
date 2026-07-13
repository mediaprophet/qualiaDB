use crate::domains::geospatial::adapters::AdapterHttpRequest;
use crate::net::disclosure::NetworkDisclosureRegistry;

pub struct DemAdapter {
    pub id: &'static str,
    pub endpoint: String,
}

impl DemAdapter {
    pub fn new(id: &'static str, endpoint: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
        }
    }

    pub fn adapter_id(&self) -> &'static str {
        self.id
    }

    pub fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                self.endpoint,
                self.adapter_id()
            ));
        }

        // 1. Convert bbox to appropriate COG tile coordinates / WCS query
        let query = format!(
            "{}?SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&SUBSET=x({},{})&SUBSET=y({},{})",
            self.endpoint, bbox.0, bbox.2, bbox.1, bbox.3
        );

        Ok(AdapterHttpRequest::get(query, "DEM"))
    }

    pub fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        let request = self.build_fetch_request(bbox, time_range, registry)?;
        super::execute_http_request_status(&request)?;
        // 2. Stream elevation data from endpoint is verified.
        // 3. Piping elevation heightfield into the Marching Cubes / QEM LOD engine to output .10d meshes is deferred.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dem_adapter_egress() {
        let adapter = DemAdapter::new("dem_adapter", "https://elevation.example.com");
        let mut registry = NetworkDisclosureRegistry::new();

        let res1 = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res1.is_err());

        registry.register_egress(
            adapter.adapter_id(),
            "https://elevation.example.com",
            "Fetch DEM data",
            "User viewed map",
        );

        let res2 = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        if let Err(e) = res2 {
            assert!(!e.contains("Consent denied"), "Failed on consent when it should have been granted: {}", e);
        }
    }
}
