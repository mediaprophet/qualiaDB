//! P21.9 — 3-D Medial Axis Transform via Voronoi poles
//! (Amenta & Bern 1998; Amenta, Choi & Kolluri 2001 power-shape pipeline).
//!
//! For each surface sample `p` with outward normal `n`, the two poles are the
//! farthest Voronoi vertices (circumcenters of incident Delaunay tetrahedra)
//! on each side of `n`. Interior poles trace the medial axis; `|pole − p|` is
//! the local feature size / medial radius — enough to seed rigging
//! hierarchies and volumetric thickness fields.
//!
//! `ColdBounded`: Delaunay assembly allocates internally.

use super::delaunay_3::{delaunay_tetrahedralization_3, Delaunay3Error};
use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MedialSample {
    /// Pole position on the medial surface.
    pub pole: Point3,
    /// Medial radius — distance to the surface sample.
    pub radius: f64,
    /// True for the "outer" pole (along +n), false for the interior pole.
    pub outer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MedialAxisError {
    TooFewSamples { got: usize },
    DimensionMismatch,
    OutputTooSmall { required: usize },
    DegenerateTetrahedra,
}

impl From<Delaunay3Error> for MedialAxisError {
    fn from(e: Delaunay3Error) -> Self {
        match e {
            Delaunay3Error::TooFewPoints { got } => MedialAxisError::TooFewSamples { got },
            Delaunay3Error::OutputTooSmall { required, .. } => {
                MedialAxisError::OutputTooSmall { required }
            }
            Delaunay3Error::CoplanarInput | Delaunay3Error::NonFiniteCoordinate { .. } => {
                MedialAxisError::DegenerateTetrahedra
            }
        }
    }
}

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}
#[inline]
fn dot(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}
#[inline]
fn dotv(a: Point3, b: Point3) -> f64 {
    a.x * b.x + a.y * b.y + a.z * b.z
}

/// Circumcenter of a tetrahedron — solves `2(b−a)·c = |b|²−|a|²` etc.
/// Returns `None` for degenerate tets. Tier-1 `HotZeroHeap`.
pub fn tet_circumcenter(a: Point3, b: Point3, c: Point3, d: Point3) -> Option<Point3> {
    let (ba, ca, da) = (sub(b, a), sub(c, a), sub(d, a));
    let cross_cd = Point3::new(
        ca.y * da.z - ca.z * da.y,
        ca.z * da.x - ca.x * da.z,
        ca.x * da.y - ca.y * da.x,
    );
    let denom = 2.0 * dot(ba, cross_cd);
    if denom.abs() < 1e-14 {
        return None;
    }
    let (a2, b2, c2, d2) = (dot(a, a), dot(b, b), dot(c, c), dot(d, d));
    let cross_da = Point3::new(
        da.y * ba.z - da.z * ba.y,
        da.z * ba.x - da.x * ba.z,
        da.x * ba.y - da.y * ba.x,
    );
    let cross_bc = Point3::new(
        ba.y * ca.z - ba.z * ca.y,
        ba.z * ca.x - ba.x * ca.z,
        ba.x * ca.y - ba.y * ca.x,
    );
    Some(Point3::new(
        ((b2 - a2) * cross_cd.x + (c2 - a2) * cross_da.x + (d2 - a2) * cross_bc.x) / denom,
        ((b2 - a2) * cross_cd.y + (c2 - a2) * cross_da.y + (d2 - a2) * cross_bc.y) / denom,
        ((b2 - a2) * cross_cd.z + (c2 - a2) * cross_da.z + (d2 - a2) * cross_bc.z) / denom,
    ))
}

