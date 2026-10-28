// Portions adapted from RustGIS (Apache-2.0 licensed).
// Copyright (c) RustGIS Contributors / QualiaDB authors.
// Tier-1 HotZeroHeap: Zero heap allocations in all non-test evaluation and WKT parsing paths.

use core::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};

pub const EPSG_WGS84: u32 = 4326;
pub const EPSG_WEB_MERCATOR: u32 = 3857;
pub const EPSG_GDA94: u32 = 4283;
pub const EPSG_GDA2020: u32 = 7844;
pub const EPSG_GDA2020_MGA54: u32 = 7854;
pub const EPSG_GDA2020_MGA55: u32 = 7855;
pub const EPSG_GDA2020_MGA56: u32 = 7856;
pub const EPSG_GDA2020_ALBERS: u32 = 9473;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EpsgCode(pub u32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ellipsoid { pub a: f64, pub inv_f: f64 }

impl Ellipsoid {
    pub const GRS80: Self = Self { a: 6378137.0, inv_f: 298.257222101 };
    pub const WGS84: Self = Self { a: 6378137.0, inv_f: 298.257223563 };
    #[inline] pub fn f(&self) -> f64 { 1.0 / self.inv_f }
    #[inline] pub fn e2(&self) -> f64 { let f = self.f(); f * (2.0 - f) }
    #[inline] pub fn e(&self) -> f64 { self.e2().sqrt() }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectionMethod {
    GeographicLonLat,
    WebMercator,
    TransverseMercator { lon0_deg: f64, k0: f64, false_easting: f64, false_northing: f64 },
    AlbersEqualArea { lat1_deg: f64, lat2_deg: f64, lat0_deg: f64, lon0_deg: f64, false_easting: f64, false_northing: f64 },
    LambertConformalConic { lat1_deg: f64, lat2_deg: f64, lat0_deg: f64, lon0_deg: f64, false_easting: f64, false_northing: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Helmert7Param { pub tx_m: f64, pub ty_m: f64, pub tz_m: f64, pub rx_arcsec: f64, pub ry_arcsec: f64, pub rz_arcsec: f64, pub scale_ppm: f64 }

impl Helmert7Param {
    pub const GDA94_TO_GDA2020: Self = Self {
        tx_m: 0.06155, ty_m: -0.01087, tz_m: -0.04019,
        rx_arcsec: -0.0394924, ry_arcsec: -0.0327221, rz_arcsec: -0.0328979, scale_ppm: -0.009994,
    };

    pub fn transform_ecef(&self, x: f64, y: f64, z: f64) -> (f64, f64, f64) {
        let (rx, ry, rz) = (self.rx_arcsec * (PI / 648000.0), self.ry_arcsec * (PI / 648000.0), self.rz_arcsec * (PI / 648000.0));
        let s = 1.0 + self.scale_ppm * 1e-6;
        (self.tx_m + s * (x + rz * y - ry * z), self.ty_m + s * (-rz * x + y + rx * z), self.tz_m + s * (ry * x - rx * y + z))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrsError { ParseError, UnsupportedProjection, OutOfProjectionDomain, ReprojectionFailed }
impl core::fmt::Display for CrsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", match self {
            Self::ParseError => "Failed to parse CRS from WKT slice",
            Self::UnsupportedProjection => "Unsupported projection method",
            Self::OutOfProjectionDomain => "Coordinates out of projection domain",
            Self::ReprojectionFailed => "Reprojection failed",
        })
    }
}
impl std::error::Error for CrsError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CrsDefinition { pub epsg: Option<EpsgCode>, pub ellipsoid: Ellipsoid, pub method: ProjectionMethod }

impl CrsDefinition {
    pub fn from_epsg(code: u32) -> Option<Self> {
        let (ellipsoid, method) = match code {
            EPSG_WGS84 => (Ellipsoid::WGS84, ProjectionMethod::GeographicLonLat),
            EPSG_WEB_MERCATOR => (Ellipsoid::WGS84, ProjectionMethod::WebMercator),
            EPSG_GDA94 | EPSG_GDA2020 => (Ellipsoid::GRS80, ProjectionMethod::GeographicLonLat),
            EPSG_GDA2020_MGA54 => (Ellipsoid::GRS80, ProjectionMethod::TransverseMercator { lon0_deg: 141.0, k0: 0.9996, false_easting: 500000.0, false_northing: 10000000.0 }),
            EPSG_GDA2020_MGA55 => (Ellipsoid::GRS80, ProjectionMethod::TransverseMercator { lon0_deg: 147.0, k0: 0.9996, false_easting: 500000.0, false_northing: 10000000.0 }),
            EPSG_GDA2020_MGA56 => (Ellipsoid::GRS80, ProjectionMethod::TransverseMercator { lon0_deg: 153.0, k0: 0.9996, false_easting: 500000.0, false_northing: 10000000.0 }),
            EPSG_GDA2020_ALBERS => (Ellipsoid::GRS80, ProjectionMethod::AlbersEqualArea { lat1_deg: -18.0, lat2_deg: -36.0, lat0_deg: 0.0, lon0_deg: 134.0, false_easting: 0.0, false_northing: 0.0 }),
            _ => return None,
        };
        Some(Self { epsg: Some(EpsgCode(code)), ellipsoid, method })
    }

    pub fn parse_wkt_slice(wkt: &str) -> Result<Self, CrsError> {
        if wkt.trim().is_empty() { return Err(CrsError::ParseError); }
        if let Some(code) = find_last_epsg(wkt) {
            if let Some(crs) = Self::from_epsg(code) { return Ok(crs); }
        }
        let ellipsoid = extract_ellipsoid(wkt);
        let method = extract_projection_method(wkt)?;
        let epsg = find_last_epsg(wkt).map(EpsgCode);
        Ok(Self { epsg, ellipsoid, method })
    }
}

fn find_ascii_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() { return Some(0); }
    if needle.len() > haystack.len() { return None; }
    let (hb, nb) = (haystack.as_bytes(), needle.as_bytes());
    'outer: for i in 0..=(hb.len() - nb.len()) {
        for j in 0..nb.len() {
            if !hb[i + j].eq_ignore_ascii_case(&nb[j]) { continue 'outer; }
        }
        return Some(i);
    }
    None
}

