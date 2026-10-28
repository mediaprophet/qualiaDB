// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 QualiaDB contributors
//! Geospatial Geometric Processing: Simplification, Smoothing, Hulls, Polylabel, Delaunay/Voronoi, and Regular Tessellation Grids.

/// Errors encountered during geospatial processing and geometry generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpGeomError { InsufficientPoints, InvalidParameter, ByteBudgetExceeded { requested: usize, budget: usize } }

/// Regular 2D tessellation grid geometry shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TessellationShape { Square, HexagonFlatTop, Diamond }

#[inline]
fn dist_sq(a: [f64; 2], b: [f64; 2]) -> f64 { (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) }

#[inline]
fn pt_seg_dist_sq(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let l2 = dist_sq(a, b);
    if l2 == 0.0 { return dist_sq(p, a); }
    let t = (((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / l2).clamp(0.0, 1.0);
    dist_sq(p, [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])])
}

#[inline]
fn tri_area_2d(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs()
}

/// Compute 2D polygon area via shoelace formula.
pub fn polygon_area(ring: &[[f64; 2]]) -> f64 {
    if ring.len() < 3 { return 0.0; }
    let mut a = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        a += ring[i][0] * ring[j][1] - ring[j][0] * ring[i][1];
    }
    0.5 * a.abs()
}

/// Non-recursive iterative stack Ramer-Douglas-Peucker simplification (Tier-1 HotZeroHeap).
pub fn gp_simplify_douglas_peucker(pts: &[[f64; 2]], epsilon: f64, keep_flags: &mut [bool], out_pts: &mut [[f64; 2]]) -> Result<usize, GpGeomError> {
    if pts.len() < 2 { return Err(GpGeomError::InsufficientPoints); }
    if epsilon < 0.0 || keep_flags.len() < pts.len() { return Err(GpGeomError::InvalidParameter); }
    for flag in keep_flags[..pts.len()].iter_mut() { *flag = false; }
    keep_flags[0] = true; keep_flags[pts.len() - 1] = true;
    let (mut stack, mut top, eps_sq) = ([(0usize, pts.len() - 1); 64], 1, epsilon * epsilon);
    while top > 0 {
        top -= 1;
        let (start, end) = stack[top];
        if end <= start + 1 { continue; }
        let (mut max_d_sq, mut max_idx) = (-1.0, start);
        for i in (start + 1)..end {
            let d_sq = pt_seg_dist_sq(pts[i], pts[start], pts[end]);
            if d_sq > max_d_sq { max_d_sq = d_sq; max_idx = i; }
        }
        if max_d_sq > eps_sq {
            keep_flags[max_idx] = true;
            if top + 2 > 64 { return Err(GpGeomError::InvalidParameter); }
            stack[top] = (start, max_idx); top += 1;
            stack[top] = (max_idx, end); top += 1;
        }
    }
    let mut count = 0;
    for (i, &keep) in keep_flags[..pts.len()].iter().enumerate() {
        if keep {
            if count >= out_pts.len() { return Err(GpGeomError::InvalidParameter); }
            out_pts[count] = pts[i];
            count += 1;
        }
    }
    Ok(count)
}

/// Visvalingam-Whyatt progressive effective triangle area simplification (Tier-2 ColdBounded).
pub fn gp_simplify_visvalingam_whyatt(pts: &[[f64; 2]], min_triangle_area: f64, byte_budget: usize) -> Result<Vec<[f64; 2]>, GpGeomError> {
    let n = pts.len();
    if n < 2 { return Err(GpGeomError::InsufficientPoints); }
    if min_triangle_area < 0.0 { return Err(GpGeomError::InvalidParameter); }
    if n == 2 { return Ok(pts.to_vec()); }
    let req = n * 48 + 128;
    if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    let mut prev: Vec<usize> = (0..n).map(|i| if i > 0 { i - 1 } else { 0 }).collect();
    let mut next: Vec<usize> = (0..n).map(|i| i + 1).collect();
    let mut count = n;
    while count > 2 {
        let (mut min_area, mut min_idx, mut curr) = (f64::MAX, 0, next[0]);
        while curr < n - 1 {
            let area = tri_area_2d(pts[prev[curr]], pts[curr], pts[next[curr]]);
            if area < min_area { min_area = area; min_idx = curr; }
            curr = next[curr];
        }
        if min_area >= min_triangle_area { break; }
        let (p, nx) = (prev[min_idx], next[min_idx]);
        next[p] = nx; prev[nx] = p; count -= 1;
    }
    let (mut out, mut curr) = (Vec::with_capacity(count), 0);
    while curr < n { out.push(pts[curr]); curr = next[curr]; }
    Ok(out)
}

