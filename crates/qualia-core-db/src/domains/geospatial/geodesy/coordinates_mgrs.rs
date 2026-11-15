// Adapted from EarthCraft (crates/earthcraft-core/src/coordinates.rs)
// Copyright (c) 2026 EarthCraft contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Zero-heap geodesic coordinates, UTM/UPS, MGRS, Geohash, and Plus Code (OLC).
//! Tier-1 HotZeroHeap compliant: all allocations occur on stack byte buffers.

use std::f64::consts::PI;

const DEG_TO_RAD: f64 = PI / 180.0;
const RAD_TO_DEG: f64 = 180.0 / PI;
const WGS84_A: f64 = 6378137.0;
const WGS84_F: f64 = 1.0 / 298.257223563;
const WGS84_E2: f64 = 2.0 * WGS84_F - WGS84_F * WGS84_F;
const WGS84_EP2: f64 = WGS84_E2 / (1.0 - WGS84_E2);
const UTM_K0: f64 = 0.9996;

/// Fixed-capacity 64-byte stack cursor buffer for zero-heap string formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackText64 {
    pub bytes: [u8; 64],
    pub len: u8,
}

impl StackText64 {
    pub const fn empty() -> Self { Self { bytes: [0u8; 64], len: 0 } }
    pub fn as_str(&self) -> &str { std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or("") }
    pub fn push_byte(&mut self, b: u8) -> bool {
        if (self.len as usize) < self.bytes.len() {
            self.bytes[self.len as usize] = b; self.len += 1; true
        } else { false }
    }
    pub fn push_str(&mut self, s: &str) -> bool {
        let b = s.as_bytes();
        if (self.len as usize) + b.len() <= self.bytes.len() {
            self.bytes[self.len as usize..self.len as usize + b.len()].copy_from_slice(b);
            self.len += b.len() as u8; true
        } else { false }
    }
}

/// UTM projected coordinate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtmUpsCoord {
    pub zone: u8, pub band: u8, pub is_north: bool,
    pub easting_m: f64, pub northing_m: f64,
}

/// Military Grid Reference System (MGRS) coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MgrsCoord {
    pub zone: u8, pub band: u8, pub col_letter: u8, pub row_letter: u8,
    pub precision: u8, pub easting_m: u32, pub northing_m: u32,
}

/// Errors occurring during coordinate parsing or conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordParseError {
    InvalidLatitude, InvalidLongitude, InvalidZone, InvalidBand,
    InvalidGridSquare, InvalidFormat, InvalidPrecision, BufferOverflow,
}

impl std::fmt::Display for CoordParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidLatitude => "Latitude must be within [-90, 90] degrees",
            Self::InvalidLongitude => "Longitude must be within [-180, 180] degrees",
            Self::InvalidZone => "UTM zone must be between 1 and 60",
            Self::InvalidBand => "Invalid MGRS latitude band letter",
            Self::InvalidGridSquare => "Invalid MGRS 100km grid square letters",
            Self::InvalidFormat => "Unrecognized or malformed coordinate string",
            Self::InvalidPrecision => "Precision must be between 1 and 5",
            Self::BufferOverflow => "Stack text buffer capacity exceeded",
        })
    }
}
impl std::error::Error for CoordParseError {}

fn push_padded_u32(out: &mut StackText64, val: u32, digits: u8) {
    let mut buf = [b'0'; 10];
    let (mut v, d) = (val, digits as usize);
    for i in (0..d).rev() { buf[i] = b'0' + (v % 10) as u8; v /= 10; }
    for &b in &buf[..d] { out.push_byte(b); }
}

