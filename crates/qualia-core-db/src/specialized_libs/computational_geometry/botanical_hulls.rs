//! P20.2 — Botanical envelope primitives: 6-point `AsymmetricHull` (Cescatti
//! 1997; Boudon et al.), orthogonal silhouette `ExtrudedHull`, and point-cloud
//! crown envelope fitting.
//!
//! All mesh generators are deterministic and write into caller-supplied slices.

use super::primitives::{Point2, Point3};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BotanicalHullError {
    InvalidParameters,
    EmptyPointSet,
    OutputTooSmall {
        required_vertices: usize,
        required_triangles: usize,
    },
}

/// Parameters of a 6-point asymmetric tree crown envelope (`AsymmetricHull`).
///
/// The crown is defined by a `bottom` point, a `top` point, four peripheral
/// radii along the $-X, +X, -Y, +Y$ directions with independent heights, and
/// super-ellipsoid shape exponents `bottom_shape` and `top_shape`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AsymmetricHullParams {
    pub neg_x_radius: f64,
    pub pos_x_radius: f64,
    pub neg_y_radius: f64,
    pub pos_y_radius: f64,
    pub neg_x_height: f64,
    pub pos_x_height: f64,
    pub neg_y_height: f64,
    pub pos_y_height: f64,
    pub bottom: Point3,
    pub top: Point3,
    pub bottom_shape: f64,
    pub top_shape: f64,
    pub slices: usize,
    pub stacks: usize,
}

impl Default for AsymmetricHullParams {
    fn default() -> Self {
        Self {
            neg_x_radius: 0.5,
            pos_x_radius: 0.5,
            neg_y_radius: 0.5,
            pos_y_radius: 0.5,
            neg_x_height: 0.5,
            pos_x_height: 0.5,
            neg_y_height: 0.5,
            pos_y_height: 0.5,
            bottom: Point3::new(0.0, 0.0, 0.0),
            top: Point3::new(0.0, 0.0, 1.0),
            bottom_shape: 2.0,
            top_shape: 2.0,
            slices: 16,
            stacks: 8,
        }
    }
}

/// Required `(vertices, triangles)` buffer capacity for [`asymmetric_hull_into`].
#[inline]
pub const fn asymmetric_hull_capacity(slices: usize, stacks: usize) -> (usize, usize) {
    let ring_count = 2 * stacks - 1;
    let vertices = 2 + ring_count * slices;
    let triangles = 2 * slices + (ring_count - 1) * slices * 2;
    (vertices, triangles)
}