/// Chaikin corner cutting smoothing for open or closed polyline (Tier-2 ColdBounded).
pub fn gp_smooth_chaikin(pts: &[[f64; 2]], iterations: u8, closed: bool, byte_budget: usize) -> Result<Vec<[f64; 2]>, GpGeomError> {
    if pts.len() < 2 || (closed && pts.len() < 3) { return Err(GpGeomError::InsufficientPoints); }
    if iterations == 0 { return Ok(pts.to_vec()); }
    let req = pts.len() * (1usize << iterations.min(8) as usize) * 32 + 256;
    if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    let mut current = pts.to_vec();
    for _ in 0..iterations {
        let mut next = Vec::new();
        if closed {
            let base = if current.len() > 1 && current.first() == current.last() { &current[..current.len() - 1] } else { &current[..] };
            for i in 0..base.len() {
                let (p0, p1) = (base[i], base[(i + 1) % base.len()]);
                next.push([0.75 * p0[0] + 0.25 * p1[0], 0.75 * p0[1] + 0.25 * p1[1]]);
                next.push([0.25 * p0[0] + 0.75 * p1[0], 0.25 * p0[1] + 0.75 * p1[1]]);
            }
            next.push(next[0]);
        } else {
            next.push(current[0]);
            for i in 0..current.len() - 1 {
                let (p0, p1) = (current[i], current[i + 1]);
                next.push([0.75 * p0[0] + 0.25 * p1[0], 0.75 * p0[1] + 0.25 * p1[1]]);
                next.push([0.25 * p0[0] + 0.75 * p1[0], 0.25 * p0[1] + 0.75 * p1[1]]);
            }
            next.push(*current.last().unwrap());
        }
        current = next;
    }
    Ok(current)
}

/// Andrew's monotone chain convex hull O(N log N) (Tier-2 ColdBounded).
pub fn gp_convex_hull(pts: &[[f64; 2]], byte_budget: usize) -> Result<Vec<[f64; 2]>, GpGeomError> {
    if pts.len() < 3 { return Err(GpGeomError::InsufficientPoints); }
    let req = pts.len() * 32 + 128;
    if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    let mut sorted = pts.to_vec();
    sorted.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap().then_with(|| a[1].partial_cmp(&b[1]).unwrap()));
    sorted.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12);
    if sorted.len() < 3 { return Err(GpGeomError::InsufficientPoints); }
    let cross_2d = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let (mut lower, mut upper) = (Vec::new(), Vec::new());
    for &p in &sorted {
        while lower.len() >= 2 && cross_2d(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 { lower.pop(); }
        lower.push(p);
    }
    for &p in sorted.iter().rev() {
        while upper.len() >= 2 && cross_2d(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 { upper.pop(); }
        upper.push(p);
    }
    lower.pop(); upper.pop(); lower.extend(upper); lower.push(lower[0]);
    Ok(lower)
}

fn in_circumcircle(p: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
    let d = 2.0 * (a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1]));
    if d.abs() < 1e-12 { return false; }
    let (a2, b2, c2) = (a[0] * a[0] + a[1] * a[1], b[0] * b[0] + b[1] * b[1], c[0] * c[0] + c[1] * c[1]);
    let ux = (a2 * (b[1] - c[1]) + b2 * (c[1] - a[1]) + c2 * (a[1] - b[1])) / d;
    let uy = (a2 * (c[0] - b[0]) + b2 * (a[0] - c[0]) + c2 * (b[0] - a[0])) / d;
    (p[0] - ux).powi(2) + (p[1] - uy).powi(2) <= (a[0] - ux).powi(2) + (a[1] - uy).powi(2) + 1e-9
}

