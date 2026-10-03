//! Deterministic primitive assembly for small authored scenes.
//! Geometry is built once when scene state changes; rendering remains in the portal.

use crate::render::assets::Mesh;
use crate::specialized_libs::computational_geometry::authoring::box_mesh;
use crate::specialized_libs::computational_geometry::geometry_workspace::GeometryWorkspace;

#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Box { center: [f32; 3], size: [f32; 3] },
    Roof { center: [f32; 3], size: [f32; 3] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenePrimitiveError {
    InvalidDimension,
    TooManyVertices,
    EmptyScene,
    OutputTooSmall,
    WorkspaceLimit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssemblyReceipt {
    pub vertex_count: usize,
    pub triangle_count: usize,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// Assemble at most 256 primitives in caller-owned buffers. The caller owns
/// the geometry arena and output storage; no scene-sized allocation is hidden
/// inside this API. Existing primitive builders may use bounded cold scratch.
pub fn assemble_into(
    primitives: &[Primitive],
    positions: &mut [[f32; 3]],
    triangles: &mut [[u32; 3]],
    workspace: &mut GeometryWorkspace<'_>,
) -> Result<AssemblyReceipt, ScenePrimitiveError> {
    if primitives.is_empty() {
        return Err(ScenePrimitiveError::EmptyScene);
    }
    if primitives.len() > 256 {
        return Err(ScenePrimitiveError::TooManyVertices);
    }
    let needed_vertices: usize = primitives
        .iter()
        .map(|p| match p {
            Primitive::Box { .. } => 8,
            Primitive::Roof { .. } => 6,
        })
        .sum();
    let needed_triangles: usize = primitives
        .iter()
        .map(|p| match p {
            Primitive::Box { .. } => 12,
            Primitive::Roof { .. } => 8,
        })
        .sum();
    if positions.len() < needed_vertices || triangles.len() < needed_triangles {
        return Err(ScenePrimitiveError::OutputTooSmall);
    }
    let recipe_len = 6 + primitives.len() * 25;
    workspace
        .admit_pass(recipe_len)
        .map_err(|_| ScenePrimitiveError::WorkspaceLimit)?;
    let scratch = workspace
        .alloc(recipe_len)
        .map_err(|_| ScenePrimitiveError::WorkspaceLimit)?;
    recipe_write(primitives, scratch)?;
    let mut receipt = AssemblyReceipt {
        vertex_count: 0,
        triangle_count: 0,
        min: [f32::INFINITY; 3],
        max: [f32::NEG_INFINITY; 3],
    };
    for primitive in primitives {
        let (center, size, mesh) = match *primitive {
            Primitive::Box { center, size } => {
                let mesh = box_mesh(size[0], size[1], size[2])
                    .map_err(|_| ScenePrimitiveError::InvalidDimension)?;
                (center, size, mesh)
            }
            Primitive::Roof { center, size } => {
                validate(center, size)?;
                let x = size[0] * 0.5;
                let z = size[2] * 0.5;
                let mesh = Mesh {
                    positions: vec![
                        [-x, 0.0, -z],
                        [x, 0.0, -z],
                        [x, 0.0, z],
                        [-x, 0.0, z],
                        [-x, size[1], 0.0],
                        [x, size[1], 0.0],
                    ],
                    triangles: vec![
                        [0, 4, 3],
                        [0, 1, 5],
                        [0, 5, 4],
                        [1, 2, 5],
                        [2, 3, 5],
                        [3, 5, 4],
                        [2, 3, 0],
                        [2, 0, 1],
                    ],
                    min: [-x, 0.0, -z],
                    max: [x, size[1], z],
                };
                (center, size, mesh)
            }
        };
        validate(center, size)?;
        let offset = receipt.vertex_count as u32;
        for mut p in mesh.positions {
            for axis in 0..3 {
                p[axis] += center[axis];
                receipt.min[axis] = receipt.min[axis].min(p[axis]);
                receipt.max[axis] = receipt.max[axis].max(p[axis]);
            }
            positions[receipt.vertex_count] = p;
            receipt.vertex_count += 1;
        }
        for t in mesh.triangles {
            triangles[receipt.triangle_count] = [t[0] + offset, t[1] + offset, t[2] + offset];
            receipt.triangle_count += 1;
        }
    }
    Ok(receipt)
}

/// Write a canonical primitive source receipt into caller storage. The recipe
/// is carried inside a `.10d` provenance sidecar; the mesh is derived data.
/// `QSP1` = four-byte magic, u16 little-endian primitive count, then one
/// u8 kind (1 box, 2 roof) and six little-endian f32 values per primitive
/// (center xyz followed by size xyz), preserving declared primitive order.
pub fn recipe_write(
    primitives: &[Primitive],
    bytes: &mut [u8],
) -> Result<usize, ScenePrimitiveError> {
    if primitives.is_empty() {
        return Err(ScenePrimitiveError::EmptyScene);
    }
    if primitives.len() > 256 {
        return Err(ScenePrimitiveError::TooManyVertices);
    }
    let len = 6 + primitives.len() * 25;
    if bytes.len() < len {
        return Err(ScenePrimitiveError::OutputTooSmall);
    }
    bytes[..4].copy_from_slice(b"QSP1");
    bytes[4..6].copy_from_slice(&(primitives.len() as u16).to_le_bytes());
    let mut offset = 6;
    for primitive in primitives {
        let (kind, center, size) = match *primitive {
            Primitive::Box { center, size } => (1_u8, center, size),
            Primitive::Roof { center, size } => (2_u8, center, size),
        };
        validate(center, size)?;
        bytes[offset] = kind;
        offset += 1;
        for value in center.into_iter().chain(size) {
            bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            offset += 4;
        }
    }
    Ok(len)
}

fn validate(center: [f32; 3], size: [f32; 3]) -> Result<(), ScenePrimitiveError> {
    if center.iter().all(|v| v.is_finite()) && size.iter().all(|v| v.is_finite() && *v > 0.0) {
        Ok(())
    } else {
        Err(ScenePrimitiveError::InvalidDimension)
    }
}

/// Match the portal's shared mesh orbit normalisation for semantic pick nodes.
pub fn portal_point(bounds_min: [f32; 3], bounds_max: [f32; 3], point: [f32; 3]) -> [f32; 3] {
    let span = (0..3)
        .map(|i| bounds_max[i] - bounds_min[i])
        .fold(1e-6_f32, f32::max);
    std::array::from_fn(|i| (point[i] - (bounds_min[i] + bounds_max[i]) * 0.5) * 1.6 / span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::geometry_workspace::Cancellation;

    #[test]
    fn assembles_translated_primitives_and_rejects_invalid_input() {
        let mut positions = [[0.0; 3]; 16];
        let mut triangles = [[0; 3]; 24];
        let mut scratch = [0u8; 256];
        let cancel = Cancellation::new();
        let mut workspace = GeometryWorkspace::new(&mut scratch, &cancel);
        let scene = assemble_into(
            &[
                Primitive::Box {
                    center: [2.0, 0.5, 0.0],
                    size: [2.0, 1.0, 1.0],
                },
                Primitive::Roof {
                    center: [2.0, 1.0, 0.0],
                    size: [2.0, 0.5, 1.0],
                },
            ],
            &mut positions,
            &mut triangles,
            &mut workspace,
        )
        .unwrap();
        assert_eq!(scene.vertex_count, 14);
        assert_eq!(scene.triangle_count, 20);
        assert_eq!(scene.min[0], 1.0);
        assert_eq!(scene.max[1], 1.5);
        assert_eq!(portal_point(scene.min, scene.max, [2.0, 0.75, 0.0])[0], 0.0);
        assert_eq!(
            assemble_into(&[], &mut positions, &mut triangles, &mut workspace),
            Err(ScenePrimitiveError::EmptyScene)
        );
        let mut recipe = [0u8; 31];
        let written = recipe_write(
            &[Primitive::Box {
                center: [2.0, 0.5, 0.0],
                size: [2.0, 1.0, 1.0],
            }],
            &mut recipe,
        )
        .unwrap();
        assert_eq!(&recipe[..4], b"QSP1");
        assert_eq!(written, 31);
    }
}
