//! Geospatial data adapters for integrating external layers into the canvas.

pub mod astrometry_adapter;
pub mod dem_adapter;
pub mod gbif_adapter;
pub mod ivoa_tap_adapter;
pub mod ogc_3d_tiles;
pub mod opendap_adapter;
pub mod osm_adapter;
pub mod sparql_adapter;
pub mod stac_adapter;
pub mod wms_adapter;
pub mod ckan_adapter;

pub mod canvas_defaults;

use std::collections::HashMap;

use crate::net::disclosure::NetworkDisclosureRegistry;

pub use astrometry_adapter::AstrometryAdapter;
pub use dem_adapter::DemAdapter;
pub use gbif_adapter::GbifAdapter;
pub use ivoa_tap_adapter::IvoaTapAdapter;
pub use ogc_3d_tiles::Ogc3dTilesAdapter;
pub use opendap_adapter::OpendapAdapter;
pub use osm_adapter::OsmAdapter;
pub use sparql_adapter::SparqlAdapter;
pub use stac_adapter::StacAdapter;
pub use wms_adapter::WmsAdapter;
pub use ckan_adapter::CkanAdapter;

pub trait DataAdapter {
    /// Returns the unique identifier for this adapter (e.g., "dem_adapter")
    fn adapter_id(&self) -> &'static str;

    /// Initiate fetching for a spatial bounding box [x1, y1, x2, y2]
    /// and temporal range [t0, t1]. Implementations must check `NetworkDisclosureRegistry`
    /// before initiating actual network calls.
    fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String>;

    /// Primary egress endpoint for disclosure checks and fetch reports.
    fn primary_endpoint(&self) -> &str;

    /// Honest estimate of how many tile/API units would be requested (no fake payloads).
    fn estimate_tile_count(&self, bbox: (f64, f64, f64, f64)) -> u32;
}

impl DataAdapter for DemAdapter {
    fn adapter_id(&self) -> &'static str {
        "dem_adapter"
    }

    fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        DemAdapter::fetch_region(self, bbox, time_range, registry)
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, bbox: (f64, f64, f64, f64)) -> u32 {
        estimate_raster_tiles(bbox, 14)
    }
}

impl DataAdapter for OsmAdapter {
    fn adapter_id(&self) -> &'static str {
        "osm_adapter"
    }

    fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        OsmAdapter::fetch_region(self, bbox, time_range, registry)
    }

    fn primary_endpoint(&self) -> &str {
        &self.overpass_endpoint
    }

    fn estimate_tile_count(&self, bbox: (f64, f64, f64, f64)) -> u32 {
        // Overpass = 1 query + MVT tiles at z15
        1 + estimate_raster_tiles(bbox, 15)
    }
}

impl DataAdapter for WmsAdapter {
    fn adapter_id(&self) -> &'static str {
        "wms_adapter"
    }

    fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        WmsAdapter::fetch_region(self, bbox, time_range, registry)
    }

    fn primary_endpoint(&self) -> &str {
        &self.endpoint
    }

    fn estimate_tile_count(&self, bbox: (f64, f64, f64, f64)) -> u32 {
        // WMS GetMap: one request per 256px tile at typical viewport scale
        estimate_raster_tiles(bbox, 12)
    }
}

impl DataAdapter for GbifAdapter {
    fn adapter_id(&self) -> &'static str {
        "gbif_adapter"
    }

    fn fetch_region(
        &self,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
        registry: &NetworkDisclosureRegistry,
    ) -> Result<(), String> {
        GbifAdapter::fetch_region(self, bbox, time_range, registry)
    }

    fn primary_endpoint(&self) -> &str {
        &self.occurrence_endpoint
    }

    fn estimate_tile_count(&self, bbox: (f64, f64, f64, f64)) -> u32 {
        // GBIF paginates at 300 records/page; estimate pages from bbox area (deg²).
        let area = (bbox.2 - bbox.0).abs() * (bbox.3 - bbox.1).abs();
        let pages = (area * 10.0).ceil() as u32;
        pages.max(1)
    }
}

/// Status of a layer fetch plan — honest, no fabricated payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerFetchStatus {
    /// Egress consent not registered; fail-closed.
    ConsentDenied,
    /// Consent granted; fetch would proceed (stub — no network I/O performed).
    ReadyToFetch,
    /// Adapter id not found in registry.
    AdapterNotFound,
}

/// Structured report describing what a layer fetch *would* request.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerFetchReport {
    pub adapter_id: String,
    pub endpoint: String,
    pub bbox: (f64, f64, f64, f64),
    pub time_range: (u64, u64),
    pub estimated_tile_count: u32,
    pub status: LayerFetchStatus,
}

