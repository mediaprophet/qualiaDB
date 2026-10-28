//! Minkowski variable-width buffer, multi-ring buffer, attribute-keyed shared-edge dissolve,
//! polygon/polyline planarization, and topological snap & sliver elimination.
//!
//! Allocation class: Tier-2 ColdBounded with explicit byte budget. Standalone with zero sibling imports.
//! Attribution: MIT / Apache-2.0.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub enum BufferJoinStyle { Round(usize), Miter(f64), Bevel }
impl PartialEq for BufferJoinStyle {
    fn eq(&self, o: &Self) -> bool {
        match (self, o) {
            (Self::Round(a), Self::Round(b)) => a == b,
            (Self::Miter(a), Self::Miter(b)) => a.to_bits() == b.to_bits(),
            (Self::Bevel, Self::Bevel) => true,
            _ => false,
        }
    }
}
impl Eq for BufferJoinStyle {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpBufferError {
    InvalidDistance,
    DegenerateGeometry,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

struct BudgetTracker { allocated: usize, budget: usize }
impl BudgetTracker {
    fn new(budget: usize) -> Self { Self { allocated: 0, budget } }
    fn consume(&mut self, bytes: usize) -> Result<(), GpBufferError> {
        let req = self.allocated.saturating_add(bytes);
        if req > self.budget { Err(GpBufferError::ByteBudgetExceeded { requested: req, budget: self.budget }) }
        else { self.allocated = req; Ok(()) }
    }
}

#[inline]
pub fn signed_ring_area(pts: &[[f64; 2]]) -> f64 {
    let n = pts.len();
    if n < 3 { return 0.0; }
    let mut sum = 0.0;
    for i in 0..n {
        let next = if i + 1 == n { 0 } else { i + 1 };
        sum += pts[i][0] * pts[next][1] - pts[next][0] * pts[i][1];
    }
    0.5 * sum
}

#[inline]
pub fn ring_perimeter(pts: &[[f64; 2]]) -> f64 {
    let n = pts.len();
    if n < 2 { return 0.0; }
    let mut p = 0.0;
    for i in 0..n {
        let next = if i + 1 == n { 0 } else { i + 1 };
        p += ((pts[next][0] - pts[i][0]).powi(2) + (pts[next][1] - pts[i][1]).powi(2)).sqrt();
    }
    p
}

#[inline] fn snap_pt(p: [f64; 2], s: f64) -> (i64, i64) { ((p[0] * s).round() as i64, (p[1] * s).round() as i64) }
#[inline] fn unsnap_pt(g: (i64, i64), s: f64) -> [f64; 2] { [g.0 as f64 / s, g.1 as f64 / s] }

fn clean_ring(ring: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out: Vec<[f64; 2]> = Vec::with_capacity(ring.len());
    for &pt in ring { if out.last() != Some(&pt) { out.push(pt); } }
    if out.len() > 1 && out.first() == out.last() { out.pop(); }
    out
}

fn intersect_lines(p1: [f64; 2], d1: [f64; 2], p2: [f64; 2], d2: [f64; 2]) -> Option<[f64; 2]> {
    let det = d1[0] * d2[1] - d1[1] * d2[0];
    if det.abs() < 1e-12 { return None; }
    let t = ((p2[0] - p1[0]) * d2[1] - (p2[1] - p1[1]) * d2[0]) / det;
    Some([p1[0] + t * d1[0], p1[1] + t * d1[1]])
}

fn append_arc(
    out: &mut Vec<[f64; 2]>, c: [f64; 2], p1: [f64; 2], p2: [f64; 2],
    r: f64, steps: usize, tr: &mut BudgetTracker,
) -> Result<(), GpBufferError> {
    let (a1, a2) = ((p1[1] - c[1]).atan2(p1[0] - c[0]), (p2[1] - c[1]).atan2(p2[0] - c[0]));
    let mut d_ang = a2 - a1;
    while d_ang > std::f64::consts::PI { d_ang -= 2.0 * std::f64::consts::PI; }
    while d_ang < -std::f64::consts::PI { d_ang += 2.0 * std::f64::consts::PI; }
    for s in 0..=steps {
        tr.consume(16)?;
        let a = a1 + (s as f64 / steps as f64) * d_ang;
        out.push([c[0] + r * a.cos(), c[1] + r * a.sin()]);
    }
    Ok(())
}

fn append_corner_join(
    out: &mut Vec<[f64; 2]>, p_curr: [f64; 2], p_in: [f64; 2], p_out: [f64; 2],
    t_in: [f64; 2], t_out: [f64; 2], d: f64, join_style: BufferJoinStyle,
    tr: &mut BudgetTracker,
) -> Result<(), GpBufferError> {
    let turn = t_in[0] * t_out[1] - t_in[1] * t_out[0];
    let (is_fan, is_reflex, abs_d) = (
        if d > 0.0 { turn > 1e-9 } else { turn < -1e-9 },
        if d > 0.0 { turn < -1e-9 } else { turn > 1e-9 },
        d.abs(),
    );
    if is_fan {
        match join_style {
            BufferJoinStyle::Bevel => { tr.consume(32)?; out.push(p_in); out.push(p_out); }
            BufferJoinStyle::Miter(limit) => {
                if let Some(m) = intersect_lines(p_in, t_in, p_out, t_out) {
                    let dist = ((m[0] - p_curr[0]).powi(2) + (m[1] - p_curr[1]).powi(2)).sqrt();
                    if dist / abs_d <= limit { tr.consume(16)?; out.push(m); }
                    else { tr.consume(32)?; out.push(p_in); out.push(p_out); }
                } else { tr.consume(16)?; out.push(p_in); }
            }
            BufferJoinStyle::Round(steps) => append_arc(out, p_curr, p_in, p_out, abs_d, steps.max(1), tr)?,
        }
    } else if is_reflex {
        if let Some(inter) = intersect_lines(p_in, t_in, p_out, t_out) {
            tr.consume(16)?; out.push(inter);
        } else { tr.consume(16)?; out.push(p_in); }
    } else { tr.consume(16)?; out.push(p_in); }
    Ok(())
}

pub fn gp_buffer_polygon(
    ring: &[[f64; 2]], distance: f64, join_style: BufferJoinStyle, byte_budget: usize,
) -> Result<Vec<[f64; 2]>, GpBufferError> {
    if distance.is_nan() || distance.is_infinite() { return Err(GpBufferError::InvalidDistance); }
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.consume(64)?;

    let mut pts = clean_ring(ring);
    if pts.len() < 3 { return Err(GpBufferError::DegenerateGeometry); }
    let area = signed_ring_area(&pts);
    if area.abs() < 1e-12 { return Err(GpBufferError::DegenerateGeometry); }
    if area < 0.0 { pts.reverse(); }

    if distance.abs() < 1e-12 {
        pts.push(pts[0]);
        tracker.consume(pts.len() * 16)?;
        return Ok(pts);
    }

    let n = pts.len();
    let (mut tangents, mut normals) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for i in 0..n {
        let next = if i + 1 == n { 0 } else { i + 1 };
        let (dx, dy) = (pts[next][0] - pts[i][0], pts[next][1] - pts[i][1]);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 { return Err(GpBufferError::DegenerateGeometry); }
        let t = [dx / len, dy / len];
        tangents.push(t);
        normals.push([t[1], -t[0]]);
    }

    let mut out: Vec<[f64; 2]> = Vec::new();
    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let p_in = [pts[i][0] + distance * normals[prev][0], pts[i][1] + distance * normals[prev][1]];
        let p_out = [pts[i][0] + distance * normals[i][0], pts[i][1] + distance * normals[i][1]];
        append_corner_join(&mut out, pts[i], p_in, p_out, tangents[prev], tangents[i], distance, join_style, &mut tracker)?;
    }