/// Bowyer-Watson Delaunay Triangulation returning triangle index triples (Tier-2 ColdBounded).
pub fn gp_delaunay_triangulate(pts: &[[f64; 2]], byte_budget: usize) -> Result<Vec<[usize; 3]>, GpGeomError> {
    let n = pts.len();
    if n < 3 { return Err(GpGeomError::InsufficientPoints); }
    let req = n * 128 + 256;
    if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (pts[0][0], pts[0][1], pts[0][0], pts[0][1]);
    for p in pts.iter().skip(1) {
        min_x = min_x.min(p[0]); min_y = min_y.min(p[1]);
        max_x = max_x.max(p[0]); max_y = max_y.max(p[1]);
    }
    let d = (max_x - min_x).max(max_y - min_y).max(1.0);
    let (cx, cy) = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
    let mut all_pts = pts.to_vec();
    all_pts.extend_from_slice(&[[cx - 20.0 * d, cy - d], [cx, cy + 20.0 * d], [cx + 20.0 * d, cy - d]]);
    let mut tris = vec![[n, n + 1, n + 2]];
    for i in 0..n {
        let p = all_pts[i];
        let mut bad = Vec::new();
        for (ti, &t) in tris.iter().enumerate() {
            if in_circumcircle(p, all_pts[t[0]], all_pts[t[1]], all_pts[t[2]]) { bad.push(ti); }
        }
        let mut edges = Vec::new();
        for &bi in &bad {
            let t = tris[bi];
            for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let mut shared = false;
                for &bj in &bad {
                    if bi == bj { continue; }
                    let ot = tris[bj];
                    for (ou, ov) in [(ot[0], ot[1]), (ot[1], ot[2]), (ot[2], ot[0])] {
                        if (u == ou && v == ov) || (u == ov && v == ou) { shared = true; break; }
                    }
                    if shared { break; }
                }
                if !shared { edges.push((u, v)); }
            }
        }
        for bi in bad.into_iter().rev() { tris.swap_remove(bi); }
        for (u, v) in edges { tris.push([u, v, i]); }
    }
    tris.retain(|t| t[0] < n && t[1] < n && t[2] < n);
    Ok(tris)
}

/// Alpha-shape concave boundary hull extraction (Tier-2 ColdBounded).
pub fn gp_concave_hull_alpha(pts: &[[f64; 2]], max_edge_len: f64, byte_budget: usize) -> Result<Vec<[f64; 2]>, GpGeomError> {
    if pts.len() < 3 { return Err(GpGeomError::InsufficientPoints); }
    if max_edge_len <= 0.0 { return Err(GpGeomError::InvalidParameter); }
    let tris = gp_delaunay_triangulate(pts, byte_budget)?;
    let max_len_sq = max_edge_len * max_edge_len;
    let valid_tris: Vec<[usize; 3]> = tris.into_iter().filter(|t| {
        dist_sq(pts[t[0]], pts[t[1]]) <= max_len_sq && dist_sq(pts[t[1]], pts[t[2]]) <= max_len_sq && dist_sq(pts[t[2]], pts[t[0]]) <= max_len_sq
    }).collect();
    if valid_tris.is_empty() { return gp_convex_hull(pts, byte_budget); }
    let mut edges = Vec::new();
    for t in &valid_tris {
        for (u, v) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = if u < v { (u, v) } else { (v, u) };
            if let Some(pos) = edges.iter().position(|&e| e == key) { edges.swap_remove(pos); } else { edges.push(key); }
        }
    }
    if edges.is_empty() { return gp_convex_hull(pts, byte_budget); }
    let (mut ring, mut curr) = (Vec::new(), edges[0].0);
    ring.push(pts[curr]);
    while !edges.is_empty() {
        let mut found = false;
        for i in 0..edges.len() {
            let (u, v) = edges[i];
            if u == curr { curr = v; ring.push(pts[curr]); edges.swap_remove(i); found = true; break; }
            else if v == curr { curr = u; ring.push(pts[curr]); edges.swap_remove(i); found = true; break; }
        }
        if !found { break; }
    }
    if ring.len() >= 3 && ring.first() != ring.last() { ring.push(ring[0]); }
    Ok(ring)
}

fn pt_in_ring(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let (pi, pj) = (ring[i], ring[j]);
        if ((pi[1] > p[1]) != (pj[1] > p[1])) && (p[0] < (pj[0] - pi[0]) * (p[1] - pi[1]) / (pj[1] - pi[1]) + pi[0]) { inside = !inside; }
        j = i;
    }
    inside
}

