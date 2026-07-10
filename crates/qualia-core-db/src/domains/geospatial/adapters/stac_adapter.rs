use crate::domains::geospatial::adapters::{AdapterHttpRequest, DataAdapter};
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for SpatioTemporal Asset Catalog (STAC) API endpoints.
/// Supports querying JSON-based metadata via 4D bounding boxes (x, y, z, t).
pub struct StacAdapter {
    pub id: &'static str,
    pub endpoint: String,
    pub collection: Option<String>,
}

impl StacAdapter {
    pub fn new(id: &'static str, endpoint: &str, collection: Option<&str>) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
            collection: collection.map(|s| s.to_string()),
        }
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

        let mut url = format!("{}/search?bbox={}&datetime={}", self.endpoint.trim_end_matches('/'), stac_bbox, stac_datetime);
        if let Some(c) = &self.collection {
            url.push_str(&format!("&collections={}", c));
        }

        Ok(AdapterHttpRequest::get(url, "STAC"))
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
        let adapter = StacAdapter::new("stac_adapter", "https://planetarycomputer.microsoft.com/api/stac/v1", Some("landsat-c2-l2"));

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }
}
