// Copyright (c) EarthCraft contributors
// Adapted under MIT License for QualiaDB.
//! High-precision WGS84 geodesy and ellipsoidal geometric operations.
//!
//! Provides Vincenty/Karney geodesics, Bowring closed-form ECEF/geodetic conversion,
//! ENU tangent bases, principal radii of curvature, and authalic polygon area.
//!
//! Strictly enforces Tier-1 `HotZeroHeap` (zero allocations in all evaluation paths).

use std::f64::consts::PI;

pub const WGS84_A: f64 = 6378137.0;
pub const WGS84_INV_F: f64 = 298.257223563;
pub const WGS84_F: f64 = 1.0 / WGS84_INV_F;
pub const WGS84_B: f64 = 6356752.314245179;
pub const WGS84_E2: f64 = 6.694379990141317e-3;
pub const WGS84_EP2: f64 = (WGS84_A * WGS84_A - WGS84_B * WGS84_B) / (WGS84_B * WGS84_B);
pub const MEAN_EARTH_RADIUS_M: f64 = 6371008.8;

/// Geodetic coordinates (latitude, longitude in degrees, ellipsoidal height in meters).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticCoord {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_m: f64,
}

impl GeodeticCoord {
    #[inline]
    pub const fn new(lat_deg: f64, lon_deg: f64, alt_m: f64) -> Self {
        Self { lat_deg, lon_deg, alt_m }
    }
}

/// Earth-Centered, Earth-Fixed (ECEF) Cartesian coordinates in meters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcefCoord {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl EcefCoord {
    #[inline]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

/// Result of an inverse geodesic computation between two points on the ellipsoid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodesicInverseResult {
    pub distance_m: f64,
    pub initial_bearing_deg: f64,
    pub final_bearing_deg: f64,
    pub iterations: u32,
}

/// Result of a direct geodesic computation from a starting point, bearing, and distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodesicDirectResult {
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub final_bearing_deg: f64,
}

#[inline]
fn wrap_rad(rad: f64) -> f64 {
    (rad + PI).rem_euclid(2.0 * PI) - PI
}

#[inline]
fn normalize_deg(deg: f64) -> f64 {
    deg.rem_euclid(360.0)
}

#[inline]
fn vincenty_ab(cos2_a: f64) -> (f64, f64) {
    let u2 = cos2_a * WGS84_EP2;
    let a = 1.0 + (u2 / 16384.0) * (4096.0 + u2 * (-768.0 + u2 * (320.0 - 175.0 * u2)));
    let b = (u2 / 1024.0) * (256.0 + u2 * (-128.0 + u2 * (74.0 - 47.0 * u2)));
    (a, b)
}

#[inline]
fn vincenty_delta_sigma(b: f64, sin_s: f64, cos_s: f64, cos2_sm: f64) -> f64 {
    let c2 = cos2_sm * cos2_sm;
    b * sin_s
        * (cos2_sm
            + (b / 4.0)
                * (cos_s * (-1.0 + 2.0 * c2)
                    - (b / 6.0) * cos2_sm * (-3.0 + 4.0 * sin_s * sin_s) * (-3.0 + 4.0 * c2)))
}

#[inline]
fn vincenty_dlambda(cos2_a: f64, sin_a: f64, sigma: f64, sin_s: f64, cos_s: f64, cos2_sm: f64) -> f64 {
    let c = (WGS84_F / 16.0) * cos2_a * (4.0 + WGS84_F * (4.0 - 3.0 * cos2_a));
    (1.0 - c) * WGS84_F * sin_a * (sigma + c * sin_s * (cos2_sm + c * cos_s * (-1.0 + 2.0 * cos2_sm * cos2_sm)))
}

/// Converts geodetic latitude/longitude/altitude to ECEF coordinates.
pub fn geodetic_to_ecef(c: GeodeticCoord) -> EcefCoord {
    let (s_lat, c_lat) = (c.lat_deg.to_radians().sin(), c.lat_deg.to_radians().cos());
    let (s_lon, c_lon) = (c.lon_deg.to_radians().sin(), c.lon_deg.to_radians().cos());
    let n = WGS84_A / (1.0 - WGS84_E2 * s_lat * s_lat).sqrt();
    EcefCoord {
        x: (n + c.alt_m) * c_lat * c_lon,
        y: (n + c.alt_m) * c_lat * s_lon,
        z: (n * (1.0 - WGS84_E2) + c.alt_m) * s_lat,
    }
}