fn ring_dist(p: [f64; 2], ring: &[[f64; 2]]) -> f64 {
    let mut min_d = f64::MAX;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        min_d = min_d.min(pt_seg_dist_sq(p, ring[j], ring[i]));
        j = i;
    }
    let d = min_d.sqrt();
    if pt_in_ring(p, ring) { d } else { -d }
}

/// HotZeroHeap area centroid and pole of inaccessibility (polylabel) with signed clearance.
pub fn gp_centroid_and_polylabel(ring: &[[f64; 2]], precision: f64) -> Result<([f64; 2], [f64; 2], f64), GpGeomError> {
    let n = ring.len();
    if n < 3 { return Err(GpGeomError::InsufficientPoints); }
    if precision <= 0.0 { return Err(GpGeomError::InvalidParameter); }
    let (mut min, mut max) = ([f64::MAX, f64::MAX], [f64::MIN, f64::MIN]);
    let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let j = (i + 1) % n;
        min[0] = min[0].min(ring[i][0]); min[1] = min[1].min(ring[i][1]);
        max[0] = max[0].max(ring[i][0]); max[1] = max[1].max(ring[i][1]);
        let cross = ring[i][0] * ring[j][1] - ring[j][0] * ring[i][1];
        a += cross; cx += (ring[i][0] + ring[j][0]) * cross; cy += (ring[i][1] + ring[j][1]) * cross;
    }
    let area = 0.5 * a;
    let centroid = if area.abs() > 1e-12 { [cx / (6.0 * area), cy / (6.0 * area)] } else { [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5] };
    let (mut best_p, mut best_d) = (centroid, ring_dist(centroid, ring));
    #[derive(Clone, Copy)]
    struct Cell { x: f64, y: f64, h: f64, max: f64 }
    let mut cells = [Cell { x: 0.0, y: 0.0, h: 0.0, max: 0.0 }; 64];
    let mut count = 0;
    let cell_size = (max[0] - min[0]).min(max[1] - min[1]);
    let h_step = cell_size * 0.5;
    if h_step > 0.0 {
        let mut x = min[0];
        while x < max[0] && count < 64 {
            let mut y = min[1];
            while y < max[1] && count < 64 {
                let c = [x + h_step, y + h_step];
                let d = ring_dist(c, ring);
                if d > best_d { best_d = d; best_p = c; }
                cells[count] = Cell { x: c[0], y: c[1], h: h_step, max: d + h_step * 1.41421356 };
                count += 1; y += cell_size;
            }
            x += cell_size;
        }
    }
    while count > 0 {
        let mut best_idx = 0;
        for i in 1..count { if cells[i].max > cells[best_idx].max { best_idx = i; } }
        let cell = cells[best_idx];
        if cell.max - best_d <= precision { break; }
        cells[best_idx] = cells[count - 1]; count -= 1;
        let sub_h = cell.h * 0.5;
        for &(dx, dy) in &[(-sub_h, -sub_h), (-sub_h, sub_h), (sub_h, -sub_h), (sub_h, sub_h)] {
            let p = [cell.x + dx, cell.y + dy];
            let d = ring_dist(p, ring);
            if d > best_d { best_d = d; best_p = p; }
            let pot = d + sub_h * 1.41421356;
            if pot > best_d && count < 64 { cells[count] = Cell { x: p[0], y: p[1], h: sub_h, max: pot }; count += 1; }
        }
    }
    Ok((centroid, best_p, best_d))
}