    if out.len() < 3 || signed_ring_area(&out) <= 1e-12 { return Err(GpBufferError::DegenerateGeometry); }
    if out.first() != out.last() { tracker.consume(16)?; out.push(out[0]); }
    Ok(out)
}

pub fn gp_buffer_polyline(
    line: &[[f64; 2]], half_width: f64, join_style: BufferJoinStyle, byte_budget: usize,
) -> Result<Vec<[f64; 2]>, GpBufferError> {
    if half_width <= 0.0 || half_width.is_nan() || half_width.is_infinite() { return Err(GpBufferError::InvalidDistance); }
    let m = line.len();
    if m < 2 { return Err(GpBufferError::DegenerateGeometry); }

    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.consume(64)?;

    let (mut tangents, mut normals) = (Vec::with_capacity(m - 1), Vec::with_capacity(m - 1));
    for j in 0..m - 1 {
        let (dx, dy) = (line[j + 1][0] - line[j][0], line[j + 1][1] - line[j][1]);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 { return Err(GpBufferError::DegenerateGeometry); }
        let t = [dx / len, dy / len];
        tangents.push(t);
        normals.push([t[1], -t[0]]);
    }

    let w = half_width;
    let mut out = Vec::new();
    tracker.consume(16)?;
    out.push([line[0][0] + w * normals[0][0], line[0][1] + w * normals[0][1]]);

    for i in 1..m - 1 {
        let p_in = [line[i][0] + w * normals[i - 1][0], line[i][1] + w * normals[i - 1][1]];
        let p_out = [line[i][0] + w * normals[i][0], line[i][1] + w * normals[i][1]];
        append_corner_join(&mut out, line[i], p_in, p_out, tangents[i - 1], tangents[i], w, join_style, &mut tracker)?;
    }

    let end_p = line[m - 1];
    let (r_end, l_end) = ([end_p[0] + w * normals[m - 2][0], end_p[1] + w * normals[m - 2][1]], [end_p[0] - w * normals[m - 2][0], end_p[1] - w * normals[m - 2][1]]);
    match join_style {
        BufferJoinStyle::Round(s) => append_arc(&mut out, end_p, r_end, l_end, w, s.max(1), &mut tracker)?,
        _ => { tracker.consume(32)?; out.push(r_end); out.push(l_end); }
    }

    for i in (1..m - 1).rev() {
        let p_in = [line[i][0] - w * normals[i][0], line[i][1] - w * normals[i][1]];
        let p_out = [line[i][0] - w * normals[i - 1][0], line[i][1] - w * normals[i - 1][1]];
        let (rev_in, rev_out) = ([-tangents[i][0], -tangents[i][1]], [-tangents[i - 1][0], -tangents[i - 1][1]]);
        append_corner_join(&mut out, line[i], p_in, p_out, rev_in, rev_out, w, join_style, &mut tracker)?;
    }

    let start_p = line[0];
    let (l_start, r_start) = ([start_p[0] - w * normals[0][0], start_p[1] - w * normals[0][1]], [start_p[0] + w * normals[0][0], start_p[1] + w * normals[0][1]]);
    match join_style {
        BufferJoinStyle::Round(s) => append_arc(&mut out, start_p, l_start, r_start, w, s.max(1), &mut tracker)?,
        _ => { tracker.consume(32)?; out.push(l_start); out.push(r_start); }
    }

    if out.first() != out.last() { tracker.consume(16)?; out.push(out[0]); }
    Ok(out)
}

pub fn gp_multi_ring_buffer(
    ring: &[[f64; 2]], distances: &[f64], join_style: BufferJoinStyle, byte_budget: usize,
) -> Result<Vec<(f64, Vec<[f64; 2]>)>, GpBufferError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.consume(64)?;