/// Generate a closed 2-manifold triangle mesh for a 6-point `AsymmetricHull`.
///
/// For each azimuthal angle $\theta \in [0, 2\pi)$, the peripheral crown radius
/// $R(\theta)$ and peripheral height $H(\theta)$ are blended across the four
/// quadrants. Lower stacks ($v \in (0, 1)$) follow the super-ellipsoidal law
/// $(r/R)^{s_b} + ((H - z)/(H - z_{\text{bot}}))^{s_b} = 1$, and upper stacks
/// follow $(r/R)^{s_t} + ((z - H)/(z_{\text{top}} - H))^{s_t} = 1$.
pub fn asymmetric_hull_into(
    p: &AsymmetricHullParams,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), BotanicalHullError> {
    if p.slices < 4
        || p.stacks < 2
        || p.neg_x_radius <= 0.0
        || p.pos_x_radius <= 0.0
        || p.neg_y_radius <= 0.0
        || p.pos_y_radius <= 0.0
        || p.bottom_shape <= 0.0
        || p.top_shape <= 0.0
        || !(p.top.z > p.bottom.z)
    {
        return Err(BotanicalHullError::InvalidParameters);
    }

    let (req_v, req_t) = asymmetric_hull_capacity(p.slices, p.stacks);
    if out_vertices.len() < req_v || out_triangles.len() < req_t {
        return Err(BotanicalHullError::OutputTooSmall {
            required_vertices: req_v,
            required_triangles: req_t,
        });
    }

    let ring_count = 2 * p.stacks - 1;
    let bot_idx = 0u32;
    let top_idx = (1 + ring_count * p.slices) as u32;
    out_vertices[bot_idx as usize] = p.bottom;
    out_vertices[top_idx as usize] = p.top;

    for sl in 0..p.slices {
        let theta = core::f64::consts::TAU * (sl as f64) / (p.slices as f64);
        let (ct, st) = (theta.cos(), theta.sin());
        let rx = if ct >= 0.0 {
            p.pos_x_radius
        } else {
            p.neg_x_radius
        };
        let ry = if st >= 0.0 {
            p.pos_y_radius
        } else {
            p.neg_y_radius
        };
        let hx = if ct >= 0.0 {
            p.pos_x_height
        } else {
            p.neg_x_height
        };
        let hy = if st >= 0.0 {
            p.pos_y_height
        } else {
            p.neg_y_height
        };

        let ct2 = ct * ct;
        let st2 = st * st;
        let r_periph = (rx * rx * ct2 + ry * ry * st2).sqrt();
        let h_periph = (hx * ct2 + hy * st2).clamp(p.bottom.z, p.top.z);

        for r_idx in 0..ring_count {
            let (radial_frac, z, pole_blend, pole_xy) = if r_idx < p.stacks - 1 {
                // Bottom hemisphere: v in (0, 1) from bottom pole to equator
                let v = (r_idx + 1) as f64 / (p.stacks as f64);
                let z_rel = (1.0 - v).powf(1.0 / p.bottom_shape);
                let r_rel = (1.0 - z_rel.powf(p.bottom_shape))
                    .max(0.0)
                    .powf(1.0 / p.bottom_shape);
                let z = h_periph - z_rel * (h_periph - p.bottom.z);
                (r_rel, z, z_rel, (p.bottom.x, p.bottom.y))
            } else if r_idx == p.stacks - 1 {
                (1.0, h_periph, 0.0, (0.0, 0.0))
            } else {
                // Top hemisphere: v in (0, 1) from equator to top pole
                let step = r_idx - (p.stacks - 1);
                let v = (step as f64) / (p.stacks as f64);
                let z_rel = v.powf(1.0 / p.top_shape);
                let r_rel = (1.0 - z_rel.powf(p.top_shape))
                    .max(0.0)
                    .powf(1.0 / p.top_shape);
                let z = h_periph + z_rel * (p.top.z - h_periph);
                (r_rel, z, z_rel, (p.top.x, p.top.y))
            };

            let x = pole_blend * pole_xy.0 + radial_frac * r_periph * ct;
            let y = pole_blend * pole_xy.1 + radial_frac * r_periph * st;
            out_vertices[1 + r_idx * p.slices + sl] = Point3::new(x, y, z);
        }
    }

    let mut ti = 0usize;
    // Bottom pole fan (outward normal pointing -Z)
    for sl in 0..p.slices {
        let nsl = (sl + 1) % p.slices;
        out_triangles[ti] = [bot_idx, (1 + nsl) as u32, (1 + sl) as u32];
        ti += 1;
    }
    // Quad strips between consecutive rings
    for r_idx in 0..ring_count - 1 {
        let r0 = (1 + r_idx * p.slices) as u32;
        let r1 = (1 + (r_idx + 1) * p.slices) as u32;
        for sl in 0..p.slices {
            let nsl = (sl + 1) % p.slices;
            let a = r0 + sl as u32;
            let b = r0 + nsl as u32;
            let c = r1 + nsl as u32;
            let d = r1 + sl as u32;
            out_triangles[ti] = [a, b, c];
            out_triangles[ti + 1] = [a, c, d];
            ti += 2;
        }
    }
    // Top pole fan (outward normal pointing +Z)
    let last_ring = (1 + (ring_count - 1) * p.slices) as u32;
    for sl in 0..p.slices {
        let nsl = (sl + 1) % p.slices;
        out_triangles[ti] = [top_idx, last_ring + sl as u32, last_ring + nsl as u32];
        ti += 1;
    }

    Ok((req_v, req_t))
}