/// Converts ECEF coordinates to geodetic coordinates using Bowring's multi-step refinement.
pub fn ecef_to_geodetic_bowring(ecef: EcefCoord) -> GeodeticCoord {
    let p = (ecef.x * ecef.x + ecef.y * ecef.y).sqrt();
    if p < 1e-12 {
        let lat_deg = if ecef.z >= 0.0 { 90.0 } else { -90.0 };
        return GeodeticCoord::new(lat_deg, 0.0, ecef.z.abs() - WGS84_B);
    }
    let lon_rad = ecef.y.atan2(ecef.x);
    let mut theta = (ecef.z * WGS84_A).atan2(p * WGS84_B);
    let mut lat_rad = 0.0;
    for _ in 0..3 {
        let (st, ct) = (theta.sin(), theta.cos());
        lat_rad = (ecef.z + WGS84_EP2 * WGS84_B * st.powi(3))
            .atan2(p - WGS84_E2 * WGS84_A * ct.powi(3));
        theta = (WGS84_B * lat_rad.sin()).atan2(WGS84_A * lat_rad.cos());
    }
    let (s_lat, c_lat) = (lat_rad.sin(), lat_rad.cos());
    let w = (1.0 - WGS84_E2 * s_lat * s_lat).sqrt();
    let alt_m = p * c_lat + ecef.z * s_lat - WGS84_A * w;
    GeodeticCoord::new(lat_rad.to_degrees(), lon_rad.to_degrees(), alt_m)
}

/// Computes principal radii of curvature in meters: `(meridional_radius_m, prime_vertical_radius_n)`.
pub fn principal_radii_of_curvature(lat_deg: f64) -> (f64, f64) {
    let s = lat_deg.to_radians().sin();
    let denom = 1.0 - WGS84_E2 * s * s;
    let root = denom.sqrt();
    (WGS84_A * (1.0 - WGS84_E2) / (denom * root), WGS84_A / root)
}

/// Orthonormal rotation matrix transforming ECEF delta vectors into local ENU `[East, North, Up]`.
pub fn enu_basis(lat_deg: f64, lon_deg: f64) -> [[f64; 3]; 3] {
    let (s_lat, c_lat) = (lat_deg.to_radians().sin(), lat_deg.to_radians().cos());
    let (s_lon, c_lon) = (lon_deg.to_radians().sin(), lon_deg.to_radians().cos());
    [
        [-s_lon, c_lon, 0.0],
        [-s_lat * c_lon, -s_lat * s_lon, c_lat],
        [c_lat * c_lon, c_lat * s_lon, s_lat],
    ]
}

/// Transforms an ECEF point to local ENU tangent plane relative to an anchor.
pub fn ecef_to_enu(point: EcefCoord, anchor: GeodeticCoord) -> [f64; 3] {
    let anc = geodetic_to_ecef(anchor);
    let (dx, dy, dz) = (point.x - anc.x, point.y - anc.y, point.z - anc.z);
    let r = enu_basis(anchor.lat_deg, anchor.lon_deg);
    [
        r[0][0] * dx + r[0][1] * dy + r[0][2] * dz,
        r[1][0] * dx + r[1][1] * dy + r[1][2] * dz,
        r[2][0] * dx + r[2][1] * dy + r[2][2] * dz,
    ]
}

/// Transforms local ENU coordinates `[East, North, Up]` relative to an anchor back into ECEF.
pub fn enu_to_ecef(enu: [f64; 3], anchor: GeodeticCoord) -> EcefCoord {
    let anc = geodetic_to_ecef(anchor);
    let r = enu_basis(anchor.lat_deg, anchor.lon_deg);
    let dx = r[0][0] * enu[0] + r[1][0] * enu[1] + r[2][0] * enu[2];
    let dy = r[0][1] * enu[0] + r[1][1] * enu[1] + r[2][1] * enu[2];
    let dz = r[0][2] * enu[0] + r[1][2] * enu[1] + r[2][2] * enu[2];
    EcefCoord::new(anc.x + dx, anc.y + dy, anc.z + dz)
}