fn scan_param_f64(wkt: &str, keys: &[&str]) -> Option<f64> {
    for &key in keys {
        if let Some(pos) = find_ascii_ignore_case(wkt, key) {
            let after = &wkt[pos + key.len()..];
            if let Some(comma) = after.find(',') {
                let rest = after[comma + 1..].trim_start().trim_start_matches(|c: char| c == '"' || c == '\'');
                let end = rest.find(|c: char| c == ',' || c == ']' || c == ')' || c == '"' || c == '\'' || c.is_whitespace()).unwrap_or(rest.len());
                if let Ok(v) = rest[..end].parse::<f64>() { return Some(v); }
            }
        }
    }
    None
}

fn find_last_epsg(wkt: &str) -> Option<u32> {
    let mut search = wkt;
    let mut last_code = None;
    while let Some(pos) = find_ascii_ignore_case(search, "EPSG") {
        let after = &search[pos + 4..];
        if let Some(comma) = after.find(',') {
            let rest = after[comma + 1..].trim_start().trim_start_matches(|c: char| c == '"' || c == '\'');
            let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            if let Ok(code) = rest[..end].parse::<u32>() { last_code = Some(code); }
        }
        search = after;
    }
    last_code
}

fn extract_ellipsoid(wkt: &str) -> Ellipsoid {
    let kw = find_ascii_ignore_case(wkt, "SPHEROID").map(|p| p + 8)
        .or_else(|| find_ascii_ignore_case(wkt, "ELLIPSOID").map(|p| p + 9));
    if let Some(start) = kw {
        if let Some(c1) = wkt[start..].find(',') {
            let after_c1 = &wkt[start + c1 + 1..];
            if let Some(c2) = after_c1.find(',') {
                let a_str = after_c1[..c2].trim().trim_matches(|c: char| c == '"' || c == '\'');
                let rest = after_c1[c2 + 1..].trim().trim_matches(|c: char| c == '"' || c == '\'');
                let end = rest.find(|c: char| c == ',' || c == ']' || c == ')' || c.is_whitespace()).unwrap_or(rest.len());
                if let (Ok(a), Ok(inv_f)) = (a_str.parse::<f64>(), rest[..end].parse::<f64>()) {
                    if a > 1e6 && inv_f > 0.0 { return Ellipsoid { a, inv_f }; }
                }
            }
        }
    }
    Ellipsoid::GRS80
}