/// Convert WGS-84 latitude and longitude to UTM coordinate with Norway/Svalbard zone rules.
pub fn latlon_to_utm(lat_deg: f64, lon_deg: f64) -> Result<UtmUpsCoord, CoordParseError> {
    if !lat_deg.is_finite() || lat_deg < -80.0 || lat_deg > 84.0 { return Err(CoordParseError::InvalidLatitude); }
    if !lon_deg.is_finite() || lon_deg < -180.0 || lon_deg > 180.0 { return Err(CoordParseError::InvalidLongitude); }
    let mut zone = (((lon_deg + 180.0) / 6.0).floor() as u8) + 1;
    if zone > 60 { zone = 60; }
    if (56.0..64.0).contains(&lat_deg) && (3.0..12.0).contains(&lon_deg) {
        zone = 32;
    } else if (72.0..84.0).contains(&lat_deg) {
        zone = if (0.0..9.0).contains(&lon_deg) { 31 }
        else if (9.0..21.0).contains(&lon_deg) { 33 }
        else if (21.0..33.0).contains(&lon_deg) { 35 }
        else if (33.0..42.0).contains(&lon_deg) { 37 } else { zone };
    }
    let band_idx = if lat_deg >= 72.0 { 19 } else { (((lat_deg + 80.0) / 8.0).floor() as usize).min(19) };
    let band = b"CDEFGHJKLMNPQRSTUVWX"[band_idx];
    let is_north = lat_deg >= 0.0;
    let (lat, lon0) = (lat_deg * DEG_TO_RAD, ((zone as f64 - 1.0) * 6.0 - 180.0 + 3.0) * DEG_TO_RAD);
    let dlon = lon_deg * DEG_TO_RAD - lon0;
    let (sin_lat, cos_lat, tan_lat) = (lat.sin(), lat.cos(), lat.tan());

    let c0 = 1.0 - WGS84_E2 * (1.0 / 4.0 + WGS84_E2 * (3.0 / 64.0 + 5.0 * WGS84_E2 / 256.0));
    let c1 = WGS84_E2 * (3.0 / 8.0 + WGS84_E2 * (3.0 / 32.0 + 45.0 * WGS84_E2 / 1024.0));
    let c2 = WGS84_E2 * WGS84_E2 * (15.0 / 256.0 + 45.0 * WGS84_E2 / 1024.0);
    let c3 = 35.0 * WGS84_E2.powi(3) / 3072.0;
    let m = WGS84_A * (c0 * lat - c1 * (2.0 * lat).sin() + c2 * (4.0 * lat).sin() - c3 * (6.0 * lat).sin());
    let n = WGS84_A / (1.0 - WGS84_E2 * sin_lat * sin_lat).sqrt();
    let (t, c, a) = (tan_lat * tan_lat, WGS84_EP2 * cos_lat * cos_lat, dlon * cos_lat);

    let easting = 500_000.0 + UTM_K0 * n * (a + (1.0 - t + c) * a.powi(3) / 6.0
        + (5.0 - 18.0 * t + t * t + 72.0 * c - 58.0 * WGS84_EP2) * a.powi(5) / 120.0);
    let mut northing = UTM_K0 * (m + n * tan_lat * (a * a / 2.0
        + (5.0 - t + 9.0 * c + 4.0 * c * c) * a.powi(4) / 24.0
        + (61.0 - 58.0 * t + t * t + 600.0 * c - 330.0 * WGS84_EP2) * a.powi(6) / 720.0));
    if !is_north { northing += 10_000_000.0; }
    Ok(UtmUpsCoord { zone, band, is_north, easting_m: easting, northing_m: northing })
}

/// Convert UTM projected coordinate back to WGS-84 latitude and longitude.
pub fn utm_to_latlon(utm: UtmUpsCoord) -> Result<(f64, f64), CoordParseError> {
    if utm.zone < 1 || utm.zone > 60 { return Err(CoordParseError::InvalidZone); }
    let e1 = (1.0 - (1.0 - WGS84_E2).sqrt()) / (1.0 + (1.0 - WGS84_E2).sqrt());
    let x_adj = utm.easting_m - 500_000.0;
    let y_adj = if utm.is_north { utm.northing_m } else { utm.northing_m - 10_000_000.0 };
    let m = y_adj / UTM_K0;
    let c0 = 1.0 - WGS84_E2 * (1.0 / 4.0 + WGS84_E2 * (3.0 / 64.0 + 5.0 * WGS84_E2 / 256.0));
    let mu = m / (WGS84_A * c0);
    let phi1 = mu + (3.0 * e1 / 2.0 - 27.0 * e1.powi(3) / 32.0) * (2.0 * mu).sin()
        + (21.0 * e1 * e1 / 16.0 - 55.0 * e1.powi(4) / 32.0) * (4.0 * mu).sin()
        + (151.0 * e1.powi(3) / 96.0) * (6.0 * mu).sin() + (1097.0 * e1.powi(4) / 512.0) * (8.0 * mu).sin();
    let (sin_phi1, cos_phi1, tan_phi1) = (phi1.sin(), phi1.cos(), phi1.tan());
    let n1 = WGS84_A / (1.0 - WGS84_E2 * sin_phi1 * sin_phi1).sqrt();
    let r1 = WGS84_A * (1.0 - WGS84_E2) / (1.0 - WGS84_E2 * sin_phi1 * sin_phi1).powf(1.5);
    let (d, t1, c1) = (x_adj / (n1 * UTM_K0), tan_phi1 * tan_phi1, WGS84_EP2 * cos_phi1 * cos_phi1);

    let lat = phi1 - (n1 * tan_phi1 / r1) * (d * d / 2.0
        - (5.0 + 3.0 * t1 + 10.0 * c1 - 4.0 * c1 * c1 - 9.0 * WGS84_EP2) * d.powi(4) / 24.0
        + (61.0 + 90.0 * t1 + 298.0 * c1 + 45.0 * t1 * t1 - 252.0 * WGS84_EP2 - 3.0 * c1 * c1) * d.powi(6) / 720.0);
    let lon0 = ((utm.zone as f64 - 1.0) * 6.0 - 180.0 + 3.0) * DEG_TO_RAD;
    let lon = lon0 + (d - (1.0 + 2.0 * t1 + c1) * d.powi(3) / 6.0
        + (5.0 - 2.0 * c1 + 28.0 * t1 - 3.0 * c1 * c1 + 8.0 * WGS84_EP2 + 24.0 * t1 * t1) * d.powi(5) / 120.0) / cos_phi1;
    Ok((lat * RAD_TO_DEG, lon * RAD_TO_DEG))
}

