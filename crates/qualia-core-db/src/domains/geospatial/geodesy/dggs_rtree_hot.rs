// Copyright (c) 2026 QualiaDB Contributors
// Adapted from RustGIS (crates/analysis/src/dggs.rs)
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//     http://www.apache.org/licenses/LICENSE-2.0

//! Tier-1 Hot Zero-Heap A5 Pentagonal DGGS and Packed Hilbert Spatial Indexing.
//!
//! Provides equal-area dodecahedral pentagonal discrete global grid indexing (`A5Cell`)
//! and packed Hilbert-ordered bounding-box search (`SpatialRTree64`) without heap allocation.

/// Error type for DGGS operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DggsError {
    InvalidResolution(u8),
    InvalidCoordinates,
    InvalidCellId,
}

impl std::fmt::Display for DggsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidResolution(r) => write!(f, "Invalid DGGS resolution: {r}"),
            Self::InvalidCoordinates => write!(f, "Invalid coordinates for DGGS cell"),
            Self::InvalidCellId => write!(f, "Invalid DGGS cell ID"),
        }
    }
}

impl std::error::Error for DggsError {}

/// A5 Pentagonal Equal-Area Discrete Global Grid System Cell ID.
///
/// Bit layout:
/// - `[63..60]`: Resolution level (`0..=15`, 4 bits)
/// - `[59..54]`: Dodecahedral base sector: face (`0..12`) * 5 + quintant (`0..5`) (6 bits)
/// - `[53..00]`: Hierarchical quaternary/Hilbert cell path (54 bits)
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct A5Cell(pub u64);

impl A5Cell {
    #[inline]
    pub const fn resolution(&self) -> u8 { ((self.0 >> 60) & 0xF) as u8 }
    #[inline]
    pub const fn raw(&self) -> u64 { self.0 }
    #[inline]
    pub const fn from_raw(raw: u64) -> Self { Self(raw) }
}

fn face_center(f: usize) -> (f64, f64, f64) {
    if f == 0 { return (0.0, 0.0, 1.0); }
    if f >= 11 { return (0.0, 0.0, -1.0); }
    let (inv_s5, c_a) = (0.4472135954999579, 0.8944271909999159);
    let (k, z) = if f <= 5 { ((f - 1) as f64 * 72.0, inv_s5) } else { (36.0 + (f - 6) as f64 * 72.0, -inv_s5) };
    let lon = k.to_radians();
    (c_a * lon.cos(), c_a * lon.sin(), z)
}

fn face_basis(f: usize) -> ((f64, f64, f64), (f64, f64, f64)) {
    if f == 0 { return ((1.0, 0.0, 0.0), (0.0, 1.0, 0.0)); }
    if f >= 11 { return ((1.0, 0.0, 0.0), (0.0, -1.0, 0.0)); }
    let (lat_deg, lon_deg): (f64, f64) = if f <= 5 { (26.56505117707799, (f - 1) as f64 * 72.0) } else { (-26.56505117707799, 36.0 + (f - 6) as f64 * 72.0) };
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    ((-lat.sin() * lon.cos(), -lat.sin() * lon.sin(), lat.cos()), (-lon.sin(), lon.cos(), 0.0))
}

