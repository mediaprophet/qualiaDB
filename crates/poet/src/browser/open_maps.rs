//! Open map basemap helpers for GIS containers (OSM and peers).
//!
//! The Earth map container defaults to a public OpenStreetMap embed, not a
//! placeholder SVG and not daemon GIS tiles. Cosmos / fiction realms stay
//! authored SVG chrome.

/// Official OSM export embed (allowed to be framed; includes OSM chrome + attribution).
pub const OSM_EMBED_ORIGIN: &str = "https://www.openstreetmap.org/export/embed.html";
/// OSM copyright / attribution landing page (required by the tile/embed licence).
pub const OSM_COPYRIGHT_URL: &str = "https://www.openstreetmap.org/copyright";
/// Larger-map link origin (same site as the embed).
pub const OSM_SITE_ORIGIN: &str = "https://www.openstreetmap.org";
/// Default OSM embed layer (`mapnik` = standard Mapnik / open-map view).
pub const OSM_DEFAULT_LAYER: &str = "mapnik";
/// Half-width of the default bbox in WGS84 degrees (~neighbourhood / catchment).
pub const OSM_DEFAULT_HALF_SPAN_DEG: f64 = 0.05;

/// Normalise a layer chip id onto OSM's `layer=` query values.
pub fn normalize_osm_layer(layer: &str) -> &'static str {
    match layer.trim().to_ascii_lowercase().as_str() {
        "cyclemap" | "cycle" | "cycling" => "cyclemap",
        "transportmap" | "transport" | "transit" => "transportmap",
        "hot" | "humanitarian" => "hot",
        _ => OSM_DEFAULT_LAYER,
    }
}

/// OSM export bbox: `(min_lon, min_lat, max_lon, max_lat)`.
pub fn osm_bbox_around(lat: f64, lon: f64, half_span_deg: f64) -> (f64, f64, f64, f64) {
    let span = half_span_deg.abs().max(0.001);
    (lon - span, lat - span, lon + span, lat + span)
}

/// OpenStreetMap export-embed URL centred on `lat`/`lon` with a marker.
pub fn osm_embed_url(lat: f64, lon: f64, layer: &str, half_span_deg: f64) -> String {
    let layer = normalize_osm_layer(layer);
    let (west, south, east, north) = osm_bbox_around(lat, lon, half_span_deg);
    format!(
        "{OSM_EMBED_ORIGIN}?bbox={west},{south},{east},{north}&layer={layer}&marker={lat},{lon}"
    )
}

/// Same-site “view larger map” URL (OSM’s recommended companion to the embed).
pub fn osm_site_url(lat: f64, lon: f64, zoom: u8) -> String {
    format!("{OSM_SITE_ORIGIN}/?mlat={lat}&mlon={lon}#map={zoom}/{lat}/{lon}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layer_is_mapnik_open_map() {
        assert_eq!(normalize_osm_layer(""), OSM_DEFAULT_LAYER);
        assert_eq!(normalize_osm_layer("OSM"), "mapnik");
        assert_eq!(normalize_osm_layer("cycle"), "cyclemap");
        assert_eq!(normalize_osm_layer("HOT"), "hot");
        assert_eq!(normalize_osm_layer("transit"), "transportmap");
    }

    #[test]
    fn embed_url_is_official_osm_export_centered_on_marker() {
        let lat = -37.8;
        let lon = 144.9;
        let url = osm_embed_url(lat, lon, OSM_DEFAULT_LAYER, OSM_DEFAULT_HALF_SPAN_DEG);
        assert!(url.starts_with(OSM_EMBED_ORIGIN));
        assert!(url.contains("layer=mapnik"));
        assert!(url.contains(&format!("marker={lat},{lon}")));
        assert!(url.contains("bbox="));
        let (west, south, east, north) = osm_bbox_around(lat, lon, OSM_DEFAULT_HALF_SPAN_DEG);
        assert!(west < lon && east > lon && south < lat && north > lat);
        assert!(url.contains(&format!("{west},{south},{east},{north}")));
    }

    #[test]
    fn site_url_points_at_openstreetmap_with_marker() {
        let url = osm_site_url(-37.8, 144.9, 13);
        assert!(url.starts_with(OSM_SITE_ORIGIN));
        assert!(url.contains("mlat=-37.8"));
        assert!(url.contains("mlon=144.9"));
        assert!(url.contains("#map=13/-37.8/144.9"));
    }
}
