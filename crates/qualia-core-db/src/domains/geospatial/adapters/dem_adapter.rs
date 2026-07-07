use crate::net::disclosure::NetworkDisclosureRegistry;

pub struct DemAdapter {
    pub endpoint: String,
}

impl DemAdapter {
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
        }
    }

    pub fn adapter_id(&self) -> &'static str {
        "dem_adapter"
    }

    pub fn fetch_region(
        &self,
        _bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                self.endpoint,
                self.adapter_id()
            ));
        }

        // Stub for fetching Cloud-Optimised GeoTIFF and passing to heightfield.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dem_adapter_egress() {
        let adapter = DemAdapter::new("https://elevation.example.com");
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
        assert!(res2.is_ok());
    }
}