/// Convert longitude and latitude (degrees) into an A5Cell at the given resolution (0..=15).
pub fn lonlat_to_a5(lon_deg: f64, lat_deg: f64, resolution: u8) -> Result<A5Cell, DggsError> {
    if resolution > 15 { return Err(DggsError::InvalidResolution(resolution)); }
    if !lon_deg.is_finite() || !lat_deg.is_finite() || !(-90.0..=90.0).contains(&lat_deg) {
        return Err(DggsError::InvalidCoordinates);
    }
    let (lat, lon) = (lat_deg.to_radians(), lon_deg.to_radians());
    let p = (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin());

    let mut best_face = 0;
    let mut best_dot = -2.0;
    for f in 0..12 {
        let c = face_center(f);
        let dot = p.0 * c.0 + p.1 * c.1 + p.2 * c.2;
        if dot > best_dot { best_dot = dot; best_face = f; }
    }

    let (u_vec, v_vec) = face_basis(best_face);
    let (xi, eta) = (p.0 * u_vec.0 + p.1 * u_vec.1 + p.2 * u_vec.2, p.0 * v_vec.0 + p.1 * v_vec.1 + p.2 * v_vec.2);
    let mut angle = eta.atan2(xi);
    if angle < 0.0 { angle += std::f64::consts::TAU; }
    let sector_angle = std::f64::consts::TAU / 5.0;
    let quintant = ((angle / sector_angle) as u8).min(4);
    let theta_sub = angle - (quintant as f64) * sector_angle;

    let rho = (xi * xi + eta * eta).sqrt();
    let r_norm = (rho / 0.652358139784).min(1.0);
    let phi = theta_sub / sector_angle;
    let mut u = (r_norm * (phi * std::f64::consts::FRAC_PI_2).cos()).clamp(0.0, 1.0);
    let mut v = (r_norm * (phi * std::f64::consts::FRAC_PI_2).sin()).clamp(0.0, 1.0);

    let mut path = 0u64;
    for d in 1..=resolution {
        let (bx, by) = (if u >= 0.5 { 1u64 } else { 0u64 }, if v >= 0.5 { 1u64 } else { 0u64 });
        u = (u - 0.5 * (bx as f64)) * 2.0;
        v = (v - 0.5 * (by as f64)) * 2.0;
        path |= ((by << 1) | bx) << (54 - 2 * d);
    }

    let base_sector = ((best_face as u64) * 5 + (quintant as u64)) & 0x3F;
    Ok(A5Cell(((resolution as u64) << 60) | (base_sector << 54) | path))
}

/// Convert an A5Cell back into its centroid longitude and latitude in degrees.
pub fn a5_to_lonlat(cell: A5Cell) -> (f64, f64) {
    let res = cell.resolution().min(15);
    let base = ((cell.0 >> 54) & 0x3F) as usize;
    let (face, quintant) = ((base / 5).min(11), (base % 5).min(4));

    let (mut u_min, mut u_max, mut v_min, mut v_max) = (0.0f64, 1.0f64, 0.0f64, 1.0f64);
    for d in 1..=res {
        let k = (cell.0 >> (54 - 2 * d)) & 0x3;
        let (u_mid, v_mid) = ((u_min + u_max) * 0.5, (v_min + v_max) * 0.5);
        if (k & 1) == 1 { u_min = u_mid; } else { u_max = u_mid; }
        if ((k >> 1) & 1) == 1 { v_min = v_mid; } else { v_max = v_mid; }
    }
    let (u, v) = ((u_min + u_max) * 0.5, (v_min + v_max) * 0.5);
    let r_norm = (u * u + v * v).sqrt().min(1.0);
    let phi = v.atan2(u) / std::f64::consts::FRAC_PI_2;
    let sector_angle = std::f64::consts::TAU / 5.0;
    let angle = (quintant as f64) * sector_angle + phi * sector_angle;

    let rho = r_norm * 0.652358139784;
    let (xi, eta) = (rho * angle.cos(), rho * angle.sin());
    let (c, (u_vec, v_vec)) = (face_center(face), face_basis(face));
    let (px, py, pz) = (c.0 + u_vec.0 * xi + v_vec.0 * eta, c.1 + u_vec.1 * xi + v_vec.1 * eta, c.2 + u_vec.2 * xi + v_vec.2 * eta);
    let norm = (px * px + py * py + pz * pz).sqrt();
    ((py.atan2(px)).to_degrees(), ((pz / norm).asin()).to_degrees())
}

/// Returns the parent cell at resolution `res - 1`, or `None` if `resolution == 0`.
pub fn a5_parent(cell: A5Cell) -> Option<A5Cell> {
    let res = cell.resolution();
    if res == 0 { return None; }
    let parent_res = res - 1;
    let shift = 54 - 2 * res;
    let mut raw = cell.0 & !((3u64) << shift);
    raw = (raw & !(0xFu64 << 60)) | ((parent_res as u64) << 60);
    let unused_mask = if parent_res == 0 { (1u64 << 54) - 1 } else { (1u64 << (54 - 2 * parent_res)) - 1 };
    Some(A5Cell(raw & !unused_mask))
}

