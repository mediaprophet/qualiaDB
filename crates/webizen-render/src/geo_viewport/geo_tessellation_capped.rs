// Webizen Render: Capped-Angle Miter/Round Polyline & Polygon Tessellation,
// CDLOD Terrain Quadtree Mesh Generator, and Crack-Free Skirt Stitcher.
//
// Adapted from RustGIS (v0.4.0, Apache-2.0 / MIT) and EarthCraft (v0.2.0, MIT).
// SPDX-License-Identifier: MIT OR Apache-2.0

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoRenderVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StrokeJoinCapStyle {
    MiterCapped { miter_limit: f32 },
    Bevel,
    Round { segments: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CdlodNodeSpec {
    pub min_xy: [f32; 2],
    pub size_m: f32,
    pub lod_level: u8,
    pub morph_factor: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeoTessError {
    ByteBudgetExceeded { required_bytes: usize, byte_budget: usize },
    InvalidInput(&'static str),
    DegeneratePolygon,
}

impl core::fmt::Display for GeoTessError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ByteBudgetExceeded { required_bytes, byte_budget } => {
                write!(f, "byte budget exceeded: required {required_bytes} > budget {byte_budget}")
            }
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::DegeneratePolygon => write!(f, "degenerate polygon cannot be triangulated"),
        }
    }
}
impl std::error::Error for GeoTessError {}

#[inline]
fn check_budget(curr: usize, add: usize, budget: usize) -> Result<usize, GeoTessError> {
    let tot = curr.saturating_add(add);
    if tot > budget { Err(GeoTessError::ByteBudgetExceeded { required_bytes: tot, byte_budget: budget }) }
    else { Ok(tot) }
}

pub fn tessellate_polyline_stroke_capped(
    pts: &[[f32; 3]],
    half_width: f32,
    style: StrokeJoinCapStyle,
    byte_budget: usize,
) -> Result<(Vec<GeoRenderVertex>, Vec<u32>), GeoTessError> {
    if pts.len() < 2 { return Err(GeoTessError::InvalidInput("polyline requires >= 2 points")); }
    let n = pts.len();
    let (mut total_len, mut seg_lens) = (0.0f32, Vec::with_capacity(n - 1));
    let (mut dirs, mut norms) = (Vec::with_capacity(n - 1), Vec::with_capacity(n - 1));

    for i in 0..n - 1 {
        let (dx, dy, dz) = (pts[i + 1][0] - pts[i][0], pts[i + 1][1] - pts[i][1], pts[i + 1][2] - pts[i][2]);
        let (l3, l2) = ((dx * dx + dy * dy + dz * dz).sqrt().max(1e-6), (dx * dx + dy * dy).sqrt().max(1e-6));
        seg_lens.push(l3); total_len += l3;
        dirs.push([dx / l2, dy / l2]); norms.push([-dy / l2, dx / l2]);
    }

    let (mut verts, mut indices, mut bytes) = (Vec::new(), Vec::new(), 0usize);
    let (v_sz, i_sz, up) = (core::mem::size_of::<GeoRenderVertex>(), core::mem::size_of::<u32>(), [0.0, 0.0, 1.0]);

    let mut cum_len = 0.0f32;
    for i in 0..n - 1 {
        let (p0, p1, norm) = (pts[i], pts[i + 1], norms[i]);
        let (u0, u1) = (cum_len / total_len, (cum_len + seg_lens[i]) / total_len);
        cum_len += seg_lens[i];
        bytes = check_budget(bytes, 4 * v_sz + 6 * i_sz, byte_budget)?;
        let base = verts.len() as u32;
        let r0 = [p0[0] - half_width * norm[0], p0[1] - half_width * norm[1], p0[2]];
        let l0 = [p0[0] + half_width * norm[0], p0[1] + half_width * norm[1], p0[2]];
        let r1 = [p1[0] - half_width * norm[0], p1[1] - half_width * norm[1], p1[2]];
        let l1 = [p1[0] + half_width * norm[0], p1[1] + half_width * norm[1], p1[2]];

        verts.push(GeoRenderVertex { pos: r0, normal: up, uv: [u0, 0.0] });
        verts.push(GeoRenderVertex { pos: l0, normal: up, uv: [u0, 1.0] });
        verts.push(GeoRenderVertex { pos: r1, normal: up, uv: [u1, 0.0] });
        verts.push(GeoRenderVertex { pos: l1, normal: up, uv: [u1, 1.0] });
        indices.extend_from_slice(&[base, base + 2, base + 1, base + 1, base + 2, base + 3]);
    }

    cum_len = 0.0f32;
    for k in 0..n - 2 {
        cum_len += seg_lens[k];
        let (p_j, n0, n1, d0, d1) = (pts[k + 1], norms[k], norms[k + 1], dirs[k], dirs[k + 1]);
        let cross = d0[0] * d1[1] - d0[1] * d1[0];
        if cross.abs() <= 1e-4 { continue; }

        let u_join = cum_len / total_len;
        let c_idx = verts.len() as u32;
        bytes = check_budget(bytes, v_sz, byte_budget)?;
        verts.push(GeoRenderVertex { pos: p_j, normal: up, uv: [u_join, 0.5] });

        let bisect = [n0[0] + n1[0], n0[1] + n1[1]];
        let b_len = (bisect[0] * bisect[0] + bisect[1] * bisect[1]).sqrt().max(1e-6);
        let b_hat = [bisect[0] / b_len, bisect[1] / b_len];
        let dot = (n0[0] * b_hat[0] + n0[1] * b_hat[1]).abs().max(1e-4);
        let miter_len = half_width / dot;

        let (limit, is_round, segs) = match style {
            StrokeJoinCapStyle::MiterCapped { miter_limit } => (miter_limit, false, 1u8),
            StrokeJoinCapStyle::Bevel => (1.0, false, 1u8),
            StrokeJoinCapStyle::Round { segments } => (1.0, true, segments.max(1)),
        };

        let (r_prev, l_prev) = ((k * 4 + 2) as u32, (k * 4 + 3) as u32);
        let (r_next, l_next) = (((k + 1) * 4) as u32, ((k + 1) * 4 + 1) as u32);
        let (outer_r, sgn) = if cross > 0.0 { (true, -1.0f32) } else { (false, 1.0f32) };
        let (o_prev, o_next) = if outer_r { (r_prev, r_next) } else { (l_prev, l_next) };
        let (i_prev, i_next) = if outer_r { (l_prev, l_next) } else { (r_prev, r_next) };
        let o_uv = if outer_r { 0.0 } else { 1.0 };

        bytes = check_budget(bytes, 3 * i_sz, byte_budget)?;
        if outer_r { indices.extend_from_slice(&[c_idx, i_prev, i_next]); }
        else { indices.extend_from_slice(&[c_idx, i_next, i_prev]); }

        if !is_round && miter_len <= limit * half_width {
            let m_pos = [p_j[0] + sgn * miter_len * b_hat[0], p_j[1] + sgn * miter_len * b_hat[1], p_j[2]];
            let m_idx = verts.len() as u32;
            bytes = check_budget(bytes, v_sz + 6 * i_sz, byte_budget)?;
            verts.push(GeoRenderVertex { pos: m_pos, normal: up, uv: [u_join, o_uv] });
            if outer_r { indices.extend_from_slice(&[c_idx, o_prev, m_idx, c_idx, m_idx, o_next]); }
            else { indices.extend_from_slice(&[c_idx, m_idx, o_prev, c_idx, o_next, m_idx]); }
        } else if is_round {
            let base_n0 = if outer_r { [-n0[0], -n0[1]] } else { n0 };
            let base_n1 = if outer_r { [-n1[0], -n1[1]] } else { n1 };
            let a0 = base_n0[1].atan2(base_n0[0]);
            let mut a1 = base_n1[1].atan2(base_n1[0]);
            if a1 < a0 { a1 += core::f32::consts::TAU; }
            let mut prev_arc = o_prev;
            for s in 1..segs {
                let ang = a0 + (s as f32 / segs as f32) * (a1 - a0);
                let arc_pos = [p_j[0] + half_width * ang.cos(), p_j[1] + half_width * ang.sin(), p_j[2]];
                let arc_idx = verts.len() as u32;
                bytes = check_budget(bytes, v_sz + 3 * i_sz, byte_budget)?;
                verts.push(GeoRenderVertex { pos: arc_pos, normal: up, uv: [u_join, o_uv] });
                if outer_r { indices.extend_from_slice(&[c_idx, prev_arc, arc_idx]); }
                else { indices.extend_from_slice(&[c_idx, arc_idx, prev_arc]); }
                prev_arc = arc_idx;
            }
            bytes = check_budget(bytes, 3 * i_sz, byte_budget)?;
            if outer_r { indices.extend_from_slice(&[c_idx, prev_arc, o_next]); }
            else { indices.extend_from_slice(&[c_idx, o_next, prev_arc]); }
        } else {
            bytes = check_budget(bytes, 3 * i_sz, byte_budget)?;
            if outer_r { indices.extend_from_slice(&[c_idx, o_prev, o_next]); }
            else { indices.extend_from_slice(&[c_idx, o_next, o_prev]); }
        }
    }
    Ok((verts, indices))
}

#[inline]
fn point_in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let c1 = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
    let c2 = (c[0] - b[0]) * (p[1] - b[1]) - (c[1] - b[1]) * (p[0] - b[0]);
    let c3 = (a[0] - c[0]) * (p[1] - c[1]) - (a[1] - c[1]) * (p[0] - c[0]);
    c1 >= -1e-6 && c2 >= -1e-6 && c3 >= -1e-6
}