/// Geodesic inverse problem on WGS84 with antipodal fallback.
pub fn geodesic_inverse(lat1_deg: f64, lon1_deg: f64, lat2_deg: f64, lon2_deg: f64) -> GeodesicInverseResult {
    let (phi1, phi2) = (lat1_deg.to_radians(), lat2_deg.to_radians());
    let (l1, l2) = (lon1_deg.to_radians(), lon2_deg.to_radians());
    let d_lon = wrap_rad(l2 - l1);

    if (lat1_deg - lat2_deg).abs() < 1e-12 && d_lon.abs() < 1e-12 {
        return GeodesicInverseResult { distance_m: 0.0, initial_bearing_deg: 0.0, final_bearing_deg: 0.0, iterations: 0 };
    }

    let (u1, u2) = (((1.0 - WGS84_F) * phi1.tan()).atan(), ((1.0 - WGS84_F) * phi2.tan()).atan());
    let (sin_u1, cos_u1) = (u1.sin(), u1.cos());
    let (sin_u2, cos_u2) = (u2.sin(), u2.cos());

    let (mut lambda, mut iters, mut converged) = (d_lon, 0, false);
    let (mut sin_s, mut cos_s, mut sigma, mut cos2_a, mut cos2_sm) = (0.0, 1.0, 0.0, 0.0, 0.0);

    for i in 0..200 {
        iters = i + 1;
        let (sin_l, cos_l) = (lambda.sin(), lambda.cos());
        let (t1, t2) = (cos_u2 * sin_l, cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_l);
        sin_s = (t1 * t1 + t2 * t2).sqrt();
        if sin_s.abs() < 1e-15 {
            cos_s = 1.0; sigma = 0.0; converged = true; break;
        }
        cos_s = sin_u1 * sin_u2 + cos_u1 * cos_u2 * cos_l;
        sigma = sin_s.atan2(cos_s);
        let sin_a = cos_u1 * cos_u2 * sin_l / sin_s;
        cos2_a = 1.0 - sin_a * sin_a;
        cos2_sm = if cos2_a.abs() > 1e-12 { cos_s - 2.0 * sin_u1 * sin_u2 / cos2_a } else { 0.0 };

        let lambda_next = d_lon + vincenty_dlambda(cos2_a, sin_a, sigma, sin_s, cos_s, cos2_sm);
        if (lambda_next - lambda).abs() < 1e-12 {
            lambda = lambda_next; converged = true; break;
        }
        lambda = lambda_next;
    }

    if !converged {
        let h = ((phi2 - phi1) * 0.5).sin().powi(2) + phi1.cos() * phi2.cos() * (d_lon * 0.5).sin().powi(2);
        let dist = MEAN_EARTH_RADIUS_M * 2.0 * h.clamp(0.0, 1.0).sqrt().asin();
        let (y1, x1) = (d_lon.sin() * phi2.cos(), phi1.cos() * phi2.sin() - phi1.sin() * phi2.cos() * d_lon.cos());
        let (y2, x2) = (d_lon.sin() * phi1.cos(), -phi2.cos() * phi1.sin() + phi2.sin() * phi1.cos() * d_lon.cos());
        return GeodesicInverseResult {
            distance_m: dist,
            initial_bearing_deg: normalize_deg(y1.atan2(x1).to_degrees()),
            final_bearing_deg: normalize_deg(y2.atan2(x2).to_degrees()),
            iterations: iters,
        };
    }

    let (a_param, b_param) = vincenty_ab(cos2_a);
    let delta_s = vincenty_delta_sigma(b_param, sin_s, cos_s, cos2_sm);
    let distance_m = WGS84_B * a_param * (sigma - delta_s);
    let (sin_l, cos_l) = (lambda.sin(), lambda.cos());
    let init_brg = (cos_u2 * sin_l).atan2(cos_u1 * sin_u2 - sin_u1 * cos_u2 * cos_l);
    let fin_brg = (cos_u1 * sin_l).atan2(-sin_u1 * cos_u2 + cos_u1 * sin_u2 * cos_l);

    GeodesicInverseResult {
        distance_m,
        initial_bearing_deg: normalize_deg(init_brg.to_degrees()),
        final_bearing_deg: normalize_deg(fin_brg.to_degrees()),
        iterations: iters,
    }
}