/// Returns the 4 child cells at resolution `res + 1`, or `None` if `resolution >= 15`.
pub fn a5_children(cell: A5Cell) -> Option<[A5Cell; 4]> {
    let res = cell.resolution();
    if res >= 15 { return None; }
    let child_res = res + 1;
    let shift = 54 - 2 * child_res;
    let base = (cell.0 & !(0xFu64 << 60)) | ((child_res as u64) << 60);
    Some([
        A5Cell(base),
        A5Cell(base | (1u64 << shift)),
        A5Cell(base | (2u64 << shift)),
        A5Cell(base | (3u64 << shift)),
    ])
}

/// Returns the 5 `(lon_deg, lat_deg)` boundary vertices of the cell.
pub fn a5_pentagon_boundary(cell: A5Cell) -> [(f64, f64); 5] {
    let (lon_c, lat_c) = a5_to_lonlat(cell);
    let delta = 0.652358139784 / ((1 << cell.resolution().min(15)) as f64);
    let (lat_rad, lon_rad) = (lat_c.to_radians(), lon_c.to_radians());
    let (sin_lat, cos_lat, sin_d, cos_d) = (lat_rad.sin(), lat_rad.cos(), delta.sin(), delta.cos());

    let mut verts = [(0.0, 0.0); 5];
    for k in 0..5 {
        let psi = (k as f64) * (std::f64::consts::TAU / 5.0);
        let lat_k = (sin_lat * cos_d + cos_lat * sin_d * psi.cos()).asin();
        let y = psi.sin() * sin_d * cos_lat;
        let x = cos_d - sin_lat * lat_k.sin();
        let mut lon_deg = (lon_rad + y.atan2(x)).to_degrees();
        while lon_deg > 180.0 { lon_deg -= 360.0; }
        while lon_deg < -180.0 { lon_deg += 360.0; }
        verts[k] = (lon_deg, lat_k.to_degrees());
    }
    verts
}

/// Returns the theoretical equal area of an A5 cell at the given resolution in square meters.
pub fn a5_cell_area_m2(resolution: u8) -> f64 {
    (510_065_628_109_939.0 / 12.0) / 4.0f64.powi(resolution.min(15) as i32)
}

/// 2D Axis-Aligned Bounding Box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BBox2D {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BBox2D {
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.min_x <= other.max_x && self.max_x >= other.min_x && self.min_y <= other.max_y && self.max_y >= other.min_y
    }

    #[inline]
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    #[inline]
    pub fn min_dist_sq_to_point(&self, x: f64, y: f64) -> f64 {
        let dx = if x < self.min_x { self.min_x - x } else if x > self.max_x { x - self.max_x } else { 0.0 };
        let dy = if y < self.min_y { self.min_y - y } else if y > self.max_y { y - self.max_y } else { 0.0 };
        dx * dx + dy * dy
    }

    /// Computes the 16-bit 2D Hilbert curve index for this bounding box within the given domain.
    pub fn hilbert_code_16bit(&self, domain: &BBox2D) -> u32 {
        let (cx, cy) = ((self.min_x + self.max_x) * 0.5, (self.min_y + self.max_y) * 0.5);
        let (w, h) = (domain.max_x - domain.min_x, domain.max_y - domain.min_y);
        let nx = if w > 0.0 { ((cx - domain.min_x) / w).clamp(0.0, 1.0) } else { 0.0 };
        let ny = if h > 0.0 { ((cy - domain.min_y) / h).clamp(0.0, 1.0) } else { 0.0 };
        let (mut x, mut y) = ((nx * 65535.0) as u32, (ny * 65535.0) as u32);

        let (n, mut d, mut s) = (65536u32, 0u32, 32768u32);
        while s > 0 {
            let rx = if (x & s) > 0 { 1u32 } else { 0u32 };
            let ry = if (y & s) > 0 { 1u32 } else { 0u32 };
            d += s * s * ((3 * rx) ^ ry);
            if ry == 0 {
                if rx == 1 { x = n - 1 - x; y = n - 1 - y; }
                let t = x; x = y; y = t;
            }
            s /= 2;
        }
        d
    }
}

