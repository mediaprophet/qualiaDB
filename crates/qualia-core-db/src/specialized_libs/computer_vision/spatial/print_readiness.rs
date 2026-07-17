//! Print readiness checks for MeshIR (3D print excellence).

use super::geometry_ir::MeshIR;
use super::validate::validate_mesh_ir;

#[derive(Debug, Clone, Copy)]
pub struct PrintReadiness {
    pub ok: bool,
    pub manifold_ok: bool,
    pub triangle_count: u32,
    pub bounds_mm: [f32; 3],
    pub within_envelope: bool,
    pub note: &'static str,
}

/// `envelope_mm` max XYZ extents (e.g. printer bed). Mesh positions assumed mm.
pub fn print_readiness(mesh: &MeshIR, envelope_mm: [f32; 3]) -> PrintReadiness {
    let rep = validate_mesh_ir(mesh);
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    for p in &mesh.positions {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let bounds = [max[0] - min[0], max[1] - min[1], max[2] - min[2]];
    let within = bounds[0] <= envelope_mm[0]
        && bounds[1] <= envelope_mm[1]
        && bounds[2] <= envelope_mm[2]
        && mesh.positions.len() > 0;
    let ok = rep.ok() && within && !mesh.indices.is_empty();
    PrintReadiness {
        ok,
        manifold_ok: rep.ok(),
        triangle_count: rep.triangle_count,
        bounds_mm: bounds,
        within_envelope: within,
        note: if ok {
            "print-ready heuristic: validated mesh within envelope (not guaranteed slicer success)"
        } else {
            "not print-ready: validation or envelope failed"
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computer_vision::spatial::geometry_ir::MeshIR;
    #[test]
    fn small_tri_ok() {
        let mut m = MeshIR::empty();
        m.positions = vec![[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]];
        m.indices = vec![0, 1, 2];
        m.recompute_bounds_and_hash();
        let r = print_readiness(&m, [200.0, 200.0, 200.0]);
        assert!(r.ok);
    }
}
