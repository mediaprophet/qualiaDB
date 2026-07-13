//! P13.1 - Mesh quality metrics and size/anisotropy fields.
//!
//! Finite-element-grade element quality measurement for 2-D triangles and
//! 3-D tetrahedra, plus the size/anisotropy *fields* that a mesher targets.
//!
//! ## Element quality
//!
//! For **triangles** (`tri_*`): min/max interior angle, edge ratio, aspect
//! ratio (circumradius over inradius), radius-edge ratio (circumradius over
//! shortest edge), area, and an orientation/inversion flag (signed area).
//!
//! For **tetrahedra** (`tet_*`): min/max dihedral angle, radius-edge ratio,
//! scaled Jacobian (normalised solid-corner determinant, range `[-1, 1]`,
//! regular tet ~ `sqrt(2)/2`, inverted < 0), volume, and an orientation flag.
//!
//! Inverted / degenerate cells **fail closed**: a negative signed area or
//! scaled Jacobian is reported as `valid: false`; a zero signed measure is
//! reported as degenerate. Quality functions never return a plausible number
//! for an inverted element - they return the signed quantity and a validity
//! flag so the caller can reject.
//!
//! ## Size / anisotropy fields
//!
//! A [`SizeField`] maps a position to a desired isotropic edge length. An
//! [`AnisotropyField`] maps a position to a 3x3 symmetric **metric tensor**
//! `M`; the metric length of an edge `e` is `sqrt(e^T M e)`, and a mesher aims
//! for unit metric length. Field conformance
//! ([`check_field_conformance`]) measures every element edge in the metric
//! and reports the min/max ratio to the target (1.0), so a mesh can be
//! accepted or rejected against a declared tolerance.
//!
//! ## Determinism
//!
//! All metrics are pure functions of the input coordinates in fixed order
//! (vertex order within the element). Identical input -> bit-identical output.
//! Field interpolation is barycentric with deterministic tie-breaking
//! (lowest-index triangle wins on equal barycentric weight).

use super::primitives::Point3;

// ---------------------------------------------------------------------------
//  Errors
// ---------------------------------------------------------------------------

/// Mesh-quality measurement error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshQualityError {
    /// A vertex index was out of range.
    IndexOutOfBounds { element: usize, vertex: u32 },
    /// A coordinate was non-finite (NaN / +/-inf).
    NonFiniteCoordinate { index: u32 },
    /// A metric tensor was not positive-definite (a non-positive eigenvalue
    /// was detected via a non-positive metric length on a probe axis).
    IndefiniteMetric { vertex: u32 },
    /// Background mesh for field interpolation was empty.
    EmptyBackground,
    /// A query point fell outside every background triangle (no barycentric
    /// containment) and extrapolation was not requested.
    QueryOutsideBackground,
}

impl core::fmt::Display for MeshQualityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IndexOutOfBounds { element, vertex } => write!(
                f,
                "mesh_quality: element {element} references out-of-bounds vertex {vertex}"
            ),
            Self::NonFiniteCoordinate { index } => {
                write!(f, "mesh_quality: non-finite coordinate at vertex {index}")
            }
            Self::IndefiniteMetric { vertex } => {
                write!(
                    f,
                    "mesh_quality: indefinite metric tensor at vertex {vertex}"
                )
            }
            Self::EmptyBackground => write!(f, "mesh_quality: empty background mesh"),
            Self::QueryOutsideBackground => {
                write!(f, "mesh_quality: query point outside background mesh")
            }
        }
    }
}

impl std::error::Error for MeshQualityError {}

// ---------------------------------------------------------------------------
//  Small vector helpers (no allocations, operate on Point3)
// ---------------------------------------------------------------------------