    let mut dists: Vec<f64> = distances.iter().copied().filter(|d| *d > 0.0 && d.is_finite()).collect();
    dists.sort_by(|a, b| a.total_cmp(b));
    dists.dedup_by(|a, b| (*a - *b).abs() < 1e-9);

    let mut results = Vec::with_capacity(dists.len());
    for d in dists {
        let remaining = tracker.budget.saturating_sub(tracker.allocated);
        let buffered = gp_buffer_polygon(ring, d, join_style, remaining)?;
        tracker.consume(buffered.len() * 16 + 24)?;
        results.push((d, buffered));
    }
    Ok(results)
}

fn dissolve_polygon_rings(
    rings: &[Vec<[f64; 2]>], snap_tol: f64, tracker: &mut BudgetTracker,
) -> Result<Vec<Vec<[f64; 2]>>, GpBufferError> {
    tracker.consume(64)?;
    let scale = if snap_tol > 1e-12 { 1.0 / snap_tol } else { 1e7 };
    let mut edge_counts: HashMap<((i64, i64), (i64, i64)), usize> = HashMap::new();

    for ring in rings {
        let n = ring.len();
        if n < 3 { continue; }
        for i in 0..n {
            let next = if i + 1 == n { 0 } else { i + 1 };
            let (u, v) = (snap_pt(ring[i], scale), snap_pt(ring[next], scale));
            if u == v { continue; }
            tracker.consume(48)?;
            if let Some(cnt) = edge_counts.get_mut(&(v, u)) {
                if *cnt <= 1 { edge_counts.remove(&(v, u)); } else { *cnt -= 1; }
            } else { *edge_counts.entry((u, v)).or_insert(0) += 1; }
        }
    }

    let mut adj: HashMap<(i64, i64), Vec<(i64, i64)>> = HashMap::new();
    for ((u, v), count) in edge_counts {
        tracker.consume(24 * count)?;
        adj.entry(u).or_default().extend(std::iter::repeat(v).take(count));
    }

    let mut dissolved = Vec::new();
    while let Some(&start) = adj.keys().find(|k| adj.get(k).map_or(false, |v| !v.is_empty())) {
        let (mut r, mut curr) = (vec![unsnap_pt(start, scale)], start);
        while let Some(next) = adj.get_mut(&curr).and_then(|v| v.pop()) {
            curr = next;
            r.push(unsnap_pt(curr, scale));
            if curr == start { break; }
        }
        if r.len() >= 3 {
            if r.first() != r.last() { r.push(r[0]); }
            tracker.consume(r.len() * 16)?;
            dissolved.push(r);
        }
    }
    Ok(dissolved)
}

