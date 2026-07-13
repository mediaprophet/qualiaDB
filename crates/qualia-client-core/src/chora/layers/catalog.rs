use serde::{Deserialize, Serialize};

pub type LayerId = &'static str;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LayerSource {
    NasaGibs { layer: &'static str, projection: &'static str },
    NasaHorizons { body: &'static str },
    HipparcosCatalog,
    YaleBrightStars,
    DemWcs { endpoint: &'static str, coverage: &'static str },
    OsmOverpass,
    WmsImagery { endpoint: &'static str, layer: &'static str },
    UsgsAstrogeology { body: &'static str, layer: &'static str },
    StacCatalog { catalog: &'static str, collection: &'static str },
    Generated { generator: &'static str },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LayerCategory {
    EarthImagery,
    EarthTerrain,
    Stars,
    SolarSystem,
    Planetary,
    Osm,
    Biodiversity,
    Weather,
    Historical,
    Social,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LayerDefinition {
    pub id: LayerId,
    pub name: &'static str,
    pub category: LayerCategory,
    pub source: LayerSource,
    pub description: &'static str,
    pub license: &'static str,
    pub spatial_coverage: Option<((f64, f64), (f64, f64))>,
    pub temporal_range: Option<(u64, u64)>,
    pub default_resolution: u32,
    pub max_resolution: u32,
    pub preview_color: [f32; 3],
}

pub static LAYER_CATALOG: &[LayerDefinition] = &[
    LayerDefinition {
        id: "earth-modis-truecolor",
        name: "Earth — MODIS True Color (NASA GIBS)",
        category: LayerCategory::EarthImagery,
        source: LayerSource::NasaGibs {
            layer: "MODIS_Terra_CorrectedReflectance_TrueColor",
            projection: "epsg4326",
        },
        description: "NASA MODIS Terra corrected reflectance true color imagery of Earth. Updated daily.",
        license: "Public domain (NASA)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: Some((1_000_000_000, 2_000_000_000)),
        default_resolution: 1024,
        max_resolution: 4096,
        preview_color: [0.2, 0.4, 0.8],
    },
    LayerDefinition {
        id: "earth-blue-marble",
        name: "Earth — Blue Marble (NASA GIBS)",
        category: LayerCategory::EarthImagery,
        source: LayerSource::NasaGibs {
            layer: "BlueMarble_NextGeneration",
            projection: "epsg4326",
        },
        description: "NASA Blue Marble Next Generation — the classic Earth image.",
        license: "Public domain (NASA)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: None,
        default_resolution: 2048,
        max_resolution: 8192,
        preview_color: [0.2, 0.5, 0.9],
    },
    LayerDefinition {
        id: "earth-night-lights",
        name: "Earth — Night Lights (NASA GIBS)",
        category: LayerCategory::EarthImagery,
        source: LayerSource::NasaGibs {
            layer: "VIIRS_SNPP_DayNightBand_AT_Sensor_Radiance",
            projection: "epsg4326",
        },
        description: "Earth at night — city lights visible from the NASA SNPP VIIRS day/night band.",
        license: "Public domain (NASA)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: Some((1_000_000_000, 2_000_000_000)),
        default_resolution: 1024,
        max_resolution: 4096,
        preview_color: [0.9, 0.9, 0.3],
    },
    LayerDefinition {
        id: "earth-aster-dem",
        name: "Earth — ASTER DEM Terrain",
        category: LayerCategory::EarthTerrain,
        source: LayerSource::DemWcs {
            endpoint: "https://lpdaacsvc.cr.usgs.gov/aster",
            coverage: "ASTGTM",
        },
        description: "ASTER Global Digital Elevation Model — terrain heightfield for Earth.",
        license: "Public domain (NASA/USGS)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: None,
        default_resolution: 256,
        max_resolution: 1024,
        preview_color: [0.5, 0.35, 0.2],
    },
    LayerDefinition {
        id: "stars-hipparcos",
        name: "Stars — Hipparcos Catalog (118k stars)",
        category: LayerCategory::Stars,
        source: LayerSource::HipparcosCatalog,
        description: "Hipparcos satellite star catalog — 118,218 stars with positions, magnitudes, and color indices.",
        license: "Public domain (ESA)",
        spatial_coverage: None,
        temporal_range: Some((1_000_000_000, 2_000_000_000)),
        default_resolution: 118_218,
        max_resolution: 118_218,
        preview_color: [1.0, 1.0, 0.9],
    },
    LayerDefinition {
        id: "stars-bright",
        name: "Stars — Yale Bright Star Catalog (9k stars)",
        category: LayerCategory::Stars,
        source: LayerSource::YaleBrightStars,
        description: "Yale Bright Star Catalog — 9,110 stars visible to the naked eye, with magnitudes and colors.",
        license: "Public domain",
        spatial_coverage: None,
        temporal_range: None,
        default_resolution: 9_110,
        max_resolution: 9_110,
        preview_color: [1.0, 0.95, 0.8],
    },
    LayerDefinition {
        id: "mars-mola-dem",
        name: "Mars — MOLA DEM Terrain",
        category: LayerCategory::Planetary,
        source: LayerSource::WmsImagery {
            endpoint: "https://planetarymaps.usgs.gov/cgi-bin/mapserv",
            layer: "mars_mola",
        },
        description: "Mars Orbiter Laser Altimeter (MOLA) digital elevation model.",
        license: "Public domain (NASA/USGS)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: None,
        default_resolution: 512,
        max_resolution: 2048,
        preview_color: [0.7, 0.4, 0.2],
    },
    LayerDefinition {
        id: "moon-lro-wac",
        name: "Moon — LRO WAC Imagery",
        category: LayerCategory::Planetary,
        source: LayerSource::WmsImagery {
            endpoint: "https://planetarymaps.usgs.gov/cgi-bin/mapserv",
            layer: "moon_lro_wac",
        },
        description: "Lunar Reconnaissance Orbiter Wide Angle Camera imagery of the Moon.",
        license: "Public domain (NASA/USGS)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: None,
        default_resolution: 1024,
        max_resolution: 4096,
        preview_color: [0.6, 0.6, 0.6],
    },
    LayerDefinition {
        id: "earth-sentinel-2",
        name: "Earth — Sentinel-2 L2A (Microsoft Planetary Computer)",
        category: LayerCategory::EarthImagery,
        source: LayerSource::StacCatalog {
            catalog: "https://planetarycomputer.microsoft.com/api/stac/v1",
            collection: "sentinel-2-l2a",
        },
        description: "Sentinel-2 Level-2A surface reflectance imagery via Microsoft Planetary Computer STAC.",
        license: "CC-BY 4.0 (Copernicus)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: Some((1_400_000_000, 2_000_000_000)),
        default_resolution: 1024,
        max_resolution: 4096,
        preview_color: [0.3, 0.6, 0.3],
    },
    LayerDefinition {
        id: "earth-osm-buildings",
        name: "Earth — OSM Buildings (Overpass)",
        category: LayerCategory::Osm,
        source: LayerSource::OsmOverpass,
        description: "OpenStreetMap building footprints extruded into 3D via the Overpass API.",
        license: "ODbL (OpenStreetMap)",
        spatial_coverage: None,
        temporal_range: None,
        default_resolution: 256,
        max_resolution: 2048,
        preview_color: [0.8, 0.6, 0.4],
    },
    LayerDefinition {
        id: "earth-gebco-bathymetry",
        name: "Earth — GEBCO Ocean Bathymetry",
        category: LayerCategory::EarthTerrain,
        source: LayerSource::WmsImagery {
            endpoint: "https://www.gebco.net/data_and_products/gebco_web_services/web_map_service",
            layer: "GEBCO_LATEST",
        },
        description: "GEBCO global ocean bathymetry — underwater terrain.",
        license: "Free (GEBCO)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: None,
        default_resolution: 1024,
        max_resolution: 4096,
        preview_color: [0.1, 0.2, 0.5],
    },
    LayerDefinition {
        id: "earth-temperature",
        name: "Earth — Land Surface Temperature (NASA GIBS)",
        category: LayerCategory::Weather,
        source: LayerSource::NasaGibs {
            layer: "MODIS_Terra_Land_Surface_Temp_Day",
            projection: "epsg4326",
        },
        description: "MODIS Terra land surface temperature — daytime readings mapped to color.",
        license: "Public domain (NASA)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: Some((1_000_000_000, 2_000_000_000)),
        default_resolution: 1024,
        max_resolution: 4096,
        preview_color: [0.9, 0.3, 0.1],
    },
    LayerDefinition {
        id: "earth-fire-active",
        name: "Earth — Active Fires (NASA GIBS FIRMS)",
        category: LayerCategory::EarthImagery,
        source: LayerSource::NasaGibs {
            layer: "FIRMS_MODIS_Terra_Thermal_Anomalies_All",
            projection: "epsg4326",
        },
        description: "Active fire detections from MODIS Terra — thermal anomalies shown as bright points.",
        license: "Public domain (NASA)",
        spatial_coverage: Some(((-90.0, -180.0), (90.0, 180.0))),
        temporal_range: Some((1_000_000_000, 2_000_000_000)),
        default_resolution: 1024,
        max_resolution: 4096,
        preview_color: [1.0, 0.2, 0.0],
    },
];

pub fn layers_by_category(category: &LayerCategory) -> Vec<&'static LayerDefinition> {
    LAYER_CATALOG.iter().filter(|l| std::mem::discriminant(&l.category) == std::mem::discriminant(category)).collect()
}

pub fn all_categories() -> Vec<LayerCategory> {
    use LayerCategory::*;
    vec![
        EarthImagery,
        EarthTerrain,
        Stars,
        SolarSystem,
        Planetary,
        Osm,
        Biodiversity,
        Weather,
        Historical,
        Social,
    ]
}

pub fn find_layer(id: &str) -> Option<&'static LayerDefinition> {
    LAYER_CATALOG.iter().find(|l| l.id == id)
}
