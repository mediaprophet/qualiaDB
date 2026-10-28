// Copyright 2026 QualiaDB Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! High-performance OGC GeoPackage, FlatGeobuf, and Esri FileGDB binary codecs.
//! Implements Tier-1 HotZeroHeap header parsing and Tier-2 ColdBounded container encoding.

#[derive(Debug, Clone, PartialEq)]
pub enum GisBinaryError {
    InvalidMagic,
    TruncatedBuffer,
    UnsupportedGeometryType(u32),
    InvalidEnvelope,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox2D {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BBox2D {
    pub const fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Self {
        Self { min_x, min_y, max_x, max_y }
    }
    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x <= other.max_x && self.max_x >= other.min_x
            && self.min_y <= other.max_y && self.max_y >= other.min_y
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SimpleGeom2D {
    Point(f64, f64),
    LineString(Vec<[f64; 2]>),
    Polygon(Vec<[f64; 2]>),
}

impl SimpleGeom2D {
    pub fn compute_bbox(&self) -> Option<BBox2D> {
        match self {
            Self::Point(x, y) => Some(BBox2D::new(*x, *y, *x, *y)),
            Self::LineString(pts) | Self::Polygon(pts) => pts.first().map(|p| {
                let (mut min_x, mut min_y, mut max_x, mut max_y) = (p[0], p[1], p[0], p[1]);
                for q in &pts[1..] {
                    min_x = min_x.min(q[0]); min_y = min_y.min(q[1]);
                    max_x = max_x.max(q[0]); max_y = max_y.max(q[1]);
                }
                BBox2D::new(min_x, min_y, max_x, max_y)
            }),
        }
    }
}

fn wkb_byte_size(geom: &SimpleGeom2D) -> usize {
    match geom {
        SimpleGeom2D::Point(..) => 21,
        SimpleGeom2D::LineString(pts) => 9 + pts.len() * 16,
        SimpleGeom2D::Polygon(pts) => 13 + pts.len() * 16,
    }
}

fn encode_wkb(geom: &SimpleGeom2D, buf: &mut Vec<u8>) {
    buf.push(1); // LE
    match geom {
        SimpleGeom2D::Point(x, y) => {
            buf.extend_from_slice(&1u32.to_le_bytes());
            buf.extend_from_slice(&x.to_le_bytes());
            buf.extend_from_slice(&y.to_le_bytes());
        }
        SimpleGeom2D::LineString(pts) | SimpleGeom2D::Polygon(pts) => {
            let is_poly = matches!(geom, SimpleGeom2D::Polygon(_));
            buf.extend_from_slice(&(if is_poly { 3u32 } else { 2u32 }).to_le_bytes());
            if is_poly { buf.extend_from_slice(&1u32.to_le_bytes()); }
            buf.extend_from_slice(&(pts.len() as u32).to_le_bytes());
            for p in pts { buf.extend_from_slice(&p[0].to_le_bytes()); buf.extend_from_slice(&p[1].to_le_bytes()); }
        }
    }
}

fn read_wkb_pts(wkb: &[u8], mut off: usize, n: usize, is_le: bool, budget: usize) -> Result<Vec<[f64; 2]>, GisBinaryError> {
    if wkb.len() < off + n * 16 { return Err(GisBinaryError::TruncatedBuffer); }
    if n * 16 > budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: n * 16, budget }); }
    let mut pts = Vec::with_capacity(n);
    for _ in 0..n {
        let x = if is_le { f64::from_le_bytes(wkb[off..off + 8].try_into().unwrap()) } else { f64::from_be_bytes(wkb[off..off + 8].try_into().unwrap()) };
        let y = if is_le { f64::from_le_bytes(wkb[off + 8..off + 16].try_into().unwrap()) } else { f64::from_be_bytes(wkb[off + 8..off + 16].try_into().unwrap()) };
        pts.push([x, y]);
        off += 16;
    }
    Ok(pts)
}

