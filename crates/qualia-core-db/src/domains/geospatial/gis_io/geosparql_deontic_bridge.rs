// Copyright 2026 QualiaDB Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! OGC GeoSPARQL 1.1 DE-9IM Topological Engine & 48-Byte NQuin Deontic Spatial Norm Bridge.
//! Implements Tier-1 HotZeroHeap DE-9IM matrix computation, GeoSPARQL predicates, and
//! spatial deontic norm covenant evaluation over 48-byte Super-Quins.

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct De9imMatrix {
    pub cells: [u8; 9],
}

impl De9imMatrix {
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 9] {
        &self.cells
    }

    pub fn matches_pattern(&self, pattern: &[u8; 9]) -> bool {
        for i in 0..9 {
            let (pat, cell) = (pattern[i], self.cells[i]);
            let ok = match pat {
                b'*' => true,
                b'T' | b't' => matches!(cell, b'0' | b'1' | b'2'),
                b'F' | b'f' => cell == b'F',
                b'0' | b'1' | b'2' => cell == pat,
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PointPolyLocation {
    Interior,
    Boundary,
    Exterior,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SegmentIntersection {
    None,
    Point([f64; 2], bool),
    CollinearOverlap,
}

#[inline]
fn poly_vertex_count(poly: &[[f64; 2]]) -> usize {
    if poly.len() < 3 {
        return 0;
    }
    let last = poly.len() - 1;
    let (dx, dy) = (poly[0][0] - poly[last][0], poly[0][1] - poly[last][1]);
    if (dx * dx + dy * dy) < 1e-14 { last } else { poly.len() }
}

#[inline]
fn point_to_segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (vx, vy) = (b[0] - a[0], b[1] - a[1]);
    let (wx, wy) = (p[0] - a[0], p[1] - a[1]);
    let c1 = wx * vx + wy * vy;
    if c1 <= 0.0 {
        return (wx * wx + wy * wy).sqrt();
    }
    let c2 = vx * vx + vy * vy;
    if c2 <= c1 {
        let (dx, dy) = (p[0] - b[0], p[1] - b[1]);
        return (dx * dx + dy * dy).sqrt();
    }
    let t = c1 / c2;
    let (dx, dy) = (p[0] - (a[0] + t * vx), p[1] - (a[1] + t * vy));
    (dx * dx + dy * dy).sqrt()
}

fn point_in_polygon_ring(pt: [f64; 2], poly: &[[f64; 2]], eps: f64) -> PointPolyLocation {
    let n = poly_vertex_count(poly);
    if n < 3 {
        return PointPolyLocation::Exterior;
    }
    for i in 0..n {
        if point_to_segment_distance(pt, poly[i], poly[(i + 1) % n]) <= eps {
            return PointPolyLocation::Boundary;
        }
    }
    let mut inside = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a[1] > pt[1]) != (b[1] > pt[1]) {
            let ix = a[0] + (pt[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if pt[0] < ix {
                inside = !inside;
            }
        }
    }
    if inside { PointPolyLocation::Interior } else { PointPolyLocation::Exterior }
}

fn polygon_centroid(poly: &[[f64; 2]], n: usize) -> [f64; 2] {
    let (mut area2, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let cross = a[0] * b[1] - b[0] * a[1];
        area2 += cross;
        cx += (a[0] + b[0]) * cross;
        cy += (a[1] + b[1]) * cross;
    }
    let signed_area = area2 * 0.5;
    if signed_area.abs() > 1e-12 {
        [cx / (6.0 * signed_area), cy / (6.0 * signed_area)]
    } else {
        let (mut sx, mut sy) = (0.0, 0.0);
        for v in poly.iter().take(n) {
            sx += v[0];
            sy += v[1];
        }
        [sx / n as f64, sy / n as f64]
    }
}

fn find_interior_point(poly: &[[f64; 2]], n: usize) -> Option<[f64; 2]> {
    let c = polygon_centroid(poly, n);
    if point_in_polygon_ring(c, poly, 1e-7) == PointPolyLocation::Interior {
        return Some(c);
    }
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (mx, my) = ((a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1e-8 {
            let (nx, ny) = (-dy / len, dx / len);
            for factor in [1e-4, -1e-4, 1e-2, -1e-2] {
                let cand = [mx + nx * factor, my + ny * factor];
                if point_in_polygon_ring(cand, poly, 1e-7) == PointPolyLocation::Interior {
                    return Some(cand);
                }
            }
        }
    }
    None
}

fn intersect_segments(p1: [f64; 2], p2: [f64; 2], q1: [f64; 2], q2: [f64; 2], eps: f64) -> SegmentIntersection {
    let (rx, ry) = (p2[0] - p1[0], p2[1] - p1[1]);
    let (sx, sy) = (q2[0] - q1[0], q2[1] - q1[1]);
    let r_cross_s = rx * sy - ry * sx;
    let (qp_x, qp_y) = (q1[0] - p1[0], q1[1] - p1[1]);
    let qp_cross_r = qp_x * ry - qp_y * rx;

    if r_cross_s.abs() <= eps {
        if qp_cross_r.abs() <= eps {
            let r_len_sq = rx * rx + ry * ry;
            if r_len_sq <= eps * eps {
                return SegmentIntersection::None;
            }
            let t0 = (qp_x * rx + qp_y * ry) / r_len_sq;
            let t1 = t0 + (sx * rx + sy * ry) / r_len_sq;
            let (ov_s, ov_e) = (0.0_f64.max(t0.min(t1)), 1.0_f64.min(t0.max(t1)));
            if ov_e - ov_s > eps {
                return SegmentIntersection::CollinearOverlap;
            } else if (ov_e - ov_s).abs() <= eps && ov_s <= 1.0 + eps && ov_e >= -eps {
                let t_pt = ov_s.clamp(0.0, 1.0);
                return SegmentIntersection::Point([p1[0] + t_pt * rx, p1[1] + t_pt * ry], false);
            }
        }
        SegmentIntersection::None
    } else {
        let t = (qp_x * sy - qp_y * sx) / r_cross_s;
        let u = (qp_x * ry - qp_y * rx) / r_cross_s;
        if (-eps..=1.0 + eps).contains(&t) && (-eps..=1.0 + eps).contains(&u) {
            let pt = [p1[0] + t.clamp(0.0, 1.0) * rx, p1[1] + t.clamp(0.0, 1.0) * ry];
            let trans = t > eps && t < 1.0 - eps && u > eps && u < 1.0 - eps;
            SegmentIntersection::Point(pt, trans)
        } else {
            SegmentIntersection::None
        }
    }
}

pub fn compute_polygon_de9im(poly_a: &[[f64; 2]], poly_b: &[[f64; 2]]) -> De9imMatrix {
    let (n_a, n_b) = (poly_vertex_count(poly_a), poly_vertex_count(poly_b));
    if n_a < 3 || n_b < 3 {
        return De9imMatrix { cells: *b"FFFFFFFFF" };
    }
    let eps = 1e-7;
    let (mut bb_dim, mut transverse_crossing) = (-1i8, false);

    for i in 0..n_a {
        let (p1, p2) = (poly_a[i], poly_a[(i + 1) % n_a]);
        for j in 0..n_b {
            match intersect_segments(p1, p2, poly_b[j], poly_b[(j + 1) % n_b], eps) {
                SegmentIntersection::CollinearOverlap => bb_dim = 1,
                SegmentIntersection::Point(_, trans) => {
                    if bb_dim < 0 { bb_dim = 0; }
                    if trans { transverse_crossing = true; }
                }
                SegmentIntersection::None => {}
            }
        }
    }

    let (mut va_in, mut va_ex, mut va_bd, mut ma_in, mut ma_ex) = (false, false, false, false, false);
    for i in 0..n_a {
        let (v, nv) = (poly_a[i], poly_a[(i + 1) % n_a]);
        match point_in_polygon_ring(v, poly_b, eps) {
            PointPolyLocation::Interior => va_in = true,
            PointPolyLocation::Exterior => va_ex = true,
            PointPolyLocation::Boundary => va_bd = true,
        }
        match point_in_polygon_ring([(v[0] + nv[0]) * 0.5, (v[1] + nv[1]) * 0.5], poly_b, eps) {
            PointPolyLocation::Interior => ma_in = true,
            PointPolyLocation::Exterior => ma_ex = true,
            PointPolyLocation::Boundary => {}
        }
    }

    let (mut vb_in, mut vb_ex, mut vb_bd, mut mb_in, mut mb_ex) = (false, false, false, false, false);
    for j in 0..n_b {
        let (v, nv) = (poly_b[j], poly_b[(j + 1) % n_b]);
        match point_in_polygon_ring(v, poly_a, eps) {
            PointPolyLocation::Interior => vb_in = true,
            PointPolyLocation::Exterior => vb_ex = true,
            PointPolyLocation::Boundary => vb_bd = true,
        }
        match point_in_polygon_ring([(v[0] + nv[0]) * 0.5, (v[1] + nv[1]) * 0.5], poly_a, eps) {
            PointPolyLocation::Interior => mb_in = true,
            PointPolyLocation::Exterior => mb_ex = true,
            PointPolyLocation::Boundary => {}
        }
    }

    let ia_in_b = find_interior_point(poly_a, n_a)
        .map(|pt| point_in_polygon_ring(pt, poly_b, eps) == PointPolyLocation::Interior)
        .unwrap_or(false);
    let ib_in_a = find_interior_point(poly_b, n_b)
        .map(|pt| point_in_polygon_ring(pt, poly_a, eps) == PointPolyLocation::Interior)
        .unwrap_or(false);

    let bi = va_in || ma_in || transverse_crossing;
    let ib = vb_in || mb_in || transverse_crossing;
    let be = va_ex || ma_ex || transverse_crossing;
    let eb = vb_ex || mb_ex || transverse_crossing;
    let bb = if bb_dim == 1 { b'1' } else if bb_dim == 0 || va_bd || vb_bd { b'0' } else { b'F' };
    let ii = transverse_crossing || (bi && ib) || va_in || vb_in || ia_in_b || ib_in_a;

    De9imMatrix {
        cells: [
            if ii { b'2' } else { b'F' },
            if ib { b'1' } else { b'F' },
            if be { b'2' } else { b'F' },
            if bi { b'1' } else { b'F' },
            bb,
            if be { b'1' } else { b'F' },
            if eb { b'2' } else { b'F' },
            if eb { b'1' } else { b'F' },
            b'2',
        ],
    }
}

#[inline] pub fn geof_sf_equals(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool { compute_polygon_de9im(a, b).matches_pattern(b"T*F**FFF*") }
#[inline] pub fn geof_sf_disjoint(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool { compute_polygon_de9im(a, b).matches_pattern(b"FF*FF****") }
#[inline] pub fn geof_sf_intersects(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool { !geof_sf_disjoint(a, b) }
#[inline] pub fn geof_sf_within(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool { compute_polygon_de9im(a, b).matches_pattern(b"T*F**F***") }
#[inline] pub fn geof_sf_contains(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool { compute_polygon_de9im(a, b).matches_pattern(b"T*****FF*") }
#[inline] pub fn geof_sf_overlaps(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool { compute_polygon_de9im(a, b).matches_pattern(b"T*T***T**") }
#[inline]
pub fn geof_sf_touches(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    let m = compute_polygon_de9im(a, b);
    m.matches_pattern(b"FT*******") || m.matches_pattern(b"F**T*****") || m.matches_pattern(b"F***T****")
}

pub fn geof_distance_polygons(a: &[[f64; 2]], b: &[[f64; 2]]) -> f64 {
    if geof_sf_intersects(a, b) { return 0.0; }
    let (n_a, n_b) = (poly_vertex_count(a), poly_vertex_count(b));
    if n_a < 3 || n_b < 3 { return 0.0; }
    let mut min_dist = f64::MAX;
    for i in 0..n_a {
        for j in 0..n_b {
            min_dist = min_dist.min(point_to_segment_distance(a[i], b[j], b[(j + 1) % n_b]));
            min_dist = min_dist.min(point_to_segment_distance(b[j], a[i], a[(i + 1) % n_a]));
        }
    }
    min_dist
}

pub const OP_OBLIGATE: u8 = 0x10;
pub const OP_PERMIT: u8 = 0x11;
pub const OP_FORBID: u8 = 0x12;
pub const DEFEATER_BIT: u64 = 1u64 << 63;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpatialQuin48 {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

const _: () = assert!(core::mem::size_of::<SpatialQuin48>() == 48);

pub fn mint_spatial_deontic_quin(
    actor_did_hash: u64, action_hash: u64, opcode: u8, is_defeater: bool,
    zone_a5_cell: u64, covenant_did_hash: u64, expiry_epoch: u32, lamport_clock: u32,
) -> SpatialQuin48 {
    let mut predicate = ((action_hash & 0x007F_FFFF_FFFF_FFFF) << 8) | (opcode as u64);
    if is_defeater { predicate |= DEFEATER_BIT; }
    let metadata = ((lamport_clock as u64 & 0x1FFF_FFFF) << 32) | (expiry_epoch as u64);
    let parity = actor_did_hash ^ predicate ^ zone_a5_cell ^ covenant_did_hash;
    SpatialQuin48 { subject: actor_did_hash, predicate, object: zone_a5_cell, context: covenant_did_hash, metadata, parity }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialNormVerdict {
    Permitted,
    Obligated,
    ForbiddenByCovenant { defeater_quin: SpatialQuin48 },
    ExpiredOrNotApplicable,
}

#[derive(Debug, Clone, Copy)]
pub struct SpatialCovenantZone<'a> {
    pub quin: SpatialQuin48,
    pub boundary_ring: &'a [[f64; 2]],
}

pub fn evaluate_spatial_deontic_covenants(
    proposed_footprint: &[[f64; 2]], actor_did_hash: u64, action_hash: u64,
    current_epoch: u32, zones: &[SpatialCovenantZone<'_>],
) -> SpatialNormVerdict {
    let (mut defeater_forbid, mut standard_forbid) = (None, None);
    let (mut has_obligate, mut has_permit) = (false, false);
    let target_action = action_hash & 0x007F_FFFF_FFFF_FFFF;

    for zone in zones {
        let q = zone.quin;
        if q.parity != (q.subject ^ q.predicate ^ q.object ^ q.context) { continue; }
        if ((q.metadata & 0xFFFF_FFFF) as u32) < current_epoch { continue; }
        if q.subject != 0 && q.subject != actor_did_hash { continue; }
        let q_act = (q.predicate >> 8) & 0x007F_FFFF_FFFF_FFFF;
        if q_act != 0 && q_act != target_action { continue; }
        if !geof_sf_intersects(proposed_footprint, zone.boundary_ring) { continue; }

        let (opcode, is_def) = ((q.predicate & 0xFF) as u8, (q.predicate & DEFEATER_BIT) != 0);
        match opcode {
            OP_FORBID => if is_def { defeater_forbid = Some(q); } else if standard_forbid.is_none() { standard_forbid = Some(q); },
            OP_OBLIGATE => has_obligate = true,
            OP_PERMIT => has_permit = true,
            _ => {}
        }
    }

    if let Some(dq) = defeater_forbid { SpatialNormVerdict::ForbiddenByCovenant { defeater_quin: dq } }
    else if let Some(fq) = standard_forbid { SpatialNormVerdict::ForbiddenByCovenant { defeater_quin: fq } }
    else if has_obligate { SpatialNormVerdict::Obligated }
    else if has_permit { SpatialNormVerdict::Permitted }
    else { SpatialNormVerdict::ExpiredOrNotApplicable }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_de9im_and_sf_predicates_all() {
        let sq_a = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];
        let sq_disjoint = [[4.0, 0.0], [6.0, 0.0], [6.0, 2.0], [4.0, 2.0]];
        let sq_touch = [[2.0, 0.0], [4.0, 0.0], [4.0, 2.0], [2.0, 2.0]];
        let sq_overlap = [[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0]];
        let sq_small = [[0.5, 0.5], [1.5, 0.5], [1.5, 1.5], [0.5, 1.5]];
        let sq_identical = [[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0]];

        assert!(geof_sf_disjoint(&sq_a, &sq_disjoint) && !geof_sf_intersects(&sq_a, &sq_disjoint));
        assert!((geof_distance_polygons(&sq_a, &sq_disjoint) - 2.0).abs() < 1e-9);
        assert!(geof_sf_touches(&sq_a, &sq_touch) && geof_sf_intersects(&sq_a, &sq_touch) && !geof_sf_overlaps(&sq_a, &sq_touch));
        assert!(geof_sf_overlaps(&sq_a, &sq_overlap) && !geof_sf_within(&sq_a, &sq_overlap));
        assert!(geof_sf_within(&sq_small, &sq_a) && geof_sf_contains(&sq_a, &sq_small));
        assert!(geof_sf_equals(&sq_a, &sq_identical) && !geof_sf_overlaps(&sq_a, &sq_identical));
    }

    #[test]
    fn test_spatial_deontic_covenant_resolution() {
        let (actor, action) = (0xA110_CAFE_0000_0001, 0x0000_B001_D000_0000);
        let permit_quin = mint_spatial_deontic_quin(actor, action, OP_PERMIT, false, 0x100, 0xC001, 5000, 10);
        let forbid_defeater = mint_spatial_deontic_quin(0, action, OP_FORBID, true, 0x200, 0xAB06, 5000, 11);

        let permit_ring = [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let bog_ring = [[2.0, 2.0], [5.0, 2.0], [5.0, 5.0], [2.0, 5.0]];
        let zones = [
            SpatialCovenantZone { quin: permit_quin, boundary_ring: &permit_ring },
            SpatialCovenantZone { quin: forbid_defeater, boundary_ring: &bog_ring },
        ];

        let encroaching = [[3.0, 3.0], [4.0, 3.0], [4.0, 4.0], [3.0, 4.0]];
        assert_eq!(evaluate_spatial_deontic_covenants(&encroaching, actor, action, 1000, &zones),
                   SpatialNormVerdict::ForbiddenByCovenant { defeater_quin: forbid_defeater });

        let clear = [[7.0, 7.0], [8.0, 7.0], [8.0, 8.0], [7.0, 8.0]];
        assert_eq!(evaluate_spatial_deontic_covenants(&clear, actor, action, 1000, &zones), SpatialNormVerdict::Permitted);
        assert_eq!(evaluate_spatial_deontic_covenants(&clear, actor, action, 6000, &zones), SpatialNormVerdict::ExpiredOrNotApplicable);
    }
}
