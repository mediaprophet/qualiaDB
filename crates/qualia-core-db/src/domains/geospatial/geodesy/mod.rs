//! Planetary Geodesy, CRS Projections, AusGeoid, A5 DGGS, Packed Hilbert R-Tree & IAU Ephemeris (`GEO-S1`).
//!
//! Tier-1 `HotZeroHeap` geospatial foundation combining:
//! - [`wgs84_karney`]: WGS84 ellipsoid constants, Bowring closed-form ECEF<->geodetic, Karney/Vincenty geodesics, ENU basis, and authalic polygon area.
//! - [`coordinates_mgrs`]: Zero-heap (`StackText64`) parsing and formatting for DD, DMS, UTM, UPS, MGRS, Geohash, and Open Location Code (`PlusCode`).
//! - [`crs_proj_wkt`]: Zero-heap WKT1/WKT2/ESRI CRS slice parser, Transverse Mercator (GDA2020 MGA 49-56), Australian Albers (`EPSG:9473`), Web Mercator (`EPSG:3857`), Helmert 7-parameter datum shifts, and `AusGeoid2020`/`EGM2008` undulation.
//! - [`dggs_rtree_hot`]: Pentagonal equal-area `A5` DGGS hierarchical indexing and zero-heap packed Hilbert R-Tree (`RTreeEntry`) spatial predicates.
//! - [`ephemeris_orbit`]: IAU 2006/2000A analytical solar/lunar ephemeris, sloped topographic solar irradiance (`W/m²` and daily `Wh/m²`), and Keplerian + $J_2$ secular orbital propagation.

pub mod coordinates_mgrs;
pub mod crs_proj_wkt;
pub mod dggs_rtree_hot;
pub mod ephemeris_orbit;
pub mod wgs84_karney;

pub use coordinates_mgrs::{
    decode_geohash_bytes, encode_geohash_stack, encode_plus_code_stack, format_dms_stack,
    format_mgrs_stack, latlon_to_mgrs, latlon_to_utm, mgrs_to_latlon, parse_coordinate_str,
    parse_mgrs_bytes, utm_to_latlon, CoordParseError, MgrsCoord, StackText64, UtmUpsCoord,
};
pub use crs_proj_wkt::{
    ausgeoid2020_undulation_m, egm2008_undulation_m, project_forward, project_inverse,
    reproject_coord, CrsDefinition, CrsError, Ellipsoid, EpsgCode, Helmert7Param, ProjectionMethod,
    EPSG_GDA2020, EPSG_GDA2020_ALBERS, EPSG_GDA2020_MGA54, EPSG_GDA2020_MGA55, EPSG_GDA2020_MGA56,
    EPSG_GDA94, EPSG_WEB_MERCATOR, EPSG_WGS84,
};
pub use dggs_rtree_hot::{
    a5_cell_area_m2, a5_children, a5_parent, a5_pentagon_boundary, a5_to_lonlat,
    build_packed_hilbert_index, lonlat_to_a5, point_in_polygon_hot, query_bbox_hot,
    query_k_nearest_hot, segments_intersect_hot, A5Cell, BBox2D, DggsError, RTreeEntry,
};
pub use ephemeris_orbit::{
    daily_integrated_insolation_wh_m2, earth_rotation_angle_rad, lunar_ephemeris,
    propagate_orbit_j2, solar_ephemeris, topographic_solar_irradiance, unix_to_julian_date,
    InsolationResult, KeplerElements, LunarPosition, OrbitalStateEcef, SolarPosition,
};
pub use wgs84_karney::{
    ecef_to_enu, ecef_to_geodetic_bowring, enu_basis, enu_to_ecef, geodetic_to_ecef,
    geodesic_direct, geodesic_intermediate, geodesic_inverse, geodesic_polygon_area_m2,
    ground_resolution_m_per_px, principal_radii_of_curvature, EcefCoord, GeodesicDirectResult,
    GeodesicInverseResult, GeodeticCoord, MEAN_EARTH_RADIUS_M, WGS84_A, WGS84_B, WGS84_E2,
    WGS84_EP2, WGS84_F, WGS84_INV_F,
};