fn decode_wkb(wkb: &[u8], budget: usize) -> Result<SimpleGeom2D, GisBinaryError> {
    if wkb.len() < 5 { return Err(GisBinaryError::TruncatedBuffer); }
    let is_le = wkb[0] == 1;
    let raw_type = if is_le { u32::from_le_bytes(wkb[1..5].try_into().unwrap()) } else { u32::from_be_bytes(wkb[1..5].try_into().unwrap()) };
    match raw_type & 0xFFFF {
        1 => {
            if wkb.len() < 21 { return Err(GisBinaryError::TruncatedBuffer); }
            let x = if is_le { f64::from_le_bytes(wkb[5..13].try_into().unwrap()) } else { f64::from_be_bytes(wkb[5..13].try_into().unwrap()) };
            let y = if is_le { f64::from_le_bytes(wkb[13..21].try_into().unwrap()) } else { f64::from_be_bytes(wkb[13..21].try_into().unwrap()) };
            Ok(SimpleGeom2D::Point(x, y))
        }
        2 => {
            if wkb.len() < 9 { return Err(GisBinaryError::TruncatedBuffer); }
            let n = if is_le { u32::from_le_bytes(wkb[5..9].try_into().unwrap()) } else { u32::from_be_bytes(wkb[5..9].try_into().unwrap()) } as usize;
            Ok(SimpleGeom2D::LineString(read_wkb_pts(wkb, 9, n, is_le, budget)?))
        }
        3 => {
            if wkb.len() < 9 { return Err(GisBinaryError::TruncatedBuffer); }
            let rings = if is_le { u32::from_le_bytes(wkb[5..9].try_into().unwrap()) } else { u32::from_be_bytes(wkb[5..9].try_into().unwrap()) };
            if rings == 0 { return Ok(SimpleGeom2D::Polygon(Vec::new())); }
            if wkb.len() < 13 { return Err(GisBinaryError::TruncatedBuffer); }
            let n = if is_le { u32::from_le_bytes(wkb[9..13].try_into().unwrap()) } else { u32::from_be_bytes(wkb[9..13].try_into().unwrap()) } as usize;
            Ok(SimpleGeom2D::Polygon(read_wkb_pts(wkb, 13, n, is_le, budget)?))
        }
        other => Err(GisBinaryError::UnsupportedGeometryType(other)),
    }
}

pub const GPKG_GEOM_MAGIC: [u8; 2] = *b"GP";
pub const SQLITE3_HEADER_MAGIC: &[u8; 16] = b"SQLite format 3\0";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpkgGeomHeader {
    pub version: u8,
    pub flags: u8,
    pub srs_id: i32,
    pub envelope: Option<BBox2D>,
    pub wkb_offset: usize,
}

pub fn parse_gpkg_geom_header(bytes: &[u8]) -> Result<GpkgGeomHeader, GisBinaryError> {
    if bytes.len() < 8 { return Err(GisBinaryError::TruncatedBuffer); }
    if bytes[0..2] != GPKG_GEOM_MAGIC { return Err(GisBinaryError::InvalidMagic); }
    let (version, flags) = (bytes[2], bytes[3]);
    let is_le = (flags & 0x01) == 1;
    let env_indicator = (flags >> 1) & 0x07;
    let srs_id = if is_le { i32::from_le_bytes(bytes[4..8].try_into().unwrap()) } else { i32::from_be_bytes(bytes[4..8].try_into().unwrap()) };
    let env_len = match env_indicator { 0 => 0, 1 => 32, 2 | 3 => 48, 4 => 64, _ => return Err(GisBinaryError::InvalidEnvelope) };
    let wkb_offset = 8 + env_len;
    if bytes.len() < wkb_offset { return Err(GisBinaryError::TruncatedBuffer); }
    let envelope = if env_indicator >= 1 {
        let b = &bytes[8..40];
        let (x0, x1, y0, y1) = if is_le {
            (f64::from_le_bytes(b[0..8].try_into().unwrap()), f64::from_le_bytes(b[8..16].try_into().unwrap()),
             f64::from_le_bytes(b[16..24].try_into().unwrap()), f64::from_le_bytes(b[24..32].try_into().unwrap()))
        } else {
            (f64::from_be_bytes(b[0..8].try_into().unwrap()), f64::from_be_bytes(b[8..16].try_into().unwrap()),
             f64::from_be_bytes(b[16..24].try_into().unwrap()), f64::from_be_bytes(b[24..32].try_into().unwrap()))
        };
        Some(BBox2D::new(x0, y0, x1, y1))
    } else { None };
    Ok(GpkgGeomHeader { version, flags, srs_id, envelope, wkb_offset })
}