pub fn gp_dissolve_by_key(
    polygons_with_keys: &[(u64, Vec<[f64; 2]>)], snap_tol: f64, byte_budget: usize,
) -> Result<Vec<(u64, Vec<[f64; 2]>)>, GpBufferError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.consume(64)?;

    let mut groups: HashMap<u64, Vec<Vec<[f64; 2]>>> = HashMap::new();
    for (k, poly) in polygons_with_keys {
        tracker.consume(32 + poly.len() * 16)?;
        groups.entry(*k).or_default().push(poly.clone());
    }

    let mut results = Vec::new();
    let mut keys: Vec<u64> = groups.keys().copied().collect();
    keys.sort_unstable();

    for k in keys {
        let dissolved = dissolve_polygon_rings(&groups[&k], snap_tol, &mut tracker)?;
        for r in dissolved {
            tracker.consume(32 + r.len() * 16)?;
            results.push((k, r));
        }
    }
    Ok(results)
}

pub fn gp_polygons_to_unique_lines(
    polygons: &[Vec<[f64; 2]>], snap_tol: f64, byte_budget: usize,
) -> Result<Vec<[[f64; 2]; 2]>, GpBufferError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.consume(64)?;
    let scale = if snap_tol > 1e-12 { 1.0 / snap_tol } else { 1e7 };

    let mut seen: HashSet<((i64, i64), (i64, i64))> = HashSet::new();
    let mut lines = Vec::new();

    for poly in polygons {
        let n = poly.len();
        if n < 2 { continue; }
        for i in 0..n {
            let next = if i + 1 == n { 0 } else { i + 1 };
            let (u, v) = (snap_pt(poly[i], scale), snap_pt(poly[next], scale));
            if u == v { continue; }
            let edge = if u <= v { (u, v) } else { (v, u) };
            tracker.consume(48)?;
            if seen.insert(edge) {
                tracker.consume(32)?;
                lines.push([unsnap_pt(edge.0, scale), unsnap_pt(edge.1, scale)]);
            }
        }
    }
    Ok(lines)
}