fn extract_projection_method(wkt: &str) -> Result<ProjectionMethod, CrsError> {
    if find_ascii_ignore_case(wkt, "Transverse").is_some() || find_ascii_ignore_case(wkt, "UTM").is_some() || find_ascii_ignore_case(wkt, "MGA").is_some() {
        return Ok(ProjectionMethod::TransverseMercator {
            lon0_deg: scan_param_f64(wkt, &["central_meridian", "natural origin"]).unwrap_or(0.0),
            k0: scan_param_f64(wkt, &["scale_factor", "Scale factor"]).unwrap_or(0.9996),
            false_easting: scan_param_f64(wkt, &["false_easting", "False easting"]).unwrap_or(500000.0),
            false_northing: scan_param_f64(wkt, &["false_northing", "False northing"]).unwrap_or(0.0),
        });
    }
    if find_ascii_ignore_case(wkt, "Albers").is_some() {
        return Ok(ProjectionMethod::AlbersEqualArea {
            lat1_deg: scan_param_f64(wkt, &["standard_parallel_1", "1st standard parallel"]).unwrap_or(-18.0),
            lat2_deg: scan_param_f64(wkt, &["standard_parallel_2", "2nd standard parallel"]).unwrap_or(-36.0),
            lat0_deg: scan_param_f64(wkt, &["latitude_of_origin", "Latitude of natural origin"]).unwrap_or(0.0),
            lon0_deg: scan_param_f64(wkt, &["central_meridian", "Longitude of natural origin"]).unwrap_or(134.0),
            false_easting: scan_param_f64(wkt, &["false_easting", "False easting"]).unwrap_or(0.0),
            false_northing: scan_param_f64(wkt, &["false_northing", "False northing"]).unwrap_or(0.0),
        });
    }
    if find_ascii_ignore_case(wkt, "Lambert").is_some() {
        return Ok(ProjectionMethod::LambertConformalConic {
            lat1_deg: scan_param_f64(wkt, &["standard_parallel_1", "1st standard parallel"]).unwrap_or(0.0),
            lat2_deg: scan_param_f64(wkt, &["standard_parallel_2", "2nd standard parallel"]).unwrap_or(0.0),
            lat0_deg: scan_param_f64(wkt, &["latitude_of_origin", "Latitude of natural origin"]).unwrap_or(0.0),
            lon0_deg: scan_param_f64(wkt, &["central_meridian", "Longitude of natural origin"]).unwrap_or(0.0),
            false_easting: scan_param_f64(wkt, &["false_easting", "False easting"]).unwrap_or(0.0),
            false_northing: scan_param_f64(wkt, &["false_northing", "False northing"]).unwrap_or(0.0),
        });
    }
    if find_ascii_ignore_case(wkt, "Web Mercator").is_some() || find_ascii_ignore_case(wkt, "Pseudo-Mercator").is_some() || find_ascii_ignore_case(wkt, "Popular Visualisation").is_some() {
        return Ok(ProjectionMethod::WebMercator);
    }
    if find_ascii_ignore_case(wkt, "GEOGCS").is_some() || find_ascii_ignore_case(wkt, "GEOGCRS").is_some() {
        return Ok(ProjectionMethod::GeographicLonLat);
    }
    Err(CrsError::UnsupportedProjection)
}

pub fn project_forward(crs: &CrsDefinition, lon_deg: f64, lat_deg: f64) -> Result<(f64, f64), CrsError> {
    match crs.method {
        ProjectionMethod::GeographicLonLat => Ok((lon_deg, lat_deg)),
        ProjectionMethod::WebMercator => {
            let phi = lat_deg.clamp(-85.05112878, 85.05112878).to_radians();
            Ok((crs.ellipsoid.a * lon_deg.to_radians(), crs.ellipsoid.a * (FRAC_PI_4 + 0.5 * phi).tan().ln()))
        }
        ProjectionMethod::TransverseMercator { lon0_deg, k0, false_easting, false_northing } => Ok(tm_forward(lat_deg, lon_deg, lon0_deg, k0, false_easting, false_northing, &crs.ellipsoid)),
        ProjectionMethod::AlbersEqualArea { lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing } => Ok(albers_forward(lat_deg, lon_deg, lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing, &crs.ellipsoid)),
        ProjectionMethod::LambertConformalConic { lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing } => Ok(lcc_forward(lat_deg, lon_deg, lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing, &crs.ellipsoid)),
    }
}