pub fn encode_gpkg_geometry(geom: &SimpleGeom2D, srs_id: i32, include_envelope: bool, byte_budget: usize) -> Result<Vec<u8>, GisBinaryError> {
    let total_size = 8 + if include_envelope { 32 } else { 0 } + wkb_byte_size(geom);
    if total_size > byte_budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: total_size, budget: byte_budget }); }
    let mut buf = Vec::with_capacity(total_size);
    buf.extend_from_slice(&GPKG_GEOM_MAGIC);
    buf.push(0);
    buf.push(0x01 | if include_envelope { 0x02 } else { 0x00 });
    buf.extend_from_slice(&srs_id.to_le_bytes());
    if include_envelope {
        let b = geom.compute_bbox().unwrap_or(BBox2D::new(0.0, 0.0, 0.0, 0.0));
        buf.extend_from_slice(&b.min_x.to_le_bytes()); buf.extend_from_slice(&b.max_x.to_le_bytes());
        buf.extend_from_slice(&b.min_y.to_le_bytes()); buf.extend_from_slice(&b.max_y.to_le_bytes());
    }
    encode_wkb(geom, &mut buf);
    Ok(buf)
}

pub fn decode_gpkg_geometry(bytes: &[u8], byte_budget: usize) -> Result<(GpkgGeomHeader, SimpleGeom2D), GisBinaryError> {
    if bytes.len() > byte_budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: bytes.len(), budget: byte_budget }); }
    let header = parse_gpkg_geom_header(bytes)?;
    let geom = decode_wkb(&bytes[header.wkb_offset..], byte_budget)?;
    Ok((header, geom))
}

pub fn validate_sqlite_gpkg_file_header(bytes: &[u8]) -> Result<(u32, u32), GisBinaryError> {
    if bytes.len() < 100 { return Err(GisBinaryError::TruncatedBuffer); }
    if &bytes[0..16] != SQLITE3_HEADER_MAGIC { return Err(GisBinaryError::InvalidMagic); }
    let raw_pg = u16::from_be_bytes(bytes[16..18].try_into().unwrap());
    let page_size = if raw_pg == 1 { 65536 } else { raw_pg as u32 };
    let app_id = u32::from_be_bytes(bytes[68..72].try_into().unwrap());
    if app_id != 0x4750_4B47 && (app_id < 0x4750_3130 || app_id > 0x4750_3139) {
        return Err(GisBinaryError::InvalidMagic);
    }
    Ok((page_size, app_id))
}

pub const FGB_MAGIC: [u8; 8] = [b'f', b'g', b'b', 0x03, b'f', b'g', b'b', 0x01];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FgbIndexNode {
    pub bbox: BBox2D,
    pub offset: u64,
}

fn hilbert_2d(mut x: u32, mut y: u32) -> u64 {
    let mut d = 0u64;
    for i in (0..16).rev() {
        let (xi, yi) = ((x >> i) & 1, (y >> i) & 1);
        d = (d << 2) | (((xi << 1) | (xi ^ yi)) as u64);
        if yi == 0 {
            if xi == 1 { x = !x; y = !y; }
            std::mem::swap(&mut x, &mut y);
        }
    }
    d
}

