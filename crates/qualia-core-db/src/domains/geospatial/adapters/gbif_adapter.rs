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

        let gbif_query = format!(
            "{}?decimalLatitude={},{}&decimalLongitude={},{}&year={},{}",
            self.occurrence_endpoint,
            bbox.1, bbox.3,
            bbox.0, bbox.2,
            time_range.0, time_range.1
        );

        let client = reqwest::blocking::Client::new();
        let resp = client.get(&gbif_query).send().map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("GBIF API returned error: {}", resp.status()));
        }

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
}