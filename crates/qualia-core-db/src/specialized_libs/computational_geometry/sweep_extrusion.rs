//! P20.1 — Rotation-Minimizing Frames (Bishop / Double-Reflection), generalized
//! 3-D profile extrusion with per-station scale and twist, swung surfaces,
//! analytic botanical primitives (frustum, cone, paraboloid, disc), axial
//! taper/twist/bend deformations, and bounded Iterated Function Systems (IFS).
//!
//! Clean-room implementation from public mathematical literature:
//! - Wang, Jüttler, Zheng & Liu (2008), "Computation of Rotation Minimizing
//!   Frames", *ACM TOG* 27(1) (Double-Reflection Method).

use super::math_geometry::AffineFrame3;
use super::primitives::{Point2, Point3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepError {
    InvalidInput,
    DegenerateTangent {
        index: usize,
    },
    OutputTooSmall {
        required_vertices: usize,
        required_triangles: usize,
    },
    DepthExceeded {
        max_depth: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExtrusionStation {
    pub scale_x: f64,
    pub scale_y: f64,
    pub twist_rad: f64,
}

impl Default for ExtrusionStation {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            twist_rad: 0.0,
        }
    }
}

#[inline]
fn add(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}
#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn scale(a: Point3, s: f64) -> Point3 {
    Point3::new(a.x * s, a.y * s, a.z * s)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn cross(a: Point3, b: Point3) -> Point3 {
    Point3::new(
        a.y * b.z - a.z * b.y,
        a.z * b.x - a.x * b.z,
        a.x * b.y - a.y * b.x,
    )
}
#[inline]
fn norm(a: Point3) -> f64 {
    dot(a, a).sqrt()
}
#[inline]
fn normalize(a: Point3) -> Option<Point3> {
    let n = norm(a);
    (n > 1e-12 && n.is_finite()).then(|| scale(a, 1.0 / n))
}

/// Compute Rotation-Minimizing Frames (Bishop frames) along a 3-D polyline
/// using Wang et al.'s (2008) exact Double-Reflection Method.
///
/// In each output [`AffineFrame3`]:
/// - `origin` is `path[i]`
/// - `e0` is the normal vector $\mathbf{R}_i$
/// - `e1` is the binormal vector $\mathbf{S}_i = \mathbf{T}_i \times \mathbf{R}_i$
/// - `e2` is the unit tangent vector $\mathbf{T}_i$
///
/// Tier-1 `HotZeroHeap`: zero heap allocations, writes directly to `out_frames`.
pub fn compute_bishop_frames(
    path: &[Point3],
    initial_normal: Option<Point3>,
    out_frames: &mut [AffineFrame3],
) -> Result<usize, SweepError> {
    let n = path.len();
    if n < 2 {
        return Err(SweepError::InvalidInput);
    }
    if out_frames.len() < n {
        return Err(SweepError::OutputTooSmall {
            required_vertices: n,
            required_triangles: 0,
        });
    }

    let tangent_at = |idx: usize| -> Result<Point3, SweepError> {
        let dir = if idx == 0 {
            sub(path[1], path[0])
        } else if idx + 1 == n {
            sub(path[n - 1], path[n - 2])
        } else {
            sub(path[idx + 1], path[idx - 1])
        };
        normalize(dir).ok_or(SweepError::DegenerateTangent { index: idx })
    };

    let t0 = tangent_at(0)?;
    let r0 = if let Some(init) = initial_normal {
        let proj = sub(init, scale(t0, dot(init, t0)));
        normalize(proj).unwrap_or_else(|| orthogonal_unit(t0))
    } else {
        orthogonal_unit(t0)
    };
    let s0 = cross(t0, r0);
    out_frames[0] = AffineFrame3 {
        origin: path[0],
        e0: r0,
        e1: s0,
        e2: t0,
    };

    for i in 0..n - 1 {
        let ti = out_frames[i].e2;
        let ri = out_frames[i].e0;
        let ti1 = tangent_at(i + 1)?;
        let v1 = sub(path[i + 1], path[i]);
        let c1 = dot(v1, v1);
        let r_next = if c1 > 1e-20 {
            // First reflection across plane bisecting path[i] -> path[i+1]
            let r_l = sub(ri, scale(v1, (2.0 / c1) * dot(v1, ri)));
            let t_l = sub(ti, scale(v1, (2.0 / c1) * dot(v1, ti)));
            // Second reflection aligning t_l with ti1
            let v2 = sub(ti1, t_l);
            let c2 = dot(v2, v2);
            if c2 > 1e-20 {
                sub(r_l, scale(v2, (2.0 / c2) * dot(v2, r_l)))
            } else {
                r_l
            }
        } else {
            ri
        };
        let r_ortho = sub(r_next, scale(ti1, dot(r_next, ti1)));
        let r1 = normalize(r_ortho).unwrap_or_else(|| orthogonal_unit(ti1));
        let s1 = cross(ti1, r1);
        out_frames[i + 1] = AffineFrame3 {
            origin: path[i + 1],
            e0: r1,
            e1: s1,
            e2: ti1,
        };
    }

    Ok(n)
}

#[inline]
fn orthogonal_unit(t: Point3) -> Point3 {
    let cand = if t.x.abs() < 0.9 {
        Point3::new(1.0, 0.0, 0.0)
    } else {
        Point3::new(0.0, 1.0, 0.0)
    };
    let proj = sub(cand, scale(t, dot(cand, t)));
    normalize(proj).unwrap_or(Point3::new(1.0, 0.0, 0.0))
}

/// Sweep a 2-D cross-section profile along a 3-D axis curve using Bishop frames,
/// per-station 2-D scaling, and twist angles.
///
/// If `closed_section` is true, connects the last profile point back to point 0.
/// If `cap_ends` is true (and `closed_section` is true), adds bottom and top
/// centroid fans to produce a watertight 2-manifold surface.
pub fn extrude_profile_along_path(
    axis: &[Point3],
    cross_section: &[Point2],
    stations: Option<&[ExtrusionStation]>,
    closed_section: bool,
    cap_ends: bool,
    frames_scratch: &mut [AffineFrame3],
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), SweepError> {
    let n_axis = axis.len();
    let m_sec = cross_section.len();
    if n_axis < 2 || m_sec < 2 || (closed_section && m_sec < 3) {
        return Err(SweepError::InvalidInput);
    }
    if let Some(st) = stations {
        if st.len() != n_axis {
            return Err(SweepError::InvalidInput);
        }
    }

    let cap_verts = if closed_section && cap_ends { 2 } else { 0 };
    let req_v = n_axis * m_sec + cap_verts;
    let segs_per_ring = if closed_section { m_sec } else { m_sec - 1 };
    let cap_tris = if closed_section && cap_ends {
        2 * m_sec
    } else {
        0
    };
    let req_t = (n_axis - 1) * segs_per_ring * 2 + cap_tris;

    if out_vertices.len() < req_v || out_triangles.len() < req_t || frames_scratch.len() < n_axis {
        return Err(SweepError::OutputTooSmall {
            required_vertices: req_v,
            required_triangles: req_t,
        });
    }

    compute_bishop_frames(axis, None, frames_scratch)?;

    for i in 0..n_axis {
        let frame = frames_scratch[i];
        let st = stations.map(|s| s[i]).unwrap_or_default();
        if !st.scale_x.is_finite() || !st.scale_y.is_finite() || !st.twist_rad.is_finite() {
            return Err(SweepError::InvalidInput);
        }
        let (ct, stw) = (st.twist_rad.cos(), st.twist_rad.sin());
        for j in 0..m_sec {
            let sx = cross_section[j].x * st.scale_x;
            let sy = cross_section[j].y * st.scale_y;
            let rx = sx * ct - sy * stw;
            let ry = sx * stw + sy * ct;
            out_vertices[i * m_sec + j] =
                add(frame.origin, add(scale(frame.e0, rx), scale(frame.e1, ry)));
        }
    }

    let mut ti = 0usize;
    for i in 0..n_axis - 1 {
        let base0 = (i * m_sec) as u32;
        let base1 = ((i + 1) * m_sec) as u32;
        for j in 0..segs_per_ring {
            let j_next = if closed_section {
                (j + 1) % m_sec
            } else {
                j + 1
            };
            let a = base0 + j as u32;
            let b = base0 + j_next as u32;
            let c = base1 + j_next as u32;
            let d = base1 + j as u32;
            out_triangles[ti] = [a, b, c];
            out_triangles[ti + 1] = [a, c, d];
            ti += 2;
        }
    }

    if closed_section && cap_ends {
        let bot_center_idx = (n_axis * m_sec) as u32;
        let top_center_idx = bot_center_idx + 1;
        out_vertices[bot_center_idx as usize] = ring_centroid(&out_vertices[0..m_sec]);
        let top_start = (n_axis - 1) * m_sec;
        out_vertices[top_center_idx as usize] =
            ring_centroid(&out_vertices[top_start..top_start + m_sec]);

        for j in 0..m_sec {
            let j_next = (j + 1) % m_sec;
            out_triangles[ti] = [bot_center_idx, j_next as u32, j as u32];
            ti += 1;
        }
        let top_base = top_start as u32;
        for j in 0..m_sec {
            let j_next = (j + 1) % m_sec;
            out_triangles[ti] = [
                top_center_idx,
                top_base + j as u32,
                top_base + j_next as u32,
            ];
            ti += 1;
        }
    }

    Ok((req_v, req_t))
}

#[inline]
fn ring_centroid(ring: &[Point3]) -> Point3 {
    let mut sum = Point3::new(0.0, 0.0, 0.0);
    for &p in ring {
        sum = add(sum, p);
    }
    scale(sum, 1.0 / (ring.len().max(1) as f64))
}

/// Generate a PlantGL-class `Swung` rotational surface by interpolating between
/// `profiles` (each a 2-D `(radius, z)` curve of identical vertex count `m`)
/// placed at monotonically increasing azimuthal `angles` in $[0, 2\pi)$.
pub fn swung_surface(
    profiles: &[&[Point2]],
    angles: &[f64],
    slices: usize,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), SweepError> {
    if profiles.is_empty()
        || profiles.len() != angles.len()
        || slices < 3
        || angles
            .iter()
            .any(|a| !a.is_finite() || *a < 0.0 || *a >= core::f64::consts::TAU)
    {
        return Err(SweepError::InvalidInput);
    }
    let m = profiles[0].len();
    if m < 2
        || profiles.iter().any(|p| {
            p.len() != m
                || p.iter()
                    .any(|point| !point.x.is_finite() || !point.y.is_finite())
        })
    {
        return Err(SweepError::InvalidInput);
    }
    for w in angles.windows(2) {
        if !(w[1] > w[0]) {
            return Err(SweepError::InvalidInput);
        }
    }

    let req_v = slices.checked_mul(m).ok_or(SweepError::InvalidInput)?;
    let req_t = slices
        .checked_mul(m - 1)
        .and_then(|n| n.checked_mul(2))
        .ok_or(SweepError::InvalidInput)?;
    if req_v > u32::MAX as usize {
        return Err(SweepError::InvalidInput);
    }
    if out_vertices.len() < req_v || out_triangles.len() < req_t {
        return Err(SweepError::OutputTooSmall {
            required_vertices: req_v,
            required_triangles: req_t,
        });
    }

    let k = profiles.len();
    for s in 0..slices {
        let theta = core::f64::consts::TAU * (s as f64) / (slices as f64);
        // Find surrounding angular profile pair (wrapping around 2*pi).
        let (p0, p1, t_blend) = if k == 1 {
            (profiles[0], profiles[0], 0.0)
        } else {
            let mut seg = k - 1;
            for idx in 0..k - 1 {
                if theta >= angles[idx] && theta <= angles[idx + 1] {
                    seg = idx;
                    break;
                }
            }
            if seg < k - 1 && theta >= angles[seg] && theta <= angles[seg + 1] {
                let span = (angles[seg + 1] - angles[seg]).max(1e-12);
                (
                    profiles[seg],
                    profiles[seg + 1],
                    (theta - angles[seg]) / span,
                )
            } else {
                let a_last = angles[k - 1];
                let a_first = angles[0] + core::f64::consts::TAU;
                let theta_w = if theta < angles[0] {
                    theta + core::f64::consts::TAU
                } else {
                    theta
                };
                let span = (a_first - a_last).max(1e-12);
                (
                    profiles[k - 1],
                    profiles[0],
                    ((theta_w - a_last) / span).clamp(0.0, 1.0),
                )
            }
        };

        let (ca, sa) = (theta.cos(), theta.sin());
        for j in 0..m {
            let r = p0[j].x * (1.0 - t_blend) + p1[j].x * t_blend;
            let z = p0[j].y * (1.0 - t_blend) + p1[j].y * t_blend;
            out_vertices[s * m + j] = Point3::new(r * ca, r * sa, z);
        }
    }

    let mut ti = 0usize;
    for s in 0..slices {
        let ns = (s + 1) % slices;
        for j in 0..m - 1 {
            let a = (s * m + j) as u32;
            let b = (ns * m + j) as u32;
            let c = (ns * m + j + 1) as u32;
            let d = (s * m + j + 1) as u32;
            out_triangles[ti] = [a, b, c];
            out_triangles[ti + 1] = [a, c, d];
            ti += 2;
        }
    }

    Ok((req_v, req_t))
}

/// Generate a closed frustum (truncated cone) or cone (`top_radius == 0.0`) along +Z.
pub fn frustum_into(
    base_radius: f64,
    top_radius: f64,
    height: f64,
    slices: usize,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), SweepError> {
    if !(base_radius > 0.0 && top_radius >= 0.0 && height > 0.0) || slices < 3 {
        return Err(SweepError::InvalidInput);
    }
    let req_v = slices * 2 + 2;
    let req_t = slices * 4;
    if out_vertices.len() < req_v || out_triangles.len() < req_t {
        return Err(SweepError::OutputTooSmall {
            required_vertices: req_v,
            required_triangles: req_t,
        });
    }

    for i in 0..slices {
        let a = core::f64::consts::TAU * (i as f64) / (slices as f64);
        let (ca, sa) = (a.cos(), a.sin());
        out_vertices[i] = Point3::new(base_radius * ca, base_radius * sa, 0.0);
        out_vertices[slices + i] = Point3::new(top_radius * ca, top_radius * sa, height);
    }
    let bot_c = (slices * 2) as u32;
    let top_c = bot_c + 1;
    out_vertices[bot_c as usize] = Point3::new(0.0, 0.0, 0.0);
    out_vertices[top_c as usize] = Point3::new(0.0, 0.0, height);

    let mut ti = 0usize;
    for i in 0..slices {
        let ni = (i + 1) % slices;
        let b0 = i as u32;
        let b1 = ni as u32;
        let t0 = (slices + i) as u32;
        let t1 = (slices + ni) as u32;
        out_triangles[ti] = [b0, b1, t1];
        out_triangles[ti + 1] = [b0, t1, t0];
        out_triangles[ti + 2] = [bot_c, b1, b0];
        out_triangles[ti + 3] = [top_c, t0, t1];
        ti += 4;
    }

    Ok((req_v, req_t))
}

/// Apply PlantGL `Taper` axial deformation in-place over $[z_{\min}, z_{\max}]$.
/// Tier-1 `HotZeroHeap`.
pub fn deform_taper_in_place(
    points: &mut [Point3],
    base_radius: f64,
    top_radius: f64,
) -> Result<(), SweepError> {
    if points.is_empty() || !(base_radius >= 0.0 && top_radius >= 0.0) {
        return Err(SweepError::InvalidInput);
    }
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;
    for p in points.iter() {
        z_min = z_min.min(p.z);
        z_max = z_max.max(p.z);
    }
    let dz = (z_max - z_min).max(1e-12);
    for p in points.iter_mut() {
        let u = ((p.z - z_min) / dz).clamp(0.0, 1.0);
        let factor = base_radius + u * (top_radius - base_radius);
        p.x *= factor;
        p.y *= factor;
    }
    Ok(())
}

/// Expand a bounded Iterated Function System (`IFS`) of 4×4 column-major affine
/// transforms to `depth` iterations on `base_points`.
pub fn expand_ifs_points(
    base_points: &[Point3],
    transforms: &[[[f64; 4]; 4]],
    depth: u8,
    out_points: &mut [Point3],
) -> Result<usize, SweepError> {
    const MAX_IFS_DEPTH: u8 = 6;
    if depth > MAX_IFS_DEPTH {
        return Err(SweepError::DepthExceeded {
            max_depth: MAX_IFS_DEPTH,
        });
    }
    if base_points.is_empty() || (depth > 0 && transforms.is_empty()) {
        return Err(SweepError::InvalidInput);
    }
    let k = transforms.len();
    let instances = if depth == 0 { 1 } else { k.pow(depth as u32) };
    let total = instances.saturating_mul(base_points.len());
    if out_points.len() < total {
        return Err(SweepError::OutputTooSmall {
            required_vertices: total,
            required_triangles: 0,
        });
    }

    for inst in 0..instances {
        let mut mat = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut code = inst;
        for _ in 0..depth {
            mat = super::authoring::mat_mul(&transforms[code % k], &mat);
            code /= k;
        }
        let base_off = inst * base_points.len();
        for (pi, &p) in base_points.iter().enumerate() {
            let nx = p.x * mat[0][0] + p.y * mat[1][0] + p.z * mat[2][0] + mat[3][0];
            let ny = p.x * mat[0][1] + p.y * mat[1][1] + p.z * mat[2][1] + mat[3][1];
            let nz = p.x * mat[0][2] + p.y * mat[1][2] + p.z * mat[2][2] + mat[3][2];
            out_points[base_off + pi] = Point3::new(nx, ny, nz);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::super::surface_mesh_processing::signed_volume;
    use super::*;

    #[test]
    fn bishop_frames_and_extrusion_produce_valid_geometry() {
        let path = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.2, 0.0, 2.0),
        ];
        let profile = [
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(-1.0, 0.0),
            Point2::new(0.0, -1.0),
        ];
        let mut frames = [AffineFrame3 {
            origin: Point3::default(),
            e0: Point3::default(),
            e1: Point3::default(),
            e2: Point3::default(),
        }; 3];
        let mut verts = vec![Point3::default(); 128];
        let mut tris = vec![[0u32; 3]; 256];
        let (nv, nt) = extrude_profile_along_path(
            &path,
            &profile,
            None,
            true,
            true,
            &mut frames,
            &mut verts,
            &mut tris,
        )
        .unwrap();
        for f in &frames {
            assert!((norm(f.e0) - 1.0).abs() < 1e-10);
            assert!(dot(f.e0, f.e2).abs() < 1e-10);
        }
        assert!(signed_volume(&verts[..nv], &tris[..nt]).unwrap() > 3.5);
        let (fv, ft) = frustum_into(1.0, 0.5, 2.0, 16, &mut verts, &mut tris).unwrap();
        assert!(signed_volume(&verts[..fv], &tris[..ft]).unwrap() > 3.0);
    }
}