#[inline]
fn sub(a: Point3, b: Point3) -> Point3 {
    Point3::new(a.x - b.x, a.y - b.y, a.z - b.z)
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
fn clamp_finite(x: f64) -> bool {
    x.is_finite()
}

/// Validate and fetch the three corners of a triangle.
fn fetch_tri(
    vertices: &[Point3],
    tri: &[u32; 3],
    element: usize,
) -> Result<[Point3; 3], MeshQualityError> {
    let mut out = [Point3::new(0.0, 0.0, 0.0); 3];
    for (i, &vi) in tri.iter().enumerate() {
        let v = *vertices
            .get(vi as usize)
            .ok_or(MeshQualityError::IndexOutOfBounds {
                element,
                vertex: vi,
            })?;
        if !clamp_finite(v.x) || !clamp_finite(v.y) || !clamp_finite(v.z) {
            return Err(MeshQualityError::NonFiniteCoordinate { index: vi });
        }
        out[i] = v;
    }
    Ok(out)
}

/// Validate and fetch the four corners of a tet.
fn fetch_tet(
    vertices: &[Point3],
    tet: &[u32; 4],
    element: usize,
) -> Result<[Point3; 4], MeshQualityError> {
    let mut out = [Point3::new(0.0, 0.0, 0.0); 4];
    for (i, &vi) in tet.iter().enumerate() {
        let v = *vertices
            .get(vi as usize)
            .ok_or(MeshQualityError::IndexOutOfBounds {
                element,
                vertex: vi,
            })?;
        if !clamp_finite(v.x) || !clamp_finite(v.y) || !clamp_finite(v.z) {
            return Err(MeshQualityError::NonFiniteCoordinate { index: vi });
        }
        out[i] = v;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
//  Triangle quality
// ---------------------------------------------------------------------------

/// Per-triangle quality report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriQuality {
    /// Minimum interior angle (radians). Degenerate (collinear) -> 0.
    pub min_angle: f64,
    /// Maximum interior angle (radians). Degenerate -> pi.
    pub max_angle: f64,
    /// Edge ratio = longest_edge / shortest_edge (>= 1; 1 = equilateral).
    pub edge_ratio: f64,
    /// Aspect ratio = circumradius / (2 * inradius) (>= 1; 1 = equilateral).
    pub aspect_ratio: f64,
    /// Radius-edge ratio = circumradius / shortest_edge (>= 1/sqrt(3); large = bad).
    pub radius_edge: f64,
    /// Unsigned area (>= 0).
    pub area: f64,
    /// For a 3-D triangle the sign requires a reference normal, so this field
    /// carries the **unsigned** area (equal to `area`). Use
    /// [`tri_signed_area_2d`] to detect inversion in planar (`z = 0`) meshes.
    pub signed_area: f64,
    /// `true` iff the triangle is non-degenerate (`area > 0` and all edges
    /// non-zero). Inversion in planar meshes is detected separately via
    /// [`tri_signed_area_2d`].
    pub valid: bool,
}

impl TriQuality {
    /// `true` if the triangle is degenerate (zero area).
    #[inline]
    pub fn is_degenerate(&self) -> bool {
        self.area == 0.0
    }
}

/// Interior angle at vertex `b` of the corner triple `(a, b, c)` (radians).
///
/// Returns `0` for a degenerate (collinear) corner and `pi` if `b` lies between
/// `a` and `c`. Robust against zero-length edges (returns `0`).
#[inline]
fn interior_angle(a: Point3, b: Point3, c: Point3) -> f64 {
    let u = sub(a, b);
    let v = sub(c, b);
    let nu = norm(u);
    let nv = norm(v);
    if nu == 0.0 || nv == 0.0 {
        return 0.0;
    }
    let cos = (dot(u, v) / (nu * nv)).clamp(-1.0, 1.0);
    cos.acos()
}

/// Circumradius of a triangle from its three side lengths and area.
///
/// `R = (a * b * c) / (4 * area)`. Returns `+inf` for a degenerate triangle.
#[inline]
fn circumradius_tri(a: f64, b: f64, c: f64, area: f64) -> f64 {
    if area <= 0.0 {
        f64::INFINITY
    } else {
        (a * b * c) / (4.0 * area)
    }
}

/// Inradius of a triangle from its three side lengths and area.
///
/// `r = area / s` where `s = (a+b+c)/2`. Returns `0` for a degenerate triangle.
#[inline]
fn inradius_tri(a: f64, b: f64, c: f64, area: f64) -> f64 {
    if area <= 0.0 {
        0.0
    } else {
        let s = 0.5 * (a + b + c);
        if s <= 0.0 {
            0.0
        } else {
            area / s
        }
    }
}

/// Measure the quality of a single triangle.
///
/// `vertices` is the full vertex array; `tri` is the three vertex indices.
/// The winding convention: CCW vertex order (right-hand rule) is the **valid**
/// orientation and yields `signed_area > 0`, `valid: true`.
pub fn tri_quality(vertices: &[Point3], tri: &[u32; 3]) -> Result<TriQuality, MeshQualityError> {
    let [a, b, c] = fetch_tri(vertices, tri, 0)?;
    Ok(tri_quality_points(a, b, c))
}

/// Measure the quality of a single triangle from its three corner points
/// directly (no index lookup). Winding convention as [`tri_quality`].
pub fn tri_quality_points(a: Point3, b: Point3, c: Point3) -> TriQuality {
    let e0 = sub(b, a); // edge a->b
    let e1 = sub(c, b); // edge b->c
    let e2 = sub(a, c); // edge c->a
    let l0 = norm(e0);
    let l1 = norm(e1);
    let l2 = norm(e2);
    let cr = cross(e0, e1);
    let area = 0.5 * norm(cr);
    // For a 3-D triangle the sign requires a reference normal; we report the
    // unsigned area as `signed_area` (see the field doc) and detect planar
    // inversion via `tri_signed_area_2d`.
    let min_edge = l0.min(l1).min(l2);
    let max_edge = l0.max(l1).max(l2);
    let ang_a = interior_angle(c, a, b); // angle at vertex a
    let ang_b = interior_angle(a, b, c); // angle at vertex b
    let ang_c = interior_angle(b, c, a); // angle at vertex c
    let min_angle = ang_a.min(ang_b).min(ang_c);
    let max_angle = ang_a.max(ang_b).max(ang_c);
    let r = inradius_tri(l0, l1, l2, area);
    let r_circ = circumradius_tri(l0, l1, l2, area);
    let edge_ratio = if min_edge > 0.0 {
        max_edge / min_edge
    } else {
        f64::INFINITY
    };
    let aspect_ratio = if r > 0.0 {
        r_circ / (2.0 * r)
    } else {
        f64::INFINITY
    };
    let radius_edge = if min_edge > 0.0 {
        r_circ / min_edge
    } else {
        f64::INFINITY
    };
    let valid = area > 0.0 && min_edge > 0.0;
    TriQuality {
        min_angle,
        max_angle,
        edge_ratio,
        aspect_ratio,
        radius_edge,
        area,
        signed_area: area,
        valid,
    }
}

/// Signed area of a 2-D triangle (vertices as `Point3` with `z = 0`), positive
/// for CCW. Use this to detect inversion in planar meshes.
pub fn tri_signed_area_2d(a: Point3, b: Point3, c: Point3) -> f64 {
    0.5 * ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y))
}

// ---------------------------------------------------------------------------
//  Tetrahedron quality
// ---------------------------------------------------------------------------

/// Per-tetrahedron quality report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TetQuality {
    /// Minimum dihedral angle (radians) over the 6 edges.
    pub min_dihedral: f64,
    /// Maximum dihedral angle (radians) over the 6 edges.
    pub max_dihedral: f64,
    /// Radius-edge ratio = circumradius / shortest_edge (>= sqrt(6)/4 ~ 0.612
    /// for regular tet; large = bad, slivers have large values).
    pub radius_edge: f64,
    /// Scaled Jacobian = det(a,b,c) / (|a|*|b|*|c|) at the reference vertex,
    /// range `[-1, 1]`. Regular tet ~ `sqrt(2)/2 ~ 0.707`; inverted < 0.
    pub scaled_jacobian: f64,
    /// Unsigned volume (>= 0).
    pub volume: f64,
    /// Signed volume: positive for the standard orientation
    /// (det(v1-v0, v2-v0, v3-v0) > 0), negative for inverted.
    pub signed_volume: f64,
    /// `true` iff non-degenerate and not inverted (`signed_volume > 0`).
    pub valid: bool,
    /// Edge ratio = longest_edge / shortest_edge (>= 1).
    pub edge_ratio: f64,
}

impl TetQuality {
    /// `true` if the tet is degenerate (zero volume).
    #[inline]
    pub fn is_degenerate(&self) -> bool {
        self.volume == 0.0
    }
    /// `true` if the tet is a sliver (small min dihedral AND large max dihedral
    /// - the classic sliver signature). Threshold: min_dihedral < ~16 deg and
    /// max_dihedral > ~164 deg (complementary).
    pub fn is_sliver(&self, min_dihedral_deg: f64) -> bool {
        let thresh = min_dihedral_deg.to_radians();
        self.valid && self.min_dihedral < thresh
    }
}

/// The six edges of a tet (as pairs of vertex-local-indices 0..3).
const TET_EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

/// The four faces of a tet as triples of vertex-local-indices, outward-oriented
/// (opposite vertex listed last is excluded; winding chosen so the normal
/// points away from the excluded vertex).
const TET_FACES: [(usize, usize, usize); 4] = [
    (1, 2, 3), // face opposite vertex 0
    (0, 3, 2), // face opposite vertex 1
    (0, 1, 3), // face opposite vertex 2
    (0, 2, 1), // face opposite vertex 3
];

