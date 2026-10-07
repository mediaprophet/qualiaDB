use std::collections::HashMap;

use super::AssetError;

/// A per-primitive MikkTSpace result with topology expanded only where corner tangents differ.
pub(super) struct TangentSplit {
    /// Original vertex index for each output vertex; copy every source vertex attribute by this map.
    pub source_vertices: Vec<u32>,
    /// Rewritten triangle indices addressing the split output vertices.
    pub indices: Vec<u32>,
    /// One MikkTSpace tangent and handedness value per output vertex.
    pub tangents: Vec<[f32; 4]>,
}

struct TriangleGeometry<'a> {
    positions: &'a [[f32; 3]],
    normals: &'a [[f32; 3]],
    texcoords: &'a [[f32; 2]],
    indices: &'a [u32],
    corner_tangents: Vec<[f32; 4]>,
}

impl mikktspace::Geometry for TriangleGeometry<'_> {
    fn num_faces(&self) -> usize {
        self.indices.len() / 3
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        self.positions[self.indices[face * 3 + vert] as usize]
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        self.normals[self.indices[face * 3 + vert] as usize]
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        self.texcoords[self.indices[face * 3 + vert] as usize]
    }

    fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
        self.corner_tangents[face * 3 + vert] = tangent;
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct TangentVertexKey {
    source: u32,
    tangent_bits: [u32; 4],
}

/// Generate MikkTSpace tangents per triangle corner, then deterministically split indexed vertices
/// when corners require different tangent frames. Averaging corner frames through the original
/// index list is incompatible with MikkTSpace and can corrupt normal-map seams.
pub(super) fn generate_mikktspace_tangents(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    texcoords: &[[f32; 2]],
    indices: &[u32],
) -> Result<TangentSplit, AssetError> {
    if indices.is_empty() || indices.len() % 3 != 0 {
        return Err(AssetError::Parse(
            "MikkTSpace requires a non-empty triangle index stream".into(),
        ));
    }
    if positions.len() != normals.len() || positions.len() != texcoords.len() {
        return Err(AssetError::Parse(
            "MikkTSpace position, normal and TEXCOORD_0 counts must match".into(),
        ));
    }
    if indices
        .iter()
        .any(|&index| index as usize >= positions.len())
    {
        return Err(AssetError::Parse(
            "MikkTSpace index is outside the vertex streams".into(),
        ));
    }

    let mut geometry = TriangleGeometry {
        positions,
        normals,
        texcoords,
        indices,
        corner_tangents: vec![[0.0; 4]; indices.len()],
    };
    if !mikktspace::generate_tangents(&mut geometry) {
        return Err(AssetError::Parse(
            "MikkTSpace rejected the supplied triangle/UV geometry".into(),
        ));
    }

    // Hash lookup is used only for membership. Output IDs are assigned in original face/corner
    // order, so HashMap iteration order cannot affect deterministic compiled geometry.
    let mut vertices = HashMap::<TangentVertexKey, u32>::with_capacity(indices.len());
    let mut source_vertices = Vec::with_capacity(indices.len());
    let mut tangents = Vec::with_capacity(indices.len());
    let mut remapped_indices = Vec::with_capacity(indices.len());
    for (corner, &source) in indices.iter().enumerate() {
        let tangent = orthogonalize_imported_tangent(
            geometry.corner_tangents[corner],
            normals[source as usize],
        )?;
        let key = TangentVertexKey {
            source,
            tangent_bits: tangent.map(f32::to_bits),
        };
        let output = match vertices.get(&key) {
            Some(&output) => output,
            None => {
                let output = u32::try_from(source_vertices.len()).map_err(|_| {
                    AssetError::Parse("MikkTSpace output exceeds the u32 vertex limit".into())
                })?;
                source_vertices.push(source);
                tangents.push(tangent);
                vertices.insert(key, output);
                output
            }
        };
        remapped_indices.push(output);
    }

    Ok(TangentSplit {
        source_vertices,
        indices: remapped_indices,
        tangents,
    })
}

pub(super) fn orthogonalize_imported_tangent(
    tangent: [f32; 4],
    normal: [f32; 3],
) -> Result<[f32; 4], AssetError> {
    if tangent.iter().any(|value| !value.is_finite()) || (tangent[3].abs() - 1.0).abs() > 1e-3 {
        return Err(AssetError::Parse(
            "TANGENT must be finite with handedness w = -1 or +1".into(),
        ));
    }
    let dot = tangent[0] * normal[0] + tangent[1] * normal[1] + tangent[2] * normal[2];
    let direction = [
        tangent[0] - dot * normal[0],
        tangent[1] - dot * normal[1],
        tangent[2] - dot * normal[2],
    ];
    let normalized = normalize_imported_normal(direction)
        .map_err(|_| AssetError::Parse("TANGENT direction is degenerate".into()))?;
    Ok([
        normalized[0],
        normalized[1],
        normalized[2],
        tangent[3].signum(),
    ])
}

pub(super) fn default_tangent_for_normal(normal: [f32; 3]) -> [f32; 4] {
    let axis = if normal[0].abs() < 0.8 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let cross = [
        normal[1] * axis[2] - normal[2] * axis[1],
        normal[2] * axis[0] - normal[0] * axis[2],
        normal[0] * axis[1] - normal[1] * axis[0],
    ];
    let tangent = normalize_imported_normal(cross).unwrap_or([1.0, 0.0, 0.0]);
    [tangent[0], tangent[1], tangent[2], 1.0]
}

pub(super) fn normalize_imported_normal(normal: [f32; 3]) -> Result<[f32; 3], AssetError> {
    if normal.iter().any(|v| !v.is_finite()) {
        return Err(AssetError::Parse(
            "NORMAL contains non-finite values".into(),
        ));
    }
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        return Err(AssetError::Parse(
            "NORMAL contains a zero-length vector".into(),
        ));
    }
    Ok([normal[0] / length, normal[1] / length, normal[2] / length])
}

pub(super) fn generate_import_normals(
    positions: &[[f32; 3]],
    indices: &[u32],
) -> Result<Vec<[f32; 3]>, AssetError> {
    let mut normals = vec![[0.0f32; 3]; positions.len()];
    for tri in indices.chunks_exact(3) {
        let (a, b, c) = (
            positions[tri[0] as usize],
            positions[tri[1] as usize],
            positions[tri[2] as usize],
        );
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let n = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        if n.iter().any(|v| !v.is_finite()) {
            return Err(AssetError::Parse("generated NORMAL is non-finite".into()));
        }
        for &index in tri {
            let target = &mut normals[index as usize];
            for axis in 0..3 {
                target[axis] += n[axis];
            }
        }
    }
    for normal in &mut normals {
        if normal.iter().any(|v| !v.is_finite()) {
            return Err(AssetError::Parse("generated NORMAL is non-finite".into()));
        }
        let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if length > f32::EPSILON {
            for value in normal {
                *value /= length;
            }
        } else {
            *normal = [0.0, 0.0, 1.0];
        }
    }
    Ok(normals)
}