/// Voronoi Diagram clipped to bounding box via half-plane Sutherland-Hodgman (Tier-2 ColdBounded).
pub fn gp_voronoi_diagram_bounded(pts: &[[f64; 2]], clip_bbox: [f64; 4], byte_budget: usize) -> Result<Vec<Vec<[f64; 2]>>, GpGeomError> {
    if pts.is_empty() { return Err(GpGeomError::InsufficientPoints); }
    let [min_x, min_y, max_x, max_y] = clip_bbox;
    if min_x >= max_x || min_y >= max_y { return Err(GpGeomError::InvalidParameter); }
    let req = pts.len() * pts.len() * 32 + 512;
    if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
    let mut cells = Vec::with_capacity(pts.len());
    for (i, &pi) in pts.iter().enumerate() {
        let mut poly = vec![[min_x, min_y], [max_x, min_y], [max_x, max_y], [min_x, max_y]];
        for (j, &pj) in pts.iter().enumerate() {
            if i == j || poly.is_empty() { continue; }
            let (mid, norm) = ([(pi[0] + pj[0]) * 0.5, (pi[1] + pj[1]) * 0.5], [pi[0] - pj[0], pi[1] - pj[1]]);
            let mut clipped = Vec::new();
            for k in 0..poly.len() {
                let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
                let in_a = (a[0] - mid[0]) * norm[0] + (a[1] - mid[1]) * norm[1] >= -1e-12;
                let in_b = (b[0] - mid[0]) * norm[0] + (b[1] - mid[1]) * norm[1] >= -1e-12;
                if in_a && in_b { clipped.push(b); }
                else if in_a && !in_b {
                    let d = (b[0] - a[0]) * norm[0] + (b[1] - a[1]) * norm[1];
                    let t = if d.abs() > 1e-14 { ((mid[0] - a[0]) * norm[0] + (mid[1] - a[1]) * norm[1]) / d } else { 0.5 };
                    clipped.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
                } else if !in_a && in_b {
                    let d = (b[0] - a[0]) * norm[0] + (b[1] - a[1]) * norm[1];
                    let t = if d.abs() > 1e-14 { ((mid[0] - a[0]) * norm[0] + (mid[1] - a[1]) * norm[1]) / d } else { 0.5 };
                    clipped.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]); clipped.push(b);
                }
            }
            poly = clipped;
        }
        if poly.len() >= 3 && poly.first() != poly.last() { poly.push(poly[0]); }
        cells.push(poly);
    }
    Ok(cells)
}