/// Fit an [`AsymmetricHullParams`] crown envelope directly to a 3-D point cloud
/// (matching PlantGL `Fit::asymmetricHull`).
///
/// Tier-1 `HotZeroHeap`: single streaming pass over `points`.
pub fn fit_asymmetric_hull(
    points: &[Point3],
    slices: usize,
    stacks: usize,
) -> Result<AsymmetricHullParams, BotanicalHullError> {
    if points.is_empty() || slices < 4 || stacks < 2 {
        return Err(BotanicalHullError::EmptyPointSet);
    }

    let mut min_z_pt = points[0];
    let mut max_z_pt = points[0];
    let mut min_x_pt = points[0];
    let mut max_x_pt = points[0];
    let mut min_y_pt = points[0];
    let mut max_y_pt = points[0];

    for &pt in points.iter().skip(1) {
        if !pt.x.is_finite() || !pt.y.is_finite() || !pt.z.is_finite() {
            return Err(BotanicalHullError::InvalidParameters);
        }
        if pt.z < min_z_pt.z {
            min_z_pt = pt;
        }
        if pt.z > max_z_pt.z {
            max_z_pt = pt;
        }
        if pt.x < min_x_pt.x {
            min_x_pt = pt;
        }
        if pt.x > max_x_pt.x {
            max_x_pt = pt;
        }
        if pt.y < min_y_pt.y {
            min_y_pt = pt;
        }
        if pt.y > max_y_pt.y {
            max_y_pt = pt;
        }
    }

    let cx = 0.5 * (min_x_pt.x + max_x_pt.x);
    let cy = 0.5 * (min_y_pt.y + max_y_pt.y);
    let dz = max_z_pt.z - min_z_pt.z;
    if dz <= 1e-9 {
        return Err(BotanicalHullError::InvalidParameters);
    }

    Ok(AsymmetricHullParams {
        neg_x_radius: (cx - min_x_pt.x).max(1e-3),
        pos_x_radius: (max_x_pt.x - cx).max(1e-3),
        neg_y_radius: (cy - min_y_pt.y).max(1e-3),
        pos_y_radius: (max_y_pt.y - cy).max(1e-3),
        neg_x_height: min_x_pt.z,
        pos_x_height: max_x_pt.z,
        neg_y_height: min_y_pt.z,
        pos_y_height: max_y_pt.z,
        bottom: Point3::new(min_z_pt.x - cx, min_z_pt.y - cy, min_z_pt.z),
        top: Point3::new(max_z_pt.x - cx, max_z_pt.y - cy, max_z_pt.z),
        bottom_shape: 2.0,
        top_shape: 2.0,
        slices,
        stacks,
    })
}