/// Convert WGS-84 latitude and longitude to MGRS coordinate at given precision (1..=5).
pub fn latlon_to_mgrs(lat_deg: f64, lon_deg: f64, precision: u8) -> Result<MgrsCoord, CoordParseError> {
    if !(1..=5).contains(&precision) { return Err(CoordParseError::InvalidPrecision); }
    let utm = latlon_to_utm(lat_deg, lon_deg)?;
    let col_num = (utm.easting_m / 100_000.0).floor() as usize;
    if !(1..=8).contains(&col_num) { return Err(CoordParseError::InvalidGridSquare); }
    let col_letter = match ((utm.zone - 1) % 3) as usize {
        0 => b"ABCDEFGH", 1 => b"JKLMNPQR", _ => b"STUVWXYZ",
    }[col_num - 1];

    let row_offset = if utm.zone % 2 == 1 { 0 } else { 5 };
    let row_idx = (((utm.northing_m / 100_000.0).floor() as usize) + row_offset) % 20;
    let row_letter = b"ABCDEFGHJKLMNPQRSTUV"[row_idx];
    let scale = 10u32.pow(5 - precision as u32);
    let easting_m = ((utm.easting_m % 100_000.0) as u32) / scale;
    let northing_m = ((utm.northing_m % 100_000.0) as u32) / scale;
    Ok(MgrsCoord { zone: utm.zone, band: utm.band, col_letter, row_letter, precision, easting_m, northing_m })
}

/// Format MGRS coordinate into zero-heap stack buffer.
pub fn format_mgrs_stack(mgrs: MgrsCoord) -> StackText64 {
    let mut out = StackText64::empty();
    if mgrs.zone >= 10 { out.push_byte(b'0' + mgrs.zone / 10); }
    out.push_byte(b'0' + mgrs.zone % 10);
    for &b in &[mgrs.band, mgrs.col_letter, mgrs.row_letter] { out.push_byte(b); }
    push_padded_u32(&mut out, mgrs.easting_m, mgrs.precision);
    push_padded_u32(&mut out, mgrs.northing_m, mgrs.precision);
    out
}

