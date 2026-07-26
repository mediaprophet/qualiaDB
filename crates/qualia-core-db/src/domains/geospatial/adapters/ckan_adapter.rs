use crate::domains::geospatial::adapters::AdapterHttpRequest;
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for dynamically discovering and fetching datasets from CKAN federated portals.
pub struct CkanAdapter {
    pub id: &'static str,
    pub api_endpoint: String,
}

impl CkanAdapter {
    pub fn new(id: &'static str, api_endpoint: &str) -> Self {
        Self {
            id,
            api_endpoint: api_endpoint.to_string(),
        }
    }
}

impl super::DataAdapter for CkanAdapter {
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        let search_endpoint = format!("{}/action/package_search", self.api_endpoint);

        if !registry.check_egress_consent(self.adapter_id(), &search_endpoint) {
            return Err(format!(
                "Consent denied or unregistered for CKAN endpoint {} by adapter {}",
                search_endpoint,
                self.adapter_id()
            ));
        }

        let query = format!(
            "{}?ext_bbox={},{},{},{}&rows=50",
            search_endpoint, bbox.0, bbox.1, bbox.2, bbox.3
        );

        Ok(AdapterHttpRequest::get(query, "CKAN"))
    }

    fn needs_fetch_body(&self) -> bool {
        true
    }

    fn handle_fetch_body(&self, body: &str) -> Result<(), String> {
        let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;

        let results = json
            .get("result")
            .and_then(|r| r.get("results"))
            .and_then(|r| r.as_array())
            .ok_or_else(|| "Failed to parse CKAN results array".to_string())?;

        for dataset in results {
            let title = dataset
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            let license = dataset
                .get("license_title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown");
            let created = dataset
                .get("metadata_created")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if let Some(resources) = dataset.get("resources").and_then(|v| v.as_array()) {
                for res in resources {
                    let format = res.get("format").and_then(|v| v.as_str()).unwrap_or("");
                    let url = res.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    // In a full implementation, this would route to a dynamic ingestion queue.
                    println!("Discovered CKAN Dataset '{}' | Format: {} | License: {} | Created: {} | URL: {}", 
                             title, format, license, created, url);
                }
            }
        }

        Ok(())
    }

    fn primary_endpoint(&self) -> &str {
        &self.api_endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        // We estimate 1 API query to search CKAN for datasets overlapping the bbox
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::geospatial::adapters::DataAdapter;

    #[test]
    fn test_ckan_adapter_egress() {
        let adapter = CkanAdapter::new("ckan_test", "https://data.gov.au/data/api/3");
        let mut registry = NetworkDisclosureRegistry::new();

        let res1 = adapter.fetch_region((144.9, -37.9, 145.0, -37.8), (0, 0), &registry);
        assert!(res1.is_err());

        registry.register_egress(
            adapter.adapter_id(),
            "https://data.gov.au/data/api/3/action/package_search",
            "Query federated CKAN repository for datasets",
            "User executes spatial search",
        );

        let res2 = adapter.fetch_region((144.9, -37.9, 145.0, -37.8), (0, 0), &registry);
        // During tests, we tolerate network timeouts/DNS issues
        if let Err(e) = res2 {
            assert!(
                !e.contains("Consent denied"),
                "Failed on consent when it should have been granted: {}",
                e
            );
        }
    }
}
