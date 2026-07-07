use crate::net::disclosure::NetworkDisclosureRegistry;

/// Stub adapter for OGC WMS GetMap requests.
pub struct WmsAdapter {
    pub getmap_endpoint: String,
}

impl WmsAdapter {
    pub fn new(getmap_endpoint: &str) -> Self {
        Self {
            getmap_endpoint: getmap_endpoint.to_string(),
        }
    }
}

impl WmsAdapter {
    pub fn adapter_id(&self) -> &'static str {
        "wms_adapter"
    }

    pub fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.getmap_endpoint) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                self.getmap_endpoint,
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
    fn test_wms_adapter_egress() {
        let adapter = WmsAdapter::new("https://wms.example.com/geoserver/wms");
        let registry = NetworkDisclosureRegistry::new();

        let res1 = adapter.fetch_region((-1.0, 51.0, 0.0, 52.0), (0, 0), &registry);
        assert!(res1.is_err());

        let mut registry = registry;
        registry.register_egress(
            adapter.adapter_id(),
            "https://wms.example.com/geoserver/wms",
            "Fetch raster map tiles via WMS GetMap",
            "User enables base map layer",
        );

        let res2 = adapter.fetch_region((-1.0, 51.0, 0.0, 52.0), (0, 0), &registry);
        assert!(res2.is_ok());
    }
}