/// Parse raw ASCII bytes into an MGRS coordinate struct.
pub fn parse_mgrs_bytes(input: &[u8]) -> Result<MgrsCoord, CoordParseError> {
    let mut s = input;
    while let Some((&f, rest)) = s.split_first() { if f == b' ' || f == b'\t' { s = rest; } else { break; } }
    while let Some((&l, rest)) = s.split_last() { if l == b' ' || l == b'\t' || l == b'\r' || l == b'\n' { s = rest; } else { break; } }
    if s.len() < 5 { return Err(CoordParseError::InvalidFormat); }
    let (mut idx, mut zone) = (0, 0u8);
    while idx < s.len() && s[idx].is_ascii_digit() && idx < 2 { zone = zone * 10 + (s[idx] - b'0'); idx += 1; }
    if zone < 1 || zone > 60 || idx == 0 || idx + 3 > s.len() { return Err(CoordParseError::InvalidZone); }
    let band = s[idx].to_ascii_uppercase(); idx += 1;
    if !b"CDEFGHJKLMNPQRSTUVWX".contains(&band) { return Err(CoordParseError::InvalidBand); }
    let col_letter = s[idx].to_ascii_uppercase(); idx += 1;
    let row_letter = s[idx].to_ascii_uppercase(); idx += 1;

    let rem = &s[idx..];
    if rem.is_empty() || rem.len() % 2 != 0 || rem.len() > 10 { return Err(CoordParseError::InvalidFormat); }
    let precision = (rem.len() / 2) as u8;
    let (mut easting_m, mut northing_m) = (0u32, 0u32);
    for &b in &rem[..precision as usize] {
        if !b.is_ascii_digit() { return Err(CoordParseError::InvalidFormat); }
        easting_m = easting_m * 10 + (b - b'0') as u32;
    }
    for &b in &rem[precision as usize..] {
        if !b.is_ascii_digit() { return Err(CoordParseError::InvalidFormat); }
        northing_m = northing_m * 10 + (b - b'0') as u32;
    }
    Ok(MgrsCoord { zone, band, col_letter, row_letter, precision, easting_m, northing_m })
}

/// Convert MGRS coordinate back to WGS-84 latitude and longitude.
pub fn mgrs_to_latlon(mgrs: MgrsCoord) -> Result<(f64, f64), CoordParseError> {
    if mgrs.zone < 1 || mgrs.zone > 60 { return Err(CoordParseError::InvalidZone); }
    let band_idx = b"CDEFGHJKLMNPQRSTUVWX".iter().position(|&b| b == mgrs.band).ok_or(CoordParseError::InvalidBand)?;
    let min_lat = -80.0 + (band_idx as f64) * 8.0;
    let max_lat = min_lat + if mgrs.band == b'X' { 12.0 } else { 8.0 };
    let center_lat = (min_lat + max_lat) * 0.5;
    let is_north = mgrs.band >= b'N';
    let approx_n = if is_north { center_lat * 111_000.0 } else { 10_000_000.0 + center_lat * 111_000.0 };

    let col_str = match ((mgrs.zone - 1) % 3) as usize { 0 => b"ABCDEFGH", 1 => b"JKLMNPQR", _ => b"STUVWXYZ" };
    let col_idx = col_str.iter().position(|&c| c == mgrs.col_letter).ok_or(CoordParseError::InvalidGridSquare)?;
    let base_e = (col_idx as f64 + 1.0) * 100_000.0;

    let row_idx = b"ABCDEFGHJKLMNPQRSTUV".iter().position(|&c| c == mgrs.row_letter).ok_or(CoordParseError::InvalidGridSquare)? as i32;
    let row_offset = if mgrs.zone % 2 == 1 { 0 } else { 5 };
    let rem = (row_idx - row_offset).rem_euclid(20);
    let approx_100k = (approx_n / 100_000.0).floor() as i32;
    let base_cand = (approx_100k / 20) * 20 + rem;
    let candidates = [base_cand - 20, base_cand, base_cand + 20];
    let mut best_cand = candidates[0];
    let mut best_diff = (candidates[0] as f64 * 100_000.0 - approx_n).abs();
    for &c in &candidates[1..] {
        let diff = (c as f64 * 100_000.0 - approx_n).abs();
        if diff < best_diff { best_diff = diff; best_cand = c; }
    }
    let scale = 10f64.powi(5 - mgrs.precision as i32);
    let full_e = base_e + (mgrs.easting_m as f64) * scale + 0.5 * scale;
    let full_n = (best_cand as f64 * 100_000.0) + (mgrs.northing_m as f64) * scale + 0.5 * scale;
    utm_to_latlon(UtmUpsCoord { zone: mgrs.zone, band: mgrs.band, is_north, easting_m: full_e, northing_m: full_n })
}