pub fn encode_fgb_container(features: &[(BBox2D, SimpleGeom2D)], srs_epsg: u32, byte_budget: usize) -> Result<Vec<u8>, GisBinaryError> {
    let count = features.len();
    let index_size = count * 40;
    let mut total_size = 32 + index_size;
    for (_, g) in features { total_size += 36 + wkb_byte_size(g); }
    if total_size > byte_budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: total_size, budget: byte_budget }); }

    let (min_x, min_y, max_x, max_y) = features.first().map(|(b, _)| {
        let (mut x0, mut y0, mut x1, mut y1) = (b.min_x, b.min_y, b.max_x, b.max_y);
        for (f, _) in features { x0 = x0.min(f.min_x); y0 = y0.min(f.min_y); x1 = x1.max(f.max_x); y1 = y1.max(f.max_y); }
        (x0, y0, x1, y1)
    }).unwrap_or((0.0, 0.0, 1.0, 1.0));
    let span_x = if (max_x - min_x).abs() > 1e-9 { max_x - min_x } else { 1.0 };
    let span_y = if (max_y - min_y).abs() > 1e-9 { max_y - min_y } else { 1.0 };

    let mut tagged: Vec<(u64, usize)> = features.iter().enumerate().map(|(i, (b, _))| {
        let nx = (((((b.min_x + b.max_x) * 0.5) - min_x) / span_x) * 65535.0).clamp(0.0, 65535.0) as u32;
        let ny = (((((b.min_y + b.max_y) * 0.5) - min_y) / span_y) * 65535.0).clamp(0.0, 65535.0) as u32;
        (hilbert_2d(nx, ny), i)
    }).collect();
    tagged.sort_by_key(|t| t.0);

    let mut buf = Vec::with_capacity(total_size);
    buf.extend_from_slice(&FGB_MAGIC);
    buf.extend_from_slice(&srs_epsg.to_le_bytes());
    buf.extend_from_slice(&(count as u64).to_le_bytes());
    buf.extend_from_slice(&32u64.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());

    let dir_pos = buf.len();
    buf.resize(dir_pos + index_size, 0);

    let mut cur_off = (32 + index_size) as u64;
    for (node_idx, &(_, feat_idx)) in tagged.iter().enumerate() {
        let (bbox, geom) = &features[feat_idx];
        let feat_start = buf.len();
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&bbox.min_x.to_le_bytes()); buf.extend_from_slice(&bbox.min_y.to_le_bytes());
        buf.extend_from_slice(&bbox.max_x.to_le_bytes()); buf.extend_from_slice(&bbox.max_y.to_le_bytes());
        encode_wkb(geom, &mut buf);
        let feat_len = (buf.len() - feat_start - 4) as u32;
        buf[feat_start..feat_start + 4].copy_from_slice(&feat_len.to_le_bytes());

        let npos = dir_pos + node_idx * 40;
        buf.copy_within(feat_start + 4..feat_start + 36, npos);
        buf[npos + 32..npos + 40].copy_from_slice(&cur_off.to_le_bytes());
        cur_off = buf.len() as u64;
    }
    Ok(buf)
}

pub fn query_fgb_bbox_zero_copy(fgb_bytes: &[u8], query_bbox: &BBox2D, out_feature_indices: &mut [usize]) -> Result<usize, GisBinaryError> {
    if fgb_bytes.len() < 32 { return Err(GisBinaryError::TruncatedBuffer); }
    if &fgb_bytes[0..8] != &FGB_MAGIC { return Err(GisBinaryError::InvalidMagic); }
    let count = u64::from_le_bytes(fgb_bytes[12..20].try_into().unwrap()) as usize;
    let idx_offset = u64::from_le_bytes(fgb_bytes[20..28].try_into().unwrap()) as usize;
    if idx_offset + count * 40 > fgb_bytes.len() { return Err(GisBinaryError::TruncatedBuffer); }
    let mut matched = 0;
    for i in 0..count {
        let p = idx_offset + i * 40;
        let b = &fgb_bytes[p..p + 32];
        let node_bbox = BBox2D::new(
            f64::from_le_bytes(b[0..8].try_into().unwrap()), f64::from_le_bytes(b[8..16].try_into().unwrap()),
            f64::from_le_bytes(b[16..24].try_into().unwrap()), f64::from_le_bytes(b[24..32].try_into().unwrap()),
        );
        if query_bbox.intersects(&node_bbox) {
            if matched < out_feature_indices.len() { out_feature_indices[matched] = i; }
            matched += 1;
        }
    }
    Ok(matched)
}