/// Construct a closed 3-D `ExtrudedHull` from a closed `vertical` $(x, z)$
/// silhouette and a closed `horizontal` $(x, y)$ cross-section profile.
///
/// At each height slice $k \in [0, \text{stacks}-1]$, the vertical profile's
/// $x$-width modulates the horizontal profile ring, capped at both poles.
pub fn extruded_hull_into(
    vertical_profile: &[Point2],
    horizontal_profile: &[Point2],
    stacks: usize,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), BotanicalHullError> {
    let nv = vertical_profile.len();
    let nh = horizontal_profile.len();
    if nv < 3 || nh < 3 || stacks < 2 {
        return Err(BotanicalHullError::InvalidParameters);
    }

    let req_v = 2 + stacks * nh;
    let req_t = 2 * nh + (stacks - 1) * nh * 2;
    if out_vertices.len() < req_v || out_triangles.len() < req_t {
        return Err(BotanicalHullError::OutputTooSmall {
            required_vertices: req_v,
            required_triangles: req_t,
        });
    }

    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    for p in vertical_profile {
        z_min = z_min.min(p.y);
        z_max = z_max.max(p.y);
        x_min = x_min.min(p.x);
        x_max = x_max.max(p.x);
    }
    let half_w = (0.5 * (x_max - x_min)).max(1e-9);
    let mid_x = 0.5 * (x_max + x_min);
    let dz = z_max - z_min;
    if dz <= 1e-9 {
        return Err(BotanicalHullError::InvalidParameters);
    }

    out_vertices[0] = Point3::new(0.0, 0.0, z_min);
    let top_idx = (1 + stacks * nh) as u32;
    out_vertices[top_idx as usize] = Point3::new(0.0, 0.0, z_max);

    for st in 0..stacks {
        let u = (st + 1) as f64 / ((stacks + 1) as f64);
        let z = z_min + u * dz;
        // Estimate horizontal span of vertical_profile at height z, fallback to sin(pi*u).
        let span = vertical_profile_half_width(vertical_profile, z, mid_x)
            .unwrap_or_else(|| half_w * (core::f64::consts::PI * u).sin())
            / half_w;
        let scale_factor = span.max(0.05);
        for j in 0..nh {
            let hp = horizontal_profile[j];
            out_vertices[1 + st * nh + j] =
                Point3::new(hp.x * scale_factor, hp.y * scale_factor, z);
        }
    }

    let mut ti = 0usize;
    for j in 0..nh {
        let nj = (j + 1) % nh;
        out_triangles[ti] = [0, (1 + nj) as u32, (1 + j) as u32];
        ti += 1;
    }
    for st in 0..stacks - 1 {
        let r0 = (1 + st * nh) as u32;
        let r1 = (1 + (st + 1) * nh) as u32;
        for j in 0..nh {
            let nj = (j + 1) % nh;
            out_triangles[ti] = [r0 + j as u32, r0 + nj as u32, r1 + nj as u32];
            out_triangles[ti + 1] = [r0 + j as u32, r1 + nj as u32, r1 + j as u32];
            ti += 2;
        }
    }
    let last_r = (1 + (stacks - 1) * nh) as u32;
    for j in 0..nh {
        let nj = (j + 1) % nh;
        out_triangles[ti] = [top_idx, last_r + j as u32, last_r + nj as u32];
        ti += 1;
    }

    Ok((req_v, req_t))
}

fn vertical_profile_half_width(profile: &[Point2], z: f64, mid_x: f64) -> Option<f64> {
    let mut max_dx = 0.0f64;
    let mut found = false;
    for i in 0..profile.len() {
        let a = profile[i];
        let b = profile[(i + 1) % profile.len()];
        if (a.y <= z && b.y >= z) || (b.y <= z && a.y >= z) {
            let dy = b.y - a.y;
            let x = if dy.abs() < 1e-12 {
                a.x
            } else {
                a.x + (z - a.y) / dy * (b.x - a.x)
            };
            max_dx = max_dx.max((x - mid_x).abs());
            found = true;
        }
    }
    if found {
        Some(max_dx)
    } else {
        None
    }
}