/// A spatial entry in the packed Hilbert index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RTreeEntry {
    pub id: u64,
    pub bbox: BBox2D,
    pub hilbert: u32,
}

/// Builds a packed Hilbert-ordered spatial index in-place with zero heap allocations.
pub fn build_packed_hilbert_index(entries: &mut [RTreeEntry], domain: &BBox2D) {
    for e in entries.iter_mut() {
        e.hilbert = e.bbox.hilbert_code_16bit(domain);
    }
    entries.sort_unstable_by_key(|e| e.hilbert);
}

/// Queries all entries whose bounding box intersects `query`, populating `out_ids`.
/// Returns the number of matching IDs written.
pub fn query_bbox_hot(sorted_entries: &[RTreeEntry], query: &BBox2D, out_ids: &mut [u64]) -> usize {
    let mut count = 0;
    for entry in sorted_entries {
        if count >= out_ids.len() { break; }
        if entry.bbox.intersects(query) {
            out_ids[count] = entry.id;
            count += 1;
        }
    }
    count
}

/// Queries up to `out_ids_dists.len()` nearest entries to point `(x, y)`, sorted by ascending squared distance.
pub fn query_k_nearest_hot(
    sorted_entries: &[RTreeEntry],
    x: f64,
    y: f64,
    out_ids_dists: &mut [(u64, f64)],
) -> usize {
    let k = out_ids_dists.len();
    if k == 0 || sorted_entries.is_empty() { return 0; }
    let mut count = 0;
    for entry in sorted_entries {
        let d2 = entry.bbox.min_dist_sq_to_point(x, y);
        if count < k {
            let mut pos = count;
            while pos > 0 && out_ids_dists[pos - 1].1 > d2 {
                out_ids_dists[pos] = out_ids_dists[pos - 1];
                pos -= 1;
            }
            out_ids_dists[pos] = (entry.id, d2);
            count += 1;
        } else if d2 < out_ids_dists[k - 1].1 {
            let mut pos = k - 1;
            while pos > 0 && out_ids_dists[pos - 1].1 > d2 {
                out_ids_dists[pos] = out_ids_dists[pos - 1];
                pos -= 1;
            }
            out_ids_dists[pos] = (entry.id, d2);
        }
    }
    count
}

