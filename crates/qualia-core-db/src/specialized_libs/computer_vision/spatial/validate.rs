//! Mesh validation before commit to geometry store / Q42.

use super::geometry_ir::{MeshIR, MAX_INDICES, MAX_VERTICES};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshValidationStatus {
    Valid = 0,
    Empty = 1,
    TooManyVertices = 2,
    TooManyIndices = 3,
    IndexOutOfBounds = 4,
    NonFinite = 5,
    DegenerateTriangle = 6,
    BadIndexCount = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshValidationReport {
    pub status: MeshValidationStatus,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub degenerate_count: u32,
}

impl MeshValidationReport {
    pub fn ok(self) -> bool {
        self.status == MeshValidationStatus::Valid
    }
}

/// Validate mesh IR. Does not allocate.
pub fn validate_mesh_ir(mesh: &MeshIR) -> MeshValidationReport {
    let vc = mesh.positions.len();
    let ic = mesh.indices.len();
    let mut rep = MeshValidationReport {
        status: MeshValidationStatus::Valid,
        vertex_count: vc as u32,
        triangle_count: (ic / 3) as u32,
        degenerate_count: 0,
    };

    if vc == 0 || ic == 0 {
        rep.status = MeshValidationStatus::Empty;
        return rep;
    }
    if vc > MAX_VERTICES {
        rep.status = MeshValidationStatus::TooManyVertices;
        return rep;
    }
    if ic > MAX_INDICES {
        rep.status = MeshValidationStatus::TooManyIndices;
        return rep;
    }
    if ic % 3 != 0 {
        rep.status = MeshValidationStatus::BadIndexCount;
        return rep;
    }

    for p in &mesh.positions {
        if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
            rep.status = MeshValidationStatus::NonFinite;
            return rep;
        }
    }
    if !mesh.bounds.is_finite() && vc > 0 {
        // bounds may be EMPTY if not recomputed — recheck positions only
    }

    for &idx in &mesh.indices {
        if idx as usize >= vc {
            rep.status = MeshValidationStatus::IndexOutOfBounds;
            return rep;
        }
    }

    let mut deg = 0u32;
    let mut t = 0usize;
    while t + 2 < ic {
        let i0 = mesh.indices[t] as usize;
        let i1 = mesh.indices[t + 1] as usize;
        let i2 = mesh.indices[t + 2] as usize;
        let a = mesh.positions[i0];
        let b = mesh.positions[i1];
        let c = mesh.positions[i2];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cx = ab[1] * ac[2] - ab[2] * ac[1];
        let cy = ab[2] * ac[0] - ab[0] * ac[2];
        let cz = ab[0] * ac[1] - ab[1] * ac[0];
        let area2 = cx * cx + cy * cy + cz * cz;
        if area2 < 1e-20 {
            deg += 1;
        }
        t += 3;
    }
    rep.degenerate_count = deg;
    // All-degenerate is a failure; a few skinny tris allowed.
    if deg == rep.triangle_count && rep.triangle_count > 0 {
        rep.status = MeshValidationStatus::DegenerateTriangle;
    }
    rep
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::spatial::geometry_ir::MeshIR;

    #[test]
    fn valid_triangle() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        m.indices = vec![0, 1, 2];
        m.recompute_bounds_and_hash();
        let r = validate_mesh_ir(&m);
        assert!(r.ok());
        assert_eq!(r.triangle_count, 1);
    }

    #[test]
    fn oob_index_fails() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0]];
        m.indices = vec![0, 1, 2];
        let r = validate_mesh_ir(&m);
        assert_eq!(r.status, MeshValidationStatus::IndexOutOfBounds);
    }
}
