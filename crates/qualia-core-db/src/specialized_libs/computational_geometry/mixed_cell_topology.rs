//! P13.9 - Quadrilateral/hexahedral and mixed-cell topology foundation.
//!
//! Mixed cells are a cold-construction representation for FEM/analysis
//! surfaces and volumes. The public records are flat `repr(C)` values with a
//! fixed eight-vertex payload so they can be copied into caller-owned buffers
//! and serialized without shape-specific heap objects.

use bytemuck::{Pod, Zeroable};

use super::primitives::Point3;

pub const CELL_KIND_TRIANGLE: u8 = 3;
pub const CELL_KIND_QUAD: u8 = 4;
pub const CELL_KIND_TETRA: u8 = 5;
pub const CELL_KIND_HEX: u8 = 8;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct MixedCell {
    pub kind: u8,
    pub vertex_count: u8,
    pub material: u16,
    pub vertices: [u32; 8],
}

impl MixedCell {
    pub fn triangle(a: u32, b: u32, c: u32, material: u16) -> Self {
        Self {
            kind: CELL_KIND_TRIANGLE,
            vertex_count: 3,
            material,
            vertices: [a, b, c, 0, 0, 0, 0, 0],
        }
    }

    pub fn quad(a: u32, b: u32, c: u32, d: u32, material: u16) -> Self {
        Self {
            kind: CELL_KIND_QUAD,
            vertex_count: 4,
            material,
            vertices: [a, b, c, d, 0, 0, 0, 0],
        }
    }

    pub fn tetra(a: u32, b: u32, c: u32, d: u32, material: u16) -> Self {
        Self {
            kind: CELL_KIND_TETRA,
            vertex_count: 4,
            material,
            vertices: [a, b, c, d, 0, 0, 0, 0],
        }
    }