pub fn tessellate_polygon_earcut(
    exterior: &[[f32; 2]],
    elevation_z: f32,
    byte_budget: usize,
) -> Result<(Vec<GeoRenderVertex>, Vec<u32>), GeoTessError> {
    let n = exterior.len();
    if n < 3 { return Err(GeoTessError::InvalidInput("polygon exterior requires >= 3 points")); }
    let req = n * core::mem::size_of::<GeoRenderVertex>() + (n - 2) * 3 * core::mem::size_of::<u32>();
    if req > byte_budget { return Err(GeoTessError::ByteBudgetExceeded { required_bytes: req, byte_budget }); }

    let (mut min_x, mut max_x, mut min_y, mut max_y, mut area) =
        (exterior[0][0], exterior[0][0], exterior[0][1], exterior[0][1], 0.0f32);
    for i in 0..n {
        let (p, next) = (exterior[i], exterior[(i + 1) % n]);
        min_x = min_x.min(p[0]); max_x = max_x.max(p[0]);
        min_y = min_y.min(p[1]); max_y = max_y.max(p[1]);
        area += p[0] * next[1] - next[0] * p[1];
    }
    if area.abs() < 1e-8 { return Err(GeoTessError::DegeneratePolygon); }

    let (dx, dy, up) = ((max_x - min_x).max(1e-6), (max_y - min_y).max(1e-6), [0.0, 0.0, 1.0]);
    let mut verts = Vec::with_capacity(n);
    for p in exterior {
        verts.push(GeoRenderVertex { pos: [p[0], p[1], elevation_z], normal: up, uv: [(p[0] - min_x) / dx, (p[1] - min_y) / dy] });
    }

    let mut v_indices: Vec<usize> = if area > 0.0 { (0..n).collect() } else { (0..n).rev().collect() };
    let mut tri_indices = Vec::with_capacity((n - 2) * 3);
    let mut watchdog = 2 * v_indices.len();

    while v_indices.len() > 3 && watchdog > 0 {
        watchdog -= 1;
        let len = v_indices.len();
        let mut ear_found = false;
        for i in 0..len {
            let (prev, curr, next) = (v_indices[(i + len - 1) % len], v_indices[i], v_indices[(i + 1) % len]);
            let (a, b, c) = (exterior[prev], exterior[curr], exterior[next]);
            if (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]) <= 1e-7 { continue; }

            let mut is_ear = true;
            for &k in &v_indices {
                if k != prev && k != curr && k != next && point_in_triangle(exterior[k], a, b, c) {
                    is_ear = false; break;
                }
            }
            if is_ear {
                tri_indices.extend_from_slice(&[prev as u32, curr as u32, next as u32]);
                v_indices.remove(i);
                ear_found = true;
                break;
            }
        }
        if !ear_found {
            let (p0, p1, p2) = (v_indices[0], v_indices[1], v_indices[2]);
            tri_indices.extend_from_slice(&[p0 as u32, p1 as u32, p2 as u32]);
            v_indices.remove(1);
        }
    }
    if v_indices.len() == 3 {
        tri_indices.extend_from_slice(&[v_indices[0] as u32, v_indices[1] as u32, v_indices[2] as u32]);
    }
    Ok((verts, tri_indices))
}

