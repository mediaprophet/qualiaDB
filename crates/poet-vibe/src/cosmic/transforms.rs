//! Coordinate transforms — WGS84 ↔ ECEF ↔ ENU, geodetic distance (OCS §4).
//!
//! Reference: OCS Specification v2.2.0 §4.

/// WGS84 ellipsoid constants.
const WGS84_A: f64 = 6_378_137.0; // Semi-major axis (m)
const WGS84_F: f64 = 1.0 / 298.257223563; // Flattening
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F); // Semi-minor axis
const WGS84_E2: f64 = WGS84_F * (2.0 - WGS84_F); // First eccentricity squared

/// Geodetic coordinates: latitude, longitude, altitude (OCS §4.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geodetic {
    /// Latitude in degrees.
    pub lat_deg: f64,
    /// Longitude in degrees.
    pub lon_deg: f64,
    /// Ellipsoidal height in meters.
    pub alt_m: f64,
}

/// ECEF Cartesian coordinates: X, Y, Z (OCS §4.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ecef {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// ENU local tangent plane: East, North, Up (OCS §4.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Enu {
    pub east: f64,
    pub north: f64,
    pub up: f64,
}

/// Convert geodetic coordinates to ECEF (OCS §4.1).
pub fn geodetic_to_ecef(g: Geodetic) -> Ecef {
    let lat = g.lat_deg.to_radians();
    let lon = g.lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    // Prime vertical radius of curvature
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();

    Ecef {
        x: (n + g.alt_m) * cos_lat * cos_lon,
        y: (n + g.alt_m) * cos_lat * sin_lon,
        z: (n * (1.0 - WGS84_E2) + g.alt_m) * sin_lat,
    }
}

/// Convert ECEF to geodetic coordinates using Bowring's closed-form method (OCS §4.1).
pub fn ecef_to_geodetic(e: Ecef) -> Geodetic {
    let x = e.x;
    let y = e.y;
    let z = e.z;

    let lon = y.atan2(x);

    let p = (x * x + y * y).sqrt();
    if p < 1e-12 {
        // At pole
        return Geodetic {
            lat_deg: if z > 0.0 { 90.0 } else { -90.0 },
            lon_deg: 0.0,
            alt_m: z.abs() - WGS84_B,
        };
    }

    // Bowring's closed-form formula (1985)
    // Second eccentricity squared: e'² = (a² - b²) / b²
    let e_prime_sq = (WGS84_A * WGS84_A - WGS84_B * WGS84_B) / (WGS84_B * WGS84_B);

    // Auxiliary angle: θ = atan2(z·a, p·b)
    let theta = (z * WGS84_A).atan2(p * WGS84_B);
    let sin_theta = theta.sin();
    let cos_theta = theta.cos();

    // Geodetic latitude
    let lat = (z + e_prime_sq * WGS84_B * sin_theta * sin_theta * sin_theta)
        .atan2(p - WGS84_E2 * WGS84_A * cos_theta * cos_theta * cos_theta);

    let sin_lat = lat.sin();
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    let alt = p / lat.cos() - n;

    Geodetic {
        lat_deg: lat.to_degrees(),
        lon_deg: lon.to_degrees(),
        alt_m: alt,
    }
}

/// Convert ECEF to ENU relative to a reference geodetic point (OCS §4.1).
pub fn ecef_to_enu(e: Ecef, ref_geo: Geodetic) -> Enu {
    let ref_ecef = geodetic_to_ecef(ref_geo);
    let dx = e.x - ref_ecef.x;
    let dy = e.y - ref_ecef.y;
    let dz = e.z - ref_ecef.z;

    let lat = ref_geo.lat_deg.to_radians();
    let lon = ref_geo.lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    // Rotation matrix: ECEF → ENU
    Enu {
        east: -sin_lon * dx + cos_lon * dy,
        north: -sin_lat * cos_lon * dx - sin_lat * sin_lon * dy + cos_lat * dz,
        up: cos_lat * cos_lon * dx + cos_lat * sin_lon * dy + sin_lat * dz,
    }
}

/// Convert ENU to ECEF relative to a reference geodetic point (OCS §4.1).
pub fn enu_to_ecef(enu: Enu, ref_geo: Geodetic) -> Ecef {
    let ref_ecef = geodetic_to_ecef(ref_geo);
    let lat = ref_geo.lat_deg.to_radians();
    let lon = ref_geo.lon_deg.to_radians();
    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    // Inverse rotation: ENU → ECEF
    Ecef {
        x: ref_ecef.x - sin_lon * enu.east - sin_lat * cos_lon * enu.north
            + cos_lat * cos_lon * enu.up,
        y: ref_ecef.y + cos_lon * enu.east - sin_lat * sin_lon * enu.north
            + cos_lat * sin_lon * enu.up,
        z: ref_ecef.z + cos_lat * enu.north + sin_lat * enu.up,
    }
}

/// Great-circle distance between two geodetic points using the Haversine formula.
/// Returns distance in meters.
pub fn geodetic_distance(a: Geodetic, b: Geodetic) -> f64 {
    let lat1 = a.lat_deg.to_radians();
    let lat2 = b.lat_deg.to_radians();
    let dlat = lat2 - lat1;
    let dlon = (b.lon_deg - a.lon_deg).to_radians();

    let sin_dlat = (dlat / 2.0).sin();
    let sin_dlon = (dlon / 2.0).sin();
    let h = sin_dlat * sin_dlat + lat1.cos() * lat2.cos() * sin_dlon * sin_dlon;
    // Earth's mean radius
    let r = 6_371_008.8;
    2.0 * r * h.sqrt().asin()
}