fn format_angle_dms(out: &mut StackText64, deg_abs: f64, hemi: u8) {
    let (d, rem_m) = (deg_abs.floor() as u32, (deg_abs - deg_abs.floor()) * 60.0);
    let (m, s) = (rem_m.floor() as u32, (rem_m - rem_m.floor()) * 60.0);
    let mut s_tenths = (s * 10.0).round() as u32;
    let (mut m, mut d) = (m, d);
    if s_tenths >= 600 { m += 1; s_tenths -= 600; }
    if m >= 60 { d += 1; m -= 60; }
    push_padded_u32(out, d, if d >= 100 { 3 } else if d >= 10 { 2 } else { 1 });
    out.push_str("°"); push_padded_u32(out, m, 2); out.push_byte(b'\'');
    push_padded_u32(out, s_tenths / 10, 2); out.push_byte(b'.');
    out.push_byte(b'0' + (s_tenths % 10) as u8); out.push_byte(b'"'); out.push_byte(hemi);
}

/// Format latitude and longitude in Degrees Minutes Seconds (DMS) into a zero-heap stack buffer.
pub fn format_dms_stack(lat_deg: f64, lon_deg: f64) -> StackText64 {
    let mut out = StackText64::empty();
    let (lat_h, lat_abs) = if lat_deg < 0.0 { (b'S', -lat_deg) } else { (b'N', lat_deg) };
    let (lon_h, lon_abs) = if lon_deg < 0.0 { (b'W', -lon_deg) } else { (b'E', lon_deg) };
    format_angle_dms(&mut out, lat_abs, lat_h); out.push_byte(b' ');
    format_angle_dms(&mut out, lon_abs, lon_h); out
}

fn parse_angle_token(token: &str, is_lat: bool) -> Result<f64, CoordParseError> {
    let mut s = token.trim();
    if s.is_empty() { return Err(CoordParseError::InvalidFormat); }
    let mut is_neg = false;
    if let Some(r) = s.strip_prefix('\u{2212}') { is_neg = true; s = r.trim(); }
    else if let Some(r) = s.strip_prefix('-') { is_neg = true; s = r.trim(); }
    else if let Some(r) = s.strip_prefix('+') { s = r.trim(); }

    if let Some(ch) = s.chars().last() {
        let u = ch.to_ascii_uppercase();
        if u == 'S' || u == 'W' { is_neg = true; s = s[..s.len() - ch.len_utf8()].trim(); }
        else if u == 'N' || u == 'E' { s = s[..s.len() - ch.len_utf8()].trim(); }
    }
    let (mut nums, mut count, mut start) = ([0.0f64; 3], 0, None);
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' { if start.is_none() { start = Some(i); } }
        else if let Some(st) = start {
            if count >= 3 { return Err(CoordParseError::InvalidFormat); }
            nums[count] = s[st..i].parse::<f64>().map_err(|_| CoordParseError::InvalidFormat)?;
            count += 1; start = None;
        }
    }
    if let Some(st) = start {
        if count >= 3 { return Err(CoordParseError::InvalidFormat); }
        nums[count] = s[st..].parse::<f64>().map_err(|_| CoordParseError::InvalidFormat)?;
        count += 1;
    }
    if count == 0 { return Err(CoordParseError::InvalidFormat); }
    let val = match count {
        1 => nums[0], 2 => nums[0] + nums[1] / 60.0,
        3 => nums[0] + nums[1] / 60.0 + nums[2] / 3600.0, _ => return Err(CoordParseError::InvalidFormat),
    };
    let signed = if is_neg { -val } else { val };
    if is_lat && (signed < -90.0 || signed > 90.0 || !signed.is_finite()) { return Err(CoordParseError::InvalidLatitude); }
    if !is_lat && (signed < -180.0 || signed > 180.0 || !signed.is_finite()) { return Err(CoordParseError::InvalidLongitude); }
    Ok(signed)
}