pub fn select_cdlod_quadtree_nodes(
    root_min_xy: [f32; 2],
    root_size_m: f32,
    max_lod_depth: u8,
    camera_xyz: [f32; 3],
    viewport_height_px: f32,
    fov_rad: f32,
    max_sse_px: f32,
    byte_budget: usize,
) -> Result<Vec<CdlodNodeSpec>, GeoTessError> {
    let node_sz = core::mem::size_of::<CdlodNodeSpec>();
    let tan_half_fov = (fov_rad * 0.5).tan().max(1e-4);
    let (mut selected, mut bytes) = (Vec::new(), 0usize);
    let mut stack = Vec::with_capacity(32);
    stack.push((root_min_xy, root_size_m, 0u8));

    while let Some((min_xy, size_m, depth)) = stack.pop() {
        let clamp_x = camera_xyz[0].clamp(min_xy[0], min_xy[0] + size_m);
        let clamp_y = camera_xyz[1].clamp(min_xy[1], min_xy[1] + size_m);
        let (dx, dy, dz) = (camera_xyz[0] - clamp_x, camera_xyz[1] - clamp_y, camera_xyz[2]);
        let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.01);
        let sse = (size_m * 0.05 * viewport_height_px) / (2.0 * dist * tan_half_fov);

        if sse > max_sse_px && depth < max_lod_depth {
            let (half, next_depth) = (size_m * 0.5, depth + 1);
            stack.push(([min_xy[0] + half, min_xy[1] + half], half, next_depth));
            stack.push(([min_xy[0], min_xy[1] + half], half, next_depth));
            stack.push(([min_xy[0] + half, min_xy[1]], half, next_depth));
            stack.push((min_xy, half, next_depth));
        } else {
            bytes = check_budget(bytes, node_sz, byte_budget)?;
            let morph = ((sse - 0.5 * max_sse_px) / (0.5 * max_sse_px)).clamp(0.0, 1.0);
            selected.push(CdlodNodeSpec { min_xy, size_m, lod_level: depth, morph_factor: morph });
        }
    }
    Ok(selected)
}