/// Convert Ecef to a Value::Record.
pub fn ecef_to_value(e: Ecef) -> crate::value::Value {
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("x".into(), crate::value::Value::F64(e.x));
    rec.insert("y".into(), crate::value::Value::F64(e.y));
    rec.insert("z".into(), crate::value::Value::F64(e.z));
    crate::value::Value::Record(rec)
}

/// Convert Geodetic to a Value::Record.
pub fn geodetic_to_value(g: Geodetic) -> crate::value::Value {
    let mut rec = std::collections::BTreeMap::new();
    rec.insert("lat".into(), crate::value::Value::F64(g.lat_deg));
    rec.insert("lon".into(), crate::value::Value::F64(g.lon_deg));
    rec.insert("alt".into(), crate::value::Value::F64(g.alt_m));
    crate::value::Value::Record(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geodetic_to_ecef_origin() {
        // At (0, 0, 0) — equator/prime meridian on the ellipsoid
        let g = Geodetic {
            lat_deg: 0.0,
            lon_deg: 0.0,
            alt_m: 0.0,
        };
        let e = geodetic_to_ecef(g);
        assert!((e.x - WGS84_A).abs() < 1e-6, "x should be WGS84_A");
        assert!(e.y.abs() < 1e-6);
        assert!(e.z.abs() < 1e-6);
    }

    #[test]
    fn geodetic_to_ecef_north_pole() {
        let g = Geodetic {
            lat_deg: 90.0,
            lon_deg: 0.0,
            alt_m: 0.0,
        };
        let e = geodetic_to_ecef(g);
        assert!(e.x.abs() < 1e-6);
        assert!(e.y.abs() < 1e-6);
        assert!((e.z - WGS84_B).abs() < 1e-6, "z should be WGS84_B");
    }

    #[test]
    fn ecef_to_geodetic_round_trip() {
        let original = Geodetic {
            lat_deg: 37.8080,
            lon_deg: -122.4177,
            alt_m: 10.0,
        };
        let ecef = geodetic_to_ecef(original);
        let recovered = ecef_to_geodetic(ecef);
        assert!(
            (recovered.lat_deg - original.lat_deg).abs() < 1e-6,
            "lat mismatch"
        );
        assert!(
            (recovered.lon_deg - original.lon_deg).abs() < 1e-6,
            "lon mismatch"
        );
        assert!(
            (recovered.alt_m - original.alt_m).abs() < 1e-3,
            "alt mismatch"
        );
    }

    #[test]
    fn ecef_to_geodetic_round_trip_high_lat() {
        let original = Geodetic {
            lat_deg: 78.5,
            lon_deg: 15.0,
            alt_m: 500.0,
        };
        let ecef = geodetic_to_ecef(original);
        let recovered = ecef_to_geodetic(ecef);
        assert!((recovered.lat_deg - original.lat_deg).abs() < 1e-6);
        assert!((recovered.lon_deg - original.lon_deg).abs() < 1e-6);
        assert!((recovered.alt_m - original.alt_m).abs() < 1e-3);
    }

    #[test]
    fn ecef_enu_round_trip() {
        let ref_geo = Geodetic {
            lat_deg: 37.8080,
            lon_deg: -122.4177,
            alt_m: 10.0,
        };
        let original = Enu {
            east: 100.0,
            north: 200.0,
            up: 50.0,
        };
        let ecef = enu_to_ecef(original, ref_geo);
        let recovered = ecef_to_enu(ecef, ref_geo);
        assert!(
            (recovered.east - original.east).abs() < 1e-6,
            "east mismatch"
        );
        assert!(
            (recovered.north - original.north).abs() < 1e-6,
            "north mismatch"
        );
        assert!((recovered.up - original.up).abs() < 1e-6, "up mismatch");
    }

    #[test]
    fn geodetic_distance_sf_to_la() {
        // San Francisco to Los Angeles — ~559 km
        let sf = Geodetic {
            lat_deg: 37.7749,
            lon_deg: -122.4194,
            alt_m: 0.0,
        };
        let la = Geodetic {
            lat_deg: 34.0522,
            lon_deg: -118.2437,
            alt_m: 0.0,
        };
        let d = geodetic_distance(sf, la);
        assert!((d - 559_000.0).abs() < 5_000.0, "got {} expected ~559km", d);
    }

    #[test]
    fn geodetic_distance_same_point() {
        let g = Geodetic {
            lat_deg: 45.0,
            lon_deg: 90.0,
            alt_m: 0.0,
        };
        assert!(geodetic_distance(g, g).abs() < 1e-6);
    }

    #[test]
    fn geodetic_distance_antipode() {
        // Antipodal points — ~20015 km (half circumference)
        let a = Geodetic {
            lat_deg: 0.0,
            lon_deg: 0.0,
            alt_m: 0.0,
        };
        let b = Geodetic {
            lat_deg: 0.0,
            lon_deg: 180.0,
            alt_m: 0.0,
        };
        let d = geodetic_distance(a, b);
        assert!(
            (d - 20_015_000.0).abs() < 50_000.0,
            "got {} expected ~20015km",
            d
        );
    }

    #[test]
    fn ecef_to_geodetic_at_pole() {
        let e = Ecef {
            x: 0.0,
            y: 0.0,
            z: WGS84_B,
        };
        let g = ecef_to_geodetic(e);
        assert!((g.lat_deg - 90.0).abs() < 1e-6);
    }
}
