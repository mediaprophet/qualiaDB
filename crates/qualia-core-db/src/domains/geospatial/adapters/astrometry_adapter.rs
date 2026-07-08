use crate::domains::geospatial::adapters::DataAdapter;
use crate::net::disclosure::NetworkDisclosureRegistry;

/// Adapter for Trajectory & Ephemeris REST APIs (e.g., NASA JPL Horizons, NeoWs, MPC).
pub struct AstrometryAdapter {
    pub id: &'static str,
    pub endpoint: String,
}

impl AstrometryAdapter {
    pub fn new(id: &'static str, endpoint: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
        }
    }
}

impl DataAdapter for AstrometryAdapter {
    fn adapter_id(&self) -> &'static str {
        self.id
    }

    fn fetch_region(
        &self,
        _bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        if !registry.check_egress_consent(self.adapter_id(), &self.endpoint) {
            return Err(format!(
                "Consent denied or unregistered for astrometry endpoint {}",
                self.endpoint
            ));
        }

        let url = format!(
            "{}?format=json&START_TIME={}&STOP_TIME={}",
            self.endpoint, time_range.0, time_range.1
        );

        let client = reqwest::blocking::Client::new();
        let resp = client.get(&url).send().map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Astrometry API returned error: {}", resp.status()));
        }

        // Parsing into NQuins is deferred.
        Ok(())
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_astrometry_adapter_consent_denied() {
        let registry = NetworkDisclosureRegistry::new();
        let adapter = AstrometryAdapter::new("astrometry_adapter", "https://ssd.jpl.nasa.gov/api/horizons.api");

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }
}