/// For each of the 6 edges, the two faces that share it (indices into
/// `TET_FACES`). Used for dihedral-angle computation.
const TET_EDGE_FACES: [(usize, usize); 6] = [
    (0, 1), // edge 0-1 shared by faces 0 and 1
    (0, 2), // edge 0-2 shared by faces 0 and 2
    (0, 3), // edge 0-3 shared by faces 0 and 3
    (1, 2), // edge 1-2 shared by faces 1 and 2
    (1, 3), // edge 1-3 shared by faces 1 and 3
    (2, 3), // edge 2-3 shared by faces 2 and 3
];

/// Outward unit normal of a face given the four tet corners.
fn face_normal_outward(corners: &[Point3; 4], face: usize) -> Point3 {
    let (i, j, k) = TET_FACES[face];
    let u = sub(corners[j], corners[i]);
    let v = sub(corners[k], corners[i]);
    let n = cross(u, v);
    let len = norm(n);
    if len == 0.0 {
        return Point3::new(0.0, 0.0, 0.0);
    }
    // The face winding in TET_FACES is chosen so the normal already points
    // outward (away from the excluded vertex). Verify by dotting with the
    // direction from the face centroid to the excluded vertex.
    let excluded = [0, 1, 2, 3][face];
    let centroid = Point3::new(
        (corners[i].x + corners[j].x + corners[k].x) / 3.0,
        (corners[i].y + corners[j].y + corners[k].y) / 3.0,
        (corners[i].z + corners[j].z + corners[k].z) / 3.0,
    );
    let outward_dir = sub(corners[excluded], centroid);
    let n_unit = Point3::new(n.x / len, n.y / len, n.z / len);
    if dot(n_unit, outward_dir) > 0.0 {
        // Normal points toward the excluded vertex - flip (inward). We want
        // outward, so flip.
        Point3::new(-n_unit.x, -n_unit.y, -n_unit.z)
    } else {
        n_unit
    }
}

/// Dihedral angle (radians) along an edge shared by two faces, measured as the
/// interior angle between the two **outward** face normals: `pi - angle(n0, n1)`.
fn dihedral_angle(n0: Point3, n1: Point3) -> f64 {
    let d = dot(n0, n1).clamp(-1.0, 1.0);
    // Interior dihedral = pi - exterior angle between outward normals.
    core::f64::consts::PI - d.acos()
}

/// Circumradius of a tet from its four corners.
///
/// Solves |x - a|^2 = |x - b|^2 = |x - c|^2 = |x - d|^2 for the circumcenter and
/// returns the radius. Returns `+inf` for a degenerate (coplanar) tet.
fn circumradius_tet(corners: &[Point3; 4]) -> f64 {
    let a = corners[0];
    let b = corners[1];
    let c = corners[2];
    let d = corners[3];
    // Set a as origin; solve M * x = rhs where M rows are (b-a, c-a, d-a)
    // and rhs_i = 0.5 * |v_i|^2. x is the circumcenter relative to a.
    let v1 = sub(b, a);
    let v2 = sub(c, a);
    let v3 = sub(d, a);
    let rhs = Point3::new(0.5 * dot(v1, v1), 0.5 * dot(v2, v2), 0.5 * dot(v3, v3));
    // The inverse of a 3x3 matrix with rows r1, r2, r3 has *columns*
    // (r2 x r3, r3 x r1, r1 x r2) / det, so
    //   x = (rhs.x * (r2 x r3) + rhs.y * (r3 x r1) + rhs.z * (r1 x r2)) / det,
    // component-wise. (Replacing columns, not rows, in Cramer's rule.)
    let cr23 = cross(v2, v3);
    let cr31 = cross(v3, v1);
    let cr12 = cross(v1, v2);
    let det_m = dot(v1, cr23);
    if det_m == 0.0 {
        return f64::INFINITY;
    }
    let inv_det = 1.0 / det_m;
    let cx = (rhs.x * cr23.x + rhs.y * cr31.x + rhs.z * cr12.x) * inv_det;
    let cy = (rhs.x * cr23.y + rhs.y * cr31.y + rhs.z * cr12.y) * inv_det;
    let cz = (rhs.x * cr23.z + rhs.y * cr31.z + rhs.z * cr12.z) * inv_det;
    let center = Point3::new(a.x + cx, a.y + cy, a.z + cz);
    norm(sub(center, a))
}

/// Measure the quality of a single tetrahedron.
///
/// `vertices` is the full vertex array; `tet` is the four vertex indices.
/// The standard orientation (positive signed volume) is
/// `det(v1-v0, v2-v0, v3-v0) > 0`; the opposite winding is **inverted**
/// (`valid: false`, `signed_volume < 0`).
pub fn tet_quality(vertices: &[Point3], tet: &[u32; 4]) -> Result<TetQuality, MeshQualityError> {
    let corners = fetch_tet(vertices, tet, 0)?;
    Ok(tet_quality_points(
        corners[0], corners[1], corners[2], corners[3],
    ))
}

/// Measure the quality of a single tet from its four corner points directly.
pub fn tet_quality_points(a: Point3, b: Point3, c: Point3, d: Point3) -> TetQuality {
    let corners = [a, b, c, d];
    // Edge lengths.
    let mut min_edge = f64::INFINITY;
    let mut max_edge = 0.0f64;
    for (i, j) in TET_EDGES {
        let l = norm(sub(corners[j], corners[i]));
        if l < min_edge {
            min_edge = l;
        }
        if l > max_edge {
            max_edge = l;
        }
    }
    // Signed volume = det(v1, v2, v3) / 6 with v1 = b-a etc.
    let v1 = sub(b, a);
    let v2 = sub(c, a);
    let v3 = sub(d, a);
    let det6 = dot(v1, cross(v2, v3));
    let signed_volume = det6 / 6.0;
    let volume = signed_volume.abs();
    // Scaled Jacobian at vertex a: det(v1,v2,v3) / (||v1|| ||v2|| ||v3||).
    let n1 = norm(v1);
    let n2 = norm(v2);
    let n3 = norm(v3);
    let scaled_jacobian = if n1 > 0.0 && n2 > 0.0 && n3 > 0.0 {
        det6 / (n1 * n2 * n3)
    } else {
        0.0
    };
    // Dihedral angles.
    let n0 = face_normal_outward(&corners, 0);
    let n1f = face_normal_outward(&corners, 1);
    let n2f = face_normal_outward(&corners, 2);
    let n3f = face_normal_outward(&corners, 3);
    let face_normals = [n0, n1f, n2f, n3f];
    let mut min_dihedral = f64::INFINITY;
    let mut max_dihedral = 0.0f64;
    for (f0, f1) in TET_EDGE_FACES {
        let na = face_normals[f0];
        let nb = face_normals[f1];
        if norm(na) == 0.0 || norm(nb) == 0.0 {
            // Degenerate face - dihedral undefined.
            min_dihedral = 0.0;
            max_dihedral = core::f64::consts::PI;
            continue;
        }
        let dh = dihedral_angle(na, nb);
        if dh < min_dihedral {
            min_dihedral = dh;
        }
        if dh > max_dihedral {
            max_dihedral = dh;
        }
    }
    if !min_dihedral.is_finite() {
        min_dihedral = 0.0;
    }
    // Circumradius.
    let r_circ = circumradius_tet(&corners);
    let radius_edge = if min_edge > 0.0 {
        r_circ / min_edge
    } else {
        f64::INFINITY
    };
    let edge_ratio = if min_edge > 0.0 {
        max_edge / min_edge
    } else {
        f64::INFINITY
    };
    let valid = volume > 0.0 && min_edge > 0.0 && signed_volume > 0.0;
    TetQuality {
        min_dihedral,
        max_dihedral,
        radius_edge,
        scaled_jacobian,
        volume,
        signed_volume,
        valid,
        edge_ratio,
    }
}