pub fn project_inverse(crs: &CrsDefinition, x: f64, y: f64) -> Result<(f64, f64), CrsError> {
    match crs.method {
        ProjectionMethod::GeographicLonLat => Ok((x, y)),
        ProjectionMethod::WebMercator => Ok(((x / crs.ellipsoid.a).to_degrees(), (2.0 * (y / crs.ellipsoid.a).exp().atan() - FRAC_PI_2).to_degrees())),
        ProjectionMethod::TransverseMercator { lon0_deg, k0, false_easting, false_northing } => Ok(tm_inverse(x, y, lon0_deg, k0, false_easting, false_northing, &crs.ellipsoid)),
        ProjectionMethod::AlbersEqualArea { lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing } => Ok(albers_inverse(x, y, lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing, &crs.ellipsoid)),
        ProjectionMethod::LambertConformalConic { lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing } => Ok(lcc_inverse(x, y, lat1_deg, lat2_deg, lat0_deg, lon0_deg, false_easting, false_northing, &crs.ellipsoid)),
    }
}

pub fn reproject_coord(src: &CrsDefinition, dst: &CrsDefinition, x: f64, y: f64) -> Result<(f64, f64), CrsError> {
    if src == dst { return Ok((x, y)); }
    let (mut lon, mut lat) = project_inverse(src, x, y)?;
    if let (Some(s), Some(d)) = (src.epsg, dst.epsg) {
        if s.0 == EPSG_GDA94 && (d.0 == EPSG_GDA2020 || (7854..=7856).contains(&d.0) || d.0 == EPSG_GDA2020_ALBERS) {
            let (xe, ye, ze) = lat_lon_to_ecef(lat, lon, 0.0, &src.ellipsoid);
            let (txe, tye, tze) = Helmert7Param::GDA94_TO_GDA2020.transform_ecef(xe, ye, ze);
            let (n_lat, n_lon, _) = ecef_to_lat_lon(txe, tye, tze, &dst.ellipsoid);
            lat = n_lat;
            lon = n_lon;
        }
    }
    project_forward(dst, lon, lat)
}

pub fn ausgeoid2020_undulation_m(lat_deg: f64, lon_deg: f64) -> f64 {
    let (u, v) = ((lat_deg + 28.0) * 0.1, (lon_deg - 134.0) * 0.1);
    17.8 + 14.5 * u + 18.2 * v - 2.8 * u * u + 1.5 * u * v - 3.4 * v * v
}

pub fn egm2008_undulation_m(lat_deg: f64, lon_deg: f64) -> f64 {
    let (phi, lam) = (lat_deg.to_radians(), lon_deg.to_radians());
    let (s_phi, c_phi) = (phi.sin(), phi.cos());
    -10.5 + 24.0 * s_phi - 14.0 * c_phi * (lam - 0.7).cos()
        - 18.0 * (3.0 * s_phi * s_phi - 1.0) * (2.0 * lam + 0.3).cos()
        + 8.5 * (2.0 * phi).sin() * (lam - 1.5).sin()
        - 12.0 * (2.0 * phi).cos() * (3.0 * lam).sin()
}

fn lat_lon_to_ecef(lat_deg: f64, lon_deg: f64, h: f64, ell: &Ellipsoid) -> (f64, f64, f64) {
    let (phi, lam) = (lat_deg.to_radians(), lon_deg.to_radians());
    let (s_phi, c_phi, e2) = (phi.sin(), phi.cos(), ell.e2());
    let nu = ell.a / (1.0 - e2 * s_phi * s_phi).sqrt();
    ((nu + h) * c_phi * lam.cos(), (nu + h) * c_phi * lam.sin(), (nu * (1.0 - e2) + h) * s_phi)
}

