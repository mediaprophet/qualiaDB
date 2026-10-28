//! Geodesy and ephemeris capability schemas (GEO-S1 / R1-V).
//!
//! Provides machine schema definitions for all 17 `Geodesy.*` host capability
//! invocations and unit suffix validation for geodetic, hydrological, and
//! cartographic quantities.

use super::{EffectClass, HonestyLabel, SchemaArg, SchemaEntry};

/// Canonical sequence of all 17 Geodesy capability invoke IDs.
pub const GEODESY_INVOKE_IDS: &[&str] = &[
    "Geodesy.wgs84_inverse",
    "Geodesy.wgs84_direct",
    "Geodesy.wgs84_intermediate",
    "Geodesy.polygon_area_m2",
    "Geodesy.geodetic_to_ecef",
    "Geodesy.ecef_to_geodetic",
    "Geodesy.enu_basis",
    "Geodesy.parse_coord",
    "Geodesy.format_mgrs",
    "Geodesy.crs_transform",
    "Geodesy.ausgeoid_undulation",
    "Geodesy.lonlat_to_a5",
    "Geodesy.a5_to_lonlat",
    "Geodesy.rtree_query_bbox",
    "Geodesy.sun_ephemeris",
    "Geodesy.topographic_insolation",
    "Geodesy.orbit_propagate",
];

/// Registered geodetic, hydrological, and cartographic unit tokens.
pub const GEODESY_UNIT_SUFFIXES: &[&str] = &[
    "mm", "cm", "m", "km", "nmi", "deg", "rad", "arcmin", "arcsec", "mrad", "m2", "ha", "km2",
    "m3_s", "mm_hr", "kg_m2_s", "W_m2", "Wh_m2", "pt", "dpi",
];

/// Returns `true` if `unit` matches any registered geodetic, hydrological, or cartographic unit token.
pub fn is_geodesy_unit_suffix(unit: &str) -> bool {
    matches!(
        unit,
        "mm" | "cm"
            | "m"
            | "km"
            | "nmi"
            | "deg"
            | "rad"
            | "arcmin"
            | "arcsec"
            | "mrad"
            | "m2"
            | "ha"
            | "km2"
            | "m3_s"
            | "mm_hr"
            | "kg_m2_s"
            | "W_m2"
            | "Wh_m2"
            | "pt"
            | "dpi"
    )
}

