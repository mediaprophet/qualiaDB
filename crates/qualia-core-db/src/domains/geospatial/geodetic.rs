use std::f64::consts::PI;

pub const WGS84_A: f64 = 6378137.0;
pub const WGS84_INV_F: f64 = 298.257223563;
pub const WGS84_F: f64 = 1.0 / WGS84_INV_F;
pub const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F);
pub const WGS84_E2: f64 = 1.0 - (WGS84_B * WGS84_B) / (WGS84_A * WGS84_A);
pub const WGS84_EP2: f64 = (WGS84_A * WGS84_A - WGS84_B * WGS84_B) / (WGS84_B * WGS84_B);

/// Convert degrees to radians
#[inline]
pub fn deg2rad(deg: f64) -> f64 {
    deg * PI / 180.0
}

/// Convert radians to degrees
#[inline]
pub fn rad2deg(rad: f64) -> f64 {
    rad * 180.0 / PI
}

/// Converts Geodetic coordinates (latitude, longitude, altitude) to Earth-Centered, Earth-Fixed (ECEF) coordinates.
/// Latitude and longitude are in degrees. Altitude is in meters.
/// Returns (X, Y, Z) in meters.
pub fn lat_lon_alt_to_ecef(lat_deg: f64, lon_deg: f64, alt_m: f64) -> (f64, f64, f64) {
    let lat = deg2rad(lat_deg);
    let lon = deg2rad(lon_deg);

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();

    let x = (n + alt_m) * cos_lat * cos_lon;
    let y = (n + alt_m) * cos_lat * sin_lon;
    let z = (n * (1.0 - WGS84_E2) + alt_m) * sin_lat;

    (x, y, z)
}

/// Converts Earth-Centered, Earth-Fixed (ECEF) coordinates to Geodetic (latitude, longitude, altitude).
/// Returns (latitude in degrees, longitude in degrees, altitude in meters).
pub fn ecef_to_lat_lon_alt(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let p = (x * x + y * y).sqrt();
    let lon = y.atan2(x);

    let theta = (z * WGS84_A).atan2(p * WGS84_B);
    let sin_theta = theta.sin();
    let cos_theta = theta.cos();

    let lat = (z + WGS84_EP2 * WGS84_B * sin_theta * sin_theta * sin_theta)
        .atan2(p - WGS84_E2 * WGS84_A * cos_theta * cos_theta * cos_theta);

    let sin_lat = lat.sin();
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    let alt = p / lat.cos() - n;

    (rad2deg(lat), rad2deg(lon), alt)
}

/// Computes the rotation matrix to convert from ECEF to Local Tangent Plane (ENU) at a given reference point.
/// The reference point is given in Geodetic coordinates (degrees).
pub fn ecef_to_enu_matrix(ref_lat_deg: f64, ref_lon_deg: f64) -> [[f64; 3]; 3] {
    let lat = deg2rad(ref_lat_deg);
    let lon = deg2rad(ref_lon_deg);

    let sin_lat = lat.sin();
    let cos_lat = lat.cos();
    let sin_lon = lon.sin();
    let cos_lon = lon.cos();

    [
        [-sin_lon, cos_lon, 0.0],
        [-sin_lat * cos_lon, -sin_lat * sin_lon, cos_lat],
        [cos_lat * cos_lon, cos_lat * sin_lon, sin_lat],
    ]
}

/// Converts a position in ECEF to Local Tangent Plane (ENU) relative to a reference point.
pub fn ecef_to_enu(
    x: f64,
    y: f64,
    z: f64,
    ref_lat_deg: f64,
    ref_lon_deg: f64,
    ref_alt_m: f64,
) -> (f64, f64, f64) {
    let (ref_x, ref_y, ref_z) = lat_lon_alt_to_ecef(ref_lat_deg, ref_lon_deg, ref_alt_m);

    let dx = x - ref_x;
    let dy = y - ref_y;
    let dz = z - ref_z;

    let r = ecef_to_enu_matrix(ref_lat_deg, ref_lon_deg);

    let e = r[0][0] * dx + r[0][1] * dy + r[0][2] * dz;
    let n = r[1][0] * dx + r[1][1] * dy + r[1][2] * dz;
    let u = r[2][0] * dx + r[2][1] * dy + r[2][2] * dz;

    (e, n, u)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lla_ecef_roundtrip() {
        let lat = -33.8688; // Sydney
        let lon = 151.2093;
        let alt = 50.0;

        let (x, y, z) = lat_lon_alt_to_ecef(lat, lon, alt);
        let (r_lat, r_lon, r_alt) = ecef_to_lat_lon_alt(x, y, z);

        assert!((lat - r_lat).abs() < 1e-8);
        assert!((lon - r_lon).abs() < 1e-8);
        assert!((alt - r_alt).abs() < 1e-4);
    }
}