    pub fn hex(vertices: [u32; 8], material: u16) -> Self {
        Self {
            kind: CELL_KIND_HEX,
            vertex_count: 8,
            material,
            vertices,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct BoundaryFace {
    pub kind: u8,
    pub vertex_count: u8,
    pub material: u16,
    pub vertices: [u32; 4],
    pub owner_cell: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixedTopologyReport {
    pub cell_count: usize,
    pub triangle_count: usize,
    pub quad_count: usize,
    pub tetra_count: usize,
    pub hex_count: usize,
    pub boundary_face_count: usize,
    pub min_signed_volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MixedTopologyError {
    UnknownCellKind { cell: usize, kind: u8 },
    WrongVertexCount { cell: usize, kind: u8, got: u8 },
    IndexOutOfBounds { cell: usize, vertex: u32 },
    DuplicateVertex { cell: usize },
    DegenerateCell { cell: usize },
    OutputTooSmall { required: usize },
}

pub fn triangles_to_mixed(
    triangles: &[[u32; 3]],
    material: u16,
    out: &mut [MixedCell],
) -> Result<usize, MixedTopologyError> {
    if out.len() < triangles.len() {
        return Err(MixedTopologyError::OutputTooSmall {
            required: triangles.len(),
        });
    }
    for (i, tri) in triangles.iter().enumerate() {
        out[i] = MixedCell::triangle(tri[0], tri[1], tri[2], material);
    }
    Ok(triangles.len())
}

pub fn tets_to_mixed(
    tets: &[[u32; 4]],
    material: u16,
    out: &mut [MixedCell],
) -> Result<usize, MixedTopologyError> {
    if out.len() < tets.len() {
        return Err(MixedTopologyError::OutputTooSmall {
            required: tets.len(),
        });
    }
    for (i, tet) in tets.iter().enumerate() {
        out[i] = MixedCell::tetra(tet[0], tet[1], tet[2], tet[3], material);
    }
    Ok(tets.len())
}

pub fn quad_to_triangles(cell: MixedCell) -> Result<[[u32; 3]; 2], MixedTopologyError> {
    if cell.kind != CELL_KIND_QUAD || cell.vertex_count != 4 {
        return Err(MixedTopologyError::WrongVertexCount {
            cell: 0,
            kind: cell.kind,
            got: cell.vertex_count,
        });
    }
    let v = cell.vertices;
    Ok([[v[0], v[1], v[2]], [v[0], v[2], v[3]]])
}

pub fn hex_to_tetrahedra(cell: MixedCell) -> Result<[[u32; 4]; 6], MixedTopologyError> {
    if cell.kind != CELL_KIND_HEX || cell.vertex_count != 8 {
        return Err(MixedTopologyError::WrongVertexCount {
            cell: 0,
            kind: cell.kind,
            got: cell.vertex_count,
        });
    }
    let v = cell.vertices;
    Ok([
        [v[0], v[1], v[2], v[6]],
        [v[0], v[2], v[3], v[6]],
        [v[0], v[3], v[7], v[6]],
        [v[0], v[7], v[4], v[6]],
        [v[0], v[4], v[5], v[6]],
        [v[0], v[5], v[1], v[6]],
    ])
}

pub fn validate_mixed_cells(
    vertices: &[Point3],
    cells: &[MixedCell],
) -> Result<MixedTopologyReport, MixedTopologyError> {
    let mut report = MixedTopologyReport {
        cell_count: cells.len(),
        triangle_count: 0,
        quad_count: 0,
        tetra_count: 0,
        hex_count: 0,
        boundary_face_count: 0,
        min_signed_volume: f64::INFINITY,
    };

    for (i, cell) in cells.iter().enumerate() {
        let expected =
            expected_vertex_count(cell.kind).ok_or(MixedTopologyError::UnknownCellKind {
                cell: i,
                kind: cell.kind,
            })?;
        if cell.vertex_count != expected {
            return Err(MixedTopologyError::WrongVertexCount {
                cell: i,
                kind: cell.kind,
                got: cell.vertex_count,
            });
        }
        validate_indices(vertices, *cell, i)?;
        validate_unique(*cell, i)?;

        match cell.kind {
            CELL_KIND_TRIANGLE => {
                report.triangle_count += 1;
                let a = vertices[cell.vertices[0] as usize];
                let b = vertices[cell.vertices[1] as usize];
                let c = vertices[cell.vertices[2] as usize];
                if norm(cross(sub(b, a), sub(c, a))) <= 0.0 {
                    return Err(MixedTopologyError::DegenerateCell { cell: i });
                }
            }
            CELL_KIND_QUAD => {
                report.quad_count += 1;
                let tris = quad_to_triangles(*cell)?;
                for tri in tris {
                    let a = vertices[tri[0] as usize];
                    let b = vertices[tri[1] as usize];
                    let c = vertices[tri[2] as usize];
                    if norm(cross(sub(b, a), sub(c, a))) <= 0.0 {
                        return Err(MixedTopologyError::DegenerateCell { cell: i });
                    }
                }
            }
            CELL_KIND_TETRA => {
                report.tetra_count += 1;
                let vol = signed_tet_volume(
                    vertices[cell.vertices[0] as usize],
                    vertices[cell.vertices[1] as usize],
                    vertices[cell.vertices[2] as usize],
                    vertices[cell.vertices[3] as usize],
                );
                if vol <= 0.0 {
                    return Err(MixedTopologyError::DegenerateCell { cell: i });
                }
                report.min_signed_volume = report.min_signed_volume.min(vol);
            }
            CELL_KIND_HEX => {
                report.hex_count += 1;
                let tets = hex_to_tetrahedra(*cell)?;
                let mut volume = 0.0;
                for tet in tets {
                    volume += signed_tet_volume(
                        vertices[tet[0] as usize],
                        vertices[tet[1] as usize],
                        vertices[tet[2] as usize],
                        vertices[tet[3] as usize],
                    )
                    .abs();
                }
                if volume <= 0.0 {
                    return Err(MixedTopologyError::DegenerateCell { cell: i });
                }
                report.min_signed_volume = report.min_signed_volume.min(volume);
            }
            _ => unreachable!(),
        }
    }
    report.boundary_face_count =
        extract_boundary_faces(cells, &mut vec![BoundaryFace::zeroed(); cells.len() * 6])?;
    if report.min_signed_volume == f64::INFINITY {
        report.min_signed_volume = 0.0;
    }
    Ok(report)
}

pub fn extract_boundary_faces(
    cells: &[MixedCell],
    out: &mut [BoundaryFace],
) -> Result<usize, MixedTopologyError> {
    let mut faces: Vec<BoundaryFace> = Vec::new();
    for (ci, cell) in cells.iter().enumerate() {
        match cell.kind {
            CELL_KIND_TETRA => {
                let v = cell.vertices;
                for f in [
                    [v[0], v[2], v[1], 0],
                    [v[0], v[1], v[3], 0],
                    [v[1], v[2], v[3], 0],
                    [v[2], v[0], v[3], 0],
                ] {
                    faces.push(BoundaryFace {
                        kind: CELL_KIND_TRIANGLE,
                        vertex_count: 3,
                        material: cell.material,
                        vertices: f,
                        owner_cell: ci as u32,
                    });
                }
            }
            CELL_KIND_HEX => {
                let v = cell.vertices;
                for f in [
                    [v[0], v[1], v[2], v[3]],
                    [v[4], v[7], v[6], v[5]],
                    [v[0], v[4], v[5], v[1]],
                    [v[1], v[5], v[6], v[2]],
                    [v[2], v[6], v[7], v[3]],
                    [v[3], v[7], v[4], v[0]],
                ] {
                    faces.push(BoundaryFace {
                        kind: CELL_KIND_QUAD,
                        vertex_count: 4,
                        material: cell.material,
                        vertices: f,
                        owner_cell: ci as u32,
                    });
                }
            }
            CELL_KIND_TRIANGLE | CELL_KIND_QUAD => {}
            _ => {
                return Err(MixedTopologyError::UnknownCellKind {
                    cell: ci,
                    kind: cell.kind,
                })
            }
        }
    }
    faces.sort_by_key(face_key);

    let mut count = 0usize;
    let mut i = 0usize;
    while i < faces.len() {
        let key = face_key(&faces[i]);
        let start = i;
        while i < faces.len() && face_key(&faces[i]) == key {
            i += 1;
        }
        if i - start == 1 {
            if count >= out.len() {
                return Err(MixedTopologyError::OutputTooSmall {
                    required: count + 1,
                });
            }
            out[count] = faces[start];
            count += 1;
        }
    }
    Ok(count)
}

fn expected_vertex_count(kind: u8) -> Option<u8> {
    match kind {
        CELL_KIND_TRIANGLE => Some(3),
        CELL_KIND_QUAD => Some(4),
        CELL_KIND_TETRA => Some(4),
        CELL_KIND_HEX => Some(8),
        _ => None,
    }
}

fn validate_indices(
    vertices: &[Point3],
    cell: MixedCell,
    cell_idx: usize,
) -> Result<(), MixedTopologyError> {
    for &v in &cell.vertices[..cell.vertex_count as usize] {
        if v as usize >= vertices.len() {
            return Err(MixedTopologyError::IndexOutOfBounds {
                cell: cell_idx,
                vertex: v,
            });
        }
    }
    Ok(())
}

fn validate_unique(cell: MixedCell, cell_idx: usize) -> Result<(), MixedTopologyError> {
    for i in 0..cell.vertex_count as usize {
        for j in i + 1..cell.vertex_count as usize {
            if cell.vertices[i] == cell.vertices[j] {
                return Err(MixedTopologyError::DuplicateVertex { cell: cell_idx });
            }
        }
    }
    Ok(())
}

fn face_key(face: &BoundaryFace) -> (u8, [u32; 4]) {
    let mut v = face.vertices;
    v[..face.vertex_count as usize].sort_unstable();
    (face.vertex_count, v)
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
fn signed_tet_volume(a: Point3, b: Point3, c: Point3, d: Point3) -> f64 {
    dot(sub(b, a), cross(sub(c, a), sub(d, a))) / 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tet_vertices() -> Vec<Point3> {
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ]
    }

    fn cube_vertices() -> Vec<Point3> {
        vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(1.0, 1.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
            Point3::new(1.0, 0.0, 1.0),
            Point3::new(1.0, 1.0, 1.0),
            Point3::new(0.0, 1.0, 1.0),
        ]
    }

    #[test]
    fn mixed_cell_is_flat_pod() {
        assert_eq!(core::mem::size_of::<MixedCell>(), 36);
        let c = MixedCell::triangle(0, 1, 2, 7);
        let bytes = bytemuck::bytes_of(&c);
        assert_eq!(bytes.len(), 36);
    }

    #[test]
    fn triangles_convert_to_mixed() {
        let mut out = [MixedCell::zeroed(); 2];
        let n = triangles_to_mixed(&[[0, 1, 2], [2, 3, 0]], 4, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(out[0], MixedCell::triangle(0, 1, 2, 4));
    }

    #[test]
    fn tetra_validation_and_boundary_faces() {
        let v = tet_vertices();
        let cells = [MixedCell::tetra(0, 1, 2, 3, 1)];
        let report = validate_mixed_cells(&v, &cells).unwrap();
        assert_eq!(report.tetra_count, 1);
        assert_eq!(report.boundary_face_count, 4);
        assert!(report.min_signed_volume > 0.0);
        let mut faces = [BoundaryFace::zeroed(); 4];
        assert_eq!(extract_boundary_faces(&cells, &mut faces).unwrap(), 4);
    }

    #[test]
    fn shared_tet_face_is_not_boundary() {
        let mut v = tet_vertices();
        v.push(Point3::new(0.0, 0.0, -1.0));
        let cells = [
            MixedCell::tetra(0, 1, 2, 3, 1),
            MixedCell::tetra(0, 2, 1, 4, 1),
        ];
        let mut faces = [BoundaryFace::zeroed(); 8];
        assert_eq!(extract_boundary_faces(&cells, &mut faces).unwrap(), 6);
    }

    #[test]
    fn hex_converts_to_six_tets() {
        let cell = MixedCell::hex([0, 1, 2, 3, 4, 5, 6, 7], 9);
        let tets = hex_to_tetrahedra(cell).unwrap();
        assert_eq!(tets.len(), 6);
        assert_eq!(tets[0], [0, 1, 2, 6]);
    }

    #[test]
    fn hex_validation_reports_boundary_quads() {
        let v = cube_vertices();
        let cells = [MixedCell::hex([0, 1, 2, 3, 4, 5, 6, 7], 2)];
        let report = validate_mixed_cells(&v, &cells).unwrap();
        assert_eq!(report.hex_count, 1);
        assert_eq!(report.boundary_face_count, 6);
    }

    #[test]
    fn invalid_index_fails_closed() {
        let v = tet_vertices();
        let err = validate_mixed_cells(&v, &[MixedCell::tetra(0, 1, 2, 9, 0)]).unwrap_err();
        assert_eq!(
            err,
            MixedTopologyError::IndexOutOfBounds { cell: 0, vertex: 9 }
        );
    }

    #[test]
    fn duplicate_vertex_fails_closed() {
        let v = tet_vertices();
        let err = validate_mixed_cells(&v, &[MixedCell::tetra(0, 1, 1, 3, 0)]).unwrap_err();
        assert_eq!(err, MixedTopologyError::DuplicateVertex { cell: 0 });
    }
}