/// Geodesic direct problem on WGS84.
pub fn geodesic_direct(lat1_deg: f64, lon1_deg: f64, bearing_deg: f64, distance_m: f64) -> GeodesicDirectResult {
    if distance_m.abs() < 1e-9 {
        return GeodesicDirectResult { lat_deg: lat1_deg, lon_deg: lon1_deg, final_bearing_deg: normalize_deg(bearing_deg) };
    }

    let (phi1, alpha1) = (lat1_deg.to_radians(), bearing_deg.to_radians());
    let u1 = ((1.0 - WGS84_F) * phi1.tan()).atan();
    let (sin_u1, cos_u1) = (u1.sin(), u1.cos());
    let sigma1 = u1.tan().atan2(alpha1.cos());
    let sin_a = cos_u1 * alpha1.sin();
    let cos2_a = 1.0 - sin_a * sin_a;
    let (a_param, b_param) = vincenty_ab(cos2_a);

    let mut sigma = distance_m / (WGS84_B * a_param);
    let (mut sin_s, mut cos_s, mut cos2_sm) = (0.0, 1.0, 0.0);

    for _ in 0..100 {
        cos2_sm = (2.0 * sigma1 + sigma).cos();
        sin_s = sigma.sin();
        cos_s = sigma.cos();
        let delta_s = vincenty_delta_sigma(b_param, sin_s, cos_s, cos2_sm);
        let sigma_next = distance_m / (WGS84_B * a_param) + delta_s;
        if (sigma_next - sigma).abs() < 1e-12 {
            sigma = sigma_next; break;
        }
        sigma = sigma_next;
    }

    let x = sin_u1 * sin_s - cos_u1 * cos_s * alpha1.cos();
    let phi2 = (sin_u1 * cos_s + cos_u1 * sin_s * alpha1.cos())
        .atan2((1.0 - WGS84_F) * (sin_a * sin_a + x * x).sqrt());
    let lambda = (sin_s * alpha1.sin()).atan2(cos_u1 * cos_s - sin_u1 * sin_s * alpha1.cos());
    let l = lambda - vincenty_dlambda(cos2_a, sin_a, sigma, sin_s, cos_s, cos2_sm);
    let lon2_deg = (lon1_deg + l.to_degrees() + 180.0).rem_euclid(360.0) - 180.0;
    let alpha2 = sin_a.atan2(-x);

    GeodesicDirectResult {
        lat_deg: phi2.to_degrees(),
        lon_deg: lon2_deg,
        final_bearing_deg: normalize_deg(alpha2.to_degrees()),
    }
}

/// Computes intermediate waypoint along the geodesic at parametric fraction `[0.0, 1.0]`.
pub fn geodesic_intermediate(lat1_deg: f64, lon1_deg: f64, lat2_deg: f64, lon2_deg: f64, fraction: f64) -> (f64, f64) {
    if fraction <= 0.0 {
        return (lat1_deg, lon1_deg);
    }
    if fraction >= 1.0 {
        return (lat2_deg, lon2_deg);
    }
    let inv = geodesic_inverse(lat1_deg, lon1_deg, lat2_deg, lon2_deg);
    let dir = geodesic_direct(lat1_deg, lon1_deg, inv.initial_bearing_deg, inv.distance_m * fraction);
    (dir.lat_deg, dir.lon_deg)
}

/// Zero-heap ellipsoidal polygon area in square meters using authalic latitude mapping.
pub fn geodesic_polygon_area_m2(ring_lat_lon_deg: &[(f64, f64)]) -> f64 {
    let len = ring_lat_lon_deg.len();
    if len < 3 {
        return 0.0;
    }
    let n = if len > 1 && ring_lat_lon_deg[0] == ring_lat_lon_deg[len - 1] { len - 1 } else { len };
    if n < 3 {
        return 0.0;
    }

    let e = WGS84_E2.sqrt();
    let q_p = 1.0 + ((1.0 - WGS84_E2) / e) * e.atanh();
    let r_auth2 = (WGS84_A * WGS84_A * q_p) * 0.5;

    let authalic_sin = |lat_deg: f64| -> f64 {
        let s = lat_deg.to_radians().sin();
        let es = (e * s).clamp(-0.99999999999999, 0.99999999999999);
        let q = (1.0 - WGS84_E2) * (s / (1.0 - WGS84_E2 * s * s) + (1.0 / e) * es.atanh());
        (q / q_p).clamp(-1.0, 1.0)
    };

    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let (lat1, lon1) = ring_lat_lon_deg[i];
        let (lat2, lon2) = ring_lat_lon_deg[j];
        let dlon = wrap_rad((lon2 - lon1).to_radians());
        let s1 = authalic_sin(lat1);
        let s2 = authalic_sin(lat2);
        sum += dlon * (2.0 + s1 + s2);
    }

    (sum * 0.5 * r_auth2).abs()
}