/// Parse decimal degrees ("−36.4558, 148.2636"), DMS ("36°27'20.9\"S 148°15'49.0\"E"), or MGRS ("55HFA...").
pub fn parse_coordinate_str(input: &str) -> Result<(f64, f64), CoordParseError> {
    let s = input.trim();
    if s.is_empty() { return Err(CoordParseError::InvalidFormat); }
    let bytes = s.as_bytes();
    let mut d_count = 0;
    while d_count < bytes.len() && bytes[d_count].is_ascii_digit() { d_count += 1; }
    if (1..=2).contains(&d_count) && d_count + 3 <= bytes.len()
        && bytes[d_count].is_ascii_alphabetic() && bytes[d_count + 1].is_ascii_alphabetic() && bytes[d_count + 2].is_ascii_alphabetic()
    {
        let rem = &bytes[d_count + 3..];
        if !rem.is_empty() && rem.len() % 2 == 0 && rem.iter().all(|b| b.is_ascii_digit()) {
            if let Ok(mgrs) = parse_mgrs_bytes(bytes) { return mgrs_to_latlon(mgrs); }
        }
    }
    let (p1, p2) = if let Some(pos) = s.find(',') { (&s[..pos], &s[pos + 1..]) }
    else if let Some(pos) = s.find(';') { (&s[..pos], &s[pos + 1..]) }
    else if let Some(pos) = s.find(|c: char| c == 'N' || c == 'n' || c == 'S' || c == 's') {
        let sp = pos + 1;
        if sp < s.len() { (&s[..sp], &s[sp..]) } else { return Err(CoordParseError::InvalidFormat); }
    } else {
        let (mut split, mut in_space) = (None, false);
        for (idx, b) in s.bytes().enumerate() {
            if b == b' ' || b == b'\t' {
                if !in_space && idx > 0 { split = Some(idx); break; }
                in_space = true;
            }
        }
        if let Some(pos) = split { (&s[..pos], &s[pos..]) } else { return Err(CoordParseError::InvalidFormat); }
    };
    Ok((parse_angle_token(p1, true)?, parse_angle_token(p2, false)?))
}

/// Encode geographic coordinates into standard Geohash base32 string.
pub fn encode_geohash_stack(lat_deg: f64, lon_deg: f64, precision: u8) -> Result<StackText64, CoordParseError> {
    if !lat_deg.is_finite() || lat_deg < -90.0 || lat_deg > 90.0 { return Err(CoordParseError::InvalidLatitude); }
    if !lon_deg.is_finite() || lon_deg < -180.0 || lon_deg > 180.0 { return Err(CoordParseError::InvalidLongitude); }
    if precision == 0 || precision > 12 { return Err(CoordParseError::InvalidPrecision); }

    const BASE32: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";
    let (mut lat_min, mut lat_max, mut lon_min, mut lon_max) = (-90.0, 90.0, -180.0, 180.0);
    let (mut is_even, mut bits, mut bit_count) = (true, 0u8, 0u8);
    let mut out = StackText64::empty();
    while out.len < precision {
        if is_even {
            let mid = (lon_min + lon_max) * 0.5;
            if lon_deg >= mid { bits = (bits << 1) | 1; lon_min = mid; } else { bits <<= 1; lon_max = mid; }
        } else {
            let mid = (lat_min + lat_max) * 0.5;
            if lat_deg >= mid { bits = (bits << 1) | 1; lat_min = mid; } else { bits <<= 1; lat_max = mid; }
        }
        is_even = !is_even;
        bit_count += 1;
        if bit_count == 5 { out.push_byte(BASE32[bits as usize]); bit_count = 0; bits = 0; }
    }
    Ok(out)
}

/// Decode a Geohash base32 byte slice into latitude and longitude center point.
pub fn decode_geohash_bytes(hash: &[u8]) -> Result<(f64, f64), CoordParseError> {
    if hash.is_empty() { return Err(CoordParseError::InvalidFormat); }
    const BASE32: &[u8; 32] = b"0123456789bcdefghjkmnpqrstuvwxyz";
    let (mut lat_min, mut lat_max, mut lon_min, mut lon_max) = (-90.0, 90.0, -180.0, 180.0);
    let mut is_even = true;
    for &b in hash {
        let lower = b.to_ascii_lowercase();
        let val = BASE32.iter().position(|&c| c == lower).ok_or(CoordParseError::InvalidFormat)? as u8;
        for mask in [16u8, 8, 4, 2, 1] {
            if is_even {
                let mid = (lon_min + lon_max) * 0.5;
                if val & mask != 0 { lon_min = mid; } else { lon_max = mid; }
            } else {
                let mid = (lat_min + lat_max) * 0.5;
                if val & mask != 0 { lat_min = mid; } else { lat_max = mid; }
            }
            is_even = !is_even;
        }
    }
    Ok(((lat_min + lat_max) * 0.5, (lon_min + lon_max) * 0.5))
}