/// Returns the 17 `Geodesy.*` host capability schema entries.
pub fn entries() -> Vec<SchemaEntry> {
    vec![
        SchemaEntry::new(
            "Geodesy.wgs84_inverse",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("lat1_deg", "f64", true))
        .with_arg(SchemaArg::new("lon1_deg", "f64", true))
        .with_arg(SchemaArg::new("lat2_deg", "f64", true))
        .with_arg(SchemaArg::new("lon2_deg", "f64", true))
        .with_description("Solves the inverse geodesic problem on WGS-84 (distance, azimuth, reverse azimuth)."),

        SchemaEntry::new(
            "Geodesy.wgs84_direct",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("lat1_deg", "f64", true))
        .with_arg(SchemaArg::new("lon1_deg", "f64", true))
        .with_arg(SchemaArg::new("bearing_deg", "f64", true))
        .with_arg(SchemaArg::new("distance_m", "f64", true))
        .with_description("Solves the direct geodesic problem on WGS-84 given start point, bearing, and distance."),

        SchemaEntry::new(
            "Geodesy.wgs84_intermediate",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("lat1_deg", "f64", true))
        .with_arg(SchemaArg::new("lon1_deg", "f64", true))
        .with_arg(SchemaArg::new("lat2_deg", "f64", true))
        .with_arg(SchemaArg::new("lon2_deg", "f64", true))
        .with_arg(SchemaArg::new("fraction", "f64", true))
        .with_description("Interpolates an intermediate point along the WGS-84 geodesic arc."),

        SchemaEntry::new(
            "Geodesy.polygon_area_m2",
            EffectClass::Pure,
            HonestyLabel::Always,
            "f64",
        )
        .with_arg(SchemaArg::new("points", "List<Record>", true))
        .with_description("Computes the ellipsoidal surface area in square meters for a closed geodesic polygon."),

        SchemaEntry::new(
            "Geodesy.geodetic_to_ecef",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("lat_deg", "f64", true))
        .with_arg(SchemaArg::new("lon_deg", "f64", true))
        .with_arg(SchemaArg::new("alt_m", "f64", true))
        .with_description("Converts geodetic coordinates to Earth-Centered Earth-Fixed (ECEF) Cartesian XYZ."),

        SchemaEntry::new(
            "Geodesy.ecef_to_geodetic",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("x", "f64", true))
        .with_arg(SchemaArg::new("y", "f64", true))
        .with_arg(SchemaArg::new("z", "f64", true))
        .with_description("Converts ECEF Cartesian XYZ to geodetic latitude, longitude, and ellipsoidal altitude."),

        SchemaEntry::new(
            "Geodesy.enu_basis",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("lat_deg", "f64", true))
        .with_arg(SchemaArg::new("lon_deg", "f64", true))
        .with_description("Calculates the local East-North-Up (ENU) tangent basis orthonormal matrix."),

        SchemaEntry::new(
            "Geodesy.parse_coord",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("text", "string", true))
        .with_description("Parses coordinate text in DMS, decimal degrees, UTM, or MGRS format into lat/lon record."),

        SchemaEntry::new(
            "Geodesy.format_mgrs",
            EffectClass::Pure,
            HonestyLabel::Always,
            "string",
        )
        .with_arg(SchemaArg::new("lat_deg", "f64", true))
        .with_arg(SchemaArg::new("lon_deg", "f64", true))
        .with_arg(SchemaArg::new("precision", "u64", true))
        .with_description("Formats geodetic coordinates into a Military Grid Reference System (MGRS) string."),

        SchemaEntry::new(
            "Geodesy.crs_transform",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("src_epsg", "u64", true))
        .with_arg(SchemaArg::new("dst_epsg", "u64", true))
        .with_arg(SchemaArg::new("x", "f64", true))
        .with_arg(SchemaArg::new("y", "f64", true))
        .with_description("Transforms 2D coordinates between EPSG Coordinate Reference Systems."),

        SchemaEntry::new(
            "Geodesy.ausgeoid_undulation",
            EffectClass::Pure,
            HonestyLabel::Always,
            "f64",
        )
        .with_arg(SchemaArg::new("lat_deg", "f64", true))
        .with_arg(SchemaArg::new("lon_deg", "f64", true))
        .with_description("Evaluates AUSGeoid2020 gravimetric geoid undulation (N value) for Australian height datum."),

        SchemaEntry::new(
            "Geodesy.lonlat_to_a5",
            EffectClass::Pure,
            HonestyLabel::Always,
            "u64",
        )
        .with_arg(SchemaArg::new("lon_deg", "f64", true))
        .with_arg(SchemaArg::new("lat_deg", "f64", true))
        .with_arg(SchemaArg::new("resolution", "u64", true))
        .with_description("Encodes longitude and latitude into an A5 Discrete Global Grid System (DGGS) cell ID."),

        SchemaEntry::new(
            "Geodesy.a5_to_lonlat",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("cell", "u64", true))
        .with_description("Decodes an A5 Discrete Global Grid System cell ID to its centroid longitude and latitude."),

        SchemaEntry::new(
            "Geodesy.rtree_query_bbox",
            EffectClass::Read,
            HonestyLabel::Always,
            "List<u64>",
        )
        .with_arg(SchemaArg::new("entries", "List<Record>", true))
        .with_arg(SchemaArg::new("bbox", "Record", true))
        .with_description("Queries an in-memory spatial R-tree bounding box index for intersecting candidate IDs."),

        SchemaEntry::new(
            "Geodesy.sun_ephemeris",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("unix_secs", "f64", true))
        .with_arg(SchemaArg::new("lat_deg", "f64", true))
        .with_arg(SchemaArg::new("lon_deg", "f64", true))
        .with_description("Computes solar ephemeris: elevation, azimuth, distance, and declination for an epoch and location."),

        SchemaEntry::new(
            "Geodesy.topographic_insolation",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("unix_secs", "f64", true))
        .with_arg(SchemaArg::new("lat_deg", "f64", true))
        .with_arg(SchemaArg::new("lon_deg", "f64", true))
        .with_arg(SchemaArg::new("alt_m", "f64", true))
        .with_arg(SchemaArg::new("slope_deg", "f64", true))
        .with_arg(SchemaArg::new("aspect_deg", "f64", true))
        .with_description("Calculates direct, diffuse, and reflected solar irradiance on an inclined topographic surface."),

        SchemaEntry::new(
            "Geodesy.orbit_propagate",
            EffectClass::Pure,
            HonestyLabel::Always,
            "Record",
        )
        .with_arg(SchemaArg::new("elements", "Record", true))
        .with_arg(SchemaArg::new("target_unix_secs", "f64", true))
        .with_description("Propagates Keplerian or SGP4 orbital elements to target epoch, returning ECEF/ECI state."),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geodesy_invoke_ids_uniqueness_and_order() {
        assert_eq!(GEODESY_INVOKE_IDS.len(), 17);
        let mut ids = GEODESY_INVOKE_IDS.to_vec();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), 17, "GEODESY_INVOKE_IDS must contain unique entries");
    }

    #[test]
    fn test_entries_count_and_matches_ids() {
        let all_entries = entries();
        assert_eq!(all_entries.len(), 17);
        for (i, entry) in all_entries.iter().enumerate() {
            assert_eq!(entry.id, GEODESY_INVOKE_IDS[i]);
            assert_eq!(entry.honesty, HonestyLabel::Always);
            assert!(
                entry.effect == EffectClass::Pure || entry.effect == EffectClass::Read,
                "Expected Pure or Read effect class, got {:?}",
                entry.effect
            );
            assert!(!entry.returns.is_empty());
        }
    }

    #[test]
    fn test_is_geodesy_unit_suffix() {
        assert_eq!(GEODESY_UNIT_SUFFIXES.len(), 20);
        for unit in GEODESY_UNIT_SUFFIXES {
            assert!(
                is_geodesy_unit_suffix(unit),
                "Expected unit '{unit}' to be recognized as geodesy unit suffix"
            );
        }
        assert!(!is_geodesy_unit_suffix(""));
        assert!(!is_geodesy_unit_suffix("foo"));
        assert!(!is_geodesy_unit_suffix("meter"));
        assert!(!is_geodesy_unit_suffix("sec"));
    }
}
