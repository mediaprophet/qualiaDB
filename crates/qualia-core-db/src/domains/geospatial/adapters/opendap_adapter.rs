use crate::domains::geospatial::adapters::DataAdapter;
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for Open-source Project for a Network Data Access Protocol (OPeNDAP) endpoints.
/// Primarily used for subsetting massive 4D/5D NetCDF arrays (e.g. atmospheric layers, ocean currents).
pub struct OpendapAdapter {
    pub id: &'static str,
    pub endpoint: String,
    pub dataset_id: String,
}

impl OpendapAdapter {
    pub fn new(id: &'static str, endpoint: &str, dataset_id: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
            dataset_id: dataset_id.to_string(),
        }
    }
}

impl DataAdapter for OpendapAdapter {
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    fn fetch_region(
        &self,
        _bbox: (f64, f64, f64, f64),
        _time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for OPeNDAP endpoint {}",
                self.endpoint
            ));
        }

        // Construct OPeNDAP constraint expression assuming standard lat/lon mapping for a spatial subset.
        // Example: ?variable[lat_idx_min:1:lat_idx_max][lon_idx_min:1:lon_idx_max]
        // Since we don't have the DDS to resolve indices here, we fetch the DDS endpoint to verify access.
        let dds_url = format!("{}.dds", self.endpoint);

        let client = reqwest::blocking::Client::new();
        let resp = client.get(&dds_url).send().map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("OPeNDAP API returned error: {}", resp.status()));
        }

        // Translation of regional search into constraint expressions via DDS parsing is deferred.
        Ok(())
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        // Typically a DDS fetch followed by a targeted subset request
        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opendap_adapter_consent_denied() {
        let registry = NetworkDisclosureRegistry::new();
        let adapter = OpendapAdapter::new("opendap_adapter", "https://cds.climate.copernicus.eu/api", "era5");

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }
}
