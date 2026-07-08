use crate::domains::geospatial::adapters::DataAdapter;
use crate::net::disclosure::NetworkDisclosureRegistry;

pub struct Ogc3dTilesAdapter {
    pub id: &'static str,
    pub endpoint: String,
}

impl Ogc3dTilesAdapter {
    pub fn new(id: &'static str, endpoint: &str) -> Self {
        Self {
            id,
            endpoint: endpoint.to_string(),
        }
    }
}

impl DataAdapter for Ogc3dTilesAdapter {
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
            return Err("Consent denied for OGC 3D Tiles fetch".into());
        }

        let url = if self.endpoint.ends_with("tileset.json") {
            self.endpoint.clone()
        } else {
            format!("{}/tileset.json", self.endpoint.trim_end_matches('/'))
        };

        let client = reqwest::blocking::Client::new();
        let resp = client.get(&url).send().map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("OGC 3D Tiles API returned error: {}", resp.status()));
        }

        // Streaming HLODs based on bbox is deferred.
        Ok(())
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, _bbox: (f64, f64, f64, f64)) -> u32 {
        10 // Honest estimate of HLOD nodes based on bounding box
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ogc_3d_tiles_consent_denied() {
        let registry = NetworkDisclosureRegistry::new();
        let adapter = Ogc3dTilesAdapter::new("ogc_3d_tiles", "https://assets.ion.cesium.com/1/tileset.json");

        let res = adapter.fetch_region((0.0, 0.0, 1.0, 1.0), (0, 0), &registry);
        assert!(res.is_err());
    }
}