/// Extract medial-axis samples (interior poles) from a surface point cloud
/// with outward normals.
///
/// For every sample, incident-tetra circumcenters are scanned; the farthest
/// center whose offset aligns with `+n` is the outer pole and the farthest on
/// the `−n` side is the interior pole. Only interior poles are emitted —
/// the medial-axis trace.
///
/// `capacity` bounds the Delaunay tetra buffer. Returns sample count.
/// `ColdBounded`.
pub fn extract_medial_axis_3d(
    points: &[Point3],
    normals: &[Point3],
    out: &mut [MedialSample],
    tet_scratch: &mut [[u32; 4]],
) -> Result<usize, MedialAxisError> {
    let n = points.len();
    if n < 4 {
        return Err(MedialAxisError::TooFewSamples { got: n });
    }
    if normals.len() < n {
        return Err(MedialAxisError::DimensionMismatch);
    }
    let tet_n = delaunay_tetrahedralization_3(points, tet_scratch)?;

    // Collect circumcenters per tet.
    let mut centers: Vec<Point3> = Vec::with_capacity(tet_n);
    for t in &tet_scratch[..tet_n] {
        let c = tet_circumcenter(
            points[t[0] as usize],
            points[t[1] as usize],
            points[t[2] as usize],
            points[t[3] as usize],
        );
        centers.push(c.unwrap_or(Point3::new(f64::NAN, f64::NAN, f64::NAN)));
    }

    let mut out_n = 0usize;
    for i in 0..n {
        let p = points[i];
        let nrm = normals[i];
        let mut best_d2 = -1.0f64;
        let mut best = Point3::default();
        for t in 0..tet_n {
            let tet = &tet_scratch[t];
            if !(tet[0] as usize == i
                || tet[1] as usize == i
                || tet[2] as usize == i
                || tet[3] as usize == i)
            {
                continue;
            }
            let c = centers[t];
            if !c.x.is_finite() {
                continue;
            }
            let off = sub(c, p);
            if dotv(off, nrm) >= 0.0 {
                continue; // exterior pole — keep only interior
            }
            let d2 = dot(off, off);
            if d2 > best_d2 {
                best_d2 = d2;
                best = c;
            }
        }
        if best_d2 >= 0.0 {
            if out_n >= out.len() {
                return Err(MedialAxisError::OutputTooSmall {
                    required: out_n + 1,
                });
            }
            out[out_n] = MedialSample {
                pole: best,
                radius: best_d2.sqrt(),
                outer: false,
            };
            out_n += 1;
        }
    }
    Ok(out_n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circumcenter_of_regular_tet() {
        // Unit-ish tet: equilateral base at z=0, apex above centroid.
        let h = (2.0f64 / 3.0).sqrt();
        let a = Point3::new(1.0, 0.0, 0.0);
        let b = Point3::new(-0.5, 3.0f64.sqrt() * 0.5, 0.0);
        let c = Point3::new(-0.5, -3.0f64.sqrt() * 0.5, 0.0);
        let d = Point3::new(0.0, 0.0, h);
        let cc = tet_circumcenter(a, b, c, d).unwrap();
        // All four vertices equidistant from the center.
        let da = dot(sub(cc, a), sub(cc, a));
        let db = dot(sub(cc, b), sub(cc, b));
        let dc = dot(sub(cc, c), sub(cc, c));
        let dd = dot(sub(cc, d), sub(cc, d));
        assert!((da - db).abs() < 1e-9 && (da - dc).abs() < 1e-9 && (da - dd).abs() < 1e-9);
    }

    #[test]
    fn octahedron_mat_converges_to_center() {
        // 6-vertex octahedron: medial axis should place interior poles near
        // the origin with radius ≈ inradius.
        let pts = [
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, 0.0, -1.0),
        ];
        let nrms = [
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, -1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(0.0, 0.0, -1.0),
        ];
        let mut out = [MedialSample {
            pole: Point3::default(),
            radius: 0.0,
            outer: false,
        }; 8];
        let mut tets = [[0u32; 4]; 128];
        let n = extract_medial_axis_3d(&pts, &nrms, &mut out, &mut tets).unwrap();
        assert!(n >= 3, "expected interior poles, got {n}");
        for s in &out[..n] {
            let d = dot(s.pole, s.pole).sqrt();
            assert!(d < 0.6, "pole {s:?} should sit near center");
        }
    }
}