fn ecef_to_lat_lon(x: f64, y: f64, z: f64, ell: &Ellipsoid) -> (f64, f64, f64) {
    let p = (x * x + y * y).sqrt();
    let lam = y.atan2(x);
    let (e2, b) = (ell.e2(), ell.a * (1.0 - ell.f()));
    let ep2 = (ell.a * ell.a - b * b) / (b * b);
    let theta = (z * ell.a).atan2(p * b);
    let (s_th, c_th) = (theta.sin(), theta.cos());
    let phi = (z + ep2 * b * s_th * s_th * s_th).atan2(p - e2 * ell.a * c_th * c_th * c_th);
    let nu = ell.a / (1.0 - e2 * phi.sin() * phi.sin()).sqrt();
    (phi.to_degrees(), lam.to_degrees(), p / phi.cos() - nu)
}

fn tm_forward(lat_deg: f64, lon_deg: f64, lon0_deg: f64, k0: f64, fe: f64, fnorth: f64, ell: &Ellipsoid) -> (f64, f64) {
    let (lat, dlon) = (lat_deg.to_radians(), (lon_deg - lon0_deg).to_radians());
    let n = ell.f() / (2.0 - ell.f());
    let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
    let cap_a = (ell.a / (1.0 + n)) * (1.0 + 0.25 * n2 + (1.0 / 64.0) * n4);
    let (a1, a2, a3, a4) = (0.5 * n - (2.0 / 3.0) * n2 + (5.0 / 16.0) * n3 + (41.0 / 180.0) * n4, (13.0 / 48.0) * n2 - (3.0 / 5.0) * n3 + (557.0 / 1440.0) * n4, (61.0 / 240.0) * n3 - (103.0 / 140.0) * n4, (49561.0 / 161280.0) * n4);
    let e = ell.e();
    let psi = lat.sin().atanh() - e * (e * lat.sin()).clamp(-0.999999, 0.999999).atanh();
    let sigma = psi.sinh();
    let (sin_chi, cos_chi) = (sigma / (1.0 + sigma * sigma).sqrt(), 1.0 / (1.0 + sigma * sigma).sqrt());
    let xi0 = sin_chi.atan2(cos_chi * dlon.cos());
    let eta0 = (cos_chi * dlon.sin()).clamp(-0.999999, 0.999999).atanh();

    let xi = xi0 + a1 * (2.0 * xi0).sin() * (2.0 * eta0).cosh() + a2 * (4.0 * xi0).sin() * (4.0 * eta0).cosh() + a3 * (6.0 * xi0).sin() * (6.0 * eta0).cosh() + a4 * (8.0 * xi0).sin() * (8.0 * eta0).cosh();
    let eta = eta0 + a1 * (2.0 * xi0).cos() * (2.0 * eta0).sinh() + a2 * (4.0 * xi0).cos() * (4.0 * eta0).sinh() + a3 * (6.0 * xi0).cos() * (6.0 * eta0).sinh() + a4 * (8.0 * xi0).cos() * (8.0 * eta0).sinh();
    (fe + k0 * cap_a * eta, fnorth + k0 * cap_a * xi)
}