// ---------------------------------------------------------------------------
//  Aggregate quality over a mesh
// ---------------------------------------------------------------------------

/// Aggregate triangle-mesh quality statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TriMeshQualityStats {
    /// Number of triangles measured.
    pub count: usize,
    /// Minimum min-angle over all triangles (radians).
    pub global_min_angle: f64,
    /// Maximum max-angle over all triangles (radians).
    pub global_max_angle: f64,
    /// Maximum edge ratio over all triangles.
    pub max_edge_ratio: f64,
    /// Maximum aspect ratio over all triangles.
    pub max_aspect_ratio: f64,
    /// Maximum radius-edge ratio over all triangles.
    pub max_radius_edge: f64,
    /// Number of inverted triangles (signed_area <= 0 in 2-D / degenerate).
    pub invalid_count: usize,
    /// Number of degenerate triangles (zero area).
    pub degenerate_count: usize,
}

/// Measure aggregate quality over a triangle mesh.
///
/// Uses [`tri_signed_area_2d`] for inversion detection (planar `z = 0` meshes).
/// For 3-D surface meshes use [`tri_quality`] per-element and aggregate by
/// hand; this helper is for planar meshes where winding is meaningful.
pub fn tri_mesh_quality_2d(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
) -> Result<TriMeshQualityStats, MeshQualityError> {
    let mut stats = TriMeshQualityStats {
        count: 0,
        global_min_angle: f64::INFINITY,
        global_max_angle: 0.0,
        max_edge_ratio: 0.0,
        max_aspect_ratio: 0.0,
        max_radius_edge: 0.0,
        invalid_count: 0,
        degenerate_count: 0,
    };
    for (t, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch_tri(vertices, tri, t)?;
        let q = tri_quality_points(a, b, c);
        let signed = tri_signed_area_2d(a, b, c);
        stats.count += 1;
        if q.area == 0.0 {
            stats.degenerate_count += 1;
        }
        if signed <= 0.0 && q.area > 0.0 {
            stats.invalid_count += 1;
        }
        if q.min_angle < stats.global_min_angle {
            stats.global_min_angle = q.min_angle;
        }
        if q.max_angle > stats.global_max_angle {
            stats.global_max_angle = q.max_angle;
        }
        if q.edge_ratio > stats.max_edge_ratio {
            stats.max_edge_ratio = q.edge_ratio;
        }
        if q.aspect_ratio > stats.max_aspect_ratio {
            stats.max_aspect_ratio = q.aspect_ratio;
        }
        if q.radius_edge > stats.max_radius_edge {
            stats.max_radius_edge = q.radius_edge;
        }
    }
    if stats.count == 0 {
        stats.global_min_angle = 0.0;
    }
    Ok(stats)
}

/// Aggregate tet-mesh quality statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TetMeshQualityStats {
    /// Number of tets measured.
    pub count: usize,
    /// Minimum min-dihedral over all tets (radians).
    pub global_min_dihedral: f64,
    /// Maximum max-dihedral over all tets (radians).
    pub global_max_dihedral: f64,
    /// Maximum radius-edge ratio over all tets.
    pub max_radius_edge: f64,
    /// Minimum scaled Jacobian over all tets (negative = inverted).
    pub min_scaled_jacobian: f64,
    /// Number of inverted tets (signed_volume <= 0).
    pub invalid_count: usize,
    /// Number of degenerate tets (zero volume).
    pub degenerate_count: usize,
}

/// Measure aggregate quality over a tet mesh (slice of `[u32; 4]`).
pub fn tet_mesh_quality_slice(
    vertices: &[Point3],
    tets: &[[u32; 4]],
) -> Result<TetMeshQualityStats, MeshQualityError> {
    let mut stats = TetMeshQualityStats {
        count: 0,
        global_min_dihedral: f64::INFINITY,
        global_max_dihedral: 0.0,
        max_radius_edge: 0.0,
        min_scaled_jacobian: f64::INFINITY,
        invalid_count: 0,
        degenerate_count: 0,
    };
    for (t, tet) in tets.iter().enumerate() {
        let corners = fetch_tet(vertices, tet, t)?;
        let q = tet_quality_points(corners[0], corners[1], corners[2], corners[3]);
        stats.count += 1;
        if q.volume == 0.0 {
            stats.degenerate_count += 1;
        }
        if q.signed_volume <= 0.0 && q.volume > 0.0 {
            stats.invalid_count += 1;
        }
        if q.min_dihedral < stats.global_min_dihedral {
            stats.global_min_dihedral = q.min_dihedral;
        }
        if q.max_dihedral > stats.global_max_dihedral {
            stats.global_max_dihedral = q.max_dihedral;
        }
        if q.radius_edge > stats.max_radius_edge {
            stats.max_radius_edge = q.radius_edge;
        }
        if q.scaled_jacobian < stats.min_scaled_jacobian {
            stats.min_scaled_jacobian = q.scaled_jacobian;
        }
    }
    if stats.count == 0 {
        stats.global_min_dihedral = 0.0;
        stats.min_scaled_jacobian = 0.0;
    }
    Ok(stats)
}

// ---------------------------------------------------------------------------
//  Size field (isotropic)
// ---------------------------------------------------------------------------