pub fn build_terrain_patch_with_skirts(
    dem: &[f32],
    grid_w: usize,
    grid_h: usize,
    cell_size_m: f32,
    skirt_drop_m: f32,
    byte_budget: usize,
) -> Result<(Vec<GeoRenderVertex>, Vec<u32>), GeoTessError> {
    if grid_w < 2 || grid_h < 2 || dem.len() < grid_w * grid_h {
        return Err(GeoTessError::InvalidInput("grid must be >= 2x2 and match dem length"));
    }
    let total_v = grid_w * grid_h + 2 * grid_w + 2 * grid_h;
    let total_t = 2 * (grid_w - 1) * (grid_h - 1) + 4 * (grid_w - 1) + 4 * (grid_h - 1);
    let req = total_v * core::mem::size_of::<GeoRenderVertex>() + total_t * 3 * core::mem::size_of::<u32>();
    if req > byte_budget { return Err(GeoTessError::ByteBudgetExceeded { required_bytes: req, byte_budget }); }

    let (mut verts, mut indices) = (Vec::with_capacity(total_v), Vec::with_capacity(total_t * 3));

    // 1. Interior vertices
    for y in 0..grid_h {
        for x in 0..grid_w {
            let z = dem[y * grid_w + x];
            let dz_dx = if x > 0 && x < grid_w - 1 {
                (dem[y * grid_w + (x + 1)] - dem[y * grid_w + (x - 1)]) / (2.0 * cell_size_m)
            } else if x == 0 { (dem[y * grid_w + 1] - dem[y * grid_w]) / cell_size_m }
            else { (dem[y * grid_w + x] - dem[y * grid_w + (x - 1)]) / cell_size_m };

            let dz_dy = if y > 0 && y < grid_h - 1 {
                (dem[(y + 1) * grid_w + x] - dem[(y - 1) * grid_w + x]) / (2.0 * cell_size_m)
            } else if y == 0 { (dem[grid_w + x] - dem[x]) / cell_size_m }
            else { (dem[y * grid_w + x] - dem[(y - 1) * grid_w + x]) / cell_size_m };

            let norm_len = (dz_dx * dz_dx + dz_dy * dz_dy + 1.0).sqrt();
            verts.push(GeoRenderVertex {
                pos: [x as f32 * cell_size_m, y as f32 * cell_size_m, z],
                normal: [-dz_dx / norm_len, -dz_dy / norm_len, 1.0 / norm_len],
                uv: [x as f32 / (grid_w - 1) as f32, y as f32 / (grid_h - 1) as f32],
            });
        }
    }

    // 2. Interior triangles
    for y in 0..grid_h - 1 {
        for x in 0..grid_w - 1 {
            let (i00, i10) = ((y * grid_w + x) as u32, (y * grid_w + (x + 1)) as u32);
            let (i01, i11) = (((y + 1) * grid_w + x) as u32, ((y + 1) * grid_w + (x + 1)) as u32);
            indices.extend_from_slice(&[i00, i10, i01, i01, i10, i11]);
        }
    }

    // 3. Perimeter skirts: South (-Y), North (+Y), West (-X), East (+X)
    let skirts: [(usize, [f32; 3], bool); 4] = [
        (0, [0.0, -1.0, 0.0], false),
        (1, [0.0, 1.0, 0.0], true),
        (2, [-1.0, 0.0, 0.0], true),
        (3, [1.0, 0.0, 0.0], false),
    ];

    for (side, norm, rev_wind) in skirts {
        let count = if side < 2 { grid_w } else { grid_h };
        let base_idx = verts.len() as u32;
        for i in 0..count {
            let (top_idx, x_m, y_m, uv) = match side {
                0 => (i, i as f32 * cell_size_m, 0.0, [i as f32 / (grid_w - 1) as f32, 0.0]),
                1 => ((grid_h - 1) * grid_w + i, i as f32 * cell_size_m, (grid_h - 1) as f32 * cell_size_m, [i as f32 / (grid_w - 1) as f32, 1.0]),
                2 => (i * grid_w, 0.0, i as f32 * cell_size_m, [0.0, i as f32 / (grid_h - 1) as f32]),
                _ => (i * grid_w + (grid_w - 1), (grid_w - 1) as f32 * cell_size_m, i as f32 * cell_size_m, [1.0, i as f32 / (grid_h - 1) as f32]),
            };
            verts.push(GeoRenderVertex { pos: [x_m, y_m, dem[top_idx] - skirt_drop_m], normal: norm, uv });
        }
        for i in 0..count - 1 {
            let (t0, t1) = match side {
                0 => (i as u32, (i + 1) as u32),
                1 => (((grid_h - 1) * grid_w + i) as u32, ((grid_h - 1) * grid_w + i + 1) as u32),
                2 => ((i * grid_w) as u32, ((i + 1) * grid_w) as u32),
                _ => ((i * grid_w + grid_w - 1) as u32, ((i + 1) * grid_w + grid_w - 1) as u32),
            };
            let (b0, b1) = (base_idx + i as u32, base_idx + (i + 1) as u32);
            if rev_wind { indices.extend_from_slice(&[t0, t1, b0, t1, b1, b0]); }
            else { indices.extend_from_slice(&[t0, b0, t1, b0, b1, t1]); }
        }
    }
    Ok((verts, indices))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_render_vertex_size() {
        assert_eq!(core::mem::size_of::<GeoRenderVertex>(), 32);
    }

    #[test]
    fn test_capped_miter_polyline_clamping_sharp_turn() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [10.0, 0.0, 0.0];
        let rad = 5.0f32.to_radians();
        let p2 = [10.0 - 10.0 * rad.cos(), 10.0 * rad.sin(), 0.0];
        let (half_width, miter_limit) = (1.0f32, 2.0f32);
        let style = StrokeJoinCapStyle::MiterCapped { miter_limit };
        let (verts, indices) = tessellate_polyline_stroke_capped(&[p0, p1, p2], half_width, style, 100_000).unwrap();

        assert!(!verts.is_empty() && !indices.is_empty());
        let max_allowed = miter_limit * half_width + 0.01;
        for v in &verts {
            let (dx, dy) = (v.pos[0] - p1[0], v.pos[1] - p1[1]);
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < 8.0 {
                assert!(dist <= max_allowed, "vertex at {:?} dist {} > max {}", v.pos, dist, max_allowed);
            }
        }
    }

    #[test]
    fn test_concave_l_shape_polygon_earcut() {
        let exterior = [
            [0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0],
        ];
        let (verts, indices) = tessellate_polygon_earcut(&exterior, 10.0, 100_000).unwrap();
        assert_eq!(verts.len(), 6);
        assert_eq!(indices.len(), 12);

        let mut total_area = 0.0f32;
        for i in (0..indices.len()).step_by(3) {
            let (p0, p1, p2) = (verts[indices[i] as usize].pos, verts[indices[i + 1] as usize].pos, verts[indices[i + 2] as usize].pos);
            let a = (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p1[1] - p0[1]) * (p2[0] - p0[0]);
            total_area += a.abs() * 0.5;
        }
        assert!((total_area - 3.0).abs() < 1e-4);
    }

    #[test]
    fn test_cdlod_quadtree_subdivision_and_morph() {
        let (root_min, root_size, max_lod, camera) = ([0.0, 0.0], 1000.0, 3, [10.0, 10.0, 5.0]);
        let nodes = select_cdlod_quadtree_nodes(
            root_min, root_size, max_lod, camera, 1080.0, 1.0, 15.0, 100_000,
        ).unwrap();
        assert!(!nodes.is_empty());
        assert!(nodes.iter().any(|n| n.lod_level == max_lod));
        assert!(nodes.iter().any(|n| n.lod_level < max_lod));
        for n in &nodes {
            assert!(n.morph_factor >= 0.0 && n.morph_factor <= 1.0);
        }
    }

    #[test]
    fn test_terrain_patch_with_skirts_and_normals() {
        let dem = [0.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 3.0, 4.0];
        let (verts, indices) = build_terrain_patch_with_skirts(&dem, 3, 3, 10.0, 5.0, 100_000).unwrap();
        assert_eq!(verts.len(), 21);
        assert!(verts.iter().any(|v| (v.pos[2] - (-5.0)).abs() < 1e-4));
        assert!(verts.iter().any(|v| v.normal == [0.0, -1.0, 0.0]));
        assert!(verts.iter().any(|v| v.normal == [0.0, 1.0, 0.0]));
        assert!(verts.iter().any(|v| v.normal == [-1.0, 0.0, 0.0]));
        assert!(verts.iter().any(|v| v.normal == [1.0, 0.0, 0.0]));
        assert!(!indices.is_empty());
    }

    #[test]
    fn test_byte_budget_exceeded_fail_closed() {
        let pts = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0]];
        let err = tessellate_polyline_stroke_capped(&pts, 1.0, StrokeJoinCapStyle::Bevel, 10).unwrap_err();
        assert!(matches!(err, GeoTessError::ByteBudgetExceeded { .. }));

        let ext = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
        let err = tessellate_polygon_earcut(&ext, 0.0, 10).unwrap_err();
        assert!(matches!(err, GeoTessError::ByteBudgetExceeded { .. }));

        let dem = [0.0; 9];
        let err = build_terrain_patch_with_skirts(&dem, 3, 3, 1.0, 1.0, 10).unwrap_err();
        assert!(matches!(err, GeoTessError::ByteBudgetExceeded { .. }));
    }
}