/// Generate a PlantGL-class paraboloid solid envelope $r(z) = R \cdot (1 - z/H)^{\text{shape}}$
/// along $[0, H]$.
pub fn paraboloid_into(
    radius: f64,
    height: f64,
    shape: f64,
    slices: usize,
    stacks: usize,
    solid: bool,
    out_vertices: &mut [Point3],
    out_triangles: &mut [[u32; 3]],
) -> Result<(usize, usize), BotanicalHullError> {
    if !(radius > 0.0 && height > 0.0 && shape > 0.0) || slices < 3 || stacks < 2 {
        return Err(BotanicalHullError::InvalidParameters);
    }
    let cap_v = if solid { 1 } else { 0 };
    let req_v = stacks * slices + 1 + cap_v;
    let cap_t = if solid { slices } else { 0 };
    let req_t = (stacks - 1) * slices * 2 + slices + cap_t;
    if out_vertices.len() < req_v || out_triangles.len() < req_t {
        return Err(BotanicalHullError::OutputTooSmall {
            required_vertices: req_v,
            required_triangles: req_t,
        });
    }

    for st in 0..stacks {
        let u = (st as f64) / (stacks as f64);
        let z = u * height;
        let r = radius * (1.0 - u).powf(shape);
        for sl in 0..slices {
            let a = core::f64::consts::TAU * (sl as f64) / (slices as f64);
            out_vertices[st * slices + sl] = Point3::new(r * a.cos(), r * a.sin(), z);
        }
    }
    let apex = (stacks * slices) as u32;
    out_vertices[apex as usize] = Point3::new(0.0, 0.0, height);

    let mut ti = 0usize;
    for st in 0..stacks - 1 {
        for sl in 0..slices {
            let nsl = (sl + 1) % slices;
            let a = (st * slices + sl) as u32;
            let b = (st * slices + nsl) as u32;
            let c = ((st + 1) * slices + nsl) as u32;
            let d = ((st + 1) * slices + sl) as u32;
            out_triangles[ti] = [a, b, c];
            out_triangles[ti + 1] = [a, c, d];
            ti += 2;
        }
    }
    let last_ring = (stacks - 1) * slices;
    for sl in 0..slices {
        let nsl = (sl + 1) % slices;
        out_triangles[ti] = [(last_ring + sl) as u32, (last_ring + nsl) as u32, apex];
        ti += 1;
    }
    if solid {
        let base_c = apex + 1;
        out_vertices[base_c as usize] = Point3::new(0.0, 0.0, 0.0);
        for sl in 0..slices {
            let nsl = (sl + 1) % slices;
            out_triangles[ti] = [base_c, nsl as u32, sl as u32];
            ti += 1;
        }
    }

    Ok((req_v, req_t))
}

#[cfg(test)]
mod tests {
    use super::super::surface_mesh_processing::signed_volume;
    use super::*;

    #[test]
    fn asymmetric_hull_generates_closed_positive_volume_crown() {
        let params = AsymmetricHullParams {
            neg_x_radius: 1.2,
            pos_x_radius: 1.8,
            neg_y_radius: 1.0,
            pos_y_radius: 1.5,
            neg_x_height: 1.2,
            pos_x_height: 1.6,
            neg_y_height: 1.3,
            pos_y_height: 1.4,
            bottom: Point3::new(0.0, 0.0, 0.0),
            top: Point3::new(0.1, -0.1, 3.0),
            bottom_shape: 2.0,
            top_shape: 1.8,
            slices: 16,
            stacks: 6,
        };
        let (cap_v, cap_t) = asymmetric_hull_capacity(params.slices, params.stacks);
        let mut verts = vec![Point3::default(); cap_v];
        let mut tris = vec![[0u32; 3]; cap_t];
        let (nv, nt) = asymmetric_hull_into(&params, &mut verts, &mut tris).unwrap();
        assert_eq!((nv, nt), (cap_v, cap_t));
        let vol = signed_volume(&verts[..nv], &tris[..nt]).unwrap();
        assert!(vol > 5.0, "expected positive crown volume, got {vol}");

        let fitted = fit_asymmetric_hull(&verts[..nv], 12, 4).unwrap();
        assert!((fitted.top.z - 3.0).abs() < 1e-9);
    }

    #[test]
    fn extruded_hull_builds_closed_envelope() {
        let vert = [
            Point2::new(-1.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.5, 1.0),
            Point2::new(0.0, 2.0),
            Point2::new(-1.5, 1.0),
        ];
        let horiz = [
            Point2::new(1.0, 0.0),
            Point2::new(0.0, 1.0),
            Point2::new(-1.0, 0.0),
            Point2::new(0.0, -1.0),
        ];
        let mut verts = vec![Point3::default(); 64];
        let mut tris = vec![[0u32; 3]; 128];
        let (nv, nt) = extruded_hull_into(&vert, &horiz, 5, &mut verts, &mut tris).unwrap();
        let vol = signed_volume(&verts[..nv], &tris[..nt]).unwrap();
        assert!(vol > 1.0, "extruded hull volume = {vol}");
    }
}
