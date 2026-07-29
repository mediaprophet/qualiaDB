use crate::domains::geospatial::adapters::{AdapterHttpRequest, DataAdapter};
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for IVOA Table Access Protocol (TAP) endpoints using ADQL.
/// Maps bounding boxes to ICRS Right Ascension (RA) / Declination (Dec).
pub struct IvoaTapAdapter {
    pub id: &'static str,
    pub endpoint: String,
    pub catalog_name: String,
}

impl IvoaTapAdapter {
    pub fn new(id: &'static str, endpoint: &str, catalog_name: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
            catalog_name: catalog_name.to_string(),
        }
    }
}

impl DataAdapter for IvoaTapAdapter {
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    fn build_fetch_request(
        &self,
        bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<AdapterHttpRequest, String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for IVOA TAP endpoint {}",
                self.endpoint
            ));
        }

        let (ra_min, dec_min, ra_max, dec_max) = bbox;
        let adql_query = format!(
            "SELECT * FROM {} WHERE 1=CONTAINS(POINT('ICRS', ra, dec), POLYGON('ICRS', {},{}, {},{}, {},{}, {},{}))",
            self.catalog_name,
            ra_min, dec_min,
            ra_max, dec_min,
            ra_max, dec_max,
            ra_min, dec_max
        );

        let body = format!(
            "REQUEST=doQuery&LANG=ADQL&QUERY={}",
            urlencoding::encode(&adql_query)
        );

        Ok(AdapterHttpRequest::post_form(
            self.endpoint.clone(),
            body,
            "IVOA TAP",
        ))
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        // TAP query is typically 1 request. Paging via MAXREC if supported.
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ivoa_tap_consent_denied() {
        let registry = NetworkDisclosureRegistry::new();
        let adapter = IvoaTapAdapter::new(
            "ivoa_tap",
            "https://gea.esac.esa.int/tap-server/tap/sync",
            "gaiadr3.gaia_source",
        );

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }
}
