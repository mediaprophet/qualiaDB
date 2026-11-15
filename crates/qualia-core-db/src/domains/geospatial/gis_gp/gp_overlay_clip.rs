// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) RustGIS contributors (adapted under MIT License)
//! Boolean Polygon & Polyline Overlay Geoprocessing Tools.
//! Provides Sutherland-Hodgman / Weiler-Atherton / Greiner-Hormann 2D vector
//! clipping, boolean set operations, polyline clipping, and Identity/Update overlay.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpOverlayError {
    DegeneratePolygon,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

pub type Ring2D = Vec<[f64; 2]>;

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayFeature { pub id: u64, pub ring: Ring2D, pub attr_tag: u64 }

#[inline]
fn get_open_ring(ring: &[[f64; 2]]) -> &[[f64; 2]] {
    if ring.len() >= 2 {
        let (first, last) = (ring[0], ring[ring.len() - 1]);
        if (first[0] - last[0]).abs() < 1e-12 && (first[1] - last[1]).abs() < 1e-12 {
            return &ring[..ring.len() - 1];
        }
    }
    ring
}

/// Compute signed shoelace area (positive for CCW orientation).
#[inline]
pub fn ring_signed_area(ring: &[[f64; 2]]) -> f64 {
    let r = get_open_ring(ring);
    let n = r.len();
    if n < 3 { return 0.0; }
    let mut sum = 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        sum += r[i][0] * r[j][1] - r[j][0] * r[i][1];
    }
    0.5 * sum
}

/// Compute absolute ring area.
#[inline]
pub fn ring_area(ring: &[[f64; 2]]) -> f64 { ring_signed_area(ring).abs() }

