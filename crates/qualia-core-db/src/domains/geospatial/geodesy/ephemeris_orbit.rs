// EarthCraft / IAU SOFA derived high-precision ephemeris, insolation & orbital mechanics.
// Portions derived from EarthCraft (MIT License) and IAU SOFA (SOFA Software License).
// Copyright (c) 2026 EarthCraft contributors / QualiaDB contributors.
//
// Tier-1 HotZeroHeap: Pure analytical f64 ephemeris, insolation & orbital mechanics.
// Allocates zero heap: no Vec, String, or Box in any execution path.

use std::f64::consts::{PI, TAU};

const DEG_TO_RAD: f64 = PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / PI;

const SECONDS_PER_DAY: f64 = 86_400.0;
const JD_UNIX_EPOCH: f64 = 2_440_587.5;
const JD_J2000: f64 = 2_451_545.0;

// Earth physical constants (WGS84 & IERS Numerical Standards)
const WGS84_A: f64 = 6_378_137.0;
const WGS84_INV_F: f64 = 298.257_223_563;
const WGS84_F: f64 = 1.0 / WGS84_INV_F;
const WGS84_B: f64 = WGS84_A * (1.0 - WGS84_F);
const WGS84_E2: f64 = 1.0 - (WGS84_B * WGS84_B) / (WGS84_A * WGS84_A);
const WGS84_EP2: f64 = (WGS84_A * WGS84_A - WGS84_B * WGS84_B) / (WGS84_B * WGS84_B);