/// Regular Square, HexagonFlatTop, or Diamond Tessellation grid cells (Tier-2 ColdBounded).
pub fn gp_generate_tessellation_grid(bbox: [f64; 4], cell_size: f64, shape: TessellationShape, byte_budget: usize) -> Result<Vec<Vec<[f64; 2]>>, GpGeomError> {
    let [min_x, min_y, max_x, max_y] = bbox;
    if cell_size <= 0.0 || min_x >= max_x || min_y >= max_y { return Err(GpGeomError::InvalidParameter); }
    let mut grid = Vec::new();
    match shape {
        TessellationShape::Square => {
            let (cols, rows) = (((max_x - min_x) / cell_size).ceil() as usize, ((max_y - min_y) / cell_size).ceil() as usize);
            let req = cols * rows * 80 + 256;
            if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
            for c in 0..cols {
                let x = min_x + c as f64 * cell_size;
                for r in 0..rows {
                    let y = min_y + r as f64 * cell_size;
                    grid.push(vec![[x, y], [x + cell_size, y], [x + cell_size, y + cell_size], [x, y + cell_size], [x, y]]);
                }
            }
        }
        TessellationShape::HexagonFlatTop => {
            let (r, h) = (cell_size * 0.5, 3.0f64.sqrt() * cell_size * 0.5);
            let (dx, dy) = (1.5 * r, h);
            let (cols, rows) = ((((max_x - min_x) + r) / dx).ceil() as usize + 1, (((max_y - min_y) + h) / dy).ceil() as usize + 1);
            let req = cols * rows * 112 + 256;
            if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
            for c in 0..cols {
                let cx = min_x + c as f64 * dx;
                let y_shift = if c % 2 != 0 { h * 0.5 } else { 0.0 };
                for row in 0..rows {
                    let cy = min_y + row as f64 * dy + y_shift;
                    grid.push(vec![
                        [cx + r, cy], [cx + 0.5 * r, cy + 0.5 * h], [cx - 0.5 * r, cy + 0.5 * h],
                        [cx - r, cy], [cx - 0.5 * r, cy - 0.5 * h], [cx + 0.5 * r, cy - 0.5 * h], [cx + r, cy],
                    ]);
                }
            }
        }
        TessellationShape::Diamond => {
            let hs = cell_size * 0.5;
            let (cols, rows) = (((max_x - min_x) / cell_size).ceil() as usize + 1, ((max_y - min_y) / cell_size).ceil() as usize + 1);
            let req = cols * rows * 80 + 256;
            if req > byte_budget { return Err(GpGeomError::ByteBudgetExceeded { requested: req, budget: byte_budget }); }
            for c in 0..cols {
                let cx = min_x + c as f64 * cell_size + hs;
                for r in 0..rows {
                    let cy = min_y + r as f64 * cell_size + hs;
                    grid.push(vec![[cx, cy - hs], [cx + hs, cy], [cx, cy + hs], [cx - hs, cy], [cx, cy - hs]]);
                }
            }
        }
    }
    Ok(grid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simplification_and_smoothing() {
        let pts = vec![[0.0, 0.0], [1.0, 0.1], [2.0, -0.05], [3.0, 0.0], [4.0, 2.0], [5.0, 0.0]];
        let (mut flags, mut out) = (vec![false; pts.len()], vec![[0.0, 0.0]; pts.len()]);
        let n = gp_simplify_douglas_peucker(&pts, 0.5, &mut flags, &mut out).unwrap();
        assert_eq!(n, 4);
        assert_eq!(out[0], [0.0, 0.0]); assert_eq!(out[1], [3.0, 0.0]); assert_eq!(out[2], [4.0, 2.0]); assert_eq!(out[3], [5.0, 0.0]);

        let vw = gp_simplify_visvalingam_whyatt(&pts, 0.5, 10_000).unwrap();
        assert!(vw.len() < pts.len());
        assert_eq!(vw.first().unwrap(), &[0.0, 0.0]); assert_eq!(vw.last().unwrap(), &[5.0, 0.0]);

        let ch = gp_smooth_chaikin(&pts[..4], 2, false, 10_000).unwrap();
        assert!(ch.len() > 4);
    }

    #[test]
    fn test_convex_concave_and_polylabel_l_shape() {
        let pts = vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [1.0, 1.0]];
        let hull = gp_convex_hull(&pts, 10_000).unwrap();
        assert_eq!(hull.len(), 5);

        let alpha = gp_concave_hull_alpha(&pts, 2.5, 10_000).unwrap();
        assert!(alpha.len() >= 4);

        let l_shape = vec![[0.0, 0.0], [3.0, 0.0], [3.0, 1.0], [1.0, 1.0], [1.0, 3.0], [0.0, 3.0], [0.0, 0.0]];
        let (centroid, pole, clearance) = gp_centroid_and_polylabel(&l_shape, 0.05).unwrap();
        assert!(!pt_in_ring(centroid, &l_shape), "Centroid must be outside L-shape: {:?}", centroid);
        assert!(pt_in_ring(pole, &l_shape), "Polylabel must lie strictly inside L-shape: {:?}", pole);
        assert!(clearance > 0.4, "Clearance radius inside L-shape arms: {}", clearance);
    }

    #[test]
    fn test_delaunay_voronoi_tessellation_and_budget() {
        let pts = vec![[2.0, 2.0], [8.0, 2.0], [2.0, 8.0], [8.0, 8.0], [5.0, 5.0]];
        let tris = gp_delaunay_triangulate(&pts, 10_000).unwrap();
        assert!(tris.len() >= 4);

        let bbox = [0.0, 0.0, 10.0, 10.0];
        let cells = gp_voronoi_diagram_bounded(&pts, bbox, 100_000).unwrap();
        assert_eq!(cells.len(), 5);
        let mut total_area = 0.0;
        for cell in &cells { total_area += polygon_area(cell); }
        assert!((total_area - 100.0).abs() < 1e-4, "Voronoi cells must partition bbox (100): {}", total_area);

        let sq = gp_generate_tessellation_grid(bbox, 2.0, TessellationShape::Square, 100_000).unwrap();
        assert_eq!(sq.len(), 25);
        let hex = gp_generate_tessellation_grid(bbox, 2.0, TessellationShape::HexagonFlatTop, 100_000).unwrap();
        assert!(!hex.is_empty());
        let dia = gp_generate_tessellation_grid(bbox, 2.0, TessellationShape::Diamond, 100_000).unwrap();
        assert!(!dia.is_empty());

        let budget_err = gp_delaunay_triangulate(&pts, 10).unwrap_err();
        assert!(matches!(budget_err, GpGeomError::ByteBudgetExceeded { .. }));
    }
}