pub fn decode_fgb_feature_at(fgb_bytes: &[u8], feature_idx: usize, byte_budget: usize) -> Result<(BBox2D, SimpleGeom2D), GisBinaryError> {
    if fgb_bytes.len() < 32 { return Err(GisBinaryError::TruncatedBuffer); }
    if &fgb_bytes[0..8] != &FGB_MAGIC { return Err(GisBinaryError::InvalidMagic); }
    let count = u64::from_le_bytes(fgb_bytes[12..20].try_into().unwrap()) as usize;
    let idx_offset = u64::from_le_bytes(fgb_bytes[20..28].try_into().unwrap()) as usize;
    if feature_idx >= count { return Err(GisBinaryError::TruncatedBuffer); }
    let p = idx_offset + feature_idx * 40;
    let offset = u64::from_le_bytes(fgb_bytes[p + 32..p + 40].try_into().unwrap()) as usize;
    if offset + 36 > fgb_bytes.len() { return Err(GisBinaryError::TruncatedBuffer); }
    let feat_len = u32::from_le_bytes(fgb_bytes[offset..offset + 4].try_into().unwrap()) as usize;
    if feat_len > byte_budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: feat_len, budget: byte_budget }); }
    if offset + 4 + feat_len > fgb_bytes.len() { return Err(GisBinaryError::TruncatedBuffer); }
    let b = &fgb_bytes[offset + 4..offset + 36];
    let bbox = BBox2D::new(
        f64::from_le_bytes(b[0..8].try_into().unwrap()), f64::from_le_bytes(b[8..16].try_into().unwrap()),
        f64::from_le_bytes(b[16..24].try_into().unwrap()), f64::from_le_bytes(b[24..32].try_into().unwrap()),
    );
    let geom = decode_wkb(&fgb_bytes[offset + 36..offset + 4 + feat_len], byte_budget)?;
    Ok((bbox, geom))
}

fn read_var_uint(bytes: &[u8], pos: &mut usize) -> Result<u64, GisBinaryError> {
    let mut res = 0u64;
    let mut shift = 0;
    while *pos < bytes.len() {
        let b = bytes[*pos];
        *pos += 1;
        res |= ((b & 0x7F) as u64) << shift;
        if (b & 0x80) == 0 { return Ok(res); }
        shift += 7;
        if shift >= 64 { return Err(GisBinaryError::TruncatedBuffer); }
    }
    Err(GisBinaryError::TruncatedBuffer)
}

fn write_var_uint(mut val: u64, buf: &mut Vec<u8>) {
    loop {
        let mut b = (val & 0x7F) as u8;
        val >>= 7;
        if val > 0 { b |= 0x80; buf.push(b); } else { buf.push(b); break; }
    }
}

fn read_var_int(bytes: &[u8], pos: &mut usize) -> Result<i64, GisBinaryError> {
    let u = read_var_uint(bytes, pos)?;
    Ok(((u >> 1) as i64) ^ (-((u & 1) as i64)))
}

fn write_var_int(val: i64, buf: &mut Vec<u8>) {
    write_var_uint(((val << 1) ^ (val >> 63)) as u64, buf);
}

pub fn parse_filegdb_tablx_offset(tablx_bytes: &[u8], object_id: usize) -> Result<u64, GisBinaryError> {
    if tablx_bytes.len() < 16 || object_id == 0 { return Err(GisBinaryError::TruncatedBuffer); }
    let pos = 16 + (object_id - 1) * 5;
    if pos + 5 > tablx_bytes.len() { return Err(GisBinaryError::TruncatedBuffer); }
    let b = &tablx_bytes[pos..pos + 5];
    Ok((b[0] as u64) | ((b[1] as u64) << 8) | ((b[2] as u64) << 16) | ((b[3] as u64) << 24) | ((b[4] as u64) << 32))
}