/// Isotropic size field: maps a position to a desired edge length.
///
/// Two modes:
/// - [`SizeField::Constant`] - uniform target `h` everywhere.
/// - [`SizeField::Background`] - barycentric interpolation of per-vertex sizes
///   over a background triangle mesh (planar, `z = 0`).
#[derive(Debug, Clone)]
pub enum SizeField {
    /// Uniform target edge length `h` everywhere.
    Constant { h: f64 },
    /// Background-mesh interpolation: `bg_vertices` carry per-vertex sizes in
    /// `bg_sizes`, interpolated barycentrically over `bg_triangles`.
    Background {
        bg_vertices: Vec<Point3>,
        bg_triangles: Vec<[u32; 3]>,
        bg_sizes: Vec<f64>,
        /// If `true`, a query outside the background returns the size of the
        /// nearest background vertex (nearest-neighbour extrapolation). If
        /// `false`, it returns [`MeshQualityError::QueryOutsideBackground`].
        extrapolate: bool,
    },
}

impl SizeField {
    /// Desired edge length at `p`.
    pub fn size_at(&self, p: Point3) -> Result<f64, MeshQualityError> {
        match self {
            SizeField::Constant { h } => {
                if !h.is_finite() || *h <= 0.0 {
                    return Err(MeshQualityError::NonFiniteCoordinate { index: u32::MAX });
                }
                Ok(*h)
            }
            SizeField::Background {
                bg_vertices,
                bg_triangles,
                bg_sizes,
                extrapolate,
            } => {
                if bg_triangles.is_empty() || bg_vertices.is_empty() {
                    return Err(MeshQualityError::EmptyBackground);
                }
                let mut best: Option<(usize, [f64; 3])> = None;
                for (ti, tri) in bg_triangles.iter().enumerate() {
                    let a = bg_vertices[tri[0] as usize];
                    let b = bg_vertices[tri[1] as usize];
                    let c = bg_vertices[tri[2] as usize];
                    let (lambda, inside) = barycentric_2d(a, b, c, p);
                    if inside {
                        // Deterministic: first triangle that contains p wins.
                        let s = lambda[0] * bg_sizes[tri[0] as usize]
                            + lambda[1] * bg_sizes[tri[1] as usize]
                            + lambda[2] * bg_sizes[tri[2] as usize];
                        return Ok(s);
                    }
                    // Track the closest barycentric point for extrapolation.
                    let min_lam = lambda[0].min(lambda[1]).min(lambda[2]);
                    let prev_min = best.map(|(_, l)| l[0].min(l[1]).min(l[2]));
                    match prev_min {
                        None => best = Some((ti, lambda)),
                        Some(pm) if min_lam > pm => best = Some((ti, lambda)),
                        _ => {}
                    }
                }
                if *extrapolate {
                    if let Some((ti, lambda)) = best {
                        let tri = bg_triangles[ti];
                        let s = lambda[0] * bg_sizes[tri[0] as usize]
                            + lambda[1] * bg_sizes[tri[1] as usize]
                            + lambda[2] * bg_sizes[tri[2] as usize];
                        return Ok(s.max(1e-12));
                    }
                }
                Err(MeshQualityError::QueryOutsideBackground)
            }
        }
    }
}

/// Barycentric coordinates of `p` w.r.t. triangle `(a, b, c)` (planar, `z = 0`).
///
/// Returns `(lambda, inside)` where `inside` is `true` iff all three weights
/// are `>= -eps` (with `eps = 1e-12`).
fn barycentric_2d(a: Point3, b: Point3, c: Point3, p: Point3) -> ([f64; 3], bool) {
    let det_t = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    if det_t == 0.0 {
        return ([0.0, 0.0, 0.0], false);
    }
    let inv = 1.0 / det_t;
    let l0 = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) * inv;
    let l1 = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) * inv;
    let l2 = 1.0 - l0 - l1;
    let eps = 1e-12;
    ([l0, l1, l2], l0 >= -eps && l1 >= -eps && l2 >= -eps)
}

// ---------------------------------------------------------------------------
//  Anisotropy field (metric tensor)
// ---------------------------------------------------------------------------

/// A 3x3 symmetric metric tensor stored as the 6 unique components in row-major
/// lower-triangular order: `[M00, M10, M11, M20, M21, M22]`.
///
/// The metric length of an edge `e` is `sqrt(e^T M e)`. A mesher aims for unit
/// metric length. `M` must be symmetric positive-definite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricTensor {
    pub m00: f64,
    pub m10: f64,
    pub m11: f64,
    pub m20: f64,
    pub m21: f64,
    pub m22: f64,
}

impl MetricTensor {
    /// Identity metric (isotropic, unit target).
    pub const IDENTITY: MetricTensor = MetricTensor {
        m00: 1.0,
        m10: 0.0,
        m11: 1.0,
        m20: 0.0,
        m21: 0.0,
        m22: 1.0,
    };

    /// Isotropic metric with target edge length `h` (`M = I / h^2`).
    pub fn isotropic(h: f64) -> MetricTensor {
        let s = 1.0 / (h * h);
        MetricTensor {
            m00: s,
            m10: 0.0,
            m11: s,
            m20: 0.0,
            m21: 0.0,
            m22: s,
        }
    }

    /// Metric length of an edge `e = b - a`: `sqrt(e^T M e)`.
    pub fn length_of(&self, a: Point3, b: Point3) -> f64 {
        let e = sub(b, a);
        let ex = e.x;
        let ey = e.y;
        let ez = e.z;
        let q = self.m00 * ex * ex
            + 2.0 * self.m10 * ex * ey
            + 2.0 * self.m20 * ex * ez
            + self.m11 * ey * ey
            + 2.0 * self.m21 * ey * ez
            + self.m22 * ez * ez;
        if q <= 0.0 {
            0.0
        } else {
            q.sqrt()
        }
    }

    /// Check positive-definiteness via the leading principal minors
    /// (Sylvester's criterion): all three leading principal minors > 0.
    pub fn is_positive_definite(&self) -> bool {
        let d1 = self.m00;
        let d2 = self.m00 * self.m11 - self.m10 * self.m10;
        let d3 = self.m00 * (self.m11 * self.m22 - self.m21 * self.m21)
            - self.m10 * (self.m10 * self.m22 - self.m21 * self.m20)
            + self.m20 * (self.m10 * self.m21 - self.m11 * self.m20);
        d1 > 0.0 && d2 > 0.0 && d3 > 0.0
    }
}

/// Anisotropy field: maps a position to a [`MetricTensor`].
#[derive(Debug, Clone)]
pub enum AnisotropyField {
    /// Uniform metric everywhere.
    Uniform { metric: MetricTensor },
    /// Background-mesh interpolation of per-vertex metric tensors.
    Background {
        bg_vertices: Vec<Point3>,
        bg_triangles: Vec<[u32; 3]>,
        bg_metrics: Vec<MetricTensor>,
        extrapolate: bool,
    },
}

