use crate::net::disclosure::NetworkDisclosureRegistry;

/// Stub adapter for GBIF occurrence API bbox queries.
pub struct GbifAdapter {
    pub occurrence_endpoint: String,
}

impl GbifAdapter {
    pub fn new(occurrence_endpoint: &str) -> Self {
        Self {
            occurrence_endpoint: occurrence_endpoint.to_string(),
        }
    }
}

impl GbifAdapter {
    pub fn adapter_id(&self) -> &'static str {
        "gbif_adapter"
    }

    pub fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.occurrence_endpoint) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                self.occurrence_endpoint,
                self.adapter_id()
            ));
        }

        let _ = (bbox, time_range);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbif_adapter_egress() {
        let adapter = GbifAdapter::new("https://api.gbif.org/v1/occurrence/search");
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
}