//! Caller-buffered triangle-mesh intersection with a plane.

use super::primitives::Point3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshPlaneSliceError {
    InvalidPlane,
    InvalidTriangle { index: usize },
    OutputTooSmall { required: usize },
}

#[inline]
fn signed_distance(p: Point3, origin: Point3, normal: Point3) -> f64 {
    (p.x - origin.x) * normal.x + (p.y - origin.y) * normal.y + (p.z - origin.z) * normal.z
}

#[inline]
fn interpolate(a: Point3, b: Point3, da: f64, db: f64) -> Point3 {
    let t = da / (da - db);
    Point3::new(
        a.x + t * (b.x - a.x),
        a.y + t * (b.y - a.y),
        a.z + t * (b.z - a.z),
    )
}

fn section_of_triangle(
    vertices: &[Point3],
    tri: [u32; 3],
    origin: Point3,
    normal: Point3,
) -> Option<[Point3; 2]> {
    let p = [
        vertices[tri[0] as usize],
        vertices[tri[1] as usize],
        vertices[tri[2] as usize],
    ];
    let d = p.map(|v| signed_distance(v, origin, normal));
    // A coplanar face has no unique section segment. It belongs to the
    // separate coplanar-region boundary problem.
    if d.iter().all(|&x| x == 0.0) {
        return None;
    }
    let mut hits = [Point3::default(); 2];
    let mut count = 0usize;
    for i in 0..3 {
        let j = (i + 1) % 3;
        let hit = if d[i] == 0.0 {
            Some(p[i])
        } else if (d[i] < 0.0 && d[j] > 0.0) || (d[i] > 0.0 && d[j] < 0.0) {
            Some(interpolate(p[i], p[j], d[i], d[j]))
        } else {
            None
        };
        if let Some(point) = hit {
            if count == 0 || hits[0] != point {
                if count < 2 {
                    hits[count] = point;
                }
                count += 1;
            }
        }
    }
    (count == 2 && hits[0] != hits[1]).then_some(hits)
}

/// Emit one independent segment per intersected triangle in triangle order.
/// Coplanar faces and single-vertex touches emit no segment. Shared segments
/// are intentionally retained so callers can apply their own topology policy.
pub fn slice_mesh_with_plane(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    origin: Point3,
    normal: Point3,
    out: &mut [[Point3; 2]],
) -> Result<usize, MeshPlaneSliceError> {
    let norm2 = normal.x * normal.x + normal.y * normal.y + normal.z * normal.z;
    if !norm2.is_finite()
        || norm2 <= 0.0
        || ![origin.x, origin.y, origin.z].iter().all(|x| x.is_finite())
        || vertices
            .iter()
            .any(|p| ![p.x, p.y, p.z].iter().all(|x| x.is_finite()))
    {
        return Err(MeshPlaneSliceError::InvalidPlane);
    }
    let mut required = 0usize;
    for (index, &tri) in triangles.iter().enumerate() {
        if tri.iter().any(|&i| i as usize >= vertices.len()) {
            return Err(MeshPlaneSliceError::InvalidTriangle { index });
        }
        required += section_of_triangle(vertices, tri, origin, normal).is_some() as usize;
    }
    if out.len() < required {
        return Err(MeshPlaneSliceError::OutputTooSmall { required });
    }
    let mut written = 0;
    for &tri in triangles {
        if let Some(segment) = section_of_triangle(vertices, tri, origin, normal) {
            out[written] = segment;
            written += 1;
        }
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triangle_crossing_plane_has_expected_segment() {
        let v = [
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut out = [[Point3::default(); 2]; 1];
        let n = slice_mesh_with_plane(
            &v,
            &[[0, 1, 2]],
            Point3::default(),
            Point3::new(1.0, 0.0, 0.0),
            &mut out,
        )
        .unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0][0].x, 0.0);
        assert_eq!(out[0][1].x, 0.0);
    }

    #[test]
    fn short_output_is_unmodified() {
        let v = [
            Point3::new(-1.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let mut out = [];
        assert_eq!(
            slice_mesh_with_plane(
                &v,
                &[[0, 1, 2]],
                Point3::default(),
                Point3::new(1.0, 0.0, 0.0),
                &mut out
            ),
            Err(MeshPlaneSliceError::OutputTooSmall { required: 1 })
        );
    }
}