/// Point-in-polygon ray-casting test (even-odd rule).
pub fn point_in_ring(pt: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let r = get_open_ring(ring);
    let n = r.len();
    if n < 3 { return false; }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (r[i][0], r[i][1]);
        let (xj, yj) = (r[j][0], r[j][1]);
        if ((yi > pt[1]) != (yj > pt[1])) && (pt[0] < (xj - xi) * (pt[1] - yi) / (yj - yi) + xi) {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[inline]
fn ensure_ccw(ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut v = ring.to_vec();
    if ring_signed_area(&v) < 0.0 { v.reverse(); }
    v
}

#[derive(Clone, Debug)]
struct GhNode {
    pt: [f64; 2], is_intersection: bool, entry: bool, visited: bool,
    partner: usize, next: usize, prev: usize,
}

struct RawIntersection { edge_a: usize, t_a: f64, edge_b: usize, t_b: f64, pt: [f64; 2] }

#[derive(Copy, Clone, PartialEq, Eq)]
enum BooleanOp { Intersection, Union, Difference }

fn segment_intersection(p1: [f64; 2], p2: [f64; 2], q1: [f64; 2], q2: [f64; 2]) -> Option<(f64, f64, [f64; 2])> {
    let (rx, ry) = (p2[0] - p1[0], p2[1] - p1[1]);
    let (sx, sy) = (q2[0] - q1[0], q2[1] - q1[1]);
    let denom = rx * sy - ry * sx;
    if denom.abs() < 1e-12 { return None; }
    let (qpx, qpy) = (q1[0] - p1[0], q1[1] - p1[1]);
    let (t, u) = ((qpx * sy - qpy * sx) / denom, (qpx * ry - qpy * rx) / denom);
    let eps = 1e-9;
    if t > eps && t < 1.0 - eps && u > eps && u < 1.0 - eps {
        Some((t, u, [p1[0] + t * rx, p1[1] + t * ry]))
    } else {
        None
    }
}

fn build_extended_nodes(
    ccw: &[[f64; 2]],
    raw_inters: &[RawIntersection],
    use_edge_a: bool,
    partner_map: &mut [usize],
) -> Vec<GhNode> {
    let n = ccw.len();
    let mut nodes: Vec<GhNode> = ccw.iter().map(|&pt| GhNode {
        pt, is_intersection: false, entry: false, visited: false, partner: 0, next: 0, prev: 0,
    }).collect();
    for i in 0..n { nodes[i].next = (i + 1) % n; nodes[i].prev = (i + n - 1) % n; }
    for i in 0..n {
        let mut on_edge: Vec<(usize, f64)> = raw_inters.iter().enumerate()
            .filter(|(_, inter)| if use_edge_a { inter.edge_a == i } else { inter.edge_b == i })
            .map(|(idx, inter)| (idx, if use_edge_a { inter.t_a } else { inter.t_b })).collect();
        on_edge.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let mut curr = i;
        for (inter_idx, _) in on_edge {
            let (new_idx, next_node) = (nodes.len(), nodes[curr].next);
            nodes.push(GhNode {
                pt: raw_inters[inter_idx].pt, is_intersection: true, entry: false,
                visited: false, partner: 0, next: next_node, prev: curr,
            });
            nodes[curr].next = new_idx; nodes[next_node].prev = new_idx;
            curr = new_idx; partner_map[inter_idx] = new_idx;
        }
    }
    nodes
}

fn execute_boolean_op(
    poly_a: &[[f64; 2]],
    poly_b: &[[f64; 2]],
    op: BooleanOp,
    byte_budget: usize,
) -> Result<Vec<Ring2D>, GpOverlayError> {
    let (open_a, open_b) = (get_open_ring(poly_a), get_open_ring(poly_b));
    if open_a.len() < 3 || ring_area(open_a) < 1e-12 || open_b.len() < 3 || ring_area(open_b) < 1e-12 {
        return Err(GpOverlayError::DegeneratePolygon);
    }
    let initial_est = (open_a.len() + open_b.len()) * 32 + 64;
    if initial_est > byte_budget {
        return Err(GpOverlayError::ByteBudgetExceeded { requested: initial_est, budget: byte_budget });
    }
    let (ccw_a, ccw_b) = (ensure_ccw(open_a), ensure_ccw(open_b));
    let (na, nb) = (ccw_a.len(), ccw_b.len());

    let mut raw_inters = Vec::new();
    for i in 0..na {
        let (a1, a2) = (ccw_a[i], ccw_a[(i + 1) % na]);
        for j in 0..nb {
            let (b1, b2) = (ccw_b[j], ccw_b[(j + 1) % nb]);
            if let Some((ta, tb, pt)) = segment_intersection(a1, a2, b1, b2) {
                raw_inters.push(RawIntersection { edge_a: i, t_a: ta, edge_b: j, t_b: tb, pt });
            }
        }
    }

    if raw_inters.is_empty() {
        let (a_in_b, b_in_a) = (point_in_ring(ccw_a[0], &ccw_b), point_in_ring(ccw_b[0], &ccw_a));
        let result = match op {
            BooleanOp::Intersection => { if a_in_b { vec![ccw_a] } else if b_in_a { vec![ccw_b] } else { vec![] } }
            BooleanOp::Union => { if a_in_b { vec![ccw_b] } else if b_in_a { vec![ccw_a] } else { vec![ccw_a, ccw_b] } }
            BooleanOp::Difference => { if a_in_b { vec![] } else { vec![ccw_a] } }
        };
        let req = result.iter().map(|r| r.len() * 16 + 24).sum::<usize>() + 24;
        if req > byte_budget {
            return Err(GpOverlayError::ByteBudgetExceeded { requested: req, budget: byte_budget });
        }
        return Ok(result);
    }

    let (mut inter_to_a, mut inter_to_b) = (vec![0usize; raw_inters.len()], vec![0usize; raw_inters.len()]);
    let mut nodes_a = build_extended_nodes(&ccw_a, &raw_inters, true, &mut inter_to_a);
    let mut nodes_b = build_extended_nodes(&ccw_b, &raw_inters, false, &mut inter_to_b);

    for k in 0..raw_inters.len() {
        nodes_a[inter_to_a[k]].partner = inter_to_b[k];
        nodes_b[inter_to_b[k]].partner = inter_to_a[k];
    }

    for (nodes, other_ccw) in [(&mut nodes_a, &ccw_b), (&mut nodes_b, &ccw_a)] {
        let mid = [0.5 * (nodes[0].pt[0] + nodes[nodes[0].next].pt[0]), 0.5 * (nodes[0].pt[1] + nodes[nodes[0].next].pt[1])];
        let mut inside = point_in_ring(mid, other_ccw);
        let mut curr = nodes[0].next;
        while curr != 0 {
            if nodes[curr].is_intersection { nodes[curr].entry = !inside; inside = !inside; }
            curr = nodes[curr].next;
        }
    }

    let mut rings: Vec<Ring2D> = Vec::new();
    let max_steps = (nodes_a.len() + nodes_b.len()) * 2;

    for i in 0..nodes_a.len() {
        if !nodes_a[i].is_intersection || nodes_a[i].visited { continue; }
        let valid = match op {
            BooleanOp::Intersection => nodes_a[i].entry,
            BooleanOp::Union | BooleanOp::Difference => !nodes_a[i].entry,
        };
        if !valid { continue; }

        let mut ring: Ring2D = Vec::new();
        let (mut curr_node, mut on_a, mut steps) = (i, true, 0);

        loop {
            steps += 1;
            if steps > max_steps { break; }
            if on_a {
                nodes_a[curr_node].visited = true;
                nodes_b[nodes_a[curr_node].partner].visited = true;
                ring.push(nodes_a[curr_node].pt);
                loop {
                    curr_node = nodes_a[curr_node].next;
                    if nodes_a[curr_node].is_intersection { break; }
                    ring.push(nodes_a[curr_node].pt);
                }
                if curr_node == i { break; }
                curr_node = nodes_a[curr_node].partner;
                on_a = false;
            } else {
                nodes_b[curr_node].visited = true;
                nodes_a[nodes_b[curr_node].partner].visited = true;
                ring.push(nodes_b[curr_node].pt);
                loop {
                    curr_node = if op == BooleanOp::Difference { nodes_b[curr_node].prev } else { nodes_b[curr_node].next };
                    if nodes_b[curr_node].is_intersection { break; }
                    ring.push(nodes_b[curr_node].pt);
                }
                curr_node = nodes_b[curr_node].partner;
                on_a = true;
                if curr_node == i { break; }
            }
        }
        if ring.len() >= 3 && ring_area(&ring) > 1e-9 { rings.push(ring); }
    }

    let requested = rings.iter().map(|r| r.len() * 16 + 24).sum::<usize>() + 24;
    if requested > byte_budget {
        return Err(GpOverlayError::ByteBudgetExceeded { requested, budget: byte_budget });
    }
    Ok(rings)
}

/// Clip subject polygon against clip polygon (Sutherland-Hodgman / Greiner-Hormann).
pub fn gp_clip_polygon(subject: &[[f64; 2]], clip_convex_or_general: &[[f64; 2]], byte_budget: usize) -> Result<Vec<Ring2D>, GpOverlayError> {
    gp_intersection(subject, clip_convex_or_general, byte_budget)
}

fn line_segment_intersection(p1: [f64; 2], p2: [f64; 2], q1: [f64; 2], q2: [f64; 2]) -> Option<f64> {
    let (rx, ry) = (p2[0] - p1[0], p2[1] - p1[1]);
    let (sx, sy) = (q2[0] - q1[0], q2[1] - q1[1]);
    let denom = rx * sy - ry * sx;
    if denom.abs() < 1e-12 { return None; }
    let (qpx, qpy) = (q1[0] - p1[0], q1[1] - p1[1]);
    let (t, u) = ((qpx * sy - qpy * sx) / denom, (qpx * ry - qpy * rx) / denom);
    let eps = 1e-9;
    if t > eps && t < 1.0 - eps && u >= -eps && u <= 1.0 + eps { Some(t) } else { None }
}

/// Clip polyline segments to the interior of `clip_ring`.
pub fn gp_clip_polyline(polyline: &[[f64; 2]], clip_ring: &[[f64; 2]], byte_budget: usize) -> Result<Vec<Vec<[f64; 2]>>, GpOverlayError> {
    let open_clip = get_open_ring(clip_ring);
    if open_clip.len() < 3 || ring_area(open_clip) < 1e-12 { return Err(GpOverlayError::DegeneratePolygon); }
    let requested = polyline.len() * 32 + 64;
    if requested > byte_budget { return Err(GpOverlayError::ByteBudgetExceeded { requested, budget: byte_budget }); }
    if polyline.len() < 2 { return Ok(Vec::new()); }

    let nc = open_clip.len();
    let mut result: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut current_segment: Vec<[f64; 2]> = Vec::new();

    for i in 0..(polyline.len() - 1) {
        let (p1, p2) = (polyline[i], polyline[i + 1]);
        let mut t_values = vec![0.0, 1.0];
        for j in 0..nc {
            if let Some(t) = line_segment_intersection(p1, p2, open_clip[j], open_clip[(j + 1) % nc]) { t_values.push(t); }
        }
        t_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        t_values.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

        for w in t_values.windows(2) {
            let (t0, t1) = (w[0], w[1]);
            let mid_pt = [p1[0] + 0.5 * (t0 + t1) * (p2[0] - p1[0]), p1[1] + 0.5 * (t0 + t1) * (p2[1] - p1[1])];
            if point_in_ring(mid_pt, open_clip) {
                let start_pt = [p1[0] + t0 * (p2[0] - p1[0]), p1[1] + t0 * (p2[1] - p1[1])];
                let end_pt = [p1[0] + t1 * (p2[0] - p1[0]), p1[1] + t1 * (p2[1] - p1[1])];
                if current_segment.is_empty() {
                    current_segment.push(start_pt); current_segment.push(end_pt);
                } else {
                    let last = *current_segment.last().unwrap();
                    let (dx, dy) = (last[0] - start_pt[0], last[1] - start_pt[1]);
                    if (dx * dx + dy * dy).sqrt() < 1e-9 {
                        current_segment.push(end_pt);
                    } else {
                        result.push(current_segment); current_segment = vec![start_pt, end_pt];
                    }
                }
            }
        }
    }
    if !current_segment.is_empty() { result.push(current_segment); }

    let final_bytes = result.iter().map(|line| line.len() * 16 + 24).sum::<usize>() + 24;
    if final_bytes > byte_budget { return Err(GpOverlayError::ByteBudgetExceeded { requested: final_bytes, budget: byte_budget }); }
    Ok(result)
}

/// Boolean Intersection `A ∩ B`.
pub fn gp_intersection(poly_a: &[[f64; 2]], poly_b: &[[f64; 2]], byte_budget: usize) -> Result<Vec<Ring2D>, GpOverlayError> {
    execute_boolean_op(poly_a, poly_b, BooleanOp::Intersection, byte_budget)
}

/// Boolean Union `A ∪ B` (outer perimeter ring, or 2 disjoint rings if non-overlapping).
pub fn gp_union(poly_a: &[[f64; 2]], poly_b: &[[f64; 2]], byte_budget: usize) -> Result<Vec<Ring2D>, GpOverlayError> {
    execute_boolean_op(poly_a, poly_b, BooleanOp::Union, byte_budget)
}

/// Boolean Difference `A \ B` (Erase).
pub fn gp_difference(poly_a: &[[f64; 2]], poly_b: &[[f64; 2]], byte_budget: usize) -> Result<Vec<Ring2D>, GpOverlayError> {
    execute_boolean_op(poly_a, poly_b, BooleanOp::Difference, byte_budget)
}

/// Boolean Symmetric Difference `(A \ B) ∪ (B \ A)` (XOR).
pub fn gp_sym_difference(poly_a: &[[f64; 2]], poly_b: &[[f64; 2]], byte_budget: usize) -> Result<Vec<Ring2D>, GpOverlayError> {
    let mut diff_a = gp_difference(poly_a, poly_b, byte_budget)?;
    let diff_b = gp_difference(poly_b, poly_a, byte_budget)?;
    diff_a.extend(diff_b);
    let requested = diff_a.iter().map(|r| r.len() * 16 + 24).sum::<usize>() + 24;
    if requested > byte_budget { return Err(GpOverlayError::ByteBudgetExceeded { requested, budget: byte_budget }); }
    Ok(diff_a)
}

/// Vector Identity (is_update=false) and Update (is_update=true) overlay tool.
pub fn gp_identity_and_update_overlay(
    features_a: &[OverlayFeature],
    features_b: &[OverlayFeature],
    is_update: bool,
    byte_budget: usize,
) -> Result<Vec<OverlayFeature>, GpOverlayError> {
    let initial_est = (features_a.len() + features_b.len()) * 128 + 64;
    if initial_est > byte_budget { return Err(GpOverlayError::ByteBudgetExceeded { requested: initial_est, budget: byte_budget }); }

    let mut out_features = Vec::new();
    let mut next_id = 1u64;

    for feat_a in features_a {
        let mut rem_a_rings = vec![feat_a.ring.clone()];
        for feat_b in features_b {
            let mut next_rem = Vec::new();
            for r_a in rem_a_rings {
                if !is_update {
                    for ir in gp_intersection(&r_a, &feat_b.ring, byte_budget)? {
                        out_features.push(OverlayFeature { id: next_id, ring: ir, attr_tag: feat_a.attr_tag ^ feat_b.attr_tag });
                        next_id += 1;
                    }
                }
                next_rem.extend(gp_difference(&r_a, &feat_b.ring, byte_budget)?);
            }
            rem_a_rings = next_rem;
        }
        for r in rem_a_rings {
            out_features.push(OverlayFeature { id: next_id, ring: r, attr_tag: feat_a.attr_tag });
            next_id += 1;
        }
    }

    if is_update {
        for feat_b in features_b {
            out_features.push(OverlayFeature { id: next_id, ring: feat_b.ring.clone(), attr_tag: feat_b.attr_tag });
            next_id += 1;
        }
    }

    let final_bytes = out_features.iter().map(|f| f.ring.len() * 16 + 48).sum::<usize>() + 24;
    if final_bytes > byte_budget { return Err(GpOverlayError::ByteBudgetExceeded { requested: final_bytes, budget: byte_budget }); }
    Ok(out_features)
}

#[cfg(test)]
mod tests {
    use super::*;
    const BUDGET: usize = 1_000_000;

    #[test]
    fn test_overlapping_squares_boolean_ops() {
        let sq_a = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let sq_b = vec![[2.0, 2.0], [6.0, 2.0], [6.0, 6.0], [2.0, 6.0]];
        assert!((ring_area(&sq_a) - 16.0).abs() < 1e-9 && (ring_area(&sq_b) - 16.0).abs() < 1e-9);

        // A ∩ B: exact square [2,2]..[4,4] -> area 4.0
        let inter = gp_intersection(&sq_a, &sq_b, BUDGET).expect("intersection failed");
        let inter_area: f64 = inter.iter().map(|r| ring_area(r)).sum();
        assert!((inter_area - 4.0).abs() < 1e-9, "area(A ∩ B) expected 4.0, got {inter_area}");

        // gp_clip_polygon: equivalent to intersection -> area 4.0
        let clipped = gp_clip_polygon(&sq_a, &sq_b, BUDGET).expect("clip polygon failed");
        let clipped_area: f64 = clipped.iter().map(|r| ring_area(r)).sum();
        assert!((clipped_area - 4.0).abs() < 1e-9, "clip area expected 4.0, got {clipped_area}");

        // A \ B: L-shape [0,0]..[4,4] minus [2,2]..[4,4] -> area 12.0
        let diff_ab = gp_difference(&sq_a, &sq_b, BUDGET).expect("difference A\\B failed");
        let diff_ab_area: f64 = diff_ab.iter().map(|r| ring_area(r)).sum();
        assert!((diff_ab_area - 12.0).abs() < 1e-9, "area(A \\ B) expected 12.0, got {diff_ab_area}");

        // B \ A: L-shape [2,2]..[6,6] minus [2,2]..[4,4] -> area 12.0
        let diff_ba = gp_difference(&sq_b, &sq_a, BUDGET).expect("difference B\\A failed");
        let diff_ba_area: f64 = diff_ba.iter().map(|r| ring_area(r)).sum();
        assert!((diff_ba_area - 12.0).abs() < 1e-9, "area(B \\ A) expected 12.0, got {diff_ba_area}");

        // A ∪ B: outer perimeter 8-vertex polygon -> area 28.0 (16 + 16 - 4)
        let union_res = gp_union(&sq_a, &sq_b, BUDGET).expect("union failed");
        let union_area: f64 = union_res.iter().map(|r| ring_area(r)).sum();
        assert!((union_area - 28.0).abs() < 1e-9, "area(A ∪ B) expected 28.0, got {union_area}");

        // A ⊕ B: symmetric difference (A \ B) ∪ (B \ A) -> area 24.0 (12 + 12)
        let sym_res = gp_sym_difference(&sq_a, &sq_b, BUDGET).expect("sym diff failed");
        let sym_area: f64 = sym_res.iter().map(|r| ring_area(r)).sum();
        assert!((sym_area - 24.0).abs() < 1e-9, "area(A ⊕ B) expected 24.0, got {sym_area}");
    }

    #[test]
    fn test_gp_clip_polyline() {
        let sq_a = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let line = vec![[-1.0, 3.0], [5.0, 3.0]];
        let clipped = gp_clip_polyline(&line, &sq_a, BUDGET).expect("clip polyline failed");
        assert_eq!(clipped.len(), 1);
        let seg = &clipped[0];
        assert_eq!(seg.len(), 2);
        assert!((seg[0][0] - 0.0).abs() < 1e-9 && (seg[0][1] - 3.0).abs() < 1e-9);
        assert!((seg[1][0] - 4.0).abs() < 1e-9 && (seg[1][1] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_gp_identity_and_update_overlay() {
        let sq_a = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let sq_b = vec![[2.0, 2.0], [6.0, 2.0], [6.0, 6.0], [2.0, 6.0]];
        let feats_a = vec![OverlayFeature { id: 1, ring: sq_a, attr_tag: 100 }];
        let feats_b = vec![OverlayFeature { id: 2, ring: sq_b, attr_tag: 200 }];

        // Identity: A partitioned into A \ B (100) and A ∩ B (100 ^ 200)
        let id_res = gp_identity_and_update_overlay(&feats_a, &feats_b, false, BUDGET).expect("identity failed");
        assert_eq!(id_res.len(), 2);
        let id_area: f64 = id_res.iter().map(|f| ring_area(&f.ring)).sum();
        assert!((id_area - 16.0).abs() < 1e-9);
        let inter_feat = id_res.iter().find(|f| f.attr_tag == (100 ^ 200)).unwrap();
        assert!((ring_area(&inter_feat.ring) - 4.0).abs() < 1e-9);

        // Update: A \ B (100) and B (200)
        let up_res = gp_identity_and_update_overlay(&feats_a, &feats_b, true, BUDGET).expect("update failed");
        assert_eq!(up_res.len(), 2);
        let up_area: f64 = up_res.iter().map(|f| ring_area(&f.ring)).sum();
        assert!((up_area - 28.0).abs() < 1e-9);
        let b_feat = up_res.iter().find(|f| f.attr_tag == 200).unwrap();
        assert!((ring_area(&b_feat.ring) - 16.0).abs() < 1e-9);
    }

    #[test]
    fn test_fail_closed_byte_budget_exceeded() {
        let sq_a = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]];
        let sq_b = vec![[2.0, 2.0], [6.0, 2.0], [6.0, 6.0], [2.0, 6.0]];
        let res = gp_intersection(&sq_a, &sq_b, 10);
        match res {
            Err(GpOverlayError::ByteBudgetExceeded { requested, budget }) => {
                assert_eq!(budget, 10);
                assert!(requested > budget);
            }
            other => panic!("expected ByteBudgetExceeded, got {other:?}"),
        }
    }
}