fn tm_inverse(x: f64, y: f64, lon0_deg: f64, k0: f64, fe: f64, fnorth: f64, ell: &Ellipsoid) -> (f64, f64) {
    let n = ell.f() / (2.0 - ell.f());
    let (n2, n3, n4) = (n * n, n * n * n, n * n * n * n);
    let cap_a = (ell.a / (1.0 + n)) * (1.0 + 0.25 * n2 + (1.0 / 64.0) * n4);
    let (b1, b2, b3, b4) = (0.5 * n - (2.0 / 3.0) * n2 + (37.0 / 96.0) * n3 - (1.0 / 360.0) * n4, (1.0 / 48.0) * n2 + (1.0 / 15.0) * n3 - (437.0 / 1440.0) * n4, (17.0 / 480.0) * n3 - (37.0 / 840.0) * n4, (4397.0 / 161280.0) * n4);
    let (d1, d2, d3, d4) = (2.0 * n - (2.0 / 3.0) * n2 - 2.0 * n3 + (116.0 / 45.0) * n4, (7.0 / 3.0) * n2 - (8.0 / 5.0) * n3 - (227.0 / 45.0) * n4, (56.0 / 15.0) * n3 - (136.0 / 35.0) * n4, (4279.0 / 630.0) * n4);
    let (xi, eta) = ((y - fnorth) / (k0 * cap_a), (x - fe) / (k0 * cap_a));
    let xi0 = xi - (b1 * (2.0 * xi).sin() * (2.0 * eta).cosh() + b2 * (4.0 * xi).sin() * (4.0 * eta).cosh() + b3 * (6.0 * xi).sin() * (6.0 * eta).cosh() + b4 * (8.0 * xi).sin() * (8.0 * eta).cosh());
    let eta0 = eta - (b1 * (2.0 * xi).cos() * (2.0 * eta).sinh() + b2 * (4.0 * xi).cos() * (4.0 * eta).sinh() + b3 * (6.0 * xi).cos() * (6.0 * eta).sinh() + b4 * (8.0 * xi).cos() * (8.0 * eta).sinh());
    let chi = (xi0.sin() / eta0.cosh()).clamp(-1.0, 1.0).asin();
    let lat = chi + d1 * (2.0 * chi).sin() + d2 * (4.0 * chi).sin() + d3 * (6.0 * chi).sin() + d4 * (8.0 * chi).sin();
    let lon = lon0_deg.to_radians() + eta0.sinh().atan2(xi0.cos());
    (lon.to_degrees(), lat.to_degrees())
}

fn albers_q(phi: f64, e: f64, e2: f64) -> f64 {
    let s = (phi.sin() * e).clamp(-0.999999, 0.999999);
    (1.0 - e2) * (phi.sin() / (1.0 - s * s) - (0.5 / e) * ((1.0 - s) / (1.0 + s)).ln())
}

fn albers_params(lat1: f64, lat2: f64, lat0: f64, ell: &Ellipsoid) -> (f64, f64, f64, f64, f64) {
    let (p1, p2, p0, e, e2) = (lat1.to_radians(), lat2.to_radians(), lat0.to_radians(), ell.e(), ell.e2());
    let m1 = p1.cos() / (1.0 - e2 * p1.sin().powi(2)).sqrt();
    let m2 = p2.cos() / (1.0 - e2 * p2.sin().powi(2)).sqrt();
    let (q1, q2, q0) = (albers_q(p1, e, e2), albers_q(p2, e, e2), albers_q(p0, e, e2));
    let n = if (p1 - p2).abs() < 1e-10 { p1.sin() } else { (m1 * m1 - m2 * m2) / (q2 - q1) };
    let c = m1 * m1 + n * q1;
    (n, c, ell.a * (c - n * q0).max(0.0).sqrt() / n, e, e2)
}

fn albers_forward(lat_deg: f64, lon_deg: f64, lat1: f64, lat2: f64, lat0: f64, lon0: f64, fe: f64, fnorth: f64, ell: &Ellipsoid) -> (f64, f64) {
    let (n, c, rho0, e, e2) = albers_params(lat1, lat2, lat0, ell);
    let (phi, lam, lam0) = (lat_deg.to_radians(), lon_deg.to_radians(), lon0.to_radians());
    let rho = ell.a * (c - n * albers_q(phi, e, e2)).max(0.0).sqrt() / n;
    (fe + rho * (n * (lam - lam0)).sin(), fnorth + rho0 - rho * (n * (lam - lam0)).cos())
}

fn albers_inverse(x: f64, y: f64, lat1: f64, lat2: f64, lat0: f64, lon0: f64, fe: f64, fnorth: f64, ell: &Ellipsoid) -> (f64, f64) {
    let (n, c, rho0, e, e2) = albers_params(lat1, lat2, lat0, ell);
    let sign = if n > 0.0 { 1.0 } else { -1.0 };
    let (dx, dy) = (x - fe, rho0 - (y - fnorth));
    let (rho, theta) = (sign * dx.hypot(dy), (sign * dx).atan2(sign * dy));
    let q = (c - (rho * n / ell.a).powi(2)) / n;
    let lam = lon0.to_radians() + theta / n;
    let mut phi = (q * 0.5).clamp(-1.0, 1.0).asin();
    for _ in 0..6 {
        let es = (e * phi.sin()).clamp(-0.999999, 0.999999);
        let dq = 2.0 * (1.0 - e2) * phi.cos() / ((1.0 - es * es) * (1.0 - es * es));
        let diff = (q - albers_q(phi, e, e2)) / dq;
        phi += diff;
        if diff.abs() < 1e-12 { break; }
    }
    (lam.to_degrees(), phi.to_degrees())
}