/// Holds registered geospatial adapters and the network disclosure registry.
pub struct AdapterRegistry {
    adapters: HashMap<String, Box<dyn DataAdapter>>,
    disclosure: NetworkDisclosureRegistry,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            disclosure: NetworkDisclosureRegistry::new(),
        }
    }

    pub fn disclosure_registry(&self) -> &NetworkDisclosureRegistry {
        &self.disclosure
    }

    pub fn disclosure_registry_mut(&mut self) -> &mut NetworkDisclosureRegistry {
        &mut self.disclosure
    }

    pub fn register_adapter(&mut self, adapter: Box<dyn DataAdapter>) {
        let id = adapter.adapter_id().to_string();
        self.adapters.insert(id, adapter);
    }

    /// Describe what would be fetched for a layer. Performs consent check; does not
    /// initiate network I/O or return fabricated data.
    pub fn fetch_layer(
        &self,
        adapter_id: &str,
        bbox: (f64, f64, f64, f64),
        time_range: (u64, u64),
    ) -> Result<LayerFetchReport, String> {
        let adapter = self.adapters.get(adapter_id).ok_or_else(|| {
            format!("Adapter '{}' not registered", adapter_id)
        })?;

        let endpoint = adapter.primary_endpoint().to_string();
        let tile_count = adapter.estimate_tile_count(bbox);

        let status = if self
            .disclosure
            .check_egress_consent(adapter_id, &endpoint)
        {
            LayerFetchStatus::ReadyToFetch
        } else {
            LayerFetchStatus::ConsentDenied
        };

        Ok(LayerFetchReport {
            adapter_id: adapter_id.to_string(),
            endpoint,
            bbox,
            time_range,
            estimated_tile_count: tile_count,
            status,
        })
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        let mut registry = Self::new();
        canvas_defaults::register_canvas_defaults(&mut registry);
        registry
    }
}

/// Rough Web-Mercator tile count estimate from a lon/lat bbox at a given zoom.
fn estimate_raster_tiles(bbox: (f64, f64, f64, f64), zoom: u8) -> u32 {
    let (x1, y1, x2, y2) = bbox;
    let n = 1u32 << zoom;
    let tx1 = ((x1 + 180.0) / 360.0 * n as f64).floor() as u32;
    let tx2 = ((x2 + 180.0) / 360.0 * n as f64).ceil() as u32;
    let ty1 = ((1.0 - (y2.to_radians().tan() + 1.0 / y2.to_radians().cos()).ln() / std::f64::consts::PI)
        / 2.0
        * n as f64)
        .floor() as u32;
    let ty2 = ((1.0 - (y1.to_radians().tan() + 1.0 / y1.to_radians().cos()).ln() / std::f64::consts::PI)
        / 2.0
        * n as f64)
        .ceil() as u32;
    let tiles_x = tx2.saturating_sub(tx1).max(1);
    let tiles_y = ty2.saturating_sub(ty1).max(1);
    tiles_x.saturating_mul(tiles_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_registry_consent_denied() {
        let mut registry = AdapterRegistry::new();
        registry.register_adapter(Box::new(DemAdapter::new("dem_adapter", "https://elevation.example.com")));

        let report = registry
            .fetch_layer("dem_adapter", (0.0, 0.0, 1.0, 1.0), (0, 0))
            .expect("report");

        assert_eq!(report.status, LayerFetchStatus::ConsentDenied);
        assert_eq!(report.endpoint, "https://elevation.example.com");
        assert!(report.estimated_tile_count >= 1);
    }

    #[test]
    fn test_adapter_registry_consent_granted() {
        let mut registry = AdapterRegistry::new();
        registry.register_adapter(Box::new(OsmAdapter::new(
            "osm_adapter",
            "https://overpass-api.de/api/interpreter",
            "https://tiles.example.com/osm",
        )));
        registry.disclosure_registry_mut().register_egress(
            "osm_adapter",
            "https://overpass-api.de/api/interpreter",
            "Fetch OSM features",
            "User pans map",
        );

        let report = registry
            .fetch_layer("osm_adapter", (-0.1, 51.4, 0.1, 51.6), (0, 0))
            .expect("report");

        assert_eq!(report.status, LayerFetchStatus::ReadyToFetch);
        assert!(report.estimated_tile_count >= 1);
    }

    #[test]
    fn test_adapter_registry_unknown_adapter() {
        let registry = AdapterRegistry::new();
        let err = registry
            .fetch_layer("unknown_adapter", (0.0, 0.0, 1.0, 1.0), (0, 0))
            .unwrap_err();
        assert!(err.contains("not registered"));
    }
}