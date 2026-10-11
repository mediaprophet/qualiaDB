use super::AuthoringError;
#[derive(Debug, Clone, Copy)]
pub struct ClosedLoftReceipt {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Closed loft of convex XY sections ordered along Z. This is an offline
/// authoring operation: every section has the same counterclockwise ring
/// topology, finite coordinates, and a strictly increasing station position.
/// Side surfaces use the CAD loft API; terminal sections receive convex caps.
pub fn loft_convex_sections_into(
    sections: &[Vec<[f32; 3]>],
    out_positions: &mut [[f32; 3]],
    out_triangles: &mut [[u32; 3]],
) -> Result<ClosedLoftReceipt, AuthoringError> {
    use super::super::{parametric_cad, Point3};
    if sections.len() < 2 || sections.len() > 256 {
        return Err(AuthoringError::InvalidParameters("loft needs 2..256 sections"));
    }
    let count = sections[0].len();
    if !(3..=128).contains(&count) {
        return Err(AuthoringError::InvalidParameters("loft ring needs 3..128 vertices"));
    }
    let required_vertices = (sections.len() - 1) * (count + 1) * 2 + (count + 1) * 2;
    let required_triangles = sections.len() * count * 2;
    if out_positions.len() < required_vertices {
        return Err(AuthoringError::BufferTooSmall { needed: required_vertices, have: out_positions.len() });
    }
    if out_triangles.len() < required_triangles {
        return Err(AuthoringError::BufferTooSmall { needed: required_triangles, have: out_triangles.len() });
    }
    let mut previous_z = f32::NEG_INFINITY;
    for ring in sections {
        if ring.len() != count || ring.iter().flatten().any(|v| !v.is_finite()) {
            return Err(AuthoringError::InvalidParameters("loft section topology or coordinates"));
        }
        let z = ring[0][2];
        if z <= previous_z || ring.iter().any(|p| (p[2] - z).abs() > 1e-5) {
            return Err(AuthoringError::InvalidParameters("loft sections must be ordered XY planes"));
        }
        previous_z = z;
        let mut area = 0.0_f64;
        for i in 0..count {
            let a = ring[i]; let b = ring[(i + 1) % count]; let c = ring[(i + 2) % count];
            area += a[0] as f64 * b[1] as f64 - b[0] as f64 * a[1] as f64;
            let turn = (b[0] - a[0]) as f64 * (c[1] - b[1]) as f64
                - (b[1] - a[1]) as f64 * (c[0] - b[0]) as f64;
            if turn < -1e-6 || a == b {
                return Err(AuthoringError::InvalidParameters("loft rings must be convex counterclockwise"));
            }
            for p in ring {
                let side = (b[0] - a[0]) as f64 * (p[1] - a[1]) as f64
                    - (b[1] - a[1]) as f64 * (p[0] - a[0]) as f64;
                if side < -1e-6 {
                    return Err(AuthoringError::InvalidParameters("loft ring is not convex"));
                }
            }
        }
        if area <= 1e-8 {
            return Err(AuthoringError::InvalidParameters("loft section has no positive area"));
        }
    }
    let mut positions = Vec::with_capacity((sections.len() - 1) * (count + 1) * 2 + count * 2 + 2);
    let mut triangles = Vec::with_capacity((sections.len() - 1) * count * 2 + count * 2);
    for pair in sections.windows(2) {
        let ring = |points: &[[f32; 3]]| -> Vec<Point3> {
            points.iter().chain(points.first()).map(|p|
                Point3::new(p[0] as f64, p[1] as f64, p[2] as f64)).collect()
        };
        let a = ring(&pair[0]); let b = ring(&pair[1]);
        let mut vertices = vec![Point3::new(0.0, 0.0, 0.0); (count + 1) * 2];
        let mut faces = vec![[0; 3]; count * 2];
        let (nv, nt) = parametric_cad::loft_profiles(&a, &b, &mut vertices, &mut faces)
            .map_err(|_| AuthoringError::InvalidParameters("CAD loft rejected section"))?;
        let base = positions.len() as u32;
        positions.extend(vertices[..nv].iter().map(|p| [p.x as f32, p.y as f32, p.z as f32]));
        triangles.extend(faces[..nt].iter().map(|t| [base + t[0], base + t[1], base + t[2]]));
    }
    for (ring, reverse) in [(&sections[0], true), (&sections[sections.len() - 1], false)] {
        let base = positions.len() as u32;
        positions.extend_from_slice(ring);
        let mut center = [0.0; 3];
        for p in ring { for axis in 0..3 { center[axis] += p[axis] / count as f32; } }
        let middle = positions.len() as u32;
        positions.push(center);
        for i in 0..count {
            let a = base + i as u32; let b = base + ((i + 1) % count) as u32;
            triangles.push(if reverse { [middle, b, a] } else { [middle, a, b] });
        }
    }
    let mut min = [f32::INFINITY; 3]; let mut max = [f32::NEG_INFINITY; 3];
    for p in &positions { for axis in 0..3 { min[axis] = min[axis].min(p[axis]); max[axis] = max[axis].max(p[axis]); } }
    out_positions[..positions.len()].copy_from_slice(&positions);
    out_triangles[..triangles.len()].copy_from_slice(&triangles);
    Ok(ClosedLoftReceipt { vertex_count: positions.len(), triangle_count: triangles.len(), min, max })
}

#[cfg(test)]
mod convex_loft_tests {
    use super::*;
    use crate::render::assets::Mesh;