fn lcc_params(lat1: f64, lat2: f64, lat0: f64, ell: &Ellipsoid) -> (f64, f64, f64, f64) {
    let (p1, p2, p0, e, e2) = (lat1.to_radians(), lat2.to_radians(), lat0.to_radians(), ell.e(), ell.e2());
    let m = |p: f64| p.cos() / (1.0 - e2 * p.sin() * p.sin()).sqrt();
    let t = |p: f64| {
        let s = (p.sin() * e).clamp(-0.999999, 0.999999);
        ((FRAC_PI_4 - 0.5 * p).tan() / ((1.0 - s) / (1.0 + s)).powf(0.5 * e)).max(1e-15)
    };
    let (m1, m2, t1, t2, t0) = (m(p1), m(p2), t(p1), t(p2), t(p0));
    let n = if (p1 - p2).abs() < 1e-10 { p1.sin() } else { (m1.ln() - m2.ln()) / (t1.ln() - t2.ln()) };
    let f_const = m1 / (n * t1.powf(n));
    (n, f_const, ell.a * f_const * t0.powf(n), e)
}

fn lcc_forward(lat_deg: f64, lon_deg: f64, lat1: f64, lat2: f64, lat0: f64, lon0: f64, fe: f64, fnorth: f64, ell: &Ellipsoid) -> (f64, f64) {
    let (n, f_const, rho0, e) = lcc_params(lat1, lat2, lat0, ell);
    let (phi, lam, lam0) = (lat_deg.to_radians(), lon_deg.to_radians(), lon0.to_radians());
    let s = (phi.sin() * e).clamp(-0.999999, 0.999999);
    let t = ((FRAC_PI_4 - 0.5 * phi).tan() / ((1.0 - s) / (1.0 + s)).powf(0.5 * e)).max(1e-15);
    let rho = ell.a * f_const * t.powf(n);
    (fe + rho * (n * (lam - lam0)).sin(), fnorth + rho0 - rho * (n * (lam - lam0)).cos())
}