pub fn encode_filegdb_point_or_polygon_row(geom: &SimpleGeom2D, byte_budget: usize) -> Result<Vec<u8>, GisBinaryError> {
    let mut payload = Vec::new();
    match geom {
        SimpleGeom2D::Point(x, y) => {
            write_var_uint(1, &mut payload);
            write_var_int(*x as i64, &mut payload);
            write_var_int(*y as i64, &mut payload);
        }
        SimpleGeom2D::LineString(pts) | SimpleGeom2D::Polygon(pts) => {
            write_var_uint(if matches!(geom, SimpleGeom2D::Polygon(_)) { 13 } else { 3 }, &mut payload);
            write_var_uint(pts.len() as u64, &mut payload);
            write_var_uint(1, &mut payload);
            let mut last_x = 0i64;
            for p in pts { let ix = p[0] as i64; write_var_int(ix - last_x, &mut payload); last_x = ix; }
            let mut last_y = 0i64;
            for p in pts { let iy = p[1] as i64; write_var_int(iy - last_y, &mut payload); last_y = iy; }
        }
    }
    let total_len = 4 + payload.len();
    if total_len > byte_budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: total_len, budget: byte_budget }); }
    let mut row = Vec::with_capacity(total_len);
    row.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    row.extend_from_slice(&payload);
    Ok(row)
}

fn read_filegdb_pts(row: &[u8], pos: &mut usize, budget: usize) -> Result<Vec<[f64; 2]>, GisBinaryError> {
    let count = read_var_uint(row, pos)? as usize;
    if count * 16 > budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: count * 16, budget }); }
    let parts = read_var_uint(row, pos)? as usize;
    for _ in 1..parts { let _ = read_var_uint(row, pos)?; }
    let mut cur_x = 0i64;
    let mut xs = Vec::with_capacity(count);
    for _ in 0..count { cur_x += read_var_int(row, pos)?; xs.push(cur_x as f64); }
    let mut cur_y = 0i64;
    let mut pts = Vec::with_capacity(count);
    for i in 0..count { cur_y += read_var_int(row, pos)?; pts.push([xs[i], cur_y as f64]); }
    Ok(pts)
}

