use crate::domains::geospatial::adapters::AdapterHttpRequest;
use crate::net::disclosure::NetworkDisclosureRegistry;

pub enum OgcServiceType {
    Wms,
    Wfs,
    Wcs,
    Wmts,
}

/// Adapter for OGC Web Services (WMS, WFS, WCS, WMTS).
pub struct WmsAdapter {
    pub id: &'static str,
    pub endpoint: String,
    pub service_type: OgcServiceType,
}

impl WmsAdapter {
    pub fn new(id: &'static str, endpoint: &str, service_type: OgcServiceType) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
            service_type,
        }
    }
}

impl WmsAdapter {
    pub fn adapter_id(&self) -> &'static str {
        self.id
    }

    pub fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for endpoint {} by adapter {}",
                self.endpoint,
                self.adapter_id()
            ));
        }

        let (service, request) = match self.service_type {
            OgcServiceType::Wms => ("WMS", "GetMap"),
            OgcServiceType::Wfs => ("WFS", "GetFeature"),
            OgcServiceType::Wcs => ("WCS", "GetCoverage"),
            OgcServiceType::Wmts => ("WMTS", "GetTile"),
        };

        let mut url = format!(
            "{}?SERVICE={}&REQUEST={}&BBOX={},{},{},{}",
            self.endpoint, service, request, bbox.0, bbox.1, bbox.2, bbox.3
        );

        if time_range.1 > 0 {
            // Stub: proper ISO8601 formatting for TIME parameter
            url.push_str(&format!("&TIME={}/{}", time_range.0, time_range.1));
        }

        Ok(AdapterHttpRequest::get(url, "OGC"))
    }

    pub fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        let request = self.build_fetch_request(bbox, time_range, registry)?;
        super::execute_http_request_status(&request)?;
        // Parsing response data is deferred.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wms_adapter_egress() {
        let adapter = WmsAdapter::new(
            "wms_adapter",
            "https://wms.example.com/geoserver/wms",
            OgcServiceType::Wms,
        );
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
        if let Err(e) = res2 {
            assert!(
                !e.contains("Consent denied"),
                "Failed on consent when it should have been granted: {}",
                e
            );
        }
    }
}