const GM_EARTH: f64 = 3.986_004_418e14;
const J2_EARTH: f64 = 1.082_626_68e-3;
const OMEGA_EARTH: f64 = 7.292_115e-5;
const SOLAR_CONSTANT_W_M2: f64 = 1361.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarPosition {
    pub ecef_unit: [f64; 3],
    pub enu_unit: [f64; 3],
    pub elevation_deg: f64,
    pub azimuth_deg: f64,
    pub declination_deg: f64,
    pub right_ascension_deg: f64,
    pub distance_au: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarPosition {
    pub ecef_unit: [f64; 3],
    pub enu_unit: [f64; 3],
    pub elevation_deg: f64,
    pub azimuth_deg: f64,
    pub illumination_fraction: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InsolationResult {
    pub direct_w_m2: f64,
    pub diffuse_w_m2: f64,
    pub global_w_m2: f64,
    pub incidence_cos: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeplerElements {
    pub semi_major_axis_m: f64,
    pub eccentricity: f64,
    pub inclination_deg: f64,
    pub raan_deg: f64,
    pub arg_perigee_deg: f64,
    pub mean_anomaly_epoch_deg: f64,
    pub epoch_unix_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitalStateEcef {
    pub pos_ecef_m: [f64; 3],
    pub vel_ecef_m_s: [f64; 3],
    pub subpoint_lat_deg: f64,
    pub subpoint_lon_deg: f64,
    pub altitude_m: f64,
}

#[inline]
fn ecef_to_enu_unit(ecef: [f64; 3], lat_deg: f64, lon_deg: f64) -> [f64; 3] {
    let (sin_lat, cos_lat) = (lat_deg * DEG_TO_RAD).sin_cos();
    let (sin_lon, cos_lon) = (lon_deg * DEG_TO_RAD).sin_cos();
    [
        -sin_lon * ecef[0] + cos_lon * ecef[1],
        -sin_lat * cos_lon * ecef[0] - sin_lat * sin_lon * ecef[1] + cos_lat * ecef[2],
        cos_lat * cos_lon * ecef[0] + cos_lat * sin_lon * ecef[1] + sin_lat * ecef[2],
    ]
}

#[inline]
pub fn unix_to_julian_date(unix_secs: f64) -> f64 {
    unix_secs / SECONDS_PER_DAY + JD_UNIX_EPOCH
}

/// Earth Rotation Angle (IAU 2000/2006) in radians in [0, 2π).
#[inline]
pub fn earth_rotation_angle_rad(jd_ut1: f64) -> f64 {
    let d = jd_ut1 - JD_J2000;
    (TAU * (0.7790572732640 + 1.00273781191135448 * d)).rem_euclid(TAU)
}

/// Analytical solar ephemeris providing geocentric and topocentric solar position.
pub fn solar_ephemeris(unix_secs: f64, lat_deg: f64, lon_deg: f64) -> SolarPosition {
    let jd = unix_to_julian_date(unix_secs);
    let t = (jd - JD_J2000) / 36_525.0;

    let l0 = (280.46646 + 36000.76983 * t + 0.0003032 * t * t).rem_euclid(360.0) * DEG_TO_RAD;
    let m = (357.52911 + 35999.05029 * t - 0.0001537 * t * t).rem_euclid(360.0) * DEG_TO_RAD;
    let e = 0.016708634 - 0.000042037 * t - 0.0000001267 * t * t;

    let c = ((1.914602 - 0.004817 * t) * m.sin()
        + (0.019993 - 0.000101 * t) * (2.0 * m).sin()
        + 0.000289 * (3.0 * m).sin()) * DEG_TO_RAD;
    let (true_long, true_anom) = (l0 + c, m + c);
    let distance_au = (1.000001018 * (1.0 - e * e)) / (1.0 + e * true_anom.cos());

    let (sin_eps, cos_eps) = ((23.439291 - 0.0130042 * t) * DEG_TO_RAD).sin_cos();
    let (sin_tl, cos_tl) = true_long.sin_cos();

    let sin_dec = sin_eps * sin_tl;
    let dec_rad = sin_dec.clamp(-1.0, 1.0).asin();
    let ra_rad = (cos_eps * sin_tl).atan2(cos_tl).rem_euclid(TAU);

    let (sin_ra, cos_ra) = ra_rad.sin_cos();
    let cos_dec = dec_rad.cos();
    let (x_eci, y_eci, z_eci) = (cos_dec * cos_ra, cos_dec * sin_ra, sin_dec);

    let (sin_era, cos_era) = earth_rotation_angle_rad(jd).sin_cos();
    let ecef_unit = [cos_era * x_eci + sin_era * y_eci, -sin_era * x_eci + cos_era * y_eci, z_eci];

    let enu_unit = ecef_to_enu_unit(ecef_unit, lat_deg, lon_deg);
    let elevation_deg = enu_unit[2].clamp(-1.0, 1.0).asin() * RAD_TO_DEG;
    let azimuth_deg = enu_unit[0].atan2(enu_unit[1]).rem_euclid(TAU) * RAD_TO_DEG;

    SolarPosition {
        ecef_unit,
        enu_unit,
        elevation_deg,
        azimuth_deg,
        declination_deg: dec_rad * RAD_TO_DEG,
        right_ascension_deg: ra_rad * RAD_TO_DEG,
        distance_au,
    }
}

/// Analytical lunar ephemeris providing topocentric position and illuminated fraction.
pub fn lunar_ephemeris(unix_secs: f64, lat_deg: f64, lon_deg: f64) -> LunarPosition {
    let jd = unix_to_julian_date(unix_secs);
    let t = (jd - JD_J2000) / 36_525.0;

    let lp = (218.3164477 + 481267.8812342 * t).rem_euclid(360.0) * DEG_TO_RAD;
    let d = (297.8501921 + 445267.1114034 * t).rem_euclid(360.0) * DEG_TO_RAD;
    let m = (357.5291092 + 35999.0502909 * t).rem_euclid(360.0) * DEG_TO_RAD;
    let mp = (134.9633964 + 477198.8675055 * t).rem_euclid(360.0) * DEG_TO_RAD;
    let f = (93.2720950 + 483202.0175233 * t).rem_euclid(360.0) * DEG_TO_RAD;

    let dl = (6.288774 * mp.sin() + 1.274027 * (2.0 * d - mp).sin() + 0.658314 * (2.0 * d).sin()
        + 0.213618 * (2.0 * mp).sin() - 0.185116 * m.sin() - 0.114332 * (2.0 * f).sin()
        + 0.058793 * (2.0 * (d - mp)).sin() + 0.057066 * (2.0 * d - m - mp).sin()
        + 0.053322 * (2.0 * d + mp).sin()) * DEG_TO_RAD;
    let lambda_m = lp + dl;

    let db = (5.128154 * f.sin() + 0.280606 * (mp + f).sin() + 0.277693 * (mp - f).sin()
        + 0.173237 * (2.0 * d - f).sin() + 0.055413 * (2.0 * d - mp + f).sin()
        + 0.046271 * (2.0 * d - mp - f).sin()) * DEG_TO_RAD;
    let beta_m = db;

    let (sin_eps, cos_eps) = ((23.439291 - 0.0130042 * t) * DEG_TO_RAD).sin_cos();
    let (sin_bm, cos_bm) = beta_m.sin_cos();
    let (sin_lm, cos_lm) = lambda_m.sin_cos();

    let sin_dec = sin_bm * cos_eps + cos_bm * sin_eps * sin_lm;
    let dec_rad = sin_dec.clamp(-1.0, 1.0).asin();
    let ra_rad = (cos_bm * cos_eps * sin_lm - sin_bm * sin_eps).atan2(cos_bm * cos_lm).rem_euclid(TAU);

    let (sin_ra, cos_ra) = ra_rad.sin_cos();
    let cos_dec = dec_rad.cos();
    let (x_eci, y_eci, z_eci) = (cos_dec * cos_ra, cos_dec * sin_ra, sin_dec);

    let (sin_era, cos_era) = earth_rotation_angle_rad(jd).sin_cos();
    let ecef_unit = [cos_era * x_eci + sin_era * y_eci, -sin_era * x_eci + cos_era * y_eci, z_eci];

    let enu_unit = ecef_to_enu_unit(ecef_unit, lat_deg, lon_deg);
    let elevation_deg = enu_unit[2].clamp(-1.0, 1.0).asin() * RAD_TO_DEG;
    let azimuth_deg = enu_unit[0].atan2(enu_unit[1]).rem_euclid(TAU) * RAD_TO_DEG;
    let illumination_fraction = ((1.0 - d.cos()) * 0.5).clamp(0.0, 1.0);

    LunarPosition {
        ecef_unit,
        enu_unit,
        elevation_deg,
        azimuth_deg,
        illumination_fraction,
    }
}

/// Topographic solar irradiance accounting for atmospheric attenuation, elevation, slope, and aspect.
pub fn topographic_solar_irradiance(
    unix_secs: f64,
    lat_deg: f64,
    lon_deg: f64,
    alt_m: f64,
    slope_deg: f64,
    aspect_deg: f64,
    atmospheric_transmittance: f64,
) -> InsolationResult {
    let sol = solar_ephemeris(unix_secs, lat_deg, lon_deg);
    if sol.elevation_deg <= 0.0 {
        return InsolationResult { direct_w_m2: 0.0, diffuse_w_m2: 0.0, global_w_m2: 0.0, incidence_cos: 0.0 };
    }

    let i0 = SOLAR_CONSTANT_W_M2 / (sol.distance_au * sol.distance_au);
    let (elev_deg, elev_rad) = (sol.elevation_deg, sol.elevation_deg * DEG_TO_RAD);

    // Kasten-Young (1989) optical air mass with pressure adjustment for altitude
    let am_rel = 1.0 / (elev_rad.sin() + 0.50572 * (elev_deg + 6.07995).powf(-1.6364));
    let p_ratio = (-alt_m / 8434.5).exp();
    let am = (am_rel * p_ratio).max(0.2);

    let tau = atmospheric_transmittance.clamp(0.05, 0.98);
    let idn = i0 * tau.powf(am);

    // Surface normal in ENU (aspect: 0 = North, 90 = East, 180 = South, 270 = West)
    let (sin_s, cos_s) = (slope_deg * DEG_TO_RAD).sin_cos();
    let (sin_a, cos_a) = (aspect_deg * DEG_TO_RAD).sin_cos();
    let (norm_e, norm_n, norm_u) = (sin_s * sin_a, sin_s * cos_a, cos_s);

    let cos_inc = (sol.enu_unit[0] * norm_e + sol.enu_unit[1] * norm_n + sol.enu_unit[2] * norm_u).max(0.0);
    let direct = idn * cos_inc;

    // Diffuse horizontal irradiance and sky view factor
    let id_horiz = i0 * elev_rad.sin() * (0.271 - 0.294 * tau.powf(am)).max(0.04);
    let diffuse = id_horiz * (0.5 * (1.0 + cos_s));

    InsolationResult {
        direct_w_m2: direct,
        diffuse_w_m2: diffuse,
        global_w_m2: direct + diffuse,
        incidence_cos: cos_inc,
    }
}

/// 24-step integration of solar irradiance over 24 hours in Wh/m² (default clear-sky transmittance 0.75).
pub fn daily_integrated_insolation_wh_m2(
    day_start_unix_secs: f64,
    lat_deg: f64,
    lon_deg: f64,
    alt_m: f64,
    slope_deg: f64,
    aspect_deg: f64,
) -> f64 {
    let mut total_wh = 0.0;
    for i in 0..24 {
        let t = day_start_unix_secs + (i as f64 + 0.5) * 3600.0;
        let res = topographic_solar_irradiance(t, lat_deg, lon_deg, alt_m, slope_deg, aspect_deg, 0.75);
        total_wh += res.global_w_m2; // dt is 1.0 hour
    }
    total_wh
}

/// Propagate Keplerian elements under J2 gravitational secular perturbations to target ECEF state.
pub fn propagate_orbit_j2(elements: &KeplerElements, target_unix_secs: f64) -> OrbitalStateEcef {
    let a = elements.semi_major_axis_m.max(WGS84_A + 10_000.0);
    let e = elements.eccentricity.clamp(0.0, 0.9999);
    let dt = target_unix_secs - elements.epoch_unix_s;

    let n0 = (GM_EARTH / (a * a * a)).sqrt();
    let p = a * (1.0 - e * e);
    let eta = (1.0 - e * e).sqrt();
    let p_ratio = WGS84_A / p;
    let factor = 1.5 * J2_EARTH * p_ratio * p_ratio * n0;

    let (sin_i, cos_i) = (elements.inclination_deg * DEG_TO_RAD).sin_cos();
    let raan_dot = -factor * cos_i;
    let arg_per_dot = 0.5 * factor * (5.0 * cos_i * cos_i - 1.0);
    let mean_motion_bar = n0 * (1.0 + 0.5 * factor / n0 * eta * (3.0 * cos_i * cos_i - 1.0));

    let raan = (elements.raan_deg * DEG_TO_RAD + raan_dot * dt).rem_euclid(TAU);
    let arg_per = (elements.arg_perigee_deg * DEG_TO_RAD + arg_per_dot * dt).rem_euclid(TAU);
    let m = (elements.mean_anomaly_epoch_deg * DEG_TO_RAD + mean_motion_bar * dt).rem_euclid(TAU);

    // Solve Kepler's equation via Newton-Raphson
    let mut ecc_anom = m + e * m.sin();
    for _ in 0..10 {
        let delta = (ecc_anom - e * ecc_anom.sin() - m) / (1.0 - e * ecc_anom.cos());
        ecc_anom -= delta;
        if delta.abs() < 1e-12 { break; }
    }

    let (sin_e, cos_e) = ecc_anom.sin_cos();
    let r = a * (1.0 - e * cos_e);
    let (x_orb, y_orb) = (a * (cos_e - e), a * eta * sin_e);

    let v_factor = (GM_EARTH * a).sqrt() / r;
    let (vx_orb, vy_orb) = (-v_factor * sin_e, v_factor * eta * cos_e);

    let (sin_w, cos_w) = arg_per.sin_cos();
    let (sin_o, cos_o) = raan.sin_cos();

    let (px, py, pz) = (cos_o * cos_w - sin_o * sin_w * cos_i, sin_o * cos_w + cos_o * sin_w * cos_i, sin_w * sin_i);
    let (qx, qy, qz) = (-cos_o * sin_w - sin_o * cos_w * cos_i, -sin_o * sin_w + cos_o * cos_w * cos_i, cos_w * sin_i);

    let (x_eci, y_eci, z_eci) = (x_orb * px + y_orb * qx, x_orb * py + y_orb * qy, x_orb * pz + y_orb * qz);
    let (vx_eci, vy_eci, vz_eci) = (vx_orb * px + vy_orb * qx, vx_orb * py + vy_orb * qy, vx_orb * pz + vy_orb * qz);

    let jd = unix_to_julian_date(target_unix_secs);
    let (sin_era, cos_era) = earth_rotation_angle_rad(jd).sin_cos();

    let (x_ecef, y_ecef, z_ecef) = (cos_era * x_eci + sin_era * y_eci, -sin_era * x_eci + cos_era * y_eci, z_eci);
    let (vx_rot, vy_rot, vz_rot) = (cos_era * vx_eci + sin_era * vy_eci, -sin_era * vx_eci + cos_era * vy_eci, vz_eci);
    let (vx_ecef, vy_ecef, vz_ecef) = (vx_rot + OMEGA_EARTH * y_ecef, vy_rot - OMEGA_EARTH * x_ecef, vz_rot);

    // Bowring's geodetic conversion (ECEF -> WGS84 lat, lon, altitude)
    let p_dist = (x_ecef * x_ecef + y_ecef * y_ecef).sqrt();
    let lon_deg = y_ecef.atan2(x_ecef) * RAD_TO_DEG;
    let theta = (z_ecef * WGS84_A).atan2(p_dist * WGS84_B);
    let (sin_th, cos_th) = theta.sin_cos();
    let lat_rad = (z_ecef + WGS84_EP2 * WGS84_B * sin_th.powi(3))
        .atan2(p_dist - WGS84_E2 * WGS84_A * cos_th.powi(3));
    let sin_lat = lat_rad.sin();
    let n_val = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    let alt_m = if lat_rad.cos().abs() > 1e-4 {
        p_dist / lat_rad.cos() - n_val
    } else {
        z_ecef.abs() / sin_lat.abs() - n_val * (1.0 - WGS84_E2)
    };

    OrbitalStateEcef {
        pos_ecef_m: [x_ecef, y_ecef, z_ecef],
        vel_ecef_m_s: [vx_ecef, vy_ecef, vz_ecef],
        subpoint_lat_deg: lat_rad * RAD_TO_DEG,
        subpoint_lon_deg: lon_deg,
        altitude_m: alt_m,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solar_ephemeris_kosciuszko() {
        let (lat, lon) = (-36.4558, 148.2636);
        let noon_utc = 1_700_014_020.0;
        let midnight_utc = noon_utc + 43_200.0;

        let sol_noon = solar_ephemeris(noon_utc, lat, lon);
        let sol_mid = solar_ephemeris(midnight_utc, lat, lon);

        assert!(sol_noon.elevation_deg > 30.0, "Expected positive noon elevation: {}", sol_noon.elevation_deg);
        assert!(sol_mid.elevation_deg < -30.0, "Expected negative midnight elevation: {}", sol_mid.elevation_deg);

        let norm_ecef = (sol_noon.ecef_unit[0].powi(2) + sol_noon.ecef_unit[1].powi(2) + sol_noon.ecef_unit[2].powi(2)).sqrt();
        let norm_enu = (sol_noon.enu_unit[0].powi(2) + sol_noon.enu_unit[1].powi(2) + sol_noon.enu_unit[2].powi(2)).sqrt();
        assert!((norm_ecef - 1.0).abs() < 1e-7 && (norm_enu - 1.0).abs() < 1e-7);
        assert!(sol_noon.distance_au > 0.98 && sol_noon.distance_au < 1.02);
    }

    #[test]
    fn test_topographic_solar_irradiance_slopes() {
        let (lat, lon, alt) = (-36.4558, 148.2636, 2228.0);
        let noon_utc = 1_700_014_020.0; // Mid-November (late spring)
        let winter_noon_utc = 1_687_312_800.0; // Austral winter solstice (June 21)

        let res_north = topographic_solar_irradiance(noon_utc, lat, lon, alt, 30.0, 0.0, 0.75);
        let res_south = topographic_solar_irradiance(noon_utc, lat, lon, alt, 30.0, 180.0, 0.75);
        assert!(res_north.direct_w_m2 > res_south.direct_w_m2 * 1.4);
        assert!(res_north.global_w_m2 > res_south.global_w_m2 * 1.35);

        let win_north = topographic_solar_irradiance(winter_noon_utc, lat, lon, alt, 30.0, 0.0, 0.75);
        let win_south = topographic_solar_irradiance(winter_noon_utc, lat, lon, alt, 30.0, 180.0, 0.75);
        assert!(win_north.global_w_m2 > win_south.global_w_m2 * 1.8);

        let day_start = winter_noon_utc - 12.0 * 3600.0;
        let daily_north = daily_integrated_insolation_wh_m2(day_start, lat, lon, alt, 30.0, 0.0);
        let daily_south = daily_integrated_insolation_wh_m2(day_start, lat, lon, alt, 30.0, 180.0);

        assert!(daily_north > daily_south * 1.5);
    }

    #[test]
    fn test_propagate_orbit_j2_sentinel2() {
        let elements = KeplerElements {
            semi_major_axis_m: 7_167_000.0,
            eccentricity: 0.0001,
            inclination_deg: 98.56,
            raan_deg: 45.0,
            arg_perigee_deg: 20.0,
            mean_anomaly_epoch_deg: 0.0,
            epoch_unix_s: 1_700_000_000.0,
        };

        let state_t0 = propagate_orbit_j2(&elements, elements.epoch_unix_s);
        let state_t1 = propagate_orbit_j2(&elements, elements.epoch_unix_s + 1500.0);

        assert!(state_t0.altitude_m > 760_000.0 && state_t0.altitude_m < 815_000.0);
        assert!(state_t1.altitude_m > 760_000.0 && state_t1.altitude_m < 815_000.0);

        let v0_mag = (state_t0.vel_ecef_m_s[0].powi(2) + state_t0.vel_ecef_m_s[1].powi(2) + state_t0.vel_ecef_m_s[2].powi(2)).sqrt();
        assert!(v0_mag > 6_800.0 && v0_mag < 8_000.0);

        assert!(state_t0.subpoint_lat_deg.abs() <= 82.0 && state_t1.subpoint_lat_deg.abs() <= 82.0);

        let state_1day = propagate_orbit_j2(&elements, elements.epoch_unix_s + 86400.0);
        assert!(state_1day.pos_ecef_m[0].is_finite());
    }

    #[test]
    fn test_lunar_ephemeris_and_time() {
        let jd = unix_to_julian_date(1_700_000_000.0);
        assert!((jd - 2_460_263.4259).abs() < 0.01);

        let era = earth_rotation_angle_rad(jd);
        assert!(era >= 0.0 && era < TAU);

        let moon = lunar_ephemeris(1_700_000_000.0, -36.4558, 148.2636);
        assert!(moon.illumination_fraction >= 0.0 && moon.illumination_fraction <= 1.0);
        let norm = (moon.ecef_unit[0].powi(2) + moon.ecef_unit[1].powi(2) + moon.ecef_unit[2].powi(2)).sqrt();
        assert!((norm - 1.0).abs() < 1e-7);
    }
}