pub fn parse_filegdb_point_or_polygon_row(table_bytes: &[u8], row_offset: usize, byte_budget: usize) -> Result<SimpleGeom2D, GisBinaryError> {
    if row_offset + 4 > table_bytes.len() { return Err(GisBinaryError::TruncatedBuffer); }
    let row_len = u32::from_le_bytes(table_bytes[row_offset..row_offset + 4].try_into().unwrap()) as usize;
    if row_len > byte_budget { return Err(GisBinaryError::ByteBudgetExceeded { requested: row_len, budget: byte_budget }); }
    if row_offset + 4 + row_len > table_bytes.len() { return Err(GisBinaryError::TruncatedBuffer); }
    let row = &table_bytes[row_offset + 4..row_offset + 4 + row_len];
    let mut pos = 0;
    match read_var_uint(row, &mut pos)? {
        1 => {
            let x = read_var_int(row, &mut pos)? as f64;
            let y = read_var_int(row, &mut pos)? as f64;
            Ok(SimpleGeom2D::Point(x, y))
        }
        2 | 3 | 19 => Ok(SimpleGeom2D::LineString(read_filegdb_pts(row, &mut pos, byte_budget)?)),
        4 | 5 | 13 => Ok(SimpleGeom2D::Polygon(read_filegdb_pts(row, &mut pos, byte_budget)?)),
        other => Err(GisBinaryError::UnsupportedGeometryType(other as u32)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpkg_round_trip() {
        let pt = SimpleGeom2D::Point(144.9631, -37.8136);
        let enc = encode_gpkg_geometry(&pt, 4326, true, 1024).expect("enc point");
        let (hdr, dec) = decode_gpkg_geometry(&enc, 1024).expect("dec point");
        assert_eq!(hdr.srs_id, 4326);
        assert_eq!(dec, pt);

        let ls = SimpleGeom2D::LineString(vec![[0.0, 0.0], [1.0, 1.0], [2.0, 0.0]]);
        let enc_ls = encode_gpkg_geometry(&ls, 3857, false, 1024).expect("enc ls");
        assert_eq!(decode_gpkg_geometry(&enc_ls, 1024).expect("dec ls").1, ls);

        let poly = SimpleGeom2D::Polygon(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]);
        let enc_poly = encode_gpkg_geometry(&poly, 4326, true, 1024).expect("enc poly");
        assert_eq!(decode_gpkg_geometry(&enc_poly, 1024).expect("dec poly").1, poly);
    }

    #[test]
    fn test_sqlite_gpkg_file_header() {
        let mut header = vec![0u8; 100];
        header[0..16].copy_from_slice(SQLITE3_HEADER_MAGIC);
        header[16..18].copy_from_slice(&4096u16.to_be_bytes());
        header[68..72].copy_from_slice(&0x4750_4B47u32.to_be_bytes());
        let (pg, app) = validate_sqlite_gpkg_file_header(&header).expect("valid header");
        assert_eq!(pg, 4096);
        assert_eq!(app, 0x4750_4B47);

        header[68..72].copy_from_slice(&0x1234_5678u32.to_be_bytes());
        assert!(validate_sqlite_gpkg_file_header(&header).is_err());
    }

    #[test]
    fn test_fgb_container_and_query() {
        let f1 = (BBox2D::new(0.0, 0.0, 1.0, 1.0), SimpleGeom2D::Point(0.5, 0.5));
        let f2 = (BBox2D::new(10.0, 10.0, 11.0, 11.0), SimpleGeom2D::Point(10.5, 10.5));
        let fgb = encode_fgb_container(&[f1.clone(), f2], 4326, 4096).expect("fgb encode");
        let mut out = [0usize; 8];
        let n1 = query_fgb_bbox_zero_copy(&fgb, &BBox2D::new(-1.0, -1.0, 2.0, 2.0), &mut out).expect("q1");
        assert_eq!(n1, 1);
        let (b_dec, g_dec) = decode_fgb_feature_at(&fgb, out[0], 1024).expect("dec feat");
        assert_eq!(g_dec, SimpleGeom2D::Point(0.5, 0.5));
        assert_eq!(b_dec, f1.0);
        assert!(matches!(encode_fgb_container(&[f1], 4326, 10), Err(GisBinaryError::ByteBudgetExceeded { .. })));
    }

    #[test]
    fn test_filegdb_tablx_and_row() {
        let mut tablx = vec![0u8; 26];
        tablx[0..4].copy_from_slice(&3u32.to_le_bytes());
        tablx[4..8].copy_from_slice(&2u32.to_le_bytes());
        tablx[16..21].copy_from_slice(&100u64.to_le_bytes()[0..5]);
        tablx[21..26].copy_from_slice(&250u64.to_le_bytes()[0..5]);
        assert_eq!(parse_filegdb_tablx_offset(&tablx, 1).expect("oid 1"), 100);
        assert_eq!(parse_filegdb_tablx_offset(&tablx, 2).expect("oid 2"), 250);
        assert!(parse_filegdb_tablx_offset(&tablx, 0).is_err());
        assert!(parse_filegdb_tablx_offset(&tablx, 3).is_err());

        let geom = SimpleGeom2D::Polygon(vec![[10.0, 10.0], [20.0, 10.0], [20.0, 20.0], [10.0, 10.0]]);
        let row = encode_filegdb_point_or_polygon_row(&geom, 1024).expect("enc row");
        assert_eq!(parse_filegdb_point_or_polygon_row(&row, 0, 1024).expect("dec row"), geom);
        let pt = SimpleGeom2D::Point(15.0, 25.0);
        let row_pt = encode_filegdb_point_or_polygon_row(&pt, 1024).expect("enc pt");
        assert_eq!(parse_filegdb_point_or_polygon_row(&row_pt, 0, 1024).expect("dec pt"), pt);
    }
}