pub fn gp_snap_and_eliminate_slivers(
    polygons: &mut Vec<(u64, Vec<[f64; 2]>)>, snap_tol: f64,
    min_sliver_area: f64, min_compactness_4pi_a_p2: f64, byte_budget: usize,
) -> Result<usize, GpBufferError> {
    let mut tracker = BudgetTracker::new(byte_budget);
    tracker.consume(128)?;
    let scale = if snap_tol > 1e-12 { 1.0 / snap_tol } else { 1e7 };

    for (_, ring) in polygons.iter_mut() {
        let mut snapped = Vec::with_capacity(ring.len());
        for &pt in ring.iter() {
            let spt = unsnap_pt(snap_pt(pt, scale), scale);
            if snapped.last() != Some(&spt) { snapped.push(spt); }
        }
        if snapped.len() >= 3 && snapped.first() != snapped.last() { snapped.push(snapped[0]); }
        tracker.consume(snapped.len() * 16)?;
        *ring = snapped;
    }

    let mut eliminated = 0;
    let mut i = 0;
    while i < polygons.len() {
        let area = signed_ring_area(&polygons[i].1).abs();
        let perim = ring_perimeter(&polygons[i].1);
        let comp = if perim > 1e-12 { (4.0 * std::f64::consts::PI * area) / (perim * perim) } else { 0.0 };

        if area < min_sliver_area || comp < min_compactness_4pi_a_p2 {
            let mut s_edges = HashSet::new();
            let r_s = &polygons[i].1;
            for j in 0..r_s.len() {
                let next = if j + 1 == r_s.len() { 0 } else { j + 1 };
                let (u, v) = (snap_pt(r_s[j], scale), snap_pt(r_s[next], scale));
                if u != v { s_edges.insert(if u <= v { (u, v) } else { (v, u) }); }
            }

            let mut best_n: Option<(usize, f64)> = None;
            for j in 0..polygons.len() {
                if j == i { continue; }
                let (mut shared, r_n) = (0.0, &polygons[j].1);
                for k in 0..r_n.len() {
                    let next = if k + 1 == r_n.len() { 0 } else { k + 1 };
                    let (u, v) = (snap_pt(r_n[k], scale), snap_pt(r_n[next], scale));
                    if u != v && s_edges.contains(&(if u <= v { (u, v) } else { (v, u) })) {
                        shared += ((r_n[next][0] - r_n[k][0]).powi(2) + (r_n[next][1] - r_n[k][1]).powi(2)).sqrt();
                    }
                }
                if shared > 1e-9 && best_n.map_or(true, |(_, bl)| shared > bl) { best_n = Some((j, shared)); }
            }

            if let Some((n_idx, _)) = best_n {
                let dissolved = dissolve_polygon_rings(&[polygons[i].1.clone(), polygons[n_idx].1.clone()], snap_tol, &mut tracker)?;
                if let Some(merged) = dissolved.into_iter().next() { polygons[n_idx].1 = merged; }
                polygons.remove(i);
                eliminated += 1;
                i = 0;
            } else {
                polygons.remove(i);
                eliminated += 1;
            }
            continue;
        }
        i += 1;
    }
    Ok(eliminated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_polygon_styles() {
        let sq = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]];
        let m = gp_buffer_polygon(&sq, 0.5, BufferJoinStyle::Miter(2.0), 10_000).unwrap();
        assert_eq!(m.first(), m.last());
        assert!((signed_ring_area(&m).abs() - 4.0).abs() < 1e-4);

        let r = gp_buffer_polygon(&sq, 0.5, BufferJoinStyle::Round(8), 10_000).unwrap();
        assert!((signed_ring_area(&r).abs() - (1.0 + 4.0 * 0.5 + std::f64::consts::PI * 0.25)).abs() < 0.1);

        let b = gp_buffer_polygon(&sq, 0.5, BufferJoinStyle::Bevel, 10_000).unwrap();
        assert!((signed_ring_area(&b).abs() - 3.5).abs() < 1e-4);

        let inv = gp_buffer_polygon(&sq, -0.2, BufferJoinStyle::Miter(2.0), 10_000).unwrap();
        assert!((signed_ring_area(&inv).abs() - 0.36).abs() < 1e-4);
    }

    #[test]
    fn test_buffer_polyline_and_multi_ring() {
        let line = vec![[0.0, 0.0], [10.0, 0.0]];
        let corridor = gp_buffer_polyline(&line, 1.0, BufferJoinStyle::Bevel, 10_000).unwrap();
        assert_eq!(corridor.first(), corridor.last());
        assert!((signed_ring_area(&corridor).abs() - 20.0).abs() < 1e-4);

        let sq = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]];
        let rings = gp_multi_ring_buffer(&sq, &[1.0, 0.5], BufferJoinStyle::Miter(2.0), 20_000).unwrap();
        assert_eq!(rings.len(), 2);
        assert_eq!(rings[0].0, 0.5);
        assert_eq!(rings[1].0, 1.0);
    }

    #[test]
    fn test_dissolve_adjacent_squares() {
        let (sq1, sq2) = (vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]], vec![[1.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        let res = gp_dissolve_by_key(&[(100u64, sq1), (100u64, sq2)], 0.01, 10_000).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].0, 100);
        assert!((signed_ring_area(&res[0].1).abs() - 2.0).abs() < 1e-4);
    }

    #[test]
    fn test_polygons_to_unique_lines() {
        let (sq1, sq2) = (vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]], vec![[1.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert_eq!(gp_polygons_to_unique_lines(&[sq1, sq2], 0.01, 10_000).unwrap().len(), 7);
    }

    #[test]
    fn test_snap_and_eliminate_slivers() {
        let (main_sq, sliver) = (vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]], vec![[0.0, 2.0], [2.0, 2.0], [2.0, 2.02], [0.0, 2.02], [0.0, 2.0]]);
        let mut polys = vec![(1u64, main_sq), (2u64, sliver)];
        assert_eq!(gp_snap_and_eliminate_slivers(&mut polys, 0.001, 0.1, 0.5, 10_000).unwrap(), 1);
        assert_eq!(polys.len(), 1);
        assert!((signed_ring_area(&polys[0].1).abs() - 4.04).abs() < 0.01);
    }

    #[test]
    fn test_budget_exceeded() {
        let sq = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0]];
        assert!(matches!(gp_buffer_polygon(&sq, 0.5, BufferJoinStyle::Miter(2.0), 10), Err(GpBufferError::ByteBudgetExceeded { .. })));
    }
}