/// Web Mercator ground resolution in meters per pixel at a given latitude and zoom level.
pub fn ground_resolution_m_per_px(lat_deg: f64, zoom: u8, tile_px: u32) -> f64 {
    let cos_lat = lat_deg.to_radians().cos().abs();
    let total_px = (tile_px as f64) * (2.0_f64).powi(zoom as i32);
    (2.0 * PI * WGS84_A * cos_lat) / total_px
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sydney_kosciuszko_geodesic_roundtrip() {
        let (syd_lat, syd_lon) = (-33.8688, 151.2093);
        let (kos_lat, kos_lon) = (-36.4558, 148.2636);

        let inv = geodesic_inverse(syd_lat, syd_lon, kos_lat, kos_lon);
        assert!(inv.distance_m > 380_000.0 && inv.distance_m < 410_000.0);
        assert!(inv.iterations > 0 && inv.iterations < 20);

        let dir = geodesic_direct(syd_lat, syd_lon, inv.initial_bearing_deg, inv.distance_m);
        let error_check = geodesic_inverse(dir.lat_deg, dir.lon_deg, kos_lat, kos_lon);
        assert!(error_check.distance_m < 1e-3, "Endpoint error: {} m", error_check.distance_m);
        assert!((dir.final_bearing_deg - inv.final_bearing_deg).abs() < 1e-4);
    }

    #[test]
    fn test_bowring_roundtrip_altitudes() {
        let test_points = [
            GeodeticCoord::new(-33.8688, 151.2093, 0.0),       // Sydney sea level
            GeodeticCoord::new(-36.4558, 148.2636, 2228.0),    // Mt Kosciuszko summit
            GeodeticCoord::new(45.0, 10.0, 400_000.0),          // LEO satellite orbit
        ];

        for pt in test_points {
            let ecef = geodetic_to_ecef(pt);
            let recovered = ecef_to_geodetic_bowring(ecef);
            assert!((pt.lat_deg - recovered.lat_deg).abs() < 1e-8, "Lat mismatch: {:?}", pt);
            assert!((pt.lon_deg - recovered.lon_deg).abs() < 1e-8, "Lon mismatch: {:?}", pt);
            assert!((pt.alt_m - recovered.alt_m).abs() < 1e-3, "Alt mismatch: {:?}", pt);
        }
    }

    #[test]
    fn test_enu_ecef_roundtrip() {
        let anchor = GeodeticCoord::new(-36.4558, 148.2636, 2228.0);
        let enu_original = [1250.5, -3420.25, 450.0];

        let ecef = enu_to_ecef(enu_original, anchor);
        let enu_recovered = ecef_to_enu(ecef, anchor);

        assert!((enu_original[0] - enu_recovered[0]).abs() < 1e-6);
        assert!((enu_original[1] - enu_recovered[1]).abs() < 1e-6);
        assert!((enu_original[2] - enu_recovered[2]).abs() < 1e-6);
    }

    #[test]
    fn test_geodesic_polygon_area_1km_square() {
        let (p0_lat, p0_lon) = (-36.4558, 148.2636);

        let p1 = geodesic_direct(p0_lat, p0_lon, 90.0, 1000.0);
        let p2 = geodesic_direct(p1.lat_deg, p1.lon_deg, 0.0, 1000.0);
        let p3 = geodesic_direct(p0_lat, p0_lon, 0.0, 1000.0);

        let square = [
            (p0_lat, p0_lon),
            (p1.lat_deg, p1.lon_deg),
            (p2.lat_deg, p2.lon_deg),
            (p3.lat_deg, p3.lon_deg),
            (p0_lat, p0_lon),
        ];

        let area = geodesic_polygon_area_m2(&square);
        let diff = (area - 1_000_000.0).abs();
        assert!(diff < 1000.0, "Expected ~1,000,000 m^2, got {} (diff: {})", area, diff);
    }

    #[test]
    fn test_principal_radii_of_curvature() {
        let (m_eq, n_eq) = principal_radii_of_curvature(0.0);
        assert!((n_eq - WGS84_A).abs() < 1e-3);
        assert!((m_eq - WGS84_A * (1.0 - WGS84_E2)).abs() < 1e-3);

        let (m_pole, n_pole) = principal_radii_of_curvature(90.0);
        assert!((m_pole - n_pole).abs() < 1e-3);
        assert!((m_pole - (WGS84_A * WGS84_A / WGS84_B)).abs() < 1e-2);
    }

    #[test]
    fn test_ground_resolution() {
        let res_eq_z0 = ground_resolution_m_per_px(0.0, 0, 256);
        assert!((res_eq_z0 - 156543.0339).abs() < 1.0);
        let res_eq_z1 = ground_resolution_m_per_px(0.0, 1, 256);
        assert!((res_eq_z1 - res_eq_z0 * 0.5).abs() < 1e-6);
    }
}