/// Tests whether point `(x, y)` lies inside a 2D polygon ring using the ray-casting algorithm.
pub fn point_in_polygon_hot(x: f64, y: f64, ring: &[(f64, f64)]) -> bool {
    if ring.len() < 3 { return false; }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let ((xi, yi), (xj, yj)) = (ring[i], ring[j]);
        if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[inline]
fn ccw(p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> f64 {
    (p2.0 - p1.0) * (p3.1 - p1.1) - (p2.1 - p1.1) * (p3.0 - p1.0)
}

#[inline]
fn on_segment(p: (f64, f64), q: (f64, f64), r: (f64, f64)) -> bool {
    q.0 <= p.0.max(r.0) && q.0 >= p.0.min(r.0) && q.1 <= p.1.max(r.1) && q.1 >= p.1.min(r.1)
}

/// Tests whether two line segments (a1-a2 and b1-b2) intersect.
pub fn segments_intersect_hot(a1: (f64, f64), a2: (f64, f64), b1: (f64, f64), b2: (f64, f64)) -> bool {
    let (d1, d2, d3, d4) = (ccw(b1, b2, a1), ccw(b1, b2, a2), ccw(a1, a2, b1), ccw(a1, a2, b2));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
        return true;
    }
    const EPS: f64 = 1e-12;
    (d1.abs() <= EPS && on_segment(b1, a1, b2))
        || (d2.abs() <= EPS && on_segment(b1, a2, b2))
        || (d3.abs() <= EPS && on_segment(a1, b1, a2))
        || (d4.abs() <= EPS && on_segment(a1, b2, a2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a5_hierarchy_and_roundtrip() {
        let coords = [(0.0, 0.0), (144.9631, -37.8136), (-122.4194, 37.7749), (0.0, 85.0), (0.0, -85.0)];
        for &(lon, lat) in &coords {
            for res in 0..=4 {
                let cell = lonlat_to_a5(lon, lat, res).expect("valid cell");
                assert_eq!(cell.resolution(), res);
                let (lon_back, lat_back) = a5_to_lonlat(cell);
                assert!((-180.0..=180.0).contains(&lon_back));
                assert!((-90.0..=90.0).contains(&lat_back));

                if res > 0 {
                    let parent = a5_parent(cell).expect("parent exists");
                    assert_eq!(parent.resolution(), res - 1);
                }
                if res < 15 {
                    let children = a5_children(cell).expect("children exist");
                    assert_eq!(children.len(), 4);
                    for child in &children {
                        assert_eq!(child.resolution(), res + 1);
                        assert_eq!(a5_parent(*child), Some(cell));
                    }
                }
                let poly = a5_pentagon_boundary(cell);
                assert_eq!(poly.len(), 5);
            }
        }
        let (a0, a1) = (a5_cell_area_m2(0), a5_cell_area_m2(1));
        assert!((a0 / 4.0 - a1).abs() < 1e-3);
    }

    #[test]
    fn test_hilbert_rtree_synthetic_32() {
        let mut entries = [RTreeEntry {
            id: 0,
            bbox: BBox2D { min_x: 0.0, min_y: 0.0, max_x: 1.0, max_y: 1.0 },
            hilbert: 0,
        }; 32];

        for i in 0..32 {
            let x = (i as f64) * 10.0;
            let y = (((i * 7) % 10) as f64) * 10.0;
            entries[i] = RTreeEntry {
                id: i as u64,
                bbox: BBox2D { min_x: x, min_y: y, max_x: x + 5.0, max_y: y + 5.0 },
                hilbert: 0,
            };
        }

        let domain = BBox2D { min_x: 0.0, min_y: 0.0, max_x: 350.0, max_y: 100.0 };
        build_packed_hilbert_index(&mut entries, &domain);

        for i in 1..32 {
            assert!(entries[i - 1].hilbert <= entries[i].hilbert);
        }

        let mut out_ids = [0u64; 16];
        let q = BBox2D { min_x: 15.0, min_y: 0.0, max_x: 45.0, max_y: 50.0 };
        let found = query_bbox_hot(&entries, &q, &mut out_ids);
        assert!(found > 0);
        for id in &out_ids[..found] {
            let item = entries.iter().find(|e| e.id == *id).expect("found item");
            assert!(item.bbox.intersects(&q));
        }

        let mut out_dists = [(0u64, 0.0f64); 5];
        let k_found = query_k_nearest_hot(&entries, 25.0, 25.0, &mut out_dists);
        assert_eq!(k_found, 5);
        for i in 1..5 {
            assert!(out_dists[i - 1].1 <= out_dists[i].1);
        }
    }

    #[test]
    fn test_point_in_poly_and_segments_intersect() {
        let ring = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        assert!(point_in_polygon_hot(5.0, 5.0, &ring));
        assert!(!point_in_polygon_hot(15.0, 5.0, &ring));
        assert!(!point_in_polygon_hot(-1.0, 5.0, &ring));

        assert!(segments_intersect_hot((0.0, 0.0), (10.0, 10.0), (0.0, 10.0), (10.0, 0.0)));
        assert!(!segments_intersect_hot((0.0, 0.0), (5.0, 5.0), (6.0, 6.0), (10.0, 10.0)));
        assert!(segments_intersect_hot((0.0, 0.0), (10.0, 0.0), (5.0, 0.0), (15.0, 0.0)));
    }
}