impl AnisotropyField {
    /// Metric tensor at `p`.
    pub fn metric_at(&self, p: Point3) -> Result<MetricTensor, MeshQualityError> {
        match self {
            AnisotropyField::Uniform { metric } => {
                if !metric.is_positive_definite() {
                    return Err(MeshQualityError::IndefiniteMetric { vertex: u32::MAX });
                }
                Ok(*metric)
            }
            AnisotropyField::Background {
                bg_vertices,
                bg_triangles,
                bg_metrics,
                extrapolate,
            } => {
                if bg_triangles.is_empty() || bg_vertices.is_empty() {
                    return Err(MeshQualityError::EmptyBackground);
                }
                let mut best: Option<(usize, [f64; 3])> = None;
                for (ti, tri) in bg_triangles.iter().enumerate() {
                    let a = bg_vertices[tri[0] as usize];
                    let b = bg_vertices[tri[1] as usize];
                    let c = bg_vertices[tri[2] as usize];
                    let (lambda, inside) = barycentric_2d(a, b, c, p);
                    if inside {
                        let m0 = bg_metrics[tri[0] as usize];
                        let m1 = bg_metrics[tri[1] as usize];
                        let m2 = bg_metrics[tri[2] as usize];
                        let m = blend_metric(m0, m1, m2, lambda[0], lambda[1], lambda[2]);
                        if !m.is_positive_definite() {
                            return Err(MeshQualityError::IndefiniteMetric { vertex: tri[0] });
                        }
                        return Ok(m);
                    }
                    let min_lam = lambda[0].min(lambda[1]).min(lambda[2]);
                    let prev_min = best.map(|(_, l)| l[0].min(l[1]).min(l[2]));
                    match prev_min {
                        None => best = Some((ti, lambda)),
                        Some(pm) if min_lam > pm => best = Some((ti, lambda)),
                        _ => {}
                    }
                }
                if *extrapolate {
                    if let Some((ti, lambda)) = best {
                        let tri = bg_triangles[ti];
                        let m0 = bg_metrics[tri[0] as usize];
                        let m1 = bg_metrics[tri[1] as usize];
                        let m2 = bg_metrics[tri[2] as usize];
                        let m = blend_metric(m0, m1, m2, lambda[0], lambda[1], lambda[2]);
                        return Ok(m);
                    }
                }
                Err(MeshQualityError::QueryOutsideBackground)
            }
        }
    }
}

/// Linear blend of three metric tensors by barycentric weights.
fn blend_metric(
    m0: MetricTensor,
    m1: MetricTensor,
    m2: MetricTensor,
    l0: f64,
    l1: f64,
    l2: f64,
) -> MetricTensor {
    MetricTensor {
        m00: l0 * m0.m00 + l1 * m1.m00 + l2 * m2.m00,
        m10: l0 * m0.m10 + l1 * m1.m10 + l2 * m2.m10,
        m11: l0 * m0.m11 + l1 * m1.m11 + l2 * m2.m11,
        m20: l0 * m0.m20 + l1 * m1.m20 + l2 * m2.m20,
        m21: l0 * m0.m21 + l1 * m1.m21 + l2 * m2.m21,
        m22: l0 * m0.m22 + l1 * m1.m22 + l2 * m2.m22,
    }
}

// ---------------------------------------------------------------------------
//  Field conformance
// ---------------------------------------------------------------------------

/// Per-element field-conformance report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldConformance {
    /// Minimum metric edge length / target (1.0 = conformant; < 1 = too small).
    pub min_ratio: f64,
    /// Maximum metric edge length / target (1.0 = conformant; > 1 = too large).
    pub max_ratio: f64,
    /// Number of edges measured.
    pub edge_count: usize,
}

impl FieldConformance {
    /// `true` iff all edge ratios are within `[1/tol, tol]` of the target (1.0).
    #[inline]
    pub fn within(&self, tol: f64) -> bool {
        self.min_ratio >= 1.0 / tol && self.max_ratio <= tol
    }
}

/// Check field conformance for a triangle mesh against an anisotropy field.
///
/// Each triangle edge is measured in the metric at the edge midpoint; the
/// ratio to the unit target is reported. `tol > 1.0` is the acceptance band
/// (e.g. `1.5` means edges may be 0.67x-1.5x the target).
pub fn check_field_conformance_tri(
    vertices: &[Point3],
    triangles: &[[u32; 3]],
    field: &AnisotropyField,
) -> Result<FieldConformance, MeshQualityError> {
    let mut min_ratio = f64::INFINITY;
    let mut max_ratio = 0.0f64;
    let mut edge_count = 0usize;
    for (t, tri) in triangles.iter().enumerate() {
        let [a, b, c] = fetch_tri(vertices, tri, t)?;
        for (p, q) in [(a, b), (b, c), (c, a)] {
            let mid = Point3::new((p.x + q.x) * 0.5, (p.y + q.y) * 0.5, (p.z + q.z) * 0.5);
            let m = field.metric_at(mid)?;
            let len = m.length_of(p, q);
            if len > 0.0 {
                let r = len; // target is 1.0 in metric space
                if r < min_ratio {
                    min_ratio = r;
                }
                if r > max_ratio {
                    max_ratio = r;
                }
                edge_count += 1;
            }
        }
    }
    if edge_count == 0 {
        min_ratio = 1.0;
        max_ratio = 1.0;
    }
    Ok(FieldConformance {
        min_ratio,
        max_ratio,
        edge_count,
    })
}

/// Check field conformance for a tet mesh against an anisotropy field.
pub fn check_field_conformance_tet(
    vertices: &[Point3],
    tets: &[[u32; 4]],
    field: &AnisotropyField,
) -> Result<FieldConformance, MeshQualityError> {
    let mut min_ratio = f64::INFINITY;
    let mut max_ratio = 0.0f64;
    let mut edge_count = 0usize;
    for (t, tet) in tets.iter().enumerate() {
        let corners = fetch_tet(vertices, tet, t)?;
        for (i, j) in TET_EDGES {
            let p = corners[i];
            let q = corners[j];
            let mid = Point3::new((p.x + q.x) * 0.5, (p.y + q.y) * 0.5, (p.z + q.z) * 0.5);
            let m = field.metric_at(mid)?;
            let len = m.length_of(p, q);
            if len > 0.0 {
                let r = len;
                if r < min_ratio {
                    min_ratio = r;
                }
                if r > max_ratio {
                    max_ratio = r;
                }
                edge_count += 1;
            }
        }
    }
    if edge_count == 0 {
        min_ratio = 1.0;
        max_ratio = 1.0;
    }
    Ok(FieldConformance {
        min_ratio,
        max_ratio,
        edge_count,
    })
}