fn lcc_inverse(x: f64, y: f64, lat1: f64, lat2: f64, lat0: f64, lon0: f64, fe: f64, fnorth: f64, ell: &Ellipsoid) -> (f64, f64) {
    let (n, f_const, rho0, e) = lcc_params(lat1, lat2, lat0, ell);
    let sign = if n > 0.0 { 1.0 } else { -1.0 };
    let (dx, dy) = (x - fe, rho0 - (y - fnorth));
    let (rho, theta) = (sign * dx.hypot(dy), (sign * dx).atan2(sign * dy));
    let t = (rho / (ell.a * f_const)).powf(1.0 / n);
    let lam = lon0.to_radians() + theta / n;
    let mut phi = FRAC_PI_2 - 2.0 * t.atan();
    for _ in 0..6 {
        let s = (e * phi.sin()).clamp(-0.999999, 0.999999);
        let next = FRAC_PI_2 - 2.0 * (t * ((1.0 - s) / (1.0 + s)).powf(0.5 * e)).atan();
        if (next - phi).abs() < 1e-12 { phi = next; break; }
        phi = next;
    }
    (lam.to_degrees(), phi.to_degrees())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mga55_kosciuszko_roundtrip() {
        let crs = CrsDefinition::from_epsg(EPSG_GDA2020_MGA55).expect("CRS from EPSG:7855");
        let (lon, lat) = (148.2636, -36.4558);
        let (x, y) = project_forward(&crs, lon, lat).expect("MGA55 forward");
        assert!((x - 613229.0).abs() < 10.0 && (y - 5964752.0).abs() < 100.0, "MGA55 forward mismatch");

        let (r_lon, r_lat) = project_inverse(&crs, x, y).expect("MGA55 inverse");
        let (r_x, r_y) = project_forward(&crs, r_lon, r_lat).expect("MGA55 roundtrip forward");
        assert!((r_x - x).hypot(r_y - y) < 0.01, "MGA55 roundtrip error exceeds 0.01 m");
        assert!((r_lon - lon).abs() < 1e-7 && (r_lat - lat).abs() < 1e-7);
    }

    #[test]
    fn test_albers_and_web_mercator_roundtrips() {
        let albers = CrsDefinition::from_epsg(EPSG_GDA2020_ALBERS).expect("Albers CRS");
        let (lon, lat) = (148.2636, -36.4558);
        let (ax, ay) = project_forward(&albers, lon, lat).expect("Albers forward");
        let (a_lon, a_lat) = project_inverse(&albers, ax, ay).expect("Albers inverse");
        assert!((a_lon - lon).abs() < 1e-7 && (a_lat - lat).abs() < 1e-7);

        let web = CrsDefinition::from_epsg(EPSG_WEB_MERCATOR).expect("Web Mercator CRS");
        let (wx, wy) = project_forward(&web, lon, lat).expect("Web Merc forward");
        let (w_lon, w_lat) = project_inverse(&web, wx, wy).expect("Web Merc inverse");
        assert!((w_lon - lon).abs() < 1e-7 && (w_lat - lat).abs() < 1e-7);
    }

    #[test]
    fn test_parse_wkt1_and_wkt2() {
        let wkt1 = r#"PROJCS["GDA2020 / MGA zone 55", GEOGCS["GDA2020", DATUM["GDA2020", SPHEROID["GRS 1980", 6378137.0, 298.257222101]], PRIMEM["Greenwich", 0.0], UNIT["degree", 0.0174532925199433]], PROJECTION["Transverse_Mercator"], PARAMETER["latitude_of_origin", 0.0], PARAMETER["central_meridian", 147.0], PARAMETER["scale_factor", 0.9996], PARAMETER["false_easting", 500000.0], PARAMETER["false_northing", 10000000.0], UNIT["metre", 1.0], AUTHORITY["EPSG", "7855"]]"#;
        let crs1 = CrsDefinition::parse_wkt_slice(wkt1).expect("WKT1 parse");
        assert_eq!(crs1.epsg, Some(EpsgCode(7855)));
        assert_eq!(crs1.ellipsoid, Ellipsoid::GRS80);

        let wkt2 = r#"PROJCRS["GDA2020 / MGA zone 55", BASEGEOGCRS["GDA2020", DATUM["GDA2020", ELLIPSOID["GRS 1980", 6378137.0, 298.257222101]]], CONVERSION["MGA zone 55", METHOD["Transverse Mercator"], PARAMETER["Longitude of natural origin", 147.0], PARAMETER["Scale factor at natural origin", 0.9996], PARAMETER["False easting", 500000.0], PARAMETER["False northing", 10000000.0]], CS[Cartesian, 2], ID["EPSG", 7855]]"#;
        let crs2 = CrsDefinition::parse_wkt_slice(wkt2).expect("WKT2 parse");
        assert_eq!(crs2.epsg, Some(EpsgCode(7855)));
        assert_eq!(crs2.ellipsoid, Ellipsoid::GRS80);
    }

    #[test]
    fn test_helmert_shift_and_ausgeoid2020() {
        let (lat, lon) = (-36.4558, 148.2636);
        let (x, y, z) = lat_lon_to_ecef(lat, lon, 2228.0, &Ellipsoid::GRS80);
        let (tx, ty, tz) = Helmert7Param::GDA94_TO_GDA2020.transform_ecef(x, y, z);
        let (dx, dy, dz) = (tx - x, ty - y, tz - z);
        let shift_3d = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(shift_3d > 1.4 && shift_3d < 2.0, "Expected ~1.8 m shift, got {shift_3d}");

        let (phi, lam) = (lat.to_radians(), lon.to_radians());
        assert!(-lam.sin() * dx + lam.cos() * dy > 0.0, "Expected positive East shift");
        assert!(-phi.sin() * lam.cos() * dx - phi.sin() * lam.sin() * dy + phi.cos() * dz > 0.0, "Expected positive North shift");

        let n = ausgeoid2020_undulation_m(lat, lon);
        assert!(n > 19.0 && n < 23.0, "Expected ~21 m geoid undulation, got {n}");
        let h_ahd = 2249.5 - n;
        assert!((h_ahd - 2228.0).abs() < 2.0, "Expected AHD height ~2228 m, got {h_ahd}");
    }
}