    fn loft_convex_sections(sections: &[Vec<[f32; 3]>]) -> Result<Mesh, AuthoringError> {
        let mut positions = vec![[0.0; 3]; 512];
        let mut triangles = vec![[0; 3]; 512];
        let receipt = loft_convex_sections_into(sections, &mut positions, &mut triangles)?;
        positions.truncate(receipt.vertex_count);
        triangles.truncate(receipt.triangle_count);
        Ok(Mesh { positions, triangles, min: receipt.min, max: receipt.max })
    }

    fn section(z: f32, width: f32) -> Vec<[f32; 3]> {
        vec![[-width, 0.0, z], [width, 0.0, z], [width, 1.0, z], [-width, 1.0, z]]
    }

    #[test]
    fn closed_loft_has_outward_caps_and_repeatable_geometry() {
        let sections = vec![section(-2.0, 0.7), section(0.0, 0.9), section(2.0, 0.8)];
        let mesh = loft_convex_sections(&sections).unwrap();
        let repeat = loft_convex_sections(&sections).unwrap();
        assert_eq!(mesh.positions, repeat.positions);
        assert_eq!(mesh.triangles, repeat.triangles);
        assert_eq!(mesh.triangles.len(), 24);
        for (offset, sign) in [(16, -1.0), (20, 1.0)] {
            for t in &mesh.triangles[offset..offset + 4] {
                let a = mesh.positions[t[0] as usize];
                let b = mesh.positions[t[1] as usize];
                let c = mesh.positions[t[2] as usize];
                let normal_z = (b[0] - a[0]) * (c[1] - a[1])
                    - (b[1] - a[1]) * (c[0] - a[0]);
                assert!(normal_z * sign > 0.0);
            }
        }
    }

    #[test]
    fn loft_rejects_invalid_sections_instead_of_emitting_bad_meshes() {
        let mut invalid = vec![section(0.0, 1.0), section(1.0, 1.0)];
        invalid[1][0][0] = f32::NAN;
        assert!(loft_convex_sections(&invalid).is_err());
        assert!(loft_convex_sections(&[section(1.0, 1.0), section(0.0, 1.0)]).is_err());
        let mut clockwise = section(0.0, 1.0);
        clockwise.reverse();
        assert!(loft_convex_sections(&[clockwise, section(1.0, 1.0)]).is_err());
        let mut concave = section(1.0, 1.0);
        concave[2] = [0.0, 0.2, 1.0];
        assert!(loft_convex_sections(&[section(0.0, 1.0), concave]).is_err());
    }
}