/// Encode geographic coordinates into a standard 10-digit Open Location Code (Plus Code).
pub fn encode_plus_code_stack(lat_deg: f64, lon_deg: f64) -> Result<StackText64, CoordParseError> {
    if !lat_deg.is_finite() || lat_deg < -90.0 || lat_deg > 90.0 { return Err(CoordParseError::InvalidLatitude); }
    if !lon_deg.is_finite() { return Err(CoordParseError::InvalidLongitude); }
    const ALPHABET: &[u8; 20] = b"23456789CFGHJMPQRVWX";
    let mut lat = lat_deg.clamp(-90.0, 90.0) + 90.0;
    if lat >= 180.0 { lat = 180.0 - 1e-10; }
    let mut lon = lon_deg + 180.0;
    while lon < 0.0 { lon += 360.0; }
    while lon >= 360.0 { lon -= 360.0; }

    const BLOCKS: [f64; 5] = [20.0, 1.0, 0.05, 0.0025, 0.000125];
    let mut out = StackText64::empty();
    for (i, &b) in BLOCKS.iter().enumerate() {
        if i == 4 { out.push_byte(b'+'); }
        let lat_d = ((lat / b).floor() as usize).clamp(0, 19);
        let lon_d = ((lon / b).floor() as usize).clamp(0, 19);
        out.push_byte(ALPHABET[lat_d]); out.push_byte(ALPHABET[lon_d]);
        lat -= (lat_d as f64) * b; lon -= (lon_d as f64) * b;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kosciuszko_utm_roundtrip() {
        let (lat, lon) = (-36.4558, 148.2636);
        let utm = latlon_to_utm(lat, lon).expect("Kosciuszko UTM");
        assert_eq!(utm.zone, 55); assert_eq!(utm.band, b'H'); assert!(!utm.is_north);
        assert!((utm.easting_m - 613229.0).abs() < 1.0);
        assert!((utm.northing_m - 5964752.7).abs() < 1.0);
        let (rt_lat, rt_lon) = utm_to_latlon(utm).expect("UTM reverse");
        assert!((rt_lat - lat).abs() < 1e-6); assert!((rt_lon - lon).abs() < 1e-6);
    }

    #[test]
    fn test_kosciuszko_mgrs_roundtrip() {
        let (lat, lon) = (-36.4558, 148.2636);
        let mgrs = latlon_to_mgrs(lat, lon, 5).expect("latlon to MGRS");
        assert_eq!(mgrs.zone, 55); assert_eq!(mgrs.band, b'H');
        assert_eq!(mgrs.col_letter, b'F'); assert_eq!(mgrs.row_letter, b'V');
        let text = format_mgrs_stack(mgrs);
        assert_eq!(text.as_str(), "55HFV1322964752");
        let parsed = parse_mgrs_bytes(text.as_str().as_bytes()).expect("parse MGRS");
        assert_eq!(parsed, mgrs);
        let (rt_lat, rt_lon) = mgrs_to_latlon(parsed).expect("MGRS to latlon");
        assert!((rt_lat - lat).abs() < 1e-4); assert!((rt_lon - lon).abs() < 1e-4);
    }

    #[test]
    fn test_parse_coordinate_str_dd_dms_mgrs() {
        let (lat1, lon1) = parse_coordinate_str("−36.4558, 148.2636").expect("parse DD unicode minus");
        assert!((lat1 - (-36.4558)).abs() < 1e-5); assert!((lon1 - 148.2636).abs() < 1e-5);
        let (lat2, lon2) = parse_coordinate_str("36°27'20.9\"S 148°15'49.0\"E").expect("parse DMS");
        assert!((lat2 - (-36.4558)).abs() < 1e-3); assert!((lon2 - 148.2636).abs() < 1e-3);
        let (lat3, lon3) = parse_coordinate_str("55HFV1322964752").expect("parse MGRS");
        assert!((lat3 - (-36.4558)).abs() < 1e-3); assert!((lon3 - 148.2636).abs() < 1e-3);
        let (lat4, lon4) = parse_coordinate_str("55HFA1234567890").expect("parse MGRS square FA");
        assert!(lat4 < 0.0 && lon4 > 0.0);
    }

    #[test]
    fn test_geohash_and_plus_code() {
        let (lat, lon) = (-36.4558, 148.2636);
        let gh = encode_geohash_stack(lat, lon, 8).expect("geohash encode");
        assert_eq!(gh.as_str(), "r392tg6m");
        let (dec_lat, dec_lon) = decode_geohash_bytes(gh.as_str().as_bytes()).expect("geohash decode");
        assert!((dec_lat - lat).abs() < 0.001); assert!((dec_lon - lon).abs() < 0.001);
        let plus = encode_plus_code_stack(lat, lon).expect("plus code encode");
        assert_eq!(plus.as_str(), "4RMCG7V7+MC");
    }
}