// ---------------------------------------------------------------------------
//  Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use core::f64::consts::FRAC_PI_3; // 60 deg
    use core::f64::consts::PI;

    fn approx(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[test]
    fn equilateral_triangle_quality() {
        let s = 3.0f64.sqrt();
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.5, s / 2.0, 0.0);
        let q = tri_quality_points(a, b, c);
        assert!(
            approx(q.min_angle, FRAC_PI_3, 1e-12),
            "min angle {}",
            q.min_angle
        );
        assert!(
            approx(q.max_angle, FRAC_PI_3, 1e-12),
            "max angle {}",
            q.max_angle
        );
        assert!(approx(q.edge_ratio, 1.0, 1e-12));
        assert!(
            approx(q.aspect_ratio, 1.0, 1e-9),
            "aspect {}",
            q.aspect_ratio
        );
        assert!(
            approx(q.radius_edge, 1.0 / s, 1e-9),
            "radius_edge {}",
            q.radius_edge
        );
        assert!(approx(q.area, s / 4.0, 1e-12));
        assert!(q.valid);
    }

    #[test]
    fn right_triangle_quality() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let q = tri_quality_points(a, b, c);
        assert!(approx(q.min_angle, PI / 4.0, 1e-12)); // 45 deg
        assert!(approx(q.max_angle, PI / 2.0, 1e-12)); // 90 deg
        assert!(approx(q.area, 0.5, 1e-12));
        assert!(q.edge_ratio > 1.0);
        assert!(q.valid);
    }

    #[test]
    fn degenerate_triangle_is_invalid() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(2.0, 0.0, 0.0);
        let q = tri_quality_points(a, b, c);
        assert!(q.is_degenerate());
        assert!(!q.valid);
        assert!(q.min_angle == 0.0);
        assert!(q.area == 0.0);
    }

    #[test]
    fn inverted_triangle_2d_detected() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        assert!(tri_signed_area_2d(a, b, c) > 0.0); // CCW
        assert!(tri_signed_area_2d(a, c, b) < 0.0); // CW = inverted
    }

    #[test]
    fn tri_mesh_quality_2d_aggregate() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        // Two triangles forming a unit square: [0,1,3] CCW, [0,3,2] CCW.
        let t = vec![[0u32, 1, 3], [0, 3, 2]];
        let s = tri_mesh_quality_2d(&v, &t).unwrap();
        assert_eq!(s.count, 2);
        assert_eq!(s.invalid_count, 0);
        assert_eq!(s.degenerate_count, 0);
        assert!(s.global_min_angle > 0.0);
    }

    #[test]
    fn tri_mesh_quality_2d_detects_inverted() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t = vec![[0u32, 2, 1]]; // CW = inverted
        let s = tri_mesh_quality_2d(&v, &t).unwrap();
        assert_eq!(s.invalid_count, 1);
    }

    // -- Tet quality --

    fn regular_tet(edge: f64) -> [Point3; 4] {
        // Regular tet with given edge length, vertex 0 at origin.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(edge, 0.0, 0.0);
        let s3 = 3.0f64.sqrt();
        let c = Point3::new(edge / 2.0, edge * s3 / 2.0, 0.0);
        let s6 = 6.0f64.sqrt();
        let d = Point3::new(edge / 2.0, edge * s3 / 6.0, edge * s6 / 3.0);
        [a, b, c, d]
    }

    #[test]
    fn regular_tet_quality() {
        let [a, b, c, d] = regular_tet(1.0);
        let q = tet_quality_points(a, b, c, d);
        assert!(q.valid);
        assert!(
            approx(q.volume, 2.0f64.sqrt() / 12.0, 1e-12),
            "volume {}",
            q.volume
        );
        // Scaled Jacobian of regular tet = sqrt(2)/2.
        assert!(
            approx(q.scaled_jacobian, 2.0f64.sqrt() / 2.0, 1e-12),
            "scaled_jacobian {}",
            q.scaled_jacobian
        );
        // Radius-edge ratio = sqrt(6)/4.
        assert!(
            approx(q.radius_edge, 6.0f64.sqrt() / 4.0, 1e-12),
            "radius_edge {}",
            q.radius_edge
        );
        // Dihedral angle of regular tet = arccos(1/3) ~ 70.53 deg.
        let expected = (1.0f64 / 3.0).acos();
        assert!(
            approx(q.min_dihedral, expected, 1e-9),
            "min dihedral {} expected {}",
            q.min_dihedral,
            expected
        );
        assert!(approx(q.max_dihedral, expected, 1e-9));
        assert!(approx(q.edge_ratio, 1.0, 1e-12));
    }

    #[test]
    fn inverted_tet_is_invalid() {
        let [a, b, c, d] = regular_tet(1.0);
        // Swap c and d to invert orientation.
        let q = tet_quality_points(a, b, d, c);
        assert!(!q.valid);
        assert!(q.signed_volume < 0.0);
        assert!(q.scaled_jacobian < 0.0);
    }

    #[test]
    fn degenerate_tet_is_invalid() {
        // Four coplanar points.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(1.0, 1.0, 0.0);
        let q = tet_quality_points(a, b, c, d);
        assert!(q.is_degenerate());
        assert!(!q.valid);
        assert!(q.volume == 0.0);
    }

    #[test]
    fn sliver_tet_detected() {
        // A sliver: four points nearly coplanar but forming a valid (positive
        // volume) very-flat tet.
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.5, 0.8, 0.0);
        let d = Point3::new(0.5, 0.4, 0.01);
        let q = tet_quality_points(a, b, c, d);
        assert!(q.valid, "sliver should be valid (positive volume)");
        assert!(
            q.is_sliver(20.0),
            "should be flagged sliver at 20 deg threshold"
        );
        assert!(q.min_dihedral < 20.0f64.to_radians());
    }

    #[test]
    fn tet_mesh_quality_slice_aggregate() {
        let v: Vec<Point3> = regular_tet(1.0).to_vec();
        let tets = vec![[0u32, 1, 2, 3]];
        let s = tet_mesh_quality_slice(&v, &tets).unwrap();
        assert_eq!(s.count, 1);
        assert_eq!(s.invalid_count, 0);
        assert!(s.min_scaled_jacobian > 0.0);
    }

    #[test]
    fn tet_mesh_quality_slice_detects_inverted() {
        let v: Vec<Point3> = regular_tet(1.0).to_vec();
        let tets = vec![[0u32, 1, 3, 2]]; // inverted
        let s = tet_mesh_quality_slice(&v, &tets).unwrap();
        assert_eq!(s.invalid_count, 1);
        assert!(s.min_scaled_jacobian < 0.0);
    }

    // -- Size field --

    #[test]
    fn size_field_constant() {
        let f = SizeField::Constant { h: 0.5 };
        assert!(approx(
            f.size_at(Point3::new(1.0, 2.0, 3.0)).unwrap(),
            0.5,
            1e-12
        ));
    }

    #[test]
    fn size_field_background_interpolation() {
        // Background: one triangle with sizes 1.0, 2.0, 3.0 at the corners.
        let bg_v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let bg_t = vec![[0u32, 1, 2]];
        let bg_s = vec![1.0, 2.0, 3.0];
        let f = SizeField::Background {
            bg_vertices: bg_v,
            bg_triangles: bg_t,
            bg_sizes: bg_s,
            extrapolate: false,
        };
        // At the centroid, size = (1+2+3)/3 = 2.0.
        let c = Point3::new(1.0 / 3.0, 1.0 / 3.0, 0.0);
        assert!(approx(f.size_at(c).unwrap(), 2.0, 1e-12));
        // At vertex 0, size = 1.0.
        assert!(approx(
            f.size_at(Point3::new(0.0, 0.0, 0.0)).unwrap(),
            1.0,
            1e-12
        ));
        // Outside -> error.
        assert!(f.size_at(Point3::new(2.0, 2.0, 0.0)).is_err());
    }

    #[test]
    fn size_field_background_extrapolate() {
        let bg_v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let bg_t = vec![[0u32, 1, 2]];
        let bg_s = vec![1.0, 1.0, 1.0];
        let f = SizeField::Background {
            bg_vertices: bg_v,
            bg_triangles: bg_t,
            bg_sizes: bg_s,
            extrapolate: true,
        };
        // Outside but extrapolating -> returns ~1.0 (nearest triangle blend).
        let s = f.size_at(Point3::new(2.0, 2.0, 0.0)).unwrap();
        assert!(s.is_finite() && s > 0.0);
    }

    // -- Anisotropy field --

    #[test]
    fn metric_identity_length() {
        let m = MetricTensor::IDENTITY;
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(3.0, 4.0, 0.0);
        assert!(approx(m.length_of(a, b), 5.0, 1e-12));
    }

    #[test]
    fn metric_isotropic_scales_length() {
        let m = MetricTensor::isotropic(2.0); // target h=2 -> M = I/4
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(2.0, 0.0, 0.0);
        // metric length = sqrt( (2)^2 / 4 ) = 1.0 -> conformant.
        assert!(approx(m.length_of(a, b), 1.0, 1e-12));
    }

    #[test]
    fn metric_positive_definite_check() {
        assert!(MetricTensor::IDENTITY.is_positive_definite());
        assert!(MetricTensor::isotropic(0.5).is_positive_definite());
        // Indefinite: m00 = -1.
        let bad = MetricTensor {
            m00: -1.0,
            m10: 0.0,
            m11: 1.0,
            m20: 0.0,
            m21: 0.0,
            m22: 1.0,
        };
        assert!(!bad.is_positive_definite());
    }

    #[test]
    fn anisotropy_field_uniform() {
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(1.0),
        };
        let m = f.metric_at(Point3::new(0.0, 0.0, 0.0)).unwrap();
        assert!(m.is_positive_definite());
    }

    #[test]
    fn anisotropy_field_background() {
        let bg_v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let bg_t = vec![[0u32, 1, 2]];
        let bg_m = vec![
            MetricTensor::isotropic(1.0),
            MetricTensor::isotropic(2.0),
            MetricTensor::isotropic(3.0),
        ];
        let f = AnisotropyField::Background {
            bg_vertices: bg_v,
            bg_triangles: bg_t,
            bg_metrics: bg_m,
            extrapolate: false,
        };
        let m = f.metric_at(Point3::new(1.0 / 3.0, 1.0 / 3.0, 0.0)).unwrap();
        assert!(m.is_positive_definite());
    }

    // -- Field conformance --

    #[test]
    fn field_conformance_conformant_mesh() {
        // Two triangles forming a unit square, isotropic field h=1.
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
        ];
        let t = vec![[0u32, 1, 3], [0, 3, 2]];
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(1.0),
        };
        let c = check_field_conformance_tri(&v, &t, &f).unwrap();
        // All edges are length 1 or sqrt(2); metric length with h=1 -> 1 or sqrt(2).
        assert!(c.min_ratio >= 1.0 - 1e-12);
        assert!(c.max_ratio <= 2.0f64.sqrt() + 1e-12);
        assert_eq!(c.edge_count, 6); // 3 + 3 (diagonal counted once per triangle)
    }

    #[test]
    fn field_conformance_within_tolerance() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        let t = vec![[0u32, 1, 2]];
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(1.0),
        };
        let c = check_field_conformance_tri(&v, &t, &f).unwrap();
        // Edges: 1, 1, sqrt(2). max_ratio = sqrt(2) ~ 1.414.
        assert!(c.within(1.5));
        assert!(!c.within(1.3)); // sqrt(2) > 1.3
    }

    #[test]
    fn field_conformance_tet_mesh() {
        let v: Vec<Point3> = regular_tet(1.0).to_vec();
        let tets = vec![[0u32, 1, 2, 3]];
        let f = AnisotropyField::Uniform {
            metric: MetricTensor::isotropic(1.0),
        };
        let c = check_field_conformance_tet(&v, &tets, &f).unwrap();
        assert_eq!(c.edge_count, 6);
        assert!(c.min_ratio >= 1.0 - 1e-12);
        assert!(c.max_ratio <= 1.0 + 1e-12);
    }

    // -- Error paths --

    #[test]
    fn index_out_of_bounds_errors() {
        let v = vec![Point3::new(0.0, 0.0, 0.0)];
        assert!(tri_quality(&v, &[0, 1, 2]).is_err());
        assert!(tet_quality(&v, &[0, 1, 2, 3]).is_err());
    }

    #[test]
    fn non_finite_coordinate_errors() {
        let v = vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(f64::NAN, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ];
        assert!(tri_quality(&v, &[0, 1, 2]).is_err());
    }

    #[test]
    fn empty_size_field_errors() {
        let f = SizeField::Background {
            bg_vertices: vec![],
            bg_triangles: vec![],
            bg_sizes: vec![],
            extrapolate: true,
        };
        assert!(f.size_at(Point3::new(0.0, 0.0, 0.0)).is_err());
    }

    #[test]
    fn indefinite_metric_rejected() {
        let bad = MetricTensor {
            m00: -1.0,
            m10: 0.0,
            m11: 1.0,
            m20: 0.0,
            m21: 0.0,
            m22: 1.0,
        };
        let f = AnisotropyField::Uniform { metric: bad };
        assert!(f.metric_at(Point3::new(0.0, 0.0, 0.0)).is_err());
    }

    #[test]
    fn constant_size_field_rejects_non_finite() {
        let f = SizeField::Constant { h: -1.0 };
        assert!(f.size_at(Point3::new(0.0, 0.0, 0.0)).is_err());
    }
}
