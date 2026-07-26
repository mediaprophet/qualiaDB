use crate::domains::geospatial::adapters::wms_adapter::OgcServiceType;
use crate::domains::geospatial::adapters::{
    AdapterRegistry, AstrometryAdapter, CkanAdapter, IvoaTapAdapter, OsmAdapter, SparqlAdapter,
    StacAdapter, WmsAdapter,
};

/// Registers all default geospatial, astronomical, and temporal endpoints into the canvas registry.
pub fn register_canvas_defaults(registry: &mut AdapterRegistry) {
    // --------------------------------------------------------------------------------
    // 1. Astronomy & Space (IVOA TAP, SPARQL, Ephemeris)
    // --------------------------------------------------------------------------------
    registry.register_adapter(Box::new(IvoaTapAdapter::new(
        "ivoa_tap_gaia",
        "https://gea.esac.esa.int/tap-server/tap/sync",
        "gaiadr3.gaia_source",
    )));
    registry.register_adapter(Box::new(IvoaTapAdapter::new(
        "ivoa_tap_irsa",
        "https://irsa.ipac.caltech.edu/TAP/sync",
        "ipac",
    )));
    registry.register_adapter(Box::new(IvoaTapAdapter::new(
        "ivoa_tap_vizier",
        "http://tapvizier.cds.unistra.fr/TAPVizieR/tap/sync",
        "vizier",
    )));
    registry.register_adapter(Box::new(IvoaTapAdapter::new(
        "ivoa_tap_simbad",
        "http://simbad.cds.unistra.fr/simbad/sim-tap/sync",
        "simbad",
    )));
    registry.register_adapter(Box::new(IvoaTapAdapter::new(
        "ivoa_tap_chandra",
        "https://cda.cfa.harvard.edu/cxctap/sync",
        "chandra",
    )));
    registry.register_adapter(Box::new(IvoaTapAdapter::new(
        "ivoa_tap_csa",
        "https://csa.esac.esa.int/csa/tap/sync",
        "csa",
    )));

    registry.register_adapter(Box::new(SparqlAdapter::new(
        "sparql_wikidata",
        "https://query.wikidata.org/sparql",
    )));
    registry.register_adapter(Box::new(SparqlAdapter::new(
        "sparql_eu_open_data",
        "https://data.europa.eu/data/sparql",
    )));

    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "astrometry_jpl_horizons",
        "https://ssd.jpl.nasa.gov/api/horizons.api",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "astrometry_neows",
        "https://api.nasa.gov/neo/rest/v1/",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "astrometry_sdss_casjobs",
        "https://skyserver.sdss.org/casjobs/services/api/",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "astrometry_mpc",
        "https://minorplanetcenter.net/data",
    )));

    // --------------------------------------------------------------------------------
    // 2. Earth Observation, Terrain, and Imagery (NASA GIBS, USGS, STAC)
    // --------------------------------------------------------------------------------
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wmts_nasa_gibs_epsg4326",
        "https://gibs.earthdata.nasa.gov/wmts/epsg4326/best/wmts.cgi",
        OgcServiceType::Wmts,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wmts_nasa_gibs_epsg3857",
        "https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/wmts.cgi",
        OgcServiceType::Wmts,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wmts_nasa_gibs_epsg3413",
        "https://gibs.earthdata.nasa.gov/wmts/epsg3413/best/wmts.cgi",
        OgcServiceType::Wmts,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wmts_nasa_gibs_epsg3031",
        "https://gibs.earthdata.nasa.gov/wmts/epsg3031/best/wmts.cgi",
        OgcServiceType::Wmts,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_nasa_gibs",
        "https://gibs.earthdata.nasa.gov/wms/epsg4326/best/wms.cgi",
        OgcServiceType::Wms,
    )));

    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_usgs_topo",
        "https://basemap.nationalmap.gov/arcgis/services/USGSTopo/MapServer/WMSServer",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_usgs_imagery",
        "https://imagery.nationalmap.gov/arcgis/services/USGSImageryOnly/MapServer/WMSServer",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_usgs_3dep",
        "https://elevation.nationalmap.gov/arcgis/services/3DEPElevation/ImageServer/WMSServer",
        OgcServiceType::Wms,
    )));

    registry.register_adapter(Box::new(StacAdapter::new(
        "stac_usgs_landsatlook",
        "https://landsatlook.usgs.gov/stac-server",
        None,
    )));
    registry.register_adapter(Box::new(StacAdapter::new(
        "stac_ms_planetary_computer",
        "https://planetarycomputer.microsoft.com/api/stac/v1",
        None,
    )));
    registry.register_adapter(Box::new(StacAdapter::new(
        "stac_aws_earth_search",
        "https://earth-search.aws.element84.com/v1",
        None,
    )));

    // --------------------------------------------------------------------------------
    // 3. Marine, Bathymetry & Oceanography (GEBCO, Copernicus, EMODnet, NOAA)
    // --------------------------------------------------------------------------------
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_gebco_global",
        "https://wms.gebco.net/mapserv",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_gebco_north_polar",
        "https://wms.gebco.net/latest/north-polar/mapserv",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_gebco_south_polar",
        "https://wms.gebco.net/latest/south-polar/mapserv",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wmts_copernicus_marine",
        "https://wmts.marine.copernicus.eu/teroWmts",
        OgcServiceType::Wmts,
    )));

    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_emodnet_seabed",
        "https://ows.emodnet-seabedhabitats.eu/geoserver/emodnet_view/wms",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_emodnet_bathymetry",
        "https://ows.emodnet-bathymetry.eu/wms",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_emodnet_biology",
        "https://ows.emodnet-biology.eu/geoserver/wms",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_emodnet_physics",
        "https://geoserver.emodnet-physics.eu/geoserver/wms",
        OgcServiceType::Wms,
    )));

    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_marine_regions_gazetteer",
        "https://marineregions.org/rest/",
    ))); // Using generic REST adapter
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_noaa_coastal_dem",
        "https://gis.ngdc.noaa.gov/arcgis/services/DEM_mosaics/DEM_all/ImageServer/WMSServer",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_noaa_trackline",
        "https://gis.ngdc.noaa.gov/arcgis/services/trackline_geophysics/MapServer/WMSServer",
        OgcServiceType::Wms,
    )));

    // --------------------------------------------------------------------------------
    // 4. Extraterrestrial Planetary Mapping (USGS Astrogeology)
    // --------------------------------------------------------------------------------
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_astro_mars",
        "https://planetarymaps.usgs.gov/cgi-bin/mapserv?map=/maps/mars/mars_simp_cyl.map",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_astro_moon",
        "https://planetarymaps.usgs.gov/cgi-bin/mapserv?map=/maps/earth/moon_simp_cyl.map",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_astro_venus",
        "https://planetarymaps.usgs.gov/cgi-bin/mapserv?map=/maps/venus/venus_simp_cyl.map",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_astro_mercury",
        "https://planetarymaps.usgs.gov/cgi-bin/mapserv?map=/maps/mercury/mercury_simp_cyl.map",
        OgcServiceType::Wms,
    )));

    // --------------------------------------------------------------------------------
    // 5. Geoscience, Ecological & Socioeconomic (GA, EEA, GFW, SEDAC, UN)
    // --------------------------------------------------------------------------------
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_ga_geophysical",
        "https://services.ga.gov.au/gis/geophysical-grids/ows",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_ga_rock_properties",
        "http://www.ga.gov.au/geophysics-rockpropertypub-gws/ga_rock_properties_wms/ows",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_ga_earthquakes",
        "https://earthquakes.ga.gov.au/geoserver/wms",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_eea_corine",
        "https://discomap.eea.europa.eu/arcgis/services/Corine/CLC2018_WM/MapServer/WMSServer",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_opentopo",
        "https://opentopo.sdsc.edu/geoserver/wms",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_sedac_gpw",
        "https://sedac.ciesin.columbia.edu/geoserver/wms",
        OgcServiceType::Wms,
    )));

    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_gfw_api",
        "https://data-api.globalforestwatch.org/",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_opensensemap",
        "https://api.opensensemap.org/boxes",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_un_biodiversity",
        "https://unbiodiversitylab.org/api/v1/",
    )));

    // --------------------------------------------------------------------------------
    // 6. Weather, Air Traffic, and Mobility (Open-Meteo, OpenSky, OSRM, Transitland)
    // --------------------------------------------------------------------------------
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_open_meteo_forecast",
        "https://api.open-meteo.com/v1/forecast",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_open_meteo_archive",
        "https://archive-api.open-meteo.com/v1/archive",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_open_meteo_marine",
        "https://marine-api.open-meteo.com/v1/marine",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_nws_api",
        "https://api.weather.gov/",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_opensky_all",
        "https://opensky-network.org/api/states/all",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_aviationstack",
        "http://api.aviationstack.com/v1/flights",
    )));

    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_barentswatch_ais",
        "https://historic.ais.barentswatch.no/v1/",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_aishub",
        "http://data.aishub.net/ws.php",
    )));

    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_tomtom_incident",
        "https://api.tomtom.com/traffic/services/5/incidentDetails",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_tomtom_flow",
        "https://api.tomtom.com/traffic/services/4/flowSegmentData",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_osrm_routing",
        "https://router.project-osrm.org/route/v1/driving/",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_transitland_v2",
        "https://transit.land/api/v2/rest/",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_mobilitydata",
        "https://database.mobilitydata.org/api/",
    )));

    // --------------------------------------------------------------------------------
    // 7. OSM & General Web Map Tile Services
    // --------------------------------------------------------------------------------
    registry.register_adapter(Box::new(OsmAdapter::new(
        "osm_default",
        "https://overpass-api.de/api/interpreter",
        "https://tile.openstreetmap.org",
    )));

    // --------------------------------------------------------------------------------
    // 8. Australia - ABS ASGS, Geoscape, GA, and State Portals
    // --------------------------------------------------------------------------------
    // ABS ASGS REST APIs
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_lga_2024",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2024/LGA/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_lga_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/LGA/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sal_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SAL/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_mb_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/MB/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_ced_2024",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2024/CED/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sed_2024",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2024/SED/MapServer",
    )));

    // Additional ASGS Statistical & Indigenous Boundaries
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sa1_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SA1/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sa2_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SA2/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sa3_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SA3/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sa4_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SA4/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_gccsa_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/GCCSA/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_ste_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/STE/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_aus_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/AUS/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_iloc_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/ILOC/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_iare_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/IARE/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_ireg_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/IREG/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_poa_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/POA/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_add_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/ADD/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_dzn_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/DZN/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_ra_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/RA/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sos_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SOS/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sosr_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SOSR/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_sua_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/SUA/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_tr_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/TR/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_ucl_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/UCL/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_nrmr_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/NRMR/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_lhn_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/LHN/MapServer",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_abs_phn_2021",
        "https://geo.abs.gov.au/arcgis/rest/services/ASGS2021/PHN/MapServer",
    )));

    // Geoscape (CKAN Bulk Data)
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_geoscape_admin_bounds",
        "https://data.gov.au/data/api/3/action/package_show?id=geoscape-administrative-boundaries",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new("rest_geoscape_gnaf", "https://data.gov.au/data/api/3/action/package_show?id=geocoded-national-address-file-g-naf")));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_aec_federal_electorates",
        "https://data.gov.au/data/api/3/action/package_show?id=national-electoral-boundaries",
    )));

    // Geoscience Australia (WMS/WFS)
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wms_ga_aus_land_borders",
        "https://services.ga.gov.au/gis/services/AustraliasLandBorders/MapServer/WMSServer",
        OgcServiceType::Wms,
    )));
    registry.register_adapter(Box::new(WmsAdapter::new("wms_ga_lga_council_offices", "http://services.ga.gov.au/gis/services/Local_Government_Area_Council_Offices/MapServer/WMSServer", OgcServiceType::Wms)));
    registry.register_adapter(Box::new(WmsAdapter::new(
        "wfs_ga_nexis_building_exposure",
        "https://services.ga.gov.au/gis/services/NEXIS_Building_Exposure/MapServer/WFSServer",
        OgcServiceType::Wfs,
    )));

    // State-Specific Portals
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_nsw_spatial",
        "https://portal.spatial.nsw.gov.au/client/services",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_vicmap",
        "https://services6.arcgis.com/GB33F62SbDxJjwEL/arcgis/rest/services",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_qspatial",
        "https://spatial-gis.information.qld.gov.au/arcgis/rest/services",
    )));
    registry.register_adapter(Box::new(AstrometryAdapter::new(
        "rest_sa_location",
        "https://lsa4.geohub.sa.gov.au/server/rest/services",
    )));

    // --------------------------------------------------------------------------------
    // 9. Global Federated CKAN Portals
    // --------------------------------------------------------------------------------
    // National Portals
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_usa",
        "https://catalog.data.gov/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_uk",
        "https://data.gov.uk/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_aus",
        "https://data.gov.au/data/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_can",
        "https://open.canada.ca/data/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_brazil",
        "https://dados.gov.br/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_singapore",
        "https://data.gov.sg/api/3",
    ))); // Note: data.gov.sg moved away from vanilla CKAN recently, but often retains compatibility.
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_ireland",
        "https://data.gov.ie/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_germany",
        "https://www.govdata.de/ckan/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_croatia",
        "https://data.gov.hr/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_romania",
        "https://data.gov.ro/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_finland",
        "https://www.avoindata.fi/data/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_denmark",
        "https://www.opendata.dk/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_austria",
        "https://www.data.gv.at/katalog/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_sweden",
        "https://admin.dataportal.se/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_mexico",
        "https://datos.gob.mx/busca/api/3",
    )));

    // Regional/State Portals
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_qld",
        "https://www.data.qld.gov.au/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_sa",
        "https://data.sa.gov.au/data/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_nsw",
        "https://data.nsw.gov.au/data/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_california",
        "https://data.ca.gov/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_northern_ireland",
        "https://opendatani.gov.uk/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_aragon",
        "https://opendata.aragon.es/datos/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_asturias",
        "https://datosabiertos.asturias.es/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_helsinki",
        "https://hri.fi/data/api/3",
    )));

    // Municipal Portals
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_boston",
        "https://data.boston.gov/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_amsterdam",
        "https://data.amsterdam.nl/data/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_berlin",
        "https://daten.berlin.de/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_cape_town",
        "https://odp.capetown.gov.za/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_malmo",
        "https://oppnadata.malmo.se/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_brisbane",
        "https://www.data.brisbane.qld.gov.au/data/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_bari",
        "https://opendata.comune.bari.it/api/3",
    )));

    // Supranational
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_eu",
        "https://data.europa.eu/api/hub/search/",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_un_hdx",
        "https://data.humdata.org/api/3",
    )));
    registry.register_adapter(Box::new(CkanAdapter::new(
        "ckan_openafrica",
        "https://openafrica.net/api/3",
    )));
}
